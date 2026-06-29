# LGE Cockpit - MVP Plan

## Context

Criar uma aplicacao Desktop para macOS que funcione como cockpit para a estrategia de desenvolvimento LGE. O projeto e greenfield (diretorio vazio). O usuario quer gerenciar repositorios locais e tasks associadas, com integracao Jira via MCP Atlassian (invocando Claude Code CLI como subprocess). Interface em 3 idiomas (PT-BR, ES, EN).

## Tech Stack

- **Tauri 2.x** - Rust backend + React/TypeScript frontend. Bundle leve (~10MB), usa WebKit nativo do macOS
- **React 19 + TypeScript + Vite** - Frontend SPA
- **Tailwind CSS 4** - Dark theme com accent purple
- **Zustand** - State management leve
- **react-i18next** - i18n (PT-BR, ES, EN)
- **SQLite via rusqlite** - Persistencia local (bundled, sem dependencia externa)
- **tauri-plugin-dialog** - Folder picker nativo
- **tauri-plugin-shell** - Invocar Claude Code CLI para MCP Jira

## Data Model

```sql
CREATE TABLE repositories (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    path TEXT NOT NULL UNIQUE,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE tasks (
    id TEXT PRIMARY KEY,
    repository_id TEXT NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    description TEXT,
    status TEXT NOT NULL DEFAULT 'pending',  -- pending | in_progress | completed
    source TEXT NOT NULL DEFAULT 'manual',   -- manual | jira
    jira_key TEXT,
    jira_url TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX idx_tasks_repository ON tasks(repository_id);
CREATE INDEX idx_tasks_status ON tasks(status);
```

## Project Structure

```
lge-cockpit/
├── src-tauri/
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── capabilities/default.json
│   ├── migrations/001_initial.sql
│   └── src/
│       ├── main.rs               # Entry point, plugin registration
│       ├── lib.rs                # Tauri command exports + AppState
│       ├── db/
│       │   ├── mod.rs
│       │   ├── schema.rs         # Migration runner
│       │   └── queries.rs        # CRUD queries
│       ├── commands/
│       │   ├── mod.rs
│       │   ├── repositories.rs   # add/list/remove repos
│       │   ├── tasks.rs          # CRUD tasks
│       │   └── jira.rs           # import via Claude Code CLI + MCP
│       └── models/
│           ├── mod.rs
│           ├── repository.rs
│           └── task.rs
├── src/
│   ├── main.tsx
│   ├── App.tsx
│   ├── i18n/
│   │   ├── index.ts              # i18next config
│   │   ├── pt-BR.json
│   │   ├── es.json
│   │   └── en.json
│   ├── components/
│   │   ├── layout/
│   │   │   ├── Sidebar.tsx
│   │   │   ├── TopBar.tsx
│   │   │   └── StatusBar.tsx
│   │   ├── repositories/
│   │   │   ├── RepoList.tsx
│   │   │   ├── RepoItem.tsx
│   │   │   └── AddRepoDialog.tsx
│   │   ├── tasks/
│   │   │   ├── TaskList.tsx
│   │   │   ├── TaskItem.tsx
│   │   │   ├── CreateTaskDialog.tsx
│   │   │   └── ImportJiraDialog.tsx
│   │   └── ui/
│   │       ├── Button.tsx
│   │       ├── Dialog.tsx
│   │       ├── Input.tsx
│   │       └── Badge.tsx
│   ├── stores/
│   │   ├── repositoryStore.ts
│   │   └── taskStore.ts
│   ├── hooks/
│   │   ├── useRepositories.ts
│   │   └── useTasks.ts
│   ├── lib/
│   │   ├── tauri.ts              # Typed wrappers for invoke()
│   │   └── constants.ts
│   ├── types/index.ts
│   └── styles/globals.css
├── index.html
├── vite.config.ts
├── tsconfig.json
└── package.json
```

## Implementation Steps

