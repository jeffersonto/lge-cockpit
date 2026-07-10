# UI Language Simplification (Round 2) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace raw Git/dev vocabulary (branch, worktree, pull request, commit, diff, dependency graph, API surface, API token) with plain-language pt-BR/en/es copy across the daily-use UI, Settings, and the Architecture Diff panel — implementing ADR 002's undelivered decisions plus the branch/worktree swap and expanded scope decided in the follow-up interview.

**Architecture:** This is a content-only change: no component, store, or Rust code is touched. Every task edits `src/i18n/{pt-BR,en,es}.json` string values (never keys — key parity must hold before and after), plus two documentation files (`CONTEXT.md`, a new ADR) and the four version-sync files. There is no test runner in this project (per root `CLAUDE.md`), so each task's "test cycle" is a `grep`/`jq` check that the old string is gone and the new string is present, run immediately after the edit and before commit.

**Tech Stack:** i18next JSON translation files, Markdown docs, no build step required to verify (pure string edits).

## Global Constraints

- Every new/changed key must be edited in **all three** of `src/i18n/pt-BR.json`, `src/i18n/en.json`, `src/i18n/es.json` in the same task — never one locale alone (`.claude/rules/i18n-parity.md`).
- No i18n **keys** are added, removed, or renamed in this plan — only values change. Key-count parity (already true today) must remain true after every task.
- `{{variable}}` interpolation placeholders must be preserved verbatim in every changed string.
- Do not touch any `whatsNew.releases.v0{1..8}0.*` key — those are historical changelog entries and are append-only (`.claude/rules/version-sync.md`). Only a new `v090` entry is added (Task 15).
- Do not touch `git.pr.titleLabel`, `git.pr.bodyLabel`, `git.pr.manualHint`, `git.pr.copyCommand`, `git.pr.loading`, `git.pr.copied`, or any Rust/TS identifier, DB column, or component filename — out of scope per the approved design.
- Every commit in this plan is a docs/content-only commit; no `pnpm build` step is required to validate, but run the verification grep/jq shown in each task before committing.

---

### Task 1: Área de Trabalho (branch → Task Workspace)

**Files:**
- Modify: `src/i18n/pt-BR.json` (`git.branch.*`)
- Modify: `src/i18n/en.json` (`git.branch.*`)
- Modify: `src/i18n/es.json` (`git.branch.*`)

**Interfaces:** None (leaf string values only; no code reads change).

- [ ] **Step 1: Edit `src/i18n/pt-BR.json` — `git.branch` object**

Replace the current `branch` object:
```json
    "branch": {
      "title": "Branch Git",
      "create": "Criar Branch",
      "dialogTitle": "Criar Branch Git",
      "nameLabel": "Nome da branch",
      "baseLabel": "Branch base",
      "baseHint": "A branch base será atualizada com pull antes de criar a nova branch. Padrão: develop.",
      "creating": "Criando...",
      "confirm": "Criar Branch"
    },
```
with:
```json
    "branch": {
      "title": "Área de Trabalho",
      "create": "Criar Área de Trabalho",
      "dialogTitle": "Criar Área de Trabalho",
      "nameLabel": "Nome da Área de Trabalho",
      "baseLabel": "Ponto de partida",
      "baseHint": "Vamos atualizar com a versão mais recente do projeto antes de criar sua Área de Trabalho. Padrão: develop.",
      "creating": "Criando...",
      "confirm": "Criar Área de Trabalho"
    },
```

- [ ] **Step 2: Edit `src/i18n/en.json` — `git.branch` object**

Replace:
```json
    "branch": {
      "title": "Git Branch",
      "create": "Create Branch",
      "dialogTitle": "Create Git Branch",
      "nameLabel": "Branch name",
      "baseLabel": "Base branch",
      "baseHint": "The branch will be pulled and updated before the new branch is created. Default: develop.",
      "creating": "Creating...",
      "confirm": "Create Branch"
    },
```
with:
```json
    "branch": {
      "title": "Task Workspace",
      "create": "Create Workspace",
      "dialogTitle": "Create Task Workspace",
      "nameLabel": "Workspace name",
      "baseLabel": "Starting point",
      "baseHint": "We'll update to the project's latest version before creating your Workspace. Default: develop.",
      "creating": "Creating...",
      "confirm": "Create Workspace"
    },
```

- [ ] **Step 3: Edit `src/i18n/es.json` — `git.branch` object**

Replace:
```json
    "branch": {
      "title": "Rama Git",
      "create": "Crear Rama",
      "dialogTitle": "Crear Rama Git",
      "nameLabel": "Nombre de la rama",
      "baseLabel": "Rama base",
      "baseHint": "La rama base se actualizará con pull antes de crear la nueva rama. Por defecto: develop.",
      "creating": "Creando...",
      "confirm": "Crear Rama"
    },
```
with:
```json
    "branch": {
      "title": "Espacio de Trabajo",
      "create": "Crear Espacio de Trabajo",
      "dialogTitle": "Crear Espacio de Trabajo",
      "nameLabel": "Nombre del Espacio de Trabajo",
      "baseLabel": "Punto de partida",
      "baseHint": "Actualizaremos a la versión más reciente del proyecto antes de crear tu Espacio de Trabajo. Por defecto: develop.",
      "creating": "Creando...",
      "confirm": "Crear Espacio de Trabajo"
    },
```

- [ ] **Step 4: Verify old strings are gone, new strings present**

Run:
```bash
grep -c "Criar Branch\|Nome da branch\|Branch base" src/i18n/pt-BR.json
grep -c "Create Branch\|Branch name\|Base branch" src/i18n/en.json
grep -c "Crear Rama\|Nombre de la rama\|Rama base" src/i18n/es.json
grep -c "Área de Trabalho" src/i18n/pt-BR.json
```
Expected: first three commands print `0`; last prints `4` or more (title/create/dialogTitle/nameLabel/baseHint/confirm all contain the phrase).

- [ ] **Step 5: Commit**

```bash
git add src/i18n/pt-BR.json src/i18n/en.json src/i18n/es.json
git commit -m "feat(i18n): rename branch to Task Workspace (Área de Trabalho)"
```

---

### Task 2: Ambiente Isolado (worktree → Isolated Environment)

**Files:**
- Modify: `src/i18n/pt-BR.json` (`git.worktree.*`)
- Modify: `src/i18n/en.json` (`git.worktree.*`)
- Modify: `src/i18n/es.json` (`git.worktree.*`)

**Interfaces:** None.

- [ ] **Step 1: Edit `src/i18n/pt-BR.json` — `git.worktree` object**

Replace:
```json
    "worktree": {
      "active": "Worktree ativo",
      "remove": "Remover worktree",
      "openInIde": "Abrir no VS Code",
      "copyPath": "Copiar caminho",
      "copied": "Copiado!",
      "cleanCompleted": "Limpar worktrees concluídas",
      "cleanAll": "Limpar tudo",
      "cleaning": "Limpando...",
      "cleaned": "Worktree removido com sucesso",
      "cleanupAfterPr": "Limpar worktree",
      "limitReached": "Limite de worktrees atingido",
      "staleAlert": "Você tem {{count}} worktree(s) de tarefas concluídas há mais de 7 dias"
    },
```
with:
```json
    "worktree": {
      "active": "Ambiente Isolado",
      "remove": "Remover Ambiente Isolado",
      "openInIde": "Abrir no VS Code",
      "copyPath": "Copiar caminho",
      "copied": "Copiado!",
      "cleanCompleted": "Limpar Ambientes Isolados de tarefas concluídas",
      "cleanAll": "Limpar tudo",
      "cleaning": "Limpando...",
      "cleaned": "Ambiente Isolado removido com sucesso",
      "cleanupAfterPr": "Limpar Ambiente Isolado",
      "limitReached": "Limite de Ambientes Isolados atingido",
      "staleAlert": "Você tem {{count}} Ambiente(s) Isolado(s) de tarefas concluídas há mais de 7 dias"
    },
```

