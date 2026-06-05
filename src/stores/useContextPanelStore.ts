import { create } from "zustand";

export type ContextPanelMode = "outline" | "backlinks" | "local_graph" | "info" | "none";
export type ContextPanelSource = "editor" | "pdf_viewer" | "graph" | "chat" | "welcome";

interface ContextPanelState {
  visible: boolean;
  mode: ContextPanelMode;
  context: Record<string, any> | null;
  sourceTabType: ContextPanelSource | null;
  setContext: (mode: ContextPanelMode, context: Record<string, any>) => void;
  clear: () => void;
  toggle: (mode: ContextPanelMode, context?: Record<string, any>) => void;
  autoAdapt: (tabType: ContextPanelSource, data?: Record<string, any>) => void;
}

export const useContextPanelStore = create<ContextPanelState>((set) => ({
  visible: false,
  mode: "none",
  context: null,
  sourceTabType: null,
  setContext: (mode, context) =>
    set({ visible: true, mode, context }),
  clear: () =>
    set({ visible: false, mode: "none", context: null, sourceTabType: null }),
  toggle: (mode, context) =>
    set((state) => {
      if (state.visible && state.mode === mode) {
        return { visible: false, mode: "none", context: null, sourceTabType: null };
      }
      return { visible: true, mode, context: context ?? state.context, sourceTabType: context?.sourceTabType ?? null };
    }),
  autoAdapt: (tabType, data = {}) => {
    switch (tabType) {
      case "editor":
        set({
          visible: true,
          mode: "outline",
          context: { ...data, source: "editor" },
          sourceTabType: "editor",
        });
        break;
      case "pdf_viewer":
        set({
          visible: true,
          mode: "info",
          context: { ...data, source: "pdf_viewer" },
          sourceTabType: "pdf_viewer",
        });
        break;
      case "graph":
        set({
          visible: true,
          mode: "info",
          context: { ...data, source: "graph" },
          sourceTabType: "graph",
        });
        break;
      case "chat":
        set({
          visible: true,
          mode: "info",
          context: { ...data, source: "chat" },
          sourceTabType: "chat",
        });
        break;
      case "welcome":
        set({
          visible: true,
          mode: "info",
          context: { ...data, source: "welcome" },
          sourceTabType: "welcome",
        });
        break;
    }
  },
}));
