# LGE Cockpit — Roadmap de Integrações GitHub & Jira

## Context

O LGE Cockpit já possui pipeline LGE completo (4 fases), CRUD de tarefas/repositórios, importação Jira e i18n. Este roadmap define o próximo bloco de evoluções focado em integração com o ecossistema de desenvolvimento: automação Git (branches, worktrees, Pull Requests) e retroalimentação do Jira com comentários e transição de status.

---

## Feature A — Integração GitHub para Pull Request

**O que é:** Antes de iniciar o LGE, criar automaticamente uma branch Git com o nome da tarefa. Ao final (Guardian completo), oferecer ao desenvolvedor a opção de commitar, fazer push e abrir um Pull Request.

**Por que:** Hoje o desenvolvedor precisa criar a branch manualmente antes de iniciar o LGE e depois commitar os artefatos à mão. Com essa integração, o Cockpit controla todo o ciclo: branch → desenvolvimento guiado por IA → PR — sem sair do app.

**Fluxo do usuário:**
1. Ao clicar "Iniciar LGE", dialog pergunta se deve criar branch.
2. Se confirma a criacao de branch, deverá questionar qual branch base para criar a nova branch (sugestão automática: `feature/{task_code}`)
   3. Garantir que a branch base está atualizada: `git pull`e apenas esta certificacao deverá criar a nova branch
3. Backend executa `git checkout -b <branch>` no path do repositório
4. LGE roda normalmente na nova branch
5. Após Guardian completar: painel "Pronto para PR" com diff summary, campos de title/body (pré-preenchidos com resumo do Guardian), botão "Commit & Push & Abrir PR"
6. Backend executa `git add -A`, `git commit`, `git push -u origin <branch>`, depois `gh pr create` (GitHub CLI) ou fallback para URL no browser

**Novos commands Tauri (novo módulo `commands/git.rs`):**
- `create_git_branch(repo_path, branch_name) -> Result<String>` — `git checkout -b`
- `get_current_git_branch(repo_path) -> Result<String>` — branch atual
- `commit_and_push(repo_path, message) -> Result<String>` — git add + commit + push
- `create_pull_request(repo_path, title, body, base_branch) -> Result<String>` — `gh pr create` ou URL fallback

**Mudanças no DB:**
```sql
-- 002_git_branch.sql
ALTER TABLE tasks ADD COLUMN git_branch TEXT;
```

**Arquivos a modificar:**
- `src-tauri/src/commands/git.rs` — novo módulo com 4 commands
- `src-tauri/src/lib.rs` — registrar commands
- `src-tauri/migrations/002_git_branch.sql`
- `src/lib/tauri.ts` — 4 wrappers tipados
- `src/stores/taskStore.ts` — campo `gitBranch` no tipo Task
- `src/stores/lgeStore.ts` — estado `branchCreated: boolean`, `prReady: boolean`
- `src/components/tasks/TaskDetail.tsx` — dialog de criação de branch + painel PR ao final
- `src/components/lge/LgePhasePipeline.tsx` — badge de branch ativa + painel PR após Guardian
- `src/i18n/*.json` — keys: `git.createBranch`, `git.branchCreated`, `git.prReady`, `git.commitAndPush`, `git.openPr`

**Health Check:** Adicionar verificação de `gh` (GitHub CLI) em `commands/health.rs`

**Estimativa:** 10-14h

---

## Feature B — Anexar Arquivos para Contexto da IA

**O que é:** Attachments (PDF, `.md`, `.txt`, código-fonte) vinculados a uma tarefa. O conteúdo é lido e injetado no início dos prompts de cada fase LGE como contexto adicional.

**Por que:** Os prompts LGE não têm acesso a documentos externos — PRD, ADRs, especificações técnicas, contratos de API. O desenvolvedor não consegue dizer "implemente seguindo esta especificação" apontando um arquivo. Com attachments, o Cockpit torna-se um orquestrador completo sem depender de prompts manuais.

**Fluxo do usuário:**
1. Em `TaskDetail`, botão "Anexar arquivo" abre file picker nativo (`tauri-plugin-dialog`)
2. Arquivo é lido, conteúdo armazenado no SQLite, nome exibido como chip removível
3. Ao rodar qualquer fase LGE, conteúdo de todos os anexos é prependado ao prompt, considerando a sua fasesão de injeção, ou seja, cada attachment tem um campo `injection_phase` (builder, review, guardian) que determina em qual fase ele será injetado. O formato de injeção no prompt é:
   ```
   ## Contexto Adicional

   ### {filename}
   {content}

   ---
   ```
