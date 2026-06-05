import { useEffect, useState, useRef } from "react";
import { useKBStore } from "@/stores/useKBStore";
import { useEditorStore } from "@/stores/useEditorStore";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  BookOpen, Plus, Trash2, RefreshCw, Search, X,
  Loader2, FileText, GitGraph, ExternalLink,
} from "lucide-react";
import type { WikiPage } from "@/types/wiki";
import type { GraphData, GraphNode } from "@/types/graph";
import { formatDateTime } from "@/lib/utils";
import MindMapView from "@/components/graph/MindMapView";

const PAGE_TYPE_LABELS: Record<string, string> = {
  source: "来源", concept: "概念", entity: "实体", topic: "主题",
  question: "问答", review: "审阅", dataset: "数据集", method: "方法",
};

export default function WikiGraphTab() {
  const currentKB = useKBStore((s) => s.currentKB);
  const stats = useKBStore((s) => s.stats);
  const setStats = useKBStore((s) => s.setStats);
  const openFile = useEditorStore((s) => s.openFile);

  // Wiki state
  const [pages, setPages] = useState<WikiPage[]>([]);
  const [wikiLoading, setWikiLoading] = useState(true);
  const [wikiSearch, setWikiSearch] = useState("");
  const [msg, setMsg] = useState("");
  const [error, setError] = useState("");

  // Graph state
  const [graphData, setGraphData] = useState<GraphData | null>(null);
  const [graphLoading, setGraphLoading] = useState(true);

  // View mode: "list" | "graph" | "split"
  const [viewMode, setViewMode] = useState<"list" | "graph" | "split">("split");

  useEffect(() => {
    if (!currentKB) return;
    loadPages();
    loadGraph();
    loadStats();
  }, [currentKB]);

  const lastWikiRefresh = useRef(0);
  useEffect(() => {
    const unlisten = listen<any>("wiki-updated", (event) => {
      if (currentKB && event.payload.kb_id === currentKB.id) {
        const now = Date.now();
        if (now - lastWikiRefresh.current > 2000) {
          lastWikiRefresh.current = now;
          loadPages();
        }
      }
    });
    return () => { unlisten.then((fn) => fn()); };
  }, [currentKB]);

  const loadStats = async () => {
    if (!currentKB) return;
    try {
      const s = await invoke<any>("get_kb_stats", { kbId: currentKB.id });
      setStats(s);
    } catch { /* ignore */ }
  };

  const loadPages = async () => {
    if (!currentKB) return;
    setWikiLoading(true);
    try {
      const list = await invoke<WikiPage[]>("list_wiki_pages", { kbId: currentKB.id });
      setPages(list);
    } catch (e) { setError(`加载Wiki页面失败: ${e}`); }
    setWikiLoading(false);
  };

  const loadGraph = async () => {
    if (!currentKB) return;
    setGraphLoading(true);
    try {
      const data = await invoke<GraphData>("get_graph_data", { kbId: currentKB.id });
      setGraphData(data);
    } catch (e) { console.error("加载图谱失败:", e); }
    setGraphLoading(false);
  };

  const handleSelectPage = (page: WikiPage) => {
    openFile({
      path: page.path,
      title: page.title,
      type: "wiki",
    });
  };

  const handleDeletePage = async (page: WikiPage) => {
    if (!currentKB || !confirm(`确定要删除 "${page.title}" 吗？`)) return;
    try {
      await invoke("delete_wiki_page", { kbId: currentKB.id, pagePath: page.path });
      setMsg("页面已删除"); loadPages();
    } catch (e) { setError(`删除失败: ${e}`); }
  };

  const handleGraphNodeClick = (node: GraphNode) => {
    if (node.path) {
      openFile({
        path: node.path,
        title: node.label,
        type: "wiki",
      });
    }
  };

  const filteredPages = wikiSearch
    ? pages.filter(p => p.title.toLowerCase().includes(wikiSearch.toLowerCase()) || p.path.toLowerCase().includes(wikiSearch.toLowerCase()))
    : pages;

  // Group pages by type
  const grouped = filteredPages.reduce<Record<string, WikiPage[]>>((acc, p) => {
    const t = p.page_type || "other";
    if (!acc[t]) acc[t] = [];
    acc[t].push(p);
    return acc;
  }, {});

  return (
    <div className="flex-1 flex flex-col overflow-hidden">
      {error && <div className="mx-4 mt-2 px-3 py-2 bg-red-50 border border-red-200 rounded text-sm text-red-700 flex items-center justify-between"><span>{error}</span><button onClick={() => setError("")}><X size={14} /></button></div>}
      {msg && <div className="mx-4 mt-2 px-3 py-2 bg-green-50 border border-green-200 rounded text-sm text-green-700 flex items-center justify-between"><span>{msg}</span><button onClick={() => setMsg("")}><X size={14} /></button></div>}

      {/* View mode bar */}
      <div className="flex items-center justify-between px-4 py-2 border-b border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 shrink-0">
        <div className="flex items-center gap-1 text-xs">
          <button onClick={() => setViewMode("list")} className={`px-3 py-1 rounded ${viewMode === "list" ? "bg-slate-200 dark:bg-slate-700 text-slate-800 dark:text-slate-200 font-medium" : "text-slate-500 hover:bg-slate-100 dark:hover:bg-slate-800"}`}>Wiki 列表</button>
          <button onClick={() => setViewMode("graph")} className={`px-3 py-1 rounded ${viewMode === "graph" ? "bg-slate-200 dark:bg-slate-700 text-slate-800 dark:text-slate-200 font-medium" : "text-slate-500 hover:bg-slate-100 dark:hover:bg-slate-800"}`}>知识图谱</button>
          <button onClick={() => setViewMode("split")} className={`px-3 py-1 rounded ${viewMode === "split" ? "bg-slate-200 dark:bg-slate-700 text-slate-800 dark:text-slate-200 font-medium" : "text-slate-500 hover:bg-slate-100 dark:hover:bg-slate-800"}`}>分屏</button>
        </div>
        <div className="flex items-center gap-2 text-xs text-slate-400">
          <span>{(stats?.page_count ?? 0)} 页面</span>
          <span>·</span>
          <span>{(stats?.graph_node_count ?? 0)} 节点</span>
          <span>·</span>
          <span>{(stats?.relationship_count ?? 0)} 关系</span>
        </div>
      </div>

      {/* Content */}
      <div className="flex-1 flex overflow-hidden">
        {/* Wiki list panel */}
        {(viewMode === "list" || viewMode === "split") && (
          <div className={`${viewMode === "split" ? "w-[320px] border-r border-slate-200 dark:border-slate-800" : "flex-1"} flex flex-col bg-white dark:bg-slate-900 shrink-0 overflow-hidden`}>
            <div className="px-3 py-2 border-b border-slate-100 dark:border-slate-800">
              <div className="relative">
                <Search size={13} className="absolute left-2 top-1/2 -translate-y-1/2 text-slate-400" />
                <input type="text" value={wikiSearch} onChange={e => setWikiSearch(e.target.value)} placeholder="搜索页面..." className="w-full pl-7 pr-3 py-1.5 text-xs rounded border border-slate-200 dark:border-slate-700 bg-slate-50 dark:bg-slate-800 outline-none focus:border-slate-400" />
              </div>
            </div>
            <div className="flex-1 overflow-y-auto">
              {wikiLoading ? (
                <div className="flex items-center justify-center py-12"><Loader2 size={18} className="animate-spin text-slate-400" /></div>
              ) : filteredPages.length === 0 ? (
                <div className="text-xs text-slate-400 text-center py-8">{wikiSearch ? "无匹配页面" : "暂无 Wiki 页面"}</div>
              ) : (
                <div className="py-1">
                  {Object.entries(grouped).map(([type, typePages]) => (
                    <div key={type}>
                      <div className="px-3 py-1.5 text-[10px] font-medium text-slate-400 uppercase">{PAGE_TYPE_LABELS[type] || type}</div>
                      {typePages.map(p => (
                        <button
                          key={p.id}
                          onClick={() => handleSelectPage(p)}
                          className="w-full text-left px-3 py-2 text-xs text-slate-700 dark:text-slate-300 hover:bg-slate-50 dark:hover:bg-slate-800 flex items-center justify-between"
                        >
                          <span className="truncate flex-1">{p.title}</span>
                          <button onClick={(e) => { e.stopPropagation(); handleDeletePage(p); }} className="ml-1 p-0.5 text-slate-300 hover:text-red-500 opacity-0 group-hover:opacity-100 rounded"><Trash2 size={10} /></button>
                        </button>
                      ))}
                    </div>
                  ))}
                </div>
              )}
            </div>
          </div>
        )}

        {/* Wiki content area OR Graph */}
        {(viewMode === "list" || viewMode === "split") ? (
          <div className="flex-1 overflow-y-auto bg-slate-50 dark:bg-slate-950">
            {viewMode === "list" ? null : (
              <div className="flex items-center justify-center h-full">
                <div className="text-center">
                  <ExternalLink size={40} className="mx-auto mb-3 text-slate-300 dark:text-slate-600" />
                  <p className="text-sm text-slate-400">点击左侧页面在新标签页中打开</p>
                </div>
              </div>
            )}
          </div>
        ) : (
          /* Graph only mode */
          <div className="flex-1 overflow-hidden">
            {graphLoading ? (
              <div className="flex items-center justify-center h-full"><Loader2 size={24} className="animate-spin text-slate-400" /></div>
            ) : graphData && graphData.nodes.length > 0 ? (
              <MindMapView nodes={graphData.nodes} edges={graphData.edges} kbName={currentKB?.name ?? "LLMWiki"} onNodeClick={handleGraphNodeClick} />
            ) : (
              <div className="flex items-center justify-center h-full">
                <div className="text-center">
                  <GitGraph size={48} className="mx-auto mb-3 text-slate-300 dark:text-slate-600" />
                  <p className="text-sm text-slate-400">暂无图谱数据</p>
                  <p className="text-xs text-slate-400 mt-1">导入文件后 AI 会自动构建知识图谱</p>
                </div>
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
