# Git Worktree Isolation Per Task

## Context

Currently, all LGE tasks for a repository share the same working directory (`Repository.path`). When multiple tasks run in parallel, they conflict because git only allows one checked-out branch per working tree. `git worktree` solves this by creating linked working trees — each with its own branch — sharing the same `.git` history. This enables true parallel LGE execution without interference.

## Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Worktree location | `{repo_path}/.lge-worktrees/{task_code}/` | Discoverable alongside repo, single hidden dir, needs one `.gitignore` entry |
| When to create | Lazily on first `run_lge_phase` | Not all tasks run LGE |
| Path storage | `worktree_path TEXT` column on `tasks` table | Avoids re-deriving, handles edge cases |
| When to remove | Manual via `remove_worktree` command | User may want to inspect results |
| Branch + worktree | Created atomically via `git worktree add -b` | Single operation, no checkout on main repo |

## Implementation Steps

### 1. Database Migration + Model

**New file**: `src-tauri/migrations/004_worktree_path.sql`
```sql
ALTER TABLE tasks ADD COLUMN worktree_path TEXT;
```

**Modify** `src-tauri/src/db/schema.rs`: Add migration step for version 4.

**Modify** `src-tauri/src/models/task.rs`: Add `pub worktree_path: Option<String>` to `Task` struct.

**Modify** `src-tauri/src/db/queries.rs`:
- Add `update_task_worktree_path(conn, id, worktree_path, updated_at)`
- Add `get_task_worktree_path(conn, id) -> Option<String>`
- Update all `SELECT` queries that build `Task` to include `worktree_path` column

### 2. `ensure_worktree` Helper (internal, not a Tauri command)

**File**: `src-tauri/src/commands/git.rs`

```
async fn ensure_worktree(app, state, task_id) -> Result<String, String>
```

Logic:
1. Read task's `worktree_path` from DB. If set and directory exists, return it.
2. If set but directory gone, clear column and proceed.
3. Derive `task_code` (jira_key or first 8 chars of task_id).
4. Compute `worktree_dir = {repo_path}/.lge-worktrees/{task_code}`.
5. If `task.git_branch` is set: `git worktree add {worktree_dir} {branch}`.
6. If no branch: `git worktree add --detach {worktree_dir}` (from HEAD).
7. Ensure `.lge-worktrees/` is in `{repo_path}/.gitignore`.
8. Save `worktree_path` to DB.
9. Return worktree path.

### 3. Refactor `create_git_branch`

**File**: `src-tauri/src/commands/git.rs`

Current: `git checkout base && git checkout -b branch` on main repo (mutates main worktree).

New:
1. `git -C {repo_path} fetch origin {base_branch}`
2. If no worktree exists: `git -C {repo_path} worktree add -b {branch_name} {worktree_dir} origin/{base_branch}`
3. If worktree exists (detached): `git -C {worktree_dir} checkout -b {branch_name}`
4. Save both `git_branch` and `worktree_path` to DB.

### 4. Modify `run_lge_phase`

**File**: `src-tauri/src/commands/lge.rs`

After resolving `repo_path` (line ~435), call `ensure_worktree(app, state, task_id)` to get `working_dir`. Replace `repo_path` with `working_dir` in:
- `artifacts_dir` (line 442): `format!("{}/docs/tasks/{}", working_dir, task_code)`
- Claude CLI `-p` flag (line 462): `shell_escape(&working_dir)`

### 5. Modify Artifact Load/Save

**File**: `src-tauri/src/commands/lge.rs`

`load_lge_artifacts` and `save_lge_artifact`:
- Read `worktree_path` from task. If set and exists, use `{worktree_path}/docs/tasks/{task_code}/`.
- Fallback to `{repo_path}/docs/tasks/{task_code}/` for backward compatibility.

### 6. Refactor Git Commands to Use task_id

**File**: `src-tauri/src/commands/git.rs`

Change `commit_and_push`, `create_pull_request`, `generate_commit_message` to accept `task_id` instead of `repo_path`. Internally resolve `worktree_path` (fallback to `repo_path`).

