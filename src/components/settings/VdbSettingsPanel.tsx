import { useEffect, useState, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useVdbStore } from "@/stores/useVdbStore";
import { formatSize } from "@/lib/utils";
import type { VdbStatus, EmbeddingConfig, ReindexProgress } from "@/types/vdb";
import { GRAPH_OPT_LEVELS, MAX_SEQ_LEN_OPTIONS, POOLING_STRATEGIES } from "@/types/vdb";
import {
  Database, HardDrive, Hash, Save, RefreshCw,
  AlertTriangle, Loader2,
} from "lucide-react";

const INPUT_CLASS =
  "w-full px-3 py-1.5 text-sm border border-[var(--border)] bg-[var(--card)] text-[var(--foreground)] outline-none focus:border-[var(--primary)] placeholder:text-[var(--muted-foreground)] transition-colors";
const INPUT_CLASS_DISABLED =
  "w-full px-3 py-1.5 text-sm border border-[var(--border)] bg-[var(--muted)] text-[var(--muted-foreground)] outline-none cursor-not-allowed opacity-50";
const LABEL_CLASS = "block text-xs text-[var(--muted-foreground)] mb-1";
const BTN_PRIMARY =
  "inline-flex items-center gap-1.5 px-4 py-1.5 text-sm bg-[var(--primary)] text-[var(--primary-foreground)] hover:bg-[var(--primary-hover)] transition-colors disabled:opacity-50";
const BTN_DANGER =
  "inline-flex items-center gap-1.5 px-4 py-1.5 text-sm border border-red-400/30 text-red-500 hover:bg-red-500/10 transition-colors disabled:opacity-50";

const ENGINE_TYPE_LABELS: Record<string, string> = {
  builtin: "内置轻量模型 (bge-small-zh-v1.5)",
  high_perf: "高性能模型 (需下载)",
  custom: "自定义模型",
};

