use rusqlite::{params, Connection};

use crate::models::{Repository, Task, TaskSource, TaskStatus};

// --- Settings ---

/// Reads a single setting value by key. Returns `None` when the row is
/// missing or empty so callers can distinguish "unset" from "set but blank".
/// The value is trimmed because the settings UI may persist stray whitespace.
pub fn get_setting(conn: &Connection, key: &str) -> Option<String> {
    conn.query_row(
        "SELECT value FROM settings WHERE key = ?1",
        params![key],
        |row| row.get::<_, String>(0),
    )
    .ok()
    .map(|v| v.trim().to_string())
}

/// Upserts a setting row by key. New rows are inserted; existing rows are
/// updated in place. The `ON CONFLICT` clause keeps this a single statement
/// regardless of whether the key already exists.
pub fn set_setting(conn: &Connection, key: &str, value: &str) -> Result<(), String> {
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = ?2",
        params![key, value],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

// --- Repositories ---

pub fn insert_repository(conn: &Connection, repo: &Repository) -> Result<(), String> {
    conn.execute(
        "INSERT INTO repositories (id, name, path, created_at, updated_at, max_worktrees) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![repo.id, repo.name, repo.path, repo.created_at, repo.updated_at, repo.max_worktrees],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn list_repositories(conn: &Connection) -> Result<Vec<Repository>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT r.id, r.name, r.path, r.created_at, r.updated_at, r.max_worktrees, \
             (SELECT COUNT(*) FROM tasks t WHERE t.repository_id = r.id AND t.worktree_path IS NOT NULL) \
             FROM repositories r ORDER BY r.name"
        )
        .map_err(|e| e.to_string())?;

    let repos = stmt
        .query_map([], |row| {
            Ok(Repository {
                id: row.get(0)?,
                name: row.get(1)?,
                path: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
                max_worktrees: row.get(5)?,
                active_worktree_count: row.get(6)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(repos)
}

pub fn delete_repository(conn: &Connection, id: &str) -> Result<(), String> {
    conn.execute("DELETE FROM repositories WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn get_repository_path(conn: &Connection, id: &str) -> Result<String, String> {
    conn.query_row(
        "SELECT path FROM repositories WHERE id = ?1",
        params![id],
        |row| row.get(0),
    )
    .map_err(|e| format!("Repository not found: {}", e))
}

// --- Tasks ---

pub fn insert_task(conn: &Connection, task: &Task) -> Result<(), String> {
    conn.execute(
        "INSERT INTO tasks (id, repository_id, title, description, status, source, jira_key, jira_url, git_branch, worktree_path, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![
            task.id,
            task.repository_id,
            task.title,
            task.description,
            task.status.as_str(),
            task.source.as_str(),
            task.jira_key,
            task.jira_url,
            task.git_branch,
            task.worktree_path,
            task.created_at,
            task.updated_at,
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn list_tasks_by_repo(conn: &Connection, repository_id: &str) -> Result<Vec<Task>, String> {
    let mut stmt = conn
        .prepare("SELECT id, repository_id, title, description, status, source, jira_key, jira_url, git_branch, worktree_path, created_at, updated_at FROM tasks WHERE repository_id = ?1 ORDER BY created_at DESC")
        .map_err(|e| e.to_string())?;

    let tasks = stmt
        .query_map(params![repository_id], |row| {
            let status_str: String = row.get(4)?;
            let source_str: String = row.get(5)?;
            Ok(Task {
                id: row.get(0)?,
                repository_id: row.get(1)?,
                title: row.get(2)?,
                description: row.get(3)?,
                status: TaskStatus::from_str(&status_str).unwrap_or(TaskStatus::Pending),
                source: TaskSource::from_str(&source_str).unwrap_or(TaskSource::Manual),
                jira_key: row.get(6)?,
                jira_url: row.get(7)?,
                git_branch: row.get(8)?,
                worktree_path: row.get(9)?,
                created_at: row.get(10)?,
                updated_at: row.get(11)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(tasks)
}

pub fn update_task_status(conn: &Connection, id: &str, status: &str, updated_at: &str) -> Result<Task, String> {
    conn.execute(
        "UPDATE tasks SET status = ?1, updated_at = ?2 WHERE id = ?3",
        params![status, updated_at, id],
    )
    .map_err(|e| e.to_string())?;

    get_task_by_id(conn, id)
}

pub fn update_task_content(conn: &Connection, id: &str, title: &str, description: Option<&str>, updated_at: &str) -> Result<Task, String> {
    conn.execute(
        "UPDATE tasks SET title = ?1, description = ?2, updated_at = ?3 WHERE id = ?4",
        params![title, description, updated_at, id],
    )
    .map_err(|e| e.to_string())?;

    get_task_by_id(conn, id)
}

pub fn update_task_git_branch(conn: &Connection, id: &str, branch: &str, updated_at: &str) -> Result<Task, String> {
    conn.execute(
        "UPDATE tasks SET git_branch = ?1, updated_at = ?2 WHERE id = ?3",
        params![branch, updated_at, id],
    )
    .map_err(|e| e.to_string())?;

    get_task_by_id(conn, id)
}

pub fn get_task_by_id(conn: &Connection, id: &str) -> Result<Task, String> {
    let mut stmt = conn
        .prepare("SELECT id, repository_id, title, description, status, source, jira_key, jira_url, git_branch, worktree_path, created_at, updated_at FROM tasks WHERE id = ?1")
        .map_err(|e| e.to_string())?;

    stmt.query_row(params![id], |row| {
        let status_str: String = row.get(4)?;
        let source_str: String = row.get(5)?;
        Ok(Task {
            id: row.get(0)?,
            repository_id: row.get(1)?,
            title: row.get(2)?,
            description: row.get(3)?,
            status: TaskStatus::from_str(&status_str).unwrap_or(TaskStatus::Pending),
            source: TaskSource::from_str(&source_str).unwrap_or(TaskSource::Manual),
            jira_key: row.get(6)?,
            jira_url: row.get(7)?,
            git_branch: row.get(8)?,
            worktree_path: row.get(9)?,
            created_at: row.get(10)?,
            updated_at: row.get(11)?,
        })
    })
    .map_err(|e| e.to_string())
}

pub fn delete_task(conn: &Connection, id: &str) -> Result<(), String> {
    conn.execute("DELETE FROM tasks WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn update_task_worktree_path(conn: &Connection, id: &str, path: Option<&str>, updated_at: &str) -> Result<Task, String> {
    conn.execute(
        "UPDATE tasks SET worktree_path = ?1, updated_at = ?2 WHERE id = ?3",
        params![path, updated_at, id],
    )
    .map_err(|e| e.to_string())?;
    get_task_by_id(conn, id)
}

pub fn count_active_worktrees(conn: &Connection, repository_id: &str) -> Result<i64, String> {
    conn.query_row(
        "SELECT COUNT(*) FROM tasks WHERE repository_id = ?1 AND worktree_path IS NOT NULL",
        params![repository_id],
        |row| row.get(0),
    )
    .map_err(|e| e.to_string())
}

pub fn get_max_worktrees(conn: &Connection, repository_id: &str) -> Result<i64, String> {
    conn.query_row(
        "SELECT max_worktrees FROM repositories WHERE id = ?1",
        params![repository_id],
        |row| row.get(0),
    )
    .map_err(|e| e.to_string())
}

// --- Attachments ---

fn parse_injection_phases(raw: &str) -> Vec<String> {
    serde_json::from_str::<Vec<String>>(raw).unwrap_or_else(|_| {
        // Fallback: treat plain string as single-phase
        if raw.is_empty() { vec![] } else { vec![raw.to_string()] }
    })
}

pub fn insert_attachment(conn: &Connection, attachment: &crate::models::TaskAttachment) -> Result<(), String> {
    let phases_json = serde_json::to_string(&attachment.injection_phases)
        .map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO task_attachments (id, task_id, file_name, file_size, mime_type, content, injection_phase, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            attachment.id,
            attachment.task_id,
            attachment.file_name,
            attachment.file_size,
            attachment.mime_type,
            attachment.content,
            phases_json,
            attachment.created_at,
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn list_attachments_by_task(conn: &Connection, task_id: &str) -> Result<Vec<crate::models::TaskAttachment>, String> {
    let mut stmt = conn
        .prepare("SELECT id, task_id, file_name, file_size, mime_type, content, injection_phase, created_at FROM task_attachments WHERE task_id = ?1 ORDER BY created_at ASC")
        .map_err(|e| e.to_string())?;

    let attachments = stmt
        .query_map(params![task_id], |row| {
            let phases_raw: String = row.get(6)?;
            Ok(crate::models::TaskAttachment {
                id: row.get(0)?,
                task_id: row.get(1)?,
                file_name: row.get(2)?,
                file_size: row.get(3)?,
                mime_type: row.get(4)?,
                content: row.get(5)?,
                injection_phases: parse_injection_phases(&phases_raw),
                created_at: row.get(7)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(attachments)
}

pub fn list_attachments_by_task_and_phase(conn: &Connection, task_id: &str, phase: &str) -> Result<Vec<crate::models::TaskAttachment>, String> {
    let all = list_attachments_by_task(conn, task_id)?;
    Ok(all.into_iter().filter(|a| a.injection_phases.iter().any(|p| p == phase)).collect())
}

pub fn delete_attachment(conn: &Connection, id: &str) -> Result<(), String> {
    conn.execute("DELETE FROM task_attachments WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn set_attachment_phases(conn: &Connection, id: &str, phases: &[String]) -> Result<(), String> {
    let phases_json = serde_json::to_string(phases).map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE task_attachments SET injection_phase = ?1 WHERE id = ?2",
        params![phases_json, id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn get_project_delete_preview(conn: &Connection, repo_id: &str) -> Result<crate::models::ProjectDeletePreview, String> {
    let task_count: u32 = conn.query_row(
        "SELECT COUNT(*) FROM tasks WHERE repository_id = ?1",
        params![repo_id],
        |row| row.get(0),
    ).map_err(|e| e.to_string())?;

    let worktree_count: u32 = conn.query_row(
        "SELECT COUNT(*) FROM tasks WHERE repository_id = ?1 AND worktree_path IS NOT NULL",
        params![repo_id],
        |row| row.get(0),
    ).map_err(|e| e.to_string())?;

    let branch_count: u32 = conn.query_row(
        "SELECT COUNT(*) FROM tasks WHERE repository_id = ?1 AND git_branch IS NOT NULL",
        params![repo_id],
        |row| row.get(0),
    ).map_err(|e| e.to_string())?;

    Ok(crate::models::ProjectDeletePreview { task_count, worktree_count, branch_count })
}

/// Returns (worktree_path, git_branch, repo_path) for deletion cleanup.
pub fn get_task_cleanup_info(conn: &Connection, task_id: &str) -> Result<(Option<String>, Option<String>, String), String> {
    conn.query_row(
        "SELECT t.worktree_path, t.git_branch, r.path FROM tasks t JOIN repositories r ON r.id = t.repository_id WHERE t.id = ?1",
        params![task_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    ).map_err(|e| format!("Task not found: {}", e))
}

/// Returns list of (task_id, worktree_path, git_branch) for all tasks in a repository.
pub fn get_tasks_for_repo_cleanup(conn: &Connection, repo_id: &str) -> Result<Vec<(String, Option<String>, Option<String>)>, String> {
    let mut stmt = conn.prepare(
        "SELECT id, worktree_path, git_branch FROM tasks WHERE repository_id = ?1"
    ).map_err(|e| e.to_string())?;

    let rows = stmt.query_map(params![repo_id], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?, row.get::<_, Option<String>>(2)?))
    }).map_err(|e| e.to_string())?
    .collect::<Result<Vec<_>, _>>()
    .map_err(|e| e.to_string())?;

    Ok(rows)
}

/// Returns the worktree_path for a task if set, otherwise the repository root path.
pub fn resolve_working_dir(conn: &Connection, task_id: &str) -> Result<String, String> {
    let (worktree_path, repo_path): (Option<String>, String) = conn
        .query_row(
            "SELECT t.worktree_path, r.path FROM tasks t JOIN repositories r ON r.id = t.repository_id WHERE t.id = ?1",
            params![task_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|e| format!("Task not found: {}", e))?;

    if let Some(wt) = worktree_path {
        if std::path::Path::new(&wt).exists() {
            return Ok(wt);
        }
    }
    Ok(repo_path)
}
