import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  Loader2, Wrench, RefreshCw, AlertTriangle,
  CheckCircle, XCircle, Info,
} from "lucide-react";
import type { KnowledgeBase } from "@/types/kb";

interface SystemHealthTabProps {
  currentKB: KnowledgeBase | null;
}

interface HealthCheckItem {
  category: string;
  severity: string;
  name: string;
  description: string;
  suggestion: string;
  fix_action: string;
  detail: any;
}

interface HealthCheckResult {
  timestamp: string;
  overall_status: string;
  summary: {
    page_count: number;
    source_count: number;
    review_count: number;
    knowledge_item_count: number;
    unlinked_ki_count: number;
    graph_node_count: number;
    graph_edge_count: number;
    critical_count: number;
    warning_count: number;
  };
  items: HealthCheckItem[];
  report_md: string;
}

const SEVERITY_DOT: Record<string, string> = {
  critical: "bg-destructive",
  warning: "bg-warning",
  info: "bg-info",
  ok: "bg-success",
};

const SEVERITY_LABEL: Record<string, string> = {
  critical: "严重",
  warning: "警告",
  info: "提示",
  ok: "通过",
};

const SEVERITY_ICON: Record<string, React.ReactNode> = {
  critical: <XCircle size={14} className="text-destructive" />,
  warning: <AlertTriangle size={14} className="text-warning" />,
  info: <Info size={14} className="text-info" />,
  ok: <CheckCircle size={14} className="text-success" />,
};

