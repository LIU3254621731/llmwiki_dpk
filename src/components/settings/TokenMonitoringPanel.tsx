import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useAppStore } from "@/stores/useAppStore";
import { useEditorStore } from "@/stores/useEditorStore";
import { Line } from "react-chartjs-2";
import {
  Chart as ChartJS,
  CategoryScale,
  LinearScale,
  PointElement,
  LineElement,
  Title,
  Tooltip,
  Legend,
  Filler,
} from "chart.js";
import {
  ArrowUpRight, ArrowDownRight, Coins, BarChart3,
  ChevronLeft, ChevronRight, Shield, AlertTriangle,
} from "lucide-react";
import type {
  TokenStats, DailyTokenUsage, PaginatedTokenLogs,
  DailyTokenLimit, TokenQuotaStatus,
} from "@/types/token";

ChartJS.register(
  CategoryScale, LinearScale, PointElement, LineElement,
  Title, Tooltip, Legend, Filler
);

const CARD_CLASS =
  "border border-[var(--border)] bg-[var(--card)] p-4 flex flex-col gap-2";
const CARD_VALUE_CLASS = "text-2xl font-bold text-[var(--foreground)]";
const CARD_LABEL_CLASS = "text-xs text-[var(--muted-foreground)]";

function formatTokens(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return String(n);
}

function formatCost(yuan: number): string {
  if (yuan >= 1) return `¥${yuan.toFixed(2)}`;
  return `¥${yuan.toFixed(4)}`;
}

