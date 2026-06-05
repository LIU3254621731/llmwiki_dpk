import { useEffect, useState } from "react";
import WorkspacePage from "@/components/layout/WorkspacePage";
import OnboardingPage from "@/pages/OnboardingPage";
import { useKBStore } from "@/stores/useKBStore";

export default function App() {
  const currentKB = useKBStore((s) => s.currentKB);
  const knowledgeBases = useKBStore((s) => s.knowledgeBases);
  const setKnowledgeBases = useKBStore((s) => s.setKnowledgeBases);
  const setCurrentKB = useKBStore((s) => s.setCurrentKB);
  const [appLoading, setAppLoading] = useState(true);

  const initApp = async (showLoading = true) => {
    if (showLoading) setAppLoading(true);
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const kbs = await invoke<any[]>("list_knowledge_bases");
      setKnowledgeBases(kbs);
      if (kbs.length > 0 && !currentKB) {
        setCurrentKB(kbs[0]);
      }
    } catch (e) {
      console.error("加载知识库列表失败:", e);
    }
    setAppLoading(false);
  };

  useEffect(() => { initApp(); }, []);

  if (appLoading) {
    return (
      <div className="flex items-center justify-center h-screen bg-slate-50 dark:bg-slate-950">
        <div className="text-center">
          <div className="w-8 h-8 border-3 border-brand-500 border-t-transparent rounded-full animate-spin mx-auto mb-3" />
          <p className="text-sm text-slate-400 dark:text-slate-500">加载中...</p>
        </div>
      </div>
    );
  }

  if (knowledgeBases.length === 0) {
    return <OnboardingPage onComplete={(kb) => { setCurrentKB(kb); initApp(false); }} />;
  }

  return <WorkspacePage />;
}
