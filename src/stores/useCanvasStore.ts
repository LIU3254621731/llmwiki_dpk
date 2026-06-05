import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import type {
  OutlineNode,
  CanvasScope,
  DetailData,
  GenerationPhase,
  ScopeCheckResult,
  OutlineGenerationResult,
  WebSourceItem,
} from "@/types/canvas";
import type { MindmapNode } from "@/components/canvas-engine/types";

interface CanvasState {
  // Tag input
  tags: string[];
  savedScopes: CanvasScope[];

  // Generation state
  generationPhase: GenerationPhase;
  generationLock: boolean;
  generationError: string;

  // Content
  outlineNodes: OutlineNode[];
  textbookContent: string;
  streamingText: string;

  // Interaction
  activeNodeId: string | null;
  detailPanelVisible: boolean;
  detailData: DetailData | null;
  detailTopic: string;

  // Cache key (shared between outline/textbook generation)
  cacheKey: string;

  // Mindmap
  mindmapTree: MindmapNode | null;
  mindmapLoading: boolean;

  // Web-sourced generation
  webSources: WebSourceItem[];
  webGenerationActive: boolean;

  // UI
  scrollPosition: number;
  loadingScopes: boolean;

  // Actions — tags
  setTags: (tags: string[]) => void;
  addTag: (tag: string) => void;
  removeTag: (tag: string) => void;

  // Actions — scopes
  loadSavedScopes: (kbId: string) => Promise<void>;
  saveScope: (kbId: string, name: string) => Promise<void>;
  deleteScope: (scopeId: string) => Promise<void>;
  renameScope: (scopeId: string, name: string) => Promise<void>;

  // Actions — generation
  checkScope: (kbId: string, tags: string[]) => Promise<ScopeCheckResult>;
  generateOutline: (kbId: string, cacheKey: string) => Promise<void>;
  triggerTextbookGeneration: (kbId: string) => Promise<void>;
  setGenerationPhase: (phase: GenerationPhase) => void;
  setGenerationLock: (locked: boolean) => void;
  setGenerationError: (error: string) => void;

  // Actions — content
  setOutlineNodes: (nodes: OutlineNode[]) => void;
  appendStreamingChunk: (chunk: string) => void;
  setTextbookContent: (content: string) => void;

  // Actions — interaction
  setActiveNodeId: (id: string | null) => void;
  showDetailPanel: (topic: string) => Promise<void>;
  hideDetailPanel: () => void;
  setDetailData: (data: DetailData | null) => void;

  // Actions — mindmap
  generateMindmap: (kbId: string) => Promise<void>;
  clearMindmap: () => void;

  // Actions — web-sourced generation
  generateFromWeb: (kbId: string, sources: WebSourceItem[]) => void;
  generateMindmapFromTextbook: (kbId: string) => Promise<void>;

  // Actions — UI
  setScrollPosition: (pos: number) => void;
  resetGeneration: () => void;
}

