# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Prerequisites

- Node.js + pnpm (`packageManager: pnpm@10.18.3`)
- Rust toolchain (rustup)
- Tauri CLI 2.x (`@tauri-apps/cli` installed as devDep)

## Build & Development Commands

```bash
pnpm tauri dev          # Run full app in dev mode (frontend + Rust backend)
pnpm tauri build        # Build production .app and .dmg bundles
pnpm build              # Frontend only: tsc + vite build
pnpm dev                # Frontend only: Vite dev server on port 1420
```

No lint, test, or format scripts are configured. Use `cargo clippy` / `cargo test` for Rust-side checks.

Rust backend compiles automatically via Tauri when running `pnpm tauri dev/build`. To compile Rust only: `cd src-tauri && cargo build`.

## Architecture

**Tauri 2.x desktop app** — Rust backend (SQLite, filesystem, Jira) + React/TypeScript frontend (Vite, Tailwind CSS 4, Zustand).

### IPC Bridge

Frontend calls Rust via Tauri `invoke()`. All commands are typed in `src/lib/tauri.ts` and registered in `src-tauri/src/lib.rs` via `generate_handler![]`.

Flow: **React component → Zustand store action → `invoke("command_name")` → Rust `#[tauri::command]` fn → SQLite query → return serialized result**

### Backend (src-tauri/src/)

- `lib.rs` — App setup: initializes SQLite, registers plugins (dialog, shell, log, notification), registers all Tauri commands. `AppState` holds `Mutex<Connection>` (SQLite), `Mutex<HashMap<String, u32>>` (running LGE PIDs), `Arc<Semaphore>` (planning queue — serializes planning phases), and `Mutex<HashSet<String>>` (tracks cancelled-while-queued tasks)
- `commands/` — Tauri command handlers: `repositories.rs`, `tasks.rs`, `jira.rs`, `lge.rs` (LGE phase execution), `health.rs` (dependency checking), `claude_utils.rs` (Claude CLI path resolution + `shell_escape`), `settings.rs` (AI model config per phase), `git.rs` (branch & worktree operations), `arch_diff.rs` (live architecture diff analysis), `attachments.rs` (per-task file attachments injected into LGE prompts)
- `db/queries.rs` — Raw SQL CRUD operations (no ORM); always uses `params![]` placeholders
- `db/schema.rs` — Migration runner using `include_str!` of SQL files from `migrations/`
- `models/` — Serde-serializable structs: `Repository`, `Task`, `TaskStatus`, `TaskSource`, `LgePhaseResult` (`lge.rs`), `TaskAttachment` (`attachment.rs`), `ArchitectureDiff` (`arch_diff.rs`)

### Frontend (src/)

- `stores/` — Zustand stores: `repositoryStore.ts`, `taskStore.ts`, `lgeStore.ts` (LGE execution state: running phases, results, artifacts), `settingsStore.ts` (AI model config per LGE phase)
- `components/layout/` — App shell: TopBar (workflow stages), Sidebar (repo list), StatusBar (language switcher), HealthCheck (startup dependency verification), StaleWorktreeAlert (stale git worktree warnings), WhatsNewDialog (release notes)
- `components/repositories/` — Repository management UI (add/remove repo dialogs)
- `components/tasks/` — Task CRUD UI: TaskList, TaskItem, TaskDetail, CreateTaskDialog, ImportJiraDialog, attachments
- `components/lge/` — LGE workflow UI: LgeProcessView, LgePhasePipeline, LgeArtifactPanel
- `components/lge/arch-diff/` — Live Architecture Diff panel (on-demand impact analysis per phase)
- `components/settings/` — SettingsDialog (AI model configuration per LGE phase)
- `components/ui/` — Shared primitives: Button, Dialog, Input/TextArea, Badge
- `types/index.ts` — Core TypeScript types for all domain models and LGE phases
- `lib/constants.ts` — Status config mapping (labels, colors per task status)
- `data/releaseNotes.ts` — Release notes data for WhatsNew dialog
- `demo/` — Demo mode: `demoStore.ts`, `demoArtifacts.ts`, `useDemoMode.ts` — provides sample data for showcasing the app without a real backend

### LGE Phases

