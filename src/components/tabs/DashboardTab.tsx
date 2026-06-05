import { useEffect, useRef, useState } from "react";
import { useKBStore } from "@/stores/useKBStore";
import { useEditorStore } from "@/stores/useEditorStore";
import { useAppStore } from "@/stores/useAppStore";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  BookOpen, GitPullRequestDraft, GitGraph, Plus, Trash2,
  AlertTriangle, CheckCircle, RefreshCw, XCircle,
  FileUp, MessageSquare, Search, Settings,
} from "lucide-react";
import type { KnowledgeBase } from "@/types/kb";
import { formatDate, formatDateTime } from "@/lib/utils";

export default function DashboardTab() {
  const currentKB = useKBStore((s) => s.currentKB);
  const knowledgeBases = useKBStore((s) => s.knowledgeBases);
  const setKnowledgeBases = useKBStore((s) => s.setKnowledgeBases);
  const setCurrentKB = useKBStore((s) => s.setCurrentKB);
  const stats = useKBStore((s) => s.stats);
  const setStats = useKBStore((s) => s.setStats);
  const openFile = useEditorStore((s) => s.openFile);
  const toggleFileBrowser = useAppStore((s) => s.toggleFileBrowser);

  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [showCreateDialog, setShowCreateDialog] = useState(false);

  useEffect(() => { loadKBs(); }, []);
  useEffect(() => { if (currentKB) loadStats(); }, [currentKB]);

  const lastStatsRefresh = useRef(0);
  const statsCleanupRef = useRef<(() => void)[]>([]);
  useEffect(() => {
    let cancelled = false;

    // Clean up stale listeners
    statsCleanupRef.current.forEach((fn) => fn());
    statsCleanupRef.current = [];

    listen<any>("kb-stats-changed", (event) => {
      if (!cancelled && currentKB && event.payload.kb_id === currentKB.id) {
        const now = Date.now();
        if (now - lastStatsRefresh.current > 500) {
          lastStatsRefresh.current = now;
          loadStats();
        }
      }
    }).then((fn) => {
      if (cancelled) { fn(); }
      else { statsCleanupRef.current = [fn]; }
    }).catch(() => {});

    return () => {
      cancelled = true;
      statsCleanupRef.current.forEach((fn) => fn());
      statsCleanupRef.current = [];
    };
  }, [currentKB]);

  const loadKBs = async () => {
    try {
      const kbs = await invoke<KnowledgeBase[]>("list_knowledge_bases");
      setKnowledgeBases(kbs);
      if (kbs.length > 0 && !currentKB) setCurrentKB(kbs[0]);
    } catch (e) { setError(`加载知识库列表失败: ${e}`); }
  };

  const loadStats = async () => {
    if (!currentKB) return;
    setLoading(true);
    try {
      const s = await invoke<any>("get_kb_stats", { kbId: currentKB.id });
      setStats(s);
      setError("");
    } catch (e) { setError(`加载统计数据失败: ${e}`); }
    setLoading(false);
  };

  const handleDeleteKB = async (kb: KnowledgeBase) => {
    if (!confirm(`确定要删除知识库 "${kb.name}" 吗？\n此操作不可恢复！`)) return;
    try {
      await invoke("delete_knowledge_base", { kbId: kb.id });
      await loadKBs();
      if (currentKB?.id === kb.id) setCurrentKB(null);
    } catch (e) { setError(`删除知识库失败: ${e}`); }
  };

  const healthStatus = stats?.health_status || "healthy";
  const healthLabel: Record<string, string> = { critical: "有严重问题", warning: "需关注", review: "待审阅", graph_unsynced: "图谱未同步", healthy: "健康" };

  return (
    <div className="flex-1 overflow-y-auto p-8">
      <div className="max-w-4xl mx-auto">
        {error && (
          <div className="mb-4 px-4 py-2.5 text-sm text-red-600 bg-red-50 border border-red-100 flex items-center justify-between">
            <span>{error}</span>
            <button type="button" onClick={() => { setError(""); loadKBs(); }} className="text-xs text-red-500 hover:text-red-700 ml-3 shrink-0">重试</button>
          </div>
        )}

        {loading ? (
          <div className="flex items-center justify-center py-16 text-slate-400">
            <RefreshCw size={18} className="mr-2 animate-spin" />
            <span className="text-sm">加载统计数据...</span>
          </div>
        ) : (
          <>
            {/* Header */}
            <div className="flex items-center justify-between mb-6">
              <div>
                <h1 className="text-lg font-semibold text-slate-900 dark:text-slate-200">{currentKB?.name}</h1>
                <p className="text-xs text-slate-400 mt-0.5">
                  路径: {currentKB?.path}
                  <span className="mx-1.5 text-slate-300">|</span>
                  创建于 {currentKB ? formatDateTime(currentKB.created_at) : "-"}
                </p>
              </div>
              <div className="flex items-center gap-2">
                <button type="button" onClick={() => setShowCreateDialog(true)} className="flex items-center gap-1.5 px-3 py-1.5 border border-slate-200 dark:border-slate-700 text-slate-600 dark:text-slate-400 text-xs rounded hover:bg-slate-50 dark:hover:bg-slate-800">
                  <Plus size={14} /> 新建知识库
                </button>
                <button type="button" onClick={loadStats} className="p-1.5 text-slate-400 hover:text-slate-600 rounded" title="刷新"><RefreshCw size={14} /></button>
              </div>
            </div>

            {/* Stats bar */}
            <div className="flex flex-wrap items-center gap-x-6 gap-y-2 mb-6 pb-4 border-b border-slate-100 dark:border-slate-800">
              <StatItem label="Wiki 页面" value={stats?.page_count ?? 0} />
              <StatItem label="Source 文件" value={stats?.source_count ?? 0} />
              <button type="button" onClick={() => openFile({ path: "import-review", title: "导入与审阅", type: "import_review" })} className="text-sm text-slate-500 dark:text-slate-400 hover:text-slate-700 dark:hover:text-slate-300">
                待审阅 <span className={`font-medium ${(stats?.review_count ?? 0) > 0 ? "text-amber-600" : "text-slate-500"}`}>{stats?.review_count ?? 0}</span>
              </button>
              <StatItem label="知识关系" value={stats?.relationship_count ?? 0} />
              {(stats?.broken_page_count ?? 0) > 0 && (
                <button type="button" onClick={() => openFile({ path: "wiki-graph", title: "Wiki & 图谱", type: "wiki_graph" })} className="text-sm text-slate-500 dark:text-slate-400 hover:text-slate-700 dark:hover:text-slate-300">
                  破损页面 <span className="text-red-500 font-medium">{stats?.broken_page_count ?? 0}</span>
                </button>
              )}
              <button type="button" onClick={() => openFile({ path: "settings", title: "设置", type: "settings" })} className="text-sm text-slate-500 dark:text-slate-400 hover:text-slate-700 dark:hover:text-slate-300 inline-flex items-center gap-1">
                {healthStatus === "healthy" ? <CheckCircle size={13} className="text-emerald-500" /> : healthStatus === "critical" ? <XCircle size={13} className="text-red-500" /> : <AlertTriangle size={13} className="text-amber-500" />}
                {healthLabel[healthStatus] || "需关注"}
              </button>
            </div>

            {/* Quick actions */}
            <div className="mb-6 pb-4 border-b border-slate-100 dark:border-slate-800">
              <h2 className="text-xs font-medium text-slate-400 uppercase tracking-wide mb-3">快捷操作</h2>
              <div className="flex flex-wrap gap-2">
                <QuickBtn icon={<FileUp size={13} />} label="上传文档" onClick={() => window.dispatchEvent(new CustomEvent("trigger-file-upload"))} />
                <QuickBtn icon={<BookOpen size={13} />} label="Wiki & 图谱" onClick={() => openFile({ path: "wiki-graph", title: "Wiki & 图谱", type: "wiki_graph" })} />
                <QuickBtn icon={<MessageSquare size={13} />} label="开始问答" onClick={() => openFile({ path: "chat-session", title: "智能对话", type: "chat" })} />
                <QuickBtn icon={<GitPullRequestDraft size={13} />} label="导入与审阅" onClick={() => openFile({ path: "import-review", title: "导入与审阅", type: "import_review" })} badge={(stats?.review_count ?? 0) > 0 ? String(stats!.review_count) : undefined} />
                <QuickBtn icon={<Search size={13} />} label="文件浏览" onClick={toggleFileBrowser} />
                <QuickBtn icon={<Settings size={13} />} label="设置" onClick={() => openFile({ path: "settings", title: "设置", type: "settings" })} />
              </div>
            </div>

            {/* KB list */}
            <div className="mb-6">
              <h2 className="text-xs font-medium text-slate-400 uppercase tracking-wide mb-2">所有知识库</h2>
              {knowledgeBases.map((kb) => (
                <div key={kb.id} className="flex items-center justify-between py-2.5 border-b border-slate-100 dark:border-slate-800">
                  <div className="min-w-0 flex-1">
                    <div className="flex items-center gap-2">
                      <button type="button" onClick={() => setCurrentKB(kb)} className="text-sm text-slate-700 dark:text-slate-300 hover:text-slate-900 dark:hover:text-slate-100 text-left truncate">{kb.name}</button>
                      {kb.id === currentKB?.id && <span className="text-[10px] text-slate-400 bg-slate-100 dark:bg-slate-800 px-1.5 py-0.5">当前</span>}
                    </div>
                    <div className="text-xs text-slate-400 mt-0.5">{kb.path}<span className="mx-1 text-slate-300">|</span>{formatDate(kb.created_at)}</div>
                  </div>
                  <div className="flex items-center gap-1 shrink-0 ml-4">
                    <button type="button" onClick={() => { setCurrentKB(kb); }} className="px-2 py-1 text-xs text-slate-500 hover:text-slate-700 hover:bg-slate-50 dark:hover:bg-slate-800 rounded">切换</button>
                    <button type="button" onClick={(e) => { e.stopPropagation(); handleDeleteKB(kb); }} className="p-1 text-slate-300 hover:text-red-500 rounded" title="删除"><Trash2 size={13} /></button>
                  </div>
                </div>
              ))}
            </div>

            {/* Recommendations */}
            {((stats?.review_count ?? 0) > 0 || (stats?.broken_page_count ?? 0) > 0 || (stats?.failed_task_count ?? 0) > 0 || healthStatus !== "healthy") && (
              <div>
                <h2 className="text-xs font-medium text-slate-400 uppercase tracking-wide mb-3">推荐下一步</h2>
                <div className="space-y-1.5">
                  {(stats?.review_count ?? 0) > 0 && (
                    <RecommendRow text={`有 ${stats!.review_count} 个待审阅修改`} hint="建议前往审阅中心处理" btnText="去审阅" onClick={() => openFile({ path: "import-review", title: "导入与审阅", type: "import_review" })} />
                  )}
                  {(stats?.broken_page_count ?? 0) > 0 && (
                    <RecommendRow text={`检测到 ${stats!.broken_page_count} 个破损页面`} hint="建议前往健康检查修复" btnText="去修复" onClick={() => openFile({ path: "settings", title: "设置", type: "settings" })} />
                  )}
                  {(stats?.failed_task_count ?? 0) > 0 && (
                    <RecommendRow text={`有 ${stats!.failed_task_count} 个失败任务`} hint="建议查看并重试" btnText="去查看" onClick={() => openFile({ path: "import-review", title: "导入与审阅", type: "import_review" })} />
                  )}
                  {healthStatus !== "healthy" && (stats?.review_count ?? 0) === 0 && (
                    <RecommendRow text="检测到数据一致性问题" hint="建议运行健康检查" btnText="去检查" onClick={() => openFile({ path: "settings", title: "设置", type: "settings" })} />
                  )}
                </div>
              </div>
            )}
          </>
        )}

        <CreateKBDialog open={showCreateDialog} onClose={() => setShowCreateDialog(false)} onCreated={(kb) => { setCurrentKB(kb); loadKBs(); setShowCreateDialog(false); }} />
      </div>
    </div>
  );
}