export const useCanvasStore = create<CanvasState>((set, get) => ({
  // Initial state
  tags: [],
  savedScopes: [],
  generationPhase: "idle",
  generationLock: false,
  generationError: "",
  outlineNodes: [],
  textbookContent: "",
  streamingText: "",
  activeNodeId: null,
  detailPanelVisible: false,
  detailData: null,
  detailTopic: "",
  cacheKey: "",
  mindmapTree: null,
  mindmapLoading: false,
  webSources: [],
  webGenerationActive: false,
  scrollPosition: 0,
  loadingScopes: false,

  // Tag actions
  setTags: (tags) => set({ tags }),
  addTag: (tag) => {
    const { tags } = get();
    if (!tags.includes(tag)) {
      set({ tags: [...tags, tag] });
    }
  },
  removeTag: (tag) => set({ tags: get().tags.filter((t) => t !== tag) }),

  // Scope actions
  loadSavedScopes: async (kbId) => {
    set({ loadingScopes: true });
    try {
      const scopes = await invoke<CanvasScope[]>("get_canvas_scopes", { kbId });
      set({ savedScopes: scopes });
    } catch (e) {
      console.error("加载画布书签失败:", e);
    } finally {
      set({ loadingScopes: false });
    }
  },

  saveScope: async (kbId, name) => {
    const { tags, scrollPosition } = get();
    try {
      await invoke("save_canvas_scope", {
        kbId,
        name,
        tagsJson: JSON.stringify(tags),
        scrollPosition,
      });
      await get().loadSavedScopes(kbId);
    } catch (e) {
      console.error("保存画布书签失败:", e);
    }
  },

  deleteScope: async (scopeId) => {
    try {
      await invoke("delete_canvas_scope", { scopeId });
      // Re-load scopes via the SavedScopesBar component which has kbId from useKBStore
    } catch (e) {
      console.error("删除画布书签失败:", e);
    }
  },

  renameScope: async (scopeId, name) => {
    try {
      await invoke("rename_canvas_scope", { scopeId, name });
    } catch (e) {
      console.error("重命名画布书签失败:", e);
    }
  },

  // Generation actions
  checkScope: async (kbId, tags) => {
    const result = await invoke<ScopeCheckResult>("check_canvas_scope", {
      kbId,
      tags,
    });
    return result;
  },

  generateOutline: async (kbId, cacheKey) => {
    set({ generationPhase: "outline", generationLock: true, generationError: "", cacheKey });
    try {
      const result = await invoke<OutlineGenerationResult>("generate_canvas_outline", {
        kbId,
        tags: get().tags,
        cacheKey,
      });
      if (!result.nodes || result.nodes.length === 0) {
        set({ generationError: "大纲生成返回空结果，请检查标签是否匹配到有效内容", generationLock: false, generationPhase: "idle" });
      } else {
        set({ outlineNodes: result.nodes });
        // Don't unlock yet — auto-transition to textbook generation via CanvasHeader effect
      }
    } catch (e) {
      set({ generationError: String(e), generationLock: false, generationPhase: "idle" });
    }
  },

  triggerTextbookGeneration: async (kbId) => {
    const { outlineNodes, tags, cacheKey } = get();
    if (outlineNodes.length === 0) return;
    set({ generationPhase: "textbook", streamingText: "", generationLock: true });
    try {
      await invoke("generate_canvas_textbook", {
        kbId,
        tags,
        outlineJson: JSON.stringify(outlineNodes),
        cacheKey,
      });
      // Streaming events handle the rest (canvas-stream-done sets phase=done, lock=false)
    } catch (e) {
      set({ generationError: String(e), generationLock: false, generationPhase: "idle" });
    }
  },

  setGenerationPhase: (phase) => set({ generationPhase: phase }),
  setGenerationLock: (locked) => set({ generationLock: locked }),
  setGenerationError: (error) => set({ generationError: error }),

  // Content actions
  setOutlineNodes: (nodes) => set({ outlineNodes: nodes }),
  appendStreamingChunk: (chunk) =>
    set({ streamingText: get().streamingText + chunk }),
  setTextbookContent: (content) =>
    set({ textbookContent: content, streamingText: "" }),

  // Interaction actions
  setActiveNodeId: (id) => set({ activeNodeId: id }),

  showDetailPanel: async (topic) => {
    set({ detailPanelVisible: true, detailTopic: topic, detailData: null });
    // Detail loading is triggered by the view component via useEffect
  },

  hideDetailPanel: () => set({ detailPanelVisible: false, detailData: null, detailTopic: "" }),

  setDetailData: (data) => set({ detailData: data }),

  // Mindmap actions
  generateMindmap: async (kbId) => {
    const { tags } = get();
    if (tags.length === 0) return;
    set({ mindmapLoading: true, mindmapTree: null });
    try {
      const tree = await invoke<MindmapNode>("generate_mindmap", {
        kbId,
        topic: tags[0],
        contextPages: tags.slice(1).join("\n"),
      });
      set({ mindmapTree: tree });
    } catch (e) {
      set({ generationError: `思维导图生成失败: ${e}` });
    } finally {
      set({ mindmapLoading: false });
    }
  },
  clearMindmap: () => set({ mindmapTree: null }),

  // Web-sourced generation actions
  generateFromWeb: async (kbId, sources) => {
    const { tags } = get();
    if (tags.length === 0) return;
    set({
      webSources: sources,
      webGenerationActive: true,
      generationPhase: "outline",
      generationLock: true,
      generationError: "",
      streamingText: "",
      textbookContent: "",
      outlineNodes: [],
    });

    // Step 1: Generate outline from web sources
    try {
      const result = await invoke<OutlineGenerationResult>("generate_canvas_outline_from_web", {
        kbId,
        tags,
        webSourcesJson: JSON.stringify(sources),
      });
      if (!result.nodes || result.nodes.length === 0) {
        set({ generationError: "大纲生成返回空结果，请调整搜索关键词", generationLock: false, generationPhase: "idle", webGenerationActive: false });
        return;
      }
      set({ outlineNodes: result.nodes, generationPhase: "textbook" });
    } catch (e) {
      set({ generationError: String(e), generationLock: false, generationPhase: "idle", webGenerationActive: false });
      return;
    }

    // Step 2: Auto-trigger textbook generation from web
    try {
      await invoke("generate_canvas_textbook_from_web", {
        kbId,
        tags,
        outlineJson: JSON.stringify(get().outlineNodes),
        webSourcesJson: JSON.stringify(sources),
      });
      // Streaming events handle the rest (canvas-stream-done sets phase=done, lock=false)
    } catch (e) {
      set({ generationError: String(e), generationLock: false, generationPhase: "idle", webGenerationActive: false });
    }
  },

  generateMindmapFromTextbook: async (kbId) => {
    const { textbookContent, tags } = get();
    if (!textbookContent || tags.length === 0) return;
    set({ mindmapLoading: true, mindmapTree: null });
    try {
      const tree = await invoke<MindmapNode>("generate_mindmap_from_text", {
        topic: tags[0],
        textContent: textbookContent,
      });
      set({ mindmapTree: tree });
    } catch (e) {
      set({ generationError: `思维导图生成失败: ${e}` });
    } finally {
      set({ mindmapLoading: false });
    }
  },

  // UI actions
  setScrollPosition: (pos) => set({ scrollPosition: pos }),
  resetGeneration: () =>
    set({
      generationPhase: "idle",
      generationLock: false,
      generationError: "",
      cacheKey: "",
      outlineNodes: [],
      textbookContent: "",
      streamingText: "",
      activeNodeId: null,
      detailPanelVisible: false,
      detailData: null,
      detailTopic: "",
    }),
}));
