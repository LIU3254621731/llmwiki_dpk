import { create } from "zustand";
import type { VdbStatus, EmbeddingConfig } from "@/types/vdb";

interface VdbState {
  status: VdbStatus | null;
  config: EmbeddingConfig | null;
  progress: { current: number; total: number; message: string } | null;
  loading: boolean;
  reindexing: boolean;
  error: string | null;

  setStatus: (s: VdbStatus | null) => void;
  setConfig: (c: EmbeddingConfig | null) => void;
  setProgress: (p: { current: number; total: number; message: string } | null) => void;
  setLoading: (l: boolean) => void;
  setReindexing: (r: boolean) => void;
  setError: (e: string | null) => void;
}

export const useVdbStore = create<VdbState>((set) => ({
  status: null,
  config: null,
  progress: null,
  loading: false,
  reindexing: false,
  error: null,
  setStatus: (s) => set({ status: s }),
  setConfig: (c) => set({ config: c }),
  setProgress: (p) => set({ progress: p }),
  setLoading: (l) => set({ loading: l }),
  setReindexing: (r) => set({ reindexing: r }),
  setError: (e) => set({ error: e }),
}));
