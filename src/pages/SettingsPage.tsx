import { useEffect, useState, lazy, Suspense } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useModelStore } from "@/stores/useModelStore";
import { useKBStore } from "@/stores/useKBStore";
import { useThemeStore } from "@/stores/useThemeStore";
import { PROVIDER_DEFAULTS } from "@/types/model";
import {
  Save, Loader2, Trash2, Plus, Play, ChevronDown, ChevronRight,
  Settings, Cpu, Globe, HardDrive, AlertTriangle, FileText,
  HeartPulse, Shield, CheckCircle2, XCircle, Info, RefreshCw, Wrench,
  Sun, Moon, Monitor, Coins, Database, Bot, Activity, Clock,
} from "lucide-react";
const TokenMonitoringPanel = lazy(() => import("@/components/settings/TokenMonitoringPanel"));
const VdbSettingsPanel = lazy(() => import("@/components/settings/VdbSettingsPanel"));
const AgentManager = lazy(() => import("@/components/settings/AgentManager"));
const SkillPlatform = lazy(() => import("@/components/settings/SkillPlatform"));

function PanelFallback() {
  return (
    <div className="flex items-center justify-center py-12">
      <Loader2 size={18} className="animate-spin text-[var(--muted-foreground)]" />
    </div>
  );
}

const INPUT_CLASS =
  "w-full px-3 py-1.5 text-sm border border-[var(--border)] bg-[var(--card)] text-[var(--foreground)] outline-none focus:border-[var(--primary)] placeholder:text-[var(--muted-foreground)] transition-colors";
const LABEL_CLASS = "block text-xs text-[var(--muted-foreground)] mb-1";
const BTN_PRIMARY =
  "inline-flex items-center gap-1.5 px-4 py-1.5 text-sm bg-[var(--primary)] text-[var(--primary-foreground)] hover:bg-[var(--primary-hover)] transition-colors disabled:opacity-50";
const BTN_SECONDARY =
  "inline-flex items-center gap-1.5 px-4 py-1.5 text-sm border border-[var(--border)] text-[var(--muted-foreground)] hover:bg-[var(--card-hover)] transition-colors disabled:opacity-50";
const BTN_DANGER =
  "inline-flex items-center gap-1.5 px-4 py-1.5 text-sm border border-red-400/30 text-red-500 hover:bg-red-500/10 transition-colors disabled:opacity-50";

type NavSection = "model" | "kb" | "websearch" | "docs" | "vdb" | "appearance" | "danger" | "health" | "token" | "agent" | "agent_manage" | "skill_platform";

const NAV_ITEMS: { key: NavSection; label: string; icon: typeof Cpu }[] = [
  { key: "model", label: "模型", icon: Cpu },
  { key: "kb", label: "知识库", icon: HardDrive },
  { key: "websearch", label: "网页搜索", icon: Globe },
  { key: "docs", label: "文档解析", icon: FileText },
  { key: "vdb", label: "向量数据库", icon: Database },
  { key: "appearance", label: "外观", icon: Sun },
  { key: "danger", label: "危险区", icon: AlertTriangle },
  { key: "health", label: "知识库健康", icon: HeartPulse },
  { key: "token", label: "Token 监测", icon: Coins },
  { key: "agent", label: "Agent 状态", icon: Bot },
  { key: "agent_manage", label: "Agent 管理", icon: Bot },
  { key: "skill_platform", label: "Skill 工作台", icon: Wrench },
];

