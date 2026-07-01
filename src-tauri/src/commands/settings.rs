use std::collections::HashMap;
use tauri::State;

use crate::db::queries;
use crate::models::Phase;
use crate::AppState;

const VALID_MODELS: &[&str] = &["opus", "sonnet", "haiku"];

#[tauri::command]
pub fn get_phase_models(state: State<AppState>) -> Result<HashMap<String, String>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let mut models = HashMap::new();

    let mut stmt = conn
        .prepare("SELECT key, value FROM settings WHERE key IN (?1, ?2, ?3, ?4)")
        .map_err(|e| e.to_string())?;

    // Keys derived from the Phase module's canonical list — replaces the
    // duplicated PHASE_KEYS const (which had drifted into a 6th copy).
    let keys: [String; 4] = Phase::ALL.map(|p| format!("model_{}", p.as_str()));
    let rows = stmt
        .query_map(
            rusqlite::params![keys[0], keys[1], keys[2], keys[3]],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .map_err(|e| e.to_string())?;

    for row in rows {
        let (key, value) = row.map_err(|e| e.to_string())?;
        // Strip "model_" prefix for the frontend
        let phase = key.strip_prefix("model_").unwrap_or(&key).to_string();
        models.insert(phase, value);
    }

    Ok(models)
}

#[tauri::command]
pub fn save_phase_models(
    state: State<AppState>,
    models: HashMap<String, String>,
) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;

    for (phase, model) in &models {
        if !VALID_MODELS.contains(&model.as_str()) {
            return Err(format!("Invalid model '{}' for phase '{}'", model, phase));
        }

        // Validate the phase name via the Phase module (replaces the
        // PHASE_KEYS.contains check). Unknown phases are rejected here, at the
        // seam, rather than silently writing a dangling settings row.
        let parsed: Phase = phase.as_str().parse().map_err(|e: crate::models::ParsePhaseError| {
            format!("Unknown phase: {}", e)
        })?;

        let key = format!("model_{}", parsed.as_str());
        let affected = conn
            .execute(
                "UPDATE settings SET value = ?1 WHERE key = ?2",
                rusqlite::params![model, key],
            )
            .map_err(|e| e.to_string())?;

        if affected == 0 {
            return Err(format!("No settings row found for key: {}", key));
        }
    }

    Ok(())
}

#[tauri::command]
pub fn get_shell_env(state: State<AppState>) -> Result<String, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    queries::get_setting(&conn, "shell_env").ok_or_else(|| "shell_env is not set".to_string())
}

#[tauri::command]
pub fn save_shell_env(state: State<AppState>, shell_env: String) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    queries::set_setting(&conn, "shell_env", &shell_env)
}

#[tauri::command]
pub fn get_jira_base_url(state: State<AppState>) -> Result<String, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    Ok(queries::get_setting(&conn, "jira_base_url").unwrap_or_default())
}

#[tauri::command]
pub fn save_jira_base_url(state: State<AppState>, jira_base_url: String) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    queries::set_setting(&conn, "jira_base_url", &jira_base_url)
}

#[tauri::command]
pub fn get_jira_email(state: State<AppState>) -> Result<String, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    Ok(queries::get_setting(&conn, "jira_email").unwrap_or_default())
}

#[tauri::command]
pub fn save_jira_email(state: State<AppState>, jira_email: String) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    queries::set_setting(&conn, "jira_email", &jira_email)
}

#[tauri::command]
pub fn get_jira_api_token(state: State<AppState>) -> Result<String, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    Ok(queries::get_setting(&conn, "jira_api_token").unwrap_or_default())
}

#[tauri::command]
pub fn save_jira_api_token(state: State<AppState>, jira_api_token: String) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    queries::set_setting(&conn, "jira_api_token", &jira_api_token)
}
