use std::collections::HashMap;

use tauri::State;
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_shell::ShellExt;

use crate::claude_invocation::RealClaudeInvocation;
use crate::commands::claude_utils::user_shell;
use crate::db::queries;
use crate::models::lge::LgePhaseResult;
use crate::models::Phase;
use crate::phase_runner::{AppEmitter, PhaseRunContext, PhaseRunError, PhaseRunner, RealWorktreeProvisioner};
use crate::AppState;

fn send_phase_notification(app: &tauri::AppHandle, phase: &str, task_title: &str, success: bool) {
    let phase_label = match phase {
        "planning" => "Planning",
        "builder" => "Builder",
        "review" => "Review",
        "guardian" => "Guardian",
        _ => phase,
    };
    let body = if success {
        format!("{} concluída — {}", phase_label, task_title)
    } else {
        format!("{} falhou — {}", phase_label, task_title)
    };
    let _ = app
        .notification()
        .builder()
        .title("LGE Cockpit")
        .body(&body)
        .show();
}

/// Thin Tauri adapter over `PhaseRunner`. Constructs the runner with the three
/// real port adapters, calls `run()`, sends the system notification based on
/// the outcome (Cancelled suppresses it), and returns the IPC result.
#[tauri::command]
pub async fn run_lge_phase(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    task_id: String,
    phase: Phase,
    task_title: String,
    task_description: String,
    extra_context: Option<String>,
) -> Result<LgePhaseResult, String> {
    let runner = PhaseRunner::new(
        &state.db,
        &state.planning_semaphore,
        &state.planning_cancelled,
        &state.running_pids,
        RealClaudeInvocation::new(app.clone()),
        AppEmitter::new(app.clone()),
        RealWorktreeProvisioner::new(app.clone(), state.inner()),
    );
    let title_for_notif = task_title.clone();
    let outcome = runner
        .run(PhaseRunContext {
            task_id,
            phase,
            task_title,
            task_description,
            extra_context,
        })
        .await;

    match &outcome {
        Ok(_) => send_phase_notification(&app, phase.as_str(), &title_for_notif, true),
        Err(PhaseRunError::Cancelled) => {} // user cancelled intentionally — no notification
        Err(_) => send_phase_notification(&app, phase.as_str(), &title_for_notif, false),
    }

    let outcome = outcome.map_err(|e| e.to_string())?;
    Ok(LgePhaseResult {
        phase,
        artifact_content: outcome.artifact_content,
        artifact_path: outcome.artifact_path,
    })
}



#[tauri::command]
pub fn load_lge_artifacts(
    state: State<AppState>,
    task_id: String,
) -> Result<HashMap<String, String>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;

    // Get task info
    let (repo_id, jira_key, worktree_path): (String, Option<String>, Option<String>) = conn
        .prepare("SELECT repository_id, jira_key, worktree_path FROM tasks WHERE id = ?1")
        .map_err(|e| e.to_string())?
        .query_row(rusqlite::params![task_id], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .map_err(|e| format!("Task not found: {}", e))?;

    let repo_path = queries::get_repository_path(&conn, &repo_id)?;
    let base_path = worktree_path
        .filter(|p| std::path::Path::new(p).exists())
        .unwrap_or(repo_path);
    let task_code = jira_key.unwrap_or_else(|| task_id[..8].to_string());
    let artifacts_dir = format!("{}/docs/tasks/{}", base_path, task_code);

    let mut artifacts = HashMap::new();
    // The phase list, canonical filenames, and legacy fallbacks all come from
    // the Phase module — this used to be a duplicated 4-row table that had
    // already drifted (legacy names only this site knew).
    for phase in Phase::ALL {
        let primary = format!("{}/{}", artifacts_dir, phase.artifact_filename());
        if let Ok(content) = std::fs::read_to_string(&primary) {
            artifacts.insert(phase.as_str().to_string(), content);
            continue;
        }
        for legacy in phase.legacy_filenames() {
            let legacy_path = format!("{}/{}", artifacts_dir, legacy);
            if let Ok(content) = std::fs::read_to_string(&legacy_path) {
                artifacts.insert(phase.as_str().to_string(), content);
                break;
            }
        }
    }

    Ok(artifacts)
}

#[tauri::command]
pub fn save_lge_artifact(
    state: State<AppState>,
    task_id: String,
    phase: Phase,
    content: String,
) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;

    let (repo_id, jira_key, worktree_path): (String, Option<String>, Option<String>) = conn
        .prepare("SELECT repository_id, jira_key, worktree_path FROM tasks WHERE id = ?1")
        .map_err(|e| e.to_string())?
        .query_row(rusqlite::params![task_id], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .map_err(|e| format!("Task not found: {}", e))?;

    let repo_path = queries::get_repository_path(&conn, &repo_id)?;
    let base_path = worktree_path
        .filter(|p| std::path::Path::new(p).exists())
        .unwrap_or(repo_path);
    let task_code = jira_key.unwrap_or_else(|| task_id[..8].to_string());

    let artifact_path = format!(
        "{}/docs/tasks/{}/{}",
        base_path, task_code, phase.artifact_filename()
    );

    std::fs::write(&artifact_path, content)
        .map_err(|e| format!("Failed to save artifact: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn cancel_lge_phase(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    task_id: String,
    phase: Phase,
) -> Result<(), String> {
    let pid_key = format!("{}:{}", task_id, phase);

    // If planning is queued (not yet started), mark it for cancellation
    if phase == Phase::Planning {
        if let Ok(mut cancelled) = state.planning_cancelled.lock() {
            cancelled.insert(task_id.clone());
        }
    }

    // Try to get stored PID
    let pid = {
        let mut pids = state.running_pids.lock().map_err(|e| e.to_string())?;
        pids.remove(&pid_key)
    };

    // Kill claude processes for this context — find by command pattern
    let shell = app.shell();
    let kill_cmd = if let Some(pid) = pid {
        format!("kill -TERM -{} 2>/dev/null; kill -TERM {} 2>/dev/null", pid, pid)
    } else {
        // Fallback: kill claude processes that match our pattern
        "pkill -f 'claude --print' 2>/dev/null || true".to_string()
    };

    let _ = shell
        .command(&user_shell())
        .args(["-l", "-i", "-c", &kill_cmd])
        .output()
        .await;

    Ok(())
}
