import { useEffect, useRef, useCallback, useState } from "react";
import { useEditorStore } from "@/stores/useEditorStore";
import { useKBStore } from "@/stores/useKBStore";
import { useAppStore } from "@/stores/useAppStore";
import MarkdownEditor from "@/components/editor/MarkdownEditor";
import { useAutoSave } from "@/hooks/useAutoSave";
import DashboardTab from "@/components/tabs/DashboardTab";
import WikiGraphTab from "@/components/tabs/WikiGraphTab";
import ImportReviewTab from "@/components/tabs/ImportReviewTab";
import SettingsTab from "@/components/tabs/SettingsTab";
import FileExplorerView from "@/components/views/FileExplorerView";
import CanvasView from "@/components/views/CanvasView";
import TaskDetailView from "@/components/views/TaskDetailView";
import {
  BookOpen, FileUp, MessageSquare, GitGraph, Search,
  Loader2, FileText, Bot, Send, Settings,
} from "lucide-react";
import MindMapView from "@/components/graph/MindMapView";
import type { GraphData, GraphNode } from "@/types/graph";

function EmptyState() {
  return (
    <div className="flex items-center justify-center h-full select-none">
      <div className="text-center">
        <div className="text-slate-300 dark:text-slate-600 text-5xl mb-4 font-light">~</div>
        <p className="text-sm text-slate-400 dark:text-slate-500">使用 Ctrl+O 或双击左侧文件打开</p>
      </div>
    </div>
  );
}

function WelcomeDashboard() {
  const currentKB = useKBStore((s) => s.currentKB);
  const stats = useKBStore((s) => s.stats);
  const openFile = useEditorStore((s) => s.openFile);
  const toggleRightSidebar = useAppStore((s) => s.toggleRightSidebar);
  const setRightSidebarMode = useAppStore((s) => s.setRightSidebarMode);

  const quickActions = [
    {
      icon: <Search size={18} />,
      label: "打开文件",
      desc: "Ctrl+O 快速搜索",
      action: () => window.dispatchEvent(new CustomEvent("editor-open-quick-switcher")),
    },
    {
      icon: <FileUp size={18} />,
      label: "上传文件",
      desc: "导入文档到知识库",
      action: () => window.dispatchEvent(new CustomEvent("trigger-file-upload")),
    },
    {
      icon: <MessageSquare size={18} />,
      label: "开始问答",
      desc: "与AI对话分析知识",
      action: () => {
        openFile({ path: "chat-session", title: "智能对话", type: "chat" });
      },
    },
    {
      icon: <GitGraph size={18} />,
      label: "知识图谱",
      desc: "可视化知识网络",
      action: () => {
        openFile({ path: "knowledge-graph", title: "知识图谱", type: "graph", viewMode: "preview" });
      },
    },
    {
      icon: <Settings size={18} />,
      label: "设置",
      desc: "模型、知识库与健康检查",
      action: () => openFile({ path: "settings", title: "设置", type: "settings" }),
    },
  ];

  return (
    <div className="flex-1 overflow-y-auto p-8">
      <div className="max-w-3xl mx-auto">
        <div className="text-center mb-8">
          <BookOpen size={40} className="mx-auto mb-3 text-slate-300 dark:text-slate-600" />
          <h1 className="text-xl font-semibold text-slate-800 dark:text-slate-200 mb-1">
            {currentKB?.name ?? "LLMWiki 智维 Wiki"}
          </h1>
          <p className="text-sm text-slate-500 dark:text-slate-400">
            AI 驱动的知识库管理与双链笔记
          </p>
        </div>

        {/* Stats cards */}
        <div className="grid grid-cols-4 gap-3 mb-8">
          <StatCard label="页面" value={stats?.page_count ?? 0} color="blue" />
          <StatCard label="源文件" value={stats?.source_count ?? 0} color="green" />
          <StatCard label="待审阅" value={stats?.review_count ?? 0} color="amber" />
          <StatCard label="图谱节点" value={stats?.graph_node_count ?? 0} color="purple" />
        </div>

        {/* Quick actions */}
        <div className="grid grid-cols-2 gap-3">
          {quickActions.map((item) => (
            <button
              key={item.label}
              type="button"
              onClick={item.action}
              className="flex items-center gap-3 p-4 rounded-lg border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 hover:border-slate-300 dark:hover:border-slate-600 hover:shadow-sm transition-all text-left"
            >
              <div className="w-10 h-10 rounded-lg bg-slate-100 dark:bg-slate-700 flex items-center justify-center text-slate-500 dark:text-slate-400 shrink-0">
                {item.icon}
              </div>
              <div>
                <div className="text-sm font-medium text-slate-700 dark:text-slate-300">{item.label}</div>
                <div className="text-xs text-slate-400 dark:text-slate-500">{item.desc}</div>
              </div>
            </button>
          ))}
        </div>
      </div>
    </div>
  );
}

