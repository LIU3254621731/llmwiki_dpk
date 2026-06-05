import { useEffect } from "react";
import { useAppStore } from "@/stores/useAppStore";
import { useKBStore } from "@/stores/useKBStore";
import { useReviewStore } from "@/stores/useReviewStore";
import IconSidebar from "@/components/layout/IconSidebar";
import ChatSidebar from "@/components/layout/ChatSidebar";
import StatusBar from "@/components/layout/StatusBar";
import TaskProgressTray from "@/components/layout/TaskProgressTray";
import TabBar from "@/components/editor/TabBar";
import CenterArea from "@/components/layout/CenterArea";
import { ErrorBoundary } from "@/components/common/ErrorBoundary";
import { useCitationClick } from "@/hooks/useCitationClick";
import { useEditorStore } from "@/stores/useEditorStore";

export default function WorkspacePage() {
  // 注册全局引用角标点击事件监听
  useCitationClick();
  const setReviewBadgeCount = useAppStore((s) => s.setReviewBadgeCount);
  const currentKB = useKBStore((s) => s.currentKB);
  const loadPendingReviews = useReviewStore((s) => s.loadPendingReviews);
  const pendingCount = useReviewStore((s) => s.pendingCount);
  const openTabs = useEditorStore((s) => s.openTabs);

  const hasNonWelcomeTabs = openTabs.some((t) => t.id !== "tab:welcome");

  useEffect(() => {
    if (currentKB) {
      loadPendingReviews(currentKB.id);
    }
  }, [currentKB?.id]);

  useEffect(() => {
    setReviewBadgeCount(pendingCount);
  }, [pendingCount, setReviewBadgeCount]);

  // Listen for review-updated events from backend
  useEffect(() => {
    let unlistenFn: (() => void) | undefined;

    (async () => {
      try {
        const { listen } = await import("@tauri-apps/api/event");
        const unlisten = await listen("review-updated", () => {
          const kb = useKBStore.getState().currentKB;
          if (kb) loadPendingReviews(kb.id);
        });
        unlistenFn = unlisten;
      } catch {
        // Not running in Tauri context
      }
    })();

    return () => {
      unlistenFn?.();
    };
  }, []);

  return (
    <ErrorBoundary>
      <div className="flex flex-col h-screen overflow-hidden bg-background">
        {/* Tab bar — shown when content tabs are open (non-welcome) */}
        {hasNonWelcomeTabs && <TabBar />}

        {/* Main content row */}
        <div className="flex-1 flex overflow-hidden">
          {/* 60px Icon Sidebar */}
          <IconSidebar />

          {/* Tab content — always rendered, CenterArea handles welcome tab internally */}
          <CenterArea />

          {/* Chat sidebar — 350px, collapsible */}
          <ChatSidebar />
        </div>

        {/* Task progress tray — fixed bottom-left, above status bar */}
        <TaskProgressTray />

        {/* Status bar */}
        <StatusBar />
      </div>
    </ErrorBoundary>
  );
}
