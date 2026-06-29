# LGE Cockpit — Novas Features para Agregar Valor

## Context

A plataforma LGE Cockpit já possui: gerenciamento de repositórios/tarefas, pipeline LGE de 4 fases (Planning/Builder/Review/Guardian), visualizador de artefatos, integração Jira, health check e i18n. Este plano propõe as próximas evoluções mais impactantes, organizadas por complexidade e valor entregue.

---

## Quick Wins (< 4h cada)

### 1. Task Search & Filter Bar
**O que é:** Input de busca + pills de status acima da lista de tarefas. Filtra por título, chave Jira e status sem tocar no backend.
**Por que:** Com 20+ tarefas por repositório, scroll manual é lento. Já temos todos os dados em `taskStore.tasks` — é filtro puro no frontend.

**Arquivos a modificar:**
- `src/components/tasks/TaskList.tsx` — adicionar `filterText` + `filterStatuses` como `useState`, derivar `filteredTasks` com `useMemo`
- `src/i18n/*.json` — keys: `tasks.filter.placeholder`, `tasks.filter.noResults`

**DB:** Nenhuma mudança. **Novos commands Tauri:** Nenhum.

---

### 2. Notificações macOS ao Completar Fase
**O que é:** Notificação nativa do OS quando qualquer fase LGE termina (sucesso ou erro).
**Por que:** Planning e Guardian (Opus) demoram 3-8 min. Desenvolvedor muda de contexto e perde o momento. Com notificação, pode retornar exatamente quando pronto.

**Implementação:**
1. Adicionar `tauri-plugin-notification = "2"` ao `Cargo.toml`
2. Registrar plugin em `lib.rs`
3. Em `commands/lge.rs`, após salvar artefato ou em caso de erro, chamar `notification().title("LGE Cockpit").body(format!("{} completa — {}", phase, task_title)).show()`

**Arquivos a modificar:**
- `src-tauri/Cargo.toml`
- `src-tauri/src/lib.rs`
- `src-tauri/src/commands/lge.rs`

**DB:** Nenhuma. **Frontend:** Nenhuma.

---

### 3. Edição de Título e Descrição da Tarefa
**O que é:** Modo de edição inline no `TaskDetail` para alterar título e descrição após criação.
**Por que:** Tarefas importadas do Jira frequentemente têm descrições verbosas que não mapeiam bem para o prompt LGE. Sem edição, o desenvolvedor precisa deletar e recriar a tarefa.

**Implementação:**
1. Novo command Rust: `update_task(id, title, description)` em `commands/tasks.rs` + query SQL em `db/queries.rs`
2. Registrar em `lib.rs`, wrapper em `tauri.ts`, action `updateTask` em `taskStore.ts`
3. Em `TaskDetail.tsx`: botão "Editar" que troca título/descrição por `<Input>`/`<TextArea>` (componentes já existem em `src/components/ui/`)

**Arquivos a modificar:**
- `src-tauri/src/commands/tasks.rs`, `db/queries.rs`, `lib.rs`
- `src/lib/tauri.ts`, `src/stores/taskStore.ts`
- `src/components/tasks/TaskDetail.tsx`

---

## Power Features (4-12h cada)

### 4. Injeção de Contexto por Fase (Run-time Instructions)
**O que é:** Textarea colapsável que aparece antes de cada fase LGE, permitindo adicionar instruções extras que são injetadas no início do prompt daquela execução específica.
**Por que:** Os prompts LGE são fixos. O desenvolvedor não consegue dizer "foque apenas no módulo de auth" ou "esta é uma POC, ignore cobertura de testes." Context injection torna o pipeline steerable sem mudar código.

**Implementação:**
1. Em `commands/lge.rs`: adicionar parâmetro `extra_context: Option<String>` em `run_lge_phase`; prependar no prompt com `"CONTEXTO ADICIONAL:\n{ctx}\n\n"`
2. Propagar o parâmetro: `tauri.ts` → `lgeStore.ts` (`runPhase`) → `LgePhasePipeline.tsx`
3. No `LgePhasePipeline.tsx`: ao clicar "Continuar", expandir inline uma `<textarea>` + botões "Executar com contexto" / "Executar sem contexto"
4. Idem no `TaskDetail.tsx` para a fase Planning (botão "Iniciar LGE")