function StatItem({ label, value }: { label: string; value: number }) {
  return <span className="text-sm text-slate-500 dark:text-slate-400">{label} <span className="text-slate-700 dark:text-slate-300 font-medium">{value}</span></span>;
}

function QuickBtn({ icon, label, onClick, badge }: { icon: React.ReactNode; label: string; onClick: () => void; badge?: string }) {
  return (
    <button type="button" onClick={onClick} className="inline-flex items-center gap-1.5 px-3 py-1.5 border border-slate-200 dark:border-slate-700 text-slate-600 dark:text-slate-400 text-xs rounded hover:bg-slate-50 dark:hover:bg-slate-800">
      {icon} {label}
      {badge && <span className="text-amber-600 font-medium">{badge}</span>}
    </button>
  );
}

function RecommendRow({ text, hint, btnText, onClick }: { text: string; hint: string; btnText: string; onClick: () => void }) {
  return (
    <div className="flex items-center justify-between py-2.5 border-b border-slate-100 dark:border-slate-800">
      <div className="text-sm text-slate-600 dark:text-slate-400">{text}<span className="text-xs text-slate-400 ml-2">{hint}</span></div>
      <button type="button" onClick={onClick} className="px-3 py-1 text-xs border border-slate-200 dark:border-slate-700 text-slate-600 dark:text-slate-400 rounded hover:bg-slate-50 dark:hover:bg-slate-800 shrink-0 ml-3">{btnText}</button>
    </div>
  );
}

