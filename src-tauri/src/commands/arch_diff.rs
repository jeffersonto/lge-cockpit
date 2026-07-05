use tauri::State;

use crate::commands::git::run_git;
use crate::db::queries;
use crate::diff_analysis::{self, AnalysisInput, DiffAnalysis};
use crate::models::arch_diff::ArchitectureDiff;
use crate::settings;
use crate::AppState;

// Max bytes of full diff to parse (50KB). Applied here at the IPC adapter so
// the cap is visible at the seam; the pure `diff_analysis::analyze` does not
// re-truncate.
const MAX_DIFF_BYTES: usize = 50_000;

/// Analyzes the architecture diff of uncommitted working tree changes vs HEAD.
/// Includes both tracked modified files (git diff HEAD) and untracked new files (git status).
#[tauri::command]
pub async fn analyze_working_tree_diff(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    task_id: String,
) -> Result<ArchitectureDiff, String> {
    let (working_dir, env_prefix) = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        (
            queries::resolve_working_dir(&conn, &task_id)?,
            settings::shell_env(&conn).prefix().to_string(),
        )
    };

    // 1. git status --porcelain -uall: captures ALL changes including untracked new files.
    let status_output = run_git(
        &app,
        &working_dir,
        "status --porcelain -uall",
        &env_prefix,
    )
    .await
    .unwrap_or_default();

    if status_output.trim().is_empty() {
        return Ok(empty_diff("HEAD", "HEAD"));
    }

    // 2. Line counts for tracked files (staged + unstaged vs HEAD)
    let numstat_tracked = run_git(
        &app,
        &working_dir,
        "diff HEAD --numstat",
        &env_prefix,
    )
    .await
    .unwrap_or_default();

    let numstat_staged = run_git(
        &app,
        &working_dir,
        "diff --cached --numstat",
        &env_prefix,
    )
    .await
    .unwrap_or_default();

    let mut numstat_combined = numstat_staged;
    numstat_combined.push('\n');
    numstat_combined.push_str(&numstat_tracked);

    // 3. Full diff for tracked files (for import/API parsing)
    let full_diff_tracked = run_git(
        &app,
        &working_dir,
        "diff HEAD",
        &env_prefix,
    )
    .await
    .unwrap_or_default();

    // 4. Full diff for staged new files
    let full_diff_staged = run_git(
        &app,
        &working_dir,
        "diff --cached",
        &env_prefix,
    )
    .await
    .unwrap_or_default();

    let full_diff_raw = format!("{}\n{}", full_diff_tracked, full_diff_staged);

    // 5. Parse name-status from git status --porcelain (includes untracked)
    let name_status = build_name_status_from_porcelain(&status_output);

    // 6. Build numstat: tracked file counts + line counts for untracked files read from disk
    let numstat = build_numstat_with_untracked(&numstat_combined, &status_output, &working_dir);

    let analysis = diff_analysis::analyze(&AnalysisInput {
        base: "HEAD".to_string(),
        head: "working-tree".to_string(),
        name_status,
        numstat,
        full_diff: full_diff_raw,
        max_diff_bytes: MAX_DIFF_BYTES,
    });

    Ok(into_architecture_diff(analysis, ""))
}

/// Analyzes the architecture diff between two commits (kept for backward compatibility).
#[tauri::command]
pub async fn analyze_architecture_diff(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    task_id: String,
    base_commit: String,
    head_commit: String,
) -> Result<ArchitectureDiff, String> {
    if base_commit == head_commit {
        return Ok(empty_diff(&base_commit, &head_commit));
    }

    let (working_dir, env_prefix) = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        (
            queries::resolve_working_dir(&conn, &task_id)?,
            settings::shell_env(&conn).prefix().to_string(),
        )
    };

    let range = format!("{}..{}", base_commit, head_commit);

    let name_status = run_git(
        &app,
        &working_dir,
        &format!("diff --name-status {}", range),
        &env_prefix,
    )
    .await
    .unwrap_or_default();

    let numstat = run_git(
        &app,
        &working_dir,
        &format!("diff --numstat {}", range),
        &env_prefix,
    )
    .await
    .unwrap_or_default();

    let full_diff_raw = run_git(
        &app,
        &working_dir,
        &format!("diff {}", range),
        &env_prefix,
    )
    .await
    .unwrap_or_default();
    let analysis = diff_analysis::analyze(&AnalysisInput {
        base: base_commit.clone(),
        head: head_commit.clone(),
        name_status,
        numstat,
        full_diff: full_diff_raw,
        max_diff_bytes: MAX_DIFF_BYTES,
    });

    Ok(into_architecture_diff(analysis, ""))
}

// ─── Pure helpers kept in the adapter (impure: porcelain → name-status, FS read) ─

/// Converts `git status --porcelain` output into the same format as `git diff --name-status`.
fn build_name_status_from_porcelain(porcelain: &str) -> String {
    let mut lines: Vec<String> = Vec::new();
    for line in porcelain.lines() {
        if line.len() < 4 {
            continue;
        }
        let xy = &line[..2];
        let path = line[3..].trim();
        if path.is_empty() {
            continue;
        }
        let change_type = match xy.trim() {
            "??" => "A",
            s if s.contains('D') => "D",
            s if s.contains('A') || s.contains('C') => "A",
            s if s.contains('R') => {
                if let Some(new_path) = path.split(" -> ").last() {
                    lines.push(format!("A\t{}", new_path.trim()));
                }
                continue;
            }
            _ => "M",
        };
        lines.push(format!("{}\t{}", change_type, path));
    }
    lines.join("\n")
}

/// Builds a numstat string that includes line counts for untracked new files read from disk.
fn build_numstat_with_untracked(tracked_numstat: &str, porcelain: &str, working_dir: &str) -> String {
    let mut result = tracked_numstat.to_string();

    let covered: std::collections::HashSet<String> = tracked_numstat
        .lines()
        .filter_map(|l| {
            let parts: Vec<&str> = l.splitn(3, '\t').collect();
            if parts.len() == 3 { Some(parts[2].trim().to_string()) } else { None }
        })
        .collect();

    for line in porcelain.lines() {
        if line.len() < 4 {
            continue;
        }
        let xy = line[..2].trim();
        let path = line[3..].trim();
        if xy == "??" && !path.is_empty() && !covered.contains(path) {
            let full_path = format!("{}/{}", working_dir, path);
            let line_count = std::fs::read_to_string(&full_path)
                .map(|content| content.lines().count() as u64)
                .unwrap_or(0);
            if line_count > 0 {
                result.push('\n');
                result.push_str(&format!("{}\t0\t{}", line_count, path));
            }
        }
    }
    result
}

// ─── IPC adapter glue ──────────────────────────────────────────────────────

/// Wraps the pure `DiffAnalysis` into the IPC-bound `ArchitectureDiff`,
/// setting the LGE-owned `phase` field. The `phase` is "" here because the
/// two architecture-diff commands aren't tied to a specific LGE phase;
/// callers that want to label the diff with a phase do so at their seam.
fn into_architecture_diff(a: DiffAnalysis, phase: &str) -> ArchitectureDiff {
    ArchitectureDiff {
        phase: phase.to_string(),
        base_commit: a.base_commit,
        head_commit: a.head_commit,
        summary: a.summary,
        file_tree: a.file_tree,
        dependency_graph: a.dependency_graph,
        api_surface: a.api_surface,
    }
}

fn empty_diff(base: &str, head: &str) -> ArchitectureDiff {
    into_architecture_diff(diff_analysis::empty(base, head), "")
}