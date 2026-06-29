import { create } from "zustand";
import type { LgePhaseId } from "../types";
import * as api from "../lib/tauri";

const DEFAULT_MODELS: Record<LgePhaseId, string> = {
  planning: "opus",
  builder: "haiku",
  review: "sonnet",
  guardian: "opus",
};

interface SettingsState {
  phaseModels: Record<LgePhaseId, string>;
  shellEnv: string;
  jiraBaseUrl: string;
  loaded: boolean;
  fetchPhaseModels: () => Promise<void>;
  savePhaseModels: (models: Record<LgePhaseId, string>) => Promise<void>;
  fetchShellEnv: () => Promise<void>;
  saveShellEnv: (shellEnv: string) => Promise<void>;
  fetchJiraBaseUrl: () => Promise<void>;
  saveJiraBaseUrl: (jiraBaseUrl: string) => Promise<void>;
}

export const useSettingsStore = create<SettingsState>((set) => ({
  phaseModels: { ...DEFAULT_MODELS },
  shellEnv: "",
  jiraBaseUrl: "",
  loaded: false,

  fetchPhaseModels: async () => {
    try {
      const models = await api.getPhaseModels();
      set({
        phaseModels: {
          planning: models.planning ?? DEFAULT_MODELS.planning,
          builder: models.builder ?? DEFAULT_MODELS.builder,
          review: models.review ?? DEFAULT_MODELS.review,
          guardian: models.guardian ?? DEFAULT_MODELS.guardian,
        },
        loaded: true,
      });
    } catch {
      set({ loaded: true });
    }
  },

  savePhaseModels: async (models) => {
    await api.savePhaseModels(models);
    set({ phaseModels: { ...models } });
  },

  fetchShellEnv: async () => {
    try {
      const shellEnv = await api.getShellEnv();
      set({ shellEnv });
    } catch {
      // ignore — default empty string is fine
    }
  },

  saveShellEnv: async (shellEnv) => {
    await api.saveShellEnv(shellEnv);
    set({ shellEnv });
  },

  fetchJiraBaseUrl: async () => {
    try {
      const jiraBaseUrl = await api.getJiraBaseUrl();
      set({ jiraBaseUrl });
    } catch {
      // ignore — default empty string is fine
    }
  },

  saveJiraBaseUrl: async (jiraBaseUrl) => {
    await api.saveJiraBaseUrl(jiraBaseUrl);
    set({ jiraBaseUrl });
  },
}));
