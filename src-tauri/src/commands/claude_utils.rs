/// Returns the user's login shell (e.g. "/bin/zsh", "/bin/bash").
/// Falls back to "sh" if SHELL is not set.
/// Used with `-l -i -c` flags so subprocesses load the full user
/// environment (.zshrc/.bashrc) — including gvm, nvm, pyenv, etc.
pub fn user_shell() -> String {
    std::env::var("SHELL").unwrap_or_else(|_| "sh".to_string())
}

/// Reads the user's custom shell environment commands from the DB
/// and returns them as a shell prefix (each line terminated with `;`).
/// Returns empty string if no custom commands are set.
pub fn shell_env_prefix(conn: &rusqlite::Connection) -> String {
    let raw: String = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'shell_env'",
            [],
            |row| row.get(0),
        )
        .unwrap_or_default();

    if raw.trim().is_empty() {
        return String::new();
    }

    let commands: Vec<&str> = raw
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect();

    if commands.is_empty() {
        String::new()
    } else {
        format!("{}; ", commands.join("; "))
    }
}

pub fn resolve_claude_path() -> String {
    let home = std::env::var("HOME").unwrap_or_default();
    let candidates = [
        format!("{}/.local/bin/claude", home),
        format!("{}/.cargo/bin/claude", home),
        "/usr/local/bin/claude".to_string(),
        "/opt/homebrew/bin/claude".to_string(),
    ];

    for path in &candidates {
        if std::path::Path::new(path).exists() {
            return path.clone();
        }
    }

    "claude".to_string()
}

pub fn shell_escape(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

pub fn resolve_npx_path() -> String {
    let home = std::env::var("HOME").unwrap_or_default();
    // Check NVM versions (most common on macOS dev machines)
    let nvm_base = format!("{}/.nvm/versions/node", home);
    if let Ok(entries) = std::fs::read_dir(&nvm_base) {
        let mut versions: Vec<String> = entries
            .flatten()
            .filter_map(|e| e.file_name().into_string().ok())
            .collect();
        versions.sort();
        if let Some(latest) = versions.last() {
            let candidate = format!("{}/{}/bin/npx", nvm_base, latest);
            if std::path::Path::new(&candidate).exists() {
                return candidate;
            }
        }
    }
    // Common fixed locations
    for path in &[
        "/opt/homebrew/bin/npx",
        "/usr/local/bin/npx",
    ] {
        if std::path::Path::new(path).exists() {
            return path.to_string();
        }
    }
    "npx".to_string()
}

/// Result of looking for an existing Atlassian MCP in the user's Claude
/// configuration. We don't probe live connectivity (that requires
/// `claude mcp list`, which is slow and times out when other MCPs are
/// unreachable) — being configured is enough for both the health badge
/// and the import flow.
#[derive(Debug, Clone, Default)]
pub struct AtlassianMcpDetection {
    pub configured: bool,
    pub server_name: Option<String>,
}

/// Collects `.mcp.json` files from any installed Claude marketplace plugin
/// named `jira-story-comment`, regardless of which marketplace published it.
/// Scans `~/.claude/plugins/marketplaces/*/plugins/jira-story-comment/.mcp.json`.
/// File-system only — never spawns the Claude CLI.
fn marketplace_plugin_mcp_configs() -> Vec<String> {
    let home = std::env::var("HOME").unwrap_or_default();
    let marketplaces_dir = format!("{}/.claude/plugins/marketplaces", home);
    let mut paths = Vec::new();
    let Ok(entries) = std::fs::read_dir(&marketplaces_dir) else {
        return paths;
    };
    for entry in entries.flatten() {
        let plugin_mcp = entry
            .path()
            .join("plugins")
            .join("jira-story-comment")
            .join(".mcp.json");
        if plugin_mcp.is_file() {
            if let Some(p) = plugin_mcp.to_str() {
                paths.push(p.to_string());
            }
        }
    }
    paths.sort();
    paths
}

/// Looks for any `*atlassian*` server in the user's Claude MCP configuration.
/// Sources, in order:
///   1. `~/.claude.json` top-level `mcpServers` (where `claude mcp add`
///      stores user-scope servers).
///   2. `~/.claude/mcp/atlassian.json` (cockpit's bundled user path).
///   3. Any `~/.claude/plugins/marketplaces/.../jira-story-comment/.mcp.json`
///      (installed marketplace plugins).
///
/// Returns the first match found, preferring shorter/canonical names.
/// File-only — never spawns the Claude CLI, so it stays fast even when the
/// user has many failing MCPs.
pub fn detect_atlassian_mcp() -> AtlassianMcpDetection {
    let home = std::env::var("HOME").unwrap_or_default();
    let mut candidates = vec![
        format!("{}/.claude.json", home),
        format!("{}/.claude/mcp/atlassian.json", home),
    ];
    candidates.extend(marketplace_plugin_mcp_configs());

    for path in &candidates {
        let Ok(content) = std::fs::read_to_string(path) else { continue };
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) else { continue };
        if let Some(name) = find_atlassian_server_name(&value, path.ends_with(".claude.json")) {
            return AtlassianMcpDetection {
                configured: true,
                server_name: Some(name),
            };
        }
    }

    AtlassianMcpDetection::default()
}

/// Walks a Claude config JSON value looking for an `mcpServers` object that
/// contains an `*atlassian*` key. When `top_level_only` is true (used for
/// `~/.claude.json`), only the root `mcpServers` is inspected — this matches
/// what Claude actually loads when invoked outside any project directory
/// (the cockpit runs `claude` from `/tmp`).
fn find_atlassian_server_name(value: &serde_json::Value, top_level_only: bool) -> Option<String> {
    if let Some(servers) = value.get("mcpServers").and_then(|s| s.as_object()) {
        for key in servers.keys() {
            if key.to_lowercase().contains("atlassian") {
                return Some(key.clone());
            }
        }
    }
    if top_level_only {
        return None;
    }
    value
        .as_object()
        .and_then(|obj| {
            obj.values()
                .find_map(|v| find_atlassian_server_name(v, false))
        })
}

pub fn resolve_mcp_config() -> String {
    let home = std::env::var("HOME").unwrap_or_default();
    // Priority 1: any installed marketplace plugin (already authenticated)
    if let Some(path) = marketplace_plugin_mcp_configs().into_iter().next() {
        return path;
    }
    // Priority 2: user-created config
    let user_path = format!("{}/.claude/mcp/atlassian.json", home);
    if std::path::Path::new(&user_path).exists() {
        return user_path;
    }
    // Priority 3: generate fallback config using resolved npx path
    let npx = resolve_npx_path();
    let tmp_config = "/tmp/lge-cockpit-mcp.json";
    let config_content = format!(
        r#"{{"mcpServers":{{"atlassian-mcp":{{"type":"stdio","command":"{}","args":["-y","mcp-remote","https://mcp.atlassian.com/v1/sse"]}}}}}}"#,
        npx
    );
    let _ = std::fs::write(tmp_config, &config_content);
    tmp_config.to_string()
}
