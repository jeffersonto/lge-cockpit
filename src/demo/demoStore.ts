import { create } from "zustand";

interface DemoUIState {
  toast: string | null;
  setToast: (msg: string | null) => void;
}

export const useDemoUIStore = create<DemoUIState>((set) => ({
  toast: null,
  setToast: (msg) => set({ toast: msg }),
}));
