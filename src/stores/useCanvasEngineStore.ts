// ── Canvas Engine Zustand Store ──
// Manages state for both Macro and Micro canvases, including persistence.

import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import type { CanvasSchema, CanvasPersistState, AgentAction } from "@/components/canvas-engine/types";

interface CanvasEngineState {
  // ── Macro canvas state ──
  macroSchema: CanvasSchema | null;
  macroDirty: boolean;

  // ── Micro canvas state (multiple, keyed by tagId) ──
  microSchemas: Record<string, CanvasSchema>;
  activeMicroId: string | null;

  // ── UI state ──
  activeCanvasType: "macro" | "micro";

  // ── Actions: Macro ──
  setMacroSchema: (schema: CanvasSchema | null) => void;
  markMacroDirty: () => void;
  loadMacroCanvas: (kbId: string) => Promise<void>;
  saveMacroCanvas: (kbId: string) => Promise<void>;

  // ── Actions: Micro ──
  setMicroSchema: (tagId: string, schema: CanvasSchema) => void;
  setActiveMicroId: (tagId: string | null) => void;
  loadMicroCanvas: (kbId: string, tagId: string) => Promise<void>;
  saveMicroCanvas: (kbId: string, tagId: string) => Promise<void>;

  // ── Actions: Agent ──
  applyAgentActions: (actions: AgentAction[]) => void;
  pendingAgentActions: AgentAction[];

  // ── Actions: General ──
  setActiveCanvasType: (t: "macro" | "micro") => void;
}

export const useCanvasEngineStore = create<CanvasEngineState>((set, get) => ({
  macroSchema: null,
  macroDirty: false,
  microSchemas: {},
  activeMicroId: null,
  activeCanvasType: "macro",
  pendingAgentActions: [],

  // ── Macro actions ──
  setMacroSchema: (schema) => set({ macroSchema: schema }),
  markMacroDirty: () => set({ macroDirty: true }),

  loadMacroCanvas: async (kbId) => {
    try {
      const state = await invoke<CanvasPersistState | null>("load_canvas_state", {
        kbId,
        canvasType: "macro",
        canvasId: "default",
      });
      if (state && state.schema) {
        set({ macroSchema: state.schema, macroDirty: false });
      }
    } catch (e) {
      console.error("[CanvasEngine] loadMacroCanvas failed:", e);
    }
  },

  saveMacroCanvas: async (kbId) => {
    const { macroSchema } = get();
    if (!macroSchema) return;
    try {
      await invoke("save_canvas_state", {
        kbId,
        canvasType: "macro",
        canvasId: "default",
        schemaJson: JSON.stringify(macroSchema),
      });
      set({ macroDirty: false });
    } catch (e) {
      console.error("[CanvasEngine] saveMacroCanvas failed:", e);
    }
  },

  // ── Micro actions ──
  setMicroSchema: (tagId, schema) =>
    set((s) => ({ microSchemas: { ...s.microSchemas, [tagId]: schema } })),

  setActiveMicroId: (tagId) => set({ activeMicroId: tagId }),

  loadMicroCanvas: async (kbId, tagId) => {
    try {
      const state = await invoke<CanvasPersistState | null>("load_canvas_state", {
        kbId,
        canvasType: "micro",
        canvasId: tagId,
      });
      if (state && state.schema) {
        set((s) => ({ microSchemas: { ...s.microSchemas, [tagId]: state.schema } }));
      }
    } catch (e) {
      console.error("[CanvasEngine] loadMicroCanvas failed:", e);
    }
  },

  saveMicroCanvas: async (kbId, tagId) => {
    const schema = get().microSchemas[tagId];
    if (!schema) return;
    try {
      await invoke("save_canvas_state", {
        kbId,
        canvasType: "micro",
        canvasId: tagId,
        schemaJson: JSON.stringify(schema),
      });
    } catch (e) {
      console.error("[CanvasEngine] saveMicroCanvas failed:", e);
    }
  },

  // ── Agent actions ──
  applyAgentActions: (actions) =>
    set((s) => ({
      pendingAgentActions: [...s.pendingAgentActions, ...actions],
    })),

  // ── General actions ──
  setActiveCanvasType: (t) => set({ activeCanvasType: t }),
}));