4. Validação: tamanho máximo de 2MB por arquivo; tipos aceitos: `.pdf`, `.docx`, `.md`, `.json`, `.txt`, `.csv`

**Novos commands Tauri (novo módulo `commands/attachments.rs`):**
- `add_task_attachment(task_id, file_path) -> Result<TaskAttachment>` — lê, valida, INSERT
- `list_task_attachments(task_id) -> Result<Vec<TaskAttachment>>`
- `remove_task_attachment(attachment_id) -> Result<()>`

**Nova tabela:**
```sql
-- 003_task_attachments.sql
CREATE TABLE IF NOT EXISTS task_attachments (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    file_name TEXT NOT NULL,
    file_size INTEGER NOT NULL,
    content TEXT NOT NULL,
    created_at TEXT NOT NULL
);
CREATE INDEX idx_attachments_task ON task_attachments(task_id);
```
> Considere armazenar quais arquivos são injetados em quais fases, adicionando um campo `injection_phase TEXT` (plan, builder, review, guardian) para permitir controle granular de contexto.

**Arquivos a modificar:**
- `src-tauri/src/commands/attachments.rs` — novo módulo com 3 commands
- `src-tauri/src/commands/lge.rs` — buscar attachments e injetar no prompt em `run_lge_phase`
- `src-tauri/src/db/queries.rs` — queries CRUD de attachments
- `src-tauri/src/models/mod.rs` — struct `TaskAttachment`
- `src-tauri/src/lib.rs` — registrar commands
- `src-tauri/migrations/003_task_attachments.sql`
- `src/lib/tauri.ts` — 3 wrappers tipados
- `src/stores/taskStore.ts` — sub-store de attachments por `taskId`
- `src/types/index.ts` — tipo `TaskAttachment`
- `src/components/tasks/TaskDetail.tsx` — seção de attachments com chips + botão "Anexar"
- `src/i18n/*.json` — keys: `attachments.add`, `attachments.remove`, `attachments.tooLarge`, `attachments.empty`

**Estimativa:** 8-10h

**Warning:**
- Como arquivos consomem grande quantidade de tokens, é crucial implementar validação rigorosa no backend para rejeitar arquivos muito grandes ou tipos não suportados, evitando estouros de memória ou custos excessivos de token. O frontend deve fornecer feedback claro sobre os requisitos de arquivo.
- Como arquivos ocupam espaço no banco, é importante considerar uma estratégia de limpeza ou limite de armazenamento por tarefa para evitar crescimento descontrolado do banco de dados. Poderia ser implementado um mecanismo de expiração automática, de 7 dias após a tarefa ser marcada como "Done", por exemplo.
---

## Feature C — Retroalimentação do Jira (Comentários + Transição de Status)

**O que é:** Após qualquer fase LGE, opção de postar comentário no Jira com o resultado daquela fase. Após o Guardian, opção adicional de transicionar o status da issue (ex: In Progress → In Review → Done).

**Por que:** Sem sync-back, o LGE é invisível para o time — o lead e o PM veem a issue como "em progresso" indefinidamente. Com retroalimentação, cada fase completada gera evidência no Jira e o status reflete a realidade do desenvolvimento. Reutiliza a infraestrutura Claude CLI + MCP Atlassian já em `jira.rs`.

**Fluxo do usuário:**
1. Painel de artefatos: botão "Enviar para Jira" (visível somente se `task.source === 'jira'`)
2. Dialog confirma: preview do artefato (500 chars), dropdown de transição de status (opcional)
3. Backend formata comentário com header de fase, envia via Claude CLI + MCP Atlassian
4. Badge "Sincronizado" com timestamp em `TaskDetail`

**Novos commands Tauri (em `commands/jira.rs`):**
- `post_jira_comment(task_id, phase_id) -> Result<String>` — lê artefato, envia comentário formatado
- `transition_jira_issue(task_id, target_status) -> Result<String>` — transiciona status via MCP Atlassian

