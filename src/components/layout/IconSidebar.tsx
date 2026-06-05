import { useRef, useCallback } from "react";
import { useAppStore } from "@/stores/useAppStore";
import { useKBStore } from "@/stores/useKBStore";
import { useCanvasStore } from "@/stores/useCanvasStore";
import { useEditorStore, type TabType } from "@/stores/useEditorStore";
import {
  LayoutDashboard,
  FolderOpen,
  BookOpen,
  GitPullRequestDraft,
  GitGraph,
  Palette,
  Settings,
  MessageSquare,
  Database,
} from "lucide-react";

interface NavItem {
  tabType: TabType;
  path: string;
  title: string;
  icon: typeof LayoutDashboard;
  label: string;
}

const NAV_ITEMS: NavItem[] = [
  { tabType: "dashboard", path: "dashboard", title: "首页", icon: LayoutDashboard, label: "仪表盘" },
  { tabType: "file_explorer", path: "file_explorer", title: "文件浏览", icon: FolderOpen, label: "文件浏览" },
  { tabType: "wiki_graph", path: "wiki-graph", title: "Wiki & 图谱", icon: BookOpen, label: "Wiki 词条" },
  { tabType: "import_review", path: "import-review", title: "导入与审阅", icon: GitPullRequestDraft, label: "审阅工作台" },
  { tabType: "graph", path: "knowledge-graph", title: "知识图谱", icon: GitGraph, label: "知识图谱" },
  { tabType: "canvas", path: "canvas", title: "画布", icon: Palette, label: "画布" },
  { tabType: "settings", path: "settings", title: "设置", icon: Settings, label: "设置" },
];

export default function IconSidebar() {
  const chatSidebarVisible = useAppStore((s) => s.chatSidebarVisible);
  const toggleChatSidebar = useAppStore((s) => s.toggleChatSidebar);
  const currentKB = useKBStore((s) => s.currentKB);
  const reviewBadgeCount = useAppStore((s) => s.reviewBadgeCount);
  const generationLock = useCanvasStore((s) => s.generationLock);

  const openTabs = useEditorStore((s) => s.openTabs);
  const activeTabId = useEditorStore((s) => s.activeTabId);
  const openFile = useEditorStore((s) => s.openFile);

  const activeTab = openTabs.find((t) => t.id === activeTabId);

  const navRef = useRef<HTMLDivElement>(null);

  const handleNavKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      const container = navRef.current;
      if (!container) return;
      const buttons = container.querySelectorAll<HTMLButtonElement>('button');
      const current = document.activeElement;
      const currentIndex = Array.from(buttons).indexOf(current as HTMLButtonElement);

      if (e.key === "ArrowDown") {
        e.preventDefault();
        const nextIndex = currentIndex < 0 ? 0 : (currentIndex + 1) % buttons.length;
        buttons[nextIndex]?.focus();
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        const prevIndex = currentIndex < 0
          ? buttons.length - 1
          : (currentIndex - 1 + buttons.length) % buttons.length;
        buttons[prevIndex]?.focus();
      } else if (e.key === "Home") {
        e.preventDefault();
        buttons[0]?.focus();
      } else if (e.key === "End") {
        e.preventDefault();
        buttons[buttons.length - 1]?.focus();
      }
    },
    []
  );

  return (
    <div className="w-[60px] h-full bg-sidebar-bg border-r border-border flex flex-col items-center py-4 gap-1 shrink-0 select-none z-20">
      {/* KB Logo / Badge */}
      <div
        className="w-9 h-9 rounded-lg bg-primary flex items-center justify-center mb-4"
        title={currentKB?.name ?? "LLMWiki"}
      >
        <Database size={16} className="text-primary-foreground" />
      </div>

      {/* Nav items */}
      <div ref={navRef} role="navigation" onKeyDown={handleNavKeyDown} className="contents">
      {NAV_ITEMS.map((item) => {
        const isActive = activeTab?.type === item.tabType;
        const Icon = item.icon;
        const showBadge = item.tabType === "import_review" && reviewBadgeCount > 0;

        return (
          <button
            key={item.tabType}
            type="button"
            onClick={() => {
              if (generationLock && item.tabType === "canvas") return;
              if (isActive) {
                // Already on this tab — scroll to top or focus
                return;
              }
              openFile({
                path: item.path,
                title: item.title,
                type: item.tabType,
              });
            }}
            className={`relative w-10 h-10 flex items-center justify-center rounded-lg transition-colors ${
              isActive
                ? "text-sidebar-icon-active bg-sidebar-hover"
                : generationLock && item.tabType === "canvas"
                  ? "text-sidebar-icon/30 cursor-not-allowed"
                  : "text-sidebar-icon hover:text-foreground hover:bg-sidebar-hover"
            }`}
            title={item.tabType === "canvas" ? "画布 (Canvas) - AI 知识重组工作台" : item.label}
            aria-label={item.label}
            aria-current={isActive ? "page" : undefined}
          >
            {/* Active indicator bar */}
            {isActive && (
              <div className="absolute left-0 top-1/2 -translate-y-1/2 w-[3px] h-6 bg-primary rounded-r-full" />
            )}
            <Icon size={18} />
            {showBadge && (
              <span className="absolute top-1 right-1 w-3.5 h-3.5 rounded-full bg-destructive text-[10px] text-destructive-foreground flex items-center justify-center font-medium">
                {reviewBadgeCount > 9 ? "9+" : reviewBadgeCount}
              </span>
            )}
          </button>
        );
      })}
      </div>

      {/* Spacer */}
      <div className="flex-1" />

      {/* Chat toggle at bottom */}
      <button
        type="button"
        onClick={toggleChatSidebar}
        className={`w-10 h-10 flex items-center justify-center rounded-lg transition-colors ${
          chatSidebarVisible
            ? "text-sidebar-icon-active bg-sidebar-hover"
            : "text-sidebar-icon hover:text-foreground hover:bg-sidebar-hover"
        }`}
        title="AI 助手"
        aria-label="AI 助手"
      >
        {chatSidebarVisible && (
          <div className="absolute left-0 top-1/2 -translate-y-1/2 w-[3px] h-6 bg-primary rounded-r-full" />
        )}
        <MessageSquare size={18} />
      </button>
    </div>
  );
}
