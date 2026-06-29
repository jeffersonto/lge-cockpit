use chrono::Utc;
use tauri::State;
use tauri_plugin_shell::ShellExt;
use uuid::Uuid;

use crate::commands::claude_utils::{detect_atlassian_mcp, resolve_claude_path, resolve_mcp_config, shell_env_prefix, shell_escape, user_shell};
use crate::db::queries;
use crate::models::{Task, TaskSource, TaskStatus};
use crate::AppState;

#[derive(serde::Deserialize, Default)]
struct JiraIssueData {
    summary: Option<String>,
    description: Option<String>,
    status: Option<String>,
    url: Option<String>,
}

#[tauri::command]
pub async fn import_jira_task(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    repository_id: String,
    jira_key: String,
) -> Result<Task, String> {
    // Get the repository path and Jira base URL (if configured) for CLI context
    let (repo_path, env_prefix, jira_base_url) = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        let repo = queries::get_repository_path(&conn, &repository_id)?;
        let env = shell_env_prefix(&conn);
        let base = conn
            .query_row(
                "SELECT value FROM settings WHERE key = 'jira_base_url'",
                [],
                |row| row.get::<_, String>(0),
            )
            .unwrap_or_default()
            .trim()
            .to_string();
        (repo, env, base)
    };

    // Try to fetch Jira data via Claude Code CLI + MCP Atlassian
    let jira_data = fetch_jira_via_mcp(&app, &repo_path, &jira_key, &env_prefix, &jira_base_url).await;

    let now = Utc::now().to_rfc3339();
    let task = match jira_data {
        Ok(data) => Task {
            id: Uuid::new_v4().to_string(),
            repository_id,
            title: data
                .summary
                .unwrap_or_else(|| format!("[{}]", jira_key)),
            description: data.description,
            status: map_jira_status(data.status.as_deref()),
            source: TaskSource::Jira,
            jira_key: Some(jira_key),
            jira_url: data.url,
            git_branch: None,
            worktree_path: None,
            created_at: now.clone(),
            updated_at: now,
        },
        Err(err) => {
            // Surface OAuth requirement to the frontend instead of creating a
            // placeholder task — the user needs to authenticate before any
            // import will succeed.
            if err.starts_with(AUTH_REQUIRED_PREFIX) {
                return Err(err);
            }
            log::warn!("Failed to fetch Jira data via MCP, using fallback: {}", err);
            Task {
                id: Uuid::new_v4().to_string(),
                repository_id,
                title: format!("[{}] Imported from Jira", jira_key),
                description: Some(format!(
                    "Task imported from Jira issue {}.\n\nNote: Could not fetch details automatically: {}",
                    jira_key, err
                )),
                status: TaskStatus::Pending,
                source: TaskSource::Jira,
                jira_key: Some(jira_key),
                jira_url: None,
                git_branch: None,
                worktree_path: None,
                created_at: now.clone(),
                updated_at: now,
            }
        }
    };

    let conn = state.db.lock().map_err(|e| e.to_string())?;
    queries::insert_task(&conn, &task)?;

    Ok(task)
}