The app orchestrates four sequential AI-powered phases per task: `planning → builder → review → guardian`. Each phase calls `invoke("run_lge_phase")` which spawns a subprocess tracked in `AppState.running_pids`. Results are stored as artifacts accessible via `invoke("load_lge_artifacts")`. Phases can be cancelled via `invoke("cancel_lge_phase")`.

Key types in `src/types/index.ts`: `LgePhaseId`, `LgePhaseStatus` (`pending | queued | running | completed | failed`), `LgePhaseResult` (`phase`, `artifact_content`, `artifact_path`). The `queued` status is used when a planning phase is waiting for the serialization semaphore.

### Data Flow

Selecting a repo in Sidebar triggers `fetchTasks(repoId)` in taskStore, which calls `invoke("list_tasks")`. Task status cycles: pending → in_progress → completed → pending.

## Database

SQLite stored at `~/Library/Application Support/com.lge.cockpit/lge-cockpit.db`. Migrations in `src-tauri/migrations/` (`001_initial.sql`, `002_git_branch.sql`, `003_settings.sql`, `004_worktrees.sql`, `005_shell_env.sql`, `006_task_attachments.sql`, `007_attachment_phases_multi.sql`, `008_jira_base_url.sql`). WAL mode + foreign keys enabled. IDs are UUID v4, timestamps are RFC3339.

CASCADE delete: removing a repository deletes all its tasks.

For the migration workflow (file naming, schema.rs registration, model + queries updates), see `.claude/rules/sqlite-migration.md`.

## i18n

Three languages: pt-BR (default), en, es. Translation files in `src/i18n/*.json`. All UI text uses `useTranslation()` hook with keys like `t("tasks.create")`. When adding UI text, update all three JSON files.

## Styling

Tailwind CSS 4 with custom theme defined in `src/styles/globals.css` using `@theme`. Key colors: `bg-primary` (#0f0f1a), `accent` (#7c3aed/violet), `success`/`warning`/`error` for status badges.

## Quality Standards

The project has no test runner or linter configured, so consistency is enforced through documented patterns. Detailed constraints live in `.claude/rules/`:

- `tauri-command-flow.md` — adding/renaming an IPC command (Rust + lib.rs + tauri.ts + store)
- `sqlite-migration.md` — migration naming and `schema.rs` registration
- `subprocess-shell-escape.md` — `shell_escape` for every user-controlled value passed to `bash -lc`
- `version-sync.md` — bump `package.json`, `tauri.conf.json`, `Cargo.toml`, `releaseNotes.ts` together
- `i18n-parity.md` — keep `pt-BR/en/es` JSON files in lockstep
- `zustand-store-conventions.md` — error handling and state-update patterns

Baseline expectations:

- **Errors**: Tauri commands return `Result<T, String>` so the JS side can `.catch()` and show feedback.
- **SQL**: Every query uses `params![]` or `?` placeholders. No string-formatting of user input into SQL. All SQL belongs in `db/queries.rs` — keep `commands/*.rs` free of inline SQL.
- **Subprocesses**: Every `format!` building a shell command for `bash -lc` must wrap user-controlled values in `claude_utils::shell_escape`.
- **Secrets**: Jira tokens and similar credentials live in the SQLite `settings` table, never in code, env files, or logs.

## Gotchas

- **Planning phase is serialized:** Only one planning phase runs at a time (enforced by `planning_semaphore`). Others queue with `"queued"` status. Cancelling a queued task requires checking `planning_cancelled` set, not just killing a PID.
- **Committed .dmg artifacts:** Root directory contains built `.dmg` files — these should not be committed. Consider adding `*.dmg` to `.gitignore`.

## Versioning & What's New

When making significant changes (new features, major bug fixes, UI overhauls), always:

1. **Bump the version** in `package.json`, `src-tauri/tauri.conf.json`, and `src-tauri/Cargo.toml` — all three must stay in sync
2. **Add a release note entry** in `src/data/releaseNotes.ts` describing the new feature/change so it appears in the What's New dialog

## Adding a New Tauri Command

1. Write the function in `src-tauri/src/commands/*.rs` with `#[tauri::command]`
2. Register it in `lib.rs` inside `generate_handler![]`
3. Add typed wrapper in `src/lib/tauri.ts`
4. Call from Zustand store or component via the wrapper
