import { useEffect } from "react";
import { useLgeStore } from "../stores/lgeStore";
import { useTaskStore } from "../stores/taskStore";
import { useRepositoryStore } from "../stores/repositoryStore";
import { useDemoUIStore } from "./demoStore";
import { DEMO_TASK, DEMO_ARTIFACTS } from "./demoArtifacts";
import type { LgePhaseId } from "../types";

const PHASE_CONFIG: Record<LgePhaseId, { runningMs: number; toast: string }> = {
  planning: {
    runningMs: 6000,
    toast: "⚙️  Fase Planning — Claude gerando plano de implementação...",
  },
  builder: {
    runningMs: 7500,
    toast: "🔨  Fase Builder — Claude implementando a solução...",
  },
  review: {
    runningMs: 5500,
    toast: "🔍  Fase Review — Claude revisando a implementação...",
  },
  guardian: {
    runningMs: 6500,
    toast: "🛡️  Fase Guardian — Claude validando qualidade e segurança...",
  },
};

const delay = (ms: number) => new Promise<void>((r) => setTimeout(r, ms));

let isDemoRunning = false;

// ── Patch 2: runPhase para o taskId do demo ───────────────────────────────────
// Aplicado após o task ser criado. Intercepta cada clique de "Continuar"
// e simula a fase sem acionar o Claude CLI real.
function setupRunPhasePatch(taskId: string) {
  const originalRunPhase = useLgeStore.getState().runPhase;
  const ui = useDemoUIStore.getState;

  function cleanupPatch() {
    isDemoRunning = false;
    useLgeStore.setState({ runPhase: originalRunPhase });
    ui().setToast(null);
  }

  async function simulateDemoPhase(phaseId: LgePhaseId) {
    try {
      const config = PHASE_CONFIG[phaseId];
      ui().setToast(config.toast);

      // Fase → running
      useLgeStore.setState((state) => {
        const proc = state.processes[taskId];
        if (!proc) return state;
        return {
          processes: {
            ...state.processes,
            [taskId]: {
              ...proc,
              currentPhaseId: phaseId,
              waitingForUserAction: false,
              selectedArtifactTab: phaseId,
              phases: {
                ...proc.phases,
                [phaseId]: {
                  ...proc.phases[phaseId],
                  status: "running",
                  error: null,
                },
              },
            },
          },
        };
      });

      await delay(config.runningMs);
      ui().setToast(null);

      // Fase → completed com artefato fake
      useLgeStore.setState((state) => {
        const proc = state.processes[taskId];
        if (!proc) return state;
        return {
          processes: {
            ...state.processes,
            [taskId]: {
              ...proc,
              waitingForUserAction: true,
              phases: {
                ...proc.phases,
                [phaseId]: {
                  ...proc.phases[phaseId],
                  status: "completed",
                  artifact: DEMO_ARTIFACTS[phaseId],
                },
              },
            },
          },
        };
      });

      // Após o Guardian: marcar tarefa como concluída e restaurar o store
      if (phaseId === "guardian") {
        try {
          await useTaskStore.getState().updateTaskStatus(taskId, "completed");
        } catch {
          // best effort
        }
        cleanupPatch();
      }
    } catch {
      cleanupPatch();
    }
  }

  useLgeStore.setState({
    runPhase: async (tId, phaseId, _extraContext) => {
      if (tId === taskId) {
        await simulateDemoPhase(phaseId);
      } else {
        return originalRunPhase(tId, phaseId, _extraContext);
      }
    },
  });
}

// ── Patch 1: importJiraTask ───────────────────────────────────────────────────
// Aplicado ao digitar JEFF. Substitui a chamada real ao Jira por criação
// de task fake. Após a criação, aplica o Patch 2 para as fases LGE.
function activateDemo() {
  if (isDemoRunning) return;

  const repoId =
    useRepositoryStore.getState().selectedRepoId ??
    useRepositoryStore.getState().repositories[0]?.id;

  if (!repoId) {
    useDemoUIStore.getState().setToast("⚠️  Selecione um repositório para usar o modo demonstração");
    setTimeout(() => useDemoUIStore.getState().setToast(null), 3500);
    return;
  }

  isDemoRunning = true;
  const ui = useDemoUIStore.getState;
  const originalImportJiraTask = useTaskStore.getState().importJiraTask;

  // Avisar que o modo demo está ativo
  ui().setToast('🎬  Modo demonstração ativado — clique em "Importar do Jira" para começar');
  setTimeout(() => ui().setToast(null), 5000);

  useTaskStore.setState({
    importJiraTask: async (repositoryId: string, _jiraKey: string) => {
      // Restaurar imediatamente (só pode ser acionado uma vez)
      useTaskStore.setState({ importJiraTask: originalImportJiraTask });

      ui().setToast("🔄  Buscando tarefa ITM-2847 no Jira...");
      await delay(2500);

      // Criar o task demo real no banco
      await useTaskStore
        .getState()
        .createTask(repositoryId, DEMO_TASK.title, DEMO_TASK.description);

      const taskId = useTaskStore.getState().selectedTaskId as string;

      ui().setToast("✅  ITM-2847 importada do Jira com sucesso!");
      setTimeout(() => ui().setToast(null), 3500);

      // Ativar patch das fases LGE para este taskId
      setupRunPhasePatch(taskId);
    },
  });
}

export function useDemoMode() {
  useEffect(() => {
    let buffer = "";

    function handleKeyDown(e: KeyboardEvent) {
      const target = e.target as HTMLElement;
      if (
        target.tagName === "INPUT" ||
        target.tagName === "TEXTAREA" ||
        target.isContentEditable
      ) {
        buffer = "";
        return;
      }

      buffer = (buffer + e.key).slice(-4);
      if (buffer === "JEFF") {
        buffer = "";
        activateDemo();
      }
    }

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, []);
}