**Arquivos:**
- `src-tauri/src/commands/lge.rs` (parâmetro extra_context)
- `src/stores/lgeStore.ts` (thread extraContext por runPhase)
- `src/lib/tauri.ts`
- `src/components/lge/LgePhasePipeline.tsx`
- `src/components/tasks/TaskDetail.tsx`

---

### 5. Edição de Artefatos Antes de Avançar
**O que é:** Botão "Editar" no painel de artefatos que troca o viewer de markdown por um `<textarea>` editável; "Salvar" persiste no disco antes de executar a próxima fase.
**Por que:** O Planning é o contrato de execução do Builder. Se o Claude errou um requisito ou perdeu uma restrição arquitetural, o desenvolvedor precisa corrigir antes de rodar o Builder — caso contrário, todas as fases seguintes implementam a coisa errada.

**Novo command Tauri:** `save_lge_artifact(task_id, phase, content)` — sobrescreve o arquivo no mesmo path que `run_lge_phase` usa.

**Arquivos:**
- `src-tauri/src/commands/lge.rs` — novo command `save_lge_artifact`
- `src-tauri/src/lib.rs` — registrar
- `src/lib/tauri.ts` — wrapper `saveLgeArtifact`
- `src/stores/lgeStore.ts` — action `updateArtifact(taskId, phaseId, content)` (atualiza store + chama Tauri)
- `src/components/lge/LgeArtifactPanel.tsx` — estado `isEditing`, toggle entre `<Markdown>` e `<textarea>`, botões Save/Cancel

---

### 6. Auto-Pilot: Executar Todas as Fases Sem Confirmação
**O que é:** Botão "Executar Tudo" que roda todas as 4 fases sequencialmente sem pausar para aprovação do usuário entre elas.
**Por que:** Para tarefas bem compreendidas (imports do Jira com descrição detalhada, refactors de rotina), a confirmação manual entre fases é atrito sem valor. Com auto-pilot + notificação (Feature 2), o desenvolvedor inicia e retorna apenas para revisar o artefato do Guardian.

**Implementação:**
- Adicionar `autopilotActive: boolean` ao `LgeTaskProcess` no `lgeStore.ts`
- No success path de `runPhase`: se `autopilotActive && nextPhase`, chamar `get().runPhase(taskId, nextPhase)` automaticamente. Em falha, sempre setar `autopilotActive: false`
- Adicionar `startAutopilot(taskId)` action
- Em `LgePhasePipeline.tsx`: botão "Executar Tudo" (quando `waitingForUserAction && !isRunning`) + badge `AUTO` visível durante auto-pilot
- Em `TaskDetail.tsx`: CTA secundário "Executar tudo automaticamente" ao lado de "Iniciar LGE"

**Arquivos:**
- `src/stores/lgeStore.ts`
- `src/components/lge/LgePhasePipeline.tsx`
- `src/components/tasks/TaskDetail.tsx`

---

### 7. Export de Artefatos como Bundle Markdown
**O que é:** Botão "Exportar" no `LgeArtifactPanel` que gera um único arquivo `.md` com todos os artefatos das fases concluídas, com headers de seção, e abre um save dialog nativo.
**Por que:** Após o LGE completar, o desenvolvedor precisa compartilhar o relatório com o time, anexar ao Jira, ou arquivar como ADR. Atualmente os arquivos estão espalhados em `docs/tasks/{task_code}/`. Um export com um clique economiza 5-10 min por tarefa.

**Novo command Tauri:** `export_lge_artifacts(task_id, output_path)` — lê artefatos em order, concatena com headers `# Planning Phase\n\n`, separados por `---\n\n`, escreve em `output_path`.

**Arquivos:**
- `src-tauri/src/commands/lge.rs` — novo command `export_lge_artifacts`
- `src-tauri/src/lib.rs`
- `src/lib/tauri.ts`
- `src/components/lge/LgeArtifactPanel.tsx` — botão "Exportar", usa `@tauri-apps/plugin-dialog` `save()` para escolher destino

---

## Vision Features (12h+ cada)