export default function SettingsPage() {
  const config = useModelStore((s) => s.config);
  const setConfig = useModelStore((s) => s.setConfig);
  const currentKB = useKBStore((s) => s.currentKB);
  const setCurrentKB = useKBStore((s) => s.setCurrentKB);
  const setKnowledgeBases = useKBStore((s) => s.setKnowledgeBases);
  const theme = useThemeStore((s) => s.theme);
  const setTheme = useThemeStore((s) => s.setTheme);

  const [activeSection, setActiveSection] = useState<NavSection>("model");
  const [configLoading, setConfigLoading] = useState(true);

  // ---- Model config ----
  const [provider, setProvider] = useState("deepseek");
  const [baseUrl, setBaseUrl] = useState("https://api.deepseek.com");
  const [apiKey, setApiKey] = useState("");
  const [chatModel, setChatModel] = useState("deepseek-chat");
  const [reasonerModel, setReasonerModel] = useState("deepseek-reasoner");
  const [temperature, setTemperature] = useState(0.7);
  const [maxTokens, setMaxTokens] = useState(4096);
  const [timeout, setTimeout_V] = useState(120);
  const [retryCount, setRetryCount] = useState(3);
  const [stream, setStream] = useState(true);
  const [saving, setSaving] = useState(false);
  const [testing, setTesting] = useState(false);
  const [testResult, setTestResult] = useState("");
  const [testError, setTestError] = useState(false);
  const [msg, setMsg] = useState("");
  const [error, setError] = useState("");

  // Auto-dismiss success messages after 3s
  useEffect(() => { if (msg) { const t = setTimeout(() => setMsg(""), 3000); return () => clearTimeout(t); } }, [msg]);

  // ---- Model profiles ----
  const [modelProfiles, setModelProfiles] = useState<any[]>([]);
  const [profilesLoading, setProfilesLoading] = useState(false);
  const [showAddProfile, setShowAddProfile] = useState(false);
  const [npName, setNpName] = useState("");
  const [npProvider, setNpProvider] = useState("deepseek");
  const [npBaseUrl, setNpBaseUrl] = useState("https://api.deepseek.com");
  const [npModelName, setNpModelName] = useState("");
  const [npApiKey, setNpApiKey] = useState("");
  const [npRole, setNpRole] = useState("chat");
  const [npTemperature, setNpTemperature] = useState(0.7);
  const [npMaxTokens, setNpMaxTokens] = useState(4096);
  const [npTimeout, setNpTimeout] = useState(120);
  const [npRetryCount, setNpRetryCount] = useState(3);
  const [npSaving, setNpSaving] = useState(false);
  const [npMsg, setNpMsg] = useState("");

  // ---- KB settings ----
  const [kbName, setKbName] = useState("");
  const [kbLanguage, setKbLanguage] = useState("zh");
  const [kbReviewMode, setKbReviewMode] = useState("balanced");
  const [kbAllowAiGen, setKbAllowAiGen] = useState(true);
  const [kbSaving, setKbSaving] = useState(false);
  const [kbMsg, setKbMsg] = useState("");

  // ---- Web search ----
  const [wsEngine, setWsEngine] = useState("duckduckgo");
  const [wsMaxResults, setWsMaxResults] = useState(10);
  const [wsSearxngUrl, setWsSearxngUrl] = useState("");
  const [wsBraveApiKey, setWsBraveApiKey] = useState("");
  const [wsBingApiKey, setWsBingApiKey] = useState("");
  const [wsBingEndpoint, setWsBingEndpoint] = useState("");
  const [wsSaving, setWsSaving] = useState(false);
  const [wsMsg, setWsMsg] = useState("");

  // Auto-dismiss KB and WS messages after 3s
  useEffect(() => { if (kbMsg) { const t = setTimeout(() => setKbMsg(""), 3000); return () => clearTimeout(t); } }, [kbMsg]);
  useEffect(() => { if (wsMsg) { const t = setTimeout(() => setWsMsg(""), 3000); return () => clearTimeout(t); } }, [wsMsg]);

  // ---- Document parsing ----
  const [mdStatus, setMdStatus] = useState<{ available: boolean; python_found: boolean; description: string } | null>(null);
  const [mdChecking, setMdChecking] = useState(false);
  const [mdInstalling, setMdInstalling] = useState(false);

  // ---- Health check ----
  const [healthSubTab, setHealthSubTab] = useState<"system" | "reconciliation">("system");
  const [healthRunning, setHealthRunning] = useState(false);
  const [healthResult, setHealthResult] = useState<any>(null);
  const [healthError, setHealthError] = useState("");
  const [reconcileRunning, setReconcileRunning] = useState(false);
  const [reconcileResult, setReconcileResult] = useState<any>(null);
  const [reconcileError, setReconcileError] = useState("");
  const [quickFixRunning, setQuickFixRunning] = useState<string | null>(null);
  const [quickFixResult, setQuickFixResult] = useState("");

  // ---- Agent status ----
  const [agentTasks, setAgentTasks] = useState<any[]>([]);
  const [agentEvents, setAgentEvents] = useState<any[]>([]);
  const [agentLoading, setAgentLoading] = useState(false);
  const [agentError, setAgentError] = useState("");

  // Load configs on mount
  useEffect(() => {
    invoke<any>("get_provider_config")
      .then((c) => {
        if (c) {
          setProvider(c.provider || "deepseek");
          setBaseUrl(c.base_url);
          setChatModel(c.chat_model);
          setReasonerModel(c.reasoner_model);
          setTemperature(c.temperature);
          setMaxTokens(c.max_tokens);
          setTimeout_V(c.timeout);
          setRetryCount(c.retry_count);
          setStream(c.stream);
          setConfig(c);
          if (c.api_key_masked && c.api_key_masked !== "未配置") setApiKey("");
        }
      })
      .catch((e) => setError(`加载配置失败: ${e}`))
      .finally(() => setConfigLoading(false));

    loadProfiles();
    checkMdStatus();

    invoke<any>("get_web_search_config").then((c) => {
      if (c) {
        setWsEngine(c.engine);
        setWsMaxResults(c.max_results);
        setWsSearxngUrl(c.searxng_url || "");
        setWsBraveApiKey(c.brave_api_key || "");
        setWsBingApiKey(c.bing_api_key || "");
        setWsBingEndpoint(c.bing_endpoint || "");
      }
    }).catch((e) => console.error("加载网页搜索配置失败:", e));
  }, []);

  useEffect(() => {
    if (currentKB) {
      setKbName(currentKB.name);
      invoke<any>("get_kb_stats", { kbId: currentKB.id })
        .then((s) => {
          if (s) { setKbLanguage(s.language || "zh"); setKbReviewMode(s.review_mode || "balanced"); setKbAllowAiGen(s.allow_ai_generation ?? true); }
        })
        .catch(() => { setKbLanguage("zh"); setKbReviewMode("balanced"); });
    }
  }, [currentKB?.id]);

  const loadProfiles = async () => {
    setProfilesLoading(true);
    try { setModelProfiles(await invoke<any[]>("list_model_profiles")); } catch (e) {
      setError(`加载模型配置列表失败: ${e}`);
    }
    setProfilesLoading(false);
  };

  const checkMdStatus = async () => {
    setMdChecking(true);
    try { setMdStatus(await invoke<any>("get_markitdown_status")); } catch { /* ignore */ }
    setMdChecking(false);
  };

  // ---- Handlers ----
  const handleSave = async () => {
    setSaving(true); setMsg(""); setError("");
    try {
      await invoke("save_provider_config", { provider, baseUrl, apiKey, chatModel, reasonerModel, temperature, maxTokens, timeout, retryCount, stream });
      setMsg("已保存");
    } catch (e) { setError(`保存失败: ${e}`); }
    setSaving(false);
  };

  const handleTest = async () => {
    setTesting(true); setTestResult(""); setTestError(false);
    try { setTestResult(await invoke<string>("test_connection")); } catch (e) { setTestResult(String(e)); setTestError(true); }
    setTesting(false);
  };

  const handleTestJson = async () => {
    setTesting(true); setTestResult(""); setTestError(false);
    try { setTestResult(await invoke<string>("test_json_output")); } catch (e) { setTestResult(String(e)); setTestError(true); }
    setTesting(false);
  };

  const handleApplyProfile = async (id: string) => {
    setMsg(""); setError("");
    try {
      await invoke("apply_model_profile", { profileId: id });
      const c = await invoke<any>("get_provider_config");
      if (c) {
        setProvider(c.provider || "deepseek");
        setBaseUrl(c.base_url); setChatModel(c.chat_model); setReasonerModel(c.reasoner_model);
        setTemperature(c.temperature); setMaxTokens(c.max_tokens);
        setTimeout_V(c.timeout); setRetryCount(c.retry_count); setStream(c.stream);
        setConfig(c);
      }
      setMsg("配置已应用，点击保存以生效");
    } catch (e) { setError(`应用失败: ${e}`); }
  };

  const handleDeleteProfile = async (id: string) => {
    if (!confirm("确定要删除此模型配置吗？此操作不可撤销。")) return;
    try { await invoke("delete_model_profile", { profileId: id }); loadProfiles(); } catch (e) { setError(`删除失败: ${e}`); }
  };

  const handleOpenAddProfile = () => {
    setNpName(""); setNpProvider("deepseek"); setNpBaseUrl(baseUrl);
    setNpModelName(chatModel); setNpApiKey(apiKey); setNpRole("chat");
    setNpTemperature(temperature); setNpMaxTokens(maxTokens);
    setNpTimeout(timeout); setNpRetryCount(retryCount); setNpMsg("");
    setShowAddProfile(true);
  };

  const handleSaveProfile = async () => {
    if (!npName.trim()) { setNpMsg("请输入配置名称"); return; }
    setNpSaving(true); setNpMsg("");
    try {
      await invoke("save_model_profile", {
        name: npName.trim(), provider: npProvider, baseUrl: npBaseUrl,
        modelName: npModelName, apiKey: npApiKey, role: npRole,
        temperature: npTemperature, maxTokens: npMaxTokens,
        timeout: npTimeout, retryCount: npRetryCount,
      });
      setShowAddProfile(false); loadProfiles(); setMsg("配置已保存");
    } catch (e) { setNpMsg(`保存失败: ${e}`); }
    setNpSaving(false);
  };

  const handleSaveKB = async () => {
    if (!currentKB) return;
    if (!kbName.trim()) { setKbMsg("名称不能为空"); return; }
    setKbSaving(true); setKbMsg("");
    try {
      const updated = await invoke<any>("update_knowledge_base", {
        kbId: currentKB.id, name: kbName.trim(), templateName: "general",
        language: kbLanguage, reviewMode: kbReviewMode,
        allowAiGeneration: kbAllowAiGen,
      });
      setCurrentKB({ ...currentKB, name: updated.name, path: updated.path, updated_at: updated.updated_at });
      setKbMsg("已保存");
    } catch (e) { setKbMsg(`保存失败: ${e}`); }
    setKbSaving(false);
  };

  const handleSaveWebSearch = async () => {
    setWsSaving(true); setWsMsg("");
    try {
      await invoke("save_web_search_config", {
        engine: wsEngine, maxResults: wsMaxResults, searxngUrl: wsSearxngUrl,
        braveApiKey: wsBraveApiKey, bingApiKey: wsBingApiKey, bingEndpoint: wsBingEndpoint,
      });
      setWsMsg("已保存");
    } catch (e) { setWsMsg(`保存失败: ${e}`); }
    setWsSaving(false);
  };

  const loadAgentStatus = async () => {
    if (!currentKB) return;
    setAgentLoading(true);
    setAgentError("");
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const tasks = await invoke<any[]>("list_tasks", { kbId: currentKB.id });
      setAgentTasks(tasks || []);
    } catch (e) {
      setAgentError(`加载Agent状态失败: ${e}`);
    }
    setAgentLoading(false);
  };

  const loadAgentEvents = async (taskId: string) => {
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const events = await invoke<any[]>("get_task_events", { taskId });
      setAgentEvents(events || []);
    } catch (e) {
      setAgentEvents([]);
    }
  };

  useEffect(() => {
    if (activeSection === "agent" && currentKB) {
      loadAgentStatus();
    }
  }, [activeSection, currentKB?.id]);

  const handleResetAll = async () => {
    if (!confirm("确定要删除所有知识库数据吗？\n此操作不可恢复。")) return;
    setMsg(""); setError("");
    try {
      const result = await invoke<string>("reset_all_data");
      setMsg(result);
      setKnowledgeBases([]);
      setCurrentKB(null);
    } catch (e) { setError(`重置失败: ${e}`); }
  };

  // ---- Health check handlers ----

  const handleRunHealthCheck = async () => {
    if (!currentKB) return;
    setHealthRunning(true);
    setHealthError("");
    setHealthResult(null);
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const result = await invoke<any>("run_health_check_structured", {
        kbId: currentKB.id,
        kbPath: currentKB.path,
      });
      setHealthResult(result);
    } catch (e) {
      setHealthError(`健康检查失败: ${e}`);
    }
    setHealthRunning(false);
  };

  const handleQuickFix = async (cmd: string, label: string) => {
    if (!currentKB) return;
    setQuickFixRunning(cmd);
    setQuickFixResult("");
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const result = await invoke<string>(cmd, {
        kbId: currentKB.id,
        kbPath: currentKB.path,
      });
      setQuickFixResult(result || `${label} 完成`);
    } catch (e) {
      setQuickFixResult(`${label} 失败: ${e}`);
    }
    setQuickFixRunning(null);
  };

  const handleFixIssue = async (item: any) => {
    if (!currentKB || quickFixRunning !== null) return;
    setQuickFixRunning(item.id || "single_fix");
    setQuickFixResult("");
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const result = await invoke<string>("fix_health_issue", {
        kbId: currentKB.id,
        kbPath: currentKB.path,
        issueId: item.id,
        issueType: item.type || item.category,
      });
      setQuickFixResult(result || "修复完成");
    } catch (e) {
      setQuickFixResult(`修复失败: ${e}`);
    }
    setQuickFixRunning(null);
  };

  const handleRunReconcile = async () => {
    if (!currentKB) return;
    setReconcileRunning(true);
    setReconcileError("");
    setReconcileResult(null);
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const result = await invoke<any>("run_reconcile", {
        kbId: currentKB.id,
        kbPath: currentKB.path,
      });
      setReconcileResult(result);
    } catch (e) {
      setReconcileError(`一致性检查失败: ${e}`);
    }
    setReconcileRunning(false);
  };

  const handleFixReconcileItem = async (itemType: string, item: any) => {
    if (!currentKB || quickFixRunning !== null) return;
    setQuickFixRunning(itemType);
    setQuickFixResult("");
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const result = await invoke<string>("fix_reconcile_issue", {
        kbId: currentKB.id,
        kbPath: currentKB.path,
        issueType: itemType,
        issueId: item?.id,
        issueData: item,
      });
      setQuickFixResult(result || "修复完成");
    } catch (e) {
      setQuickFixResult(`修复失败: ${e}`);
    }
    setQuickFixRunning(null);
  };

  // ---- Render sections ----

  const renderModelSection = () => (
    <div className="space-y-8">
      {msg && <Message type="success" text={msg} onDismiss={() => setMsg("")} />}
      {error && <Message type="error" text={error} onDismiss={() => setError("")} />}
      {testResult && <Message type={testError ? "error" : "success"} text={testResult} onDismiss={() => setTestResult("")} />}

      {configLoading ? (
        <div className="flex items-center gap-2 text-sm text-slate-400"><Loader2 size={14} className="animate-spin" />加载中...</div>
      ) : (
        <CollapsibleSection title="连接配置" defaultOpen>
          <div className="mb-4">
            <label className={LABEL_CLASS}>模型供应商</label>
            <select title="供应商" value={provider} onChange={(e) => {
              const p = e.target.value;
              setProvider(p);
              const d = PROVIDER_DEFAULTS[p];
              if (d) { setBaseUrl(d.baseUrl); setChatModel(d.chatModel); }
            }} className={INPUT_CLASS}>
              {Object.entries(PROVIDER_DEFAULTS).map(([key, val]) => (
                <option key={key} value={key}>{val.label}</option>
              ))}
            </select>
          </div>
          <div className="grid grid-cols-2 gap-4">
            <Field label="API 地址" value={baseUrl} onChange={setBaseUrl} />
            <Field label="API Key" type="password" value={apiKey} onChange={setApiKey} placeholder={apiKey ? "" : "留空不修改"} />
            <Field label="对话模型" value={chatModel} onChange={setChatModel} />
            <Field label="推理模型" value={reasonerModel} onChange={setReasonerModel} />
          </div>
          <div className="grid grid-cols-4 gap-4 mt-4">
            <Field label="Temperature" type="range" value={temperature} onChange={(v) => setTemperature(Number(v))} rangeMin={0} rangeMax={2} rangeStep={0.1} />
            <Field label="Max Tokens" type="number" value={maxTokens} onChange={(v) => setMaxTokens(Number(v))} />
            <Field label="超时 (秒)" type="number" value={timeout} onChange={(v) => setTimeout_V(Number(v))} />
            <Field label="重试次数" type="number" value={retryCount} onChange={(v) => setRetryCount(Number(v))} />
          </div>
          <label className="flex items-center gap-2 mt-4 text-sm text-slate-600">
            <input type="checkbox" checked={stream} onChange={(e) => setStream(e.target.checked)} />
            流式输出
          </label>
          <div className="flex items-center gap-3 mt-4">
            <button type="button" onClick={handleSave} disabled={saving} className={BTN_PRIMARY}><Save size={14} />保存</button>
            <button type="button" onClick={handleTest} disabled={testing} className={BTN_SECONDARY}>{testing ? <Loader2 size={14} className="animate-spin" /> : null}连接测试</button>
            <button type="button" onClick={handleTestJson} disabled={testing} className={BTN_SECONDARY}>JSON 测试</button>
            <button type="button" onClick={async () => { setTesting(true); setTestResult(""); try { setTestResult(await invoke<string>("test_document_attachment")); } catch (e) { setTestResult(String(e)); setTestError(true); } setTesting(false); }} disabled={testing} className={BTN_SECONDARY}>附件测试</button>
            <button type="button" onClick={async () => { setTesting(true); setTestResult(""); try { setTestResult(await invoke<string>("check_api_key_status")); } catch (e) { setTestResult(String(e)); setTestError(true); } setTesting(false); }} disabled={testing} className={BTN_SECONDARY}>API 状态</button>
          </div>
        </CollapsibleSection>
      )}

      <CollapsibleSection title="模型配置管理" defaultOpen={false}>
        <p className="text-xs text-slate-400 mb-3">管理多个模型配置，快速切换不同的 API 提供商。</p>
        <button type="button" onClick={handleOpenAddProfile} className={`${BTN_SECONDARY} mb-3`}><Plus size={14} />保存当前为配置</button>

        {profilesLoading ? (
          <div className="flex items-center justify-center py-6"><Loader2 size={18} className="animate-spin text-slate-300 dark:text-slate-600" /></div>
        ) : modelProfiles.length === 0 ? (
          <p className="text-xs text-slate-400 py-4">暂无已保存的配置</p>
        ) : (
          <div className="space-y-1">
            {modelProfiles.map((p: any) => (
              <div key={p.id} className="flex items-center justify-between px-3 py-2 border border-slate-100 dark:border-slate-700 hover:bg-slate-50 dark:hover:bg-slate-800">
                <div className="min-w-0 flex items-center gap-3">
                  <span className="text-sm text-slate-700 dark:text-slate-300 truncate">{p.name}</span>
                  <span className="text-xs text-slate-400 dark:text-slate-500">{p.provider}</span>
                  <span className="text-xs text-slate-400 dark:text-slate-500">{p.role}</span>
                  <span className="text-xs text-slate-400 dark:text-slate-500 truncate">{p.model_name}</span>
                </div>
                <div className="flex items-center gap-1 ml-3 shrink-0">
                  <button type="button" onClick={() => handleApplyProfile(p.id)} className="px-2 py-1 text-xs text-slate-500 dark:text-slate-400 hover:text-slate-700 dark:hover:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-800" title="应用"><Play size={12} /> 应用</button>
                  <button type="button" onClick={() => handleDeleteProfile(p.id)} className="px-2 py-1 text-xs text-slate-400 hover:text-red-600 hover:bg-red-50" title="删除"><Trash2 size={12} /></button>
                </div>
              </div>
            ))}
          </div>
        )}

        {showAddProfile && (
          <div className="mt-4 border border-slate-200 dark:border-slate-700 p-4">
            <h4 className="text-sm font-medium text-slate-700 dark:text-slate-300 mb-3">新建配置</h4>
            <div className="grid grid-cols-2 gap-3">
              <Field label="名称" value={npName} onChange={setNpName} placeholder="例如：我的 DeepSeek" required />
              <div>
                <label className={LABEL_CLASS}>提供商</label>
                <select title="提供商" value={npProvider} onChange={(e) => {
                  setNpProvider(e.target.value);
                  const d = PROVIDER_DEFAULTS[e.target.value];
                  if (d) { setNpBaseUrl(d.baseUrl); setNpModelName(d.chatModel); }
                }} className={INPUT_CLASS}>
                  {Object.entries(PROVIDER_DEFAULTS).map(([key, val]) => (
                    <option key={key} value={key}>{val.label}</option>
                  ))}
                </select>
              </div>
              <Field label="API 地址" value={npBaseUrl} onChange={setNpBaseUrl} />
              <Field label="模型名称" value={npModelName} onChange={setNpModelName} />
              <Field label="API Key" type="password" value={npApiKey} onChange={setNpApiKey} placeholder="API Key" />
              <div>
                <label className={LABEL_CLASS}>角色</label>
                <select title="角色" value={npRole} onChange={(e) => setNpRole(e.target.value)} className={INPUT_CLASS}>
                  <option value="chat">对话</option>
                  <option value="reasoner">推理</option>
                </select>
              </div>
            </div>
            <div className="flex items-center gap-2 mt-3">
              <button type="button" onClick={handleSaveProfile} disabled={npSaving} className={BTN_PRIMARY}>{npSaving ? "保存中..." : "保存配置"}</button>
              <button type="button" onClick={() => { setShowAddProfile(false); setNpMsg(""); }} className={BTN_SECONDARY}>取消</button>
            </div>
            {npMsg && <Message type={npMsg.includes("失败") ? "error" : "info"} text={npMsg} />}
          </div>
        )}
      </CollapsibleSection>
    </div>
  );

  const renderKBSection = () => (
    <div className="space-y-8">
      {kbMsg && <Message type={kbMsg.includes("失败") ? "error" : "success"} text={kbMsg} onDismiss={() => setKbMsg("")} />}

      {!currentKB ? (
        <p className="text-sm text-slate-400">请先在顶部选择知识库</p>
      ) : (
        <>
          <CollapsibleSection title="基本信息" defaultOpen>
            <div className="grid grid-cols-2 gap-4">
              <Field label="名称" value={kbName} onChange={setKbName} />
              <div>
                <label className={LABEL_CLASS}>语言</label>
                <select title="语言" value={kbLanguage} onChange={(e) => setKbLanguage(e.target.value)} className={INPUT_CLASS}>
                  <option value="zh">中文</option>
                  <option value="en">English</option>
                  <option value="ja">日本語</option>
                </select>
              </div>
            </div>
          </CollapsibleSection>

          <CollapsibleSection title="审阅模式" defaultOpen>
            <div className="space-y-2">
              {[
                { value: "strict", label: "严格", desc: "所有修改均需手动审阅" },
                { value: "balanced", label: "平衡", desc: "低风险自动通过，中高风险需审阅" },
                { value: "auto", label: "自动", desc: "仅高风险修改需审阅" },
              ].map((mode) => (
                <label key={mode.value} className={`flex items-start gap-3 p-3 border cursor-pointer transition-colors ${kbReviewMode === mode.value ? "border-slate-400 dark:border-slate-500 bg-slate-50 dark:bg-slate-800" : "border-slate-200 dark:border-slate-700 hover:border-slate-300 dark:hover:border-slate-600"}`}>
                  <input type="radio" name="reviewMode" value={mode.value} checked={kbReviewMode === mode.value} onChange={(e) => setKbReviewMode(e.target.value)} className="mt-0.5" />
                  <div>
                    <span className="text-sm text-slate-700 dark:text-slate-300">{mode.label}</span>
                    <p className="text-xs text-slate-400 mt-0.5">{mode.desc}</p>
                  </div>
                </label>
              ))}
            </div>
          </CollapsibleSection>

          <CollapsibleSection title="AI 问答行为" defaultOpen>
            <div className="space-y-3">
              <label className="flex items-start gap-3 p-3 border cursor-pointer transition-colors border-slate-200 dark:border-slate-700 hover:border-slate-300 dark:hover:border-slate-600">
                <input
                  type="checkbox"
                  checked={kbAllowAiGen}
                  onChange={(e) => setKbAllowAiGen(e.target.checked)}
                  className="mt-0.5"
                />
                <div>
                  <span className="text-sm text-slate-700 dark:text-slate-300">允许 AI 自主生成内容</span>
                  <p className="text-xs text-slate-400 mt-0.5">
                    开启后，当知识库中没有相关信息时，AI 可以使用自身知识补充回答并标注来源。
                    关闭后，AI 必须严格基于 Wiki 页面内容回答，无法回答时会明确告知。
                  </p>
                </div>
              </label>
            </div>
          </CollapsibleSection>

          <div>
            <button type="button" onClick={handleSaveKB} disabled={kbSaving} className={BTN_PRIMARY}><Save size={14} />保存知识库设置</button>
          </div>
        </>
      )}
    </div>
  );

  const renderWebSearchSection = () => (
    <div className="space-y-8">
      {wsMsg && <Message type={wsMsg.includes("失败") ? "error" : "success"} text={wsMsg} onDismiss={() => setWsMsg("")} />}

      <CollapsibleSection title="搜索引擎配置" defaultOpen>
        <p className="text-xs text-slate-400 mb-4">在问答界面中开启联网搜索后，AI 将结合搜索结果回答。支持多引擎选择。</p>
        <div className="grid grid-cols-2 gap-4">
          <div>
            <label className={LABEL_CLASS}>搜索引擎</label>
            <select title="搜索引擎" value={wsEngine} onChange={(e) => setWsEngine(e.target.value)} className={INPUT_CLASS}>
              <option value="duckduckgo">DuckDuckGo</option>
              <option value="searxng">SearXNG</option>
              <option value="brave">Brave Search</option>
              <option value="bing">Bing Search</option>
            </select>
          </div>
          <div>
            <label className={LABEL_CLASS}>最大结果数</label>
            <div className="flex items-center gap-3">
              <input type="range" min="1" max="20" value={wsMaxResults} onChange={(e) => setWsMaxResults(Number(e.target.value))} className="flex-1" title="最大结果数" />
              <span className="text-sm text-slate-600 w-8 text-right">{wsMaxResults}</span>
            </div>
          </div>
        </div>

        {wsEngine === "searxng" && (
          <div className="mt-3">
            <Field label="SearXNG 实例地址" value={wsSearxngUrl} onChange={setWsSearxngUrl} placeholder="https://searx.example.com" />
          </div>
        )}
        {wsEngine === "brave" && (
          <div className="mt-3">
            <Field label="Brave Search API Key" type="password" value={wsBraveApiKey} onChange={setWsBraveApiKey} placeholder="BSA..." />
          </div>
        )}
        {wsEngine === "bing" && (
          <div className="grid grid-cols-2 gap-4 mt-3">
            <Field label="Azure API Key" type="password" value={wsBingApiKey} onChange={setWsBingApiKey} />
            <Field label="API Endpoint" value={wsBingEndpoint} onChange={setWsBingEndpoint} placeholder="https://api.bing.microsoft.com/" />
          </div>
        )}
      </CollapsibleSection>

      <div>
        <button type="button" onClick={handleSaveWebSearch} disabled={wsSaving} className={BTN_PRIMARY}><Save size={14} />保存搜索设置</button>
      </div>
    </div>
  );

  const renderDocsSection = () => (
    <div className="space-y-8">
      <CollapsibleSection title="MarkItDown 状态" defaultOpen>
        <p className="text-xs text-slate-400 mb-3">MarkItDown 用于将 PDF、DOCX、HTML 等格式转换为 Markdown 文本，依赖 Python 环境。</p>
        {mdStatus ? (
          <div className="space-y-1.5 mb-3 text-xs">
            <div className="flex items-center gap-2">
              <span className="text-slate-500">Python:</span>
              <span className={mdStatus.python_found ? "text-slate-700" : "text-red-600"}>
                {mdStatus.python_found ? "可用" : "未安装"}
              </span>
            </div>
            <div className="flex items-center gap-2">
              <span className="text-slate-500">MarkItDown:</span>
              <span className={mdStatus.available ? "text-slate-700" : "text-amber-600"}>
                {mdStatus.available ? "可用" : "未安装"}
              </span>
            </div>
            <p className="text-slate-400">{mdStatus.description}</p>
          </div>
        ) : (
          <p className="text-xs text-slate-400 mb-3">检测中...</p>
        )}
        <div className="flex items-center gap-2">
          <button type="button" onClick={checkMdStatus} disabled={mdChecking} className={BTN_SECONDARY}>{mdChecking ? "检测中..." : "重新检测"}</button>
          {mdStatus && !mdStatus.available && (
            <button type="button" onClick={async () => {
              setMdInstalling(true);
              try { await invoke<any>("retry_markitdown_install"); checkMdStatus(); } catch { /* ignore */ }
              setMdInstalling(false);
            }} disabled={mdInstalling} className={BTN_PRIMARY}>{mdInstalling ? "安装中..." : "安装 MarkItDown"}</button>
          )}
        </div>
      </CollapsibleSection>
    </div>
  );

  const renderHealthSection = () => (
    <div className="space-y-6">
      {!currentKB ? (
        <p className="text-sm text-slate-400">请先在顶部选择知识库</p>
      ) : (
        <>
          {/* Sub-tabs */}
          <div className="flex gap-0 border-b border-slate-200 dark:border-slate-700">
            <button
              type="button"
              onClick={() => setHealthSubTab("system")}
              className={`flex items-center gap-1.5 px-4 py-2 text-sm border-b-2 transition-colors ${
                healthSubTab === "system"
                  ? "border-slate-800 dark:border-slate-400 text-slate-800 dark:text-slate-200"
                  : "border-transparent text-slate-500 dark:text-slate-400 hover:text-slate-700 dark:hover:text-slate-300"
              }`}
            >
              <Shield size={14} />
              系统健康
            </button>
            <button
              type="button"
              onClick={() => setHealthSubTab("reconciliation")}
              className={`flex items-center gap-1.5 px-4 py-2 text-sm border-b-2 transition-colors ${
                healthSubTab === "reconciliation"
                  ? "border-slate-800 dark:border-slate-400 text-slate-800 dark:text-slate-200"
                  : "border-transparent text-slate-500 dark:text-slate-400 hover:text-slate-700 dark:hover:text-slate-300"
              }`}
            >
              <CheckCircle2 size={14} />
              数据一致性
            </button>
          </div>

          {/* System Health sub-tab */}
          {healthSubTab === "system" && (
            <div className="space-y-4">
              <div className="flex items-center gap-2">
                <button
                  type="button"
                  onClick={handleRunHealthCheck}
                  disabled={healthRunning}
                  className={BTN_PRIMARY}
                >
                  {healthRunning ? (
                    <Loader2 size={14} className="animate-spin" />
                  ) : (
                    <RefreshCw size={14} />
                  )}
                  运行健康检查
                </button>
              </div>

              {healthError && (
                <Message type="error" text={healthError} onDismiss={() => setHealthError("")} />
              )}

              {healthResult && (
                <>
                  {/* Stats summary */}
                  <div className="grid grid-cols-4 gap-3">
                    {[
                      { label: "总计", value: healthResult?.stats?.total ?? healthResult?.issues?.length ?? 0, color: "text-slate-700 dark:text-slate-300" },
                      { label: "严重", value: healthResult?.stats?.critical ?? 0, color: "text-red-600 dark:text-red-400" },
                      { label: "警告", value: healthResult?.stats?.warning ?? 0, color: "text-orange-500 dark:text-orange-400" },
                      { label: "信息", value: healthResult?.stats?.info ?? 0, color: "text-blue-500 dark:text-blue-400" },
                    ].map((s) => (
                      <div key={s.label} className="border border-slate-200 dark:border-slate-700 p-3 text-center">
                        <div className={`text-lg font-semibold ${s.color}`}>{s.value}</div>
                        <div className="text-xs text-slate-400 mt-0.5">{s.label}</div>
                      </div>
                    ))}
                  </div>

                  {/* Quick fix toolbar */}
                  <div className="flex flex-wrap items-center gap-2 pb-3 border-b border-slate-100 dark:border-slate-700">
                    <span className="text-xs text-slate-500 dark:text-slate-400">
                      <Wrench size={12} className="inline mr-1" />
                      快速修复:
                    </span>
                    {[
                      { label: "修复 Wiki 路径", cmd: "repair_all_wiki_paths" },
                      { label: "重建图谱", cmd: "sync_graph_data" },
                      { label: "重建 Wiki 索引", cmd: "sync_wiki_index_from_markdown" },
                      { label: "恢复检查", cmd: "run_recovery_check" },
                    ].map((fix) => (
                      <button
                        key={fix.cmd}
                        type="button"
                        onClick={() => handleQuickFix(fix.cmd, fix.label)}
                        disabled={quickFixRunning === fix.cmd}
                        className="px-2 py-1 text-xs border border-slate-200 dark:border-slate-600 text-slate-500 dark:text-slate-400 hover:bg-slate-50 dark:hover:bg-slate-800 disabled:opacity-50 transition-colors"
                      >
                        {quickFixRunning === fix.cmd ? (
                          <Loader2 size={12} className="animate-spin inline mr-1" />
                        ) : null}
                        {fix.label}
                      </button>
                    ))}
                  </div>
                  {quickFixResult && (
                    <Message
                      type={quickFixResult.includes("失败") ? "error" : "success"}
                      text={quickFixResult}
                      onDismiss={() => setQuickFixResult("")}
                    />
                  )}

                  {/* Issues list */}
                  <div className="space-y-2">
                    {(() => {
                      const issues = healthResult?.issues || healthResult?.results || [];
                      if (issues.length === 0) {
                        return (
                          <div className="flex items-center gap-2 text-sm text-slate-400 py-4">
                            <CheckCircle2 size={16} className="text-green-500" />
                            未发现问题，知识库运行良好
                          </div>
                        );
                      }
                      return issues.map((item: any, i: number) => {
                        const sev = item?.severity || item?.level || "info";
                        const borderColor =
                          sev === "critical" || sev === "error"
                            ? "border-red-300 dark:border-red-800"
                            : sev === "warning" || sev === "warn"
                            ? "border-orange-300 dark:border-orange-800"
                            : "border-blue-300 dark:border-blue-800";
                        const iconEl =
                          sev === "critical" || sev === "error" ? (
                            <XCircle size={16} className="text-red-500 shrink-0" />
                          ) : sev === "warning" || sev === "warn" ? (
                            <AlertTriangle size={16} className="text-orange-500 shrink-0" />
                          ) : (
                            <Info size={16} className="text-blue-500 shrink-0" />
                          );
                        const sevLabel =
                          sev === "critical" || sev === "error"
                            ? "严重"
                            : sev === "warning" || sev === "warn"
                            ? "警告"
                            : "信息";
                        const sevBadgeClass =
                          sev === "critical" || sev === "error"
                            ? "bg-red-100 dark:bg-red-950 text-red-700 dark:text-red-400"
                            : sev === "warning" || sev === "warn"
                            ? "bg-orange-100 dark:bg-orange-950 text-orange-700 dark:text-orange-400"
                            : "bg-blue-100 dark:bg-blue-950 text-blue-700 dark:text-blue-400";
                        return (
                          <div
                            key={i}
                            className={`border ${borderColor} p-3`}
                          >
                            <div className="flex items-start gap-2">
                              {iconEl}
                              <div className="flex-1 min-w-0">
                                <div className="flex items-center gap-2 mb-1">
                                  {item?.category && (
                                    <span className={`text-xs px-1.5 py-0.5 ${sevBadgeClass}`}>
                                      {item.category}
                                    </span>
                                  )}
                                  <span className={`text-xs px-1.5 py-0.5 ${sevBadgeClass}`}>
                                    {sevLabel}
                                  </span>
                                </div>
                                <p className="text-sm text-slate-700 dark:text-slate-300">
                                  {item?.description || item?.message || item?.title || ""}
                                </p>
                                {item?.fix_suggestion && (
                                  <p className="text-xs text-slate-500 dark:text-slate-400 mt-1">
                                    建议: {item.fix_suggestion}
                                  </p>
                                )}
                              </div>
                              {item?.fixable && (
                                <button
                                  type="button"
                                  onClick={() => handleFixIssue(item)}
                                  disabled={quickFixRunning !== null}
                                  className="px-2 py-1 text-xs border border-slate-200 dark:border-slate-600 text-slate-500 dark:text-slate-400 hover:bg-slate-50 dark:hover:bg-slate-800 disabled:opacity-50 shrink-0"
                                >
                                  <Wrench size={12} className="inline mr-1" />
                                  修复
                                </button>
                              )}
                            </div>
                          </div>
                        );
                      });
                    })()}
                  </div>
                </>
              )}
            </div>
          )}

          {/* Data Reconciliation sub-tab */}
          {healthSubTab === "reconciliation" && (
            <div className="space-y-4">
              <div className="flex items-center gap-2">
                <button
                  type="button"
                  onClick={handleRunReconcile}
                  disabled={reconcileRunning}
                  className={BTN_PRIMARY}
                >
                  {reconcileRunning ? (
                    <Loader2 size={14} className="animate-spin" />
                  ) : (
                    <RefreshCw size={14} />
                  )}
                  运行一致性检查
                </button>
              </div>

              {reconcileError && (
                <Message type="error" text={reconcileError} onDismiss={() => setReconcileError("")} />
              )}

              {reconcileResult && (
                <div className="space-y-3">
                  {/* Summary cards */}
                  <div className="grid grid-cols-3 gap-3">
                    <div className="border border-slate-200 dark:border-slate-700 p-3 text-center">
                      <div className="text-lg font-semibold text-red-600 dark:text-red-400">
                        {reconcileResult?.broken_links ?? reconcileResult?.broken_links_count ?? 0}
                      </div>
                      <div className="text-xs text-slate-400 mt-0.5">损坏链接</div>
                    </div>
                    <div className="border border-slate-200 dark:border-slate-700 p-3 text-center">
                      <div className="text-lg font-semibold text-orange-500 dark:text-orange-400">
                        {reconcileResult?.orphan_sources ?? reconcileResult?.orphan_sources_count ?? 0}
                      </div>
                      <div className="text-xs text-slate-400 mt-0.5">孤立来源</div>
                    </div>
                    <div className="border border-slate-200 dark:border-slate-700 p-3 text-center">
                      <div className="text-lg font-semibold text-blue-500 dark:text-blue-400">
                        {reconcileResult?.inconsistencies ?? reconcileResult?.db_fs_inconsistencies ?? 0}
                      </div>
                      <div className="text-xs text-slate-400 mt-0.5">DB/FS 不一致</div>
                    </div>
                  </div>

                  {/* Broken links */}
                  {(() => {
                    const brokenLinks = reconcileResult?.broken_links_details || reconcileResult?.broken_links || [];
                    if (brokenLinks.length > 0) {
                      return (
                        <div className="space-y-1">
                          <h4 className="text-sm font-medium text-slate-700 dark:text-slate-300">损坏链接</h4>
                          {brokenLinks.map((item: any, i: number) => (
                            <div key={i} className="flex items-center justify-between px-3 py-2 border border-red-200 dark:border-red-800">
                              <div className="text-xs text-slate-600 dark:text-slate-400 min-w-0 truncate">
                                {item?.source || item?.from || item?.url || item?.path || `条目 #${i + 1}`}
                                {item?.target && (
                                  <span className="text-slate-400 mx-2">→</span>
                                )}
                                {item?.target && (
                                  <span className="text-red-500">{item.target}</span>
                                )}
                              </div>
                              {item?.fixable !== false && (
                                <button
                                  type="button"
                                  onClick={() => handleFixReconcileItem("broken_link", item)}
                                  disabled={quickFixRunning !== null}
                                  className="ml-2 px-2 py-0.5 text-xs border border-slate-200 dark:border-slate-600 text-slate-500 dark:text-slate-400 hover:bg-slate-50 dark:hover:bg-slate-800 disabled:opacity-50 shrink-0"
                                >
                                  修复
                                </button>
                              )}
                            </div>
                          ))}
                        </div>
                      );
                    }
                    return null;
                  })()}

                  {/* Orphan sources */}
                  {(() => {
                    const orphans = reconcileResult?.orphan_sources_details || reconcileResult?.orphan_sources || reconcileResult?.orphans || [];
                    if (orphans.length > 0) {
                      return (
                        <div className="space-y-1">
                          <h4 className="text-sm font-medium text-slate-700 dark:text-slate-300">孤立来源</h4>
                          {orphans.map((item: any, i: number) => (
                            <div key={i} className="flex items-center justify-between px-3 py-2 border border-orange-200 dark:border-orange-800">
                              <span className="text-xs text-slate-600 dark:text-slate-400 truncate">
                                {item?.name || item?.title || item?.path || item?.source_id || `条目 #${i + 1}`}
                              </span>
                              {item?.fixable !== false && (
                                <button
                                  type="button"
                                  onClick={() => handleFixReconcileItem("orphan", item)}
                                  disabled={quickFixRunning !== null}
                                  className="ml-2 px-2 py-0.5 text-xs border border-slate-200 dark:border-slate-600 text-slate-500 dark:text-slate-400 hover:bg-slate-50 dark:hover:bg-slate-800 disabled:opacity-50 shrink-0"
                                >
                                  修复
                                </button>
                              )}
                            </div>
                          ))}
                        </div>
                      );
                    }
                    return null;
                  })()}

                  {/* DB/FS inconsistencies */}
                  {(() => {
                    const inconsistencies = reconcileResult?.inconsistency_details || reconcileResult?.db_fs_inconsistencies_details || reconcileResult?.inconsistencies_details || [];
                    if (inconsistencies.length > 0) {
                      return (
                        <div className="space-y-1">
                          <h4 className="text-sm font-medium text-slate-700 dark:text-slate-300">DB/FS 不一致</h4>
                          {inconsistencies.map((item: any, i: number) => (
                            <div key={i} className="flex items-center justify-between px-3 py-2 border border-blue-200 dark:border-blue-800">
                              <span className="text-xs text-slate-600 dark:text-slate-400 truncate">
                                {item?.description || item?.message || item?.path || `条目 #${i + 1}`}
                              </span>
                              {item?.fixable !== false && (
                                <button
                                  type="button"
                                  onClick={() => handleFixReconcileItem("inconsistency", item)}
                                  disabled={quickFixRunning !== null}
                                  className="ml-2 px-2 py-0.5 text-xs border border-slate-200 dark:border-slate-600 text-slate-500 dark:text-slate-400 hover:bg-slate-50 dark:hover:bg-slate-800 disabled:opacity-50 shrink-0"
                                >
                                  修复
                                </button>
                              )}
                            </div>
                          ))}
                        </div>
                      );
                    }
                    return null;
                  })()}

                  {!reconcileResult?.broken_links_details &&
                    !reconcileResult?.broken_links &&
                    !reconcileResult?.orphan_sources_details &&
                    !reconcileResult?.orphan_sources &&
                    !reconcileResult?.orphans &&
                    !reconcileResult?.inconsistency_details &&
                    !reconcileResult?.db_fs_inconsistencies_details &&
                    !reconcileResult?.inconsistencies_details && (
                    <div className="flex items-center gap-2 text-sm text-slate-400 py-4">
                      <CheckCircle2 size={16} className="text-green-500" />
                      数据一致，未发现问题
                    </div>
                  )}
                </div>
              )}
            </div>
          )}
        </>
      )}
    </div>
  );

  const AGENT_DEFS = [
    { name: "SourceIngestAgent", label: "文档解析", icon: FileText, desc: "解析上传的文档，提取结构化知识" },
    { name: "ResolutionAgent", label: "实体消歧", icon: Activity, desc: "将新实体与已有页面进行匹配和消歧" },
    { name: "RelationshipAgent", label: "关系发现", icon: Activity, desc: "发现实体之间的语义关系" },
    { name: "WikiUpdateAgent", label: "Wiki 更新", icon: FileText, desc: "生成 Wiki 页面更新计划" },
    { name: "QueryAgent", label: "智能问答", icon: Bot, desc: "基于 Wiki 知识库的问答检索" },
    { name: "CoordinatorAgent", label: "协调调度", icon: Settings, desc: "负责任务编排和流水线调度" },
    { name: "HealthCheckAgent", label: "健康检查", icon: HeartPulse, desc: "诊断知识库健康状态" },
  ];

  const getAgentStatus = (agentName: string): { status: "idle" | "running" | "error"; taskId?: string } => {
    const runningTask = agentTasks.find((t: any) => t.current_agent === agentName && (t.status === "running" || t.status === "locked" || t.status === "queued" || t.status === "applying"));
    if (runningTask) return { status: "running", taskId: runningTask.id };
    const failedTask = agentTasks.find((t: any) => t.current_agent === agentName && (t.status === "failed" || t.status === "interrupted"));
    if (failedTask) return { status: "error", taskId: failedTask.id };
    return { status: "idle" };
  };

  const renderAgentSection = () => (
    <div className="space-y-6">
      {!currentKB ? (
        <p className="text-sm text-slate-400">请先在顶部选择知识库</p>
      ) : (
        <>
          <div className="flex items-center justify-between">
            <h2 className="text-lg font-semibold text-slate-800 dark:text-slate-200">Agent 状态机</h2>
            <button
              type="button"
              onClick={loadAgentStatus}
              disabled={agentLoading}
              className={BTN_SECONDARY}
            >
              {agentLoading ? <Loader2 size={14} className="animate-spin" /> : <RefreshCw size={14} />}
              刷新
            </button>
          </div>

          {agentError && <Message type="error" text={agentError} onDismiss={() => setAgentError("")} />}

          {/* Agent status grid */}
          <div className="space-y-3">
            {AGENT_DEFS.map((agent) => {
              const { status, taskId } = getAgentStatus(agent.name);
              const statusColors = {
                idle: { bg: "bg-slate-50 dark:bg-slate-800", border: "border-slate-200 dark:border-slate-700", dot: "bg-slate-400", text: "空闲" },
                running: { bg: "bg-green-50 dark:bg-green-950", border: "border-green-200 dark:border-green-800", dot: "bg-green-500 animate-pulse", text: "运行中" },
                error: { bg: "bg-red-50 dark:bg-red-950", border: "border-red-200 dark:border-red-800", dot: "bg-red-500", text: "异常" },
              };
              const colors = statusColors[status];
              return (
                <div key={agent.name} className={`flex items-center gap-4 p-3 border rounded ${colors.bg} ${colors.border}`}>
                  <agent.icon size={18} className="text-slate-500 dark:text-slate-400 shrink-0" />
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2">
                      <span className="text-sm font-medium text-slate-700 dark:text-slate-300">{agent.label}</span>
                      <code className="text-[10px] text-slate-400 bg-slate-100 dark:bg-slate-700 px-1 py-0.5 rounded">{agent.name}</code>
                    </div>
                    <p className="text-xs text-slate-500 dark:text-slate-400 mt-0.5">{agent.desc}</p>
                  </div>
                  <div className="flex items-center gap-2 shrink-0">
                    <span className={`inline-block w-2 h-2 rounded-full ${colors.dot}`} />
                    <span className="text-xs text-slate-500 dark:text-slate-400 w-12">{colors.text}</span>
                    {taskId && (
                      <button
                        type="button"
                        onClick={() => loadAgentEvents(taskId)}
                        className="text-[10px] text-primary hover:underline"
                      >
                        查看日志
                      </button>
                    )}
                  </div>
                </div>
              );
            })}
          </div>

          {/* Agent work logs */}
          {agentEvents.length > 0 && (
            <div className="space-y-2">
              <div className="flex items-center gap-2">
                <Clock size={14} className="text-slate-400" />
                <h3 className="text-sm font-medium text-slate-700 dark:text-slate-300">工作日志</h3>
                <button type="button" onClick={() => setAgentEvents([])} className="text-[10px] text-muted-foreground hover:underline ml-auto">关闭</button>
              </div>
              <div className="border border-slate-200 dark:border-slate-700 max-h-[400px] overflow-y-auto">
                <table className="w-full text-xs">
                  <thead className="bg-slate-50 dark:bg-slate-800 sticky top-0">
                    <tr>
                      <th className="text-left px-3 py-2 text-slate-500 font-medium">时间</th>
                      <th className="text-left px-3 py-2 text-slate-500 font-medium">Agent</th>
                      <th className="text-left px-3 py-2 text-slate-500 font-medium">类型</th>
                      <th className="text-left px-3 py-2 text-slate-500 font-medium">消息</th>
                    </tr>
                  </thead>
                  <tbody>
                    {agentEvents.map((evt: any, i: number) => (
                      <tr key={i} className="border-t border-slate-100 dark:border-slate-700 hover:bg-slate-50 dark:hover:bg-slate-800">
                        <td className="px-3 py-1.5 text-slate-400 whitespace-nowrap font-mono">{evt.created_at?.slice(11, 19) || "-"}</td>
                        <td className="px-3 py-1.5 text-slate-600 dark:text-slate-400">{evt.agent_name || "-"}</td>
                        <td className="px-3 py-1.5 text-slate-500">{evt.event_type || "-"}</td>
                        <td className="px-3 py-1.5 text-slate-600 dark:text-slate-400 max-w-[300px] truncate">{evt.message || "-"}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </div>
          )}

          {/* Recent tasks summary */}
          <div className="space-y-2">
            <h3 className="text-sm font-medium text-slate-700 dark:text-slate-300">最近任务</h3>
            {agentTasks.length === 0 ? (
              <p className="text-xs text-slate-400 py-2">暂无任务记录</p>
            ) : (
              <div className="border border-slate-200 dark:border-slate-700 max-h-[300px] overflow-y-auto">
                <table className="w-full text-xs">
                  <thead className="bg-slate-50 dark:bg-slate-800 sticky top-0">
                    <tr>
                      <th className="text-left px-3 py-2 text-slate-500 font-medium">任务 ID</th>
                      <th className="text-left px-3 py-2 text-slate-500 font-medium">类型</th>
                      <th className="text-left px-3 py-2 text-slate-500 font-medium">Agent</th>
                      <th className="text-left px-3 py-2 text-slate-500 font-medium">状态</th>
                      <th className="text-left px-3 py-2 text-slate-500 font-medium">操作</th>
                    </tr>
                  </thead>
                  <tbody>
                    {agentTasks.slice(0, 20).map((t: any) => {
                      const statusColors: Record<string, string> = {
                        completed: "text-green-600", running: "text-blue-600", locked: "text-blue-600",
                        failed: "text-red-600", cancelled: "text-slate-400", interrupted: "text-orange-500",
                        queued: "text-slate-500", applying: "text-purple-600",
                      };
                      return (
                        <tr key={t.id} className="border-t border-slate-100 dark:border-slate-700 hover:bg-slate-50 dark:hover:bg-slate-800">
                          <td className="px-3 py-1.5 text-slate-400 font-mono text-[10px]">{t.id?.slice(0, 8)}...</td>
                          <td className="px-3 py-1.5 text-slate-600 dark:text-slate-400">{t.task_type}</td>
                          <td className="px-3 py-1.5 text-slate-500">{t.current_agent || "-"}</td>
                          <td className={`px-3 py-1.5 font-medium ${statusColors[t.status] || "text-slate-500"}`}>{t.status}</td>
                          <td className="px-3 py-1.5">
                            <button type="button" onClick={() => loadAgentEvents(t.id)} className="text-primary text-[10px] hover:underline">日志</button>
                          </td>
                        </tr>
                      );
                    })}
                  </tbody>
                </table>
              </div>
            )}
          </div>
        </>
      )}
    </div>
  );

  const renderAppearanceSection = () => (

      <div className="space-y-6">
        <h2 className="text-lg font-semibold text-slate-800 dark:text-slate-200">主题外观</h2>
        <p className="text-xs text-slate-500 dark:text-slate-400 -mt-4">选择 LLMWiki 的界面主题配色</p>
        <div className="grid grid-cols-3 gap-3">
          {([
            { key: "dark" as const, icon: Moon, label: "深色", desc: "护眼暗色界面" },
            { key: "light" as const, icon: Sun, label: "浅色", desc: "明亮简洁界面" },
            { key: "light" as const, icon: Monitor, label: "跟随系统", desc: "自动匹配系统", hidden: true },
          ]).filter(t => !t.hidden).map(({ key, icon: Icon, label, desc }) => (
            <button
              key={key}
              type="button"
              onClick={() => setTheme(key)}
              className={`flex flex-col items-center gap-2 p-4 border-2 rounded-lg transition-all ${
                theme === key
                  ? "border-[#8b5cf6] bg-[#8b5cf6]/5 ring-1 ring-[#8b5cf6]/20"
                  : "border-slate-200 dark:border-slate-700 hover:border-slate-300 dark:hover:border-slate-600"
              }`}
            >
              <Icon size={24} className={theme === key ? "text-[#8b5cf6]" : "text-slate-400 dark:text-slate-500"} />
              <span className={`text-sm font-medium ${theme === key ? "text-[#8b5cf6]" : "text-slate-700 dark:text-slate-300"}`}>
                {label}
              </span>
              <span className="text-[10px] text-slate-400 dark:text-slate-500">{desc}</span>
            </button>
          ))}
        </div>
      </div>
  );

  const renderDangerSection = () => (
    <div className="space-y-6">
      <div className="border border-red-200 p-6">
        <h3 className="text-sm font-medium text-red-600 dark:text-red-400 mb-2">重置所有数据</h3>
        <p className="text-xs text-slate-500 dark:text-slate-400 mb-4">清空数据库中的所有记录并删除工作区文件。此操作不可恢复。</p>
        <button type="button" onClick={handleResetAll} className={BTN_DANGER}><Trash2 size={14} />重置所有数据</button>
      </div>
    </div>
  );

  const renderContent = () => {
    switch (activeSection) {
      case "model": return renderModelSection();
      case "kb": return renderKBSection();
      case "websearch": return renderWebSearchSection();
      case "docs": return renderDocsSection();
      case "vdb": return <Suspense fallback={<PanelFallback />}><VdbSettingsPanel kbId={currentKB?.id || ""} /></Suspense>;
      case "appearance": return renderAppearanceSection();
      case "danger": return renderDangerSection();
      case "health": return renderHealthSection();
      case "token": return <Suspense fallback={<PanelFallback />}><TokenMonitoringPanel /></Suspense>;
      case "agent": return renderAgentSection();
      case "agent_manage": return <Suspense fallback={<PanelFallback />}><AgentManager /></Suspense>;
      case "skill_platform": return <Suspense fallback={<PanelFallback />}><SkillPlatform /></Suspense>;
    }
  };

  return (
    <div className="flex h-full">
      {/* Left nav */}
      <nav className="w-44 shrink-0 border-r border-slate-200 dark:border-slate-800 bg-slate-50/50 dark:bg-slate-900/50 pt-6">
        <div className="px-4 mb-4">
          <div className="flex items-center gap-2">
            <Settings size={16} className="text-slate-400 dark:text-slate-500" />
            <span className="text-sm font-medium text-slate-700 dark:text-slate-300">设置</span>
          </div>
        </div>
        {NAV_ITEMS.map((item) => (
          <button
            type="button"
            key={item.key}
            onClick={() => setActiveSection(item.key)}
            className={`flex items-center gap-2.5 w-full px-4 py-2 text-sm transition-colors ${
              activeSection === item.key
                ? "bg-slate-200/70 dark:bg-slate-700/70 text-slate-900 dark:text-slate-100"
                : "text-slate-500 dark:text-slate-400 hover:text-slate-700 dark:hover:text-slate-300 hover:bg-slate-100/50 dark:hover:bg-slate-800/50"
            }`}
          >
            <item.icon size={15} />
            {item.label}
          </button>
        ))}
      </nav>

      {/* Right content */}
      <div className="flex-1 overflow-y-auto p-8">
        {renderContent()}
      </div>
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
        {open ? <ChevronDown size={14} className="text-slate-400 dark:text-slate-500" /> : <ChevronRight size={14} className="text-slate-400 dark:text-slate-500" />}
        <span className="text-sm font-medium text-slate-700 dark:text-slate-300">{title}</span>
        <span className="text-xs text-slate-400 opacity-0 group-hover:opacity-100 transition-opacity ml-1">
          {open ? "收起" : "展开"}
        </span>
      </button>
      {open && <div className="pl-5">{children}</div>}
    </div>
  );
}

function Field({
  label,
  type = "text",
  value,
  onChange,
  placeholder,
  rangeMin,
  rangeMax,
  rangeStep,
  required,
}: {
  label: string;
  type?: string;
  value: string | number;
  onChange: (v: string) => void;
  placeholder?: string;
  rangeMin?: number;
  rangeMax?: number;
  rangeStep?: number;
  required?: boolean;
}) {
  return (
    <div>
      <label className={LABEL_CLASS}>
        {label}
        {type === "range" && value !== undefined && (
          <span className="ml-1 text-slate-400">({value})</span>
        )}
      </label>
      {type === "range" ? (
        <input type="range" min={rangeMin} max={rangeMax} step={rangeStep} value={value} onChange={(e) => onChange(e.target.value)} className="w-full" title={label} />
      ) : (
        <input
          type={type}
          value={value}
          onChange={(e) => onChange(e.target.value)}
          placeholder={placeholder}
          className={INPUT_CLASS}
          required={required}
        />
      )}
    </div>
  );
}

function Message({ type, text, onDismiss }: { type: "success" | "error" | "info"; text: string; onDismiss?: () => void }) {
  return (
    <div className={`px-3 py-2 text-xs flex items-center justify-between ${
      type === "error" ? "bg-red-50 dark:bg-red-950 text-red-700 dark:text-red-400" : type === "success" ? "bg-slate-50 dark:bg-slate-800 text-slate-700 dark:text-slate-300 border border-slate-200 dark:border-slate-700" : "bg-slate-50 dark:bg-slate-800 text-slate-600 dark:text-slate-400"
    }`}>
      <span>{text}</span>
      {onDismiss && <button type="button" onClick={onDismiss} className="text-slate-400 hover:text-slate-600 ml-3">×</button>}
    </div>
  );
}