New signatures:
- `commit_and_push(app, state, task_id, branch_name, message)`
- `create_pull_request(app, state, task_id, base_branch)`
- `generate_commit_message(app, state, task_id, task_title, jira_key)`

### 7. Add Cleanup Commands

**File**: `src-tauri/src/commands/git.rs`

- `remove_worktree(app, state, task_id)`: Runs `git worktree remove {path}`, clears DB column.
- `list_worktrees(app, repo_path)`: Runs `git worktree list`, informational.

### 8. Register Commands

**File**: `src-tauri/src/lib.rs`

Add `remove_worktree`, `list_worktrees` to `generate_handler![]`.

### 9. Frontend Changes

**`src/types/index.ts`**: Add `worktree_path: string | null` to `Task`.

**`src/lib/tauri.ts`**:
- `commitAndPush(taskId, branchName, message)` — changed from `repoPath`
- `createPullRequest(taskId, baseBranch)` — changed from `repoPath`
- `generateCommitMessage(taskId, taskTitle, jiraKey)` — changed from `repoPath`
- Add `removeWorktree(taskId)`

**`src/components/lge/LgePhasePipeline.tsx`**:
- PR panel: switch `commitAndPush`, `createPullRequest`, `generateCommitMessage` calls from `repo.path` to `task.id`
- Add "Clean up worktree" button in completed state

**`src/components/tasks/TaskDetail.tsx`** (or TaskItem):
- Show worktree status indicator when `task.worktree_path` is set
- Add "Remove worktree" action

### 10. i18n

Add translation keys in `src/i18n/{pt-BR,en,es}.json` for:
- Worktree status labels
- "Remove worktree" button
- Confirmation messages

## Critical Files

| File | Changes |
|------|---------|
| `src-tauri/migrations/004_worktree_path.sql` | **New** — migration |
| `src-tauri/src/db/schema.rs` | Add migration step |
| `src-tauri/src/db/queries.rs` | New helpers, update SELECTs |
| `src-tauri/src/models/task.rs` | Add `worktree_path` field |
| `src-tauri/src/commands/git.rs` | `ensure_worktree`, refactor all commands |
| `src-tauri/src/commands/lge.rs` | Use worktree in phase execution + artifacts |
| `src-tauri/src/lib.rs` | Register new commands |
| `src/types/index.ts` | Add `worktree_path` to Task |
| `src/lib/tauri.ts` | Update signatures, add `removeWorktree` |
| `src/components/lge/LgePhasePipeline.tsx` | Use task_id for git ops |
| `src/i18n/*.json` | New keys |

## Edge Cases

- **Branch already checked out**: `git worktree add` fails if branch is checked out elsewhere — catch error, provide clear message
- **Worktree dir exists but isn't valid**: Verify with `git worktree list`, recreate if needed
- **DB says worktree exists but dir deleted**: `ensure_worktree` detects and recreates
- **Backward compat**: Tasks without `worktree_path` fall back to `repo_path` for artifact loading

## Mitigations for Known Concerns

### Disk Usage

Cada worktree replica a working tree completa (exceto `.git`). Em repos grandes, múltiplas tarefas simultâneas podem consumir espaço significativo.

**Soluções:**

1. **Limite configurável de worktrees simultâneos por repositório.** Adicionar campo `max_worktrees` na tabela `repositories` (default: 5). `ensure_worktree` verifica o count antes de criar — se atingiu o limite, retorna erro claro: "Limite de {n} worktrees atingido. Remova worktrees concluídos antes de continuar."

2. **Exibir uso de disco no painel do repositório.** Ao listar worktrees, calcular tamanho com `du -sh` de cada diretório. Mostrar badge no Sidebar: "3 worktrees · 1.8 GB". Implementar como campo calculado em `list_worktrees` (já planejado).

3. **Sparse checkout para repos grandes.** Na criação do worktree, aplicar `git sparse-checkout set` incluindo apenas os paths relevantes para LGE (`docs/tasks/`, `src/`, e paths configuráveis). Reduz drasticamente o espaço em monorepos. Adicionar flag `sparse_paths: Option<Vec<String>>` na configuração do repositório.

### Cleanup de Worktrees Órfãos

Remoção manual por tarefa cria atrito quando muitas tarefas são concluídas.

