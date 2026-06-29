import { create } from "zustand";
import { listen } from "@tauri-apps/api/event";
import type { LgePhaseId, LgePhaseStatus, ArchitectureDiff } from "../types";
import * as api from "../lib/tauri";
import { useRepositoryStore } from "./repositoryStore";
import { useTaskStore } from "./taskStore";

interface LgePhaseState {
  id: LgePhaseId;
  status: LgePhaseStatus;
  artifact: string | null;
  error: string | null;
  archDiff: ArchitectureDiff | null;
  baseCommit: string | null;
  isAnalyzingArch: boolean;
}

export interface LgeTaskProcess {
  taskTitle: string;
  taskDescription: string;
  currentPhaseId: LgePhaseId | null;
  phases: Record<LgePhaseId, LgePhaseState>;
  waitingForUserAction: boolean;
  selectedArtifactTab: LgePhaseId;
  branchCreated: boolean;
  prReady: boolean;
}

const PHASE_ORDER: LgePhaseId[] = [
  "planning",
  "builder",
  "review",
  "guardian",
];

function createInitialPhases(): Record<LgePhaseId, LgePhaseState> {
  return {
    planning: { id: "planning", status: "pending", artifact: null, error: null, archDiff: null, baseCommit: null, isAnalyzingArch: false },
    builder: { id: "builder", status: "pending", artifact: null, error: null, archDiff: null, baseCommit: null, isAnalyzingArch: false },
    review: { id: "review", status: "pending", artifact: null, error: null, archDiff: null, baseCommit: null, isAnalyzingArch: false },
    guardian: { id: "guardian", status: "pending", artifact: null, error: null, archDiff: null, baseCommit: null, isAnalyzingArch: false },
  };
}

interface LgeState {
  // Per-task process state
  processes: Record<string, LgeTaskProcess>;
  // Which task's LGE view is currently shown (null = not viewing any)
  viewingTaskId: string | null;

  startProcess: (taskId: string, taskTitle: string, taskDescription: string) => void;
  resumeProcess: (taskId: string, taskTitle: string, taskDescription: string, artifacts: Record<string, string>, gitBranch?: string | null) => void;
  runPhase: (taskId: string, phaseId: LgePhaseId, extraContext?: string) => Promise<void>;
  openView: (taskId: string) => void;
  closeView: () => void;
  setSelectedArtifactTab: (taskId: string, tab: LgePhaseId) => void;
  loadExistingArtifacts: (taskId: string) => Promise<Record<string, string>>;
  getProcess: (taskId: string) => LgeTaskProcess | null;
  getNextPhase: (taskId: string) => LgePhaseId | null;
  getNextPhaseFromArtifacts: (artifacts: Record<string, string>) => LgePhaseId | null;
  isAnyPhaseRunning: (taskId: string) => boolean;
  cancelPhase: (taskId: string, phaseId: LgePhaseId) => Promise<void>;
  updateArtifact: (taskId: string, phaseId: LgePhaseId, content: string) => Promise<void>;
  setBranchCreated: (taskId: string, value: boolean) => void;
  analyzePhaseArchDiff: (taskId: string, phaseId: LgePhaseId) => Promise<void>;
}

export { PHASE_ORDER };

