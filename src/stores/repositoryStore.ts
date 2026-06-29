import { create } from "zustand";
import type { Repository } from "../types";
import * as api from "../lib/tauri";

interface RepositoryState {
  repositories: Repository[];
  selectedRepoId: string | null;
  loading: boolean;
  fetchRepositories: () => Promise<void>;
  addRepository: (path: string) => Promise<void>;
  removeRepository: (id: string) => Promise<void>;
  selectRepository: (id: string | null) => void;
}

export const useRepositoryStore = create<RepositoryState>((set) => ({
  repositories: [],
  selectedRepoId: null,
  loading: false,

  fetchRepositories: async () => {
    set({ loading: true });
    try {
      const repositories = await api.listRepositories();
      set({ repositories, loading: false });
    } catch (error) {
      console.error("Failed to fetch repositories:", error);
      set({ loading: false });
    }
  },

  addRepository: async (path: string) => {
    try {
      const repo = await api.addRepository(path);
      set((state) => ({
        repositories: [...state.repositories, repo],
        selectedRepoId: repo.id,
      }));
    } catch (error) {
      console.error("Failed to add repository:", error);
      throw error;
    }
  },

  removeRepository: async (id: string) => {
    try {
      await api.removeRepository(id);
      set((state) => ({
        repositories: state.repositories.filter((r) => r.id !== id),
        selectedRepoId:
          state.selectedRepoId === id ? null : state.selectedRepoId,
      }));
    } catch (error) {
      console.error("Failed to remove repository:", error);
      throw error;
    }
  },

  selectRepository: (id: string | null) => {
    set({ selectedRepoId: id });
  },
}));
