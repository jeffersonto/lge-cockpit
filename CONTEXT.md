# CONTEXT.md

Domain glossary for LGE Cockpit. Canonical vocabulary for the domain; architecture vocabulary (module, interface, depth, seam, adapter, leverage, locality) lives in the `codebase-design` skill and is not duplicated here. Terms are added lazily as deepened modules are named and fuzzy concepts are sharpened.

## LGE Phase

One of the four sequential AI-powered phases executed per task: **planning → builder → review → guardian**.

- Each phase has a canonical artifact filename (planning → `plan.md`, builder → `builder.md`, review → `review.md`, guardian → `guardian.md`), a default model (planning → opus, builder → haiku, review → sonnet, guardian → opus), a permission mode (planning → `plan`; the others → skip-permissions), and a prompt template.
- Planning is special: it runs under `--permission-mode plan` (read-only), does **not** receive the `ARTIFACT_LOCATION_RULES` block in its prompt, and its artifact is retrieved from `~/.claude/plans/` (currently by a 5-minute mtime heuristic — known race, to be fixed in the PhaseRunner work, not in Phase).
- Phases are serialized at the planning stage: only one planning phase runs at a time; others queue with `"queued"` status.

### Phase module (deepened)

Rust enum `Phase { Planning, Builder, Review, Guardian }` living at `src-tauri/src/models/phase.rs`. Owns the per-phase contract — a pure value-object, **no IO**:

- `artifact_filename() -> &'static str` — canonical artifact name.
- `legacy_filenames() -> &'static [&'static str]` — historical names (builder → `["builder-model-summary.md"]`, review → `["reviewer-model-summary.md"]`, guardian → `["guardian-model.md"]`). Sunset TODO: a one-shot migration renames old files and this method is removed.
- `default_model() -> &'static str` — static default. The Settings module (architecture candidate 05) owns the dynamic override (reads `model_{phase}` from the settings table) and falls back to this.
- `permission_mode() -> Permission` — returns `Permission::Plan` for planning, `Permission::SkipPermissions` for the rest. Phase does not know CLI flag strings; translating `Permission` → `"--permission-mode plan"` / `"--dangerously-skip-permissions"` is the ClaudeInvocation's job (candidate 01/07).
- `build_prompt(ctx: &PromptContext) -> String` — assembles the phase prompt. Prompt text lives in `src-tauri/prompts/{phase}.md` loaded via `include_str!`. `ARTIFACT_LOCATION_RULES` and `MARKDOWN_FORMAT_RULES` are private helpers internal to this method, not part of the interface. Planning's prompt omits the location rules block.

**Boundary:** pure value-object, no IO. Artifact retrieval (reading the file back, including the planning `~/.claude/plans` scan and the 5-minute race) is **not** Phase's job — it belongs to the PhaseRunner (architecture candidate 01).

**Wire format:** lowercase strings (`"planning"`, `"builder"`, …). The enum serializes/deserializes via serde to the same lowercase strings, so the frontend stays string-based (`LgePhaseId` union in `src/types/index.ts:53`) and is untouched by this refactor.

**Tests:** `#[cfg(test)]` unit tests in the module, run via `cargo test` in `src-tauri/`. Pin every contract: filename, legacy names, default model, permission mode, serde round-trip (enum ↔ lowercase string), and prompt substitution (incl. the planning-omits-rules invariant).

---

## PhaseRunner

The deep module that owns the **end-to-end orchestration of one LGE phase execution**: acquire the planning queue → look up task metadata → resolve/provision the working directory → fetch phase attachments → assemble the prompt (delegated to `Phase`) → invoke Claude → retrieve the artifact → update task status. It replaces the 220-line `run_lge_phase` procedure; the `#[tauri::command] run_lge_phase` becomes a thin adapter that constructs a `PhaseRunner` with real adapters, calls `run()`, sends the system notification based on the outcome, and returns the `LgePhaseResult`.

**Boundary:** PhaseRunner is **impure** (spawns subprocesses, touches DB/FS) — unlike `Phase`. To be testable without the Tauri runtime or a real Claude CLI, it takes its true-external and Tauri-coupled dependencies as **ports** (traits), each with a real adapter (production) and a fake adapter (tests) — two adapters per port, so every seam is real, not speculative. The DB (`Mutex<Connection>`) and filesystem are local-substitutable (in-memory SQLite + temp dirs in tests) and are **not** ported.

### Ports (three)

