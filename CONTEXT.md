# CONTEXT.md

Domain glossary for LGE Cockpit. Canonical vocabulary for the domain; architecture vocabulary (module, interface, depth, seam, adapter, leverage, locality) lives in the `codebase-design` skill and is not duplicated here. Terms are added lazily as deepened modules are named and fuzzy concepts are sharpened.

> Implemented: C05 (`Settings`, `ShellEnv`), C08 (`DiffAnalysis`), C07 (`CommitMessageRunner`, `Permission::None`, `ClaudeOutcome::result`) all landed 2026-07-04 with 48 new unit tests (40 → 88 total).

## LGE Phase

One of the four sequential AI-powered phases executed per task: **planning → builder → review → guardian**.

- Each phase has a canonical artifact filename (planning → `plan.md`, builder → `builder.md`, review → `review.md`, guardian → `guardian.md`), a default model (planning → opus, builder → haiku, review → sonnet, guardian → opus), a permission mode (planning → `plan`; the others → skip-permissions), and a prompt template.
- Planning is special: it runs under `--permission-mode plan` (read-only), does **not** receive the `ARTIFACT_LOCATION_RULES` block in its prompt, and its artifact is retrieved from `~/.claude/plans/` (currently by a 5-minute mtime heuristic — known race, to be fixed in the PhaseRunner work, not in Phase).
- Phases are serialized at the planning stage: only one planning phase runs at a time; others queue with `"queued"` status.
- **pt-BR labels (2026-07-06 decision):** Planning → "Planejamento", Builder → "Construção", Review → "Revisão", Guardian → "Guardião". Previously all four were left untranslated (English) in `pt-BR.json`/`es.json`, violating `i18n-parity.md`. "Revisão" is reserved for this phase specifically — see [[Task Review Request]] for why the PR/approval step is named "Aprovação" instead, to avoid collision.

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

## Task Workspace

The folder + isolated git working tree where a task's code changes live during development (backed internally by `WorktreeProvisioner` / `task.worktree_path`). Surfaced to non-technical users in the UI as **"Área de Trabalho"** — the rename is UI copy only, no change to the underlying data model or Rust-side naming (`worktree` stays the term in code, migrations, and `CONTEXT.md`'s `PhaseRunner` section above).

_Avoid_ (in user-facing copy only): Worktree, workspace folder.

## Task Work Record

The git branch created per task (`task.git_branch`), surfaced to non-technical users as **"Registro de Trabalho"**; the action that creates it is labeled **"Iniciar Registro"** (was "Criar Branch"). The badge still displays the raw branch name (e.g. `feature/oauth2-login`) — only the surrounding label changes, not the value.

_Avoid_ (in user-facing copy only): Branch, Git Branch.

## Task Review Request

The GitHub Pull Request opened once a task's LGE pipeline (Guardian) completes, surfaced to non-technical users as **"Aprovação"**: panel title "Pronto para Aprovação", action button "Enviar para Aprovação", and the single text field it collects — the same text used for both the git commit and the context handed to the reviewer — labeled "Descrição da Aprovação" (was "Mensagem de commit"). The underlying artifact is still a real GitHub Pull Request; "Pull Request"/"PR" stays untranslated in dev-facing surfaces only (the manual-fallback terminal command, code, anywhere a developer cross-references GitHub directly), since GitHub itself uses that vocabulary and the developer needs the two to match.

Named "Aprovação" rather than "Revisão" specifically to avoid colliding with the LGE **Review** phase (see below), which independently translates to "Revisão" — the two are different moments (AI self-check mid-pipeline vs. asking a human to approve the final result) and must not share a label.

_Avoid_ (in non-technical-facing copy only): Pull Request, PR, commit, push, diff, Revisão (reserved for the LGE phase).

## Task Development (non-technical UI label for "LGE Process")

The end-to-end run of the four LGE phases for a task, surfaced to non-technical users as **"Desenvolvimento"** ("Iniciar Desenvolvimento", "Desenvolvimento concluído!", "Nenhum desenvolvimento em andamento") — replacing "Processo LGE" in functional, action-oriented copy. "LGE"/"LGE Cockpit" is kept only as the product's brand name (app title, health-check screen), never in day-to-day task copy.

_Avoid_ (in non-technical-facing copy only): Processo LGE, LGE process.

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

---

## Settings

The deep module that owns all reads and writes of the app's key/value configuration (the SQLite `settings` table). Replaces the scattered `get_setting` / `set_setting` ad-hoc reads and the per-key pass-through Tauri commands. SQL itself stays in `db/queries.rs` (per AGENTS.md convention); Settings is the typed client above it. `get_setting` / `set_setting` are `pub(crate)` in `queries.rs` and used only by Settings.

_Avoid_: settings helper, config service, preferences.

### ShellEnv

Value-object wrapping the user-customized shell prefix derived from the `shell_env` setting. Owns the parsing invariant — each non-comment, non-blank line terminated with `;` and the whole joined with a trailing `"; "` — so a caller receives "ready to prepend to a `bash -lc` string", not a raw setting value. `ShellEnv::from_raw(&str)` is public so the parser is unit-testable without SQLite; `ShellEnv::empty()` exists for fakes in tests of dependent modules.

_Avoid_: shell prefix string, env prefix, shell config, raw shell_env.

### Settings interface (decided 2026-07-04)

