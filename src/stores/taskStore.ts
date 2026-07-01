import { create } from "zustand";
import type { Task, TaskAttachment, TaskStatus } from "../types";
import * as api from "../lib/tauri";
import { useRepositoryStore } from "./repositoryStore";

interface TaskState {
  tasks: Task[];
  loading: boolean;
  selectedTaskId: string | null;
  attachments: Record<string, TaskAttachment[]>;
  fetchTasks: (repositoryId: string) => Promise<void>;
  createTask: (
    repositoryId: string,
    title: string,
    description?: string
  ) => Promise<void>;
  updateTaskStatus: (id: string, status: TaskStatus) => Promise<void>;
  updateTask: (id: string, title: string, description?: string) => Promise<void>;
  deleteTask: (id: string) => Promise<string[]>;
  importJiraTask: (repositoryId: string, jiraKey: string) => Promise<void>;
  createGitBranch: (
    taskId: string,
    repoPath: string,
    branchName: string,
    baseBranch: string
  ) => Promise<string>;
  removeWorktree: (taskId: string) => Promise<void>;
  selectTask: (id: string | null) => void;
  clearTasks: () => void;
  fetchAttachments: (taskId: string) => Promise<void>;
  addAttachment: (taskId: string, filePath: string, injectionPhases: string[]) => Promise<void>;
  removeAttachment: (taskId: string, attachmentId: string) => Promise<void>;
  setAttachmentPhases: (taskId: string, attachmentId: string, injectionPhases: string[]) => Promise<void>;
}

export const useTaskStore = create<TaskState>((set) => ({
  tasks: [],
  loading: false,
  selectedTaskId: null,
  attachments: {},

  fetchTasks: async (repositoryId: string) => {
    set({ loading: true });
    try {
      const tasks = await api.listTasks(repositoryId);
      set({ tasks, loading: false });
    } catch (error) {
      console.error("Failed to fetch tasks:", error);
      set({ loading: false });
    }
  },

  createTask: async (
    repositoryId: string,
    title: string,
    description?: string
  ) => {
    try {
      const task = await api.createTask({
        repository_id: repositoryId,
        title,
        description,
      });
      set((state) => ({
        tasks: [...state.tasks, task],
        selectedTaskId: task.id,
      }));
    } catch (error) {
      console.error("Failed to create task:", error);
      throw error;
    }
  },

  updateTaskStatus: async (id: string, status: TaskStatus) => {
    try {
      const updated = await api.updateTaskStatus(id, status);
      set((state) => ({
        tasks: state.tasks.map((t) => (t.id === id ? updated : t)),
      }));
    } catch (error) {
      console.error("Failed to update task:", error);
      throw error;
    }
  },

  updateTask: async (id: string, title: string, description?: string) => {
    try {
      const updated = await api.updateTask({ id, title, description });
      set((state) => ({
        tasks: state.tasks.map((t) => (t.id === id ? updated : t)),
      }));
    } catch (error) {
      console.error("Failed to update task:", error);
      throw error;
    }
  },

  deleteTask: async (id: string) => {
    try {
      const result = await api.deleteTask(id);
      set((state) => ({
        tasks: state.tasks.filter((t) => t.id !== id),
        selectedTaskId: state.selectedTaskId === id ? null : state.selectedTaskId,
      }));
      return result.errors;
    } catch (error) {
      console.error("Failed to delete task:", error);
      throw error;
    }
  },

  importJiraTask: async (repositoryId: string, jiraKey: string) => {
    try {
      const task = await api.importJiraTask(repositoryId, jiraKey);
      set((state) => ({
        tasks: [...state.tasks, task],
        selectedTaskId: task.id,
      }));
    } catch (error) {
      console.error("Failed to import Jira task:", error);
      throw error;
    }
  },

  createGitBranch: async (taskId, repoPath, branchName, baseBranch) => {
    try {
      const branch = await api.createGitBranch(taskId, repoPath, branchName, baseBranch);
      // Refresh tasks to pick up worktree_path persisted by the backend
      const task = useTaskStore.getState().tasks.find((t) => t.id === taskId);
      if (task) {
        await useTaskStore.getState().fetchTasks(task.repository_id);
      }
      // Refresh repository list to update worktree count badge
      useRepositoryStore.getState().fetchRepositories();
      return branch;
    } catch (error) {
      console.error("Failed to create git branch:", error);
      throw error;
    }
  },

  removeWorktree: async (taskId) => {
    try {
      await api.removeWorktree(taskId);
      set((state) => ({
        tasks: state.tasks.map((t) =>
          t.id === taskId ? { ...t, worktree_path: null } : t
        ),
      }));
      // Refresh repository list to update worktree count badge
      useRepositoryStore.getState().fetchRepositories();
    } catch (error) {
      console.error("Failed to remove worktree:", error);
      throw error;
    }
  },

  selectTask: (id: string | null) => set({ selectedTaskId: id }),

  clearTasks: () => set({ tasks: [], loading: false, selectedTaskId: null, attachments: {} }),

  fetchAttachments: async (taskId: string) => {
    try {
      const result = await api.listTaskAttachments(taskId);
      set((state) => ({
        attachments: { ...state.attachments, [taskId]: result },
      }));
    } catch (error) {
      console.error("Failed to fetch attachments:", error);
    }
  },

  addAttachment: async (taskId: string, filePath: string, injectionPhases: string[]) => {
    const attachment = await api.addTaskAttachment(taskId, filePath, injectionPhases);
    set((state) => ({
      attachments: {
        ...state.attachments,
        [taskId]: [...(state.attachments[taskId] ?? []), attachment],
      },
    }));
  },

  removeAttachment: async (taskId: string, attachmentId: string) => {
    await api.removeTaskAttachment(attachmentId);
    set((state) => ({
      attachments: {
        ...state.attachments,
        [taskId]: (state.attachments[taskId] ?? []).filter((a) => a.id !== attachmentId),
      },
    }));
  },

  setAttachmentPhases: async (taskId: string, attachmentId: string, injectionPhases: string[]) => {
    await api.setAttachmentPhases(attachmentId, injectionPhases);
    set((state) => ({
      attachments: {
        ...state.attachments,
        [taskId]: (state.attachments[taskId] ?? []).map((a) =>
          a.id === attachmentId ? { ...a, injection_phases: injectionPhases as TaskAttachment["injection_phases"] } : a
        ),
      },
    }));
  },
}));
