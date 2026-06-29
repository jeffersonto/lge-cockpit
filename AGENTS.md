# AGENTS.md

Compact agent guide for LGE Cockpit. The canonical, detailed architecture walkthrough lives in `CLAUDE.md` — read it for anything beyond the points below. Repo-specific workflow rules live in `.claude/rules/` and are authoritative for their topics.

## Commands

```bash
pnpm tauri dev          # Full app dev (Vite :1420 + Rust backend, hot-reload). Use this by default.
pnpm tauri build        # Production .app + .dmg
pnpm dev                # Frontend only — no Rust backend, IPC will fail at runtime. Use only for pure UI work.
pnpm build              # tsc --noEmit + vite build (frontend typecheck/build)
```

- **No frontend lint/test/format scripts exist.** Don't run `npm test`/`npm run lint` — they aren't configured. For Rust checks: `cargo clippy` and `cargo test` in `src-tauri/`.
- Rust compiles automatically via Tauri when running `pnpm tauri dev/build`. To compile Rust only: `cd src-tauri && cargo build`.
- `tsconfig.json` enables `noUnusedLocals`/`noUnusedParameters`/`strict` — unused imports/vars fail `pnpm build`.

## Architecture (non-obvious bits)

- **Tauri 2.x IPC is the spine.** Every frontend→backend call goes through `invoke()` typed in `src/lib/tauri.ts` and registered in `src-tauri/src/lib.rs` `generate_handler![]`. Flow: component → Zustand store → invoke → `#[tauri::command]` → SQLite (`db/queries.rs`) → serialized result.
- **Adding/renaming a command is a 4-step update**: Rust handler → `lib.rs` registration → `tauri.ts` typed wrapper → Zustand store/component. See `.claude/rules/tauri-command-flow.md`.
- **LGE pipeline** runs four phases per task: `planning → builder → review → guardian`, each spawning a subprocess tracked by PID in `AppState`.
  - **Planning is serialized** via `AppState.planning_semaphore`. Concurrent planning phases get `"queued"` status, not `running`.
  - **Cancelling a queued planning phase** requires checking `AppState.planning_cancelled` (a `HashSet`), not just killing a PID. A PID kill alone leaves queued tasks running.
- Frontend demo mode (`src/demo/`) ships sample data so the UI can run without a real backend — be aware its data is fake when debugging store behavior.

## Conventions that differ from defaults

- **All SQL belongs in `src-tauri/src/db/queries.rs`.** Keep `commands/*.rs` free of inline SQL. Every query uses `params![]` / `?` placeholders — never string-format user input into SQL.
- **Subprocesses:** any `format!` building a command for `bash -lc` must wrap user-controlled values in `claude_utils::shell_escape`. See `.claude/rules/subprocess-shell-escape.md`.
- **Tauri commands return `Result<T, String>`**, so the JS side `.catch()`es and surfaces errors to the user.
- **Secrets** (Jira tokens, etc.) live only in the SQLite `settings` table — never in code, env files, or logs.
- **Zustand stores** follow the error-handling/state-update patterns in `.claude/rules/zustand-store-conventions.md`.

## Database

- SQLite at `~/Library/Application Support/com.lge.cockpit/lge-cockpit.db`. WAL + foreign keys on. UUIDs v4, RFC3339 timestamps. CASCADE delete: removing a repository deletes all its tasks.
- Migrations are numbered `NNN_name.sql` in `src-tauri/migrations/` and registered in `src-tauri/src/db/schema.rs`. Adding one means migration file + `schema.rs` registration + (if needed) model + `queries.rs` updates. See `.claude/rules/sqlite-migration.md`.

## i18n

Three languages: `src/i18n/{pt-BR,en,es}.json` (pt-BR is default). All UI text uses `useTranslation()` with keys like `t("tasks.create")`. **Adding any UI text requires updating all three JSON files in lockstep** (`.claude/rules/i18n-parity.md`).

## Versioning

Versions must stay in sync across **four** files: `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, and `src/data/releaseNotes.ts`. On significant changes, also add a release-notes entry. See `.claude/rules/version-sync.md`.

## Gotchas

- `*.dmg` files in the repo root are generated build artifacts — already in `.gitignore`; do not stage them.
- `src-tauri/gen/` is Tauri-generated — do not hand-edit.
- Tailwind CSS 4 theme is defined in `src/styles/globals.css` via `@theme` (custom `bg-primary`, `accent`, `success`/`warning`/`error`). Stick to these tokens for status colors.
- The `.claude/skills/build-dmg` skill automates DMG staging at the repo root; use it for release builds rather than running the `bundle_dmg.sh` command from the README manually.