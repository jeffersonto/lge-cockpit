//! The PhaseRunner deep module — owns the end-to-end orchestration of one LGE
//! phase execution. Replaces the 220-line `run_lge_phase` procedure; the
//! `#[tauri::command] run_lge_phase` becomes a thin adapter that constructs a
//! PhaseRunner with real port adapters, calls `run()`, sends the system
//! notification based on the outcome, and returns `LgePhaseResult`.
//!
//! Boundary: impure (spawns subprocesses, touches DB/FS) — unlike `Phase`.
//! Testable through `run()` via three fake ports + in-memory SQLite + temp
//! dirs; the DB and FS are local-substitutable (NOT ported). The three true
//! ports (`ClaudeInvocation`, `EventEmitter`, `WorktreeProvisioner`) each have
//! a real adapter (production) and a fake adapter (tests) — two adapters per
//! port, so every seam is real.
//!
//! The planning queue (semaphore + cancelled set + pids map) is inline for
//! now — the seam for architecture candidate 03 (`PhaseProcessRegistry`).

use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use chrono::Utc;
use rusqlite::Connection;
use tokio::sync::Semaphore;

use crate::claude_invocation::{ClaudeInvocation, ClaudeProcess, ClaudeRequest};
use crate::commands::claude_utils::shell_env_prefix;
use crate::db::queries;
use crate::models::{Phase, PromptContext};

// ---- Public value types ----

/// Everything the IPC caller knows at entry. Owned strings so the value can
/// outlive the caller's borrows (the `run` future is `'static`-ish).
pub struct PhaseRunContext {
    pub task_id: String,
    pub phase: Phase,
    pub task_title: String,
    pub task_description: String,
    pub extra_context: Option<String>,
}

/// The successful end state. `artifact_path` is the canonical on-disk path;
/// `artifact_content` is what was retrieved (so the adapter can return both
/// without re-reading).
#[derive(Debug)]
pub struct PhaseOutcome {
    pub artifact_content: String,
    pub artifact_path: String,
}

/// Typed errors so each failure branch is a one-line `matches!` assertion.
#[derive(Debug)]
pub enum PhaseRunError {
    /// Planning was cancelled while sitting in the queue.
    Cancelled,
    /// The task_id has no row in the DB.
    TaskNotFound(String),
    /// Claude exited non-zero with no stdout, or was killed mid-run.
    ClaudeFailed { stderr: String },
    /// Claude exited cleanly but the expected artifact file could not be read.
    ArtifactMissing { phase: Phase, path: String, reason: String },
    /// Synchronization primitive failure (semaphore / mutex) on the planning queue.
    Lock(String),
    Db(String),
    Io(String),
}

impl std::fmt::Display for PhaseRunError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cancelled => write!(f, "Planning cancelled while queued"),
            Self::TaskNotFound(t) => write!(f, "Task not found: {}", t),
            Self::ClaudeFailed { stderr } => write!(f, "Claude CLI error: {}", stderr),
            Self::ArtifactMissing { phase, path, reason } => write!(
                f,
                "Phase '{}' did not produce the expected artifact at '{}': {}",
                phase, path, reason
            ),
            Self::Lock(m) => write!(f, "Planning queue lock error: {}", m),
            Self::Db(m) | Self::Io(m) => write!(f, "{}", m),
        }
    }
}

// ---- Port: EventEmitter (sync — tauri::Emitter::emit is sync) ----

#[derive(Debug, Clone, PartialEq)]
pub enum PhaseEvent {
    Queued { task_id: String, phase: Phase },
    Dequeued { task_id: String, phase: Phase },
}

pub trait EventEmitter {
    fn emit(&self, event: PhaseEvent);
}

// Blanket impl so PhaseRunner can hold a borrowed port (tests pass &FakeEvents;
// production passes an owned AppEmitter). The real adapter owns an AppHandle.
impl<E: EventEmitter + ?Sized> EventEmitter for &E {
    fn emit(&self, event: PhaseEvent) {
        (**self).emit(event);
    }
}