**Mudanças no DB:**
```sql
-- 004_jira_sync.sql
ALTER TABLE tasks ADD COLUMN jira_synced_at TEXT;
ALTER TABLE tasks ADD COLUMN jira_last_comment TEXT;
```

**Arquivos a modificar:**
- `src-tauri/src/commands/jira.rs` — 2 novos commands (reutiliza padrão spawn Claude CLI existente)
- `src-tauri/src/lib.rs`
- `src-tauri/migrations/004_jira_sync.sql`
- `src/lib/tauri.ts`
- `src/stores/taskStore.ts` — campos `jiraSyncedAt`, `jiraLastComment`
- `src/components/lge/LgeArtifactPanel.tsx` — botão "Enviar para Jira" condicional
- `src/components/lge/LgePhasePipeline.tsx` — botão "Fechar no Jira" após Guardian
- `src/components/tasks/TaskDetail.tsx` — badge de sync com timestamp
- `src/i18n/*.json` — keys: `jira.postComment`, `jira.transition`, `jira.synced`, `jira.syncFailed`

**Dependência:** Reutiliza padrão `jira.rs` (Claude CLI + MCP Atlassian) — sem dependências novas.

**Estimativa:** 6-8h

---

## Feature D — Integração com Git Worktree

**O que é:** Criar um `git worktree` isolado por tarefa, permitindo múltiplas tarefas LGE rodando em paralelo em diretórios separados sem conflito de branches.

**Por que:** `git worktree` permite trabalhar em múltiplas branches simultaneamente em diretórios distintos — vinculados ao mesmo repositório. Com isso, o desenvolvedor pode ter 3 tarefas LGE evoluindo em paralelo, cada uma em sua própria branch e diretório, sem stash ou troca de contexto. Complementa a Feature A.

**Conceito técnico:**
```bash
git worktree add ../lge-worktrees/{task_code} -b feature/{task_code}
# Cria ./lge-worktrees/TASK-123/ com branch feature/TASK-123 isolada
```

**Fluxo do usuário:**
1. Em `TaskDetail`: toggle "Usar Worktree Isolado" (desativado por padrão)
2. Ao ativar + iniciar LGE: backend cria worktree em `{repo_path}/../lge-worktrees/{task_code}/`
3. Todas as operações LGE e git da Feature A operam no path do worktree
4. Badge "Worktree Ativo" com path no `TaskDetail`; count badge no Sidebar por repositório
5. Ao completar a tarefa: botão "Remover Worktree" (mantém branch, remove diretório)

**Novos commands Tauri (adicionados em `commands/git.rs` da Feature A):**
- `create_git_worktree(repo_path, task_code, branch_name) -> Result<String>` — `git worktree add`; retorna `worktree_path`
- `remove_git_worktree(repo_path, worktree_path) -> Result<()>` — `git worktree remove`
- `list_git_worktrees(repo_path) -> Result<Vec<GitWorktree>>` — `git worktree list --porcelain`
- `prune_git_worktrees(repo_path) -> Result<()>` — `git worktree prune`

**Mudanças no DB:**
```sql
-- 005_worktree.sql
ALTER TABLE tasks ADD COLUMN worktree_path TEXT;
```

**Novo modelo Rust:**
```rust
pub struct GitWorktree {
    pub path: String,
    pub branch: String,
    pub head_sha: String,
    pub locked: bool,
}
```

**Arquivos a modificar:**
- `src-tauri/src/commands/git.rs` — 4 commands adicionais (mesmo módulo da Feature A)
- `src-tauri/src/commands/lge.rs` — usar `worktree_path` como working directory ao invocar Claude CLI
- `src-tauri/src/models/mod.rs` — struct `GitWorktree`
- `src-tauri/src/lib.rs`
- `src-tauri/migrations/005_worktree.sql`
- `src/lib/tauri.ts`
- `src/stores/taskStore.ts` — campo `worktreePath`
- `src/types/index.ts` — tipo `GitWorktree`
- `src/components/tasks/TaskDetail.tsx` — toggle Worktree + badge de path ativo
- `src/components/layout/Sidebar.tsx` — badge de count de worktrees ativos por repo
- `src/i18n/*.json` — keys: `worktree.create`, `worktree.remove`, `worktree.active`, `worktree.isolated`

