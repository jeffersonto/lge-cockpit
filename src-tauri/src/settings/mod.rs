//! The Settings module — the deep module that owns all typed reads and writes
//! of the app's key/value configuration (the SQLite `settings` table).
//!
//! Every read or write of a setting crosses this seam. SQL itself stays in
//! `db/queries.rs` (per the AGENTS.md convention that all SQL lives there);
//! `get_setting` / `set_setting` are `pub(crate)` and used only by this module.
//! Callers above this seam receive typed values (`JiraConfig`, `ShellEnv`,
//! `Phase`, resolved model strings) — never raw setting keys.
//!
//! See `CONTEXT.md` for the design.

use std::collections::HashMap;

use rusqlite::Connection;

use crate::db::queries;
use crate::jira::JiraConfig;
use crate::models::Phase;

/// Accepted model values for `Phase` overrides. Validation is at the write
/// seam (`save_phase_models`) so reads can trust the DB.
const VALID_MODELS: &[&str] = &["opus", "sonnet", "haiku"];

// ─── ShellEnv value-object ─────────────────────────────────────────────────

/// The user-customized shell prefix derived from the `shell_env` setting.
///
/// Owns the parsing invariant — each non-comment, non-blank line is terminated
/// with `;` and the whole is joined with a trailing `"; "` — so a caller
/// receives "ready to prepend to a `bash -lc` string", not a raw setting
/// value. `from_raw` is public so the parser is unit-testable without SQLite;
/// `empty()` exists for fakes in tests of dependent modules.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellEnv(String);

impl ShellEnv {
    /// Constructs a `ShellEnv` from the raw setting value. Pure: no IO.
    /// Comments (lines starting with `#`) and blank lines are dropped; each
    /// remaining line is trimmed and the lot is joined with `"; "` and given a
    /// trailing `"; "` so the result can be prepended directly to a command.
    pub fn from_raw(raw: &str) -> Self {
        let commands: Vec<&str> = raw
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .collect();
        if commands.is_empty() {
            return Self(String::new());
        }
        Self(format!("{}; ", commands.join("; ")))
    }

    /// An empty prefix — for tests of dependent modules that don't care about
    /// the user's shell environment.
    #[allow(dead_code)]
    pub fn empty() -> Self {
        Self(String::new())
    }

    /// The formatted prefix, ready to prepend to a `bash -lc` command string.
    /// Empty when no custom env is configured.
    pub fn prefix(&self) -> &str {
        &self.0
    }
}

// ─── Jira ──────────────────────────────────────────────────────────────────

/// Reads the configured Jira credentials. All three fields are returned even
/// when empty — the Jira client turns a missing trio into a clear
/// `NotConfigured` error. Single source of truth for every call site that
/// needs the configured Jira credentials.
pub fn jira_config(conn: &Connection) -> JiraConfig {
    JiraConfig {
        base_url: queries::get_setting(conn, "jira_base_url").unwrap_or_default(),
        email: queries::get_setting(conn, "jira_email").unwrap_or_default(),
        api_token: queries::get_setting(conn, "jira_api_token").unwrap_or_default(),
    }
}

/// Persists the three Jira credential fields. Each field is written
/// individually so partial configs are still recoverable.
pub fn save_jira(conn: &Connection, config: &JiraConfig) -> Result<(), String> {
    queries::set_setting(conn, "jira_base_url", &config.base_url)?;
    queries::set_setting(conn, "jira_email", &config.email)?;
    queries::set_setting(conn, "jira_api_token", &config.api_token)?;
    Ok(())
}

// ─── Shell env ─────────────────────────────────────────────────────────────

/// Reads the user's custom shell environment as a ready-to-prepend prefix.
/// Use this from callers that spawn subprocesses.
pub fn shell_env(conn: &Connection) -> ShellEnv {
    let raw = queries::get_setting(conn, "shell_env").unwrap_or_default();
    ShellEnv::from_raw(&raw)
}

