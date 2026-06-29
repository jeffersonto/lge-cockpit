//! The Claude CLI invocation port.
//!
//! A deep-ish module behind a small seam: "given a `ClaudeRequest`, produce a
//! running process whose output I can later await." The `Permission -> CLI flag`
//! translation, shell escaping, and `bash -lc` assembly live here (in the pure
//! `build_claude_command` helper and the real adapter) — NOT in `Phase` (which
//! is a pure value-object) and NOT in `PhaseRunner` (which orchestrates but
//! never sees flag strings).
//!
//! Lives in its own file so architecture candidate 07 (Jira session) can reuse
//! the `ClaudeInvocation` trait + `build_claude_command` without dragging in
//! `PhaseRunner`.

use std::future::Future;
use std::pin::Pin;

use crate::commands::claude_utils::{resolve_claude_path, shell_escape, user_shell};
use crate::models::Permission;

/// Everything needed to spawn one Claude CLI invocation. Owned strings so the
/// request can outlive the caller's borrows (the completion future is 'static).
pub struct ClaudeRequest {
    pub prompt: String,
    pub working_dir: String,
    pub model: String,
    pub permission: Permission,
    pub env_prefix: String,
}

/// The collected result of a Claude process after it exits.
#[derive(Debug, Clone)]
pub struct ClaudeOutcome {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
}

/// A spawned Claude process: the PID (available immediately, so cancel can kill
/// it mid-run) plus a future that completes with the collected output when the
/// process exits. Splitting spawn-from-await is what makes PID tracking and
/// cancel-during-run correct AND testable (a fake returns a pid + a controllable
/// future).
pub struct ClaudeProcess {
    pub pid: u32,
    pub completion: Pin<Box<dyn Future<Output = ClaudeOutcome> + Send + 'static>>,
}

/// The port. `spawn` is sync and returns immediately; PhaseRunner inserts the
/// pid into `running_pids` BEFORE awaiting `completion`. The error is a plain
/// `String` (mapped by PhaseRunner to `PhaseRunError::Io`) so this trait stays
/// decoupled from PhaseRunner's error enum — no circular type dependency.
pub trait ClaudeInvocation {
    fn spawn(&self, req: ClaudeRequest) -> Result<ClaudeProcess, String>;
}

// Blanket impl so PhaseRunner can hold a borrowed port (tests pass &FakeClaude;
// production passes an owned RealClaudeInvocation).
impl<C: ClaudeInvocation + ?Sized> ClaudeInvocation for &C {
    fn spawn(&self, req: ClaudeRequest) -> Result<ClaudeProcess, String> {
        (**self).spawn(req)
    }
}

/// Pure string assembly of the `bash -lc` argument. Extracted so the
/// shell-escaping and `Permission -> flag` translation can be unit-tested in
/// isolation. `claude_bin` is passed in (rather than resolved here) so the unit
/// test doesn't touch the filesystem / `HOME` env.
pub fn build_claude_command(req: &ClaudeRequest, claude_bin: &str) -> String {
    let permissions_flag = match req.permission {
        Permission::Plan => "--permission-mode plan",
        Permission::SkipPermissions => "--dangerously-skip-permissions",
    };
    format!(
        "{}echo {} | {} --print --model {} {} -p {}",
        req.env_prefix,
        shell_escape(&req.prompt),
        claude_bin,
        req.model,
        permissions_flag,
        shell_escape(&req.working_dir),
    )
}

/// Production adapter: spawns the real Claude CLI via the Tauri shell plugin.
/// Holds an owned `AppHandle` (cheap inner Arc) so there is no lifetime on the
/// adapter itself.
pub struct RealClaudeInvocation {
    app: tauri::AppHandle,
}

impl RealClaudeInvocation {
    pub fn new(app: tauri::AppHandle) -> Self {
        Self { app }
    }
}

impl ClaudeInvocation for RealClaudeInvocation {
    fn spawn(&self, req: ClaudeRequest) -> Result<ClaudeProcess, String> {
        use tauri_plugin_shell::process::CommandEvent;
        use tauri_plugin_shell::ShellExt;

        let claude_bin = resolve_claude_path();
        let full_cmd = build_claude_command(&req, &claude_bin);
        let shell = self.app.shell();
        let (mut rx, child) = shell
            .command(user_shell())
            .args(["-l", "-i", "-c", &full_cmd])
            .spawn()
            .map_err(|e| format!("Failed to invoke Claude CLI: {}", e))?;

        let pid = child.pid();
        let completion = Box::pin(async move {
            let mut stdout_bytes = Vec::new();
            let mut stderr_bytes = Vec::new();
            let mut exit_code: Option<i32> = None;
            while let Some(event) = rx.recv().await {
                match event {
                    CommandEvent::Stdout(data) => stdout_bytes.extend_from_slice(&data),
                    CommandEvent::Stderr(data) => stderr_bytes.extend_from_slice(&data),
                    CommandEvent::Terminated(payload) => {
                        exit_code = payload.code;
                        break;
                    }
                    CommandEvent::Error(err) => stderr_bytes.extend_from_slice(err.as_bytes()),
                    _ => {}
                }
            }
            ClaudeOutcome {
                stdout: String::from_utf8_lossy(&stdout_bytes).to_string(),
                stderr: String::from_utf8_lossy(&stderr_bytes).to_string(),
                exit_code,
            }
        });

        Ok(ClaudeProcess { pid, completion })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req(permission: Permission) -> ClaudeRequest {
        ClaudeRequest {
            prompt: "hello $world".to_string(),
            working_dir: "/tmp/dir with space".to_string(),
            model: "opus".to_string(),
            permission,
            env_prefix: "export PATH=/x\n".to_string(),
        }
    }

    #[test]
    fn build_command_uses_plan_flag_for_planning() {
        let cmd = build_claude_command(&req(Permission::Plan), "/usr/bin/claude");
        assert!(cmd.contains("--permission-mode plan"));
        assert!(!cmd.contains("--dangerously-skip-permissions"));
    }

    #[test]
    fn build_command_uses_skip_permissions_for_others() {
        let cmd = build_claude_command(&req(Permission::SkipPermissions), "/usr/bin/claude");
        assert!(cmd.contains("--dangerously-skip-permissions"));
        assert!(!cmd.contains("--permission-mode plan"));
    }

    #[test]
    fn build_command_escapes_prompt_and_working_dir() {
        let cmd = build_claude_command(&req(Permission::Plan), "/usr/bin/claude");
        // The raw prompt/working_dir contain shell-dangerous chars; the built
        // command must route them through shell_escape (single-quote wrapping).
        assert!(cmd.contains("'hello $world'"));
        assert!(cmd.contains("'/tmp/dir with space'"));
        // And the unescaped values must NOT appear bare.
        assert!(!cmd.contains("echo hello $world |"));
    }

    #[test]
    fn build_command_includes_env_prefix_model_and_binary() {
        let cmd = build_claude_command(&req(Permission::Plan), "/usr/bin/claude");
        assert!(cmd.starts_with("export PATH=/x\n"));
        assert!(cmd.contains("/usr/bin/claude --print --model opus"));
    }
}
