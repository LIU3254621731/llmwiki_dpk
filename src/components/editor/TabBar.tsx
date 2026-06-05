import { useRef, useState, useCallback, useEffect } from "react";
import {
  ChevronLeft,
  ChevronRight,
  Plus,
  X,
  Home,
  FileText,
  File,
  GitGraph,
  MessageSquare,
  Copy,
  LayoutDashboard,
  FileUp,
  Settings,
  FolderOpen,
  Palette,
  ListTodo,
} from "lucide-react";
import { useEditorStore, type TabType } from "@/stores/useEditorStore";

const TYPE_ICONS: Record<TabType, typeof Home> = {
  welcome: Home,
  editor: FileText,
  wiki: FileText,
  file: File,
  pdf_viewer: File,
  graph: GitGraph,
  chat: MessageSquare,
  dashboard: LayoutDashboard,
  wiki_graph: GitGraph,
  import_review: FileUp,
  settings: Settings,
  file_explorer: FolderOpen,
  canvas: Palette,
  task_detail: ListTodo,
};

const WELCOME_TAB_ID = "tab:welcome";

function TabIcon({ type, size = 13 }: { type: TabType; size?: number }) {
  const Icon = TYPE_ICONS[type] || FileText;
  return <Icon size={size} />;
}

export default function TabBar() {
  const openTabs = useEditorStore((s) => s.openTabs);
  const activeTabId = useEditorStore((s) => s.activeTabId);
  const tabPinned = useEditorStore((s) => s.tabPinned);
  const setActiveTab = useEditorStore((s) => s.setActiveTab);
  const closeTab = useEditorStore((s) => s.closeTab);
  const togglePin = useEditorStore((s) => s.togglePin);
  const moveTab = useEditorStore((s) => s.moveTab);

  const [contextMenu, setContextMenu] = useState<{
    x: number;
    y: number;
    tabId: string;
  } | null>(null);

  const [scrollLeft, setScrollLeft] = useState(0);
  const [canScrollLeft, setCanScrollLeft] = useState(false);
  const [canScrollRight, setCanScrollRight] = useState(false);

  const scrollRef = useRef<HTMLDivElement>(null);
  const dragRef = useRef<{ tabId: string; fromIndex: number } | null>(null);

  const updateScrollState = useCallback(() => {
    const el = scrollRef.current;
    if (!el) return;
    setScrollLeft(el.scrollLeft);
    setCanScrollLeft(el.scrollLeft > 0);
    setCanScrollRight(el.scrollLeft + el.clientWidth < el.scrollWidth - 1);
  }, []);

  useEffect(() => {
    updateScrollState();
    const el = scrollRef.current;
    if (!el) return;
    el.addEventListener("scroll", updateScrollState, { passive: true });
    window.addEventListener("resize", updateScrollState);
    return () => {
      el.removeEventListener("scroll", updateScrollState);
      window.removeEventListener("resize", updateScrollState);
    };
  }, [updateScrollState, openTabs]);

  useEffect(() => {
    if (!contextMenu) return;
    const close = () => setContextMenu(null);
    window.addEventListener("click", close);
    return () => window.removeEventListener("click", close);
  }, [contextMenu]);

  const scrollBy = (delta: number) => {
    scrollRef.current?.scrollBy({ left: delta, behavior: "smooth" });
  };

  const handleContextMenu = (e: React.MouseEvent, tabId: string) => {
    e.preventDefault();
    setContextMenu({ x: e.clientX, y: e.clientY, tabId });
  };

  const handleCloseOthers = (tabId: string) => {
    const tabsToClose = openTabs.filter(
      (t) => t.id !== tabId && t.id !== WELCOME_TAB_ID
    );
    tabsToClose.forEach((t) => closeTab(t.id));
    setActiveTab(tabId);
    setContextMenu(null);
  };

  const handleCloseToRight = (tabId: string) => {
    const idx = openTabs.findIndex((t) => t.id === tabId);
    if (idx === -1) return;
    const tabsToClose = openTabs
      .slice(idx + 1)
      .filter((t) => t.id !== WELCOME_TAB_ID);
    tabsToClose.forEach((t) => closeTab(t.id));
    setContextMenu(null);
  };

  const handleCopyPath = async (tabId: string) => {
    const tab = openTabs.find((t) => t.id === tabId);
    if (tab?.path) {
      try {
        await navigator.clipboard.writeText(tab.path);
      } catch {
        // Fallback for environments without clipboard API
      }
    }
    setContextMenu(null);
  };

  // Drag-to-reorder handlers
  const handleDragStart = (tabId: string) => {
    const fromIndex = openTabs.findIndex((t) => t.id === tabId);
    dragRef.current = { tabId, fromIndex };
  };

  const handleDragOver = (e: React.DragEvent) => {
    e.preventDefault();
  };

  const handleDrop = (toIndex: number) => {
    if (!dragRef.current) return;
    const { fromIndex } = dragRef.current;
    // toIndex is from sortedTabs ordering; convert to openTabs index
    // so both indices are in the same array space for moveTab
    const toTab = sortedTabs[toIndex];
    const actualToIndex = openTabs.findIndex((t) => t.id === toTab.id);
    if (fromIndex !== actualToIndex && actualToIndex !== -1) {
      moveTab(fromIndex, actualToIndex);
    }
    dragRef.current = null;
  };

  const pinnedTabs = openTabs.filter((t) => tabPinned.has(t.id));
  const unpinnedTabs = openTabs.filter((t) => !tabPinned.has(t.id));
  const sortedTabs = [...pinnedTabs, ...unpinnedTabs];

  if (openTabs.length === 0) return null;

  return (
    <div className="flex items-center h-9 bg-slate-50 border-b border-slate-200 flex-shrink-0 select-none">
      {canScrollLeft && (
        <button
          type="button"
          className="flex-shrink-0 h-9 w-7 flex items-center justify-center text-slate-400 hover:text-slate-600 hover:bg-slate-100"
          onClick={() => scrollBy(-150)}
          aria-label="向左滚动标签页"
        >
          <ChevronLeft size={14} />
        </button>
      )}

      <div
        ref={scrollRef}
        className="flex-1 flex items-center overflow-x-hidden"
        style={{ scrollbarWidth: "none" }}
      >
        {sortedTabs.map((tab, idx) => {
          const isActive = tab.id === activeTabId;
          const isPinned = tabPinned.has(tab.id);
          const isWelcome = tab.id === WELCOME_TAB_ID;

          return (
            <div
              key={tab.id}
              draggable
              onDragStart={() => handleDragStart(tab.id)}
              onDragOver={handleDragOver}
              onDrop={() => handleDrop(idx)}
              onContextMenu={(e) => handleContextMenu(e, tab.id)}
              onClick={() => setActiveTab(tab.id)}
              className={`group relative flex items-center h-9 flex-shrink-0 cursor-pointer
                ${isActive ? "bg-white text-slate-900" : "text-slate-500 hover:bg-slate-100"}
                ${isPinned ? "w-10 justify-center" : "min-w-[80px] max-w-[180px] pl-3 pr-1.5"}
              `}
              role="tab"
              aria-selected={isActive}
              tabIndex={-1}
            >
              {/* Top border indicator for active tab */}
              {isActive && (
                <div className="absolute top-0 left-0 right-0 h-0.5 bg-brand-500" />
              )}

              {/* Separator between tabs */}
              {!isActive && (
                <div className="absolute right-0 top-2 bottom-2 w-px bg-slate-200" />
              )}
              {isActive && (
                <>
                  <div className="absolute left-0 top-2 bottom-2 w-px bg-slate-200" />
                  <div className="absolute right-0 top-2 bottom-2 w-px bg-slate-200" />
                </>
              )}

              {isPinned ? (
                <TabIcon
                  type={tab.type}
                  size={15}
                />
              ) : (
                <>
                  {/* Type icon */}
                  <span className="flex-shrink-0 mr-1.5 text-slate-400">
                    <TabIcon type={tab.type} size={13} />
                  </span>

                  {/* Dirty indicator */}
                  {tab.isDirty && (
                    <span className="flex-shrink-0 mr-1.5 w-1.5 h-1.5 rounded-full bg-brand-500" />
                  )}

                  {/* Title */}
                  <span className="text-xs truncate" title={tab.title}>{tab.title}</span>

                  {/* Close button */}
                  {!isWelcome && (
                    <button
                      type="button"
                      className="flex-shrink-0 ml-1.5 p-0.5 rounded-sm hover:bg-slate-200 text-slate-400 hover:text-slate-600"
                      onClick={(e) => {
                        e.stopPropagation();
                        const result = closeTab(tab.id);
                        if (result === false) {
                          if (window.confirm(`${tab.title} 有未保存的修改，确定要关闭吗？`)) {
                            closeTab(tab.id, true);
                          }
                        }
                      }}
                      aria-label={`关闭 ${tab.title}`}
                    >
                      <X size={12} />
                    </button>
                  )}
                </>
              )}
            </div>
          );
        })}
      </div>

      {canScrollRight && (
        <button
          type="button"
          className="flex-shrink-0 h-9 w-7 flex items-center justify-center text-slate-400 hover:text-slate-600 hover:bg-slate-100"
          onClick={() => scrollBy(150)}
          aria-label="向右滚动标签页"
        >
          <ChevronRight size={14} />
        </button>
      )}

      {/* Plus button */}
      <button
        type="button"
        className="flex-shrink-0 h-9 w-9 flex items-center justify-center text-slate-400 hover:text-slate-600 hover:bg-slate-100 border-l border-slate-200"
        onClick={() => {
          window.dispatchEvent(new CustomEvent("editor-open-quick-switcher"));
        }}
        aria-label="打开文件"
      >
        <Plus size={15} />
      </button>

      {/* Context menu */}
      {contextMenu && (
        <div
          className="fixed z-50 bg-white border border-slate-200 rounded-md shadow-lg py-1 min-w-[160px]"
          style={{ left: contextMenu.x, top: contextMenu.y }}
          onClick={(e) => e.stopPropagation()}
        >
          <button
            type="button"
            className="w-full text-left px-3 py-1.5 text-xs text-slate-700 hover:bg-slate-50 flex items-center gap-2"
            onClick={() => {
              const result = closeTab(contextMenu.tabId);
              if (result === false) {
                const tab = openTabs.find((t) => t.id === contextMenu.tabId);
                if (window.confirm(`${tab?.title ?? "标签页"} 有未保存的修改，确定要关闭吗？`)) {
                  closeTab(contextMenu.tabId, true);
                }
              }
              setContextMenu(null);
            }}
          >
            关闭
          </button>
          <button
            type="button"
            className="w-full text-left px-3 py-1.5 text-xs text-slate-700 hover:bg-slate-50"
            onClick={() => handleCloseOthers(contextMenu.tabId)}
          >
            关闭其他
          </button>
          <button
            type="button"
            className="w-full text-left px-3 py-1.5 text-xs text-slate-700 hover:bg-slate-50"
            onClick={() => handleCloseToRight(contextMenu.tabId)}
          >
            关闭右侧
          </button>
          <div className="border-t border-slate-100 my-1" />
          {contextMenu.tabId !== WELCOME_TAB_ID && (
            <button
              type="button"
              className="w-full text-left px-3 py-1.5 text-xs text-slate-700 hover:bg-slate-50"
              onClick={() => {
                togglePin(contextMenu.tabId);
                setContextMenu(null);
              }}
            >
              {tabPinned.has(contextMenu.tabId) ? "取消固定" : "固定"}
            </button>
          )}
          <button
            type="button"
            className="w-full text-left px-3 py-1.5 text-xs text-slate-700 hover:bg-slate-50 flex items-center gap-2"
            onClick={() => handleCopyPath(contextMenu.tabId)}
          >
            <Copy size={12} className="text-slate-400" />
            复制路径
          </button>
        </div>
      )}
    </div>
  );
}