### Step 1: Scaffolding
- `pnpm create tauri-app . --template react-ts --manager pnpm`
- Frontend deps: `zustand`, `react-i18next`, `i18next`, `@tauri-apps/plugin-dialog`, `@tauri-apps/plugin-shell`, `tailwindcss`, `@tailwindcss/vite`
- Rust deps em Cargo.toml: `rusqlite` (bundled), `uuid` (v4), `chrono`, `serde_json`, `tauri-plugin-dialog`, `tauri-plugin-shell`
- Configurar Tailwind dark theme + globals.css
- Configurar i18next com 3 locales (pt-BR, es, en)
- SQLite init no startup (app_data_dir)

### Step 2: Repository Management (Backend)
- `migrations/001_initial.sql`: schema completo
- `db/schema.rs`: migration runner no startup
- `models/repository.rs`: struct Repository (Serialize/Deserialize)
- `db/queries.rs`: insert_repository, list_repositories, delete_repository
- `commands/repositories.rs`: Tauri commands `add_repository`, `list_repositories`, `remove_repository`
- Registrar commands em `lib.rs`

### Step 3: Repository Management (Frontend)
- `App.tsx`: layout shell com 3 zonas (sidebar 240px | main flex-1 | statusbar fixed bottom)
- `Sidebar.tsx`: repo list + botao "+" para adicionar
- `AddRepoDialog.tsx`: usa `@tauri-apps/plugin-dialog` open() para folder picker nativo
- `repositoryStore.ts`: repos[], selectedRepoId, actions (add, remove, select)
- `useRepositories.ts`: hook que wrappa invoke() calls

### Step 4: Task Management (Backend)
- `models/task.rs`: struct Task, TaskStatus enum, TaskSource enum
- `db/queries.rs`: insert_task, list_tasks_by_repo, update_task_status, delete_task
- `commands/tasks.rs`: Tauri commands para CRUD
- Registrar commands

### Step 5: Task Management (Frontend)
- `TaskList.tsx`: lista tasks do repo selecionado, empty state quando sem tasks
- `TaskItem.tsx`: checkbox + titulo + badge status + badge source(jira)
- `CreateTaskDialog.tsx`: form com titulo (required) + descricao (optional)
- `taskStore.ts` + `useTasks.ts`

### Step 6: Jira via Claude Code CLI
- `commands/jira.rs`: Tauri command `import_jira_task` que:
  1. Recebe `repository_id` e `jira_key` (ex: "PROJ-123")
  2. Invoca Claude Code CLI via `tauri-plugin-shell` com prompt para buscar dados da issue via MCP Atlassian
  3. Parseia o output JSON com titulo/descricao/status
  4. Cria task no SQLite com source=jira
- `ImportJiraDialog.tsx`: input para Jira key + botao importar + loading state

### Step 7: i18n Setup
- `i18n/index.ts`: configuracao i18next com deteccao de idioma do sistema
- `i18n/pt-BR.json`, `i18n/es.json`, `i18n/en.json`: traducoes de todos labels
- Seletor de idioma no StatusBar ou Settings
- Todos componentes usam `useTranslation()` hook

### Step 8: UI Polish (Dark Theme)
- Cores: background `#0f0f1a`, surface `#1a1a2e`, card `#222240`, accent `#7c3aed` (violet)
- TopBar: workflow stages visuais (PRD > Techspec > Tasks > Execucao) - decorativo no MVP
- Status badges: pending=gray, in_progress=amber, completed=green
- Rounded corners (`rounded-xl`), hover states, smooth transitions
- Icones no sidebar conforme mockup

### Step 9: Build & Package
- `tauri.conf.json`: app "LGE Cockpit", bundle `com.lge.cockpit`, window 1200x800, min 900x600
- `pnpm tauri build` -> `.app` + `.dmg`

## Verification

1. `pnpm tauri dev` - app abre com dark theme
2. Trocar idioma (PT-BR/ES/EN) -> todos labels mudam
3. Adicionar repositorio via folder picker -> aparece no sidebar
4. Selecionar repo -> area principal mostra tasks (empty state)
5. Criar task manual (titulo + descricao) -> aparece na lista
6. Toggle status via checkbox -> badge atualiza
7. Importar task do Jira por key -> task aparece com badge "Jira"
8. Remover repositorio -> tasks cascadeiam
9. `pnpm tauri build` -> gera .dmg funcional para macOS
