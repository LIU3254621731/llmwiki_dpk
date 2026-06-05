import { useEffect, useRef, useCallback, useState } from "react";
import { useEditorStore } from "@/stores/useEditorStore";
import { useKBStore } from "@/stores/useKBStore";
import MarkdownEditor from "@/components/editor/MarkdownEditor";
import { useAutoSave } from "@/hooks/useAutoSave";

function EmptyState() {
  return (
    <div className="flex items-center justify-center h-full select-none">
      <div className="text-center">
        <div className="text-slate-300 dark:text-slate-600 text-5xl mb-4 font-light">
          ~
        </div>
        <p className="text-sm text-slate-400 dark:text-slate-500">
          使用 Ctrl+O 或点击 + 打开文件
        </p>
      </div>
    </div>
  );
}

export default function EditorPage() {
  const activeTabId = useEditorStore((s) => s.activeTabId);
  const openTabs = useEditorStore((s) => s.openTabs);
  const updateTabContent = useEditorStore((s) => s.updateTabContent);
  const setTabLoading = useEditorStore((s) => s.setTabLoading);
  const markTabClean = useEditorStore((s) => s.markTabClean);

  const currentKB = useKBStore((s) => s.currentKB);

  const [error, setError] = useState("");
  const [msg, setMsg] = useState("");

  const activeTab = openTabs.find((t) => t.id === activeTabId) ?? null;
  const loadInitiatedRef = useRef<Set<string>>(new Set());

  // Load content when tab becomes active and needs loading
  useEffect(() => {
    if (!activeTab || !currentKB) return;
    if (loadInitiatedRef.current.has(activeTab.id)) return;
    if (!activeTab.isLoading) return;

    loadInitiatedRef.current.add(activeTab.id);

    const loadContent = async () => {
      try {
        const { invoke } = await import("@tauri-apps/api/core");

        let content: string;
        if (activeTab.type === "wiki") {
          content = await invoke<string>("get_wiki_page_content", {
            kbPath: currentKB.path,
            pagePath: activeTab.path,
          });
        } else {
          content = await invoke<string>("get_workspace_file_preview", {
            kbId: currentKB.id,
            kbPath: currentKB.path,
            relativePath: activeTab.path,
          });
        }

        updateTabContent(activeTab.id, content);
        markTabClean(activeTab.id);
      } catch (e) {
        console.error("加载文件内容失败:", e);
        updateTabContent(
          activeTab.id,
          `加载失败: ${e instanceof Error ? e.message : "未知错误"}`
        );
      } finally {
        setTabLoading(activeTab.id, false);
      }
    };

    loadContent();
  }, [activeTab, currentKB]);

  // Ctrl+S save handler
  const handleSave = useCallback(async () => {
    if (!activeTab || !currentKB) return;
    if (!activeTab.isDirty) return;

    setError("");
    setMsg("");

    try {
      const { invoke } = await import("@tauri-apps/api/core");

      if (activeTab.type === "wiki") {
        await invoke("save_wiki_page", {
          kbId: currentKB.id,
          kbPath: currentKB.path,
          pageType: "page",
          title: activeTab.title,
          content: activeTab.content,
          pagePath: activeTab.path,
        });
      } else {
        await invoke("save_workspace_file", {
          kbPath: currentKB.path,
          relativePath: activeTab.path,
          content: activeTab.content,
        });
      }
      markTabClean(activeTab.id);
      setMsg("已保存");
    } catch (e) {
      console.error("保存失败:", e);
      setError(`保存失败: ${e instanceof Error ? e.message : String(e)}`);
    }
  }, [activeTab, currentKB]);

  const { triggerSave } = useAutoSave(
    activeTab?.content ?? "",
    activeTab?.isDirty ?? false,
    handleSave
  );

  // Keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.ctrlKey && e.key === "s") {
        e.preventDefault();
        triggerSave();
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [triggerSave]);

  const handleContentChange = useCallback(
    (value: string) => {
      if (!activeTab) return;
      updateTabContent(activeTab.id, value);
    },
    [activeTab, updateTabContent]
  );

  return (
    <div className="flex-1 flex flex-col overflow-hidden -m-6">
      {error && (
        <div className="mx-6 mt-2 px-3 py-2 bg-red-50 dark:bg-red-950 border border-red-200 dark:border-red-800 rounded text-sm text-red-700 dark:text-red-300">
          {error}
        </div>
      )}
      {msg && (
        <div className="mx-6 mt-2 px-3 py-2 bg-green-50 dark:bg-green-950 border border-green-200 dark:border-green-800 rounded text-sm text-green-700 dark:text-green-300">
          {msg}
        </div>
      )}
      {activeTab ? (
        activeTab.isLoading ? (
          <div className="flex items-center justify-center h-full">
            <div className="text-center">
              <div className="w-6 h-6 border-2 border-brand-500 border-t-transparent rounded-full animate-spin mx-auto mb-2" />
              <p className="text-xs text-slate-400 dark:text-slate-500">
                加载中...
              </p>
            </div>
          </div>
        ) : (
          <MarkdownEditor
            content={activeTab.content}
            onChange={handleContentChange}
            readOnly={false}
            fileName={activeTab.title}
          />
        )
      ) : (
        <EmptyState />
      )}
    </div>
  );
}
