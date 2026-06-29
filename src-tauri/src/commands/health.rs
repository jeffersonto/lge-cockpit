use serde::{Deserialize, Serialize};
use tauri_plugin_shell::ShellExt;

use crate::commands::claude_utils::{detect_atlassian_mcp, resolve_claude_path, resolve_mcp_config, resolve_npx_path, user_shell};

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
pub async fn check_dependencies(app: tauri::AppHandle) -> Result<HealthCheckResult, String> {
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

    // 2. Check Claude CLI
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
        description: "Required for LGE process execution and Jira import".to_string(),
    });

    // 3. Check Atlassian MCP availability.
    //
    // Two independent sources count as "available":
    //   (a) Detection finds a `*atlassian*` server in the user's Claude
    //       config (`~/.claude.json` or the bundled file paths). Covers
    //       servers added via `claude mcp add` (e.g. `atlassian: uvx
    //       mcp-atlassian`).
    //   (b) The bundled file-based config exists AND `npx` is reachable —
    //       the original flow, kept for backward compatibility.
    let detection = detect_atlassian_mcp();

    let mcp_config_path = resolve_mcp_config();
    let mcp_file_valid = std::path::Path::new(&mcp_config_path)
        .exists()
        .then(|| {
            std::fs::read_to_string(&mcp_config_path)
                .map(|content| content.contains("atlassian"))
                .unwrap_or(false)
        })
        .unwrap_or(false);

    // npx is required to start the bundled `atlassian-mcp` server via mcp-remote
    let npx_path = resolve_npx_path();
    let npx_available = npx_path != "npx" || std::process::Command::new("npx").arg("--version").output().map(|o| o.status.success()).unwrap_or(false);

    let file_path_available = mcp_file_valid && npx_available;
    let available = detection.configured || file_path_available;

    let path_label = if detection.configured {
        detection.server_name.clone().map(|n| format!("claude mcp: {}", n))
    } else if mcp_file_valid {
        Some(mcp_config_path.clone())
    } else {
        None
    };

    let home = std::env::var("HOME").unwrap_or_default();
    let npx_abs = resolve_npx_path();
    let install_cmd = format!(
        "mkdir -p ~/.claude/mcp && echo '{{\"mcpServers\":{{\"atlassian-mcp\":{{\"type\":\"stdio\",\"command\":\"{npx}\",\"args\":[\"-y\",\"mcp-remote\",\"https://mcp.atlassian.com/v1/sse\"]}}}}}}' > {home}/.claude/mcp/atlassian.json",
        npx = if npx_abs != "npx" { npx_abs } else { "npx".to_string() },
        home = home
    );

    deps.push(DependencyStatus {
        name: "Atlassian MCP".to_string(),
        available,
        path: path_label,
        version: None,
        install_command: Some(install_cmd),
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
        .command(&user_shell())
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
