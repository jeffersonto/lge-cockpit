---
globs: ["src-tauri/src/commands/*.rs"]
---

# Subprocess argument escaping

`commands/lge.rs`, `commands/git.rs`, and `commands/jira.rs` spawn external processes (Claude CLI, git, npx). Any user-controlled string interpolated into a shell command string is a command-injection vector.

The helper `commands::claude_utils::shell_escape` (defined at `claude_utils.rs:56`) wraps a string in single quotes with `'\''` escaping. Use it for **every** user-controlled value passed through a shell-style invocation.

```rust
pub fn shell_escape(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}
```

## Correct

```rust
let cmd = format!(
    "{} --print {} && cd {}",
    claude_path,                    // resolved path, not user input — OK
    shell_escape(&prompt),          // user input — escaped
    shell_escape(&working_dir),     // user input — escaped
);
```

## Incorrect

```rust
// User can break out via single quotes, semicolons, or backticks
let cmd = format!("git checkout {}", branch_name);

// Even seemingly-safe values like paths can contain spaces or special chars
let cmd = format!("cd {} && git status", repo_path);
```

## When `shell_escape` is not enough

- Prefer `Command::new(...).args(&[...])` with separate arg vectors over building a shell string. The shell-string pattern exists in this codebase only for `bash -lc` invocations that need shell features (PATH inheritance, NVM bootstrap). For new commands, default to argument vectors.
- Numeric values (PIDs, line numbers) can be interpolated directly without escaping but only after parsing them through `u32`/`i64`. Never interpolate a `String` that "looks like a number".
- Paths chosen by the user (repo path, worktree path, attachment path) are user input. Escape them.

## Audit checklist

Before committing a change to `lge.rs`, `git.rs`, or `jira.rs`:
- `grep -n "format!" <file>` — every `format!` building a shell command must wrap user values in `shell_escape`.
- The fallback `pkill -f 'claude --print'` at `lge.rs:741` is a known overly-broad kill; do not introduce more `pkill` patterns.