export default function VdbSettingsPanel({ kbId }: { kbId: string }) {
  const status = useVdbStore((s) => s.status);
  const config = useVdbStore((s) => s.config);
  const progress = useVdbStore((s) => s.progress);
  const reindexing = useVdbStore((s) => s.reindexing);
  const error = useVdbStore((s) => s.error);
  const setStatus = useVdbStore((s) => s.setStatus);
  const setConfig = useVdbStore((s) => s.setConfig);
  const setProgress = useVdbStore((s) => s.setProgress);
  const setReindexing = useVdbStore((s) => s.setReindexing);
  const setError = useVdbStore((s) => s.setError);

  const [engineType, setEngineType] = useState("builtin");
  const [customPath, setCustomPath] = useState("");
  const [numThreads, setNumThreads] = useState(
    Math.max(1, navigator.hardwareConcurrency || 4)
  );
  const [graphOptLevel, setGraphOptLevel] = useState("level3");
  const [maxSeqLen, setMaxSeqLen] = useState(512);
  const [poolingStrategy, setPoolingStrategy] = useState("mean");
  const [l2Normalize, setL2Normalize] = useState(true);
  const [saving, setSaving] = useState(false);
  const [saveMsg, setSaveMsg] = useState("");
  const [flushConfirm, setFlushConfirm] = useState(false);

  // Is the current engine type usable? (high_perf has no model until user provides one)
  const engineDisabled = engineType === "high_perf";

  // Track listener cleanup functions with a ref to survive async registration
  const listenerCleanupRef = useRef<(() => void)[]>([]);

  // Load initial data
  useEffect(() => {
    if (!kbId) return;

    let cancelled = false;

    // Clean up stale listeners from previous kbId
    listenerCleanupRef.current.forEach((fn) => fn());
    listenerCleanupRef.current = [];

    invoke<VdbStatus>("get_vdb_status", { kbId })
      .then(setStatus)
      .catch(() => {});

    invoke<EmbeddingConfig>("get_embedding_config")
      .then((c) => {
        setConfig(c);
        setEngineType(c.engine_type);
        if (c.model_path) setCustomPath(c.model_path);
        setNumThreads(c.num_threads);
        if (c.graph_opt_level) setGraphOptLevel(c.graph_opt_level);
        if (c.max_seq_len) setMaxSeqLen(c.max_seq_len);
        if (c.pooling_strategy) setPoolingStrategy(c.pooling_strategy);
        setL2Normalize(c.l2_normalize ?? true);
      })
      .catch(() => {});

    // Register listeners concurrently, store cleanup fns when ready
    Promise.all([
      listen<VdbStatus>("vdb-status-changed", (event) => {
        if (!cancelled && event.payload.kb_id === kbId) {
          setStatus(event.payload);
          if (event.payload.status !== "indexing") {
            setReindexing(false);
            setProgress(null);
          }
        }
      }),
      listen<ReindexProgress>("reindex-progress", (event) => {
        if (!cancelled && event.payload.kb_id === kbId) {
          setProgress({
            current: event.payload.current,
            total: event.payload.total,
            message: event.payload.message,
          });
        }
      }),
    ]).then((fns) => {
      if (cancelled) {
        fns.forEach((fn) => fn());
      } else {
        listenerCleanupRef.current = fns;
      }
    }).catch(() => {});

    return () => {
      cancelled = true;
      listenerCleanupRef.current.forEach((fn) => fn());
      listenerCleanupRef.current = [];
    };
  }, [kbId]);

  const handleSaveConfig = useCallback(async () => {
    setSaving(true);
    setSaveMsg("");
    try {
      await invoke("save_embedding_config", {
        engineType,
        modelPath: engineType === "custom" ? customPath : null,
        numThreads,
        graphOptLevel,
        maxSeqLen,
        poolingStrategy,
        l2Normalize,
      });
      setSaveMsg("配置已保存");
      setError(null);
    } catch (e: any) {
      setError(String(e));
      setSaveMsg("保存失败");
    } finally {
      setSaving(false);
    }
  }, [engineType, customPath, numThreads, graphOptLevel, maxSeqLen, poolingStrategy, l2Normalize, setError]);

  const handleReindex = useCallback(async () => {
    if (!kbId) return;
    setReindexing(true);
    setProgress(null);
    try {
      await invoke("reindex_vdb", { kbId });
    } catch (e: any) {
      setError(String(e));
      setReindexing(false);
    }
  }, [kbId, setReindexing, setProgress, setError]);

  const handleFlush = useCallback(async () => {
    if (!kbId) return;
    try {
      await invoke("flush_vdb", { kbId });
      setFlushConfirm(false);
      const s = await invoke<VdbStatus>("get_vdb_status", { kbId });
      setStatus(s);
    } catch (e: any) {
      setError(String(e));
    }
  }, [kbId, setStatus, setError]);

  const isReindexing = reindexing || status?.status === "indexing";
  const percent = progress && progress.total > 0
    ? Math.round((progress.current / progress.total) * 100)
    : 0;

  if (!kbId) {
    return (
      <div className="text-sm text-[var(--muted-foreground)] p-6 text-center">
        请先在顶部选择一个知识库
      </div>
    );
  }

  return (
    <div className="space-y-8">
      {/* Section A: Inference Engine Settings */}
      <CollapsibleSection title="推理引擎设置" defaultOpen>
        <div className="space-y-4">
          {/* Engine type dropdown */}
          <div>
            <label className={LABEL_CLASS}>模型选择</label>
            <select
              value={engineType}
              onChange={(e) => setEngineType(e.target.value)}
              className={INPUT_CLASS}
            >
              <option value="builtin">{ENGINE_TYPE_LABELS.builtin}</option>
              <option value="high_perf">{ENGINE_TYPE_LABELS.high_perf}</option>
              <option value="custom">{ENGINE_TYPE_LABELS.custom}</option>
            </select>
            {engineType === "high_perf" && (
              <p className="text-xs text-[var(--muted-foreground)] mt-1">
                高性能模型需要手动下载后，通过「自定义模型」选项指定路径。以下参数需模型就绪后方可调整。
              </p>
            )}
          </div>

          {/* Custom model path */}
          {engineType === "custom" && (
            <div>
              <label className={LABEL_CLASS}>自定义模型路径</label>
              <input
                type="text"
                value={customPath}
                onChange={(e) => setCustomPath(e.target.value)}
                placeholder="例如: D:\models\bge-large-zh.onnx"
                className={INPUT_CLASS}
              />
            </div>
          )}

          {/* Thread count slider */}
          <div>
            <label className={LABEL_CLASS}>
              推理线程数
              <span className="ml-1 text-[var(--foreground-dim)]">({numThreads})</span>
            </label>
            <div className="flex items-center gap-3">
              <span className="text-xs text-[var(--muted-foreground)]">1</span>
              <input
                type="range"
                min={1}
                max={Math.max(1, navigator.hardwareConcurrency || 4)}
                step={1}
                value={numThreads}
                onChange={(e) => setNumThreads(Number(e.target.value))}
                disabled={engineDisabled}
                className="flex-1"
                title="推理线程数"
              />
              <span className="text-xs text-[var(--muted-foreground)]">
                {Math.max(1, navigator.hardwareConcurrency || 4)}
              </span>
            </div>
            <p className="text-xs text-[var(--muted-foreground)] mt-1">
              设置为 CPU 核心数以获得最佳性能，减小可降低资源占用
            </p>
          </div>

          {/* Graph optimization level */}
          <div>
            <label className={LABEL_CLASS}>图优化级别</label>
            <select
              title="图优化级别"
              value={graphOptLevel}
              onChange={(e) => setGraphOptLevel(e.target.value)}
              disabled={engineDisabled}
              className={engineDisabled ? INPUT_CLASS_DISABLED : INPUT_CLASS}
            >
              {GRAPH_OPT_LEVELS.map((opt) => (
                <option key={opt.value} value={opt.value}>
                  {opt.label} — {opt.desc}
                </option>
              ))}
            </select>
            <p className="text-xs text-[var(--muted-foreground)] mt-1">
              Level 3 推理最快但加载稍慢，Level 1 加载最快
            </p>
          </div>

          {/* Max sequence length */}
          <div>
            <label className={LABEL_CLASS}>最大序列长度</label>
            <select
              title="最大序列长度"
              value={maxSeqLen}
              onChange={(e) => setMaxSeqLen(Number(e.target.value))}
              disabled={engineDisabled}
              className={engineDisabled ? INPUT_CLASS_DISABLED : INPUT_CLASS}
            >
              {MAX_SEQ_LEN_OPTIONS.map((opt) => (
                <option key={opt.value} value={opt.value}>
                  {opt.label}
                </option>
              ))}
            </select>
            <p className="text-xs text-[var(--muted-foreground)] mt-1">
              限制输入文本 token 数，较小值节省内存但可能截断长文本
            </p>
          </div>

          {/* Pooling strategy */}
          <div>
            <label className={LABEL_CLASS}>池化策略</label>
            <select
              value={poolingStrategy}
              onChange={(e) => setPoolingStrategy(e.target.value)}
              disabled={engineDisabled}
              className={engineDisabled ? INPUT_CLASS_DISABLED : INPUT_CLASS}
            >
              {POOLING_STRATEGIES.map((opt) => (
                <option key={opt.value} value={opt.value}>
                  {opt.label} — {opt.desc}
                </option>
              ))}
            </select>
          </div>

          {/* L2 Normalize toggle */}
          <div>
            <label className="flex items-center gap-2 cursor-pointer">
              <input
                type="checkbox"
                checked={l2Normalize}
                onChange={(e) => setL2Normalize(e.target.checked)}
                disabled={engineDisabled}
              />
              <span className="text-sm text-[var(--foreground)]">L2 归一化</span>
            </label>
            <p className="text-xs text-[var(--muted-foreground)] mt-1 ml-6">
              向量归一化使余弦相似度计算更准确，BGE 模型推荐开启
            </p>
          </div>

          {/* Save button */}
          <div className="flex items-center gap-3">
            <button type="button" onClick={handleSaveConfig} disabled={saving || engineDisabled} className={BTN_PRIMARY}>
              {saving ? <Loader2 size={14} className="animate-spin" /> : <Save size={14} />}
              保存配置
            </button>
            {saveMsg && (
              <span className={`text-xs ${saveMsg === "配置已保存" ? "text-[var(--success)]" : "text-red-500"}`}>
                {saveMsg}
              </span>
            )}
          </div>

          {/* Error display */}
          {error && (
            <div className="px-3 py-2 text-xs bg-red-50 dark:bg-red-950 text-red-700 dark:text-red-400 flex items-center justify-between">
              <span>{error}</span>
              <button type="button" onClick={() => setError(null)} className="text-red-400 hover:text-red-600 ml-3">×</button>
            </div>
          )}
        </div>
      </CollapsibleSection>

      {/* Section B: Vector DB Monitoring Dashboard */}
      <CollapsibleSection title="向量数据库监控" defaultOpen>
        <div className="space-y-6">
          {/* Stats Cards */}
          <div className="grid grid-cols-3 gap-3">
            <StatCard
              icon={<Database size={18} />}
              label="存储块数"
              value={status ? String(status.total_chunks) : "-"}
              sub="个文本块"
            />
            <StatCard
              icon={<HardDrive size={18} />}
              label="磁盘占用"
              value={status ? formatSize(status.disk_size_bytes) : "-"}
              sub={status ? `${(status.disk_size_bytes / 1024 / 1024).toFixed(1)} MB` : ""}
            />
            <StatCard
              icon={<Hash size={18} />}
              label="向量维度"
              value={status ? String(status.vector_dimensions) : "-"}
              sub={status && status.vector_dimensions > 0 ? `dim` : "未加载模型"}
            />
          </div>

          {/* Status Indicator */}
          <div className="p-4 border border-[var(--border)] bg-[var(--card)]">
            <div className="flex items-center gap-3">
              {isReindexing ? (
                <>
                  <span className="relative flex h-3 w-3">
                    <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-yellow-400 opacity-75" />
                    <span className="relative inline-flex rounded-full h-3 w-3 bg-yellow-500" />
                  </span>
                  <span className="text-sm font-medium text-yellow-600 dark:text-yellow-400">
                    正在构建索引...
                  </span>
                </>
              ) : status?.status === "error" ? (
                <>
                  <span className="relative flex h-3 w-3">
                    <span className="relative inline-flex rounded-full h-3 w-3 bg-red-500" />
                  </span>
                  <span className="text-sm font-medium text-red-600 dark:text-red-400">
                    错误{status.error_message ? `: ${status.error_message}` : ""}
                  </span>
                </>
              ) : (
                <>
                  <span className="relative flex h-3 w-3">
                    <span className="relative inline-flex rounded-full h-3 w-3 bg-green-500" />
                  </span>
                  <span className="text-sm font-medium text-green-600 dark:text-green-400">
                    就绪
                  </span>
                </>
              )}
            </div>

            {/* Progress bar */}
            {isReindexing && progress && (
              <div className="mt-3">
                <div className="flex items-center justify-between mb-1.5">
                  <span className="text-xs text-[var(--muted-foreground)]">
                    {progress.message}
                  </span>
                  <span className="text-xs text-[var(--muted-foreground)]">
                    {percent}%
                  </span>
                </div>
                <div className="h-2 bg-[var(--muted)] rounded-full overflow-hidden">
                  <div
                    className="h-full bg-[var(--primary)] rounded-full transition-all duration-300"
                    style={{ width: `${percent}%` }}
                  />
                </div>
              </div>
            )}
          </div>

          {/* Danger Zone */}
          <div className="p-4 border border-red-400/30 bg-red-500/5">
            <div className="flex items-center gap-2 mb-3">
              <AlertTriangle size={16} className="text-red-500" />
              <span className="text-sm font-medium text-red-600 dark:text-red-400">危险操作区</span>
            </div>
            <div className="flex items-center gap-3 flex-wrap">
              <button
                type="button"
                onClick={handleReindex}
                disabled={isReindexing}
                className={BTN_PRIMARY}
              >
                {isReindexing ? <Loader2 size={14} className="animate-spin" /> : <RefreshCw size={14} />}
                重构全局索引 (Reindex)
              </button>
              {!flushConfirm ? (
                <button
                  type="button"
                  onClick={() => setFlushConfirm(true)}
                  disabled={isReindexing}
                  className={BTN_DANGER}
                >
                  <AlertTriangle size={14} />
                  清空向量库 (Flush)
                </button>
              ) : (
                <div className="flex items-center gap-2">
                  <span className="text-xs text-red-500">确认清空？此操作不可恢复</span>
                  <button type="button" onClick={handleFlush} className={BTN_DANGER}>
                    确认清空
                  </button>
                  <button
                    type="button"
                    onClick={() => setFlushConfirm(false)}
                    className="inline-flex items-center gap-1 px-3 py-1 text-xs border border-[var(--border)] text-[var(--muted-foreground)] hover:bg-[var(--card-hover)] transition-colors"
                  >
                    取消
                  </button>
                </div>
              )}
            </div>
          </div>
        </div>
      </CollapsibleSection>
    </div>
  );
}