function StatCard({ label, value, color }: { label: string; value: number; color: string }) {
  const borderColors: Record<string, string> = {
    blue: "border-l-blue-500",
    green: "border-l-green-500",
    amber: "border-l-amber-500",
    purple: "border-l-purple-500",
  };
  return (
    <div className={`bg-white dark:bg-slate-800 border border-slate-200 dark:border-slate-700 border-l-2 ${borderColors[color] || "border-l-slate-400"} rounded-lg px-4 py-3`}>
      <div className="text-2xl font-semibold text-slate-800 dark:text-slate-200">{value}</div>
      <div className="text-xs text-slate-400 dark:text-slate-500">{label}</div>
    </div>
  );
}

function ChatPanel() {
  const currentKB = useKBStore((s) => s.currentKB);
  const [messages, setMessages] = useState<{ role: string; content: string }[]>([]);
  const [input, setInput] = useState("");
  const [loading, setLoading] = useState(false);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages]);

  const handleSend = async () => {
    if (!input.trim() || !currentKB || loading) return;
    const question = input.trim();
    setInput("");
    setMessages((prev) => [...prev, { role: "user", content: question }]);
    setLoading(true);
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const answer = await invoke<string>("run_query", {
        kbId: currentKB.id,
        question,
        scope: "all",
      });
      setMessages((prev) => [...prev, { role: "assistant", content: answer }]);
    } catch (e) {
      setMessages((prev) => [...prev, { role: "assistant", content: `错误: ${e}` }]);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="flex-1 flex flex-col h-full">
      <div className="flex-1 overflow-y-auto p-4 space-y-4">
        {messages.length === 0 && (
          <div className="flex items-center justify-center h-full">
            <div className="text-center">
              <Bot size={32} className="mx-auto mb-2 text-slate-300 dark:text-slate-600" />
              <p className="text-sm text-slate-400 dark:text-slate-500">向 AI 提问，基于知识库内容获取答案</p>
            </div>
          </div>
        )}
        {messages.map((m, i) => (
          <div key={i} className={`flex ${m.role === "user" ? "justify-end" : "justify-start"}`}>
            <div className={`max-w-[80%] rounded-lg px-4 py-2.5 text-sm ${
              m.role === "user"
                ? "bg-blue-500 text-white"
                : "bg-slate-100 dark:bg-slate-800 text-slate-700 dark:text-slate-300"
            }`}>
              <div className="whitespace-pre-wrap">{m.content}</div>
            </div>
          </div>
        ))}
        {loading && (
          <div className="flex justify-start">
            <div className="bg-slate-100 dark:bg-slate-800 rounded-lg px-4 py-2.5">
              <Loader2 size={16} className="animate-spin text-slate-400" />
            </div>
          </div>
        )}
        <div ref={messagesEndRef} />
      </div>
      <div className="border-t border-slate-200 dark:border-slate-800 p-3 flex items-center gap-2">
        <input
          type="text"
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && handleSend()}
          placeholder="输入问题..."
          className="flex-1 px-3 py-2 text-sm border border-slate-200 dark:border-slate-700 rounded-lg bg-white dark:bg-slate-800 text-slate-700 dark:text-slate-300 outline-none focus:border-slate-400"
        />
        <button
          type="button"
          onClick={handleSend}
          disabled={loading || !input.trim()}
          className="p-2 bg-slate-800 dark:bg-slate-700 text-white rounded-lg hover:bg-slate-700 dark:hover:bg-slate-600 disabled:opacity-50"
          aria-label="发送消息"
        >
          <Send size={16} />
        </button>
      </div>
    </div>
  );
}

