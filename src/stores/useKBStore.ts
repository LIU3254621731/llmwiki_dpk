import { create } from "zustand";
import type { KnowledgeBase, KBStats } from "@/types/kb";

interface KBState {
  knowledgeBases: KnowledgeBase[];
  currentKB: KnowledgeBase | null;
  stats: KBStats | null;
  loading: boolean;
  setKnowledgeBases: (kbs: KnowledgeBase[]) => void;
  setCurrentKB: (kb: KnowledgeBase | null) => void;
  setStats: (stats: KBStats) => void;
  setLoading: (loading: boolean) => void;
}

export const useKBStore = create<KBState>((set) => ({
  knowledgeBases: [],
  currentKB: null,
  stats: null,
  loading: false,
  setKnowledgeBases: (kbs) => set({ knowledgeBases: kbs }),
  setCurrentKB: (kb) => set({ currentKB: kb }),
  setStats: (stats) => set({ stats }),
  setLoading: (loading) => set({ loading }),
}));
