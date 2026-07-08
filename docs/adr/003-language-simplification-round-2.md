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