// ---- Sub-components ----

function CollapsibleSection({
  title,
  children,
  defaultOpen = true,
}: {
  title: string;
  children: React.ReactNode;
  defaultOpen?: boolean;
}) {
  const [open, setOpen] = useState(defaultOpen);
  return (
    <div>
      <button
        type="button"
        onClick={() => setOpen(!open)}
        className="flex items-center gap-1.5 w-full text-left mb-3 group"
      >
        <span className="text-xs text-[var(--foreground-dim)] group-hover:text-[var(--foreground)] transition-colors">
          {open ? "▾" : "▸"}
        </span>
        <span className="text-sm font-medium text-[var(--foreground-dim)]">{title}</span>
      </button>
      {open && <div className="pl-5">{children}</div>}
    </div>
  );
}

function StatCard({
  icon,
  label,
  value,
  sub,
}: {
  icon: React.ReactNode;
  label: string;
  value: string;
  sub: string;
}) {
  return (
    <div className="p-4 border border-[var(--border)] bg-[var(--card)]">
      <div className="flex items-center gap-2 mb-2 text-[var(--muted-foreground)]">
        {icon}
        <span className="text-xs">{label}</span>
      </div>
      <div className="text-2xl font-bold text-[var(--foreground)]">{value}</div>
      <div className="text-xs text-[var(--muted-foreground)] mt-1">{sub}</div>
    </div>
  );
}
