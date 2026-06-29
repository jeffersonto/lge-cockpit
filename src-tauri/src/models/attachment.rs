use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskAttachment {
    pub id: String,
    pub task_id: String,
    pub file_name: String,
    pub file_size: i64,
    pub mime_type: String,
    pub content: String,
    pub injection_phases: Vec<String>,
    pub created_at: String,
}
