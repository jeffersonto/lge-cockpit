//! `CommitMessageRunner` — the deep module that owns generating a Conventional
//! Commit message for staged changes by invoking the Claude CLI. Replaces the
//! hand-rolled `echo prompt | claude --print --model haiku` assembly that
//! lived in `commands/git.rs::generate_commit_message`, routing through the
//! [`crate::claude_invocation::ClaudeInvocation`] port so the seam becomes
//! real (the second production caller besides `PhaseRunner`) and the JSON
//! `result` extraction is shared via [`ClaudeOutcome::result`].
//!
//! See `CONTEXT.md` for the design.

use crate::claude_invocation::{ClaudeInvocation, ClaudeRequest};
use crate::models::Permission;
use crate::settings::ShellEnv;

/// Everything [`CommitMessageRunner::generate`] needs to produce a commit
/// message. The caller (the IPC adapter in `commands/git.rs`) is responsible
/// for collecting the diff strings and the configured `ShellEnv`/`model`.
#[derive(Debug, Clone)]
pub struct CommitMessageInput {
    pub task_title: String,
    /// Conventional Commit scope, already including the parentheses (e.g.
    /// `"(parser)"`), or empty when no scope is wanted.
    pub scope: String,
    pub diff_stat: String,
    pub diff_preview: String,
    pub working_dir: String,
    pub env_prefix: ShellEnv,
    pub model: String,
}

/// One-shot runner: holds the [`ClaudeInvocation`] port and produces a commit
/// message. Generic over the port (like `PhaseRunner`) so tests pass a fake
/// and production passes a `RealClaudeInvocation` — two adapters, real seam.
pub struct CommitMessageRunner<C: ClaudeInvocation> {
    claude: C,
}

impl<C: ClaudeInvocation> CommitMessageRunner<C> {
    pub fn new(claude: C) -> Self {
        Self { claude }
    }

    /// Spawns a one-shot Claude invocation with `--max-turns 1` and
    /// `Permission::None`, awaits it, and returns the normalized commit
    /// message. Falls back to `feat{scope}: {task_title}` when Claude returns
    /// nothing useful.
    pub async fn generate(&self, input: &CommitMessageInput) -> Result<String, String> {
        let prompt = build_prompt(input);

        let proc = self
            .claude
            .spawn(ClaudeRequest {
                prompt,
                working_dir: input.working_dir.clone(),
                model: input.model.clone(),
                permission: Permission::None,
                env_prefix: input.env_prefix.prefix().to_string(),
                max_turns: Some(1),
            })
            .map_err(|e| format!("Failed to invoke Claude CLI: {}", e))?;

        let outcome = proc.completion.await;

        if outcome.exit_code != Some(0) {
            return Err(outcome.stderr.trim().to_string());
        }

        let raw = outcome.result();
        let message = if raw.is_empty() {
            // Fallback so the caller still gets a usable commit message.
            format!("feat{}: {}", input.scope, input.task_title)
        } else {
            // Conventional Commits are single-line; take the first non-empty line.
            raw.lines()
                .map(|l| l.trim())
                .find(|l| !l.is_empty())
                .unwrap_or(&raw)
                .to_string()
        };
        Ok(message)
    }
}

/// Inline prompt template. Specific variables (`task_title`, `scope`,
/// `diff_stat`, `diff_preview`, `Rules block`) don't fit `Phase::build_prompt`
/// / `PromptContext`, so the template lives here in the runner — not in
/// `src-tauri/prompts/`.
fn build_prompt(input: &CommitMessageInput) -> String {
    format!(
        r#"Generate a single-line conventional commit message for the following changes.

Task: {task_title}
Format: feat{scope}: <short imperative description> (max 72 chars total)

Changed files:
{diff_stat}

Diff preview:
{diff_preview}

Rules:
- Output ONLY the commit message, nothing else
- Use feat{scope}: prefix
- Imperative mood (e.g. "add", "implement", "fix")
- Max 72 characters
- English"#,
        task_title = input.task_title,
        scope = input.scope,
        diff_stat = input.diff_stat,
        diff_preview = input.diff_preview,
    )
}

