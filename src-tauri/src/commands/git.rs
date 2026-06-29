use chrono::Utc;
use tauri::State;
use tauri_plugin_shell::ShellExt;

use crate::commands::claude_utils::{resolve_claude_path, shell_env_prefix, shell_escape, user_shell};
use crate::db::queries;
use crate::AppState;

fn now_rfc3339() -> String {
    Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

pub(crate) async fn run_git(app: &tauri::AppHandle, working_dir: &str, args: &str, env_prefix: &str) -> Result<String, String> {
    let shell = app.shell();
    let cmd = format!("{}git -C {} {}", env_prefix, shell_escape(working_dir), args);
    let output = shell
        .command(&user_shell())
        .args(["-l", "-i", "-c", &cmd])
        .output()
        .await
        .map_err(|e| format!("Git command failed: {}", e))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

/// Ensures `.lge-worktrees/` is listed in the repo's `.gitignore`.
fn ensure_gitignore_entry(repo_path: &str) {
    let gitignore_path = format!("{}/.gitignore", repo_path);
    let entry = ".lge-worktrees/";

    let contents = std::fs::read_to_string(&gitignore_path).unwrap_or_default();
    if contents.lines().any(|line| line.trim() == entry) {
        return;
    }

    let separator = if contents.ends_with('\n') || contents.is_empty() { "" } else { "\n" };
    let _ = std::fs::write(&gitignore_path, format!("{}{}{}\n", contents, separator, entry));
}

/// Creates or reuses a git worktree for a task at `{repo_path}/.lge-worktrees/{task_code}/`.
/// Enforces `max_worktrees` limit. Persists `worktree_path` on the task record.
/// Must NOT be called while holding the DB lock (calls async `run_git`).
pub(crate) async fn ensure_worktree(
    app: &tauri::AppHandle,
    state: &AppState,
    task_id: &str,
    repo_path: &str,
    repository_id: &str,
    task_code: &str,
    branch_name: Option<&str>,
    env_prefix: &str,
) -> Result<String, String> {
    let worktree_path = format!("{}/.lge-worktrees/{}", repo_path, task_code);

    // If directory already exists on disk, register in DB and return
    if std::path::Path::new(&worktree_path).exists() {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        let now = now_rfc3339();
        queries::update_task_worktree_path(&conn, task_id, Some(&worktree_path), &now)?;
        return Ok(worktree_path);
    }

    // Enforce max_worktrees limit — first clean up stale DB entries for this repo
    {
        let conn = state.db.lock().map_err(|e| e.to_string())?;

        // Clear stale worktree_path entries where the directory no longer exists
        let mut stmt = conn
            .prepare("SELECT id, worktree_path FROM tasks WHERE repository_id = ?1 AND worktree_path IS NOT NULL")
            .map_err(|e| e.to_string())?;
        let stale: Vec<String> = stmt
            .query_map(rusqlite::params![repository_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .filter(|(_, path)| !std::path::Path::new(path).exists())
            .map(|(id, _)| id)
            .collect();
        let now = now_rfc3339();
        for stale_id in &stale {
            let _ = queries::update_task_worktree_path(&conn, stale_id, None, &now);
        }

        let count = queries::count_active_worktrees(&conn, repository_id)?;
        let max = queries::get_max_worktrees(&conn, repository_id)?;
        if count >= max {
            return Err(format!(
                "Worktree limit reached ({}/{}). Remove completed worktrees before creating a new one.",
                count, max
            ));
        }
    }

    // Create the .lge-worktrees parent directory
    let worktrees_dir = format!("{}/.lge-worktrees", repo_path);
    std::fs::create_dir_all(&worktrees_dir)
        .map_err(|e| format!("Failed to create worktrees dir: {}", e))?;

    // Ensure .lge-worktrees/ is in .gitignore
    ensure_gitignore_entry(repo_path);

    // Create the worktree
    if let Some(branch) = branch_name {
        // Try attaching to an existing branch first
        let attach = run_git(
            app,
            repo_path,
            &format!("worktree add {} {}", shell_escape(&worktree_path), shell_escape(branch)),
            env_prefix,
        ).await;

        if attach.is_err() {
            // Branch doesn't exist — create it from HEAD
            run_git(
                app,
                repo_path,
                &format!("worktree add -b {} {}", shell_escape(branch), shell_escape(&worktree_path)),
                env_prefix,
            ).await
            .map_err(|e| format!("git worktree add failed: {}", e))?;
        }
    } else {
        // No branch — create a detached worktree from HEAD
        run_git(
            app,
            repo_path,
            &format!("worktree add --detach {}", shell_escape(&worktree_path)),
            env_prefix,
        ).await
        .map_err(|e| format!("git worktree add --detach failed: {}", e))?;
    }

    // Persist worktree_path to DB
    {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        let now = now_rfc3339();
        queries::update_task_worktree_path(&conn, task_id, Some(&worktree_path), &now)?;
    }

    Ok(worktree_path)
}

/// Removes a worktree from disk. Does NOT touch the DB.
pub(crate) async fn do_remove_worktree_from_disk(
    app: &tauri::AppHandle,
    repo_path: &str,
    worktree_path: &str,
    env_prefix: &str,
) -> Result<(), String> {
    let _ = run_git(app, repo_path, &format!("worktree remove --force {}", shell_escape(worktree_path)), env_prefix).await;
    if std::path::Path::new(worktree_path).exists() {
        std::fs::remove_dir_all(worktree_path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Deletes a git branch. Does NOT touch the DB.
pub(crate) async fn do_delete_branch(
    app: &tauri::AppHandle,
    repo_path: &str,
    branch_name: &str,
    env_prefix: &str,
) -> Result<(), String> {
    run_git(app, repo_path, &format!("branch -D {}", shell_escape(branch_name)), env_prefix).await.map(|_| ())
}

/// Opens a directory in the user's IDE. Tries VS Code (`code`), then falls back to
/// the system default file manager (`open` on macOS, `xdg-open` on Linux).
#[tauri::command]
pub async fn open_in_editor(app: tauri::AppHandle, path: String) -> Result<(), String> {
    let shell = app.shell();

    // Try VS Code first
    let code_result = shell
        .command(&user_shell())
        .args(["-l", "-i", "-c", &format!("code {}", shell_escape(&path))])
        .output()
        .await;

    if let Ok(output) = code_result {
        if output.status.success() {
            return Ok(());
        }
    }

    // Fallback: system default (Finder on macOS, xdg-open on Linux)
    let open_cmd = if cfg!(target_os = "macos") { "open" } else { "xdg-open" };
    shell
        .command(&user_shell())
        .args(["-l", "-i", "-c", &format!("{} {}", open_cmd, shell_escape(&path))])
        .output()
        .await
        .map_err(|e| format!("Failed to open directory: {}", e))?;

    Ok(())
}

/// Returns the current branch name of the repository.
#[tauri::command]
pub async fn get_current_git_branch(app: tauri::AppHandle, state: State<'_, AppState>, repo_path: String) -> Result<String, String> {
    let env_prefix = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        shell_env_prefix(&conn)
    };
    run_git(&app, &repo_path, "rev-parse --abbrev-ref HEAD", &env_prefix).await
}

/// Creates a git branch via worktree isolation.
/// Fetches the base branch, then creates a new worktree with `git worktree add -b`.
/// Persists both `git_branch` and `worktree_path` on the task record.
#[tauri::command]
pub async fn create_git_branch(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    task_id: String,
    repo_path: String,
    branch_name: String,
    base_branch: String,
) -> Result<String, String> {
    let env_prefix = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        shell_env_prefix(&conn)
    };

    // Fetch remote state for base branch
    run_git(&app, &repo_path, &format!("fetch origin {}", shell_escape(&base_branch)), &env_prefix).await
        .map_err(|e| format!("git fetch failed: {}", e))?;

    // Derive task_code and repository_id
    let (repository_id, task_code) = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        let (repo_id, jira_key): (String, Option<String>) = conn
            .query_row(
                "SELECT repository_id, jira_key FROM tasks WHERE id = ?1",
                rusqlite::params![task_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|e| format!("Task not found: {}", e))?;
        let code = jira_key.unwrap_or_else(|| task_id[..8].to_string());
        (repo_id, code)
    };

    // Delegate to ensure_worktree for limit checking, directory creation, and gitignore
    ensure_worktree(
        &app, &state, &task_id, &repo_path, &repository_id, &task_code, Some(&branch_name), &env_prefix,
    ).await?;

    // Persist branch name on the task
    {
        let now = now_rfc3339();
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        queries::update_task_git_branch(&conn, &task_id, &branch_name, &now)?;
    }

    Ok(branch_name)
}

/// Stages all changes, commits with the given message, and pushes to origin.
/// Resolves working directory from task (worktree or repo root).
#[tauri::command]
pub async fn commit_and_push(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    task_id: String,
    branch_name: String,
    message: String,
) -> Result<String, String> {
    let (working_dir, env_prefix) = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        (queries::resolve_working_dir(&conn, &task_id)?, shell_env_prefix(&conn))
    };

    let manual_commit = format!(
        "cd {} && git add -A && git commit -m '{}' && git push -u origin {}",
        working_dir, message, branch_name
    );
    let manual_push = format!(
        "cd {} && git push -u origin {}",
        working_dir, branch_name
    );

    run_git(&app, &working_dir, "add -A", &env_prefix).await
        .map_err(|e| format!("git add failed: {}<!--CMD-->{}", e, manual_commit))?;

    let commit_result = run_git(&app, &working_dir, &format!("commit -m {}", shell_escape(&message)), &env_prefix).await;

    if let Err(err) = commit_result {
        return Err(format!("git commit failed: {}<!--CMD-->{}", err, manual_commit));
    }

    let push_output = run_git(
        &app,
        &working_dir,
        &format!("push -u origin {}", shell_escape(&branch_name)),
        &env_prefix,
    )
    .await
    .map_err(|e| format!(
        "git push failed: {}<!--CMD-->{}",
        e, manual_push
    ))?;

    Ok(push_output)
}

/// Returns a GitHub URL to open a Pull Request in the browser.
/// Resolves working directory from task (worktree or repo root).
#[tauri::command]
pub async fn create_pull_request(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    task_id: String,
    base_branch: String,
) -> Result<String, String> {
    let (working_dir, env_prefix) = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        (queries::resolve_working_dir(&conn, &task_id)?, shell_env_prefix(&conn))
    };

    let remote_url = run_git(&app, &working_dir, "remote get-url origin", &env_prefix).await?;
    let current_branch = run_git(&app, &working_dir, "rev-parse --abbrev-ref HEAD", &env_prefix).await?;

    // Normalize SSH and HTTPS remotes to "org/repo"
    // Handles: git@github.com:org/repo.git, git@github.com-alias:org/repo.git, https://github.com/org/repo.git
    let repo_slug = if let Some(colon_pos) = remote_url.find(':') {
        if remote_url.starts_with("git@") {
            // SSH format: git@github.com:org/repo.git or git@github.com-emu:org/repo.git
            remote_url[colon_pos + 1..]
                .trim_end_matches(".git")
                .to_string()
        } else {
            // HTTPS format
            remote_url
                .trim_end_matches(".git")
                .trim_start_matches("https://github.com/")
                .to_string()
        }
    } else {
        remote_url
            .trim_end_matches(".git")
            .to_string()
    };

    Ok(format!(
        "https://github.com/{}/compare/{}...{}?expand=1",
        repo_slug, base_branch, current_branch,
    ))
}

/// Uses Claude CLI to generate a short conventional commit message based on the git diff.
/// Resolves working directory from task (worktree or repo root).
#[tauri::command]
pub async fn generate_commit_message(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    task_id: String,
    task_title: String,
    jira_key: Option<String>,
) -> Result<String, String> {
    let (working_dir, env_prefix) = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        (queries::resolve_working_dir(&conn, &task_id)?, shell_env_prefix(&conn))
    };

    // Get a compact diff summary (stat + first 3000 chars of diff)
    let diff_stat = run_git(&app, &working_dir, "diff --stat", &env_prefix).await.unwrap_or_default();
    let diff_full = run_git(&app, &working_dir, "diff", &env_prefix).await.unwrap_or_default();
    let diff_preview: String = diff_full.chars().take(3000).collect();

    let scope = jira_key
        .as_deref()
        .map(|k| format!("({})", k))
        .unwrap_or_default();

    let prompt = format!(
        r#"Generate a single-line conventional commit message for the following changes.

Task: {task_title}
Format: feat{scope}: <short imperative description> (max 72 chars total)

Changed files:
{diff_stat}

Diff preview:
{diff_preview}

Rules:
- Output ONLY the commit message, nothing else
- Use feat{scope}: prefix
- Imperative mood (e.g. "add", "implement", "fix")
- Max 72 characters
- English"#,
        task_title = task_title,
        scope = scope,
        diff_stat = diff_stat,
        diff_preview = diff_preview,
    );

    let claude_bin = resolve_claude_path();
    let shell = app.shell();
    let cmd = format!(
        "cd {} && echo {} | {} --print --model haiku --max-turns 1",
        shell_escape(&working_dir),
        shell_escape(&prompt),
        claude_bin,
    );

    let output = shell
        .command(&user_shell())
        .args(["-l", "-i", "-c", &cmd])
        .output()
        .await
        .map_err(|e| format!("Failed to invoke Claude CLI: {}", e))?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }

    let raw = String::from_utf8_lossy(&output.stdout);

    // Extract plain text — strip JSON wrapper if present
    let message = if let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) {
        v.get("result")
            .and_then(|r| r.as_str())
            .unwrap_or(raw.trim())
            .to_string()
    } else {
        raw.trim().to_string()
    };

    // Guarantee it starts with the expected prefix as fallback
    if message.is_empty() {
        Ok(format!("feat{}: {}", scope, task_title))
    } else {
        Ok(message.lines().next().unwrap_or(&message).trim().to_string())
    }
}

