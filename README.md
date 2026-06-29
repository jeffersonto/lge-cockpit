# LGE Cockpit

Desktop app for orchestrating **Layered GenAI Engineering (LGE)** workflows. Manages repositories, tasks, and executes AI-powered phases (Planning, Builder, Review, Guardian) through an integrated pipeline.

Built with **Tauri 2.x** — Rust backend + React/TypeScript frontend.

<p align="center">
  <img src="src-tauri/icons/128x128@2x.png" alt="LGE Cockpit" width="128" />
</p>

## Features

- Repository management with Git branch tracking
- Task creation (manual or imported from Jira)
- Four-phase LGE pipeline execution with real-time status
- Artifact viewer with Markdown rendering
- Configurable AI model per phase
- Multi-language UI (pt-BR, en, es)
- Git worktree isolation for parallel task execution

## Prerequisites

- [Node.js](https://nodejs.org/) (LTS)
- [pnpm](https://pnpm.io/) 10.x (`corepack enable && corepack prepare pnpm@10.18.3 --activate`)
- [Rust](https://rustup.rs/) toolchain (`rustup`)
- [Claude Code CLI](https://claude.ai/code) (required for LGE phase execution)

## Running Locally

### 1. Install dependencies

```bash
pnpm install
```

### 2. Run in development mode

```bash
pnpm tauri dev
```

This starts both the Vite dev server (port 1420) and the Rust backend with hot-reload.

To run only the frontend (without the Rust backend):

```bash
pnpm dev
```

### 3. Rust-only compilation (optional)

```bash
cd src-tauri && cargo build
```

For linting and tests on the Rust side:

```bash
cd src-tauri && cargo clippy
cd src-tauri && cargo test
```

## Generating the DMG (macOS)

### 1. Build the production app

```bash
pnpm tauri build
```

This compiles the optimized Rust binary and bundles `LGE Cockpit.app` at:

```
src-tauri/target/release/bundle/macos/LGE Cockpit.app
```

### 2. Generate the DMG

```bash
bash src-tauri/target/release/bundle/dmg/bundle_dmg.sh \
  --volname "LGE Cockpit" \
  --volicon src-tauri/icons/icon.icns \
  --icon-size 128 \
  --window-size 600 400 \
  --window-pos 200 120 \
  --icon "LGE Cockpit.app" 150 190 \
  --app-drop-link 450 190 \
  --hide-extension "LGE Cockpit.app" \
  "LGE Cockpit_$(node -p "require('./package.json').version")_aarch64.dmg" \
  "src-tauri/target/release/bundle/macos/LGE Cockpit.app"
```

The DMG is created in the project root with the app icon, proper Finder layout, and an Applications shortcut for drag-and-drop installation.

## Architecture

```
src-tauri/src/           # Rust backend
  lib.rs                 #   App setup, plugin registration, command handlers
  commands/              #   Tauri IPC commands (repos, tasks, jira, lge, git, settings)
  db/                    #   SQLite queries and migration runner
  models/                #   Serde-serializable domain structs
  migrations/            #   SQL migration files

src/                     # React/TypeScript frontend
  components/
    layout/              #   App shell (TopBar, Sidebar, StatusBar, HealthCheck)
    tasks/               #   Task CRUD (list, detail, create, Jira import)
    lge/                 #   LGE pipeline UI (process view, phase pipeline, artifacts)
    settings/            #   AI model configuration per phase
    ui/                  #   Shared primitives (Button, Dialog, Input, Badge)
  stores/                #   Zustand state management
  types/                 #   TypeScript type definitions
  i18n/                  #   Translation files (pt-BR, en, es)
  lib/                   #   Tauri IPC wrappers, constants
```

### Data Flow

```
React component → Zustand store → invoke("command") → Rust #[tauri::command] → SQLite → response
```

### LGE Pipeline

Each task runs through four sequential phases:

1. **Planning** — Analyzes the task and produces an implementation plan
2. **Builder** — Generates code based on the plan
3. **Review** — Reviews the generated code for quality and correctness
4. **Guardian** — Final validation and compliance checks

Planning phases are serialized (one at a time) via a semaphore. Each phase spawns a subprocess that can be individually cancelled.

## Database

SQLite stored at `~/Library/Application Support/com.lge.cockpit/lge-cockpit.db`.

- WAL mode with foreign keys enabled
- UUIDs (v4) for IDs, RFC3339 timestamps
- CASCADE delete: removing a repository deletes all its tasks
- Migrations applied automatically on startup

## License

MIT License. See [LICENSE](LICENSE) for details.
