import { useEditorStore } from "@/stores/useEditorStore";
import MarkdownRenderer, { parseFrontmatter } from "@/components/common/MarkdownRenderer";
import { Loader2, FileText, ChevronDown, ChevronRight } from "lucide-react";
import { useState } from "react";
import { formatDateTime } from "@/lib/utils";

const PAGE_TYPE_LABELS: Record<string, string> = {
  source: "来源", concept: "概念", entity: "实体", topic: "主题",
  question: "问答", review: "审阅", dataset: "数据集", method: "方法",
};

export default function TabContentRenderer() {
  const openTabs = useEditorStore((s) => s.openTabs);
  const activeTabId = useEditorStore((s) => s.activeTabId);

  const tab = openTabs.find((t) => t.id === activeTabId);
  if (!tab || tab.type === "welcome") return null;

  if (tab.isLoading) {
    return (
      <div className="flex-1 flex items-center justify-center">
        <div className="text-center">
          <Loader2 size={24} className="animate-spin mx-auto mb-2 text-muted-foreground" />
          <p className="text-sm text-muted-foreground">加载中...</p>
        </div>
      </div>
    );
  }

  switch (tab.type) {
    case "wiki": {
      const wikiTitle = tab.title.replace(/\.md$/, "");
      const { frontmatter, body } = tab.content ? parseFrontmatter(tab.content) : { frontmatter: {} as Record<string, any>, body: tab.content || "" };
      return <WikiPageView title={wikiTitle} frontmatter={frontmatter} body={body} />;
    }

    case "file":
    case "pdf_viewer": {
      return (
        <div className="flex-1 overflow-y-auto px-6 py-4">
          <div className="flex items-center gap-2 mb-4 text-sm text-muted-foreground">
            <FileText size={14} />
            <span>{tab.title}</span>
            {tab.page !== undefined && (
              <span className="ml-2 px-1.5 py-0.5 rounded bg-primary/10 text-primary text-xs">
                第 {tab.page} 页
              </span>
            )}
          </div>
          {tab.content ? (
            <MarkdownRenderer content={tab.content} citations={[]} />
          ) : (
            <p className="text-sm text-muted-foreground">
              文件内容将在此处预览。请先通过知识库处理该文件。
            </p>
          )}
        </div>
      );
    }

    case "editor": {
      return (
        <div className="flex-1 overflow-y-auto px-6 py-4">
          {tab.content ? (
            <MarkdownRenderer content={tab.content} citations={[]} />
          ) : (
            <p className="text-sm text-muted-foreground">无内容</p>
          )}
        </div>
      );
    }

    default:
      return (
        <div className="flex-1 flex items-center justify-center text-sm text-muted-foreground">
          暂不支持预览此类型: {tab.type}
        </div>
      );
  }
}