export function SystemHealthTab({ currentKB }: SystemHealthTabProps) {
  const [structuredResult, setStructuredResult] = useState<HealthCheckResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [msg, setMsg] = useState("");
  const [error, setError] = useState("");
  const hasLoaded = useRef(false);

  // Lazy load: only run health check when first activated
  useEffect(() => {
    if (currentKB && !hasLoaded.current) {
      hasLoaded.current = true;
      runHealthCheck();
    }
  }, [currentKB]);

  const runHealthCheck = async () => {
    if (!currentKB) return;
    setLoading(true);
    setMsg("");
    setError("");
    try {
      const s = await invoke<HealthCheckResult>("run_health_check_structured", {
        kbId: currentKB.id,
        kbPath: currentKB.path,
      });
      setStructuredResult(s);
    } catch (e) {
      setError(`健康检查失败: ${e}`);
    }
    setLoading(false);
  };

  const handleRepairAll = async () => {
    if (!currentKB) return;
    setLoading(true);
    setMsg("");
    setError("");
    try {
      const result: any = await invoke("repair_all_wiki_paths", {
        kbId: currentKB.id,
        kbPath: currentKB.path,
      });
      setMsg(`已修复 ${result.fixed ?? 0} 个问题，仍需人工处理 ${result.remaining_manual ?? 0} 个`);
      runHealthCheck();
    } catch (e) {
      setError(`批量修复失败: ${e}`);
      setLoading(false);
    }
  };

  const handleSyncGraph = async () => {
    if (!currentKB) return;
    setLoading(true);
    try {
      const result: any = await invoke("sync_graph_data", { kbId: currentKB.id });
      setMsg(`图谱已重建，推导出 ${result?.relationships_created ?? 0} 条关系`);
      setLoading(false);
      runHealthCheck();
    } catch (e) {
      setError(`图谱重建失败: ${e}`);
      setLoading(false);
    }
  };

  const handleSyncWikiIndex = async () => {
    if (!currentKB) return;
    setLoading(true);
    try {
      const result: any = await invoke("sync_wiki_index_from_markdown", {
        kbId: currentKB.id,
        kbPath: currentKB.path,
      });
      setMsg(`已重建 Wiki 索引: 新增 ${result.created ?? 0}，更新 ${result.updated ?? 0}`);
    } catch (e) {
      setError(`Wiki 索引重建失败: ${e}`);
    }
    setLoading(false);
  };

  const handleRebuildPreviews = async () => {
    if (!currentKB) return;
    setLoading(true);
    setMsg("");
    try {
      const result: any = await invoke("rebuild_all_previews", {
        kbId: currentKB.id,
        kbPath: currentKB.path,
      });
      setMsg(`已重建 ${result.rebuilt ?? 0} 个预览`);
    } catch (e) {
      setError(`预览重建失败: ${e}`);
    }
    setLoading(false);
  };

  if (!currentKB) {
    return (
      <div className="flex items-center justify-center h-64 text-muted-foreground text-sm">
        请先创建或选择一个知识库
      </div>
    );
  }

  return (
    <div className="h-full overflow-y-auto">
      <div className="max-w-4xl mx-auto p-8 space-y-6">
        {/* Actions toolbar */}
        <div className="flex items-center gap-2 flex-wrap">
          <button
            type="button"
            onClick={runHealthCheck}
            disabled={loading}
            className="flex items-center gap-1.5 px-3 py-1.5 bg-primary text-primary-foreground text-xs rounded hover:bg-primary-hover disabled:opacity-50 transition-colors"
          >
            {loading ? <Loader2 size={13} className="animate-spin" /> : <RefreshCw size={13} />}
            运行健康检查
          </button>
          <button
            type="button"
            onClick={handleRepairAll}
            disabled={loading}
            className="flex items-center gap-1.5 px-3 py-1.5 border border-border text-foreground-dim text-xs rounded hover:bg-card-hover transition-colors"
          >
            <Wrench size={13} /> 一键修复
          </button>
          <button
            type="button"
            onClick={handleSyncGraph}
            disabled={loading}
            className="px-3 py-1.5 border border-border text-foreground-dim text-xs rounded hover:bg-card-hover transition-colors"
          >
            同步图谱
          </button>
          <button
            type="button"
            onClick={handleSyncWikiIndex}
            disabled={loading}
            className="px-3 py-1.5 border border-border text-foreground-dim text-xs rounded hover:bg-card-hover transition-colors"
          >
            重建索引
          </button>
          <button
            type="button"
            onClick={handleRebuildPreviews}
            disabled={loading}
            className="px-3 py-1.5 border border-border text-foreground-dim text-xs rounded hover:bg-card-hover transition-colors"
          >
            重建预览
          </button>
        </div>

        {error && (
          <div className="px-4 py-2.5 text-sm text-destructive bg-destructive-subtle border border-destructive/20 rounded">{error}</div>
        )}
        {msg && (
          <div className="px-4 py-2.5 text-sm text-success bg-success-subtle border border-success/20 rounded">{msg}</div>
        )}

        {structuredResult && (
          <>
            {/* Overall status */}
            <div className="flex items-center gap-3">
              <span className={`text-sm font-medium ${
                structuredResult.overall_status === "critical" ? "text-destructive" :
                structuredResult.overall_status === "warning" ? "text-warning" :
                "text-success"
              }`}>
                {structuredResult.overall_status === "critical" ? "严重问题" :
                 structuredResult.overall_status === "warning" ? "需处理" : "状态良好"}
              </span>
              <span className="text-xs text-muted-foreground">
                检查时间: {new Date(structuredResult.timestamp).toLocaleString()}
              </span>
            </div>

            {/* Stats bar */}
            <div className="flex items-center gap-6 text-sm flex-wrap">
              <div>
                <span className="text-foreground font-semibold">{structuredResult.summary.page_count}</span>
                <span className="text-muted-foreground ml-1">Wiki 页面</span>
              </div>
              <div>
                <span className="text-foreground font-semibold">{structuredResult.summary.source_count}</span>
                <span className="text-muted-foreground ml-1">Source 文件</span>
              </div>
              <div>
                <span className="text-foreground font-semibold">{structuredResult.summary.graph_node_count}</span>
                <span className="text-muted-foreground ml-1">图谱节点</span>
              </div>
              <div>
                <span className="text-foreground font-semibold">{structuredResult.summary.graph_edge_count}</span>
                <span className="text-muted-foreground ml-1">图谱边</span>
              </div>
              {structuredResult.summary.critical_count > 0 && (
                <span className="text-destructive font-medium">{structuredResult.summary.critical_count} 严重</span>
              )}
              {structuredResult.summary.warning_count > 0 && (
                <span className="text-warning font-medium">{structuredResult.summary.warning_count} 警告</span>
              )}
            </div>

            {/* Issue list */}
            {structuredResult.items.length > 0 ? (
              <div className="space-y-2">
                <h3 className="text-xs font-medium text-muted-foreground uppercase tracking-wide">
                  发现 {structuredResult.items.length} 个问题
                </h3>
                {structuredResult.items
                  .sort((a, b) => {
                    const order: Record<string, number> = { critical: 0, warning: 1, info: 2, ok: 3 };
                    return (order[a.severity] ?? 3) - (order[b.severity] ?? 3);
                  })
                  .map((item, i) => (
                    <div
                      key={i}
                      className="bg-card border border-border rounded-lg p-4"
                    >
                      <div className="flex items-start gap-3">
                        <div className={`w-2 h-2 rounded-full mt-1.5 shrink-0 ${SEVERITY_DOT[item.severity] || "bg-muted"}`} />
                        <div className="flex-1 min-w-0">
                          <div className="flex items-center gap-2 mb-1">
                            <span className="text-sm font-medium text-foreground">{item.name}</span>
                            <span className={`text-[10px] px-1.5 py-0.5 rounded ${item.severity === "critical" ? "bg-destructive-subtle text-destructive" : item.severity === "warning" ? "bg-warning/10 text-warning" : "bg-muted text-muted-foreground"}`}>
                              {SEVERITY_LABEL[item.severity] || item.severity}
                            </span>
                            <span className="text-[10px] text-muted-foreground">{item.category}</span>
                          </div>
                          <p className="text-xs text-foreground-dim mb-1">{item.description}</p>
                          {item.suggestion && (
                            <p className="text-xs text-muted-foreground">
                              建议: {item.suggestion}
                            </p>
                          )}
                        </div>
                      </div>
                    </div>
                  ))}
              </div>
            ) : (
              <div className="flex items-center gap-2 text-sm text-muted-foreground py-4">
                <CheckCircle size={14} className="text-success" />
                未发现问题
              </div>
            )}

            {/* Raw report (collapsible) */}
            {structuredResult.report_md && (
              <details className="mt-4">
                <summary className="text-xs text-muted-foreground cursor-pointer hover:text-foreground transition-colors">
                  查看原始报告
                </summary>
                <pre className="mt-2 p-4 bg-muted border border-border rounded text-xs text-foreground-dim whitespace-pre-wrap max-h-96 overflow-y-auto">
                  {structuredResult.report_md}
                </pre>
              </details>
            )}
          </>
        )}

        {!structuredResult && !loading && (
          <div className="flex flex-col items-center justify-center py-16 text-muted-foreground text-sm gap-2">
            <RefreshCw size={18} />
            <span>点击"运行健康检查"开始诊断</span>
          </div>
        )}
      </div>
    </div>
  );
}