async fn fetch_jira_via_mcp(
    app: &tauri::AppHandle,
    _repo_path: &str,
    jira_key: &str,
    env_prefix: &str,
    jira_base_url: &str,
) -> Result<JiraIssueData, String> {
    let shell = app.shell();
    let claude_bin = resolve_claude_path();

    // Detect any pre-existing Atlassian MCP in the user's Claude config.
    // When present we use the user's setup directly and skip --mcp-config
    // entirely: injecting our bundled `atlassian-mcp` (npx mcp-remote)
    // would force Claude through its OAuth flow even though the user's
    // own server is already authenticated.
    let detection = detect_atlassian_mcp();
    let user_server = detection.server_name.as_deref().filter(|n| !n.contains(' '));

    // The description prompt is shared between branches. It is deliberately
    // strict about verbatim transcription — Claude has a habit of "helpfully"
    // summarizing long ADF descriptions when asked to convert to text, which
    // loses tables, code blocks, and bullet lists.
    let description_rules = r#"Convert the description from ADF to GitHub-Flavored Markdown VERBATIM. Do NOT summarize, paraphrase, abbreviate, condense, or omit any content. Preserve every paragraph, table row, code block, file path, list item, link, and heading exactly as authored. Map ADF nodes faithfully: heading→`#`/`##`/...; paragraph→plain line; bulletList→`- `; orderedList→`1. `; codeBlock→fenced ```; inlineCode→backticks; table→markdown table with header separator; rule→`---`; hardBreak→newline; mention→`@name`; link→`[text](url)`; emphasis/strong→`*`/`**`. Keep the original ordering. If the description is long, include it IN FULL — truncating or summarizing is a failure."#;

    // When the user has configured a Jira base URL in Settings, we can build
    // the browse URL deterministically. Otherwise, ask Claude to extract or
    // derive it from the MCP response — no hardcoded tenant.
    let url_clause = if jira_base_url.is_empty() {
        r#"the issue's browse URL (look for a self/issueLink/browse URL in the response; if none is found, derive it from the Atlassian cloud site the MCP is configured for)"#.to_string()
    } else {
        format!(r#"the issue's browse URL, which MUST be "{base}/browse/{key}" (base = the configured Jira site URL)"#, base = jira_base_url.trim_end_matches('/'), key = jira_key)
    };

    let prompt = if let Some(name) = user_server {
        format!(
            r#"Fetch Jira issue {jira_key} using the configured Atlassian MCP server "{name}". Try, in order, whichever tool exists: mcp__{name}__getJiraIssue, mcp__{name}__jira_get_issue. Pass the issue key in the tool's expected parameter (issueIdOrKey or issue_key). Extract: summary (issue title), description, status (status name), and {url_clause}. {description_rules} Return ONLY a JSON object with this exact shape: {{"summary":"...","description":"...","status":"...","url":"..."}}. The JSON must be the entire output — no markdown wrapping, no code fences, no commentary before or after."#,
            jira_key = jira_key,
            name = name,
            url_clause = url_clause,
            description_rules = description_rules
        )
    } else {
        format!(
            r#"Call mcp__atlassian-mcp__getJiraIssue with issueIdOrKey="{jira_key}". From the response extract fields.summary, fields.description, fields.status.name, and {url_clause}. {description_rules} Return ONLY a JSON object: {{"summary":"...","description":"...","status":"...","url":"..."}}. The JSON must be the entire output — no markdown wrapping, no code fences, no commentary."#,
            jira_key = jira_key,
            url_clause = url_clause,
            description_rules = description_rules
        )
    };

    // Export PATH to include common node/npx locations (nvm, homebrew,
    // local) so mcp-remote can be found when we fall back to the bundled
    // Atlassian MCP. Harmless when the user's MCP doesn't need npx.
    let home = std::env::var("HOME").unwrap_or_default();
    let path_export = format!(
        "export PATH=\"{home}/.nvm/versions/node/$(ls {home}/.nvm/versions/node/ 2>/dev/null | sort -V | tail -1)/bin:/opt/homebrew/bin:/usr/local/bin:$PATH\"",
        home = home
    );

    let (mcp_arg, allowed_tools_csv) = if let Some(name) = user_server {
        // Use user's existing MCP — no --mcp-config injection.
        let tools = format!(
            "mcp__{name}__getJiraIssue,mcp__{name}__jira_get_issue,Bash",
            name = name
        );
        (String::new(), tools)
    } else {
        // No user MCP detected — inject the bundled config. Tool list
        // mirrors the original cockpit behavior.
        let mcp_config = resolve_mcp_config();
        let tools = "mcp__atlassian-mcp__getJiraIssue,Bash".to_string();
        (
            format!(" --mcp-config {}", shell_escape(&mcp_config)),
            tools,
        )
    };

    // Run from /tmp to prevent Claude from reading project files.
    let full_cmd = format!(
        "{env_prefix}{path_export}; cd /tmp && echo {prompt} | {claude} --print --output-format json --max-turns 10{mcp_arg} --allowedTools {tools}",
        env_prefix = env_prefix,
        path_export = path_export,
        prompt = shell_escape(&prompt),
        claude = claude_bin,
        mcp_arg = mcp_arg,
        tools = shell_escape(&allowed_tools_csv),
    );

    let output = shell
        .command(&user_shell())
        .args(["-l", "-i", "-c", &full_cmd])
        .output()
        .await
        .map_err(|e| format!("Failed to invoke Claude CLI: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // When the CLI exits non-zero but produced output, attempt to parse it
    // anyway — the MCP may have returned an OAuth redirect (which some
    // versions surface as a non-zero exit) or a parseable error embedded in
    // the JSON wrapper. Only bail out if stdout is also empty.
    if !output.status.success() {
        if stdout.trim().is_empty() {
            // Include stdout in the message because Claude often writes the
            // actual error there, leaving stderr empty.
            let detail = if stderr.trim().is_empty() {
                format!("exit code {:?}", output.status.code())
            } else {
                stderr.to_string()
            };
            return Err(format!("Claude CLI error: {}", detail));
        }
        // Non-empty stdout even on failure — fall through to parse_claude_response
        // so OAuth and other structured responses are handled correctly.
    }

    if stdout.trim().is_empty() {
        let detail = if stderr.trim().is_empty() {
            format!("exit code {:?}", output.status.code())
        } else {
            stderr.to_string()
        };
        return Err(format!("Claude CLI returned empty output. stderr: {}", detail));
    }

    parse_claude_response(&stdout)
}

/// Sentinel returned when the Atlassian MCP responded with an OAuth
/// authorization request instead of issue data. The frontend matches on
/// this prefix to surface an "Authenticate" dialog with a clickable link
/// instead of the generic parse error.
pub const AUTH_REQUIRED_PREFIX: &str = "ATLASSIAN_AUTH_REQUIRED:";

fn parse_claude_response(stdout: &str) -> Result<JiraIssueData, String> {
    // 1. Detect OAuth authorization request first — before any JSON parsing
    // attempt. Claude returns the authorize URL as plain text inside the
    // `result` field when the MCP requires authentication. Doing this first
    // prevents the JSON-parsing steps below from accidentally swallowing the
    // auth signal.
    if let Some(url) = try_extract_auth_url(stdout) {
        return Err(format!("{}{}", AUTH_REQUIRED_PREFIX, url));
    }

    // 2. Detect max-turns exhaustion. When Claude exits after hitting
    // --max-turns, the wrapper has "subtype":"error_max_turns". The raw JSON
    // dump in the fallback task description is confusing; surface a concise
    // actionable message instead.
    if let Ok(wrapper) = serde_json::from_str::<serde_json::Value>(stdout) {
        if wrapper.get("subtype").and_then(|s| s.as_str()) == Some("error_max_turns") {
            return Err(
                "O Claude atingiu o limite de turnos ao buscar a issue no Jira. \
                 Isso pode indicar que o MCP Atlassian está precisando de autenticação \
                 ou está demorando mais turnos do que o esperado. Tente importar novamente."
                    .to_string(),
            );
        }
    }

    // 2. Try direct parse as JiraIssueData
    if let Ok(data) = serde_json::from_str::<JiraIssueData>(stdout) {
        if data.summary.is_some() {
            return Ok(data);
        }
    }

    // 3. Claude --output-format json wraps in {"result": "..."}
    if let Ok(wrapper) = serde_json::from_str::<serde_json::Value>(stdout) {
        // Try result as string containing JSON
        if let Some(result_str) = wrapper.get("result").and_then(|r| r.as_str()) {
            if let Some(data) = try_extract_jira_data(result_str) {
                return Ok(data);
            }
        }
        // Try result as object directly
        if let Some(result_obj) = wrapper.get("result") {
            if let Ok(data) = serde_json::from_value::<JiraIssueData>(result_obj.clone()) {
                if data.summary.is_some() {
                    return Ok(data);
                }
            }
        }
        // Try the wrapper itself has our fields
        if wrapper.get("summary").is_some() {
            if let Ok(data) = serde_json::from_value::<JiraIssueData>(wrapper) {
                return Ok(data);
            }
        }
    }

    // 4. Try to find JSON in raw text
    if let Some(data) = try_extract_jira_data(stdout) {
        return Ok(data);
    }

    Err(format!(
        "Could not parse Jira data from Claude output (length={}): {}",
        stdout.len(),
        stdout.chars().take(300).collect::<String>()
    ))
}

fn try_extract_auth_url(stdout: &str) -> Option<String> {
    // Look inside the wrapper's `result` field first — that's where Claude
    // puts the auth instruction. Fall back to the raw stdout.
    let result_text = serde_json::from_str::<serde_json::Value>(stdout)
        .ok()
        .and_then(|w| w.get("result").and_then(|r| r.as_str()).map(str::to_owned));

    let candidates = [result_text.as_deref(), Some(stdout)];
    for source in candidates.into_iter().flatten() {
        let lower = source.to_lowercase();
        // Accept any phrasing that indicates a browser-based auth flow.
        // "authorize" covers the classic OAuth keyword; the others cover
        // variations Claude may emit when the MCP prompts for consent.
        let is_auth_context = lower.contains("authorize")
            || lower.contains("open this url")
            || lower.contains("in your browser")
            || lower.contains("grant access");
        if !is_auth_context {
            continue;
        }
        for (start, _) in source.match_indices("https://") {
            let end = source[start..]
                .find(|c: char| {
                    c.is_whitespace()
                        || matches!(c, '"' | '\\' | '\'' | ')' | ']' | '<' | '>')
                })
                .map(|i| start + i)
                .unwrap_or(source.len());
            let candidate = &source[start..end];
            // The outer context check already confirms this is an auth response.
            // Accept any Atlassian URL or any URL with "authorize" in the path —
            // this handles URL-format variations across Atlassian OAuth endpoints.
            if candidate.contains("atlassian") || candidate.contains("authorize") {
                return Some(candidate.to_string());
            }
        }
    }
    None
}

fn try_extract_jira_data(text: &str) -> Option<JiraIssueData> {
    // Try direct parse
    if let Ok(data) = serde_json::from_str::<JiraIssueData>(text) {
        if data.summary.is_some() {
            return Some(data);
        }
    }

    // Extract JSON from text (may be wrapped in markdown code blocks or other text)
    if let Some(json_str) = extract_json_from_text(text) {
        if let Ok(data) = serde_json::from_str::<JiraIssueData>(&json_str) {
            if data.summary.is_some() {
                return Some(data);
            }
        }
    }

    None
}

fn extract_json_from_text(text: &str) -> Option<String> {
    // Find the first { and match to closing }
    if let Some(start) = text.find('{') {
        let mut depth = 0;
        for (i, c) in text[start..].char_indices() {
            match c {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(text[start..start + i + 1].to_string());
                    }
                }
                _ => {}
            }
        }
    }
    None
}

fn map_jira_status(status: Option<&str>) -> TaskStatus {
    match status.map(|s| s.to_lowercase()).as_deref() {
        Some("done") | Some("closed") | Some("resolved") | Some("concluído") => {
            TaskStatus::Completed
        }
        Some("in progress") | Some("in review") | Some("in development")
        | Some("em andamento") | Some("em progresso") => TaskStatus::InProgress,
        _ => TaskStatus::Pending,
    }
}

/// Runs a full diagnostic for a Jira import attempt and returns a plain-text
/// report suitable for pasting into a bug report. Captures: Claude CLI path
/// and version, MCP detection, raw stdout/stderr/exit-code from the actual
/// fetch attempt. No Jira data is saved — this is read-only.
#[tauri::command]
pub async fn run_jira_diagnostic(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    repository_id: String,
    jira_key: String,
) -> Result<String, String> {
    use std::fmt::Write as _;

    let mut r = String::new();
    let _ = writeln!(r, "=== LGE Cockpit — Jira Diagnostic ===");
    let _ = writeln!(r, "Timestamp : {}", Utc::now().to_rfc3339());
    let _ = writeln!(r, "Jira key  : {}", jira_key);

    // ── Resolved paths ────────────────────────────────────────────────────────
    let claude_bin = resolve_claude_path();
    let (repo_path, env_prefix) = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        (
            queries::get_repository_path(&conn, &repository_id)
                .unwrap_or_else(|_| "(not found)".to_string()),
            shell_env_prefix(&conn),
        )
    };
    let _ = writeln!(r, "\n--- Paths ---");
    let _ = writeln!(r, "Claude bin    : {}", claude_bin);
    let _ = writeln!(r, "Repository    : {}", repo_path);

    // ── MCP detection ─────────────────────────────────────────────────────────
    let detection = detect_atlassian_mcp();
    let _ = writeln!(r, "\n--- MCP Detection ---");
    let _ = writeln!(r, "Configured    : {}", detection.configured);
    let _ = writeln!(
        r,
        "Server name   : {}",
        detection.server_name.as_deref().unwrap_or("(none)")
    );

    // ── Claude CLI version ────────────────────────────────────────────────────
    let shell = app.shell();
    let _ = writeln!(r, "\n--- Claude CLI version ---");
    let ver_cmd = format!("{}{} --version 2>&1", env_prefix, claude_bin);
    match shell
        .command(&user_shell())
        .args(["-l", "-i", "-c", &ver_cmd])
        .output()
        .await
    {
        Ok(out) => {
            let _ = writeln!(r, "Exit code : {:?}", out.status.code());
            let stdout = String::from_utf8_lossy(&out.stdout);
            let stderr = String::from_utf8_lossy(&out.stderr);
            let _ = writeln!(r, "stdout    : {}", stdout.trim());
            if !stderr.trim().is_empty() {
                let _ = writeln!(r, "stderr    : {}", stderr.trim());
            }
        }
        Err(e) => {
            let _ = writeln!(r, "Error invoking shell: {}", e);
        }
    }

    // ── Actual fetch attempt ──────────────────────────────────────────────────
    let user_server = detection.server_name.as_deref().filter(|n| !n.contains(' '));
    let home = std::env::var("HOME").unwrap_or_default();
    let path_export = format!(
        "export PATH=\"{home}/.nvm/versions/node/$(ls {home}/.nvm/versions/node/ 2>/dev/null | sort -V | tail -1)/bin:/opt/homebrew/bin:/usr/local/bin:$PATH\"",
        home = home
    );

    let (mcp_arg, allowed_tools_csv) = if let Some(name) = user_server {
        let tools = format!("mcp__{name}__getJiraIssue,mcp__{name}__jira_get_issue,Bash");
        (String::new(), tools)
    } else {
        let mcp_config = resolve_mcp_config();
        (
            format!(" --mcp-config {}", shell_escape(&mcp_config)),
            "mcp__atlassian-mcp__getJiraIssue,Bash".to_string(),
        )
    };

    // Simplified prompt — just enough to trigger the MCP call.
    let diag_prompt = format!(
        "Fetch Jira issue {} using the Atlassian MCP. Return ONLY a JSON: {{\"summary\":\"...\",\"status\":\"...\"}}",
        jira_key
    );
    let full_cmd = format!(
        "{env_prefix}{path_export}; cd /tmp && echo {prompt} | {claude} --print --output-format json --max-turns 10{mcp_arg} --allowedTools {tools}",
        prompt = shell_escape(&diag_prompt),
        claude = claude_bin,
        mcp_arg = mcp_arg,
        tools = shell_escape(&allowed_tools_csv),
    );

    let _ = writeln!(r, "\n--- Fetch Attempt ---");
    let _ = writeln!(
        r,
        "MCP strategy  : {}",
        if mcp_arg.is_empty() { "user server" } else { "bundled mcp-remote config" }
    );
    let _ = writeln!(r, "Allowed tools : {}", allowed_tools_csv);

    match shell
        .command(&user_shell())
        .args(["-l", "-i", "-c", &full_cmd])
        .output()
        .await
    {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let stderr = String::from_utf8_lossy(&out.stderr);
            let _ = writeln!(r, "Exit code     : {:?}", out.status.code());
            let _ = writeln!(r, "stdout ({} bytes):\n{}", stdout.len(), stdout.trim());
            if !stderr.trim().is_empty() {
                let _ = writeln!(r, "stderr ({} bytes):\n{}", stderr.len(), stderr.trim());
            }
        }
        Err(e) => {
            let _ = writeln!(r, "Error invoking shell: {}", e);
        }
    }

    let _ = writeln!(r, "\n=== end of diagnostic ===");
    Ok(r)
}
