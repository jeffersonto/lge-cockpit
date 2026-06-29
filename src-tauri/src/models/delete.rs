use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteTaskResult {
    pub worktree_cleaned: bool,
    pub branch_cleaned: bool,
    pub worktree_path: Option<String>,
    pub branch_name: Option<String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectDeletePreview {
    pub task_count: u32,
    pub worktree_count: u32,
    pub branch_count: u32,
}
