# Design: Non-Technical UI Language, Round 2 (supersedes ADR 002)

Date: 2026-07-07
Status: Approved by user, pending implementation plan

## Context

ADR 002 (accepted 2026-07-06) decided a non-technical vocabulary for LGE Cockpit's daily-use surfaces but was never implemented — `i18n/*.json` still carries raw `worktree`/`branch`/`git`/`diff`/`commit`/`PR` strings. Before implementation started, the product owner reversed one mapping (Branch and Worktree swap which non-technical name they get) and expanded scope to two areas ADR 002 explicitly deferred: Settings (Jira + Environment tabs) and the Architecture Diff panel.

This design supersedes ADR 002. A new ADR 003 will be written during implementation to record the final decision; ADR 002 stays in git history unmodified (per this repo's convention of not rewriting accepted ADRs) but is marked superseded.

## Decisions carried over unchanged from ADR 002

- Pull Request → **"Aprovação"** (panel title "Pronto para Aprovação", action "Enviar para Aprovação", the shared commit/PR-body text field labeled "Descrição da Aprovação")
- "Processo LGE" → **"Desenvolvimento"** in functional/action copy; "LGE"/"LGE Cockpit" stays only as brand name (app title, health-check screen)
- The four phase names are translated (not reframed): Planning → Planejamento, Builder → Construção, Review → Revisão, Guardian → Guardião
- "Pull Request"/"PR" stays untranslated only in the dev-facing manual-fallback path (terminal command shown when automatic PR creation fails)
- UI-copy-only: Rust identifiers, DB columns, module names (`worktree_path`, `git_branch`, `Phase::Review`, etc.) are unchanged

## Decision reversed from ADR 002

| Concept | ADR 002 (superseded) | This design |
|---|---|---|
| Branch (`task.git_branch`) | Registro de Trabalho | **Área de Trabalho** |
| Worktree (`task.worktree_path`) | Área de Trabalho | **Ambiente Isolado** |

Canonical glossary names (used in `CONTEXT.md` and as the concept name across locales):

| Concept | pt-BR | en | es |
|---|---|---|---|
| Branch | Área de Trabalho | Task Workspace | Espacio de Trabajo |
| Worktree | Ambiente Isolado | Isolated Environment | Entorno Aislado |

This isn't a pure relabel — the action/flow copy around branch creation is reworded too (see key-level table below), not just the noun swapped in place.

## New scope: Settings

**Jira tab** — already close to plain language. Only real jargon: "Token de API" / "API token" → **"Chave de Acesso" / "Access Key"**. `baseUrl` field and the Atlassian navigation instructions in the hints stay as-is (URL is common vocabulary now; the Atlassian menu path is the external system's own UI, not ours to rename).

**Environment (shell) tab** — the field's actual content (`nvm use 18`, `unset GOROOT`) is inherently technical; renaming labels doesn't make it fillable by a non-technical user. Approach: keep the field and its technical placeholder as-is, but add a plain-language intro sentence explaining purpose and explicitly marking it optional/advanced, e.g. (pt-BR) *"Se o seu projeto precisa de versões específicas de ferramentas para rodar, cole os comandos aqui. Se não souber o que isso significa, pode deixar em branco ou pedir ajuda a quem configurou o projeto."* Title/description get this treatment; the placeholder text (the actual shell example) is untouched.

**Model per Phase tab** — fixes a pre-existing gap: `settings.models.planning/builder/review/guardian` are still hardcoded in English in **all three locales** (never picked up the phase-name translation decision). They become Planejamento/Construção/Revisão/Guardião (and es equivalents). Description drops "processo LGE" → "Desenvolvimento" ("cada etapa do Desenvolvimento"). Model names (Opus/Sonnet/Haiku) and their descriptions are untouched — proper nouns / already plain.

## New scope: Architecture Diff panel

Audience has changed (per product owner) — this panel is now also read by non-technical users, not just developers reviewing code. Chosen approach: **relabel, keep structure** (same charts/tables/layout, plain-language labels) — matches how the rest of the app was handled, lowest implementation risk.

- Dependency Graph → **Mapa de Conexões** / Connection Map / Mapa de Conexiones
- API Surface (Changes) → **Pontos de Contato Externo** / External Touchpoints / Puntos de Contacto Externo
- "Analyze Architectural Impact" / "Architectural Impact Analysis" → **Análise de Impacto no Sistema** / System Impact Analysis / Análisis de Impacto en el Sistema
- `emptyDesc` ("modules", "dependencies") reworded to avoid those nouns — see key table
- `emptyHint` ("Based on git diff") → **"Baseado no código alterado" / "Based on the code that changed" / "Basado en el código modificado"**
- "Risk" and Low/Medium/High/Critical are **unchanged** — already plain language in all three locales, not jargon
- Component/file names (`DependencyDiagramView.tsx`, `ApiSurfaceTable.tsx`, etc.) are unchanged — UI-copy-only, same as the rest of this design

## Out of scope / explicitly not touched

- Historical `releaseNotes.ts` entries (v0.1.0–v0.8.0) — append-only convention, never rewritten. The new entry documenting *this* change will naturally use the new vocabulary.
- `git.pr.manualHint` / `copyCommand` and the literal terminal command shown on automatic-PR-creation failure — stays technical (dev resolving the failure needs it to match GitHub).
- `git.pr.titleLabel` / `bodyLabel` — dead i18n keys already flagged in ADR 002, separate follow-up.
- Sidebar "Repositório" / "Adicionar Repositório" — not jargon, not raised as a concern, left as-is.

## Key-level i18n changes

All three locale files move together (`i18n-parity.md`). Placeholder `{{var}}` interpolations preserved as-is.

### `git.branch.*` → concept renamed to Área de Trabalho / Task Workspace / Espacio de Trabajo

| Key | pt-BR (new) | en (new) | es (new) |
|---|---|---|---|
| `title` | Área de Trabalho | Task Workspace | Espacio de Trabajo |
| `create` | Criar Área de Trabalho | Create Workspace | Crear Espacio de Trabajo |
| `dialogTitle` | Criar Área de Trabalho | Create Task Workspace | Crear Espacio de Trabajo |
| `nameLabel` | Nome da Área de Trabalho | Workspace name | Nombre del Espacio de Trabajo |
| `baseLabel` | Ponto de partida | Starting point | Punto de partida |
| `baseHint` | Vamos atualizar com a versão mais recente do projeto antes de criar sua Área de Trabalho. Padrão: develop. | We'll update to the project's latest version before creating your Workspace. Default: develop. | Actualizaremos a la versión más reciente del proyecto antes de crear tu Espacio de Trabajo. Por defecto: develop. |
| `creating` | Criando... | Creating... | Creando... |
| `confirm` | Criar Área de Trabalho | Create Workspace | Crear Espacio de Trabajo |

Badge that displays the raw branch value (e.g. `feature/oauth2-login`) is unaffected — only surrounding labels change, per ADR 002's original precedent.

### `git.worktree.*` → concept renamed to Ambiente Isolado / Isolated Environment / Entorno Aislado

| Key | pt-BR (new) | en (new) | es (new) |
|---|---|---|---|
| `active` | Ambiente Isolado | Isolated Environment | Entorno Aislado |
| `remove` | Remover Ambiente Isolado | Remove Isolated Environment | Eliminar Entorno Aislado |
| `openInIde` | Abrir no VS Code | Open in VS Code | Abrir en VS Code |
| `copyPath` | Copiar caminho | Copy path | Copiar ruta |
| `copied` | Copiado! | Copied! | ¡Copiado! |
| `cleanCompleted` | Limpar Ambientes Isolados de tarefas concluídas | Clean up Isolated Environments from completed tasks | Limpiar Entornos Aislados de tareas completadas |
| `cleanAll` | Limpar tudo | Clean all | Limpiar todo |
| `cleaning` | Limpando... | Cleaning... | Limpiando... |
| `cleaned` | Ambiente Isolado removido com sucesso | Isolated Environment removed successfully | Entorno Aislado eliminado con éxito |
| `cleanupAfterPr` | Limpar Ambiente Isolado | Clean up Isolated Environment | Limpiar Entorno Aislado |
| `limitReached` | Limite de Ambientes Isolados atingido | Isolated Environment limit reached | Límite de Entornos Aislados alcanzado |
| `staleAlert` | Você tem {{count}} Ambiente(s) Isolado(s) de tarefas concluídas há mais de 7 dias | You have {{count}} stale Isolated Environment(s) from completed tasks | Tienes {{count}} Entorno(s) Aislado(s) de tareas completadas hace más de 7 días |

### `tasks.*` / `repos.*`

| Key | pt-BR (new) | en (new) | es (new) |
|---|---|---|---|
| `tasks.deleteWarningWorktree` | O Ambiente Isolado ativo será removido do disco. | The active Isolated Environment will be removed from disk. | El Entorno Aislado activo será eliminado del disco. |
| `tasks.deleteWarningBranch` | A Área de Trabalho "{{branch}}" será removida. | The Workspace "{{branch}}" will be removed. | El Espacio de Trabajo "{{branch}}" será eliminado. |
| `repos.deleteStats` | {{tasks}} tarefa(s) · {{worktrees}} Ambiente(s) Isolado(s) · {{branches}} Área(s) de Trabalho serão removidas. | {{tasks}} task(s) · {{worktrees}} Isolated Environment(s) · {{branches}} Workspace(s) will be removed. | {{tasks}} tarea(s) · {{worktrees}} Entorno(s) Aislado(s) · {{branches}} Espacio(s) de Trabajo serán eliminados. |

### `git.commit.*` (field label already "Descrição da Aprovação" family per ADR 002; these keys leak "diff"/"commit" outside that field)

| Key | pt-BR (new) | en (new) | es (new) |
|---|---|---|---|
| `manualHint` | Você escreve a descrição | You write the description | Tú escribes la descripción |
| `aiHint` | Claude lê as alterações do código | Claude reads the code changes | Claude lee los cambios del código |
| `aiAnalyzingHint` | Claude está gerando a descrição com base nas alterações | Claude is generating the description based on the changes | Claude está generando la descripción basada en los cambios |

(`manual`, `ai`, `aiAnalyzing`, `aiGenerated`, `regenerate`, `changeMode` are already plain language — unchanged. `messageLabel` is superseded by ADR 002's "Descrição da Aprovação" rename, already decided, not repeated here.)

### `settings.jira.*`

| Key | pt-BR (new) | en (new) | es (new) |
|---|---|---|---|
| `apiToken` | Chave de Acesso | Access Key | Clave de Acceso |
| `apiTokenPlaceholder` | Gere uma nas configurações da conta Atlassian → Segurança → Criar e gerenciar tokens de API | Generate one at Atlassian account settings → Security → Create and manage API tokens | Genera una en la configuración de la cuenta Atlassian → Seguridad → Crear y gestionar tokens de API |
| `apiTokenHint` | Crie uma chave de acesso na sua conta Atlassian → Segurança → Tokens de API. | Create an Access Key in your Atlassian account → Security → API tokens. | Crea una Clave de Acceso en tu cuenta Atlassian → Seguridad → Tokens de API. |

(Atlassian's own menu still says "API tokens" — placeholder/hint keep that phrase where it refers to Atlassian's UI, only our field label changes.)

### `settings.shellEnv.*`

| Key | pt-BR (new) | en (new) | es (new) |
|---|---|---|---|
| `description` | Comandos executados antes de cada operação do Cockpit (git, Claude CLI, etc). Um comando por linha. Linhas com # são ignoradas.\n\nSe o seu projeto precisa de versões específicas de ferramentas para rodar, cole os comandos aqui. Se não souber o que isso significa, pode deixar em branco ou pedir ajuda a quem configurou o projeto. | Commands to run before every Cockpit operation (git, Claude CLI, etc). One command per line. Lines starting with # are ignored.\n\nIf your project needs specific tool versions to run, paste the commands here. If you're not sure what this means, you can leave it blank or ask whoever set up the project. | Comandos para ejecutar antes de cada operación del Cockpit (git, Claude CLI, etc). Un comando por línea. Líneas con # se ignoran.\n\nSi tu proyecto necesita versiones específicas de herramientas para funcionar, pega los comandos aquí. Si no sabes qué significa esto, puedes dejarlo en blanco o pedir ayuda a quien configuró el proyecto. |

(`title`, `placeholder` unchanged.)

### `settings.models.*`

| Key | pt-BR (new) | en (new) | es (new) |
|---|---|---|---|
| `description` | Escolha qual modelo de IA será usado em cada etapa do Desenvolvimento. | Choose which AI model to use for each Development step. | Elige qué modelo de IA usar en cada etapa del Desarrollo. |
| `planning` | Planejamento | Planning | Planificación |
| `builder` | Construção | Building | Construcción |
| `review` | Revisão | Review | Revisión |
| `guardian` | Guardião | Guardian | Guardián |

(`opus`/`sonnet`/`haiku` and their `*Desc` keys unchanged.)

### `lge.artifacts.archDiff.*`

| Key | pt-BR (new) | en (new) | es (new) |
|---|---|---|---|
| `depGraph` | Mapa de Conexões | Connection Map | Mapa de Conexiones |
| `depGraphEmpty` | Nenhuma conexão entre partes do sistema foi alterada | No connections between parts of the system changed | Ninguna conexión entre partes del sistema fue modificada |
| `apiSurface` | Pontos de Contato Externo | External Touchpoints | Puntos de Contacto Externo |
| `apiSurfaceEmpty` | Nenhum ponto de contato externo foi alterado | No external touchpoints changed | Ningún punto de contacto externo fue modificado |
| `analyzeBtn` | Analisar Impacto no Sistema | Analyze System Impact | Analizar Impacto en el Sistema |
| `emptyTitle` | Análise de Impacto no Sistema | System Impact Analysis | Análisis de Impacto en el Sistema |
| `emptyDesc` | Veja quais partes do sistema foram afetadas, novas conexões criadas e o tamanho do impacto desta fase. | See which parts of the system were affected, new connections created, and the size of this phase's impact. | Vea qué partes del sistema fueron afectadas, nuevas conexiones creadas y el tamaño del impacto de esta fase. |
| `emptyHint` | Rápido · Offline · Baseado no código alterado | Fast · Offline · Based on the code that changed | Rápido · Offline · Basado en el código modificado |
| `collapseAll` | Recolher tudo | Collapse all | Contraer todo |

(`summary`, `filesChanged`, `linesChanged`, `dependencies`, `riskScore`, `fileTree`, `expandAll`, `timeline`, `timelineEmpty`, `added`, `modified`, `removed`, `riskLow/Medium/High/Critical`, `analyzing`, `reanalyze` are unchanged — already plain language.)

## CONTEXT.md updates required

Rewrite the "Task Workspace" and "Task Work Record" sections (currently describing the ADR 002 mapping) to reflect the swap:

- "Task Workspace" section currently describes the **worktree** concept with pt-BR "Área de Trabalho" — must be repointed to describe the **branch** concept, or renamed/restructured so the canonical concept names stop colliding with the new pt-BR strings.
- "Task Work Record" (branch, "Registro de Trabalho") is retired; a new/renamed section describes the **worktree** as "Isolated Environment" / "Ambiente Isolado".
- `_Avoid_` lists in both sections need updating to match which raw term (Branch vs Worktree) is now avoided under which glossary entry.

## Verification

- `i18n-parity.md` script (key-count diff across pt-BR/en/es) must pass after edits.
- Manual grep for now-stale strings: `grep -rn "Registro de Trabalho\|Grafo de Dependências\|Superfície de API" src/i18n/` should return nothing.
- Version bump (`package.json`, `tauri.conf.json`, `Cargo.toml`) + new `releaseNotes.ts` entry per `version-sync.md` (this is a user-visible UI change).
- New ADR 003 written in `docs/adr/`, marking ADR 002 as superseded (ADR 002's file itself is not edited, per the "don't rewrite an accepted ADR" convention — status change is recorded in the new ADR, and optionally a one-line "Status: Superseded by ADR 003" edit to ADR 002's header only, if the project prefers that instead of leaving it silently orphaned).

## Self-review notes

- No placeholders/TBDs remain — every key in scope has an explicit new value across all three locales.
- Internal consistency check: "Aprovação" (PR) and "Revisão" (phase) don't collide — confirmed, this design doesn't touch that pairing.
- Scope check: three areas (daily-use, Settings, Architecture Diff) bundled into one plan per explicit user choice, despite touching unrelated files — acceptable, they share a single implementation unit (i18n files + CONTEXT.md + one ADR) rather than needing separate deploys.
- Ambiguity resolved: Settings → Environment tab does NOT get its technical content simplified (impossible without changing functionality) — only framed with an added plain-language intro, decided explicitly rather than left implicit.
