import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useAgentSkillStore } from "@/stores/useAgentSkillStore";
import type { AgentDefinition } from "@/types/agent";
import { HARDCODED_AGENTS, TRIGGER_EVENTS, AGENT_ROLES } from "@/types/agent";
import { Loader2, Plus, Search, Lock, Trash2, Save, X, Bot } from "lucide-react";

const INPUT_CLASS =
  "w-full px-3 py-1.5 text-sm border border-[var(--border)] bg-[var(--card)] text-[var(--foreground)] outline-none focus:border-[var(--primary)] placeholder:text-[var(--muted-foreground)] transition-colors";
const LABEL_CLASS = "block text-xs text-[var(--muted-foreground)] mb-1";
const BTN_PRIMARY =
  "inline-flex items-center gap-1.5 px-4 py-1.5 text-sm bg-[var(--primary)] text-[var(--primary-foreground)] hover:bg-[var(--primary-hover)] transition-colors disabled:opacity-50";
const BTN_SECONDARY =
  "inline-flex items-center gap-1.5 px-4 py-1.5 text-sm border border-[var(--border)] text-[var(--muted-foreground)] hover:bg-[var(--card-hover)] transition-colors disabled:opacity-50";
const BTN_DANGER =
  "inline-flex items-center gap-1.5 px-4 py-1.5 text-sm border border-red-400/30 text-red-500 hover:bg-red-500/10 transition-colors disabled:opacity-50";

function emptyAgent(): AgentDefinition {
  return {
    id: "",
    name: "",
    role: "custom",
    trigger_event: "manual",
    system_prompt: "",
    allowed_skills: [],
    status: "active",
    max_depth: 5,
    timeout_secs: 120,
    metadata_json: {},
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
  };
}