- [ ] **Step 2: Edit `src/i18n/en.json` — `git.worktree` object**

Replace:
```json
    "worktree": {
      "active": "Worktree active",
      "remove": "Remove worktree",
      "openInIde": "Open in VS Code",
      "copyPath": "Copy path",
      "copied": "Copied!",
      "cleanCompleted": "Clean completed worktrees",
      "cleanAll": "Clean all",
      "cleaning": "Cleaning...",
      "cleaned": "Worktree removed successfully",
      "cleanupAfterPr": "Clean up worktree",
      "limitReached": "Worktree limit reached",
      "staleAlert": "You have {{count}} stale worktree(s) from completed tasks"
    },
```
with:
```json
    "worktree": {
      "active": "Isolated Environment",
      "remove": "Remove Isolated Environment",
      "openInIde": "Open in VS Code",
      "copyPath": "Copy path",
      "copied": "Copied!",
      "cleanCompleted": "Clean up Isolated Environments from completed tasks",
      "cleanAll": "Clean all",
      "cleaning": "Cleaning...",
      "cleaned": "Isolated Environment removed successfully",
      "cleanupAfterPr": "Clean up Isolated Environment",
      "limitReached": "Isolated Environment limit reached",
      "staleAlert": "You have {{count}} stale Isolated Environment(s) from completed tasks"
    },
```

- [ ] **Step 3: Edit `src/i18n/es.json` — `git.worktree` object**

Replace:
```json
    "worktree": {
      "active": "Worktree activo",
      "remove": "Eliminar worktree",
      "openInIde": "Abrir en VS Code",
      "copyPath": "Copiar ruta",
      "copied": "Copiado!",
      "cleanCompleted": "Limpiar worktrees completadas",
      "cleanAll": "Limpiar todo",
      "cleaning": "Limpiando...",
      "cleaned": "Worktree eliminado con éxito",
      "cleanupAfterPr": "Limpiar worktree",
      "limitReached": "Límite de worktrees alcanzado",
      "staleAlert": "Tienes {{count}} worktree(s) de tareas completadas hace más de 7 días"
    },
```
with:
```json
    "worktree": {
      "active": "Entorno Aislado",
      "remove": "Eliminar Entorno Aislado",
      "openInIde": "Abrir en VS Code",
      "copyPath": "Copiar ruta",
      "copied": "¡Copiado!",
      "cleanCompleted": "Limpiar Entornos Aislados de tareas completadas",
      "cleanAll": "Limpiar todo",
      "cleaning": "Limpiando...",
      "cleaned": "Entorno Aislado eliminado con éxito",
      "cleanupAfterPr": "Limpiar Entorno Aislado",
      "limitReached": "Límite de Entornos Aislados alcanzado",
      "staleAlert": "Tienes {{count}} Entorno(s) Aislado(s) de tareas completadas hace más de 7 días"
    },
```

Note: `es.json`'s `copied` value changes from `"Copiado!"` to `"¡Copiado!"` only to match the inverted-exclamation convention already used elsewhere in this same file (e.g. `git.pr.copied`) — cosmetic, not part of the worktree rename, but fix it while the line is already touched.

- [ ] **Step 4: Verify**

```bash
grep -c "worktree" src/i18n/pt-BR.json src/i18n/en.json src/i18n/es.json
grep -c "Ambiente Isolado\|Isolated Environment\|Entorno Aislado" src/i18n/pt-BR.json src/i18n/en.json src/i18n/es.json
```
Expected: first command's counts drop to 0 in each file's `git.worktree.*` block specifically — since `tasks.deleteWarningWorktree` and `repos.deleteStats` still say "worktree" until Task 3, a nonzero total count here is expected right now; confirm instead with:
```bash
python3 -c "import json; d=json.load(open('src/i18n/pt-BR.json')); print(d['git']['worktree'])"
```
Expected: no value in the printed dict contains the word "worktree".

- [ ] **Step 5: Commit**

```bash
git add src/i18n/pt-BR.json src/i18n/en.json src/i18n/es.json
git commit -m "feat(i18n): rename worktree to Isolated Environment (Ambiente Isolado)"
```

---

### Task 3: Task/repo deletion warnings

**Files:**
- Modify: `src/i18n/pt-BR.json` (`tasks.deleteWarningWorktree`, `tasks.deleteWarningBranch`, `tasks.deleteDisabledRunning`, `repos.deleteStats`)
- Modify: `src/i18n/en.json` (same keys)
- Modify: `src/i18n/es.json` (same keys)

**Interfaces:** None. `tasks.deleteDisabledRunning` still says "fase LGE" — this key is being touched only because ADR 002 flagged it, but its wording currently doesn't leak branch/worktree jargon; per the design's carried-over decision, "Processo LGE"/"LGE process" phrasing moves to "Desenvolvimento" everywhere it appears in functional copy, and this key is one of those places.

- [ ] **Step 1: Edit `src/i18n/pt-BR.json`**

Replace:
```json
    "deleteWarningWorktree": "O worktree ativo será removido do disco.",
    "deleteWarningBranch": "A branch \"{{branch}}\" será deletada.",
```
with:
```json
    "deleteWarningWorktree": "O Ambiente Isolado ativo será removido do disco.",
    "deleteWarningBranch": "A Área de Trabalho \"{{branch}}\" será removida.",
```

Replace:
```json
    "deleteDisabledRunning": "Cancele a fase LGE em execução antes de excluir",
```
with:
```json
    "deleteDisabledRunning": "Cancele a etapa de Desenvolvimento em execução antes de excluir",
```

Replace (note: this is the last key in the file, no trailing comma):
```json
    "deleteStats": "{{tasks}} tarefa(s) · {{worktrees}} worktree(s) · {{branches}} branch(es) serão removidos."
```
with:
```json
    "deleteStats": "{{tasks}} tarefa(s) · {{worktrees}} Ambiente(s) Isolado(s) · {{branches}} Área(s) de Trabalho serão removidas."
```

- [ ] **Step 2: Edit `src/i18n/en.json`**

Replace:
```json
    "deleteWarningWorktree": "The active worktree will be removed from disk.",
    "deleteWarningBranch": "Branch \"{{branch}}\" will be deleted.",
```
with:
```json
    "deleteWarningWorktree": "The active Isolated Environment will be removed from disk.",
    "deleteWarningBranch": "Workspace \"{{branch}}\" will be removed.",
```

Replace:
```json
    "deleteDisabledRunning": "Cancel the running LGE phase before deleting",
```
with:
```json
    "deleteDisabledRunning": "Cancel the running Development step before deleting",
```

Replace (note: this is the last key in the file, no trailing comma):
```json
    "deleteStats": "{{tasks}} task(s) · {{worktrees}} worktree(s) · {{branches}} branch(es) will be removed."
```
with:
```json
    "deleteStats": "{{tasks}} task(s) · {{worktrees}} Isolated Environment(s) · {{branches}} Workspace(s) will be removed."
```

- [ ] **Step 3: Edit `src/i18n/es.json`**

Replace:
```json
    "deleteWarningWorktree": "El worktree activo será eliminado del disco.",
    "deleteWarningBranch": "La rama \"{{branch}}\" será eliminada.",
```
with:
```json
    "deleteWarningWorktree": "El Entorno Aislado activo será eliminado del disco.",
    "deleteWarningBranch": "El Espacio de Trabajo \"{{branch}}\" será eliminado.",
```

