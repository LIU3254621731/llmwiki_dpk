import { create } from "zustand";

export type TabType = "welcome" | "editor" | "wiki" | "file" | "pdf_viewer" | "graph" | "chat" | "dashboard" | "wiki_graph" | "import_review" | "settings" | "file_explorer" | "canvas" | "task_detail";
export type EditorViewMode = "edit" | "preview" | "split";

export interface EditorTab {
  id: string;
  title: string;
  path: string;
  type: TabType;
  content: string;
  isDirty: boolean;
  isLoading: boolean;
  viewMode: EditorViewMode;
  page?: number;
  sourceId?: string;
}

const WELCOME_TAB: EditorTab = {
  id: "tab:welcome",
  title: "欢迎",
  path: "",
  type: "welcome",
  content: "",
  isDirty: false,
  isLoading: false,
  viewMode: "preview",
};

interface EditorState {
  openTabs: EditorTab[];
  activeTabId: string | null;
  tabPinned: Set<string>;

  openFile: (file: {
    path: string;
    title: string;
    content?: string;
    type: TabType;
    viewMode?: EditorViewMode;
    page?: number;
    sourceId?: string;
  }) => void;
  canCloseTab: (tabId: string) => boolean;
  closeTab: (tabId: string, force?: boolean) => boolean;
  setActiveTab: (tabId: string) => void;
  togglePin: (tabId: string) => void;
  updateTabContent: (tabId: string, content: string) => void;
  setTabLoading: (tabId: string, isLoading: boolean) => void;
  markTabClean: (tabId: string) => void;
  moveTab: (fromIndex: number, toIndex: number) => void;
  setTabViewMode: (tabId: string, viewMode: EditorViewMode) => void;
  ensureWelcomeTab: () => void;
}

export const useEditorStore = create<EditorState>((set, get) => ({
  openTabs: [WELCOME_TAB],
  activeTabId: WELCOME_TAB.id,
  tabPinned: new Set<string>([WELCOME_TAB.id]),

  ensureWelcomeTab: () => {
    const { openTabs } = get();
    if (!openTabs.find((t) => t.id === WELCOME_TAB.id)) {
      set({
        openTabs: [WELCOME_TAB, ...openTabs],
        tabPinned: new Set([...get().tabPinned, WELCOME_TAB.id]),
      });
    }
  },

  openFile: (file) => {
    const { openTabs } = get();
    const tabId = `${file.type}:${file.path}`;

    // 1. If sourceId provided, try to find an existing tab with that sourceId
    if (file.sourceId) {
      const bySource = openTabs.find((t) => t.sourceId === file.sourceId);
      if (bySource) {
        // Update page if provided
        if (file.page !== undefined) {
          set({
            openTabs: openTabs.map((t) =>
              t.id === bySource.id ? { ...t, page: file.page } : t
            ),
          });
        }
        set({ activeTabId: bySource.id });
        return;
      }
    }

    // 2. Fall back to type:path matching
    const existing = openTabs.find((t) => t.id === tabId);
    if (existing) {
      if (file.page !== undefined) {
        set({
          openTabs: openTabs.map((t) =>
            t.id === existing.id ? { ...t, page: file.page } : t
          ),
        });
      }
      set({ activeTabId: tabId });
      return;
    }

    // 3. Create new tab
    const newTab: EditorTab = {
      id: tabId,
      title: file.title,
      path: file.path,
      type: file.type,
      content: file.content ?? "",
      isDirty: false,
      isLoading: !file.content,
      viewMode: file.viewMode ?? (file.type === "editor" ? "split" : "preview"),
      page: file.page,
      sourceId: file.sourceId,
    };

    set({
      openTabs: [...openTabs, newTab],
      activeTabId: tabId,
    });
  },

  canCloseTab: (tabId: string) => {
    const tab = get().openTabs.find((t) => t.id === tabId);
    return tab ? !tab.isDirty : true;
  },

  closeTab: (tabId, force) => {
    const { openTabs, activeTabId } = get();
    // Don't close welcome tab
    if (tabId === WELCOME_TAB.id) return false;

    const tab = openTabs.find((t) => t.id === tabId);
    if (tab?.isDirty && !force) return false;

    const idx = openTabs.findIndex((t) => t.id === tabId);
    if (idx === -1) return false;

    const nextTabs = openTabs.filter((t) => t.id !== tabId);

    let nextActive = activeTabId;
    if (activeTabId === tabId) {
      if (nextTabs.length === 0) {
        nextActive = WELCOME_TAB.id;
      } else if (idx >= nextTabs.length) {
        nextActive = nextTabs[nextTabs.length - 1].id;
      } else {
        nextActive = nextTabs[idx].id;
      }
    }

    set({ openTabs: nextTabs, activeTabId: nextActive });
    return true;
  },

  setActiveTab: (tabId) => set({ activeTabId: tabId }),

  togglePin: (tabId) => {
    const { tabPinned } = get();
    const next = new Set(tabPinned);
    if (next.has(tabId)) {
      next.delete(tabId);
    } else {
      next.add(tabId);
    }
    set({ tabPinned: next });
  },

  updateTabContent: (tabId, content) => {
    set({
      openTabs: get().openTabs.map((t) =>
        t.id === tabId ? { ...t, content, isDirty: true, isLoading: false } : t
      ),
    });
  },

  setTabLoading: (tabId, isLoading) => {
    set({
      openTabs: get().openTabs.map((t) =>
        t.id === tabId ? { ...t, isLoading } : t
      ),
    });
  },

  markTabClean: (tabId) => {
    set({
      openTabs: get().openTabs.map((t) =>
        t.id === tabId ? { ...t, isDirty: false } : t
      ),
    });
  },

  moveTab: (fromIndex, toIndex) => {
    const tabs = [...get().openTabs];
    if (fromIndex < 0 || fromIndex >= tabs.length || toIndex < 0 || toIndex >= tabs.length) return;
    const [moved] = tabs.splice(fromIndex, 1);
    tabs.splice(toIndex, 0, moved);
    set({ openTabs: tabs });
  },

  setTabViewMode: (tabId, viewMode) => {
    set({
      openTabs: get().openTabs.map((t) =>
        t.id === tabId ? { ...t, viewMode } : t
      ),
    });
  },
}));
