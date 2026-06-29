use chrono::Utc;
use std::path::Path;
use tauri::State;
use uuid::Uuid;

use crate::db::queries;
use crate::models::TaskAttachment;
use crate::AppState;

const VALID_PHASES: &[&str] = &["planning", "builder", "review", "guardian"];
const MAX_FILE_SIZE: u64 = 2 * 1024 * 1024; // 2MB

fn validate_phases(phases: &[String]) -> Result<(), String> {
    if phases.is_empty() {
        return Err("At least one injection phase must be selected.".to_string());
    }
    for phase in phases {
        if !VALID_PHASES.contains(&phase.as_str()) {
            return Err(format!(
                "Invalid injection phase '{}'. Must be one of: planning, builder, review, guardian",
                phase
            ));
        }
    }
    Ok(())
}

fn mime_type_for_ext(ext: &str) -> &'static str {
    match ext {
        "md" => "text/markdown",
        "txt" => "text/plain",
        "json" => "application/json",
        "csv" => "text/csv",
        "pdf" => "application/pdf",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        _ => "application/octet-stream",
    }
}

#[tauri::command]
pub fn add_task_attachment(
    state: State<AppState>,
    task_id: String,
    file_path: String,
    injection_phases: Vec<String>,
) -> Result<TaskAttachment, String> {
    validate_phases(&injection_phases)?;

    let path = Path::new(&file_path);

    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("Invalid file path")?
        .to_string();

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let supported_text = ["md", "txt", "json", "csv"];
    let supported_binary = ["pdf", "docx"];

    if !supported_text.contains(&ext.as_str()) && !supported_binary.contains(&ext.as_str()) {
        return Err(format!(
            "Unsupported file type '.{}'. Accepted: .md, .txt, .json, .csv",
            ext
        ));
    }

    if supported_binary.contains(&ext.as_str()) {
        return Err(format!(
            "PDF/DOCX text extraction not yet supported. Please convert '{}' to .md or .txt.",
            file_name
        ));
    }

    let metadata = std::fs::metadata(&file_path)
        .map_err(|e| format!("Cannot read file metadata: {}", e))?;

    if metadata.len() > MAX_FILE_SIZE {
        return Err(format!(
            "File '{}' exceeds the 2MB limit ({} bytes).",
            file_name,
            metadata.len()
        ));
    }

    let content = std::fs::read_to_string(&file_path)
        .map_err(|e| format!("Cannot read file '{}': {}", file_name, e))?;

    let mime_type = mime_type_for_ext(&ext).to_string();

    let attachment = TaskAttachment {
        id: Uuid::new_v4().to_string(),
        task_id,
        file_name,
        file_size: metadata.len() as i64,
        mime_type,
        content,
        injection_phases,
        created_at: Utc::now().to_rfc3339(),
    };

    let conn = state.db.lock().map_err(|e| e.to_string())?;
    queries::insert_attachment(&conn, &attachment)?;

    Ok(attachment)
}

#[tauri::command]
pub fn list_task_attachments(
    state: State<AppState>,
    task_id: String,
) -> Result<Vec<TaskAttachment>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    queries::list_attachments_by_task(&conn, &task_id)
}

#[tauri::command]
pub fn remove_task_attachment(
    state: State<AppState>,
    attachment_id: String,
) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    queries::delete_attachment(&conn, &attachment_id)
}

#[tauri::command]
pub fn set_attachment_phases(
    state: State<AppState>,
    attachment_id: String,
    injection_phases: Vec<String>,
) -> Result<(), String> {
    validate_phases(&injection_phases)?;
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    queries::set_attachment_phases(&conn, &attachment_id, &injection_phases)
}
