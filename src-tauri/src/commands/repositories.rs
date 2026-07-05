use std::path::Path;

use chrono::Utc;
use tauri::State;
use uuid::Uuid;

use crate::db::queries;
use crate::models::Repository;
use crate::AppState;

#[tauri::command]
pub fn add_repository(state: State<AppState>, path: String) -> Result<Repository, String> {
    let p = Path::new(&path);
    if !p.exists() || !p.is_dir() {
        return Err("Path does not exist or is not a directory".to_string());
    }

    let name = p
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.clone());

    let now = Utc::now().to_rfc3339();
    let repo = Repository {
        id: Uuid::new_v4().to_string(),
        name,
        path,
        created_at: now.clone(),
        updated_at: now,
        max_worktrees: 5,
        active_worktree_count: 0,
    };

    let conn = state.db.lock().map_err(|e| e.to_string())?;
    queries::insert_repository(&conn, &repo)?;

    Ok(repo)
}

#[tauri::command]
pub fn list_repositories(state: State<AppState>) -> Result<Vec<Repository>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    queries::list_repositories(&conn)
}

#[tauri::command]
pub async fn remove_repository(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let (repo_path, tasks_to_clean, env_prefix) = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        let repo_path = queries::get_repository_path(&conn, &id)?;
        let tasks = queries::get_tasks_for_repo_cleanup(&conn, &id)?;
        let env_prefix = crate::settings::shell_env(&conn).prefix().to_string();
        (repo_path, tasks, env_prefix)
    };

    for (_task_id, worktree_path, git_branch) in &tasks_to_clean {
        if let Some(wt_path) = worktree_path {
            let _ = crate::commands::git::do_remove_worktree_from_disk(&app, &repo_path, wt_path, &env_prefix).await;
        }
        if let Some(branch) = git_branch {
            let _ = crate::commands::git::do_delete_branch(&app, &repo_path, branch, &env_prefix).await;
        }
    }

    {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        queries::delete_repository(&conn, &id)?;
    }

    Ok(())
}

#[tauri::command]
pub fn get_project_delete_preview(
    state: State<AppState>,
    id: String,
) -> Result<crate::models::ProjectDeletePreview, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    queries::get_project_delete_preview(&conn, &id)
}
