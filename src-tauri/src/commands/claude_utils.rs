/// Returns the user's login shell (e.g. "/bin/zsh", "/bin/bash").
/// Falls back to "sh" if SHELL is not set.
/// Used with `-l -i -c` flags so subprocesses load the full user
/// environment (.zshrc/.bashrc) — including gvm, nvm, pyenv, etc.
pub fn user_shell() -> String {
    std::env::var("SHELL").unwrap_or_else(|_| "sh".to_string())
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
