import { create } from "zustand";
import type { ProviderConfig } from "@/types/model";

interface ModelState {
  config: ProviderConfig | null;
  loading: boolean;
  testResult: string;
  setConfig: (c: ProviderConfig) => void;
  setLoading: (l: boolean) => void;
  setTestResult: (r: string) => void;
}

export const useModelStore = create<ModelState>((set) => ({
  config: null,
  loading: false,
  testResult: "",
  setConfig: (c) => set({ config: c }),
  setLoading: (l) => set({ loading: l }),
  setTestResult: (r) => set({ testResult: r }),
}));