// ─── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::claude_invocation::{ClaudeInvocation, ClaudeOutcome, ClaudeProcess};
    use std::sync::Mutex;

    /// Fake port that records the request and returns a pre-canned outcome.
    struct FakeClaude {
        captured: Mutex<Option<CapturedReq>>,
        outcome: ClaudeOutcome,
    }

    /// Fields we assert on — `ClaudeRequest` isn't `Clone`.
    #[derive(Clone)]
    struct CapturedReq {
        prompt: String,
        working_dir: String,
        model: String,
        permission: Permission,
        #[allow(dead_code)]
        env_prefix: String,
        max_turns: Option<u32>,
    }

    impl FakeClaude {
        fn new(outcome: ClaudeOutcome) -> Self {
            Self { captured: Mutex::new(None), outcome }
        }
        fn captured(&self) -> CapturedReq {
            self.captured.lock().unwrap().as_ref().expect("no request captured").clone()
        }
    }

    impl ClaudeInvocation for FakeClaude {
        fn spawn(&self, req: ClaudeRequest) -> Result<ClaudeProcess, String> {
            // ClaudeRequest isn't Clone; capture the fields we assert on here.
            let captured = CapturedReq {
                prompt: req.prompt.clone(),
                working_dir: req.working_dir.clone(),
                model: req.model.clone(),
                permission: req.permission,
                env_prefix: req.env_prefix.clone(),
                max_turns: req.max_turns,
            };
            *self.captured.lock().unwrap() = Some(captured);
            let outcome = self.outcome.clone();
            Ok(ClaudeProcess {
                pid: 99,
                completion: Box::pin(async move { outcome }),
            })
        }
    }

    fn input() -> CommitMessageInput {
        CommitMessageInput {
            task_title: "Add feature X".to_string(),
            scope: "(api)".to_string(),
            diff_stat: " 2 files changed".to_string(),
            diff_preview: "+export function x() {}".to_string(),
            working_dir: "/tmp/repo".to_string(),
            env_prefix: ShellEnv::empty(),
            model: "haiku".to_string(),
        }
    }

    fn outcome_json(result: &str) -> ClaudeOutcome {
        ClaudeOutcome {
            stdout: format!(r#"{{"result":"{}"}}"#, result),
            stderr: String::new(),
            exit_code: Some(0),
        }
    }

    #[tokio::test]
    async fn returns_unwrapped_json_result_first_line() {
        let fake = FakeClaude::new(outcome_json("feat(api): add thing"));
        let runner = CommitMessageRunner::new(fake);
        let msg = runner.generate(&input()).await.unwrap();
        assert_eq!(msg, "feat(api): add thing");
    }

    #[tokio::test]
    async fn picks_first_line_when_result_has_multiple() {
        // JSON requires `\n` to be escaped inside strings; build the stdout
        // explicitly so the result field contains a literal encoded newline.
        let stdout = r#"{"result":"feat(api): add thing\nextra commentary"}"#.to_string();
        let fake = FakeClaude::new(ClaudeOutcome {
            stdout,
            stderr: String::new(),
            exit_code: Some(0),
        });
        let runner = CommitMessageRunner::new(fake);
        let msg = runner.generate(&input()).await.unwrap();
        assert_eq!(msg, "feat(api): add thing");
    }

    #[tokio::test]
    async fn falls_back_to_scope_title_when_result_empty() {
        let fake = FakeClaude::new(ClaudeOutcome {
            stdout: String::new(),
            stderr: String::new(),
            exit_code: Some(0),
        });
        let runner = CommitMessageRunner::new(fake);
        let msg = runner.generate(&input()).await.unwrap();
        assert_eq!(msg, "feat(api): Add feature X");
    }

    #[tokio::test]
    async fn propagates_stderr_when_exit_nonzero() {
        let fake = FakeClaude::new(ClaudeOutcome {
            stdout: String::new(),
            stderr: "boom".to_string(),
            exit_code: Some(1),
        });
        let runner = CommitMessageRunner::new(fake);
        assert_eq!(runner.generate(&input()).await.unwrap_err(), "boom");
    }

    #[tokio::test]
    async fn spawns_with_permission_none_and_max_turns_one() {
        let fake = FakeClaude::new(outcome_json("feat: ok"));
        let runner = CommitMessageRunner::new(fake);
        runner.generate(&input()).await.unwrap();
        let captured = runner.claude.captured();
        assert_eq!(captured.permission, Permission::None);
        assert_eq!(captured.max_turns, Some(1));
        assert_eq!(captured.model, "haiku");
        assert_eq!(captured.working_dir, "/tmp/repo");
    }

    #[tokio::test]
    async fn prompt_includes_task_title_scope_and_diff_preview() {
        let fake = FakeClaude::new(outcome_json("feat: ok"));
        let runner = CommitMessageRunner::new(fake);
        runner.generate(&input()).await.unwrap();
        let prompt = runner.claude.captured().prompt;
        assert!(prompt.contains("Task: Add feature X"));
        assert!(prompt.contains("feat(api):"));
        assert!(prompt.contains("+export function x() {}"));
    }
}