use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
}

impl TaskStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskStatus::Pending => "pending",
            TaskStatus::InProgress => "in_progress",
            TaskStatus::Completed => "completed",
        }
    }

    pub fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "pending" => Ok(TaskStatus::Pending),
            "in_progress" => Ok(TaskStatus::InProgress),
            "completed" => Ok(TaskStatus::Completed),
            _ => Err(format!("Invalid task status: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TaskSource {
    Manual,
    Jira,
}

impl TaskSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskSource::Manual => "manual",
            TaskSource::Jira => "jira",
        }
    }

    pub fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "manual" => Ok(TaskSource::Manual),
            "jira" => Ok(TaskSource::Jira),
            _ => Err(format!("Invalid task source: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub repository_id: String,
    pub title: String,
    pub description: Option<String>,
    pub status: TaskStatus,
    pub source: TaskSource,
    pub jira_key: Option<String>,
    pub jira_url: Option<String>,
    pub git_branch: Option<String>,
    pub worktree_path: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}