function GraphPanel() {
  const currentKB = useKBStore((s) => s.currentKB);
  const openFile = useEditorStore((s) => s.openFile);
  const [graphData, setGraphData] = useState<GraphData | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const requestRef = useRef(0);

  useEffect(() => {
    if (!currentKB) return;
    setLoading(true);
    setError("");
    const requestId = ++requestRef.current;
    (async () => {
      try {
        const { invoke } = await import("@tauri-apps/api/core");
        const data = await invoke<GraphData>("get_graph_data", { kbId: currentKB.id });
        if (requestRef.current !== requestId) return;
        setGraphData(data);
      } catch (e) {
        if (requestRef.current !== requestId) return;
        setError(`加载图谱失败: ${e}`);
      }
      if (requestRef.current !== requestId) return;
      setLoading(false);
    })();
  }, [currentKB?.id]);

  if (loading) {
    return (
      <div className="flex-1 flex items-center justify-center">
        <Loader2 size={24} className="text-slate-400 animate-spin" />
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex-1 flex items-center justify-center">
        <div className="text-center text-sm text-red-500">{error}</div>
      </div>
    );
  }

  if (!graphData || graphData.nodes.length === 0) {
    return (
      <div className="flex-1 flex items-center justify-center p-8">
        <div className="text-center">
          <GitGraph size={48} className="mx-auto mb-3 text-slate-300 dark:text-slate-600" />
          <p className="text-sm text-slate-500 dark:text-slate-400">暂无图谱数据</p>
          <p className="text-xs text-slate-400 dark:text-slate-500 mt-1">
            导入文件后 AI 会自动构建知识图谱
          </p>
        </div>
      </div>
    );
  }

  const handleNodeClick = (node: GraphNode) => {
    if (node.path) {
      openFile({
        path: node.path,
        title: node.label,
        type: "wiki",
      });
    }
  };

  return (
    <div className="flex-1 overflow-hidden">
      <MindMapView
        nodes={graphData.nodes}
        edges={graphData.edges}
        kbName={currentKB?.name ?? "LLMWiki"}
        onNodeClick={handleNodeClick}
      />
    </div>
  );
}

function PdfViewerPanel({ filePath, fileName, kbPath }: { filePath: string; fileName: string; kbPath?: string }) {
  const currentKB = useKBStore((s) => s.currentKB);
  const [content, setContent] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");

  useEffect(() => {
    if (!kbPath || !currentKB) return;
    let cancelled = false;
    setLoading(true);
    setError("");
    (async () => {
      try {
        const { invoke } = await import("@tauri-apps/api/core");
        const result = await invoke<{
          content: string;
          preview_type: string;
          error?: string | null;
        }>("get_workspace_file_preview", {
          kbId: currentKB.id,
          kbPath,
          relativePath: filePath,
        });
        if (cancelled) return;
        if (result.error) {
          setError(result.error);
        } else if (result.content) {
          setContent(result.content);
        } else {
          setError("无法解析文件内容");
        }
      } catch (e) {
        if (!cancelled) setError(`预览加载失败: ${e}`);
      }
      if (!cancelled) setLoading(false);
    })();
    return () => { cancelled = true; };
  }, [filePath, kbPath, currentKB?.id]);

  // Show loading
  if (loading) {
    return (
      <div className="flex-1 flex items-center justify-center">
        <Loader2 size={24} className="animate-spin text-slate-400" />
      </div>
    );
  }

  // Show error or empty state
  if (error || !content) {
    return (
      <div className="flex-1 flex items-center justify-center p-8">
        <div className="text-center max-w-md">
          <FileText size={48} className="mx-auto mb-3 text-red-300 dark:text-red-700" />
          <h3 className="text-sm font-medium text-slate-700 dark:text-slate-300 mb-1">{fileName}</h3>
          {error ? (
            <p className="text-xs text-red-500 mb-3">{error}</p>
          ) : (
            <p className="text-xs text-slate-400 dark:text-slate-500 mb-3">文件内容为空</p>
          )}
          <button
            type="button"
            onClick={async () => {
              try {
                const { invoke } = await import("@tauri-apps/api/core");
                const fullPath = kbPath ? `${kbPath}/${filePath}` : filePath;
                await invoke("shell_open", { path: fullPath });
              } catch { /* ignore */ }
            }}
            className="px-4 py-2 text-xs bg-slate-800 dark:bg-slate-700 text-white rounded-lg hover:bg-slate-700 dark:hover:bg-slate-600"
          >
            在外部程序中打开
          </button>
        </div>
      </div>
    );
  }

  // Show converted content
  return (
    <div className="flex-1 overflow-y-auto p-6">
      <div className="max-w-3xl mx-auto">
        <div className="mb-4 px-3 py-1.5 bg-amber-50 dark:bg-amber-900/20 border border-amber-200 dark:border-amber-800 rounded text-xs text-amber-700 dark:text-amber-400">
          此内容由 PDF 文件自动转换生成，仅用于预览
        </div>
        <MarkdownEditor
          content={content}
          onChange={() => {}}
          readOnly
          fileName={fileName}
        />
      </div>
    </div>
  );
}