/// Removes the git worktree for a single task and clears worktree_path in DB.
#[tauri::command]
pub async fn remove_worktree(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    task_id: String,
) -> Result<(), String> {
    let (repo_path, worktree_path, env_prefix) = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        let (wt, repo): (Option<String>, String) = conn
            .query_row(
                "SELECT t.worktree_path, r.path FROM tasks t JOIN repositories r ON r.id = t.repository_id WHERE t.id = ?1",
                rusqlite::params![task_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|e| e.to_string())?;
        match wt {
            Some(w) => (repo, w, shell_env_prefix(&conn)),
            None => return Ok(()), // no worktree to remove
        }
    };

    // git worktree remove --force
    let _ = run_git(&app, &repo_path, &format!("worktree remove --force {}", shell_escape(&worktree_path)), &env_prefix).await;

    // If git remove didn't clean up the directory, remove it manually
    if std::path::Path::new(&worktree_path).exists() {
        let _ = std::fs::remove_dir_all(&worktree_path);
    }

    // Clear worktree_path in DB
    {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        let now = now_rfc3339();
        queries::update_task_worktree_path(&conn, &task_id, None, &now)?;
    }

    Ok(())
}

/// Returns the count of stale worktrees (completed tasks with worktree_path set,
/// completed more than 7 days ago) across all repositories.
#[tauri::command]
pub fn check_stale_worktrees(
    state: State<'_, AppState>,
) -> Result<Vec<StaleWorktreeInfo>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT t.id, t.title, t.jira_key, t.worktree_path, t.repository_id, r.name \
             FROM tasks t \
             JOIN repositories r ON r.id = t.repository_id \
             WHERE t.status = 'completed' \
               AND t.worktree_path IS NOT NULL \
               AND t.updated_at <= datetime('now', '-7 days')"
        )
        .map_err(|e| e.to_string())?;
    let results = stmt
        .query_map([], |row| {
            Ok(StaleWorktreeInfo {
                task_id: row.get(0)?,
                task_title: row.get(1)?,
                jira_key: row.get(2)?,
                worktree_path: row.get(3)?,
                repository_id: row.get(4)?,
                repository_name: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(results)
}

#[derive(serde::Serialize, Clone)]
pub struct StaleWorktreeInfo {
    pub task_id: String,
    pub task_title: String,
    pub jira_key: Option<String>,
    pub worktree_path: String,
    pub repository_id: String,
    pub repository_name: String,
}

/// Returns the current HEAD commit SHA for the task's working directory.
#[tauri::command]
pub async fn get_head_commit(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    task_id: String,
) -> Result<String, String> {
    let (working_dir, env_prefix) = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        (queries::resolve_working_dir(&conn, &task_id)?, shell_env_prefix(&conn))
    };
    run_git(&app, &working_dir, "rev-parse HEAD", &env_prefix).await
}

/// Removes all worktrees for completed tasks in a repository.
/// Returns the list of removed worktree paths.
#[tauri::command]
pub async fn remove_completed_worktrees(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    repository_id: String,
) -> Result<Vec<String>, String> {
    // Collect completed tasks with worktrees
    let (tasks, env_prefix): (Vec<(String, String, String)>, String) = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT t.id, t.worktree_path, r.path FROM tasks t \
                 JOIN repositories r ON r.id = t.repository_id \
                 WHERE t.repository_id = ?1 AND t.status = 'completed' AND t.worktree_path IS NOT NULL"
            )
            .map_err(|e| e.to_string())?;
        let results = stmt.query_map(rusqlite::params![repository_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
        (results, shell_env_prefix(&conn))
    };

    let mut removed = Vec::new();
    for (task_id, worktree_path, repo_path) in tasks {
        let _ = run_git(&app, &repo_path, &format!("worktree remove --force {}", shell_escape(&worktree_path)), &env_prefix).await;
        if std::path::Path::new(&worktree_path).exists() {
            let _ = std::fs::remove_dir_all(&worktree_path);
        }
        {
            let conn = state.db.lock().map_err(|e| e.to_string())?;
            let now = now_rfc3339();
            let _ = queries::update_task_worktree_path(&conn, &task_id, None, &now);
        }
        removed.push(worktree_path);
    }

    Ok(removed)
}
