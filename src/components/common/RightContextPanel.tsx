import { useState } from "react";
import { X, ExternalLink, FileText, Hash, Shield, AlertTriangle, CheckCircle2, Brain, ArrowRight, ListTree } from "lucide-react";
import { parseFrontmatter } from "./MarkdownRenderer";
import { formatSize, formatDateTime } from "@/lib/utils";
import OutlineView from "@/components/contextpanel/OutlineView";
import BacklinksPanel from "@/components/contextpanel/BacklinksPanel";
import type { BacklinkItem } from "@/components/contextpanel/BacklinksPanel";
import LocalGraphView from "@/components/contextpanel/LocalGraphView";
import ContextPanelSwitcher from "@/components/contextpanel/ContextPanelSwitcher";
import type { ContextPanelMode } from "@/stores/useContextPanelStore";

export type ContextPanelType = "wiki" | "source" | "review" | "graph" | "chat" | "search" | "health" | "file" | "editor" | "outline" | "backlinks" | "local_graph";

interface RightContextPanelProps {
  visible: boolean;
  onClose: () => void;
  context: {
    type: ContextPanelType;
    data: Record<string, any>;
  } | null;
  onAction?: (action: string, payload?: any) => void;
}

export default function RightContextPanel({ visible, onClose, context, onAction }: RightContextPanelProps) {
  const [subMode, setSubMode] = useState<ContextPanelMode>("outline");

  if (!visible || !context) return null;

  const { type, data } = context;

  const renderWikiContext = () => {
    const frontmatter = data.content ? parseFrontmatter(data.content).frontmatter : {};
    return (
      <div className="space-y-4">
        <ContextPanelSwitcher activeMode={subMode} onModeChange={setSubMode} />
        {subMode === "outline" && (
          <OutlineView content={data.content || ""} onHeadingClick={(id) => onAction?.("heading_click", id)} />
        )}
        {subMode === "backlinks" && (
          <BacklinksPanel
            backlinks={(Array.isArray(data.backlinks) ? data.backlinks : []) as BacklinkItem[]}
            onNavigate={(path) => onAction?.("navigate", path)}
          />
        )}
        {subMode === "local_graph" && (
          <LocalGraphView
            nodePath={data.path || ""}
            nodeTitle={data.title || data.canonical_name || ""}
            kbId={data.kb_id || ""}
            onNavigate={(path) => onAction?.("navigate", path)}
          />
        )}

        <Section title="页面信息">
          <KV label="标题" value={data.title || data.canonical_name} />
          <KV label="类型" value={<TypeBadge type={data.page_type || frontmatter.page_type} />} />
          <KV label="规范名称" value={data.canonical_name || frontmatter.canonical_name} />
          <KV label="路径" value={data.path} mono />
          {data.status && <KV label="状态" value={<StatusBadge status={data.status} />} />}
          {data.confidence && <KV label="可信度" value={<ConfidenceBadge level={data.confidence} />} />}
          <KV label="创建时间" value={formatDateTime(data.created_at)} />
          <KV label="更新时间" value={formatDateTime(data.updated_at)} />
        </Section>

        {(frontmatter.aliases || frontmatter.tags || frontmatter.sources) && (
          <Section title="元数据">
            {frontmatter.aliases && (
              <KV label="别名" value={
                <div className="flex flex-wrap gap-1">
                  {(Array.isArray(frontmatter.aliases) ? frontmatter.aliases : [frontmatter.aliases]).map((a: string, i: number) => (
                    <span key={i} className="text-xs bg-slate-100 text-slate-600 px-1.5 py-0.5 rounded">{a}</span>
                  ))}
                </div>
              } />
            )}
            {frontmatter.tags && (
              <KV label="标签" value={
                <div className="flex flex-wrap gap-1">
                  {(Array.isArray(frontmatter.tags) ? frontmatter.tags : String(frontmatter.tags).split(",")).map((t: string, i: number) => (
                    <span key={i} className="text-xs bg-slate-100 text-slate-600 px-1.5 py-0.5 rounded">{t.trim()}</span>
                  ))}
                </div>
              } />
            )}
            {frontmatter.sources && <KV label="来源" value={frontmatter.sources} />}
          </Section>
        )}

        {data.related_pages && data.related_pages.length > 0 && (
          <Section title="关联页面">
            {data.related_pages.map((p: any, i: number) => (
              <button key={i} type="button" onClick={() => onAction?.("open_page", p)} className="flex items-center gap-2 w-full text-left py-1.5 px-2 rounded text-sm text-slate-600 hover:bg-slate-100">
                <FileText size={12} />
                <span className="truncate">{p.title || p.label}</span>
                <ExternalLink size={10} className="ml-auto shrink-0" />
              </button>
            ))}
          </Section>
        )}

        {data.backlinks && data.backlinks.length > 0 && (
          <Section title="反向链接">
            {data.backlinks.map((bl: any, i: number) => (
              <div key={i} className="text-xs text-slate-500 py-1 px-2">{bl}</div>
            ))}
          </Section>
        )}

        <Section title="AI 建议操作">
          <ActionBtn icon={<Brain size={12} />} label="总结当前页面" onClick={() => onAction?.("summarize")} />
          <ActionBtn icon={<AlertTriangle size={12} />} label="查找缺失引用" onClick={() => onAction?.("find_missing")} />
          <ActionBtn icon={<ListTree size={12} />} label="生成思维导图" onClick={() => onAction?.("mind_map")} />
          <ActionBtn icon={<Shield size={12} />} label="运行局部健康检查" onClick={() => onAction?.("health_check")} />
        </Section>
      </div>
    );
  };

  const renderSourceContext = () => (
    <div className="space-y-4">
      <Section title="文件信息">
        <KV label="文件名" value={data.file_name} />
        <KV label="类型" value={<span className="px-1.5 py-0.5 bg-slate-100 rounded text-xs">{data.file_type}</span>} />
        <KV label="大小" value={formatSize(data.file_size)} />
        <KV label="Hash" value={data.file_hash?.slice(0, 12) + "..."} mono />
        <KV label="状态" value={<SourceStatusBadge status={data.status} />} />
        <KV label="路径" value={data.relative_path || data.file_path} mono />
      </Section>

      {data.ai_summary && (
        <Section title="AI 摘要">
          <div className="text-xs text-slate-600 max-h-48 overflow-y-auto">{data.ai_summary}</div>
        </Section>
      )}

      {data.coverage_report && (
        <Section title="覆盖度报告">
          <div className="text-xs text-slate-600 max-h-48 overflow-y-auto">{data.coverage_report}</div>
        </Section>
      )}

      {data.entity_count !== undefined && (
        <Section title="抽取统计">
          <KV label="实体" value={data.entity_count} />
          <KV label="概念" value={data.concept_count} />
          <KV label="关系" value={data.relation_count} />
        </Section>
      )}

      {data.linked_wiki_pages && data.linked_wiki_pages.length > 0 && (
        <Section title="关联 Wiki 页面">
          {data.linked_wiki_pages.map((p: any, i: number) => (
            <button key={i} type="button" onClick={() => onAction?.("open_page", p)} className="flex items-center gap-2 w-full text-left py-1.5 px-2 rounded text-sm text-slate-600 hover:bg-slate-100">
              <FileText size={12} /><span className="truncate">{p.title}</span>
            </button>
          ))}
        </Section>
      )}

      <Section title="操作">
        <ActionBtn icon={<ExternalLink size={12} />} label="打开原始文件" onClick={() => onAction?.("open_file")} />
        <ActionBtn icon={<FileText size={12} />} label="查看 Source Preview" onClick={() => onAction?.("preview")} />
        <ActionBtn icon={<Brain size={12} />} label="重新分析" onClick={() => onAction?.("reanalyze")} />
        <ActionBtn icon={<Hash size={12} />} label="复制 source_id" onClick={() => onAction?.("copy_id")} />
      </Section>
    </div>
  );

  const renderGraphContext = () => (
    <div className="space-y-4">
      {data.node ? (
        <>
          <Section title="节点信息">
            <KV label="标题" value={data.node.label || data.node.title} />
            <KV label="类型" value={<TypeBadge type={data.node.node_type || data.node.type} />} />
            {data.node.path && <KV label="路径" value={data.node.path} mono />}
            {data.node.summary && (
              <div className="text-xs text-slate-600 max-h-36 overflow-y-auto mt-1 bg-slate-50 p-2 rounded">{data.node.summary}</div>
            )}
          </Section>

          {data.node.aliases && data.node.aliases.length > 0 && (
            <Section title="别名">
              <div className="flex flex-wrap gap-1">
                {data.node.aliases.map((a: string, i: number) => (
                  <span key={i} className="text-xs bg-slate-100 px-1.5 py-0.5 rounded">{a}</span>
                ))}
              </div>
            </Section>
          )}

          <Section title="统计">
            <KV label="来源数" value={data.node.sourceCount || data.node.source_count || 0} />
            <KV label="入度" value={data.node.inDegree || data.node.in_degree || 0} />
            <KV label="出度" value={data.node.outDegree || data.node.out_degree || 0} />
            <KV label="状态" value={<StatusBadge status={data.node.status || "active"} />} />
          </Section>

          <Section title="操作">
            {data.node.path && <ActionBtn icon={<FileText size={12} />} label="打开 Wiki 页面" onClick={() => onAction?.("open_page", { path: data.node.path, title: data.node.label })} />}
            <ActionBtn icon={<Brain size={12} />} label="在 Chat 中询问" onClick={() => onAction?.("ask_chat", data.node.label)} />
            <ActionBtn icon={<ArrowRight size={12} />} label="设为中心" onClick={() => onAction?.("set_center", data.node.id)} />
            <ActionBtn icon={<ListTree size={12} />} label="展开一层" onClick={() => onAction?.("expand", data.node.id)} />
          </Section>
        </>
      ) : data.edge ? (
        <>
          <Section title="关系信息">
            <KV label="关系类型" value={<RelationBadge relation={data.edge.relation || data.edge.type} />} />
            <KV label="置信度" value={<ConfidenceBadge level={data.edge.confidence || "medium"} />} />
            {data.edge.evidence_source_id && <KV label="证据来源" value={data.edge.evidence_source_id} mono />}
            <KV label="引用状态" value={data.edge.citationStatus || data.edge.citation_status || "uncited"} />
          </Section>
          <Section title="操作">
            <ActionBtn icon={<ExternalLink size={12} />} label="查看证据" onClick={() => onAction?.("view_evidence")} />
            <ActionBtn icon={<CheckCircle2 size={12} />} label="确认关系" onClick={() => onAction?.("confirm_relation")} />
            <ActionBtn icon={<X size={12} />} label="删除关系" onClick={() => onAction?.("delete_relation")} />
          </Section>
        </>
      ) : (
        <div className="text-xs text-slate-400 py-4 text-center">选择节点或关系查看详情</div>
      )}
    </div>
  );

  const renderChatContext = () => (
    <div className="space-y-4">
      <Section title="检索范围">
        <div className="text-xs text-slate-600">{data.scope || "整个知识库"}</div>
      </Section>
      {data.referenced_pages && data.referenced_pages.length > 0 && (
        <Section title="引用页面">
          {data.referenced_pages.map((p: any, i: number) => (
            <button key={i} type="button" onClick={() => onAction?.("open_page", p)} className="flex items-center gap-2 w-full text-left py-1.5 px-2 rounded text-sm text-slate-600 hover:bg-slate-100">
              <FileText size={12} /><span className="truncate">{p.title}</span>
            </button>
          ))}
        </Section>
      )}
      <Section title="操作">
        <ActionBtn icon={<FileText size={12} />} label="保存为 Wiki 页面" onClick={() => onAction?.("save_as_wiki")} />
      </Section>
    </div>
  );

  const renderSearchContext = () => (
    <div className="space-y-4">
      <Section title="搜索结果">
        <KV label="标题" value={data.title} />
        <KV label="类型" value={<TypeBadge type={data.page_type || data.type} />} />
        {data.path && <KV label="路径" value={data.path} mono />}
        {data.matched_field && <KV label="匹配字段" value={data.matched_field} />}
      </Section>
      {data.match_snippet && (
        <Section title="匹配片段">
          <div className="text-xs text-slate-600 bg-slate-50 p-2 rounded max-h-48 overflow-y-auto" dangerouslySetInnerHTML={{ __html: data.match_snippet.replace(/<script\b[^<]*(?:(?!<\/script>)<[^<]*)*<\/script>/gi, "").replace(/\s+on\w+\s*=\s*(?:"[^"]*"|'[^']*'|[^\s>]+)/gi, "") }} />
        </Section>
      )}
      <Section title="操作">
        <ActionBtn icon={<FileText size={12} />} label="打开页面" onClick={() => onAction?.("open_page", data)} />
        <ActionBtn icon={<Brain size={12} />} label="在 Chat 中询问" onClick={() => onAction?.("ask_chat", data.title)} />
      </Section>
    </div>
  );

  const renderFileContext = () => (
    <div className="space-y-4">
      <Section title="文件详情">
        <KV label="文件名" value={data.file_name} />
        <KV label="类型" value={data.file_type || "-"} />
        <KV label="大小" value={formatSize(data.file_size)} />
        {data.file_hash && <KV label="Hash" value={data.file_hash.slice(0, 16) + "..."} mono />}
        <KV label="相对路径" value={data.relative_path} mono />
        {data.record_type && <KV label="记录类型" value={data.record_type === "wiki_page" ? "Wiki 页面" : data.record_type === "source" ? "源文件" : data.record_type} />}
        {data.status && data.status !== "ok" && <KV label="状态" value={<span className="text-amber-600 text-xs">{data.status}</span>} />}
      </Section>

      <Section title="时间">
        <KV label="创建时间" value={formatDateTime(data.created_at)} />
        <KV label="修改时间" value={formatDateTime(data.modified_at)} />
      </Section>

      {data.linked_wiki_pages && data.linked_wiki_pages.length > 0 && (
        <Section title="关联页面">
          {data.linked_wiki_pages.map((p: any, i: number) => (
            <button key={i} type="button" onClick={() => onAction?.("open_page", p)} className="flex items-center gap-2 w-full text-left py-1.5 px-2 rounded text-sm text-slate-600 hover:bg-slate-100">
              <FileText size={12} /><span className="truncate">{p.title}</span>
            </button>
          ))}
        </Section>
      )}

      {data.linked_tasks && data.linked_tasks.length > 0 && (
        <Section title="关联任务">
          {data.linked_tasks.map((t: any, i: number) => (
            <div key={i} className="text-xs text-slate-500 py-1 px-2">{t.task_type} - {t.status}</div>
          ))}
        </Section>
      )}

      <Section title="操作">
        <ActionBtn icon={<ExternalLink size={12} />} label="打开原始文件" onClick={() => onAction?.("open_file")} />
        <ActionBtn icon={<FileText size={12} />} label="打开所在目录" onClick={() => onAction?.("open_folder")} />
        <ActionBtn icon={<Hash size={12} />} label="复制相对路径" onClick={() => onAction?.("copy_path")} />
      </Section>
    </div>
  );

  const renderHealthContext = () => (
    <div className="space-y-4">
      <Section title="问题详情">
        <KV label="描述" value={data.description || data.issue} />
        <KV label="严重程度" value={<SeverityBadge level={data.severity || "medium"} />} />
        {data.impact && <KV label="影响范围" value={data.impact} />}
      </Section>
      {data.recommendation && (
        <Section title="推荐修复">
          <div className="text-xs text-slate-600">{data.recommendation}</div>
        </Section>
      )}
      <Section title="操作">
        <ActionBtn icon={<Shield size={12} />} label="一键修复" onClick={() => onAction?.("fix")} />
        <ActionBtn icon={<ExternalLink size={12} />} label="查看详情" onClick={() => onAction?.("view_detail")} />
        <ActionBtn icon={<X size={12} />} label="忽略" onClick={() => onAction?.("ignore")} />
      </Section>
    </div>
  );

  const renderEditorContext = () => (
    <div className="space-y-4">
      <ContextPanelSwitcher activeMode={subMode} onModeChange={setSubMode} />
      {subMode === "outline" && (
        <OutlineView content={data.content || ""} onHeadingClick={(id) => onAction?.("heading_click", id)} />
      )}
      {subMode === "backlinks" && (
        <BacklinksPanel
          backlinks={(Array.isArray(data.backlinks) ? data.backlinks : []) as BacklinkItem[]}
          onNavigate={(path) => onAction?.("navigate", path)}
        />
      )}
      {subMode === "local_graph" && (
        <LocalGraphView
          nodePath={data.path || ""}
          nodeTitle={data.title || ""}
          kbId={data.kb_id || ""}
          onNavigate={(path) => onAction?.("navigate", path)}
        />
      )}
    </div>
  );

  const renderOutlineContext = () => (
    <OutlineView
      content={data.content || ""}
      activeHeadingId={data.activeHeadingId}
      onHeadingClick={(headingId) => onAction?.("heading_click", headingId)}
    />
  );

  const renderBacklinksContext = () => (
    <BacklinksPanel
      backlinks={(data.backlinks || []) as BacklinkItem[]}
      onNavigate={(path) => onAction?.("navigate", path)}
    />
  );

  const renderLocalGraphContext = () => (
    <LocalGraphView
      nodePath={data.nodePath || ""}
      nodeTitle={data.nodeTitle || data.title || ""}
      kbId={data.kbId || ""}
      depth={data.depth ?? 1}
      onNavigate={(path) => onAction?.("navigate", path)}
    />
  );

  return (
    <div className="w-72 shrink-0 bg-white border-l border-slate-200 h-full overflow-y-auto">
      <div className="flex items-center justify-between px-3 py-2 border-b border-slate-100 bg-slate-50 sticky top-0">
        <span className="text-xs font-medium text-slate-500 uppercase">
          {type === "wiki" ? "页面信息" : type === "editor" ? "编辑器" : type === "source" ? "文件信息" : type === "graph" ? "图谱详情" : type === "chat" ? "对话上下文" : type === "search" ? "搜索结果" : type === "file" ? "文件详情" : type === "outline" ? "大纲" : type === "backlinks" ? "反向链接" : type === "local_graph" ? "局部图谱" : "详情"}
        </span>
        <button type="button" onClick={onClose} className="p-0.5 hover:bg-slate-200 rounded text-slate-400" title="关闭面板">
          <X size={14} />
        </button>
      </div>
      <div className="p-3">
        {type === "wiki" && renderWikiContext()}
        {type === "editor" && renderEditorContext()}
        {type === "source" && renderSourceContext()}
        {type === "graph" && renderGraphContext()}
        {type === "chat" && renderChatContext()}
        {type === "search" && renderSearchContext()}
        {type === "file" && renderFileContext()}
        {type === "review" && (
          <div className="text-xs text-slate-600">
            <Section title="审阅信息">
              <KV label="摘要" value={data.summary || data.reason} />
              <KV label="风险" value={<RiskBadge level={data.risk_level || "medium"} />} />
            </Section>
          </div>
        )}
        {type === "health" && renderHealthContext()}
        {type === "outline" && renderOutlineContext()}
        {type === "backlinks" && renderBacklinksContext()}
        {type === "local_graph" && renderLocalGraphContext()}
      </div>
    </div>
  );
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div>
      <h4 className="text-[11px] font-semibold text-slate-400 uppercase mb-2">{title}</h4>
      <div className="space-y-1.5">{children}</div>
    </div>
  );
}

function KV({ label, value, mono }: { label: string; value: any; mono?: boolean }) {
  return (
    <div className="flex items-start gap-2 text-xs">
      <span className="text-slate-400 shrink-0 min-w-[48px]">{label}</span>
      <span className={`text-slate-700 break-all ${mono ? "font-mono text-[10px]" : ""}`}>
        {value || "-"}
      </span>
    </div>
  );
}

function ActionBtn({ icon, label, onClick }: { icon: React.ReactNode; label: string; onClick?: () => void }) {
  return (
    <button type="button" onClick={onClick} className="flex items-center gap-2 w-full text-left px-2 py-1.5 rounded text-xs text-slate-600 hover:bg-slate-100 hover:text-slate-700 transition-colors">
      {icon}
      {label}
    </button>
  );
}

function TypeBadge({ type }: { type: string }) {
  const colors: Record<string, string> = {
    concept: "bg-blue-50 text-blue-600", entity: "bg-green-50 text-green-600",
    topic: "bg-purple-50 text-purple-600", question: "bg-amber-50 text-amber-600",
    source: "bg-slate-100 text-slate-600", dataset: "bg-cyan-50 text-cyan-600",
    method: "bg-indigo-50 text-indigo-600", review: "bg-pink-50 text-pink-600",
    wikipage: "bg-slate-50 text-slate-600",
  };
  return <span className={`px-1.5 py-0.5 rounded text-[10px] font-medium ${colors[type] || "bg-slate-50 text-slate-600"}`}>{type}</span>;
}

function StatusBadge({ status }: { status: string }) {
  const colors: Record<string, string> = {
    active: "bg-green-50 text-green-600", broken: "bg-red-50 text-red-600",
    pending: "bg-yellow-50 text-yellow-600", draft: "bg-gray-50 text-gray-600",
  };
  return <span className={`px-1.5 py-0.5 rounded text-[10px] font-medium ${colors[status] || "bg-slate-50 text-slate-600"}`}>{status}</span>;
}

function SourceStatusBadge({ status }: { status: string }) {
  const labels: Record<string, string> = {
    pending: "待处理", analyzing: "分析中", analyzed: "已分析",
    analysis_failed: "分析失败", processed: "流水线完成", pipeline_failed: "流水线失败",
    applied: "已应用", duplicate: "重复(已跳过)",
    failed: "失败", cancelled: "已取消", skipped: "已跳过",
  };
  const colors: Record<string, string> = {
    pending: "bg-yellow-50 text-yellow-600", analyzing: "bg-blue-50 text-blue-600",
    analyzed: "bg-green-50 text-green-600", processed: "bg-green-50 text-green-600",
    applied: "bg-green-50 text-green-700",
    analysis_failed: "bg-red-50 text-red-600", pipeline_failed: "bg-red-50 text-red-600",
    duplicate: "bg-slate-200 text-slate-500",
    failed: "bg-red-50 text-red-600", cancelled: "bg-amber-50 text-amber-600", skipped: "bg-slate-100 text-slate-500",
  };
  return <span className={`px-1.5 py-0.5 rounded text-[10px] font-medium ${colors[status] || "bg-slate-50"}`}>{labels[status] || status}</span>;
}

function ConfidenceBadge({ level }: { level: string }) {
  const colors: Record<string, string> = {
    high: "bg-green-50 text-green-600", medium: "bg-yellow-50 text-yellow-600",
    low: "bg-red-50 text-red-600", model_reported: "bg-blue-50 text-blue-600",
    user_verified: "bg-teal-50 text-teal-600",
  };
  return <span className={`px-1.5 py-0.5 rounded text-[10px] font-medium ${colors[level] || "bg-slate-50"}`}>{level}</span>;
}

function RelationBadge({ relation }: { relation: string }) {
  return <span className="px-1.5 py-0.5 rounded text-[10px] font-medium bg-slate-100 text-slate-600">{relation}</span>;
}

function SeverityBadge({ level }: { level: string }) {
  const colors: Record<string, string> = {
    critical: "bg-red-100 text-red-700", high: "bg-orange-100 text-orange-700",
    medium: "bg-yellow-100 text-yellow-700", low: "bg-blue-100 text-blue-700",
  };
  return <span className={`px-1.5 py-0.5 rounded text-[10px] font-medium ${colors[level] || "bg-slate-100"}`}>{level}</span>;
}

function RiskBadge({ level }: { level: string }) {
  const colors: Record<string, string> = {
    low: "bg-green-50 text-green-600", medium: "bg-yellow-50 text-yellow-600", high: "bg-red-50 text-red-600",
  };
  return <span className={`px-1.5 py-0.5 rounded text-[10px] font-medium ${colors[level] || "bg-slate-50"}`}>{level}</span>;
}


