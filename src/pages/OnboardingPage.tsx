import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { ArrowRight, BookOpen, FolderOpen } from "lucide-react";
import type { KnowledgeBase } from "@/types/kb";

interface Props {
  onComplete: (kb: KnowledgeBase) => void;
}

export default function OnboardingPage({ onComplete }: Props) {
  const [step, setStep] = useState(1);
  const [name, setName] = useState("我的知识库");
  const [basePath, setBasePath] = useState("");
  const [msg, setMsg] = useState("");
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);

  const handlePickFolder = async () => {
    try {
      const selected = await open({ directory: true, multiple: false, title: "选择知识库存储目录" });
      if (selected && typeof selected === "string") {
        setBasePath(selected);
      }
    } catch (e) {
      console.error("[OnboardingPage] 选择目录失败:", e);
    }
  };

  const handleInit = async () => {
    setLoading(true);
    setMsg("");
    setError("");
    try {
      const defaultBase = await (async () => {
        try {
          const { join, documentDir } = await import("@tauri-apps/api/path");
          return await join(await documentDir(), "LLMWiki知识库");
        } catch {
          return "C:\\Users\\Public\\Documents\\LLMWiki知识库";
        }
      })();

      const kbPath = basePath || defaultBase;

      const newKB = await invoke<KnowledgeBase>("create_knowledge_base", {
        name,
        templateName: "general",
        basePath: kbPath,
      });

      setMsg("知识库创建成功！");
      setTimeout(() => onComplete(newKB), 500);
    } catch (e) {
      setError(`创建失败: ${e}`);
    }
    setLoading(false);
  };

  return (
    <div className="min-h-screen flex items-center justify-center bg-gradient-to-br from-slate-50 to-blue-50 dark:from-slate-950 dark:to-slate-900">
      <div className="w-full max-w-lg mx-4">
        {/* Logo and title */}
        <div className="text-center mb-8">
          <BookOpen size={48} className="mx-auto mb-4 text-brand-600" />
          <h1 className="text-2xl font-bold text-slate-800 dark:text-slate-100">LLMWiki 知识库</h1>
          <p className="text-slate-500 dark:text-slate-400 mt-2">LLM 驱动的知识维护工作台</p>
        </div>

        <div className="bg-white dark:bg-slate-800 rounded-xl border border-slate-200 dark:border-slate-700 p-8 shadow-sm">
          {/* Step indicators */}
          <div className="flex items-center justify-center gap-2 mb-6">
            {[1, 2, 3].map((s) => (
              <div key={s} className={`w-8 h-8 rounded-full flex items-center justify-center text-sm font-medium ${
                step >= s ? "bg-brand-600 text-white" : "bg-slate-100 text-slate-400 dark:bg-slate-700 dark:text-slate-500"
              }`}>
                {s}
              </div>
            ))}
          </div>

          {step === 1 && (
            <div className="space-y-4">
              <h2 className="text-lg font-semibold text-slate-800 dark:text-slate-100">欢迎</h2>
              <p className="text-sm text-slate-600 dark:text-slate-300">
                LLMWiki 知识库 是一个本地运行的 AI 知识库维护工作台。<br />
                上传文档 → AI 分析 → 审阅确认 → Wiki 沉淀。
              </p>
              <ul className="text-xs text-slate-500 dark:text-slate-400 space-y-1">
                <li>✅ 本地运行，数据安全</li>
                <li>✅ DeepSeek 驱动知识抽取</li>
                <li>✅ Markdown Wiki 长期资产</li>
                <li>✅ 可审阅、可追踪、可回滚</li>
              </ul>
              <button onClick={() => setStep(2)} className="w-full py-2.5 bg-brand-600 text-white rounded-lg hover:bg-brand-700 flex items-center justify-center gap-2 mt-4">
                下一步 <ArrowRight size={16} />
              </button>
            </div>
          )}

          {step === 2 && (
            <div className="space-y-4">
              <h2 className="text-lg font-semibold text-slate-800 dark:text-slate-100">创建知识库</h2>
              <div>
                <label className="text-xs text-slate-500 dark:text-slate-400 block mb-1">知识库名称</label>
                <input value={name} onChange={(e) => setName(e.target.value)} className="w-full px-3 py-2 text-sm border border-slate-200 dark:border-slate-600 dark:bg-slate-900 dark:text-slate-200 rounded-lg outline-none focus:border-brand-400" />
              </div>
              <div>
                <label className="text-xs text-slate-500 dark:text-slate-400 block mb-1">存储目录（留空使用默认）</label>
                <div className="flex gap-2">
                  <input value={basePath} onChange={(e) => setBasePath(e.target.value)} placeholder="默认: 文档/LLMWiki知识库" className="flex-1 px-3 py-2 text-sm border border-slate-200 dark:border-slate-600 dark:bg-slate-900 dark:text-slate-200 rounded-lg outline-none focus:border-brand-400" />
                  <button type="button" onClick={handlePickFolder} title="选择文件夹" className="px-3 py-2 text-sm border border-slate-200 dark:border-slate-600 rounded-lg hover:bg-slate-50 dark:hover:bg-slate-700 text-slate-600 dark:text-slate-300 flex items-center gap-1">
                    <FolderOpen size={16} />
                  </button>
                </div>
              </div>
              <div className="flex gap-3">
                <button onClick={() => setStep(1)} className="flex-1 py-2.5 border border-slate-200 dark:border-slate-600 rounded-lg text-slate-600 dark:text-slate-300 hover:bg-slate-50 dark:hover:bg-slate-700">上一步</button>
                <button onClick={() => setStep(3)} className="flex-1 py-2.5 bg-brand-600 text-white rounded-lg hover:bg-brand-700 flex items-center justify-center gap-2">
                  下一步 <ArrowRight size={16} />
                </button>
              </div>
            </div>
          )}

          {step === 3 && (
            <div className="space-y-4">
              <h2 className="text-lg font-semibold text-slate-800 dark:text-slate-100">配置确认</h2>
              <div className="bg-slate-50 dark:bg-slate-900 rounded-lg p-4 space-y-2 text-sm">
                <div><span className="text-slate-500 dark:text-slate-400">名称:</span> {name}</div>
                <div className="text-xs text-slate-400 dark:text-slate-500 mt-2">
                  ✅ 知识库将保存在本地<br />
                  ✅ DeepSeek API Key 可稍后在设置中配置<br />
                  ✅ 所有数据保存在本地
                </div>
              </div>
              {error && <div className="px-4 py-2 rounded text-sm bg-red-50 dark:bg-red-950 text-red-700 dark:text-red-300">{error}</div>}
              {msg && <div className="px-4 py-2 rounded text-sm bg-green-50 dark:bg-green-950 text-green-700 dark:text-green-300">{msg}</div>}
              <div className="flex gap-3">
                <button onClick={() => setStep(2)} className="flex-1 py-2.5 border border-slate-200 dark:border-slate-600 rounded-lg text-slate-600 dark:text-slate-300 hover:bg-slate-50 dark:hover:bg-slate-700">上一步</button>
                <button onClick={handleInit} disabled={loading} className="flex-1 py-2.5 bg-brand-600 text-white rounded-lg hover:bg-brand-700 disabled:opacity-50 flex items-center justify-center gap-2">
                  {loading ? "创建中..." : "完成创建"} <ArrowRight size={16} />
                </button>
              </div>
            </div>
          )}
        </div>

        <p className="text-center text-xs text-slate-400 dark:text-slate-500 mt-6">
          DeepSeek API Key 可在设置中配置 | 所有数据保存在本地
        </p>
      </div>
    </div>
  );
}
