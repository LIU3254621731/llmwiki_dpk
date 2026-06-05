import { create } from "zustand";

export type RightSidebarMode = "context" | "agent" | "rag" | "health";

interface AppState {
  // Sidebar toggles
  leftSidebarVisible: boolean;
  rightSidebarVisible: boolean;
  rightSidebarMode: RightSidebarMode;
  chatSidebarVisible: boolean;

  // Bottom panel
  bottomPanelVisible: boolean;
  bottomPanelHeight: number;
  reviewBadgeCount: number;

  // File browser panel
  fileBrowserVisible: boolean;

  // Canvas badge
  canvasBadgeDot: boolean;

  // Task detail navigation
  taskDetailId: string | null;

  // Legacy (kept for compatibility)
  sidebarCollapsed: boolean;

  // Actions
  toggleLeftSidebar: () => void;
  toggleRightSidebar: () => void;
  setRightSidebarMode: (mode: RightSidebarMode) => void;
  toggleChatSidebar: () => void;
  toggleBottomPanel: () => void;
  setBottomPanelHeight: (height: number) => void;
  setReviewBadgeCount: (count: number) => void;
  toggleSidebar: () => void;
  toggleFileBrowser: () => void;
  setCanvasBadgeDot: (show: boolean) => void;
  setTaskDetailId: (id: string | null) => void;
}

export const useAppStore = create<AppState>((set) => ({
  leftSidebarVisible: true,
  rightSidebarVisible: false,
  rightSidebarMode: "context",
  chatSidebarVisible: false,
  bottomPanelVisible: false,
  bottomPanelHeight: 200,
  reviewBadgeCount: 0,
  sidebarCollapsed: false,
  fileBrowserVisible: false,
  canvasBadgeDot: false,
  taskDetailId: null,

  toggleLeftSidebar: () => set((s) => ({ leftSidebarVisible: !s.leftSidebarVisible })),
  toggleRightSidebar: () => set((s) => ({ rightSidebarVisible: !s.rightSidebarVisible })),
  setRightSidebarMode: (mode) => set((state) => ({
    rightSidebarMode: mode,
    rightSidebarVisible: state.rightSidebarMode === mode ? !state.rightSidebarVisible : true,
  })),
  toggleChatSidebar: () => set((s) => ({ chatSidebarVisible: !s.chatSidebarVisible })),
  toggleBottomPanel: () => set((s) => ({ bottomPanelVisible: !s.bottomPanelVisible })),
  setBottomPanelHeight: (height) => set({ bottomPanelHeight: height }),
  setReviewBadgeCount: (count) => set({ reviewBadgeCount: count }),
  toggleSidebar: () => set((s) => ({ sidebarCollapsed: !s.sidebarCollapsed })),
  toggleFileBrowser: () => set((s) => ({ fileBrowserVisible: !s.fileBrowserVisible })),
  setCanvasBadgeDot: (show) => set({ canvasBadgeDot: show }),
  setTaskDetailId: (id) => set({ taskDetailId: id }),
}));