Replace:
```json
    "deleteDisabledRunning": "Cancela la fase LGE en ejecución antes de eliminar",
```
with:
```json
    "deleteDisabledRunning": "Cancela la etapa de Desarrollo en ejecución antes de eliminar",
```

Replace (note: this is the last key in the file, no trailing comma):
```json
    "deleteStats": "{{tasks}} tarea(s) · {{worktrees}} worktree(s) · {{branches}} rama(s) serán eliminados."
```
with:
```json
    "deleteStats": "{{tasks}} tarea(s) · {{worktrees}} Entorno(s) Aislado(s) · {{branches}} Espacio(s) de Trabajo serán eliminados."
```

- [ ] **Step 4: Verify**

```bash
grep -n "deleteWarningWorktree\|deleteWarningBranch\|deleteDisabledRunning\|deleteStats" src/i18n/pt-BR.json src/i18n/en.json src/i18n/es.json
```
Expected: none of the printed lines contain the bare words "worktree" or "branch" (pt-BR/es) or "LGE".

- [ ] **Step 5: Commit**

```bash
git add src/i18n/pt-BR.json src/i18n/en.json src/i18n/es.json
git commit -m "feat(i18n): update task/repo deletion copy for new Workspace/Environment names"
```

---

### Task 4: Aprovação (Pull Request → Aprovação, ADR 002 delivery)

**Files:**
- Modify: `src/i18n/pt-BR.json` (`git.pr.panelTitle`, `git.pr.commitAndOpen`, `git.pr.success`, `git.commit.messageLabel`)
- Modify: `src/i18n/en.json` (same keys)
- Modify: `src/i18n/es.json` (same keys)

