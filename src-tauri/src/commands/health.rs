use serde::{Deserialize, Serialize};
use tauri::State;
use tauri_plugin_shell::ShellExt;

use crate::commands::claude_utils::{resolve_claude_path, user_shell};
use crate::jira;
use crate::AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyStatus {
    pub name: String,
    pub available: bool,
    pub path: Option<String>,
    pub version: Option<String>,
    pub install_command: Option<String>,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    pub all_ok: bool,
    pub dependencies: Vec<DependencyStatus>,
}

#[tauri::command]
pub async fn check_dependencies(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<HealthCheckResult, String> {
    let mut deps = Vec::new();

    // 1. Check Git
    let git_version = get_command_version(&app, "git", &["--version"]).await;
    let git_available = git_version.is_some();

    deps.push(DependencyStatus {
        name: "Git".to_string(),
        available: git_available,
        path: if git_available { Some("git".to_string()) } else { None },
        version: git_version,
        install_command: Some("https://git-scm.com/downloads".to_string()),
        description: "Required for branch creation, worktrees, and version control".to_string(),
    });

    // 2. Check Claude CLI (still required for the LGE phase runner).
    let claude_path = resolve_claude_path();
    let claude_exists = std::path::Path::new(&claude_path).exists() && claude_path != "claude";

    let claude_version = if claude_exists {
        get_command_version(&app, &claude_path, &["--version"]).await
    } else {
        None
    };

    deps.push(DependencyStatus {
        name: "Claude CLI".to_string(),
        available: claude_exists,
        path: if claude_exists { Some(claude_path.clone()) } else { None },
        version: claude_version,
        install_command: Some("npm install -g @anthropic-ai/claude-code".to_string()),
        description: "Required for LGE process execution".to_string(),
    });

    // 3. Check Jira credentials configuration. This is a config check, not a
    // live connection test — the "Test connection" button in Settings does
    // the live verify via `verify_jira_connection`.
    let jira_configured = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        jira::read_jira_config(&conn).is_complete()
    };

    deps.push(DependencyStatus {
        name: "Jira credentials".to_string(),
        available: jira_configured,
        path: if jira_configured {
            Some("Settings → Jira".to_string())
        } else {
            None
        },
        version: None,
        install_command: if jira_configured {
            None
        } else {
            Some("Open Settings → Jira: set base URL, email, and API token".to_string())
        },
        description: "Required for importing tasks from Jira".to_string(),
    });

    let all_ok = deps.iter().all(|d| d.available);

    Ok(HealthCheckResult {
        all_ok,
        dependencies: deps,
    })
}

async fn get_command_version(app: &tauri::AppHandle, cmd: &str, args: &[&str]) -> Option<String> {
    let shell = app.shell();
    let output = shell
        .command(user_shell())
        .args(["-l", "-i", "-c", &format!("{} {}", cmd, args.join(" "))])
        .output()
        .await
        .ok()?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        Some(stdout.trim().lines().next()?.to_string())
    } else {
        None
    }
}