function WikiPageView({ title, frontmatter, body }: { title: string; frontmatter: Record<string, any>; body: string }) {
  const [showMeta, setShowMeta] = useState(false);
  const hasMeta = Object.keys(frontmatter).length > 0;

  return (
    <div className="flex-1 overflow-y-auto px-6 py-4">
      <h1 className="text-xl font-bold mb-2 border-b border-border pb-3">{title}</h1>

      {hasMeta && (
        <div className="mb-4">
          <button
            type="button"
            onClick={() => setShowMeta(!showMeta)}
            className="flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground transition-colors"
          >
            {showMeta ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
            页面信息
            <span className="text-muted-foreground/50">
              ({Object.keys(frontmatter).length} 项)
            </span>
          </button>
          {showMeta && (
            <div className="mt-2 p-3 bg-muted/50 border border-border rounded text-xs space-y-1.5">
              {frontmatter.title && (
                <MetaRow label="标题" value={frontmatter.title} />
              )}
              {frontmatter.type && (
                <MetaRow label="类型" value={<TypeLabel type={frontmatter.type} />} />
              )}
              {frontmatter.canonical_name && (
                <MetaRow label="规范名称" value={frontmatter.canonical_name} mono />
              )}
              {frontmatter.aliases && (
                <MetaRow label="别名" value={
                  <span className="flex flex-wrap gap-1">
                    {(Array.isArray(frontmatter.aliases) ? frontmatter.aliases : [frontmatter.aliases]).map((a: string, i: number) => (
                      <span key={i} className="px-1.5 py-0.5 bg-secondary text-secondary-foreground rounded text-[10px]">{a}</span>
                    ))}
                  </span>
                } />
              )}
              {frontmatter.tags && (
                <MetaRow label="标签" value={
                  <span className="flex flex-wrap gap-1">
                    {(Array.isArray(frontmatter.tags) ? frontmatter.tags : String(frontmatter.tags).split(",")).map((t: string, i: number) => (
                      <span key={i} className="px-1.5 py-0.5 bg-secondary text-secondary-foreground rounded text-[10px]">{t.trim()}</span>
                    ))}
                  </span>
                } />
              )}
              {frontmatter.sources && (
                <MetaRow label="来源" value={frontmatter.sources} />
              )}
              {frontmatter.confidence && (
                <MetaRow label="可信度" value={<ConfidenceLabel level={frontmatter.confidence} />} />
              )}
              {frontmatter.status && (
                <MetaRow label="状态" value={<StatusLabel status={frontmatter.status} />} />
              )}
              {frontmatter.created && (
                <MetaRow label="创建时间" value={formatDateTime(frontmatter.created)} />
              )}
              {frontmatter.updated && (
                <MetaRow label="更新时间" value={formatDateTime(frontmatter.updated)} />
              )}
              {frontmatter.last_updated_by_task && (
                <MetaRow label="更新任务" value={<code className="text-[10px] font-mono">{frontmatter.last_updated_by_task}</code>} />
              )}
            </div>
          )}
        </div>
      )}

      <MarkdownRenderer content={body} citations={[]} />
    </div>
  );
}

function MetaRow({ label, value, mono }: { label: string; value: React.ReactNode; mono?: boolean }) {
  return (
    <div className="flex items-start gap-2">
      <span className="text-muted-foreground shrink-0 min-w-[56px]">{label}</span>
      <span className={`text-foreground break-all ${mono ? "font-mono text-[10px]" : ""}`}>
        {value || "-"}
      </span>
    </div>
  );
}

function TypeLabel({ type }: { type: string }) {
  const colors: Record<string, string> = "concept:bg-blue-100 text-blue-700,entity:bg-green-100 text-green-700,topic:bg-purple-100 text-purple-700,question:bg-amber-100 text-amber-700,source:bg-slate-200 text-slate-700,dataset:bg-cyan-100 text-cyan-700,method:bg-indigo-100 text-indigo-700"
    .split(",").reduce<Record<string, string>>((acc, s) => { const [k, v] = s.split(":"); acc[k] = v; return acc; }, {});
  return <span className={`px-1.5 py-0.5 rounded text-[10px] font-medium ${colors[type] || "bg-slate-100 text-slate-600"}`}>{PAGE_TYPE_LABELS[type] || type}</span>;
}

function ConfidenceLabel({ level }: { level: string }) {
  const colors: Record<string, string> = { high: "bg-green-100 text-green-700", medium: "bg-yellow-100 text-yellow-700", low: "bg-red-100 text-red-700" };
  return <span className={`px-1.5 py-0.5 rounded text-[10px] font-medium ${colors[level] || "bg-slate-100 text-slate-600"}`}>{level}</span>;
}

function StatusLabel({ status }: { status: string }) {
  const colors: Record<string, string> = { active: "bg-green-100 text-green-700", broken: "bg-red-100 text-red-700", pending: "bg-yellow-100 text-yellow-700", draft: "bg-gray-100 text-gray-600" };
  return <span className={`px-1.5 py-0.5 rounded text-[10px] font-medium ${colors[status] || "bg-slate-100 text-slate-600"}`}>{status}</span>;
}