**Interfaces:** None. `git.pr.titleLabel`/`bodyLabel` are dead keys (never wired to a component per ADR 002's note) — left untouched, out of scope. `git.pr.manualHint`/`copyCommand`/`loading`/`copied` stay technical (dev-facing fallback) — left untouched.

- [ ] **Step 1: Edit `src/i18n/pt-BR.json`**

Replace:
```json
      "panelTitle": "Pronto para Pull Request",
```
with:
```json
      "panelTitle": "Pronto para Aprovação",
```

Replace:
```json
      "commitAndOpen": "Commit, Push & Abrir PR",
```
with:
```json
      "commitAndOpen": "Enviar para Aprovação",
```

Replace:
```json
      "success": "Pull request criado com sucesso!",
```
with:
```json
      "success": "Aprovação enviada com sucesso!",
```

Replace:
```json
      "messageLabel": "Mensagem de commit",
```
with:
```json
      "messageLabel": "Descrição da Aprovação",
```

- [ ] **Step 2: Edit `src/i18n/en.json`**

Replace:
```json
      "panelTitle": "Ready for Pull Request",
```
with:
```json
      "panelTitle": "Ready for Approval",
```

Replace:
```json
      "commitAndOpen": "Commit, Push & Open PR",
```
with:
```json
      "commitAndOpen": "Send for Approval",
```

Replace:
```json
      "success": "Pull request created successfully!",
```
with:
```json
      "success": "Approval sent successfully!",
```

Replace:
```json
      "messageLabel": "Commit message",
```
with:
```json
      "messageLabel": "Approval Description",
```

- [ ] **Step 3: Edit `src/i18n/es.json`**

Replace:
```json
      "panelTitle": "Listo para Pull Request",
```
with:
```json
      "panelTitle": "Listo para Aprobación",
```

Replace:
```json
      "commitAndOpen": "Commit, Push & Abrir PR",
```
with:
```json
      "commitAndOpen": "Enviar para Aprobación",
```

Replace:
```json
      "success": "¡Pull request creado con éxito!",
```
with:
```json
      "success": "¡Aprobación enviada con éxito!",
```

Replace:
```json
      "messageLabel": "Mensaje de commit",
```
with:
```json
      "messageLabel": "Descripción de la Aprobación",
```

- [ ] **Step 4: Verify**

```bash
grep -n "\"panelTitle\"\|\"commitAndOpen\"\|\"success\"\|\"messageLabel\"" src/i18n/pt-BR.json src/i18n/en.json src/i18n/es.json
```
Expected: pt-BR/es lines contain "Aprovação"/"Aprobación", not "Pull Request"/"PR"/"commit". en lines contain "Approval", not "Pull Request"/"PR"/"Commit".

- [ ] **Step 5: Commit**

```bash
git add src/i18n/pt-BR.json src/i18n/en.json src/i18n/es.json
git commit -m "feat(i18n): rename Pull Request flow to Aprovação/Approval (delivers ADR 002)"
```

---

### Task 5: Commit-flow copy — remove leftover "diff"/"commit" jargon

**Files:**
- Modify: `src/i18n/pt-BR.json` (`git.commit.manualHint`, `git.commit.aiHint`, `git.commit.aiAnalyzingHint`)
- Modify: `src/i18n/en.json` (same keys)
- Modify: `src/i18n/es.json` (same keys)

**Interfaces:** None. `manual`, `ai`, `aiAnalyzing`, `aiGenerated`, `regenerate`, `changeMode` are already plain language in all three locales — not touched.

- [ ] **Step 1: Edit `src/i18n/pt-BR.json`**

Replace:
```json
      "manualHint": "Você escreve a mensagem",
```
with:
```json
      "manualHint": "Você escreve a descrição",
```

Replace:
```json
      "aiHint": "Claude lê o diff",
```
with:
```json
      "aiHint": "Claude lê as alterações do código",
```

Replace:
```json
      "aiAnalyzingHint": "Claude está gerando a mensagem de commit com base no diff",
```
with:
```json
      "aiAnalyzingHint": "Claude está gerando a descrição com base nas alterações",
```

- [ ] **Step 2: Edit `src/i18n/en.json`**

Replace:
```json
      "manualHint": "You write the message",
```
with:
```json
      "manualHint": "You write the description",
```

Replace:
```json
      "aiHint": "Claude reads the diff",
```
with:
```json
      "aiHint": "Claude reads the code changes",
```

Replace:
```json
      "aiAnalyzingHint": "Claude is generating a commit message based on the diff",
```
with:
```json
      "aiAnalyzingHint": "Claude is generating the description based on the changes",
```

- [ ] **Step 3: Edit `src/i18n/es.json`**

Replace:
```json
      "manualHint": "Tú escribes el mensaje",
```
with:
```json
      "manualHint": "Tú escribes la descripción",
```

Replace:
```json
      "aiHint": "Claude lee el diff",
```
with:
```json
      "aiHint": "Claude lee los cambios del código",
```

Replace:
```json
      "aiAnalyzingHint": "Claude está generando el mensaje de commit basado en el diff",
```
with:
```json
      "aiAnalyzingHint": "Claude está generando la descripción basada en los cambios",
```

- [ ] **Step 4: Verify**

```bash
grep -in "diff\|commit" src/i18n/pt-BR.json | grep -i "manualHint\|aiHint\|aiAnalyzingHint"
```
Expected: no output (empty).

- [ ] **Step 5: Commit**

```bash
git add src/i18n/pt-BR.json src/i18n/en.json src/i18n/es.json
git commit -m "feat(i18n): remove diff/commit jargon from AI commit-mode copy"
```

---

### Task 6: Desenvolvimento ("Processo LGE" → Desenvolvimento, ADR 002 delivery)

**Files:**
- Modify: `src/i18n/pt-BR.json` (`lge.title`, `lge.processComplete`, `topbar.noProcesses`)
- Modify: `src/i18n/en.json` (same keys)
- Modify: `src/i18n/es.json` (same keys)

**Interfaces:** None. "LGE"/"LGE Cockpit" stays as the product brand name elsewhere (app title, health-check screen) — not touched.

- [ ] **Step 1: Edit `src/i18n/pt-BR.json`**

Replace:
```json
  "lge": {
    "title": "Processo LGE",
```
with:
```json
  "lge": {
    "title": "Desenvolvimento",
```

Replace:
```json
    "processComplete": "Processo LGE concluído!",
```
with:
```json
    "processComplete": "Desenvolvimento concluído!",
```

Replace (note: this is the only/last key in the `topbar` object, no trailing comma):
```json
    "noProcesses": "Nenhum processo LGE ativo"
```
with:
```json
    "noProcesses": "Nenhum Desenvolvimento em andamento"
```

- [ ] **Step 2: Edit `src/i18n/en.json`**

Replace:
```json
  "lge": {
    "title": "LGE Process",
```
with:
```json
  "lge": {
    "title": "Development",
```

Replace:
```json
    "processComplete": "LGE process completed!",
```
with:
```json
    "processComplete": "Development completed!",
```

Replace (note: this is the only/last key in the `topbar` object, no trailing comma):
```json
    "noProcesses": "No active LGE processes"
```
with:
```json
    "noProcesses": "No Development in progress"
```

- [ ] **Step 3: Edit `src/i18n/es.json`**

Replace:
```json
  "lge": {
    "title": "Proceso LGE",
```
with:
```json
  "lge": {
    "title": "Desarrollo",
```

Replace:
```json
    "processComplete": "Proceso LGE completado!",
```
with:
```json
    "processComplete": "¡Desarrollo completado!",
```

Replace (note: this is the only/last key in the `topbar` object, no trailing comma):
```json
    "noProcesses": "Ningún proceso LGE activo"
```
with:
```json
    "noProcesses": "Ningún Desarrollo en curso"
```

- [ ] **Step 4: Verify**

```bash
grep -n "\"title\": \"Processo LGE\"\|\"title\": \"LGE Process\"\|\"title\": \"Proceso LGE\"" src/i18n/*.json
grep -c "Desenvolvimento\|Development\|Desarrollo" src/i18n/pt-BR.json src/i18n/en.json src/i18n/es.json
```
Expected: first command returns no output; second command's counts are each at least 3 (title, processComplete, noProcesses).

- [ ] **Step 5: Commit**

```bash
git add src/i18n/pt-BR.json src/i18n/en.json src/i18n/es.json
git commit -m "feat(i18n): rename LGE Process to Desenvolvimento/Development (delivers ADR 002)"
```

---

### Task 7: Phase name translation (`lge.phase.*`, ADR 002 delivery)

**Files:**
- Modify: `src/i18n/pt-BR.json` (`lge.phase.planning/builder/review/guardian`)
- Modify: `src/i18n/en.json` (same keys — values unchanged, English is already correct)
- Modify: `src/i18n/es.json` (same keys)

**Interfaces:** None. This closes the pre-existing i18n-parity gap noted in `CONTEXT.md`'s LGE Phase section: all four phase names were left untranslated (English literal) in `pt-BR.json` and `es.json`.

- [ ] **Step 1: Edit `src/i18n/pt-BR.json` — `lge.phase` object**

Replace:
```json
    "phase": {
      "planning": "Planning",
      "builder": "Builder",
      "review": "Review",
      "guardian": "Guardian"
    },
```
with:
```json
    "phase": {
      "planning": "Planejamento",
      "builder": "Construção",
      "review": "Revisão",
      "guardian": "Guardião"
    },
```

- [ ] **Step 2: `src/i18n/en.json` — no change needed**

Confirm the block already reads:
```json
    "phase": {
      "planning": "Planning",
      "builder": "Builder",
      "review": "Review",
      "guardian": "Guardian"
    },
```
Leave as-is. (No edit — English was already correct; this step exists only to confirm nothing regresses.)

- [ ] **Step 3: Edit `src/i18n/es.json` — `lge.phase` object**

Replace:
```json
    "phase": {
      "planning": "Planning",
      "builder": "Builder",
      "review": "Review",
      "guardian": "Guardian"
    },
```
with:
```json
    "phase": {
      "planning": "Planificación",
      "builder": "Construcción",
      "review": "Revisión",
      "guardian": "Guardián"
    },
```

- [ ] **Step 4: Verify**

```bash
python3 -c "import json; print(json.load(open('src/i18n/pt-BR.json'))['lge']['phase'])"
python3 -c "import json; print(json.load(open('src/i18n/es.json'))['lge']['phase'])"
```
Expected: pt-BR prints `{'planning': 'Planejamento', 'builder': 'Construção', 'review': 'Revisão', 'guardian': 'Guardião'}`; es prints `{'planning': 'Planificación', 'builder': 'Construcción', 'review': 'Revisión', 'guardian': 'Guardián'}`.

- [ ] **Step 5: Commit**

```bash
git add src/i18n/pt-BR.json src/i18n/es.json
git commit -m "feat(i18n): translate LGE phase names to pt-BR/es (delivers ADR 002)"
```

---

### Task 8: Configurações → Jira: "Chave de Acesso" (was "Token de API")

**Files:**
- Modify: `src/i18n/pt-BR.json` (`settings.jira.apiToken`, `apiTokenPlaceholder`, `apiTokenHint`)
- Modify: `src/i18n/en.json` (same keys)
- Modify: `src/i18n/es.json` (same keys)

**Interfaces:** None. `baseUrl`, `email`, and their placeholders/hints, plus `testConnection`/`testing`, are unchanged — already plain language.

- [ ] **Step 1: Edit `src/i18n/pt-BR.json`**

Replace:
```json
      "apiToken": "Token de API",
      "apiTokenPlaceholder": "Gere um nas configurações da conta Atlassian → Segurança → Criar e gerenciar tokens de API",
      "apiTokenHint": "Crie um token de API na sua conta Atlassian → Segurança → Tokens de API.",
```
with:
```json
      "apiToken": "Chave de Acesso",
      "apiTokenPlaceholder": "Gere uma nas configurações da conta Atlassian → Segurança → Criar e gerenciar tokens de API",
      "apiTokenHint": "Crie uma Chave de Acesso na sua conta Atlassian → Segurança → Tokens de API.",
```

- [ ] **Step 2: Edit `src/i18n/en.json`**

Replace:
```json
      "apiToken": "API token",
      "apiTokenPlaceholder": "Generate one at Atlassian account settings → Security → Create and manage API tokens",
      "apiTokenHint": "Create an API token in your Atlassian account → Security → API tokens.",
```
with:
```json
      "apiToken": "Access Key",
      "apiTokenPlaceholder": "Generate one at Atlassian account settings → Security → Create and manage API tokens",
      "apiTokenHint": "Create an Access Key in your Atlassian account → Security → API tokens.",
```

- [ ] **Step 3: Edit `src/i18n/es.json`**

Replace:
```json
      "apiToken": "Token de API",
      "apiTokenPlaceholder": "Genera uno en la configuración de la cuenta Atlassian → Seguridad → Crear y gestionar tokens de API",
      "apiTokenHint": "Crea un token de API en tu cuenta Atlassian → Seguridad → Tokens de API.",
```
with:
```json
      "apiToken": "Clave de Acceso",
      "apiTokenPlaceholder": "Genera una en la configuración de la cuenta Atlassian → Seguridad → Crear y gestionar tokens de API",
      "apiTokenHint": "Crea una Clave de Acceso en tu cuenta Atlassian → Seguridad → Tokens de API.",
```

- [ ] **Step 4: Verify**

```bash
grep -n "\"apiToken\":" src/i18n/pt-BR.json src/i18n/en.json src/i18n/es.json
```
Expected: `"apiToken": "Chave de Acesso",` / `"apiToken": "Access Key",` / `"apiToken": "Clave de Acceso",`

- [ ] **Step 5: Commit**

```bash
git add src/i18n/pt-BR.json src/i18n/en.json src/i18n/es.json
git commit -m "feat(i18n): rename Jira API token field to Access Key / Chave de Acesso"
```

---

### Task 9: Configurações → Ambiente: plain-language intro sentence

**Files:**
- Modify: `src/i18n/pt-BR.json` (`settings.shellEnv.description`)
- Modify: `src/i18n/en.json` (same key)
- Modify: `src/i18n/es.json` (same key)

**Interfaces:** None. `title` and `placeholder` (the literal shell example) are unchanged by design — the field's technical content isn't made non-technical, only framed.

- [ ] **Step 1: Edit `src/i18n/pt-BR.json`**

Replace:
```json
      "description": "Comandos executados antes de cada operação do Cockpit (git, Claude CLI, etc). Um comando por linha. Linhas com # são ignoradas.",
```
with:
```json
      "description": "Comandos executados antes de cada operação do Cockpit (git, Claude CLI, etc). Um comando por linha. Linhas com # são ignoradas.\n\nSe o seu projeto precisa de versões específicas de ferramentas para rodar, cole os comandos aqui. Se não souber o que isso significa, pode deixar em branco ou pedir ajuda a quem configurou o projeto.",
```

- [ ] **Step 2: Edit `src/i18n/en.json`**

Replace:
```json
      "description": "Commands to run before every Cockpit operation (git, Claude CLI, etc). One command per line. Lines starting with # are ignored.",
```
with:
```json
      "description": "Commands to run before every Cockpit operation (git, Claude CLI, etc). One command per line. Lines starting with # are ignored.\n\nIf your project needs specific tool versions to run, paste the commands here. If you're not sure what this means, you can leave it blank or ask whoever set up the project.",
```

- [ ] **Step 3: Edit `src/i18n/es.json`**

Replace:
```json
      "description": "Comandos para ejecutar antes de cada operación del Cockpit (git, Claude CLI, etc). Un comando por línea. Líneas con # se ignoran.",
```
with:
```json
      "description": "Comandos para ejecutar antes de cada operación del Cockpit (git, Claude CLI, etc). Un comando por línea. Líneas con # se ignoran.\n\nSi tu proyecto necesita versiones específicas de herramientas para funcionar, pega los comandos aquí. Si no sabes qué significa esto, puedes dejarlo en blanco o pedir ayuda a quien configuró el proyecto.",
```

- [ ] **Step 4: Verify**

Confirm the JSON is still valid (a stray unescaped newline would break parsing — `\n` inside a JSON string literal is correct and does not need escaping beyond the backslash-n already shown above):
```bash
python3 -c "import json; json.load(open('src/i18n/pt-BR.json')); json.load(open('src/i18n/en.json')); json.load(open('src/i18n/es.json')); print('all valid')"
```
Expected: `all valid`

- [ ] **Step 5: Commit**

```bash
git add src/i18n/pt-BR.json src/i18n/en.json src/i18n/es.json
git commit -m "feat(i18n): add plain-language intro to shell environment settings"
```

---

### Task 10: Configurações → Modelo por Fase

**Files:**
- Modify: `src/i18n/pt-BR.json` (`settings.models.description`, `.planning`, `.builder`, `.review`, `.guardian`)
- Modify: `src/i18n/en.json` (same keys)
- Modify: `src/i18n/es.json` (same keys)

**Interfaces:** None. `opus`/`sonnet`/`haiku` and their `*Desc` keys are unchanged — proper nouns / already plain.

- [ ] **Step 1: Edit `src/i18n/pt-BR.json`**

Replace:
```json
    "description": "Escolha qual modelo Claude será usado em cada fase do processo LGE.",
    "planning": "Planning",
    "builder": "Builder",
    "review": "Review",
    "guardian": "Guardian",
```
with:
```json
    "description": "Escolha qual modelo de IA será usado em cada etapa do Desenvolvimento.",
    "planning": "Planejamento",
    "builder": "Construção",
    "review": "Revisão",
    "guardian": "Guardião",
```

- [ ] **Step 2: Edit `src/i18n/en.json`**

Replace:
```json
    "description": "Choose which Claude model to use for each LGE process phase.",
    "planning": "Planning",
    "builder": "Builder",
    "review": "Review",
    "guardian": "Guardian",
```
with:
```json
    "description": "Choose which AI model to use for each Development step.",
    "planning": "Planning",
    "builder": "Building",
    "review": "Review",
    "guardian": "Guardian",
```

- [ ] **Step 3: Edit `src/i18n/es.json`**

Replace:
```json
    "description": "Elige qué modelo Claude usar en cada fase del proceso LGE.",
    "planning": "Planning",
    "builder": "Builder",
    "review": "Review",
    "guardian": "Guardian",
```
with:
```json
    "description": "Elige qué modelo de IA usar en cada etapa del Desarrollo.",
    "planning": "Planificación",
    "builder": "Construcción",
    "review": "Revisión",
    "guardian": "Guardián",
```

- [ ] **Step 4: Verify**

```bash
python3 -c "import json; print(json.load(open('src/i18n/pt-BR.json'))['settings']['models'])"
```
Expected: `description` no longer contains "processo LGE"; `planning`/`builder`/`review`/`guardian` read `Planejamento`/`Construção`/`Revisão`/`Guardião`.

- [ ] **Step 5: Commit**

```bash
git add src/i18n/pt-BR.json src/i18n/en.json src/i18n/es.json
git commit -m "feat(i18n): fix settings.models phase names to match translated phase vocabulary"
```

---

### Task 11: Diff de Arquitetura panel relabeling

**Files:**
- Modify: `src/i18n/pt-BR.json` (`lge.artifacts.archDiff.{depGraph,depGraphEmpty,apiSurface,apiSurfaceEmpty,analyzeBtn,emptyTitle,emptyDesc,emptyHint,collapseAll}`)
- Modify: `src/i18n/en.json` (same keys)
- Modify: `src/i18n/es.json` (same keys)

**Interfaces:** None. `summary`, `filesChanged`, `linesChanged`, `dependencies`, `riskScore`, `fileTree`, `expandAll`, `timeline`, `timelineEmpty`, `added`, `modified`, `removed`, `riskLow/Medium/High/Critical`, `analyzing`, `reanalyze` are unchanged — already plain language per the approved "relabel, keep structure" approach.

- [ ] **Step 1: Edit `src/i18n/pt-BR.json`**

Replace:
```json
        "collapseAll": "Colapsar tudo",
        "depGraph": "Grafo de Dependências",
        "depGraphEmpty": "Nenhuma alteração de dependências detectada",
        "apiSurface": "Superfície de API",
        "apiSurfaceEmpty": "Nenhuma alteração de API pública detectada",
```
with:
```json
        "collapseAll": "Recolher tudo",
        "depGraph": "Mapa de Conexões",
        "depGraphEmpty": "Nenhuma conexão entre partes do sistema foi alterada",
        "apiSurface": "Pontos de Contato Externo",
        "apiSurfaceEmpty": "Nenhum ponto de contato externo foi alterado",
```

Replace:
```json
        "analyzeBtn": "Analisar Impacto Arquitetural",
```
with:
```json
        "analyzeBtn": "Analisar Impacto no Sistema",
```

Replace:
```json
        "emptyTitle": "Análise de Impacto Arquitetural",
        "emptyDesc": "Veja quais módulos foram afetados, novas dependências introduzidas e o raio de impacto desta fase.",
        "emptyHint": "Rápido · Offline · Baseado em git diff"
```
with:
```json
        "emptyTitle": "Análise de Impacto no Sistema",
        "emptyDesc": "Veja quais partes do sistema foram afetadas, novas conexões criadas e o tamanho do impacto desta fase.",
        "emptyHint": "Rápido · Offline · Baseado no código alterado"
```

- [ ] **Step 2: Edit `src/i18n/en.json`**

Replace:
```json
        "collapseAll": "Collapse all",
        "depGraph": "Dependency Graph",
        "depGraphEmpty": "No dependency changes detected",
        "apiSurface": "API Surface Changes",
        "apiSurfaceEmpty": "No public API changes detected",
```
with:
```json
        "collapseAll": "Collapse all",
        "depGraph": "Connection Map",
        "depGraphEmpty": "No connections between parts of the system changed",
        "apiSurface": "External Touchpoints",
        "apiSurfaceEmpty": "No external touchpoints changed",
```

Replace:
```json
        "analyzeBtn": "Analyze Architectural Impact",
```
with:
```json
        "analyzeBtn": "Analyze System Impact",
```

Replace:
```json
        "emptyTitle": "Architectural Impact Analysis",
        "emptyDesc": "See which modules were affected, new dependencies introduced, and the blast radius of this phase.",
        "emptyHint": "Fast · Offline · Based on git diff"
```
with:
```json
        "emptyTitle": "System Impact Analysis",
        "emptyDesc": "See which parts of the system were affected, new connections created, and the size of this phase's impact.",
        "emptyHint": "Fast · Offline · Based on the code that changed"
```

- [ ] **Step 3: Edit `src/i18n/es.json`**

Replace:
```json
        "collapseAll": "Colapsar todo",
        "depGraph": "Grafo de Dependencias",
        "depGraphEmpty": "No se detectaron cambios de dependencias",
        "apiSurface": "Superficie de API",
        "apiSurfaceEmpty": "No se detectaron cambios de API pública",
```
with:
```json
        "collapseAll": "Contraer todo",
        "depGraph": "Mapa de Conexiones",
        "depGraphEmpty": "Ninguna conexión entre partes del sistema fue modificada",
        "apiSurface": "Puntos de Contacto Externo",
        "apiSurfaceEmpty": "Ningún punto de contacto externo fue modificado",
```

Replace:
```json
        "analyzeBtn": "Analizar Impacto Arquitectural",
```
with:
```json
        "analyzeBtn": "Analizar Impacto en el Sistema",
```

Replace:
```json
        "emptyTitle": "Análisis de Impacto Arquitectural",
        "emptyDesc": "Vea qué módulos fueron afectados, nuevas dependencias introducidas y el radio de impacto de esta fase.",
        "emptyHint": "Rápido · Offline · Basado en git diff"
```
with:
```json
        "emptyTitle": "Análisis de Impacto en el Sistema",
        "emptyDesc": "Vea qué partes del sistema fueron afectadas, nuevas conexiones creadas y el tamaño del impacto de esta fase.",
        "emptyHint": "Rápido · Offline · Basado en el código modificado"
```

- [ ] **Step 4: Verify**

```bash
grep -n "Grafo de Dependências\|Superfície de API\|Impacto Arquitetural\|git diff" src/i18n/pt-BR.json src/i18n/es.json
grep -n "Dependency Graph\|API Surface\|Architectural Impact\|git diff" src/i18n/en.json
```
Expected: no output from either command.

- [ ] **Step 5: Commit**

```bash
git add src/i18n/pt-BR.json src/i18n/en.json src/i18n/es.json
git commit -m "feat(i18n): relabel Architecture Diff panel for non-technical audience"
```

---

### Task 12: i18n key-count parity verification

**Files:** None modified — read-only verification task confirming Tasks 1–11 never added/removed/renamed a key.

**Interfaces:** None.

- [ ] **Step 1: Run the parity diff from `.claude/rules/i18n-parity.md`**

```bash
for f in pt-BR en es; do
  jq -r 'paths(scalars) | join(".")' src/i18n/$f.json | sort > /tmp/$f.keys
done
diff /tmp/pt-BR.keys /tmp/en.keys
diff /tmp/en.keys /tmp/es.keys
```
Expected: both `diff` commands produce empty output (no exit output, meaning identical key sets).

- [ ] **Step 2: If either diff is non-empty**

Identify which key exists in one locale but not another (the diff output shows `<`/`>` lines with the differing key paths), go back to the task that touched that key, and add the missing key/value to the locale that's missing it. Do not proceed to Task 13 until both diffs are empty.

- [ ] **Step 3: No commit needed for this task** (verification only; if Step 2 required a fix, commit that fix with `git commit -m "fix(i18n): restore key parity after language simplification edits"`).

---

### Task 13: CONTEXT.md glossary update

**Files:**
- Modify: `CONTEXT.md:68-92` (the "Task Workspace", "Task Work Record", "Task Review Request", "Task Development" sections)

**Interfaces:** None — documentation only.

- [ ] **Step 1: Read the current sections to confirm line numbers haven't shifted**

```bash
grep -n "^## Task Workspace\|^## Task Work Record\|^## Task Review Request\|^## Task Development" CONTEXT.md
```

- [ ] **Step 2: Replace the "Task Workspace" and "Task Work Record" sections**

Replace this block (currently describing the ADR-002 mapping, now superseded):
```markdown
## Task Workspace

The folder + isolated git working tree where a task's code changes live during development (backed internally by `WorktreeProvisioner` / `task.worktree_path`). Surfaced to non-technical users in the UI as **"Área de Trabalho"** — the rename is UI copy only, no change to the underlying data model or Rust-side naming (`worktree` stays the term in code, migrations, and `CONTEXT.md`'s `PhaseRunner` section above).

_Avoid_ (in user-facing copy only): Worktree, workspace folder.

## Task Work Record

The git branch created per task (`task.git_branch`), surfaced to non-technical users as **"Registro de Trabalho"**; the action that creates it is labeled **"Iniciar Registro"** (was "Criar Branch"). The badge still displays the raw branch name (e.g. `feature/oauth2-login`) — only the surrounding label changes, not the value.

_Avoid_ (in user-facing copy only): Branch, Git Branch.
```
with:
```markdown
## Task Workspace

The git branch created per task (`task.git_branch`), surfaced to non-technical users as **"Área de Trabalho"** (English/Spanish glossary: "Task Workspace" / "Espacio de Trabajo"); the action that creates it is labeled **"Criar Área de Trabalho"** (was "Criar Branch"). The badge still displays the raw branch name (e.g. `feature/oauth2-login`) — only the surrounding label changes, not the value. This is a **2026-07-07 reversal** of the original ADR 002 mapping (superseded by ADR 003), which had assigned "Área de Trabalho" to the worktree instead — see [[Task Isolated Environment]] for the other half of the swap.

_Avoid_ (in user-facing copy only): Branch, Git Branch, Registro de Trabalho (retired 2026-07-07).

## Task Isolated Environment

The folder + isolated git working tree where a task's code changes live during development (backed internally by `WorktreeProvisioner` / `task.worktree_path`). Surfaced to non-technical users in the UI as **"Ambiente Isolado"** (English/Spanish glossary: "Isolated Environment" / "Entorno Aislado") — the rename is UI copy only, no change to the underlying data model or Rust-side naming (`worktree` stays the term in code, migrations, and `CONTEXT.md`'s `PhaseRunner` section above). Formerly labeled "Área de Trabalho" under ADR 002 (superseded); see [[Task Workspace]] above for the other half of the 2026-07-07 swap.

_Avoid_ (in user-facing copy only): Worktree, workspace folder.
```

- [ ] **Step 3: Verify no dangling references remain**

```bash
grep -n "Task Work Record\|Registro de Trabalho" CONTEXT.md
```
Expected: only the one intentional historical mention inside the new `_Avoid_` line added in Step 2 (`Registro de Trabalho (retired 2026-07-07)`); no section still titled "Task Work Record".

- [ ] **Step 4: Commit**

```bash
git add CONTEXT.md
git commit -m "docs: update CONTEXT.md glossary for Área de Trabalho/Ambiente Isolado swap"
```

---

### Task 14: ADR 003 — supersede ADR 002

**Files:**
- Create: `docs/adr/003-language-simplification-round-2.md`
- Modify: `docs/adr/002-non-technical-ui-language.md:7` (status line only)

**Interfaces:** None — documentation only.

- [ ] **Step 1: Update ADR 002's status line**

In `docs/adr/002-non-technical-ui-language.md`, replace:
```markdown
Status: Accepted
```
with:
```markdown
Status: Superseded by ADR 003
```

- [ ] **Step 2: Create `docs/adr/003-language-simplification-round-2.md`**

```markdown
# ADR 003: Non-Technical Language, Round 2 — Workspace/Environment Swap and Expanded Scope

ADR Number: 003
Title: Non-Technical Language, Round 2 — Workspace/Environment Swap and Expanded Scope
Date: 2026-07-07
Responsible: LGE Cockpit Team
Status: Accepted

## Context

ADR 002 (2026-07-06) was accepted but never implemented before the product owner reversed one of its central decisions and asked for the scope to grow. This ADR supersedes ADR 002 in full, carrying forward every decision that didn't change and recording the two that did: which non-technical name maps to Branch vs. Worktree, and whether Settings (Jira/Environment tabs) and the Architecture Diff panel are in scope.

## Decision

**Reversed from ADR 002:**

- Branch (`task.git_branch`) → **"Área de Trabalho"** ("Task Workspace" / "Espacio de Trabajo"). Previously this name was assigned to Worktree.
- Worktree (`task.worktree_path`) → **"Ambiente Isolado"** ("Isolated Environment" / "Entorno Aislado"). Previously this was "Registro de Trabalho", assigned to Branch.
- The action/flow copy around branch creation was reworded, not just relabeled: "Criar Branch" → "Criar Área de Trabalho", "Branch base"/"baseHint" (which exposed raw git base-branch/pull vocabulary) → "Ponto de partida" with a plain-language explanation of what happens before workspace creation.

**Carried forward unchanged from ADR 002 (now actually implemented):**

- Pull Request → **"Aprovação"**: panel title "Pronto para Aprovação", action "Enviar para Aprovação", the shared commit/PR-body field labeled "Descrição da Aprovação".
- "Processo LGE" → **"Desenvolvimento"** in functional/action copy ("Nenhum Desenvolvimento em andamento", "Desenvolvimento concluído!"); "LGE"/"LGE Cockpit" stays as brand name only.
- The four phase names are translated (not reframed): Planejamento, Construção, Revisão, Guardião — closing the pre-existing i18n-parity gap where all four were left in English in `pt-BR.json`/`es.json`, including in the Settings → Model per Phase tab, which had independently drifted from this decision.
- "Pull Request"/"PR" stays untranslated only in the dev-facing manual-fallback path (terminal command shown when automatic PR creation fails).

**New scope, not covered by ADR 002:**

- **Settings → Jira tab**: "Token de API" → **"Chave de Acesso"** ("Access Key" / "Clave de Acceso"). Other fields (URL, email, connection test) were already plain language.
- **Settings → Environment (shell) tab**: the field's content is inherently technical (`nvm use 18`, `unset GOROOT`) and stays that way — renaming labels can't make it fillable by someone without dev knowledge. Instead, a plain-language paragraph was added explaining the field's purpose and explicitly marking it optional, with permission to leave it blank or ask for help.
- **Architecture Diff panel**: now read by non-technical users (changed from ADR 002's assumption that it was developer-only reading material). Chosen approach — relabel technical terms, keep the existing layout/structure unchanged: "Dependency Graph" → "Mapa de Conexões"/"Connection Map", "API Surface" → "Pontos de Contato Externo"/"External Touchpoints", "Architectural Impact" → "Impacto no Sistema"/"System Impact". "Risk" and its four levels (Low/Medium/High/Critical) were judged already plain language and left unchanged.

## Justification

- The branch/worktree reversal reflects the product owner's judgment that "Área de Trabalho" (a workspace you work *in*) more naturally maps to the branch — the thing the user's changes live on — than to the worktree, which is more accurately an "isolated" execution copy of that work.
- Bundling Settings and Architecture Diff into this round, despite ADR 002 deferring them, was a deliberate scope decision made once the product owner confirmed non-technical users now interact with both surfaces, not just developers doing one-time setup.
- The Environment tab is the one place this design explicitly does **not** achieve full non-technical accessibility — the underlying task (customizing a shell environment) requires the operator to know what a shell environment is. Framing this honestly (mark it optional, permit leaving it blank) was preferred over pretending a label change makes the field self-service.

## Alternatives Considered

- **Keep ADR 002's original branch/worktree mapping and treat this as a new, additive round.** Rejected — the product owner explicitly reversed the mapping; keeping the old one would ship the wrong copy.
- **Split this into three separate ADRs/specs (daily-use, Settings, Architecture Diff) shipped independently.** Considered during the interview; rejected by the product owner in favor of one combined round, since all three share the same underlying i18n files and review cycle.
- **Reorganize the Architecture Diff panel around business questions ("what changed", "what does this affect") instead of relabeling its existing structure.** Rejected as higher-effort than justified for this round; relabeling matches how the rest of the app was already being treated and can be revisited later if the relabel-only approach proves insufficient.

## Consequences

- All three locale files (`pt-BR.json`, `en.json`, `es.json`) needed coordinated updates to `git.branch.*`, `git.worktree.*`, `git.pr.{panelTitle,commitAndOpen,success}`, `git.commit.{messageLabel,manualHint,aiHint,aiAnalyzingHint}`, `lge.{title,processComplete,phase.*}`, `topbar.noProcesses`, `tasks.{deleteWarningWorktree,deleteWarningBranch,deleteDisabledRunning}`, `repos.deleteStats`, `settings.jira.{apiToken,apiTokenPlaceholder,apiTokenHint}`, `settings.shellEnv.description`, `settings.models.{description,planning,builder,review,guardian}`, and `lge.artifacts.archDiff.{depGraph,depGraphEmpty,apiSurface,apiSurfaceEmpty,analyzeBtn,emptyTitle,emptyDesc,emptyHint,collapseAll}`.
- `CONTEXT.md`'s "Task Workspace" and "Task Work Record" sections were rewritten and renamed ("Task Isolated Environment" replaces "Task Work Record") to reflect which concept now owns which glossary name.
- This remains a **UI-copy-only** decision: no Rust identifiers, DB columns, component filenames, or module names changed.
- `git.pr.titleLabel`/`bodyLabel` remain unaddressed dead keys, as flagged in ADR 002 — still a separate follow-up.
- Historical `releaseNotes.ts` entries (v0.1.0–v0.8.0) were not rewritten, per this repo's append-only changelog convention; a new v0.9.0 entry documents this change using the new vocabulary.
```

- [ ] **Step 3: Verify**

```bash
grep -n "Status:" docs/adr/002-non-technical-ui-language.md
ls docs/adr/003-language-simplification-round-2.md
```
Expected: first command prints `Status: Superseded by ADR 003`; second confirms the new file exists.

- [ ] **Step 4: Commit**

```bash
git add docs/adr/002-non-technical-ui-language.md docs/adr/003-language-simplification-round-2.md
git commit -m "docs: add ADR 003 superseding ADR 002 for language simplification round 2"
```

---

### Task 15: Version bump and release note entry

**Files:**
- Modify: `package.json` (`version`)
- Modify: `src-tauri/tauri.conf.json` (`version`)
- Modify: `src-tauri/Cargo.toml` (`version`)
- Modify: `src/data/releaseNotes.ts` (`CURRENT_VERSION`, prepend `RELEASE_NOTES` entry)
- Modify: `src/i18n/pt-BR.json`, `en.json`, `es.json` (new `whatsNew.releases.v090.*` keys)

**Interfaces:** `RELEASE_NOTES` entries reference i18n keys via `titleKey`/`labelKey` (see `ReleaseNote`/`ReleaseFeature` interfaces already defined in `src/data/releaseNotes.ts:29-39` — unchanged, only a new array element is added).

Current version is `0.8.0`. This is a user-visible UI change (renamed labels across the whole app) — bump to `0.9.0` per `.claude/rules/version-sync.md`.

- [ ] **Step 1: Bump `package.json`**

```bash
grep -n '"version"' package.json
```
Replace the matched line's value `"0.8.0"` with `"0.9.0"` (edit the `"version": "0.8.0",` line to `"version": "0.9.0",`).

- [ ] **Step 2: Bump `src-tauri/tauri.conf.json`**

Replace:
```json
  "version": "0.8.0",
```
with:
```json
  "version": "0.9.0",
```

- [ ] **Step 3: Bump `src-tauri/Cargo.toml`**

Replace:
```toml
version = "0.8.0"
```
with:
```toml
version = "0.9.0"
```

- [ ] **Step 4: Add new i18n keys for the release note (all three locales)**

In `src/i18n/pt-BR.json`, inside the `whatsNew.releases` object (`src/i18n/pt-BR.json:260-266`), the `v080` entry is currently the first child. Replace:
```json
    "releases": {
      "v080": {
```
with:
```json
    "releases": {
      "v090": {
        "title": "v0.9.0 — Linguagem Simplificada",
        "f1": "Termos técnicos como branch, worktree e pull request viraram Área de Trabalho, Ambiente Isolado e Aprovação nas telas de uso diário",
        "f2": "Painel de Diff de Arquitetura com linguagem acessível: Mapa de Conexões e Pontos de Contato Externo substituem Dependency Graph e API Surface",
        "f3": "Configurações Jira e Ambiente com explicações em linguagem simples; token de API agora é Chave de Acesso"
      },
      "v080": {
```

In `src/i18n/en.json`, replace:
```json
    "releases": {
      "v080": {
```
with:
```json
    "releases": {
      "v090": {
        "title": "v0.9.0 — Simplified Language",
        "f1": "Technical terms like branch, worktree, and pull request became Task Workspace, Isolated Environment, and Approval across daily-use screens",
        "f2": "Architecture Diff panel uses plain language: Connection Map and External Touchpoints replace Dependency Graph and API Surface",
        "f3": "Jira and Environment settings gained plain-language explanations; API token is now called Access Key"
      },
      "v080": {
```

In `src/i18n/es.json`, replace:
```json
    "releases": {
      "v080": {
```
with:
```json
    "releases": {
      "v090": {
        "title": "v0.9.0 — Lenguaje Simplificado",
        "f1": "Términos técnicos como branch, worktree y pull request pasaron a ser Espacio de Trabajo, Entorno Aislado y Aprobación en las pantallas de uso diario",
        "f2": "El panel de Diff de Arquitectura usa lenguaje simple: Mapa de Conexiones y Puntos de Contacto Externo reemplazan a Dependency Graph y API Surface",
        "f3": "Configuración de Jira y Entorno con explicaciones en lenguaje simple; el token de API ahora se llama Clave de Acceso"
      },
      "v080": {
```

- [ ] **Step 5: Bump `CURRENT_VERSION` and prepend the `RELEASE_NOTES` entry in `src/data/releaseNotes.ts`**

Replace:
```typescript
export const CURRENT_VERSION = "0.8.0";
```
with:
```typescript
export const CURRENT_VERSION = "0.9.0";
```

Replace:
```typescript
export const RELEASE_NOTES: ReleaseNote[] = [
  {
    version: "0.8.0",
```
with:
```typescript
export const RELEASE_NOTES: ReleaseNote[] = [
  {
    version: "0.9.0",
    titleKey: "whatsNew.releases.v090.title",
    color: "accent",
    features: [
      { icon: "i18n", labelKey: "whatsNew.releases.v090.f1" },
      { icon: "artifact", labelKey: "whatsNew.releases.v090.f2" },
      { icon: "settings", labelKey: "whatsNew.releases.v090.f3" },
    ],
  },
  {
    version: "0.8.0",
```

- [ ] **Step 6: Verify all four files agree, and JSON stays valid**

```bash
grep -E '"version"|^version' package.json src-tauri/tauri.conf.json src-tauri/Cargo.toml
grep -m1 "version:" src/data/releaseNotes.ts
python3 -c "import json; json.load(open('src/i18n/pt-BR.json')); json.load(open('src/i18n/en.json')); json.load(open('src/i18n/es.json')); print('json ok')"
```
Expected: all four version lines show `0.9.0`; last line prints `json ok`.

- [ ] **Step 7: Commit**

```bash
git add package.json src-tauri/tauri.conf.json src-tauri/Cargo.toml src/data/releaseNotes.ts src/i18n/pt-BR.json src/i18n/en.json src/i18n/es.json
git commit -m "chore: bump version to 0.9.0 and add release note for language simplification"
```

---

### Task 16: Final full-repo verification sweep

**Files:** None modified — final read-only check across everything Tasks 1–15 touched.

**Interfaces:** None.

- [ ] **Step 1: Confirm no stale jargon remains in any live (non-changelog) i18n string**

```bash
python3 -c "
import json, re
jargon = re.compile(r'\bworktree\b|\bbranch\b|\brama\b|pull request|\bPR\b|Grafo de Dependências|Dependency Graph|Superfície de API|API Surface|processo LGE|LGE process|git diff', re.I)
for loc in ['pt-BR','en','es']:
    d = json.load(open(f'src/i18n/{loc}.json'))
    def flat(o,p=''):
        for k,v in o.items():
            key = p+k
            if isinstance(v, dict):
                yield from flat(v, key+'.')
            else:
                yield key, v
    for k,v in flat(d):
        if k.startswith('whatsNew.releases.v0') and not k.startswith('whatsNew.releases.v090'):
            continue  # historical changelog, excluded by design
        if k in ('git.pr.titleLabel','git.pr.bodyLabel','git.pr.manualHint','git.pr.copyCommand'):
            continue  # explicitly out of scope, dev-facing
        if jargon.search(str(v)):
            print(loc, k, '=>', v)
"
```
Expected: no output. If any line prints, it identifies a key Tasks 1–11 missed — go fix that key in all three locales, then re-run this task's Step 1 and Task 12's parity check.

- [ ] **Step 2: Confirm i18n key parity one more time (in case Step 1 required a fix)**

```bash
for f in pt-BR en es; do
  jq -r 'paths(scalars) | join(".")' src/i18n/$f.json | sort > /tmp/$f.keys
done
diff /tmp/pt-BR.keys /tmp/en.keys
diff /tmp/en.keys /tmp/es.keys
```
Expected: both empty.

- [ ] **Step 3: Confirm version-sync one more time**

```bash
grep -E '"version"|^version' package.json src-tauri/tauri.conf.json src-tauri/Cargo.toml
grep -m1 "version:" src/data/releaseNotes.ts
```
Expected: all print `0.9.0`.

- [ ] **Step 4: No commit for this task unless Step 1 required a fix**

If Step 1 found and fixed a leftover jargon string:
```bash
git add src/i18n/pt-BR.json src/i18n/en.json src/i18n/es.json
git commit -m "fix(i18n): remove remaining jargon missed in earlier language simplification tasks"
```
