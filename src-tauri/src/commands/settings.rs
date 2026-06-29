use std::collections::HashMap;
use tauri::State;

use crate::AppState;

const VALID_MODELS: &[&str] = &["opus", "sonnet", "haiku"];
const PHASE_KEYS: &[&str] = &[
    "model_planning",
    "model_builder",
    "model_review",
    "model_guardian",
];

#[tauri::command]
pub fn get_phase_models(state: State<AppState>) -> Result<HashMap<String, String>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let mut models = HashMap::new();

    let mut stmt = conn
        .prepare("SELECT key, value FROM settings WHERE key IN (?1, ?2, ?3, ?4)")
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map(
            rusqlite::params![
                PHASE_KEYS[0],
                PHASE_KEYS[1],
                PHASE_KEYS[2],
                PHASE_KEYS[3],
            ],
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

        let key = format!("model_{}", phase);
        if !PHASE_KEYS.contains(&key.as_str()) {
            return Err(format!("Unknown phase: {}", phase));
        }

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
