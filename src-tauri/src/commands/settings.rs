use std::collections::HashMap;
use tauri::State;

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
    conn.query_row(
        "SELECT value FROM settings WHERE key = 'shell_env'",
        [],
        |row| row.get::<_, String>(0),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_shell_env(state: State<AppState>, shell_env: String) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO settings (key, value) VALUES ('shell_env', ?1) ON CONFLICT(key) DO UPDATE SET value = ?1",
        rusqlite::params![shell_env],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_jira_base_url(state: State<AppState>) -> Result<String, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    conn.query_row(
        "SELECT value FROM settings WHERE key = 'jira_base_url'",
        [],
        |row| row.get::<_, String>(0),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_jira_base_url(state: State<AppState>, jira_base_url: String) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO settings (key, value) VALUES ('jira_base_url', ?1) ON CONFLICT(key) DO UPDATE SET value = ?1",
        rusqlite::params![jira_base_url],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