// ---- Port: WorktreeProvisioner (async — manual boxing for Send + no async_trait crate) ----

pub struct WorktreeRequest {
    pub task_id: String,
    pub repo_path: String,
    pub repository_id: String,
    pub task_code: String,
    pub git_branch: Option<String>,
    pub env_prefix: String,
}

pub trait WorktreeProvisioner {
    fn provision(
        &self,
        req: WorktreeRequest,
        resolved: String,
    ) -> Pin<Box<dyn Future<Output = Result<String, PhaseRunError>> + Send + '_>>;
}

// Blanket impl so PhaseRunner can hold a borrowed port.
impl<W: WorktreeProvisioner + ?Sized> WorktreeProvisioner for &W {
    fn provision(
        &self,
        req: WorktreeRequest,
        resolved: String,
    ) -> Pin<Box<dyn Future<Output = Result<String, PhaseRunError>> + Send + '_>> {
        (**self).provision(req, resolved)
    }
}

// ---- The deep module ----

pub struct PhaseRunner<'a, C: ClaudeInvocation, E: EventEmitter, W: WorktreeProvisioner> {
    db: &'a Mutex<Connection>,
    planning_semaphore: &'a Arc<Semaphore>,
    planning_cancelled: &'a Mutex<HashSet<String>>,
    running_pids: &'a Mutex<HashMap<String, u32>>,
    claude: C,
    events: E,
    worktree: W,
}

