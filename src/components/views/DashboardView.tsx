import { useKBStore } from "@/stores/useKBStore";
import { useEditorStore } from "@/stores/useEditorStore";
import { useAppStore } from "@/stores/useAppStore";
import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  BookOpen, Plus, FileUp, RefreshCw,
  FolderOpen, FileText, Zap, ArrowRight,
} from "lucide-react";
import type { KnowledgeBase } from "@/types/kb";

export default function DashboardView() {
  const currentKB = useKBStore((s) => s.currentKB);
  const knowledgeBases = useKBStore((s) => s.knowledgeBases);
  const setCurrentKB = useKBStore((s) => s.setCurrentKB);
  const setKnowledgeBases = useKBStore((s) => s.setKnowledgeBases);
  const stats = useKBStore((s) => s.stats);
  const openFile = useEditorStore((s) => s.openFile);
  const toggleFileBrowser = useAppStore((s) => s.toggleFileBrowser);

  const [showCreate, setShowCreate] = useState(false);
  const [kbName, setKbName] = useState("");
  const [creating, setCreating] = useState(false);

  const hasSources = (stats?.source_count ?? 0) > 0;
  const hasPages = (stats?.page_count ?? 0) > 0;

  const handleCreateKB = async () => {
    if (!kbName.trim()) return;
    setCreating(true);
    try {
      const defaultBase = await (async () => {
        try {
          const { documentDir } = await import("@tauri-apps/api/path");
          return (await documentDir()) + "LLMWiki知识库";
        } catch {
          return "C:\\Users\\Public\\Documents\\LLMWiki知识库";
        }
      })();
      const newKB = await invoke<KnowledgeBase>("create_knowledge_base", {
        name: kbName.trim(),
        templateName: "general",
        basePath: defaultBase,
      });
      setKnowledgeBases([...knowledgeBases, newKB]);
      setCurrentKB(newKB);
      setShowCreate(false);
      setKbName("");
    } catch (e) {
      console.error("创建知识库失败:", e);
    }
    setCreating(false);
  };

  const handleUpload = () => {
    window.dispatchEvent(new CustomEvent("trigger-file-upload"));
  };

  const handleGoToImportReview = () => {
    openFile({ path: "import-review", title: "导入与审阅", type: "import_review" });
  };

  // --- State 1: No KB selected ---
  if (!currentKB) {
    return (
      <div className="flex-1 overflow-y-auto p-6">
        <div className="max-w-lg mx-auto">
          <div className="empty-state view-enter" style={{ paddingTop: "80px" }}>
            <div className="w-16 h-16 rounded-2xl bg-slate-100 dark:bg-slate-800 flex items-center justify-center mb-5">
              <BookOpen size={32} className="text-slate-300 dark:text-slate-600" />
            </div>
            <h2 className="text-lg font-semibold text-slate-700 dark:text-slate-200 mb-2">
              欢迎使用 LLMWiki
            </h2>
            <p className="text-sm text-slate-400 dark:text-slate-500 mb-8 max-w-sm">
              选择或创建一个知识库来开始。AI 驱动的知识库管理，支持文档导入、智能分析、双链笔记。
            </p>

            {knowledgeBases.length > 0 ? (
              <div className="w-full max-w-sm space-y-2 mb-6">
                <p className="text-xs text-slate-400 text-left mb-2 font-medium uppercase tracking-wide">
                  已有知识库
                </p>
                {knowledgeBases.map((kb) => (
                  <button
                    key={kb.id}
                    type="button"
                    onClick={() => setCurrentKB(kb)}
                    className="card w-full flex items-center gap-3 p-3 text-left hover:border-slate-300 dark:hover:border-slate-600 hover:shadow transition-all cursor-pointer"
                  >
                    <FolderOpen size={18} className="text-slate-400 shrink-0" />
                    <div className="min-w-0">
                      <div className="text-sm font-medium text-slate-700 dark:text-slate-300 truncate">
                        {kb.name}
                      </div>
                      <div className="text-xs text-slate-400 truncate">{kb.path}</div>
                    </div>
                    <ArrowRight size={14} className="text-slate-300 ml-auto shrink-0" />
                  </button>
                ))}
              </div>
            ) : null}

            <button
              type="button"
              onClick={() => setShowCreate(true)}
              className="inline-flex items-center gap-2 px-5 py-2.5 bg-slate-800 dark:bg-slate-200 text-white dark:text-slate-800 rounded-lg text-sm font-medium hover:bg-slate-700 dark:hover:bg-slate-300 transition-colors shadow-sm"
            >
              <Plus size={16} />
              创建知识库
            </button>

            {showCreate && (
              <div
                className="fixed inset-0 bg-black/30 flex items-center justify-center z-50"
                onClick={() => setShowCreate(false)}
              >
                <div
                  className="bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-700 p-6 w-full max-w-md mx-4 shadow-xl rounded-lg"
                  onClick={(e) => e.stopPropagation()}
                >
                  <h3 className="text-base font-semibold text-slate-900 dark:text-slate-200 mb-4">
                    创建知识库
                  </h3>
                  <div className="space-y-3">
                    <div>
                      <label className="text-xs text-slate-500 block mb-1">名称</label>
                      <input
                        value={kbName}
                        onChange={(e) => setKbName(e.target.value)}
                        onKeyDown={(e) => e.key === "Enter" && handleCreateKB()}
                        placeholder="我的知识库"
                        autoFocus
                        className="w-full px-3 py-2 text-sm border border-slate-200 dark:border-slate-700 rounded bg-white dark:bg-slate-800 text-slate-700 dark:text-slate-300 outline-none focus:border-slate-400"
                      />
                    </div>
                    <div className="flex gap-3 pt-2">
                      <button
                        type="button"
                        onClick={() => setShowCreate(false)}
                        className="flex-1 py-2 border border-slate-200 dark:border-slate-700 rounded text-sm text-slate-600 dark:text-slate-400 hover:bg-slate-50 dark:hover:bg-slate-800"
                      >
                        取消
                      </button>
                      <button
                        type="button"
                        onClick={handleCreateKB}
                        disabled={creating || !kbName.trim()}
                        className="flex-1 py-2 bg-slate-800 text-white rounded text-sm hover:bg-slate-700 disabled:opacity-50"
                      >
                        {creating ? "创建中..." : "创建"}
                      </button>
                    </div>
                  </div>
                </div>
              </div>
            )}
          </div>
        </div>
      </div>
    );
  }

  // --- State 2: KB selected, no sources ---
  if (!hasSources) {
    return (
      <div className="flex-1 overflow-y-auto p-6">
        <div className="max-w-lg mx-auto">
          <div className="empty-state view-enter" style={{ paddingTop: "80px" }}>
            <div className="w-16 h-16 rounded-2xl bg-blue-50 dark:bg-blue-900/20 flex items-center justify-center mb-5">
              <FileUp size={32} className="text-blue-400 dark:text-blue-500" />
            </div>
            <h2 className="text-lg font-semibold text-slate-700 dark:text-slate-200 mb-2">
              {currentKB.name}
            </h2>
            <p className="text-sm text-slate-400 dark:text-slate-500 mb-2 max-w-sm">
              上传文档以开始。拖放 PDF、DOCX 或 Markdown 文件。
            </p>
            <p className="text-xs text-slate-400/60 mb-8">
              支持格式: .pdf, .docx, .md, .html, .txt
            </p>
            <div className="flex items-center gap-3">
              <button
                type="button"
                onClick={handleUpload}
                className="inline-flex items-center gap-2 px-5 py-2.5 bg-slate-800 dark:bg-slate-200 text-white dark:text-slate-800 rounded-lg text-sm font-medium hover:bg-slate-700 dark:hover:bg-slate-300 transition-colors shadow-sm"
              >
                <FileUp size={16} />
                上传文档
              </button>
              <button
                type="button"
                onClick={toggleFileBrowser}
                className="inline-flex items-center gap-2 px-5 py-2.5 border border-slate-200 dark:border-slate-700 rounded-lg text-sm text-slate-600 dark:text-slate-400 hover:bg-slate-50 dark:hover:bg-slate-800 transition-colors"
              >
                <FolderOpen size={16} />
                浏览文件
              </button>
            </div>
          </div>
        </div>
      </div>
    );
  }

  // --- State 3: Sources exist, no wiki pages ---
  if (!hasPages) {
    return (
      <div className="flex-1 overflow-y-auto p-6">
        <div className="max-w-lg mx-auto">
          <div className="empty-state view-enter" style={{ paddingTop: "80px" }}>
            <div className="w-16 h-16 rounded-2xl bg-purple-50 dark:bg-purple-900/20 flex items-center justify-center mb-5">
              <Zap size={32} className="text-purple-400 dark:text-purple-500" />
            </div>
            <h2 className="text-lg font-semibold text-slate-700 dark:text-slate-200 mb-2">
              AI 正在分析文档
            </h2>
            <p className="text-sm text-slate-400 dark:text-slate-500 mb-2 max-w-sm">
              AI 正在分析您的文档。前往导入与审阅标签页查看进度。
            </p>
            <p className="text-xs text-slate-400/60 mb-8">
              已导入 {stats?.source_count ?? 0} 个源文件
            </p>
            <button
              type="button"
              onClick={handleGoToImportReview}
              className="inline-flex items-center gap-2 px-5 py-2.5 bg-slate-800 dark:bg-slate-200 text-white dark:text-slate-800 rounded-lg text-sm font-medium hover:bg-slate-700 dark:hover:bg-slate-300 transition-colors shadow-sm"
            >
              <RefreshCw size={16} />
              查看导入与审阅
            </button>
          </div>
        </div>
      </div>
    );
  }

  // --- Normal state: has everything ---
  return (
    <div className="flex-1 overflow-y-auto p-6">
      <div className="max-w-4xl mx-auto">
        <div className="mb-6">
          <h1 className="text-lg font-semibold text-slate-700 dark:text-slate-200 mb-1">
            {currentKB.name}
          </h1>
          <p className="text-xs text-slate-400">
            页面 {stats?.page_count ?? 0} · 源文件 {stats?.source_count ?? 0} · 待审 {stats?.review_count ?? 0}
          </p>
        </div>

        <div className="grid grid-cols-2 gap-4 mb-6">
          <button
            type="button"
            onClick={() => openFile({ path: "chat-session", title: "智能对话", type: "chat" })}
            className="card p-4 text-left hover:border-slate-300 dark:hover:border-slate-600 hover:shadow transition-all"
          >
            <FileText size={20} className="text-slate-400 mb-3" />
            <div className="text-sm font-medium text-slate-700 dark:text-slate-300">浏览页面</div>
            <div className="text-xs text-slate-400 mt-1">查看所有 Wiki 页面</div>
          </button>

          <button
            type="button"
            onClick={handleUpload}
            className="card p-4 text-left hover:border-slate-300 dark:hover:border-slate-600 hover:shadow transition-all"
          >
            <FileUp size={20} className="text-slate-400 mb-3" />
            <div className="text-sm font-medium text-slate-700 dark:text-slate-300">上传文档</div>
            <div className="text-xs text-slate-400 mt-1">导入新的源文件</div>
          </button>
        </div>
      </div>
    </div>
  );
}