export default function TokenMonitoringPanel() {
  const setTaskDetailId = useAppStore((s) => s.setTaskDetailId);
  const openFile = useEditorStore((s) => s.openFile);
  const [range, setRange] = useState<"today" | "7days" | "month">("7days");
  const [stats, setStats] = useState<TokenStats | null>(null);
  const [trend, setTrend] = useState<DailyTokenUsage[]>([]);
  const [logs, setLogs] = useState<PaginatedTokenLogs | null>(null);
  const [page, setPage] = useState(1);
  const [limitCfg, setLimitCfg] = useState<DailyTokenLimit>({ enabled: false, limit: 2_000_000 });
  const [quota, setQuota] = useState<TokenQuotaStatus | null>(null);
  const [loading, setLoading] = useState(true);

  const PAGE_SIZE = 15;

  const loadStats = useCallback(async () => {
    try {
      const s = await invoke<TokenStats>("get_token_statistics", { range });
      setStats(s);
    } catch (e) {
      console.error("loadStats:", e);
    }
  }, [range]);

  const loadTrend = useCallback(async () => {
    try {
      const t = await invoke<DailyTokenUsage[]>("get_token_daily_trend");
      setTrend(t);
    } catch (e) {
      console.error("loadTrend:", e);
    }
  }, []);

  const loadLogs = useCallback(async () => {
    try {
      const l = await invoke<PaginatedTokenLogs>("get_token_logs", {
        page,
        pageSize: PAGE_SIZE,
      });
      setLogs(l);
    } catch (e) {
      console.error("loadLogs:", e);
    }
  }, [page]);

  const loadLimit = useCallback(async () => {
    try {
      const cfg = await invoke<DailyTokenLimit>("get_daily_token_limit");
      setLimitCfg(cfg);
      const q = await invoke<TokenQuotaStatus>("check_token_quota");
      setQuota(q);
    } catch (e) {
      console.error("loadLimit:", e);
    }
  }, []);

  useEffect(() => {
    setLoading(true);
    Promise.all([loadStats(), loadTrend(), loadLogs(), loadLimit()]).finally(() =>
      setLoading(false)
    );
  }, [loadStats, loadTrend, loadLogs, loadLimit]);

  const handleSetLimit = async () => {
    try {
      await invoke("set_daily_token_limit", {
        enabled: limitCfg.enabled,
        limit: limitCfg.limit,
      });
      const q = await invoke<TokenQuotaStatus>("check_token_quota");
      setQuota(q);
    } catch (e) {
      console.error("set_daily_token_limit:", e);
    }
  };

  // Chart data
  const chartData = {
    labels: trend.map((d) => d.date.slice(5)), // MM-DD
    datasets: [
      {
        label: "输入 Token",
        data: trend.map((d) => d.input_tokens),
        borderColor: "rgb(99, 102, 241)",
        backgroundColor: "rgba(99, 102, 241, 0.1)",
        fill: true,
        tension: 0.3,
        pointRadius: 3,
      },
      {
        label: "输出 Token",
        data: trend.map((d) => d.output_tokens),
        borderColor: "rgb(34, 197, 94)",
        backgroundColor: "rgba(34, 197, 94, 0.1)",
        fill: true,
        tension: 0.3,
        pointRadius: 3,
      },
    ],
  };

  const chartOptions = {
    responsive: true,
    maintainAspectRatio: false,
    plugins: {
      legend: { position: "top" as const, labels: { color: "var(--muted-foreground)", font: { size: 11 } } },
      tooltip: { callbacks: { label: (ctx: any) => `${ctx.dataset.label}: ${formatTokens(ctx.raw)}` } },
    },
    scales: {
      x: { ticks: { color: "var(--muted-foreground)", font: { size: 10 } }, grid: { color: "var(--border)" } },
      y: {
        ticks: { color: "var(--muted-foreground)", font: { size: 10 }, callback: (v: any) => formatTokens(v) },
        grid: { color: "var(--border)" },
        beginAtZero: true,
      },
    },
  };

  const totalPages = logs ? Math.ceil(logs.total / PAGE_SIZE) : 1;

  return (
    <div className="flex flex-col gap-6 h-full overflow-y-auto">
      {/* Range selector */}
      <div className="flex items-center gap-2">
        {(["today", "7days", "month"] as const).map((r) => (
          <button
            type="button"
            key={r}
            onClick={() => setRange(r)}
            className={`px-3 py-1 text-xs border transition-colors ${
              range === r
                ? "border-[var(--primary)] bg-[var(--primary)] text-[var(--primary-foreground)]"
                : "border-[var(--border)] text-[var(--muted-foreground)] hover:bg-[var(--card-hover)]"
            }`}
          >
            {r === "today" ? "今天" : r === "7days" ? "最近 7 天" : "本月"}
          </button>
        ))}
      </div>

      {/* Stats cards */}
      <div className="grid grid-cols-3 gap-4">
        <div className={CARD_CLASS}>
          <div className="flex items-center gap-2">
            <ArrowUpRight className="w-4 h-4 text-blue-400" />
            <span className={CARD_LABEL_CLASS}>累计输入 Token</span>
          </div>
          <div className={CARD_VALUE_CLASS}>
            {stats ? formatTokens(stats.total_input_tokens) : "-"}
          </div>
          {stats && (
            <div className="text-xs text-[var(--muted-foreground)]">
              共 {stats.call_count} 次调用
            </div>
          )}
        </div>
        <div className={CARD_CLASS}>
          <div className="flex items-center gap-2">
            <ArrowDownRight className="w-4 h-4 text-green-400" />
            <span className={CARD_LABEL_CLASS}>累计输出 Token</span>
          </div>
          <div className={CARD_VALUE_CLASS}>
            {stats ? formatTokens(stats.total_output_tokens) : "-"}
          </div>
        </div>
        <div className={CARD_CLASS}>
          <div className="flex items-center gap-2">
            <Coins className="w-4 h-4 text-yellow-400" />
            <span className={CARD_LABEL_CLASS}>折算总花费</span>
          </div>
          <div className={CARD_VALUE_CLASS}>
            {stats ? formatCost(stats.total_cost_yuan) : "-"}
          </div>
          <div className="text-xs text-[var(--muted-foreground)]">
            费率: 输入 ¥1/M 输出 ¥2/M
          </div>
        </div>
      </div>

      {/* Chart */}
      <div className="border border-[var(--border)] bg-[var(--card)] p-4">
        <div className="flex items-center gap-2 mb-4">
          <BarChart3 className="w-4 h-4 text-[var(--muted-foreground)]" />
          <span className="text-sm text-[var(--foreground)]">最近 7 天 Token 消耗趋势</span>
        </div>
        <div className="h-64">
          {trend.length > 0 ? (
            <Line data={chartData} options={chartOptions} />
          ) : (
            <div className="flex items-center justify-center h-full text-sm text-[var(--muted-foreground)]">
              暂无数据
            </div>
          )}
        </div>
      </div>

      {/* Circuit breaker */}
      <div className="border border-[var(--border)] bg-[var(--card)] p-4">
        <div className="flex items-center justify-between mb-4">
          <div className="flex items-center gap-2">
            <Shield className="w-4 h-4 text-[var(--muted-foreground)]" />
            <span className="text-sm text-[var(--foreground)]">每日 Token 额度限制</span>
          </div>
          <button
            type="button"
            onClick={() => {
              const next = { ...limitCfg, enabled: !limitCfg.enabled };
              setLimitCfg(next);
            }}
            className={`w-10 h-5 rounded-full transition-colors relative ${
              limitCfg.enabled ? "bg-[var(--primary)]" : "bg-[var(--border)]"
            }`}
          >
            <div
              className={`absolute top-0.5 w-4 h-4 rounded-full bg-white transition-transform ${
                limitCfg.enabled ? "left-5" : "left-0.5"
              }`}
            />
          </button>
        </div>
        {limitCfg.enabled && (
          <div className="flex items-center gap-3">
            <input
              type="number"
              value={limitCfg.limit}
              onChange={(e) =>
                setLimitCfg({ ...limitCfg, limit: Math.max(1, Number(e.target.value)) })
              }
              className="w-48 px-3 py-1.5 text-sm border border-[var(--border)] bg-[var(--card)] text-[var(--foreground)] outline-none focus:border-[var(--primary)]"
              placeholder="Token 上限"
            />
            <span className="text-xs text-[var(--muted-foreground)]">Tokens / 天</span>
            <button
              type="button"
              onClick={handleSetLimit}
              className="px-3 py-1.5 text-xs bg-[var(--primary)] text-[var(--primary-foreground)] hover:opacity-90 transition-opacity"
            >
              保存
            </button>
          </div>
        )}
        {quota && limitCfg.enabled && (
          <div className={`mt-3 p-3 border text-sm ${
            quota.allowed
              ? "border-green-400/30 bg-green-400/5 text-green-400"
              : "border-red-400/30 bg-red-400/5 text-red-400"
          }`}>
            <div className="flex items-center gap-2">
              {quota.allowed ? (
                <Shield className="w-4 h-4" />
              ) : (
                <AlertTriangle className="w-4 h-4" />
              )}
              {quota.message}
            </div>
            {quota.allowed && (
              <div className="mt-1 text-xs opacity-70">
                今日: {formatTokens(quota.today_used)} / {formatTokens(quota.limit)} | 剩余: {formatTokens(quota.remaining)}
              </div>
            )}
          </div>
        )}
      </div>

      {/* History table */}
      <div className="border border-[var(--border)] bg-[var(--card)] flex flex-col flex-1 min-h-0">
        <div className="flex items-center gap-2 p-4 border-b border-[var(--border)]">
          <span className="text-sm text-[var(--foreground)]">历史消耗明细</span>
          {logs && (
            <span className="text-xs text-[var(--muted-foreground)]">
              (共 {logs.total} 条)
            </span>
          )}
        </div>
        <div className="flex-1 overflow-auto">
          <table className="w-full text-xs">
            <thead className="sticky top-0 bg-[var(--card)] border-b border-[var(--border)]">
              <tr className="text-left text-[var(--muted-foreground)]">
                <th className="px-4 py-2 font-normal">任务名称</th>
                <th className="px-4 py-2 font-normal">Agent</th>
                <th className="px-4 py-2 font-normal text-right">输入 Token</th>
                <th className="px-4 py-2 font-normal text-right">输出 Token</th>
                <th className="px-4 py-2 font-normal">模型</th>
                <th className="px-4 py-2 font-normal">时间</th>
              </tr>
            </thead>
            <tbody>
              {logs?.entries.map((entry) => (
                <tr
                  key={entry.id}
                  className="border-b border-[var(--border)] hover:bg-[var(--card-hover)] cursor-pointer transition-colors"
                  onClick={() => {
                    if (entry.task_id) {
                      setTaskDetailId(entry.task_id);
                      openFile({ path: "task-detail", title: "任务详情", type: "task_detail" });
                    }
                  }}
                >
                  <td className="px-4 py-2 text-[var(--foreground)] max-w-40 truncate" title={entry.task_name}>
                    {entry.task_name}
                  </td>
                  <td className="px-4 py-2 text-[var(--muted-foreground)]">{entry.agent_name}</td>
                  <td className="px-4 py-2 text-right text-blue-400 font-mono">
                    {formatTokens(entry.input_tokens)}
                  </td>
                  <td className="px-4 py-2 text-right text-green-400 font-mono">
                    {formatTokens(entry.output_tokens)}
                  </td>
                  <td className="px-4 py-2 text-[var(--muted-foreground)] text-[10px]">{entry.model_name}</td>
                  <td className="px-4 py-2 text-[var(--muted-foreground)] text-[10px]">
                    {entry.created_at?.slice(0, 16)?.replace("T", " ")}
                  </td>
                </tr>
              ))}
              {(!logs || logs.entries.length === 0) && (
                <tr>
                  <td colSpan={6} className="px-4 py-8 text-center text-[var(--muted-foreground)]">
                    暂无 Token 消耗记录
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
        {logs && logs.total > PAGE_SIZE && (
          <div className="flex items-center justify-between px-4 py-2 border-t border-[var(--border)]">
            <span className="text-xs text-[var(--muted-foreground)]">
              第 {page} / {totalPages} 页
            </span>
            <div className="flex items-center gap-1">
              <button
                type="button"
                onClick={() => setPage((p) => Math.max(1, p - 1))}
                disabled={page <= 1}
                aria-label="上一页"
                className="p-1 text-[var(--muted-foreground)] hover:text-[var(--foreground)] disabled:opacity-30"
              >
                <ChevronLeft className="w-4 h-4" />
              </button>
              <button
                type="button"
                onClick={() => setPage((p) => Math.min(totalPages, p + 1))}
                disabled={page >= totalPages}
                aria-label="下一页"
                className="p-1 text-[var(--muted-foreground)] hover:text-[var(--foreground)] disabled:opacity-30"
              >
                <ChevronRight className="w-4 h-4" />
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
