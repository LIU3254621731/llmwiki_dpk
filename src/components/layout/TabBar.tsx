import { useEditorStore, type TabType } from "@/stores/useEditorStore";
import { X, FileText, BookOpen, File, ChevronDown, FolderOpen, Palette, ListTodo } from "lucide-react";

const TAB_ICONS: Record<TabType, React.ReactNode> = {
  welcome: null,
  editor: <FileText size={12} />,
  wiki: <BookOpen size={12} />,
  file: <File size={12} />,
  pdf_viewer: <FileText size={12} />,
  graph: <ChevronDown size={12} />,
  chat: null,
  dashboard: null,
  wiki_graph: <ChevronDown size={12} />,
  import_review: null,
  settings: null,
  file_explorer: <FolderOpen size={12} />,
  canvas: <Palette size={12} />,
  task_detail: <ListTodo size={12} />,
};

export default function TabBar() {
  const openTabs = useEditorStore((s) => s.openTabs);
  const activeTabId = useEditorStore((s) => s.activeTabId);
  const setActiveTab = useEditorStore((s) => s.setActiveTab);
  const closeTab = useEditorStore((s) => s.closeTab);

  const contentTabs = openTabs.filter((t) => t.type !== "welcome");
  if (contentTabs.length === 0) return null;

  return (
    <div className="flex items-center h-9 bg-sidebar-bg/50 border-b border-border overflow-x-auto shrink-0">
      {openTabs.map((tab) => {
        const isActive = tab.id === activeTabId;
        const canClose = tab.type !== "welcome";

        return (
          <div
            key={tab.id}
            onClick={() => setActiveTab(tab.id)}
            className={[
              "group flex items-center gap-1.5 h-full px-3 text-xs cursor-pointer shrink-0 border-r border-border select-none transition-colors",
              isActive
                ? "bg-background text-foreground border-t-2 border-t-primary -mt-[1px]"
                : "text-muted-foreground hover:bg-background/60",
            ].join(" ")}
          >
            <span className="opacity-70">{TAB_ICONS[tab.type]}</span>
            <span className="truncate max-w-[140px]">{tab.title}</span>
            {tab.isDirty && (
              <span className="w-2 h-2 rounded-full bg-primary shrink-0" />
            )}
            {canClose && (
              <button
                type="button"
                onClick={(e) => {
                  e.stopPropagation();
                  closeTab(tab.id);
                }}
                className="p-0.5 rounded-sm opacity-0 group-hover:opacity-100 hover:bg-border transition-opacity ml-0.5"
              >
                <X size={10} />
              </button>
            )}
          </div>
        );
      })}
    </div>
  );
}
