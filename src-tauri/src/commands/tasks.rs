use chrono::Utc;
use tauri::State;
use uuid::Uuid;

use crate::db::queries;
use crate::models::{Task, TaskSource, TaskStatus};
use crate::AppState;

#[tauri::command]
pub fn list_tasks(state: State<AppState>, repository_id: String) -> Result<Vec<Task>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    queries::list_tasks_by_repo(&conn, &repository_id)
}

#[tauri::command]
pub fn create_task(
    state: State<AppState>,
    repository_id: String,
    title: String,
    description: Option<String>,
) -> Result<Task, String> {
    let now = Utc::now().to_rfc3339();
    let task = Task {
        id: Uuid::new_v4().to_string(),
        repository_id,
        title,
        description,
        status: TaskStatus::Pending,
        source: TaskSource::Manual,
        jira_key: None,
        jira_url: None,
        git_branch: None,
        worktree_path: None,
        created_at: now.clone(),
        updated_at: now,
    };

    let conn = state.db.lock().map_err(|e| e.to_string())?;
    queries::insert_task(&conn, &task)?;

    Ok(task)
}

#[tauri::command]
pub fn update_task_status(
    state: State<AppState>,
    id: String,
    status: String,
) -> Result<Task, String> {
    // Validate status
    TaskStatus::from_str(&status)?;
    let now = Utc::now().to_rfc3339();

    let conn = state.db.lock().map_err(|e| e.to_string())?;
    queries::update_task_status(&conn, &id, &status, &now)
}

#[tauri::command]
pub fn update_task(
    state: State<AppState>,
    id: String,
    title: String,
    description: Option<String>,
) -> Result<Task, String> {
    let now = Utc::now().to_rfc3339();
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    queries::update_task_content(&conn, &id, &title, description.as_deref(), &now)
}

#[tauri::command]
pub async fn delete_task(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<crate::models::DeleteTaskResult, String> {
    let (worktree_path, git_branch, repo_path) = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        queries::get_task_cleanup_info(&conn, &id)?
    };

    let env_prefix = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        crate::settings::shell_env(&conn).prefix().to_string()
    };

    let mut worktree_cleaned = false;
    let mut branch_cleaned = false;
    let mut errors: Vec<String> = Vec::new();

    if let Some(ref wt_path) = worktree_path {
        match crate::commands::git::do_remove_worktree_from_disk(&app, &repo_path, wt_path, &env_prefix).await {
            Ok(()) => worktree_cleaned = true,
            Err(e) => errors.push(format!("Worktree: {}", e)),
        }
    }

    if let Some(ref branch) = git_branch {
        match crate::commands::git::do_delete_branch(&app, &repo_path, branch, &env_prefix).await {
            Ok(()) => branch_cleaned = true,
            Err(e) => errors.push(format!("Branch: {}", e)),
        }
    }

    {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        queries::delete_task(&conn, &id)?;
    }

    Ok(crate::models::DeleteTaskResult {
        worktree_cleaned,
        branch_cleaned,
        worktree_path,
        branch_name: git_branch,
        errors,
    })
}