### 8. Histórico de Versões de Artefatos
**O que é:** Cada vez que uma fase executa ou um artefato é editado (Feature 5), uma snapshot é salva em nova tabela. UI permite navegar versões anteriores e restaurar.
**Por que:** Quando o desenvolvedor retry uma fase ou edita o plano, a versão anterior é silenciosamente sobrescrita. O diff entre plan v1 e v2 frequentemente contém as decisões arquiteturais mais importantes. Isso torna o Cockpit um log de engenharia auditável.

**Nova tabela:**
```sql
-- 002_artifact_versions.sql
CREATE TABLE IF NOT EXISTS artifact_versions (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL,
    phase TEXT NOT NULL,
    content TEXT NOT NULL,
    created_at TEXT NOT NULL,
    source TEXT NOT NULL DEFAULT 'run'  -- 'run' | 'edit' | 'restore'
);
CREATE INDEX idx_artifact_versions_task_phase ON artifact_versions(task_id, phase);
```

**Novos commands:** `list_artifact_versions(task_id, phase)`, `restore_artifact_version(task_id, phase, version_id)`

**Integração:** Em `run_lge_phase` e `save_lge_artifact`, INSERT na nova tabela após salvar no disco.

**Novos componentes:**
- `src/components/lge/ArtifactVersionHistory.tsx` — painel lateral com lista de versões por timestamp + source badge
- `src/stores/artifactVersionStore.ts` — Zustand store para versões por `{taskId}:{phase}`

---

### 9. Sync de Volta ao Jira após Guardian
**O que é:** Após o Guardian completar, botão "Sincronizar com Jira" que posta um comentário na issue Jira com o resumo do Guardian e opcionalmente transiciona o status para "Done".
**Por que:** Sem sync-back, o LGE é invisível para o time — o lead e o PM veem a issue como "em progresso" indefinidamente. Com sync-back, o Jira reflete automaticamente que a implementação foi concluída, aprovada pelo Guardian, com link para os artefatos.

**Implementação:** Reutiliza o padrão Claude CLI + MCP Atlassian já usado em `jira.rs`. O command `sync_to_jira(task_id)` busca `jira_key` do banco, lê o artefato Guardian do disco, constrói prompt para Claude chamar `mcp__atlassian-mcp__addCommentToJiraIssue` com o resumo formatado.

**Arquivos:**
- `src-tauri/src/commands/jira.rs` — novo command `sync_to_jira`
- `src-tauri/src/lib.rs`
- `src/lib/tauri.ts`
- `src/components/lge/LgePhasePipeline.tsx` — botão "Sincronizar com Jira" quando guardian completo e `task.source === 'jira'`
- `src/components/tasks/TaskDetail.tsx` — idem

---

## Ordem de Implementação Recomendada

| # | Feature | Horas | Dependência |
|---|---------|-------|-------------|
| 1 | Task Search & Filter | 2h | — |
| 2 | Notificações macOS | 2h | — |
| 3 | Edição de Tarefa | 4h | — |
| 4 | Injeção de Contexto | 3h | 3 (entender lge.rs) |
| 5 | Edição de Artefatos | 3h | 4 (padrão de save) |
| 6 | Auto-Pilot | 5h | lgeStore estável |
| 7 | Export Bundle | 4h | — |
| 8 | Histórico de Versões | 14h | 5 (save_lge_artifact) |
| 9 | Jira Sync-Back | 15h | LGE completo e confiável |

## Arquivos Críticos

- `src-tauri/src/commands/lge.rs` — ponto central para Features 2, 4, 5, 7, 8
- `src/stores/lgeStore.ts` — state machine para Features 4, 5, 6, 8
- `src/components/lge/LgeArtifactPanel.tsx` — UI de maior alteração (Features 5, 7, 8)
- `src-tauri/src/db/queries.rs` — novas queries para Features 3, 8
- `src-tauri/src/lib.rs` — registro de todos os novos commands

## Verificação

Para cada feature:
1. `pnpm tauri dev` — confirmar sem regressão
2. Exercitar o fluxo específico da feature
3. `pnpm tauri build` — confirmar build de produção sem erros TypeScript/Rust