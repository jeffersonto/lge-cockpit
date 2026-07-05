import { create } from "zustand";
import type { LgePhaseId, JiraSelf, JiraConfig } from "../types";
import * as api from "../lib/tauri";

interface SettingsState {
  phaseModels: Record<LgePhaseId, string>;
  shellEnv: string;
  jiraConfig: JiraConfig;
  loaded: boolean;
  fetchPhaseModels: () => Promise<void>;
  savePhaseModels: (models: Record<LgePhaseId, string>) => Promise<void>;
  fetchShellEnv: () => Promise<void>;
  saveShellEnv: (shellEnv: string) => Promise<void>;
  fetchJiraConfig: () => Promise<void>;
  saveJiraConfig: (config: JiraConfig) => Promise<void>;
  verifyConnection: () => Promise<JiraSelf>;
}

const EMPTY_JIRA_CONFIG: JiraConfig = { baseUrl: "", email: "", apiToken: "" };

export const useSettingsStore = create<SettingsState>((set) => ({
  phaseModels: { planning: "opus", builder: "haiku", review: "sonnet", guardian: "opus" },
  shellEnv: "",
  jiraConfig: { ...EMPTY_JIRA_CONFIG },
  loaded: false,

  fetchPhaseModels: async () => {
    try {
      const models = await api.getPhaseModels();
      set({ phaseModels: models as Record<LgePhaseId, string>, loaded: true });
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

  fetchJiraConfig: async () => {
    try {
      const jiraConfig = await api.getJiraConfig();
      set({ jiraConfig });
    } catch {
      // ignore — empty defaults are fine
    }
  },

  saveJiraConfig: async (jiraConfig) => {
    await api.saveJiraConfig(jiraConfig);
    set({ jiraConfig });
  },

  verifyConnection: async () => {
    return api.verifyJiraConnection();
  },
}));