export default function AgentManager() {
  const store = useAgentSkillStore();
  const [search, setSearch] = useState("");
  const [editing, setEditing] = useState<AgentDefinition | null>(null);
  const [isCreating, setIsCreating] = useState(false);
  const [saving, setSaving] = useState(false);
  const [deleteConfirm, setDeleteConfirm] = useState<string | null>(null);
  const [localError, setLocalError] = useState<string | null>(null);

  const loadAgents = async () => {
    store.setLoading(true);
    store.setError(null);
    try {
      const list = await invoke<AgentDefinition[]>("list_agent_definitions");
      store.setAgents(list);
    } catch (e) {
      store.setError(String(e));
    } finally {
      store.setLoading(false);
    }
  };

  const loadSkills = async () => {
    try {
      const list = await invoke<{ name: string }[]>("list_skill_definitions");
      store.setSkills(list.map((s: any) => ({ ...s, parameter_schema: typeof s.parameter_schema === "string" ? JSON.parse(s.parameter_schema) : s.parameter_schema, allowed_skills: typeof s.allowed_skills === "string" ? JSON.parse(s.allowed_skills || "[]") : s.allowed_skills })));
    } catch (_) {
      // skills load is best-effort for AgentManager
    }
  };

  useEffect(() => {
    loadAgents();
    loadSkills();

    const unlisten = listen<{ action: string; agent_name: string }>("agent-definition-changed", () => {
      loadAgents();
    });
    return () => { unlisten.then((fn) => fn()); };
  }, []);

  const filtered = store.agents.filter(
    (a) =>
      !search ||
      a.name.toLowerCase().includes(search.toLowerCase()) ||
      a.role.toLowerCase().includes(search.toLowerCase())
  );

  const isHardcoded = (name: string) =>
    (HARDCODED_AGENTS as readonly string[]).includes(name);

  const startCreate = () => {
    setIsCreating(true);
    setEditing(emptyAgent());
    setLocalError(null);
  };

  const startEdit = (agent: AgentDefinition) => {
    if (isHardcoded(agent.name)) return;
    setIsCreating(false);
    setEditing({ ...agent });
    setLocalError(null);
  };

  const cancelEdit = () => {
    setEditing(null);
    setIsCreating(false);
    setLocalError(null);
  };

  const handleSave = async () => {
    if (!editing) return;
    setSaving(true);
    setLocalError(null);
    try {
      const payload = {
        ...editing,
        id: editing.id || crypto.randomUUID(),
        created_at: editing.created_at || new Date().toISOString(),
        updated_at: new Date().toISOString(),
      };

      if (isCreating) {
        await invoke("create_agent_definition", { definition: payload });
      } else {
        await invoke("update_agent_definition", { id: payload.id, patch: payload });
      }
      setEditing(null);
      setIsCreating(false);
      await loadAgents();
    } catch (e) {
      setLocalError(String(e));
    } finally {
      setSaving(false);
    }
  };

  const handleDelete = async (id: string) => {
    try {
      await invoke("delete_agent_definition", { id });
      setDeleteConfirm(null);
      if (editing?.id === id) {
        setEditing(null);
        setIsCreating(false);
      }
      await loadAgents();
    } catch (e) {
      setLocalError(String(e));
    }
  };

  const selectedId = editing?.id || "";

  return (
    <div className="flex h-full">
      {/* Left: Agent list */}
      <div className="w-60 shrink-0 border-r border-[var(--border)] flex flex-col">
        <div className="p-3 space-y-2">
          <div className="relative">
            <Search size={14} className="absolute left-2 top-1/2 -translate-y-1/2 text-[var(--muted-foreground)]" />
            <input
              className={`${INPUT_CLASS} pl-7`}
              placeholder="搜索 Agent..."
              value={search}
              onChange={(e) => setSearch(e.target.value)}
            />
          </div>
          <button type="button" onClick={startCreate} className={`${BTN_PRIMARY} w-full justify-center`}>
            <Plus size={14} /> 新建 Agent
          </button>
        </div>

        <div className="flex-1 overflow-y-auto">
          {store.loading && (
            <div className="flex justify-center py-8">
              <Loader2 size={18} className="animate-spin text-[var(--muted-foreground)]" />
            </div>
          )}
          {!store.loading && filtered.length === 0 && (
            <p className="text-xs text-[var(--muted-foreground)] text-center py-8 px-3">
              {search ? "无匹配结果" : "暂无用户自定义 Agent"}
            </p>
          )}
          {filtered.map((agent) => {
            const locked = isHardcoded(agent.name);
            return (
              <button
                key={agent.id}
                type="button"
                onClick={() => startEdit(agent)}
                className={`w-full text-left px-3 py-2.5 border-b border-[var(--border)] hover:bg-[var(--card-hover)] transition-colors ${
                  selectedId === agent.id ? "bg-[var(--card-hover)]" : ""
                } ${locked ? "opacity-80" : ""}`}
              >
                <div className="flex items-center gap-2">
                  {locked ? (
                    <Lock size={12} className="text-[var(--muted-foreground)] shrink-0" />
                  ) : (
                    <Bot size={14} className="text-[var(--muted-foreground)] shrink-0" />
                  )}
                  <span className="text-sm font-medium text-[var(--foreground)] truncate">
                    {agent.name}
                  </span>
                  {agent.name === "AdminAgent" && (
                    <span className="text-[10px] text-amber-600 bg-amber-100 dark:bg-amber-900/30 px-1 py-0.5 rounded shrink-0" title="内核主控，不可修改">
                      &#128274; 内核主控不可修改
                    </span>
                  )}
                </div>
                <div className="flex items-center gap-2 mt-1">
                  <span className="text-[10px] text-[var(--muted-foreground)] bg-[var(--card)] px-1.5 py-0.5 rounded">
                    {AGENT_ROLES.find((r) => r.value === agent.role)?.label || agent.role}
                  </span>
                  <span
                    className={`inline-block w-1.5 h-1.5 rounded-full ${
                      agent.status === "active" ? "bg-green-500" : "bg-gray-400"
                    }`}
                  />
                </div>
              </button>
            );
          })}
        </div>
      </div>

      {/* Right: Editor */}
      <div className="flex-1 overflow-y-auto p-6">
        {store.error && (
          <div className="mb-4 p-3 border border-red-400/30 bg-red-50 dark:bg-red-950 text-red-600 dark:text-red-400 text-xs">
            {store.error}
            <button type="button" onClick={loadAgents} className="ml-2 underline">重试</button>
          </div>
        )}
        {localError && (
          <div className="mb-4 p-3 border border-red-400/30 bg-red-50 dark:bg-red-950 text-red-600 dark:text-red-400 text-xs">
            {localError}
            <button type="button" onClick={() => setLocalError(null)} className="ml-2 underline">关闭</button>
          </div>
        )}

        {!editing ? (
          <div className="flex flex-col items-center justify-center h-full text-[var(--muted-foreground)]">
            <Bot size={40} strokeWidth={1} />
            <p className="mt-4 text-sm">选择一个 Agent 查看或编辑</p>
            <p className="text-xs mt-1">系统内置 Agent 带有锁定标记，不可修改</p>
          </div>
        ) : (
          <div className="space-y-4 max-w-lg">
            <div className="flex items-center justify-between">
              <h2 className="text-lg font-semibold text-[var(--foreground)]">
                {isCreating ? "新建 Agent" : `编辑: ${editing.name}`}
              </h2>
              {isHardcoded(editing.name) && (
                <span className="text-xs text-[var(--muted-foreground)] flex items-center gap-1">
                  <Lock size={12} /> 系统内置
                </span>
              )}
            </div>

            <div>
              <label className={LABEL_CLASS}>名称</label>
              <input
                className={INPUT_CLASS}
                value={editing.name}
                onChange={(e) => setEditing({ ...editing, name: e.target.value })}
                disabled={isHardcoded(editing.name)}
                placeholder="Agent 名称"
              />
            </div>

            <div>
              <label className={LABEL_CLASS}>角色</label>
              <select
                className={INPUT_CLASS}
                value={editing.role}
                onChange={(e) => setEditing({ ...editing, role: e.target.value })}
                disabled={isHardcoded(editing.name)}
              >
                {AGENT_ROLES.map((r) => (
                  <option key={r.value} value={r.value}>{r.label}</option>
                ))}
              </select>
            </div>

            <div>
              <label className={LABEL_CLASS}>触发时机</label>
              <select
                className={INPUT_CLASS}
                value={editing.trigger_event}
                onChange={(e) => setEditing({ ...editing, trigger_event: e.target.value })}
                disabled={isHardcoded(editing.name)}
              >
                {TRIGGER_EVENTS.map((t) => (
                  <option key={t.value} value={t.value}>{t.label}</option>
                ))}
              </select>
            </div>

            <div>
              <label className={LABEL_CLASS}>系统提示词</label>
              <textarea
                className={`${INPUT_CLASS} font-mono text-xs`}
                rows={12}
                value={editing.system_prompt}
                onChange={(e) => setEditing({ ...editing, system_prompt: e.target.value })}
                disabled={isHardcoded(editing.name)}
                placeholder="编写 Agent 的系统提示词..."
              />
            </div>

            <div className="grid grid-cols-2 gap-4">
              <div>
                <label className={LABEL_CLASS}>最大深度</label>
                <input
                  type="number"
                  className={INPUT_CLASS}
                  value={editing.max_depth}
                  min={1}
                  max={10}
                  onChange={(e) =>
                    setEditing({ ...editing, max_depth: parseInt(e.target.value) || 5 })
                  }
                />
              </div>
              <div>
                <label className={LABEL_CLASS}>超时时间 (秒)</label>
                <input
                  type="number"
                  className={INPUT_CLASS}
                  value={editing.timeout_secs}
                  min={1}
                  max={600}
                  onChange={(e) =>
                    setEditing({ ...editing, timeout_secs: parseInt(e.target.value) || 120 })
                  }
                />
              </div>
            </div>

            <div>
              <label className={LABEL_CLASS}>状态</label>
              <select
                className={INPUT_CLASS}
                value={editing.status}
                onChange={(e) =>
                  setEditing({ ...editing, status: e.target.value as "active" | "disabled" | "error" })
                }
              >
                <option value="active">激活</option>
                <option value="disabled">禁用</option>
                <option value="error">异常</option>
              </select>
            </div>

            <div className="flex items-center gap-3 pt-2">
              <button type="button" onClick={handleSave} disabled={saving || isHardcoded(editing.name)} className={BTN_PRIMARY}>
                {saving ? <Loader2 size={14} className="animate-spin" /> : <Save size={14} />}
                保存
              </button>
              <button type="button" onClick={cancelEdit} className={BTN_SECONDARY}>
                <X size={14} /> 取消
              </button>
              {!isCreating && !isHardcoded(editing.name) && (
                <button
                  type="button"
                  onClick={() => setDeleteConfirm(editing.id)}
                  className={`${BTN_DANGER} ml-auto`}
                >
                  <Trash2 size={14} /> 删除
                </button>
              )}
            </div>
          </div>
        )}

        {/* Delete confirmation modal */}
        {deleteConfirm && (
          <div className="fixed inset-0 bg-black/30 flex items-center justify-center z-50">
            <div className="bg-[var(--card)] border border-[var(--border)] p-6 max-w-sm">
              <h3 className="text-sm font-medium text-[var(--foreground)] mb-2">确认删除</h3>
              <p className="text-xs text-[var(--muted-foreground)] mb-4">
                删除后不可恢复。确定要删除此 Agent 吗？
              </p>
              <div className="flex gap-2 justify-end">
                <button type="button" onClick={() => setDeleteConfirm(null)} className={BTN_SECONDARY}>
                  取消
                </button>
                <button
                  type="button"
                  onClick={() => handleDelete(deleteConfirm)}
                  className={BTN_DANGER}
                >
                  确认删除
                </button>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