impl<'a, C: ClaudeInvocation, E: EventEmitter, W: WorktreeProvisioner> PhaseRunner<'a, C, E, W> {
    /// Construct a runner with its three ports. Fields are private; this is the
    /// only construction site, keeping the surface area minimal.
    pub fn new(
        db: &'a Mutex<Connection>,
        planning_semaphore: &'a Arc<Semaphore>,
        planning_cancelled: &'a Mutex<HashSet<String>>,
        running_pids: &'a Mutex<HashMap<String, u32>>,
        claude: C,
        events: E,
        worktree: W,
    ) -> Self {
        Self {
            db,
            planning_semaphore,
            planning_cancelled,
            running_pids,
            claude,
            events,
            worktree,
        }
    }
    /// The single orchestration entry point. Owns:
    /// planning-queue acquire/cancel/emit → task metadata → worktree provision →
    /// attachment fetch+merge → `phase.prepare()` → ClaudeInvocation::spawn →
    /// PID-track-around-await → exit check → artifact retrieval → status update.
    ///
    /// Invariants enforced (not just documented):
    ///   I1 PID is inserted before `completion.await` and removed by `PidGuard`'s
    ///      `Drop` even on early-return / panic.
    ///   I2 The planning permit is held for the whole run via a binding dropped at
    ///      scope end (RAII); on `Cancelled` it is dropped, freeing the slot.
    ///   I3 `Queued` is emitted before `acquire().await`; `Dequeued` after acquire
    ///      succeeds AND the cancelled-set check passes. Non-planning emits neither.
    ///   I4 Task status is written once, after successful retrieval.
    ///   I5 No DB lock is held across `completion.await` — every DB access is scoped.
    pub async fn run(&self, ctx: PhaseRunContext) -> Result<PhaseOutcome, PhaseRunError> {
        let phase = ctx.phase;

        // 1. Planning queue (inline — seam for candidate 03 / PhaseProcessRegistry)
        let _planning_permit = if phase == Phase::Planning {
            self.events.emit(PhaseEvent::Queued {
                task_id: ctx.task_id.clone(),
                phase,
            });
            let permit = self
                .planning_semaphore
                .acquire()
                .await
                .map_err(|e| PhaseRunError::Lock(e.to_string()))?;
            {
                let mut cancelled = self
                    .planning_cancelled
                    .lock()
                    .map_err(|e| PhaseRunError::Lock(e.to_string()))?;
                if cancelled.remove(&ctx.task_id) {
                    // permit dropped here → slot freed for the next planning
                    return Err(PhaseRunError::Cancelled);
                }
            }
            self.events.emit(PhaseEvent::Dequeued {
                task_id: ctx.task_id.clone(),
                phase,
            });
            Some(permit)
        } else {
            None
        };

        // 2. Task metadata (scoped DB lock — I5)
        let (repo_path, task_code, phase_model, repository_id, git_branch) = {
            let conn = self.db.lock().map_err(|e| PhaseRunError::Db(e.to_string()))?;
            let task = conn
                .prepare("SELECT repository_id, jira_key, worktree_path, git_branch FROM tasks WHERE id = ?1")
                .map_err(|e| PhaseRunError::Db(e.to_string()))?
                .query_row(rusqlite::params![ctx.task_id], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, Option<String>>(2)?,
                        row.get::<_, Option<String>>(3)?,
                    ))
                })
                .map_err(|e| PhaseRunError::TaskNotFound(e.to_string()))?;
            let repo_path = queries::get_repository_path(&conn, &task.0)
                .map_err(PhaseRunError::Db)?;
            let code = task.1.unwrap_or_else(|| ctx.task_id[..8].to_string());
            let model = read_phase_model(&conn, phase);
            (repo_path, code, model, task.0, task.3)
        };

        // 3. Working dir + env_prefix (scoped DB lock)
        let (resolved, env_prefix) = {
            let conn = self.db.lock().map_err(|e| PhaseRunError::Db(e.to_string()))?;
            (
                queries::resolve_working_dir(&conn, &ctx.task_id).map_err(PhaseRunError::Db)?,
                shell_env_prefix(&conn),
            )
        };

        let working_dir = if resolved == repo_path {
            if let Some(ref branch) = git_branch {
                match self
                    .worktree
                    .provision(
                        WorktreeRequest {
                            task_id: ctx.task_id.clone(),
                            repo_path: repo_path.clone(),
                            repository_id: repository_id.clone(),
                            task_code: task_code.clone(),
                            git_branch: Some(branch.clone()),
                            env_prefix: env_prefix.clone(),
                        },
                        resolved.clone(),
                    )
                    .await
                {
                    Ok(wt) => wt,
                    Err(_) => repo_path.clone(), // fallback gracefully (preserves current behaviour)
                }
            } else {
                resolved
            }
        } else {
            resolved
        };

        // 4. Artifacts dir
        let artifacts_dir = format!("{}/docs/tasks/{}", working_dir, task_code);
        std::fs::create_dir_all(&artifacts_dir)
            .map_err(|e| PhaseRunError::Io(e.to_string()))?;

        // 5. Attachments (scoped DB lock) + merge with extra_context
        let attachment_context = {
            let conn = self.db.lock().map_err(|e| PhaseRunError::Db(e.to_string()))?;
            let attachments =
                queries::list_attachments_by_task_and_phase(&conn, &ctx.task_id, phase.as_str())
                    .map_err(PhaseRunError::Db)?;
            if attachments.is_empty() {
                String::new()
            } else {
                let mut s = String::from("## Contexto Adicional\n\n");
                for att in &attachments {
                    s.push_str(&format!("### {}\n{}\n\n---\n\n", att.file_name, att.content));
                }
                s
            }
        };
        let combined_context = match (ctx.extra_context.as_deref(), attachment_context.is_empty()) {
            (Some(ec), false) => Some(format!("{}\n\n{}", attachment_context, ec)),
            (None, false) => Some(attachment_context),
            (Some(ec), true) => Some(ec.to_string()),
            (None, true) => None,
        };

        // 6. Phase contract (pure — delegated to the Phase module)
        let plan = phase.prepare(&PromptContext {
            task_code: &task_code,
            task_title: &ctx.task_title,
            task_description: &ctx.task_description,
            extra_context: combined_context.as_deref(),
        });

        // 7. Spawn + PID guard around the await (I1)
        let proc = self
            .claude
            .spawn(ClaudeRequest {
                prompt: plan.prompt,
                working_dir: working_dir.clone(),
                model: phase_model,
                permission: plan.permission,
                env_prefix: env_prefix.clone(),
            })
            .map_err(PhaseRunError::Io)?;

        let out = {
            let _guard = PidGuard::new(self.running_pids, &format!("{}:{}", ctx.task_id, phase), proc.pid);
            // completion is awaited inside the guard's scope so the PID is
            // tracked for the whole run and removed the moment the process exits.
            let ClaudeProcess { completion, .. } = proc;
            completion.await
        };

        // 8. Exit check
        if out.exit_code != Some(0) && out.stdout.trim().is_empty() {
            return Err(PhaseRunError::ClaudeFailed { stderr: out.stderr });
        }

        // 9. Retrieval (owned by PhaseRunner — Phase stops at the contract).
        //    Planning: the 5-min mtime scan in `resolve_plan_file` is a known race
        //    (TODO: fix once `claude --permission-mode plan` JSON support is confirmed).
        //    Builder/Review/Guardian: strict read of the canonical file.
        let artifact_path = format!("{}/{}", artifacts_dir, plan.filename);
        let artifact_content = if phase == Phase::Planning {
            let content = resolve_plan_file(&out.stdout)
                .unwrap_or_else(|| extract_artifact(&out.stdout));
            std::fs::write(&artifact_path, &content)
                .map_err(|e| PhaseRunError::Io(e.to_string()))?;
            content
        } else {
            std::fs::read_to_string(&artifact_path).map_err(|e| PhaseRunError::ArtifactMissing {
                phase,
                path: artifact_path.clone(),
                reason: e.to_string(),
            })?
        };

        // 10. Status state machine (planning/builder/review -> in_progress; guardian -> completed)
        {
            let conn = self.db.lock().map_err(|e| PhaseRunError::Db(e.to_string()))?;
            let status = match phase {
                Phase::Guardian => "completed",
                _ => "in_progress",
            };
            let _ = queries::update_task_status(&conn, &ctx.task_id, status, &Utc::now().to_rfc3339());
        }

        Ok(PhaseOutcome {
            artifact_content,
            artifact_path,
        })
    }
}