**Soluções:**

1. **Batch cleanup: "Remove all completed worktrees".** Adicionar comando `remove_completed_worktrees(repo_id)` que busca todas as tasks com status `completed` que possuem `worktree_path` definido, e remove em sequência. Botão na UI do repositório, com confirmação listando os worktrees a serem removidos.

2. **Auto-cleanup pós-merge.** Após `create_pull_request` com sucesso e status do PR como merged (verificável via `gh pr view --json state`), oferecer toast: "PR merged. Remover worktree?" com ação de um clique. Alternativa mais agressiva: configuração `auto_remove_on_merge: bool` no repositório que remove automaticamente.

3. **Alerta de worktrees stale.** Na inicialização do app, verificar worktrees cujas tasks estão `completed` há mais de 7 dias. Exibir notificação: "Você tem {n} worktrees de tarefas concluídas. Limpar agora?" com link para batch cleanup.

### IDE Awareness

Worktrees em `.lge-worktrees/` não são visíveis como projetos no IDE do usuário.

**Soluções:**

1. **Botão "Open in IDE"** no TaskDetail quando `worktree_path` está definido. Usar `tauri_plugin_shell::open` para abrir o diretório no editor padrão. Implementação: `open_in_editor(worktree_path)` usando `code {path}` (VS Code), com editor configurável em settings.

2. **Copiar path para clipboard.** Ação simples no indicador de worktree: clicar copia o path absoluto. Baixo esforço, alta utilidade para quem usa terminal.

3. **Gerar `.code-workspace` para VS Code.** Ao criar worktree, gerar arquivo `.lge-worktrees/{task_code}.code-workspace` que inclui tanto o repo principal quanto o worktree como folders. Permite abrir ambos no mesmo VS Code com `code {workspace_file}`.

### Visibilidade Global

Falta uma visão consolidada de todos os worktrees ativos.

**Soluções:**

1. **Worktree dashboard no repositório.** Adicionar aba ou seção no RepositoryDetail listando todos os worktrees ativos com: task code, branch, tamanho em disco, data de criação, status da task. Reusar dados de `list_worktrees` + join com tasks.

2. **Badge no Sidebar.** Ao lado de cada repositório, mostrar contador de worktrees ativos (ex: chip "3 wt"). Implementar como campo adicional retornado por `list_repositories` via subquery count.

3. **StatusBar global.** Na barra inferior do app (onde já existe o language switcher), adicionar indicador: "4 worktrees · 2.3 GB" somando todos os repositórios. Clicar abre lista rápida com opção de cleanup.

### Implementation Priority

| Mitigation                             | Effort | Impact | Priority |
|----------------------------------------|--------|--------|----------|
| Batch cleanup ("Remove all completed") | Low | High | P0 — inclui no scope inicial |
| Limite de worktrees simultâneos        | Low | Medium | P0 — previne problemas antes que aconteçam |
| Badge no Sidebar (contador)            | Low | Medium | P0 — visibilidade mínima viável |
| Botão "Open in IDE"                    | Low | High | P1 — UX essencial para inspecionar código |
| Copiar path para clipboard             | Trivial | Medium | P1 — junto com Open in IDE |
| Auto-cleanup pós-merge (toast)         | Medium | High | P1 — reduz cleanup manual significativamente |
| Exibir uso de disco                    | Medium | Medium | P2 — nice to have |
| Alerta de worktrees stale              | Medium | Medium | P2 — nice to have |
| Worktree dashboarId                    | Medium | Medium | P2 — nice to have |
| Sparse checkout                        | High | High | P3 — só relevante para repos muito grandes |
| `.code-workspace` generation           | Low | Low | P3 — nicho VS Code |

## Verification

1. `cd src-tauri && cargo build` — Rust compiles
2. `pnpm build` — Frontend compiles
3. `pnpm tauri dev` — App runs
4. Create a repo, create 2 tasks, run LGE on both simultaneously → each gets its own worktree under `.lge-worktrees/`
5. Verify artifacts are saved in the worktree path
6. Verify commit+push+PR works from worktree
7. Verify "Remove worktree" cleans up correctly
8. Verify existing tasks without worktrees still load artifacts from repo_path