```text
pub struct ShellEnv(String);                     // value-object, see above

Settings::jira_config(conn)            -> JiraConfig          // JiraConfig owned by jira/ module
Settings::save_jira(conn, &JiraConfig) -> Result<(), String>
Settings::shell_env(conn)              -> ShellEnv             // callers of subprocesses
Settings::shell_env_raw(conn)          -> String                // UI edit field
Settings::save_shell_env(conn, &str)   -> ()
Settings::phase_model(conn, Phase)      -> String                // fallback to Phase::default_model()
Settings::phase_models(conn)           -> HashMap<Phase,String> // all 4, fully resolved (for IPC)
Settings::save_phase_models(conn, &HashMap<String,String>) -> Result<(), String>  // validates Phase + VALID_MODELS
```

- `JiraConfig` stays defined in `jira/mod.rs` (the Jira domain owns its credential shape); Settings imports it and gained `Serialize`/`Deserialize` (+ `#[serde(rename_all = "camelCase")]`) so the collapsed IPC command `save_jira_config` carries it end-to-end.
- IPC collapse: 6 per-key Jira get/save commands → 2 (`get_jira_config`, `save_jira_config`). Frontend `settingsStore.ts` deletes `DEFAULT_MODELS` (the `get_phase_models` IPC now returns the full four-key map with fallback applied — single source of truth in Rust).

_Avoid_: per-key settings commands, settings table readers scattered across modules.

---

## DiffAnalysis

The deep module that owns the pure transformation of git diff outputs into an `ArchitectureDiff`. Lifts ~800 lines of pure parsers, file-tree builders, dependency-graph builders, and risk scoring out of `commands/arch_diff.rs` so the two `#[tauri::command]`s shrink to git-plumbing adapters that collect `name-status`, `numstat`, and full `diff` strings and hand them to `analyze`. Untracked-file line counting (`build_numstat_with_untracked`) stays in the adapter — `analyze` is pure (no filesystem).

_Avoid_: arch diff service, diff parser, analyzer.

### DiffAnalysis interface (decided 2026-07-04)

```text
pub struct AnalysisInput {
    pub base: String,
    pub head: String,
    pub name_status: String,
    pub numstat: String,
    pub full_diff: String,
    pub max_diff_bytes: usize,
}

pub struct DiffAnalysis {            // No `phase` field — that's an LGE concept, not a diff concept.
    pub base_commit: String,
    pub head_commit: String,
    pub summary: ChangeSummary,
    pub file_tree: Vec<FileNode>,
    pub dependency_graph: DependencyGraph,
    pub api_surface: Vec<ApiChange>,
}

pub fn analyze(input: &AnalysisInput) -> DiffAnalysis
```

- Parsers/helpers (`parse_name_status`, `parse_numstat`, `parse_diff_content`, `detect_import`, `detect_api_symbol`, `detect_new_dependencies`, `build_file_tree`, `build_dependency_graph`, `calculate_risk`, `extract_quoted`) are private; `MAX_MERMAID_NODES` is a private `const`. `max_diff_bytes` is injected (caller's call) — tests can pass a small cap to exercise truncation without producing 50KB of fixture.
- Tests live in `#[cfg(test)]` inside the module, hitting helpers privately and `analyze` from outside; larger fixtures load via `include_str!` from `diff_analysis/fixtures/*.txt` so diffs with real newlines/escapes stay readable.
- The command adapter wraps: sets `phase: String::new()` when constructing the IPC-bound `ArchitectureDiff` (the `phase` field is owned by the caller, not by DiffAnalysis).

---

## CommitMessageRunner

The deep module that owns generating a Conventional Commit message for staged changes by invoking the Claude CLI. Replaces the hand-rolled `echo prompt | claude --print --model haiku` assembly duplicated in `commands/git.rs::generate_commit_message` and routes through the `ClaudeInvocation` port — making the seam real (second production caller besides `PhaseRunner`) and the JSON `result` extraction shared via `ClaudeOutcome::result()`.

_Avoid_: commit message generator, claude commit service.

### CommitMessageRunner interface (decided 2026-07-04)

```text
pub struct CommitMessageRunner<C: ClaudeInvocation> { claude: C }

impl<C: ClaudeInvocation> CommitMessageRunner<C> {
    pub fn new(claude: C) -> Self
    pub async fn generate(&self, input: &CommitMessageInput) -> Result<String, String>
}

pub struct CommitMessageInput {
    pub task_title: String,
    pub scope: String,                // "" when no scope
    pub diff_stat: String,
    pub diff_preview: String,
    pub working_dir: String,
    pub env_prefix: ShellEnv,
    pub model: String,
}
```

- `Permission::None` (no `--permission-mode` flag) and `max_turns: Some(1)` are added to `ClaudeRequest`/`Permission`/`build_claude_command` so the commit-message invocation can opt out of permission flags and cap turns without forcing the same on `PhaseRunner`. `build_claude_command` emits `--max-turns N` when `Some`.
- The commit-message prompt template stays inline in the runner (specific variables — `task_title`, `scope`, `diff_stat`, `diff_preview`, `Rules block` — don't fit `Phase::build_prompt`/`PromptContext`).
- `ClaudeOutcome::result(&self) -> String` is the single place that parses the JSON `result` wrapper (with raw-stdout fallback); both `PhaseRunner::extract_artifact` and `CommitMessageRunner::generate` use it. Per-domain normalization (artifact content vs. first-line + scope fallback) stays in each caller.
- The `#[tauri::command] generate_commit_message` in `commands/git.rs` becomes a thin adapter: resolves task_title + scope + working_dir + env_prefix from DB/git, runs `run_git` for `diff --cached --stat` and `diff --cached`, builds `CommitMessageInput`, constructs `CommitMessageRunner::new(RealClaudeInvocation::new(app))`, awaits `generate`.