/// Reads the raw `shell_env` setting value for the UI to display and edit.
/// Subprocess callers should use [`shell_env`] instead — this value is not
/// formatted as a prefix.
pub fn shell_env_raw(conn: &Connection) -> String {
    queries::get_setting(conn, "shell_env").unwrap_or_default()
}

/// Persists the raw `shell_env` setting value (as edited by the UI).
pub fn save_shell_env(conn: &Connection, value: &str) -> Result<(), String> {
    queries::set_setting(conn, "shell_env", value)
}

// ─── Phase models ──────────────────────────────────────────────────────────

/// Reads the configured model override for one phase, falling back to the
/// phase's static default when the row is missing or empty.
pub fn phase_model(conn: &Connection, phase: Phase) -> String {
    let key = format!("model_{}", phase.as_str());
    queries::get_setting(conn, &key)
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| phase.default_model().to_string())
}

/// Reads all four phase model overrides in one batch, falling back to each
/// phase's static default for any missing/empty row. The returned map is
/// always complete — callers (e.g. the IPC command) never need their own
/// defaults.
pub fn phase_models(conn: &Connection) -> HashMap<Phase, String> {
    Phase::ALL
        .iter()
        .map(|&p| (p, phase_model(conn, p)))
        .collect()
}

/// Persists phase model overrides. Keys are the phase wire names
/// (as the frontend sends them); unknown phases or invalid model values are
/// rejected here, at the seam, before any write reaches the DB.
pub fn save_phase_models(conn: &Connection, models: &HashMap<String, String>) -> Result<(), String> {
    for (phase_wire, model) in models {
        if !VALID_MODELS.contains(&model.as_str()) {
            return Err(format!("Invalid model '{}' for phase '{}'", model, phase_wire));
        }
        let phase: Phase = phase_wire
            .as_str()
            .parse()
            .map_err(|e: crate::models::ParsePhaseError| format!("Unknown phase: {}", e))?;
        let key = format!("model_{}", phase.as_str());
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

// ─── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn mem_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
             INSERT INTO settings VALUES ('model_planning', 'opus');
             INSERT INTO settings VALUES ('model_builder', 'haiku');
             INSERT INTO settings VALUES ('model_review', 'sonnet');
             INSERT INTO settings VALUES ('model_guardian', 'opus');",
        )
        .unwrap();
        conn
    }

    // --- ShellEnv::from_raw -----------------------------------------------

    #[test]
    fn shell_env_empty_yields_empty_prefix() {
        assert_eq!(ShellEnv::from_raw("").prefix(), "");
        assert_eq!(ShellEnv::from_raw("   \n  \n").prefix(), "");
    }

    #[test]
    fn shell_env_drops_comments_and_blanks() {
        let raw = "# a comment\n   \nexport FOO=bar\n";
        assert_eq!(ShellEnv::from_raw(raw).prefix(), "export FOO=bar; ");
    }

    #[test]
    fn shell_env_joins_multiple_lines() {
        let raw = "source ~/.nvm/nvm.sh\nexport FOO=bar";
        assert_eq!(ShellEnv::from_raw(raw).prefix(), "source ~/.nvm/nvm.sh; export FOO=bar; ");
    }

    #[test]
    fn shell_env_trims_each_line() {
        let raw = "  export FOO=bar  \n";
        assert_eq!(ShellEnv::from_raw(raw).prefix(), "export FOO=bar; ");
    }

    #[test]
    fn shell_env_empty_constructor() {
        assert_eq!(ShellEnv::empty().prefix(), "");
    }

    // --- jira_config / save_jira ------------------------------------------

    #[test]
    fn jira_config_defaults_when_unset() {
        let conn = mem_conn();
        let c = jira_config(&conn);
        assert_eq!(c.base_url, "");
        assert_eq!(c.email, "");
        assert_eq!(c.api_token, "");
        assert!(!c.is_complete());
    }

    #[test]
    fn save_jira_round_trips() {
        let conn = mem_conn();
        let cfg = JiraConfig {
            base_url: "https://x.atlassian.net".to_string(),
            email: "a@b.com".to_string(),
            api_token: "tok".to_string(),
        };
        save_jira(&conn, &cfg).unwrap();
        let read = jira_config(&conn);
        assert_eq!(read.base_url, "https://x.atlassian.net");
        assert_eq!(read.email, "a@b.com");
        assert_eq!(read.api_token, "tok");
        assert!(read.is_complete());
    }

    // --- shell_env / shell_env_raw / save_shell_env -----------------------

    #[test]
    fn shell_env_read_returns_formatted_prefix() {
        let conn = mem_conn();
        queries::set_setting(&conn, "shell_env", "export FOO=bar\n# c").unwrap();
        assert_eq!(shell_env(&conn).prefix(), "export FOO=bar; ");
    }

    #[test]
    fn shell_env_raw_returns_unformatted_value() {
        let conn = mem_conn();
        let raw = "export FOO=bar\n# c";
        queries::set_setting(&conn, "shell_env", raw).unwrap();
        assert_eq!(shell_env_raw(&conn), raw);
    }

    #[test]
    fn save_shell_env_round_trips() {
        let conn = mem_conn();
        save_shell_env(&conn, "source ~/.nvm/nvm.sh").unwrap();
        assert_eq!(shell_env_raw(&conn), "source ~/.nvm/nvm.sh");
        assert_eq!(shell_env(&conn).prefix(), "source ~/.nvm/nvm.sh; ");
    }

    // --- phase_model / phase_models / save_phase_models -------------------

    #[test]
    fn phase_model_falls_back_to_default_when_missing() {
        let conn = mem_conn();
        // Remove the row to simulate missing.
        conn.execute("DELETE FROM settings WHERE key = 'model_builder'", [])
            .unwrap();
        assert_eq!(phase_model(&conn, Phase::Builder), "haiku");
    }

    #[test]
    fn phase_model_falls_back_to_default_when_empty() {
        let conn = mem_conn();
        queries::set_setting(&conn, "model_builder", "   ").unwrap();
        assert_eq!(phase_model(&conn, Phase::Builder), "haiku");
    }

    #[test]
    fn phase_model_returns_override_when_set() {
        let conn = mem_conn();
        queries::set_setting(&conn, "model_builder", "opus").unwrap();
        assert_eq!(phase_model(&conn, Phase::Builder), "opus");
    }

    #[test]
    fn phase_models_returns_all_four_resolved() {
        let conn = mem_conn();
        queries::set_setting(&conn, "model_planning", "sonnet").unwrap();
        let m = phase_models(&conn);
        assert_eq!(m.len(), 4);
        assert_eq!(m[&Phase::Planning], "sonnet");
        assert_eq!(m[&Phase::Builder], "haiku");
        assert_eq!(m[&Phase::Review], "sonnet");
        assert_eq!(m[&Phase::Guardian], "opus");
    }

    #[test]
    fn save_phase_models_rejects_unknown_phase() {
        let conn = mem_conn();
        let mut m = HashMap::new();
        m.insert("not-a-phase".to_string(), "opus".to_string());
        let err = save_phase_models(&conn, &m).unwrap_err();
        assert!(err.starts_with("Unknown phase"));
    }

    #[test]
    fn save_phase_models_rejects_invalid_model() {
        let conn = mem_conn();
        let mut m = HashMap::new();
        m.insert("builder".to_string(), "gpt-4".to_string());
        let err = save_phase_models(&conn, &m).unwrap_err();
        assert!(err.starts_with("Invalid model"));
    }

    #[test]
    fn save_phase_models_rejects_missing_row() {
        let conn = mem_conn();
        conn.execute("DELETE FROM settings WHERE key = 'model_review'", [])
            .unwrap();
        let mut m = HashMap::new();
        m.insert("review".to_string(), "opus".to_string());
        let err = save_phase_models(&conn, &m).unwrap_err();
        assert!(err.contains("model_review"));
    }

    #[test]
    fn save_phase_models_writes_valid_pair() {
        let conn = mem_conn();
        let mut m = HashMap::new();
        m.insert("guardian".to_string(), "haiku".to_string());
        save_phase_models(&conn, &m).unwrap();
        assert_eq!(phase_model(&conn, Phase::Guardian), "haiku");
    }
}