- **`ClaudeInvocation`** — spawns the Claude CLI and collects stdout/stderr. The `Permission → CLI flag` translation (`Plan → --permission-mode plan`, `SkipPermissions → --dangerously-skip-permissions`) lives in its real adapter, NOT in `Phase` or `PhaseRunner`. This is the generic port that architecture candidate 07 (Jira session) will reuse; it lives in its own file (`claude_invocation.rs`) so C07 can import it without dragging in PhaseRunner. A pure `build_claude_command(...)` helper sits behind it for unit-testing the shell-escaping and flag assembly.
- **`EventEmitter`** — emits the mid-run `lge_phase_queued` / `lge_phase_dequeued` events (fired during planning-queue acquire). Real adapter wraps `tauri::AppHandle::emit`; fake collects into a `Vec` for assertions. System **notifications** (success/failure) are NOT here — they are presentation, owned by the Tauri command adapter, driven by the `PhaseOutcome`.
- **`WorktreeProvisioner`** — resolves the task's working dir and materializes a worktree on demand (the only other shell user besides `ClaudeInvocation`; absorbs the three-policies drift that candidate 06 will fully consolidate). Real adapter calls `git::ensure_worktree`; fake returns a temp dir.

### Inline concern (not yet a port)

- **Planning queue** — the semaphore + `planning_cancelled` set + `running_pids` map (three coordinated `AppState` fields) stay **inline** in `PhaseRunner` for now. They are local-substitutable (in-memory semaphore + fake clock). Architecture candidate 03 (`PhaseProcessRegistry`) will extract them into their own module; until then PhaseRunner owns them with a clear seam for extraction.

### Interface (shape)

```text
PhaseRunner<'a, C: ClaudeInvocation, E: EventEmitter, W: WorktreeProvisioner>
  run(ctx: PhaseRunContext) -> Result<PhaseOutcome, PhaseRunError>

PhaseRunContext { task_id, phase: Phase, task_title, task_description, extra_context: Option<String> }
PhaseOutcome    { artifact_content: String, artifact_path: String }
PhaseRunError   = Cancelled | ClaudeFailed(stderr) | ArtifactMissing{phase, path} | Db(msg) | Io(msg)
```
Generics (not `&dyn`) so native `async fn` in traits works without the `async_trait` crate. The DB is passed as `&Mutex<Connection>`; the three AppState queue fields are passed by reference.

### Retrieval & the 5-minute race

PhaseRunner owns artifact **retrieval** (decided during the `Phase` grilling — `Phase` is pure and stops at the contract). For builder/review/guardian it reads `docs/tasks/{code}/{filename}` from disk; for planning it calls the existing `resolve_plan_file` (the 5-minute mtime scan of `~/.claude/plans/`). The race (two plannings within 5 min → wrong plan on wrong task) is **pre-existing and explicitly out of scope for C01** — marked TODO to fix once `claude --permission-mode plan`'s support for `--output-format json` is confirmed (plan would come back in stdout, killing the scan).

---

## Jira Integration

Boundary between the app and Atlassian Jira Cloud. Responsible for fetching issue data and, in the future, posting updates back to Jira.

### Jira Client

The deep module that owns all communication with the Jira Cloud REST API. Exposes a small, domain-oriented interface (e.g., `get_issue`) and hides HTTP, authentication, and Atlassian-specific formats behind an adapter.

_Avoid_: MCP, Claude CLI, `mcp-atlassian`, Jira importer.

### Jira Credentials

The authentication pair used to call Jira Cloud: the user's Atlassian account email and an API token generated in the Atlassian account settings. Stored as app settings; never logged or committed.

_Avoid_: Password, PAT, OAuth token, MCP token.

### Jira Issue Import

The operation of reading a Jira issue by key and creating a local `Task` from it. Captures summary, status, URL, and a Markdown representation of the description.

_Avoid_: Jira sync, Jira fetch, MCP import.

### Imported Task

A `Task` whose `source` is `jira` and that carries a `jira_key`. The source records that the task originated from Jira, regardless of which integration mechanism was used at the time of import.

_Avoid_: Jira task, external task.

### Issue Description Conversion

The transformation of a Jira issue description from Atlassian's rendered HTML into GitHub-Flavored Markdown before it is stored in a `Task`. Preserves tables, code blocks, lists, and links.

_Avoid_: ADF conversion, description parsing.

### Jira Connection Test

A lightweight operation that verifies the configured `Jira Credentials` and `Jira base URL` by calling an authenticated Jira endpoint. Used from Settings before any import is attempted.

_Avoid_: Jira diagnostic, MCP health check.


### Tests

`#[cfg(test)]` in `phase_runner.rs`, run via `cargo test`. With three fake ports + in-memory SQLite + temp dirs, the whole orchestration is testable: queue acquire emits queued→dequeued; planning invokes with `Permission::Plan`; guardian updates status to `completed`; cancel-while-queued returns `PhaseRunError::Cancelled`; Claude failure returns `ClaudeFailed`; missing artifact returns `ArtifactMissing`; attachment context is merged into the prompt (asserted via a fake `ClaudeInvocation` that captures the request).