**Dependência:** Compartilha `commands/git.rs` com Feature A. Feature D é complementar — pode usar worktree com ou sem PR.

**Estimativa:** 10-12h

---

## Feature E — PR Verdict: Veredito Consolidado dos Agentes

**O que é:** Após todas as 4 fases LGE completarem, exibir automaticamente um card de veredito consolidado mostrando se os agentes Reviewer e Guardian concordam com a implementação — APPROVED, APPROVED WITH CAVEATS ou REJECTED.

**Por que:** O Guardian já produz um veredito final em seu artefato, mas ele fica enterrado no markdown. O desenvolvedor precisa ler o arquivo completo para saber se pode abrir o PR. Com o Verdict Card, o resultado fica imediatamente visível sem abrir nenhuma aba.

**Fluxo do usuário:**
1. Após Guardian completar, um card de veredito aparece automaticamente acima das tabs de artefatos no `LgeArtifactPanel`
2. Badge grande com cor: ✅ APPROVED (verde) | ⚠️ APPROVED WITH CAVEATS (amarelo) | ❌ REJECTED (vermelho)
3. Seção colapsável com findings do Reviewer e assessment do Guardian lado a lado
4. Indicador de concordância entre os dois agentes

**Implementação:**

**Frontend-only — sem chamadas AI adicionais.** O Guardian já inclui "Final verdict: APPROVED / APPROVED WITH CAVEATS / REJECTED" e tabela de qualidade progressiva.

- `src/lib/verdictParser.ts` (novo) — funções puras de parsing:
  ```typescript
  extractGuardianVerdict(content: string): "approved" | "approved_with_caveats" | "rejected" | null
  extractReviewerConcerns(content: string): string[]  // bullet list das 3 dimensões
  ```
  Regex: captura `APPROVED WITH CAVEATS`, `APPROVED`, `REJECTED` (case-insensitive, prioridade por especificidade)

- `src/components/lge/VerdictCard.tsx` (novo) — card exibido quando `allPhasesCompleted && guardianArtifact != null`
  - Props: `reviewerArtifact: string`, `guardianArtifact: string`
  - Layout: badge de status + seção de findings + badge de concordância ("Reviewer e Guardian alinhados" / "Reviewer sinalizou problemas")

**Arquivos a modificar:**
- `src/lib/verdictParser.ts` — novo módulo de parsing
- `src/components/lge/VerdictCard.tsx` — novo componente
- `src/components/lge/LgeArtifactPanel.tsx` — renderizar `<VerdictCard>` acima das tabs quando todas as fases estiverem completas
- `src/i18n/*.json` — keys: `verdict.title`, `verdict.approved`, `verdict.approvedWithCaveats`, `verdict.rejected`, `verdict.reviewerFindings`, `verdict.guardianAssessment`, `verdict.aligned`, `verdict.diverged`

**Estimativa:** 3-4h

---

## Feature F — BDD Test Scenarios

**O que é:** Anexar arquivos `.feature` (BDD/Gherkin) a uma tarefa para que os agentes de builder, review e guardian os utilizem como contrato de testes a ser satisfeito pela implementação.

**Por que:** Diferente dos attachments genéricos da Feature B (PRDs, ADRs, specs de contexto), arquivos BDD são **contratos executáveis de comportamento** — o agente não deve apenas lê-los como contexto, mas garantir que a implementação os satisfaça. A injeção é feita numa seção dedicada do prompt com semântica específica de validação, separada do contexto geral.

**Relação com Feature B:** Complementar, não duplicada. Feature B injeta contexto documental ("leia isso para entender o que fazer"). Feature F injeta contratos de teste ("implemente de forma que estes cenários passem").

**Fluxo do usuário:**
1. Em `TaskDetail`, seção "BDD Scenarios" com botão "Anexar .feature"
2. File picker nativo filtra por extensão `.feature`
3. Arquivo exibido como chip removível com nome e tamanho
4. Ao rodar fases `builder`, `review` ou `guardian`, conteúdo injetado no prompt em seção dedicada:
   ```
   ## BDD TEST SCENARIOS

   Os cenários abaixo são contratos de comportamento e DEVEM ser satisfeitos pela implementação:

   ### {file_name}
   ```gherkin
   {file_content}
   ```
   ```