// ---- RAII: PID tracking (I1) ----

struct PidGuard<'a> {
    pids: &'a Mutex<HashMap<String, u32>>,
    key: String,
}

impl<'a> PidGuard<'a> {
    fn new(pids: &'a Mutex<HashMap<String, u32>>, key: &str, pid: u32) -> Self {
        if let Ok(mut map) = pids.lock() {
            map.insert(key.to_string(), pid);
        }
        PidGuard { pids, key: key.to_string() }
    }
}

impl Drop for PidGuard<'_> {
    fn drop(&mut self) {
        if let Ok(mut map) = self.pids.lock() {
            map.remove(&self.key);
        }
    }
}

// ---- Private helpers (moved from lge.rs; single read site until Settings module) ----

/// Reads the user-configured model override for a phase, falling back to the
/// phase's static default. Will be owned by the future Settings module (C05).
fn read_phase_model(conn: &Connection, phase: Phase) -> String {
    let key = format!("model_{}", phase.as_str());
    conn.query_row(
        "SELECT value FROM settings WHERE key = ?1",
        rusqlite::params![key],
        |row| row.get::<_, String>(0),
    )
    .unwrap_or_else(|_| phase.default_model().to_string())
}

fn extract_artifact(claude_output: &str) -> String {
    if let Ok(wrapper) = serde_json::from_str::<serde_json::Value>(claude_output) {
        if let Some(result) = wrapper.get("result").and_then(|r| r.as_str()) {
            return result.to_string();
        }
    }
    claude_output.trim().to_string()
}

