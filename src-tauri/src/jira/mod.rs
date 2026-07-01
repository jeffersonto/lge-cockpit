//! Jira integration — direct Jira Cloud REST API client.
//!
//! This module owns all communication with the Jira Cloud REST API and
//! isolates HTTP, authentication, and Atlassian-specific formats from the
//! rest of the application. The command layer (`commands/jira.rs`) is a
//! thin adapter that reads settings, calls the port, and creates a `Task`.
//!
//! See ADR 001 for the decision to replace the MCP-based integration.

mod client;
mod converter;

pub use client::{JiraClient, ReqwestJiraClient};

use crate::db::queries;
use crate::models::TaskStatus;

/// Jira Cloud connection parameters read from the app's settings table.
/// All three fields are required for any API call to succeed.
#[derive(Debug, Clone)]
pub struct JiraConfig {
    /// Atlassian Cloud site URL, e.g. `https://yourcompany.atlassian.net`.
    /// Stored as `jira_base_url` in the settings table.
    pub base_url: String,
    /// The user's Atlassian account email. Stored as `jira_email`.
    pub email: String,
    /// An API token generated in the Atlassian account settings.
    /// Stored as `jira_api_token`.
    pub api_token: String,
}

impl JiraConfig {
    /// Returns the base URL with any trailing slash removed.
    pub fn base_url_trimmed(&self) -> &str {
        self.base_url.trim().trim_end_matches('/')
    }

    /// Returns `true` when all three credential fields are non-empty.
    pub fn is_complete(&self) -> bool {
        !self.base_url.trim().is_empty()
            && !self.email.trim().is_empty()
            && !self.api_token.trim().is_empty()
    }
}

/// Reads the Jira connection params (base URL, email, API token) from the
/// settings table. All three fields are returned even when empty — the
/// client constructor turns a missing trio into a clear `NotConfigured`
/// error. Single source of truth shared by every call site that needs the
/// configured Jira credentials.
pub fn read_jira_config(conn: &rusqlite::Connection) -> JiraConfig {
    JiraConfig {
        base_url: queries::get_setting(conn, "jira_base_url").unwrap_or_default(),
        email: queries::get_setting(conn, "jira_email").unwrap_or_default(),
        api_token: queries::get_setting(conn, "jira_api_token").unwrap_or_default(),
    }
}

/// A fetched Jira issue, in display-ready form. The description is already
/// converted from Atlassian's rendered HTML to GitHub-Flavored Markdown.
#[derive(Debug, Clone)]
pub struct JiraIssue {
    pub key: String,
    pub summary: String,
    pub description: Option<String>,
    pub status: Option<String>,
    pub url: String,
}

impl JiraIssue {
    /// Maps the Jira status name to a local `TaskStatus`. Unknown statuses
    /// default to `Pending`.
    pub fn to_task_status(&self) -> TaskStatus {
        match self.status.as_deref().map(|s| s.to_lowercase()).as_deref() {
            Some("done") | Some("closed") | Some("resolved") | Some("concluído") => {
                TaskStatus::Completed
            }
            Some("in progress")
            | Some("in review")
            | Some("in development")
            | Some("em andamento")
            | Some("em progresso") => TaskStatus::InProgress,
            _ => TaskStatus::Pending,
        }
    }
}

/// Identity returned by a successful connection test (`GET /rest/api/3/myself`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct JiraSelf {
    pub account_id: String,
    pub display_name: String,
    pub email: String,
}

/// Structured errors raised by the Jira client. The command adapter maps
/// each variant to a user-friendly message at the Tauri boundary.
#[derive(Debug, thiserror::Error, Clone)]
pub enum JiraError {
    #[error("Jira credentials are not configured. Set email, API token, and base URL in Settings.")]
    NotConfigured,
    #[error("Invalid Jira issue key: {0}")]
    InvalidKey(String),
    #[error(
        "Jira base URL must start with \"http://\" or \"https://\" (got {0}). Update it in Settings."
    )]
    InvalidBaseUrl(String),
    #[error("Jira authentication failed — verify the email and API token in Settings.")]
    Unauthorized,
    #[error("Jira issue not found: {0}")]
    NotFound(String),
    #[error("Jira rate limit reached. Try again in a moment.")]
    RateLimited,
    #[error("Jira returned an unexpected status ({status}): {body}")]
    UnexpectedStatus { status: u16, body: String },
    #[error("Network error contacting Jira: {0}")]
    Network(String),
    #[error("Could not parse the Jira response: {0}")]
    Parse(String),
}