export const useLgeStore = create<LgeState>((set, get) => ({
  processes: {},
  viewingTaskId: null,

  startProcess: (taskId, taskTitle, taskDescription) => {
    set((state) => ({
      viewingTaskId: taskId,
      processes: {
        ...state.processes,
        [taskId]: {
          taskTitle,
          taskDescription,
          currentPhaseId: null,
          phases: createInitialPhases(),
          waitingForUserAction: false,
          selectedArtifactTab: "planning",
          branchCreated: false,
          prReady: false,
        },
      },
    }));
  },

  resumeProcess: (taskId, taskTitle, taskDescription, artifacts, gitBranch) => {
    const phases = createInitialPhases();
    let lastCompletedIdx = -1;

    for (const [phaseId, content] of Object.entries(artifacts)) {
      if (phaseId in phases) {
        phases[phaseId as LgePhaseId] = {
          ...phases[phaseId as LgePhaseId],
          status: "completed",
          artifact: content,
        };
        const idx = PHASE_ORDER.indexOf(phaseId as LgePhaseId);
        if (idx > lastCompletedIdx) lastCompletedIdx = idx;
      }
    }

    const lastCompleted = lastCompletedIdx >= 0 ? PHASE_ORDER[lastCompletedIdx] : null;

    const allPhasesCompleted = lastCompletedIdx === PHASE_ORDER.length - 1;

    set((state) => ({
      viewingTaskId: taskId,
      processes: {
        ...state.processes,
        [taskId]: {
          taskTitle,
          taskDescription,
          currentPhaseId: lastCompleted,
          phases,
          waitingForUserAction: true,
          selectedArtifactTab: lastCompleted ?? "planning",
          branchCreated: !!(gitBranch ?? state.processes[taskId]?.branchCreated),
          prReady: allPhasesCompleted && !!(gitBranch ?? state.processes[taskId]?.branchCreated),
        },
      },
    }));
  },

  runPhase: async (taskId: string, phaseId: LgePhaseId, extraContext?: string) => {
    const process = get().processes[taskId];
    if (!process) return;

    set((state) => ({
      processes: {
        ...state.processes,
        [taskId]: {
          ...state.processes[taskId],
          currentPhaseId: phaseId,
          waitingForUserAction: false,
          selectedArtifactTab: phaseId,
          phases: {
            ...state.processes[taskId].phases,
            [phaseId]: {
              ...state.processes[taskId].phases[phaseId],
              status: "running" as LgePhaseStatus,
              error: null,
            },
          },
        },
      },
    }));

    try {
      const result = await api.runLgePhase(
        taskId,
        phaseId,
        process.taskTitle,
        process.taskDescription,
        extraContext
      );
      set((state) => {
        const proc = state.processes[taskId];
        if (!proc) return state;
        // Don't override if already interrupted/failed by user
        if (proc.phases[phaseId]?.status === "failed") return state;
        const isGuardianComplete = phaseId === "guardian";
        const prReady = isGuardianComplete && proc.branchCreated;
        return {
          processes: {
            ...state.processes,
            [taskId]: {
              ...proc,
              waitingForUserAction: true,
              prReady: prReady || proc.prReady,
              phases: {
                ...proc.phases,
                [phaseId]: {
                  ...proc.phases[phaseId],
                  status: "completed" as LgePhaseStatus,
                  artifact: result.artifact_content,
                },
              },
            },
          },
        };
      });


      // Refresh tasks so the UI reflects updated status from backend
      const repoId = useRepositoryStore.getState().selectedRepoId;
      if (repoId) {
        useTaskStore.getState().fetchTasks(repoId);
      }
    } catch (error) {
      set((state) => {
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
                  status: "failed" as LgePhaseStatus,
                  error: String(error),
                },
              },
            },
          },
        };
      });
    }
  },

  openView: (taskId) => set({ viewingTaskId: taskId }),

  closeView: () => set({ viewingTaskId: null }),

  setSelectedArtifactTab: (taskId, tab) => {
    set((state) => {
      const proc = state.processes[taskId];
      if (!proc) return state;
      return {
        processes: {
          ...state.processes,
          [taskId]: { ...proc, selectedArtifactTab: tab },
        },
      };
    });
  },

  loadExistingArtifacts: async (taskId: string) => {
    try {
      return await api.loadLgeArtifacts(taskId);
    } catch {
      return {};
    }
  },

  getProcess: (taskId: string) => {
    return get().processes[taskId] ?? null;
  },

  getNextPhase: (taskId: string) => {
    const proc = get().processes[taskId];
    if (!proc || !proc.currentPhaseId) return "planning";
    const idx = PHASE_ORDER.indexOf(proc.currentPhaseId);
    if (idx < PHASE_ORDER.length - 1) return PHASE_ORDER[idx + 1];
    return null;
  },

  getNextPhaseFromArtifacts: (artifacts: Record<string, string>) => {
    let lastIdx = -1;
    for (const phaseId of Object.keys(artifacts)) {
      const idx = PHASE_ORDER.indexOf(phaseId as LgePhaseId);
      if (idx > lastIdx) lastIdx = idx;
    }
    if (lastIdx < PHASE_ORDER.length - 1) return PHASE_ORDER[lastIdx + 1];
    return null;
  },

  isAnyPhaseRunning: (taskId: string) => {
    const proc = get().processes[taskId];
    if (!proc) return false;
    return Object.values(proc.phases).some(
      (p) => p.status === "running" || p.status === "queued"
    );
  },

  updateArtifact: async (taskId: string, phaseId: LgePhaseId, content: string) => {
    await api.saveLgeArtifact(taskId, phaseId, content);
    set((state) => {
      const proc = state.processes[taskId];
      if (!proc) return state;
      return {
        processes: {
          ...state.processes,
          [taskId]: {
            ...proc,
            phases: {
              ...proc.phases,
              [phaseId]: { ...proc.phases[phaseId], artifact: content },
            },
          },
        },
      };
    });
  },

  setBranchCreated: (taskId: string, value: boolean) => {
    set((state) => {
      const proc = state.processes[taskId];
      if (!proc) return state;
      return {
        processes: {
          ...state.processes,
          [taskId]: { ...proc, branchCreated: value },
        },
      };
    });
  },

  analyzePhaseArchDiff: async (taskId: string, phaseId: LgePhaseId) => {
    set((state) => {
      const proc = state.processes[taskId];
      if (!proc) return state;
      return {
        processes: {
          ...state.processes,
          [taskId]: {
            ...proc,
            phases: {
              ...proc.phases,
              [phaseId]: { ...proc.phases[phaseId], isAnalyzingArch: true, archDiff: null },
            },
          },
        },
      };
    });

    try {
      const archDiff = await api.analyzeWorkingTreeDiff(taskId);
      set((state) => {
        const proc = state.processes[taskId];
        if (!proc) return state;
        return {
          processes: {
            ...state.processes,
            [taskId]: {
              ...proc,
              phases: {
                ...proc.phases,
                [phaseId]: { ...proc.phases[phaseId], archDiff, isAnalyzingArch: false },
              },
            },
          },
        };
      });
    } catch {
      set((state) => {
        const proc = state.processes[taskId];
        if (!proc) return state;
        return {
          processes: {
            ...state.processes,
            [taskId]: {
              ...proc,
              phases: {
                ...proc.phases,
                [phaseId]: { ...proc.phases[phaseId], isAnalyzingArch: false },
              },
            },
          },
        };
      });
    }
  },

  cancelPhase: async (taskId: string, phaseId: LgePhaseId) => {
    // Fire backend cancellation first (sets planning_cancelled flag or kills PID)
    try {
      await api.cancelLgePhase(taskId, phaseId);
    } catch {
      // Best effort — process may already be gone
    }
    // If planning is queued, mark it failed immediately so UI updates right away;
    // the backend cancellation flag will prevent it from executing when dequeued
    if (phaseId === "planning") {
      const proc = get().processes[taskId];
      if (proc && proc.phases.planning.status === "queued") {
        set((state) => {
          const p = state.processes[taskId];
          if (!p) return state;
          return {
            processes: {
              ...state.processes,
              [taskId]: {
                ...p,
                waitingForUserAction: true,
                phases: {
                  ...p.phases,
                  planning: {
                    ...p.phases.planning,
                    status: "failed" as LgePhaseStatus,
                    error: "Interrupted by user",
                  },
                },
              },
            },
          };
        });
      }
    }
    set((state) => {
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
                status: "failed" as LgePhaseStatus,
                error: "Interrupted by user",
              },
            },
          },
        },
      };
    });
  },
}));

export async function initPlanningQueueListeners(): Promise<() => void> {
  const unlisten1 = await listen<{ task_id: string; phase: string }>(
    "lge_phase_queued",
    ({ payload }) => {
      useLgeStore.setState((state) => {
        const proc = state.processes[payload.task_id];
        if (!proc) return state;
        return {
          processes: {
            ...state.processes,
            [payload.task_id]: {
              ...proc,
              phases: {
                ...proc.phases,
                planning: { ...proc.phases.planning, status: "queued" as LgePhaseStatus },
              },
            },
          },
        };
      });
    }
  );

  const unlisten2 = await listen<{ task_id: string; phase: string }>(
    "lge_phase_dequeued",
    ({ payload }) => {
      useLgeStore.setState((state) => {
        const proc = state.processes[payload.task_id];
        if (!proc) return state;
        return {
          processes: {
            ...state.processes,
            [payload.task_id]: {
              ...proc,
              phases: {
                ...proc.phases,
                planning: { ...proc.phases.planning, status: "running" as LgePhaseStatus },
              },
            },
          },
        };
      });
    }
  );

  return () => {
    unlisten1();
    unlisten2();
  };
}