export default function CenterArea() {
  const activeTabId = useEditorStore((s) => s.activeTabId);
  const openTabs = useEditorStore((s) => s.openTabs);
  const updateTabContent = useEditorStore((s) => s.updateTabContent);
  const setTabLoading = useEditorStore((s) => s.setTabLoading);
  const markTabClean = useEditorStore((s) => s.markTabClean);

  const currentKB = useKBStore((s) => s.currentKB);

  const [error, setError] = useState("");
  const [msg, setMsg] = useState("");

  // Auto-dismiss success message after 3s
  useEffect(() => {
    if (!msg) return;
    const t = setTimeout(() => setMsg(""), 3000);
    return () => clearTimeout(t);
  }, [msg]);

  const activeTab = openTabs.find((t) => t.id === activeTabId) ?? null;
  const loadInitiatedRef = useRef<Set<string>>(new Set());

  // Clean up loadInitiatedRef for tabs that have been closed while loading
  useEffect(() => {
    const currentTabIds = new Set(openTabs.map((t) => t.id));
    for (const id of loadInitiatedRef.current) {
      if (!currentTabIds.has(id)) {
        loadInitiatedRef.current.delete(id);
      }
    }
  }, [openTabs]);

  // Load content for editor/wiki/file tabs
  useEffect(() => {
    if (!activeTab || !currentKB) return;
    if (loadInitiatedRef.current.has(activeTab.id)) return;
    if (!activeTab.isLoading) return;
    if (activeTab.type !== "editor" && activeTab.type !== "wiki" && activeTab.type !== "file") return;

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

  // Clear load tracking when tab is fully loaded
  useEffect(() => {
    if (activeTab && !activeTab.isLoading) {
      loadInitiatedRef.current.delete(activeTab.id);
    }
  }, [activeTab?.isLoading]);

  const renderTabContent = () => {
    if (!activeTab) return <EmptyState />;

    if (activeTab.isLoading && (activeTab.type === "editor" || activeTab.type === "wiki" || activeTab.type === "file")) {
      return (
        <div className="flex items-center justify-center h-full">
          <div className="text-center">
            <div className="w-6 h-6 border-2 border-brand-500 border-t-transparent rounded-full animate-spin mx-auto mb-2" />
            <p className="text-xs text-slate-400 dark:text-slate-500">加载中...</p>
          </div>
        </div>
      );
    }

    switch (activeTab.type) {
      case "welcome":
        return <WelcomeDashboard />;

      case "chat":
        return <ChatPanel />;

      case "graph":
        return <GraphPanel />;

      case "pdf_viewer":
        return <PdfViewerPanel filePath={activeTab.path} fileName={activeTab.title} kbPath={currentKB?.path} />;

      case "dashboard":
        return <DashboardTab />;

      case "wiki_graph":
        return <WikiGraphTab />;

      case "import_review":
        return <ImportReviewTab />;

      case "settings":
        return <SettingsTab />;

      case "file_explorer":
        return <FileExplorerView />;

      case "canvas":
        return <CanvasView />;

      case "task_detail":
        return <TaskDetailView />;

      case "editor":
      case "wiki":
      case "file":
        return (
          <MarkdownEditor
            content={activeTab.content}
            onChange={handleContentChange}
            readOnly={false}
            fileName={activeTab.title}
          />
        );

      default:
        return <EmptyState />;
    }
  };

  return (
    <div className="flex-1 flex flex-col overflow-hidden bg-white dark:bg-slate-950">
      {error && (
        <div className="mx-4 mt-2 px-3 py-2 bg-red-50 dark:bg-red-950 border border-red-200 dark:border-red-800 rounded text-sm text-red-700 dark:text-red-300">
          {error}
        </div>
      )}
      {msg && (
        <div className="mx-4 mt-2 px-3 py-2 bg-green-50 dark:bg-green-950 border border-green-200 dark:border-green-800 rounded text-sm text-green-700 dark:text-green-300">
          {msg}
        </div>
      )}
      {renderTabContent()}
    </div>
  );
}