function CreateKBDialog({ open, onClose, onCreated }: { open: boolean; onClose: () => void; onCreated: (kb: KnowledgeBase) => void }) {
  const [name, setName] = useState("我的知识库");
  const [basePath, setBasePath] = useState("");
  const [msg, setMsg] = useState("");
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);

  if (!open) return null;

  const handleCreate = async () => {
    setLoading(true); setMsg(""); setError("");
    try {
      const defaultBase = await (async () => {
        try { const { documentDir } = await import("@tauri-apps/api/path"); return await documentDir() + "LLMWiki知识库"; }
        catch { return "C:\\Users\\Public\\Documents\\LLMWiki知识库"; }
      })();
      const newKB = await invoke<KnowledgeBase>("create_knowledge_base", { name, templateName: "general", basePath: basePath || defaultBase });
      setMsg("知识库创建成功！");
      setTimeout(() => onCreated(newKB), 500);
    } catch (e) { setError(`创建失败: ${e}`); }
    setLoading(false);
  };

  const handleSelectFolder = async () => {
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const selected = await open({ directory: true, multiple: false });
      if (selected) setBasePath(selected as string);
    } catch { /* dialog not available */ }
  };

  return (
    <div className="fixed inset-0 bg-black/30 flex items-center justify-center z-50" onClick={onClose}>
      <div className="bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-700 p-6 w-full max-w-md mx-4 shadow-xl" onClick={(e) => e.stopPropagation()}>
        <h2 className="text-base font-semibold text-slate-900 dark:text-slate-200 mb-4">创建知识库</h2>
        <div className="space-y-3">
          <div>
            <label className="text-xs text-slate-500 block mb-1">名称</label>
            <input value={name} onChange={(e) => setName(e.target.value)} className="w-full px-3 py-2 text-sm border border-slate-200 dark:border-slate-700 rounded bg-white dark:bg-slate-800 text-slate-700 dark:text-slate-300 outline-none focus:border-slate-400" />
          </div>
          <div>
            <label className="text-xs text-slate-500 block mb-1">存储目录（留空使用默认）</label>
            <div className="flex gap-2">
              <input value={basePath} onChange={(e) => setBasePath(e.target.value)} placeholder="默认: 文档/LLMWiki知识库" className="flex-1 px-3 py-2 text-sm border border-slate-200 dark:border-slate-700 rounded bg-white dark:bg-slate-800 text-slate-700 dark:text-slate-300 outline-none focus:border-slate-400" />
              <button type="button" onClick={handleSelectFolder} className="px-3 py-2 text-xs border border-slate-200 dark:border-slate-700 text-slate-600 dark:text-slate-400 rounded hover:bg-slate-50 dark:hover:bg-slate-800">浏览...</button>
            </div>
          </div>
          {error && <div className="px-3 py-2 text-xs text-red-600 bg-red-50 border border-red-100">{error}</div>}
          {msg && <div className="px-3 py-2 text-xs text-emerald-600 bg-emerald-50 border border-emerald-100">{msg}</div>}
          <div className="flex gap-3 pt-2">
            <button type="button" onClick={onClose} className="flex-1 py-2 border border-slate-200 dark:border-slate-700 rounded text-sm text-slate-600 dark:text-slate-400 hover:bg-slate-50 dark:hover:bg-slate-800">取消</button>
            <button type="button" onClick={handleCreate} disabled={loading} className="flex-1 py-2 bg-slate-800 text-white rounded text-sm hover:bg-slate-700 disabled:opacity-50">{loading ? "创建中..." : "创建"}</button>
          </div>
        </div>
      </div>
    </div>
  );
}