/// TODO(race): scans `~/.claude/plans/` for the most-recent `.md` modified in
/// the last 5 minutes. Two plannings within 5 min on one machine can land the
/// wrong plan on the wrong task. Fix once `claude --permission-mode plan`'s
/// `--output-format json` support is confirmed (plan returns in stdout → scan
/// deleted). Out of scope for C01 (pre-existing, not a regression).
fn resolve_plan_file(_claude_output: &str) -> Option<String> {
    let home = std::env::var("HOME").ok()?;
    let plans_dir = std::path::Path::new(&home).join(".claude").join("plans");

    let cutoff = std::time::SystemTime::now()
        .checked_sub(std::time::Duration::from_secs(300))
        .unwrap_or(std::time::SystemTime::UNIX_EPOCH);

    let mut candidates: Vec<(std::time::SystemTime, std::path::PathBuf)> = std::fs::read_dir(&plans_dir)
        .ok()?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.extension()?.to_str()? != "md" { return None; }
            let modified = entry.metadata().ok()?.modified().ok()?;
            if modified >= cutoff { Some((modified, path)) } else { None }
        })
        .collect();

    candidates.sort_by_key(|b| std::cmp::Reverse(b.0));
    std::fs::read_to_string(candidates.into_iter().next()?.1).ok()
}

// ---- Real adapters (production; wrap Tauri AppHandle / AppState) ----

/// Real EventEmitter: forwards to `tauri::AppHandle::emit`.
pub struct AppEmitter {
    app: tauri::AppHandle,
}

impl AppEmitter {
    pub fn new(app: tauri::AppHandle) -> Self {
        Self { app }
    }
}

impl EventEmitter for AppEmitter {
    fn emit(&self, event: PhaseEvent) {
        use tauri::Emitter;
        let (name, task_id, phase) = match event {
            PhaseEvent::Queued { task_id, phase } => ("lge_phase_queued", task_id, phase),
            PhaseEvent::Dequeued { task_id, phase } => ("lge_phase_dequeued", task_id, phase),
        };
        // Mirrors the original PlanningQueueEvent payload shape.
        let _ = self.app.emit(
            name,
            serde_json::json!({ "task_id": task_id, "phase": phase.as_str() }),
        );
    }
}

/// Real WorktreeProvisioner: delegates to `git::ensure_worktree`. Borrows
/// `&AppState` (ensure_worktree reads `state.db`); the lifetime is tied to the
/// Tauri command's `State<'_, AppState>` scope.
pub struct RealWorktreeProvisioner<'a> {
    app: tauri::AppHandle,
    state: &'a crate::AppState,
}

impl<'a> RealWorktreeProvisioner<'a> {
    pub fn new(app: tauri::AppHandle, state: &'a crate::AppState) -> Self {
        Self { app, state }
    }
}

impl<'a> WorktreeProvisioner for RealWorktreeProvisioner<'a> {
    fn provision(
        &self,
        req: WorktreeRequest,
        _resolved: String,
    ) -> Pin<Box<dyn Future<Output = Result<String, PhaseRunError>> + Send + '_>> {
        Box::pin(async move {
            crate::commands::git::ensure_worktree(
                &self.app,
                self.state,
                &req.task_id,
                &req.repo_path,
                &req.repository_id,
                &req.task_code,
                req.git_branch.as_deref(),
                &req.env_prefix,
            )
            .await
            .map_err(PhaseRunError::Db)
        })
    }
}