**Novos commands Tauri (novo módulo `commands/bdd.rs`):**
- `add_bdd_file(task_id, file_path) -> Result<BddFile>` — valida extensão, armazena path no DB
- `list_bdd_files(task_id) -> Result<Vec<BddFile>>`
- `remove_bdd_file(file_id) -> Result<()>`

**Nova tabela:**
```sql
-- 006_bdd_files.sql
CREATE TABLE IF NOT EXISTS task_bdd_files (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    file_path TEXT NOT NULL,
    file_name TEXT NOT NULL,
    created_at TEXT NOT NULL
);
CREATE INDEX idx_bdd_files_task ON task_bdd_files(task_id);
```
> Nota: apenas o path é armazenado (não o conteúdo). O conteúdo é lido em runtime via `std::fs::read_to_string` no momento da execução da fase, garantindo que edições externas ao arquivo sejam refletidas.

**Novo modelo Rust:**
```rust
pub struct BddFile {
    pub id: String,
    pub task_id: String,
    pub file_path: String,
    pub file_name: String,
    pub created_at: String,
}
```

**Arquivos a modificar:**
- `src-tauri/src/commands/bdd.rs` — novo módulo com 3 commands
- `src-tauri/src/commands/lge.rs` — carregar BDD files e injetar no prompt das fases `builder`, `review`, `guardian`
- `src-tauri/src/db/queries.rs` — queries CRUD de bdd_files
- `src-tauri/src/models/mod.rs` — struct `BddFile`
- `src-tauri/src/lib.rs` — registrar module + commands + migration 006
- `src-tauri/migrations/006_bdd_files.sql`
- `src/lib/tauri.ts` — 3 wrappers tipados
- `src/stores/taskStore.ts` — state `bddFiles: Record<taskId, BddFile[]>` + 3 actions
- `src/types/index.ts` — tipo `BddFile`
- `src/components/tasks/TaskDetail.tsx` — seção BDD com chips + botão "Anexar .feature"
- `src/i18n/*.json` — keys: `bdd.title`, `bdd.attach`, `bdd.noFiles`, `bdd.removeConfirm`

**Estimativa:** 6-8h

---

## Ordem de Implementação Recomendada

| # | Feature | Horas Est. | Prioridade | Dependência |
|---|---------|------------|------------|-------------|
| E | PR Verdict (Veredito dos Agentes) | 3-4h | Alta | Guardian completo |
| C | Jira Feedback (Comentários + Status) | 6-8h | Alta | LGE estável (já existe) |
| F | BDD Test Scenarios | 6-8h | Alta | — |
| B | Anexar Arquivos para Contexto | 8-10h | Média | — |
| A | GitHub Pull Request | 10-14h | Média | `gh` CLI instalado |
| D | Git Worktree | 10-12h | Média | Feature A (`commands/git.rs`) |

**Justificativa:** E é frontend-only e entrega valor imediato sem risco. C e F são independentes, têm alto impacto no fluxo diário e sem dependências externas. B complementa F (contexto geral vs contratos de teste). A e D compartilham infraestrutura Git e devem ser implementadas em sequência.

---

## Arquivos Críticos

- `src-tauri/src/commands/lge.rs` — ponto central para Features B, F (injeção de contexto/BDD) e D (worktree path)
- `src-tauri/src/commands/jira.rs` — Feature C (estende padrão existente)
- `src-tauri/src/commands/git.rs` — novo módulo compartilhado entre Features A e D
- `src-tauri/src/commands/bdd.rs` — novo módulo para Feature F
- `src-tauri/src/commands/attachments.rs` — novo módulo para Feature B
- `src/stores/taskStore.ts` — novos campos para Features A, B, C, D, F
- `src/components/tasks/TaskDetail.tsx` — UI de maior alteração (todas as features)
- `src/components/lge/LgeArtifactPanel.tsx` — Features C (botão Jira), E (VerdictCard)
- `src/components/lge/VerdictCard.tsx` — novo componente para Feature E
- `src/lib/verdictParser.ts` — novo módulo de parsing para Feature E
- `src-tauri/src/lib.rs` — registro de todos os novos commands

## Verificação

Para cada feature:
1. `pnpm tauri dev` — confirmar sem regressão
2. Exercitar o fluxo específico da feature end-to-end
3. `pnpm tauri build` — confirmar build de produção sem erros TypeScript/Rust
