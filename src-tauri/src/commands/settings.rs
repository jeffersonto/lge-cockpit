use std::collections::HashMap;
use tauri::State;

use crate::jira::JiraConfig;
use crate::settings;
use crate::AppState;

/// Returns the four phase model overrides, fully resolved with each phase's
/// static default for any missing/empty row. The frontend never needs its own
/// defaults — this is the single source of truth.
#[tauri::command]
pub fn get_phase_models(state: State<AppState>) -> Result<HashMap<String, String>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    Ok(settings::phase_models(&conn)
        .into_iter()
        .map(|(p, m)| (p.as_str().to_string(), m))
        .collect())
}

/// Persists phase model overrides. Unknown phases and invalid model values
/// are rejected here at the seam before any write reaches the DB.
#[tauri::command]
pub fn save_phase_models(
    state: State<AppState>,
    models: HashMap<String, String>,
) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    settings::save_phase_models(&conn, &models)
}

/// Reads the raw `shell_env` setting value for the UI to display and edit.
#[tauri::command]
pub fn get_shell_env(state: State<AppState>) -> Result<String, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    Ok(settings::shell_env_raw(&conn))
}

/// Persists the raw `shell_env` setting value (as edited by the UI).
#[tauri::command]
pub fn save_shell_env(state: State<AppState>, shell_env: String) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    settings::save_shell_env(&conn, &shell_env)
}

/// Returns the configured Jira credentials. All three fields default to
/// empty strings when unset; the Jira client turns a missing trio into a
/// clear `NotConfigured` error.
#[tauri::command]
pub fn get_jira_config(state: State<AppState>) -> Result<JiraConfig, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    Ok(settings::jira_config(&conn))
}

/// Persists the three Jira credential fields as a single IPC call. Replaces
/// the prior three per-key `save_jira_*` commands.
#[tauri::command]
pub fn save_jira_config(state: State<AppState>, config: JiraConfig) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    settings::save_jira(&conn, &config)
}