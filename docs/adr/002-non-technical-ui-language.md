# ADR 002: Non-Technical Language for Daily-Use UI

ADR Number: 002
Title: Non-Technical Language for Daily-Use UI
Date: 2026-07-06
Responsible: LGE Cockpit Team
Status: Superseded by ADR 003

## Context

LGE Cockpit is used by non-developer, business-oriented users who drive AI-assisted development but have no Git background. The UI historically surfaced raw Git/dev vocabulary directly to these users — "Worktree", "Branch", "Commit", "Pull Request", "diff", and the "LGE" acronym itself (Layered GenAI Engineering) — forcing a non-technical user to learn Git concepts just to operate a tool that was supposed to abstract them away.

## Decision

Reframe the *mental model*, not just the wording, for the daily-use surfaces a non-technical user actually touches: the task list, the development pipeline, the task workspace/record panel, and the final review/approval panel. Technical configuration screens (Settings → Environment, Jira credentials) and the Architecture Diff panel are explicitly left out of this round — they are either one-time setup done by a technically capable person, or genuinely technical reading material.

Specific renames (UI copy only, non-technical-facing surfaces):

- **Worktree → "Área de Trabalho"** ("Task Workspace"). The panel stays visible (not hidden); only the label changes.
- **Branch → "Registro de Trabalho"** ("Task Work Record"). Action button: "Iniciar Registro" (was "Criar Branch"). The badge still shows the raw branch name value.
- **Pull Request → "Aprovação"** ("Task Review Request"). Panel title "Pronto para Aprovação", button "Enviar para Aprovação", the single text field (commit message, reused as PR context) labeled "Descrição da Aprovação".
- **"Processo LGE" → "Desenvolvimento"** in functional/action copy ("Iniciar Desenvolvimento", "Desenvolvimento concluído!"). "LGE"/"LGE Cockpit" is kept only as the product's brand name (app title, health-check screen).
- **The four phase names (Planning/Builder/Review/Guardian) are kept** as the branded pipeline metaphor — not reframed — but are, for the first time, actually translated into pt-BR/es: Planejamento, Construção, Revisão, Guardião. This closes a pre-existing i18n-parity gap (all four were left in English in every locale file).
- **"Pull Request"/"PR" stays untranslated only in dev-facing exception paths** (the manual-fallback terminal command shown when automatic PR creation fails), since GitHub itself uses that vocabulary and the developer resolving the failure needs the two to match.

## Justification

- A non-technical user should not need a Git mental model to operate a tool built specifically to shield them from one.
- Progressive disclosure: technical jargon only resurfaces where a developer is already expected to intervene — the automatic-PR-creation failure path, or the one-time Environment/Jira setup — never in the happy path.
- A consistent naming family ("Desenvolvimento", "Área de Trabalho", "Registro de Trabalho", "Aprovação") lowers the cost of learning several new terms at once.
- "Revisão" was deliberately reserved for the pipeline's Review phase; the Pull-Request step was named "Aprovação" instead of "Revisão" specifically to avoid two unrelated moments (an automated AI self-check mid-pipeline vs. asking a human to approve the final result) sharing one label.

## Alternatives Considered

- **Hide the worktree/branch panel entirely from the daily view.** Rejected — the product owner wants the information to stay visible; only the vocabulary around it should change.
- **Keep "Pull Request" untranslated everywhere, treating it as a GitHub boundary term.** Considered because the developer reviewing the code will always see "Pull Request" on GitHub regardless of what Cockpit calls it. Rejected in favor of full translation ("Aprovação"), accepting that Cockpit's vocabulary will diverge from GitHub's in exchange for clarity for the business user.
- **Translate the Review phase and the Pull-Request step to the same word ("Revisão").** Rejected once the collision was noticed — the two are different moments in the flow and must not share a label.
- **Rewrite the Architecture Diff panel's technical vocabulary (Dependency Graph, API Surface, Risk Score) in business language.** Deferred — left out of scope for this round; only its entry-point copy is a candidate for future simplification, not the report's internal vocabulary.

## Consequences

- All three locale files (`pt-BR.json`, `en.json`, `es.json`) need coordinated updates to `git.worktree.*`, `git.branch.*`, `git.commit.*`, `git.pr.*`, `lge.phase.*`, `lge.title`, `lge.processComplete`, `topbar.noProcesses`, `settings.models.description`, `tasks.deleteDisabledRunning`, `tasks.deleteWarningWorktree`, `tasks.deleteWarningBranch` — per `i18n-parity.md`, all three must move together and key counts re-verified.
- `CONTEXT.md` gained new glossary entries (Task Workspace, Task Work Record, Task Review Request, Task Development) establishing this vocabulary as canonical, with `_Avoid_` lists to steer future UI copy away from raw Git jargon.
- This is a **UI-copy-only** decision: underlying code identifiers, DB columns, and Rust module names (`worktree_path`, `git_branch`, `Phase::Review`, `PhaseRunner`, etc.) are unchanged. Engineers should keep using the precise Git/domain terms internally; only user-facing strings change.
- During this design pass, `git.pr.titleLabel`/`bodyLabel` were found to be dead i18n keys (never wired to any component — the PR flow only ever collects one field, the commit message). Removing or implementing them is a separate follow-up, out of scope here.
- Settings → Environment/Jira and the Architecture Diff panel are explicitly deferred; they may be revisited in a future ADR.