// ============================================================================
// Tests — the interface is the test surface. Fakes double as the 2nd adapter
// making each port seam real. In-memory SQLite + temp dirs; no Tauri runtime,
// no real Claude CLI.
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::claude_invocation::{ClaudeInvocation, ClaudeOutcome, ClaudeProcess};
    use crate::db::schema;
    use crate::models::Permission;
    use rusqlite::params;
    use std::sync::atomic::{AtomicU32, Ordering};

    // ---- Fakes ----

    struct FakeClaude {
        captured: Mutex<Option<ClaudeRequest>>,
        outcome: ClaudeOutcome,
    }
    impl FakeClaude {
        fn new(outcome: ClaudeOutcome) -> Self {
            Self { captured: Mutex::new(None), outcome }
        }
        fn captured(&self) -> CapturedReq {
            // ClaudeRequest isn't Clone; clone the fields we assert on instead.
            // Tests call this only after a run that spawned once.
            self.captured.lock().unwrap().as_ref().expect("no request captured").clone_fields()
        }
    }
    // ClaudeRequest clone-for-test: a tiny helper so tests can read the captured req.
    #[allow(dead_code)]
    struct CapturedReq { prompt: String, permission: Permission, model: String, working_dir: String }
    impl ClaudeRequest {
        fn clone_fields(&self) -> CapturedReq {
            CapturedReq {
                prompt: self.prompt.clone(),
                permission: self.permission,
                model: self.model.clone(),
                working_dir: self.working_dir.clone(),
            }
        }
    }
    impl ClaudeInvocation for FakeClaude {
        fn spawn(&self, req: ClaudeRequest) -> Result<ClaudeProcess, String> {
            *self.captured.lock().unwrap() = Some(req);
            let outcome = self.outcome.clone();
            Ok(ClaudeProcess {
                pid: 4242,
                completion: Box::pin(async move { outcome }),
            })
        }
    }

    #[derive(Default)]
    struct FakeEvents(Mutex<Vec<PhaseEvent>>);
    impl EventEmitter for FakeEvents {
        fn emit(&self, event: PhaseEvent) {
            self.0.lock().unwrap().push(event);
        }
    }
    impl FakeEvents {
        fn snapshot(&self) -> Vec<PhaseEvent> {
            self.0.lock().unwrap().clone()
        }
    }

    struct FakeWorktree(String);
    impl WorktreeProvisioner for FakeWorktree {
        fn provision(
            &self,
            _req: WorktreeRequest,
            _resolved: String,
        ) -> Pin<Box<dyn Future<Output = Result<String, PhaseRunError>> + Send + '_>> {
            let dir = self.0.clone();
            Box::pin(async move { Ok(dir) })
        }
    }

    // ---- Temp dir helper (no tempfile dep — unique dir under std temp_dir, removes on Drop) ----

    static COUNTER: AtomicU32 = AtomicU32::new(0);
    struct TestTempDir(std::path::PathBuf);
    impl TestTempDir {
        fn new() -> Self {
            let n = COUNTER.fetch_add(1, Ordering::SeqCst);
            let p = std::env::temp_dir().join(format!("lge-phase-runner-test-{}-{}", std::process::id(), n));
            std::fs::create_dir_all(&p).unwrap();
            Self(p)
        }
        fn path(&self) -> &std::path::Path {
            self.0.as_path()
        }
    }
    impl Drop for TestTempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    // ---- DB seed ----

    fn in_memory_db() -> Mutex<Connection> {
        let conn = Connection::open_in_memory().unwrap();
        schema::run_migrations(&conn).unwrap();
        Mutex::new(conn)
    }

    fn seed_task(
        conn: &Connection,
        task_id: &str,
        jira_key: Option<&str>,
        git_branch: Option<&str>,
    ) -> String {
        let now = "2026-01-01T00:00:00Z";
        conn.execute(
            "INSERT OR IGNORE INTO repositories (id, name, path, created_at, updated_at) \
             VALUES ('repo-1', 'test-repo', '/tmp/lge-test-repo', ?1, ?1)",
            params![now],
        ).unwrap();
        conn.execute(
            "INSERT INTO tasks (id, repository_id, title, description, status, source, jira_key, created_at, updated_at, git_branch, worktree_path) \
             VALUES (?1, 'repo-1', 'Test task', 'a desc', 'pending', 'manual', ?2, ?3, ?3, ?4, NULL)",
            params![task_id, jira_key, now, git_branch],
        ).unwrap();
        "repo-1".to_string()
    }

    fn task_status(conn: &Connection, task_id: &str) -> String {
        conn.query_row(
            "SELECT status FROM tasks WHERE id = ?1",
            params![task_id],
            |r| r.get::<_, String>(0),
        ).unwrap()
    }

    // ---- Harness ----

    fn runner<'a>(
        db: &'a Mutex<Connection>,
        sem: &'a Arc<Semaphore>,
        cancelled: &'a Mutex<HashSet<String>>,
        pids: &'a Mutex<HashMap<String, u32>>,
        claude: &'a FakeClaude,
        events: &'a FakeEvents,
        worktree: &'a FakeWorktree,
    ) -> PhaseRunner<'a, &'a FakeClaude, &'a FakeEvents, &'a FakeWorktree> {
        PhaseRunner::new(
            db, sem, cancelled, pids, claude, events, worktree,
        )
    }

    fn ctx(task_id: &str, phase: Phase) -> PhaseRunContext {
        PhaseRunContext {
            task_id: task_id.to_string(), phase,
            task_title: "T".to_string(), task_description: "D".to_string(),
            extra_context: None,
        }
    }

    #[allow(clippy::type_complexity)]
    fn queue_harness() -> (Arc<Semaphore>, Mutex<HashSet<String>>, Mutex<HashMap<String, u32>>) {
        (Arc::new(Semaphore::new(1)), Mutex::new(HashSet::new()), Mutex::new(HashMap::new()))
    }

    // ---- Tests ----

    #[tokio::test]
    async fn planning_emits_queued_then_dequeued_and_uses_plan_permission() {
        let db = in_memory_db();
        { let c = db.lock().unwrap(); seed_task(&c, "T1", Some("LGE-1"), Some("feat")); }
        let (sem, cancelled, pids) = queue_harness();
        let tmp = TestTempDir::new();
        let claude = FakeClaude::new(ClaudeOutcome { stdout: "# the plan".into(), stderr: "".into(), exit_code: Some(0) });
        let events = FakeEvents::default();
        let worktree = FakeWorktree(tmp.path().to_str().unwrap().to_string());

        let out = runner(&db, &sem, &cancelled, &pids, &claude, &events, &worktree)
            .run(ctx("T1", Phase::Planning)).await.unwrap();

        assert_eq!(events.snapshot(), vec![
            PhaseEvent::Queued { task_id: "T1".into(), phase: Phase::Planning },
            PhaseEvent::Dequeued { task_id: "T1".into(), phase: Phase::Planning },
        ]);
        let cap = claude.captured();
        assert_eq!(cap.permission, Permission::Plan);
        assert!(out.artifact_content.contains("# the plan"));
        assert_eq!(task_status(&db.lock().unwrap(), "T1"), "in_progress");
        // PID cleaned up after run
        assert!(pids.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn cancel_while_queued_returns_cancelled_and_skips_dequeued() {
        let db = in_memory_db();
        { let c = db.lock().unwrap(); seed_task(&c, "T1", Some("LGE-1"), None); }
        let (sem, cancelled, pids) = queue_harness();
        // Pre-mark the task as cancelled — the queue acquire succeeds immediately
        // (sem has 1 free permit) and the cancelled-check fires before Dequeued.
        cancelled.lock().unwrap().insert("T1".into());
        let claude = FakeClaude::new(ClaudeOutcome { stdout: "".into(), stderr: "".into(), exit_code: Some(0) });
        let events = FakeEvents::default();
        let worktree = FakeWorktree("/tmp/unused".into());

        let err = runner(&db, &sem, &cancelled, &pids, &claude, &events, &worktree)
            .run(ctx("T1", Phase::Planning)).await.unwrap_err();

        assert!(matches!(err, PhaseRunError::Cancelled));
        assert_eq!(events.snapshot(), vec![PhaseEvent::Queued { task_id: "T1".into(), phase: Phase::Planning }]);
    }

    #[tokio::test]
    async fn claude_failure_returns_claude_failed_and_leaves_no_pid() {
        let db = in_memory_db();
        { let c = db.lock().unwrap(); seed_task(&c, "T1", Some("LGE-1"), Some("feat")); }
        let (sem, cancelled, pids) = queue_harness();
        let claude = FakeClaude::new(ClaudeOutcome { stdout: "".into(), stderr: "boom".into(), exit_code: Some(1) });
        let events = FakeEvents::default();
        let worktree = FakeWorktree(std::env::temp_dir().to_str().unwrap().to_string());

        let err = runner(&db, &sem, &cancelled, &pids, &claude, &events, &worktree)
            .run(ctx("T1", Phase::Builder)).await.unwrap_err();

        assert!(matches!(err, PhaseRunError::ClaudeFailed { .. }));
        assert!(pids.lock().unwrap().is_empty()); // I1: PID removed even on failure
    }

    #[tokio::test]
    async fn missing_artifact_returns_artifact_missing_for_non_planning() {
        let db = in_memory_db();
        { let c = db.lock().unwrap(); seed_task(&c, "T1", Some("LGE-1"), Some("feat")); }
        let (sem, cancelled, pids) = queue_harness();
        let tmp = TestTempDir::new();
        // FakeWorktree returns tmp; we do NOT pre-write builder.md → read fails.
        let claude = FakeClaude::new(ClaudeOutcome { stdout: "ok".into(), stderr: "".into(), exit_code: Some(0) });
        let events = FakeEvents::default();
        let worktree = FakeWorktree(tmp.path().to_str().unwrap().to_string());

        let err = runner(&db, &sem, &cancelled, &pids, &claude, &events, &worktree)
            .run(ctx("T1", Phase::Builder)).await.unwrap_err();

        assert!(matches!(err, PhaseRunError::ArtifactMissing { phase: Phase::Builder, .. }));
    }

    #[tokio::test]
    async fn guardian_reads_artifact_and_sets_status_completed() {
        let db = in_memory_db();
        { let c = db.lock().unwrap(); seed_task(&c, "T1", Some("LGE-1"), Some("feat")); }
        let (sem, cancelled, pids) = queue_harness();
        let tmp = TestTempDir::new();
        // Pre-write the guardian artifact where the run will look for it.
        let art_dir = tmp.path().join("docs/tasks/LGE-1");
        std::fs::create_dir_all(&art_dir).unwrap();
        std::fs::write(art_dir.join("guardian.md"), "# verdict here").unwrap();
        let claude = FakeClaude::new(ClaudeOutcome { stdout: "".into(), stderr: "".into(), exit_code: Some(0) });
        let events = FakeEvents::default();
        let worktree = FakeWorktree(tmp.path().to_str().unwrap().to_string());

        let out = runner(&db, &sem, &cancelled, &pids, &claude, &events, &worktree)
            .run(ctx("T1", Phase::Guardian)).await.unwrap();

        assert_eq!(out.artifact_content, "# verdict here");
        assert_eq!(task_status(&db.lock().unwrap(), "T1"), "completed");
        // Guardian is non-planning → no queue events
        assert!(events.snapshot().is_empty());
    }

    #[tokio::test]
    async fn attachment_context_is_merged_into_prompt() {
        let db = in_memory_db();
        {
            let c = db.lock().unwrap();
            seed_task(&c, "T1", Some("LGE-1"), Some("feat"));
            // Seed an attachment for the builder phase. injection_phase is a JSON
            // array after migration 007 (e.g. ["builder"]).
            c.execute(
                "INSERT INTO task_attachments (id, task_id, file_name, file_size, mime_type, content, injection_phase, created_at) \
                 VALUES ('a1', 'T1', 'spec.md', 100, 'text/markdown', 'do the thing', '[\"builder\"]', '2026-01-01T00:00:00Z')",
                [],
            ).unwrap();
        }
        let (sem, cancelled, pids) = queue_harness();
        let tmp = TestTempDir::new();
        let art_dir = tmp.path().join("docs/tasks/LGE-1");
        std::fs::create_dir_all(&art_dir).unwrap();
        std::fs::write(art_dir.join("builder.md"), "x").unwrap();
        let claude = FakeClaude::new(ClaudeOutcome { stdout: "".into(), stderr: "".into(), exit_code: Some(0) });
        let events = FakeEvents::default();
        let worktree = FakeWorktree(tmp.path().to_str().unwrap().to_string());

        runner(&db, &sem, &cancelled, &pids, &claude, &events, &worktree)
            .run(ctx("T1", Phase::Builder)).await.unwrap();

        let cap = claude.captured();
        assert!(cap.prompt.contains("## Contexto Adicional"));
        assert!(cap.prompt.contains("do the thing"));
        assert!(cap.prompt.contains("spec.md"));
    }
}
