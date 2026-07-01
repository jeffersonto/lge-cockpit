use chrono::Utc;
use tauri::State;
use uuid::Uuid;

use crate::db::queries;
use crate::jira::{JiraClient, JiraConfig, JiraError, JiraSelf, ReqwestJiraClient};
use crate::models::{Task, TaskSource};
use crate::AppState;

/// Reads the Jira connection params (base URL, email, API token) from the
/// settings table. All three are returned even when empty — the client
/// constructor turns a missing trio into a clear `NotConfigured` error.
fn read_jira_config(conn: &rusqlite::Connection) -> JiraConfig {
    let get = |key: &str| -> String {
        conn.query_row(
            "SELECT value FROM settings WHERE key = ?1",
            rusqlite::params![key],
            |row| row.get::<_, String>(0),
        )
        .unwrap_or_default()
        .trim()
        .to_string()
    };
    JiraConfig {
        base_url: get("jira_base_url"),
        email: get("jira_email"),
        api_token: get("jira_api_token"),
    }
}

/// Imports a Jira Cloud issue by key and creates a local `Task` from it.
/// The description is converted from Atlassian's rendered HTML to Markdown.
#[tauri::command]
pub async fn import_jira_task(
    state: State<'_, AppState>,
    repository_id: String,
    jira_key: String,
) -> Result<Task, String> {
    let jira_key = jira_key.trim().to_uppercase();
    if jira_key.is_empty() {
        return Err("Informe a chave da issue do Jira (ex: PROJ-123).".to_string());
    }

    let config = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        // Validate the repository up front for a clear error.
        let _ = queries::get_repository_path(&conn, &repository_id)?;
        read_jira_config(&conn)
    };

    let client = ReqwestJiraClient::new(config).map_err(|e| e.to_string())?;
    let issue = client
        .get_issue(&jira_key)
        .await
        .map_err(|e| e.to_string())?;

    let now = Utc::now().to_rfc3339();
    let status = issue.to_task_status();
    let task = Task {
        id: Uuid::new_v4().to_string(),
        repository_id,
        title: issue.summary,
        description: issue.description,
        status,
        source: TaskSource::Jira,
        jira_key: Some(issue.key),
        jira_url: Some(issue.url),
        git_branch: None,
        worktree_path: None,
        created_at: now.clone(),
        updated_at: now,
    };

    let conn = state.db.lock().map_err(|e| e.to_string())?;
    queries::insert_task(&conn, &task)?;

    Ok(task)
}

/// Verifies the configured Jira credentials by calling `GET /myself`.
/// Used by the Settings screen's "Test connection" button.
#[tauri::command]
pub async fn verify_jira_connection(state: State<'_, AppState>) -> Result<JiraSelf, String> {
    let config = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        read_jira_config(&conn)
    };
    let client = ReqwestJiraClient::new(config).map_err(|e| e.to_string())?;
    client
        .verify_connection()
        .await
        .map_err(|e: JiraError| e.to_string())
}