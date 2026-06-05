import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useAgentSkillStore } from "@/stores/useAgentSkillStore";
import type { SkillDefinition } from "@/types/skill";
import { HARDCODED_SKILLS, SKILL_TYPES } from "@/types/skill";
import { validateCodeBodyJson } from "@/validation/skillSchema";
import { Loader2, Plus, Search, Lock, Trash2, Save, X, Wrench, Code2, Settings, Play, CheckCircle, AlertTriangle } from "lucide-react";

const INPUT_CLASS =
  "w-full px-3 py-1.5 text-sm border border-[var(--border)] bg-[var(--card)] text-[var(--foreground)] outline-none focus:border-[var(--primary)] placeholder:text-[var(--muted-foreground)] transition-colors";
const LABEL_CLASS = "block text-xs text-[var(--muted-foreground)] mb-1";
const BTN_PRIMARY =
  "inline-flex items-center gap-1.5 px-4 py-1.5 text-sm bg-[var(--primary)] text-[var(--primary-foreground)] hover:bg-[var(--primary-hover)] transition-colors disabled:opacity-50";
const BTN_SECONDARY =
  "inline-flex items-center gap-1.5 px-4 py-1.5 text-sm border border-[var(--border)] text-[var(--muted-foreground)] hover:bg-[var(--card-hover)] transition-colors disabled:opacity-50";
const BTN_DANGER =
  "inline-flex items-center gap-1.5 px-4 py-1.5 text-sm border border-red-400/30 text-red-500 hover:bg-red-500/10 transition-colors disabled:opacity-50";

// Mock 运行测试按钮组件
function MockTestButton({ skillType, codeBody }: { skillType: string; codeBody: string }) {
  const [running, setRunning] = useState(false);
  const [result, setResult] = useState<{ success: boolean; output: string; error: string | null; duration_ms: number } | null>(null);

  const handleMockRun = async () => {
    setRunning(true);
    setResult(null);
    try {
      const res = await invoke<any>("execute_skill_mock", {
        skillType,
        codeBody,
        params: { text: "hello", input: "测试输入" },
      });
      setResult(res);
    } catch (e) {
      setResult({ success: false, output: "", error: String(e), duration_ms: 0 });
    }
    setRunning(false);
  };

  return (
    <div className="mt-3 border border-[var(--border)] rounded p-3">
      <div className="flex items-center gap-2">
        <button
          type="button"
          onClick={handleMockRun}
          disabled={running}
          className="inline-flex items-center gap-1.5 px-3 py-1 text-xs bg-green-600 text-white hover:bg-green-700 transition-colors disabled:opacity-50"
        >
          {running ? <Loader2 size={12} className="animate-spin" /> : <Play size={12} />}
          Mock 运行测试
        </button>
        <span className="text-[10px] text-[var(--muted-foreground)]">沙箱超时 5s，结果仅用于验证</span>
      </div>
      {result && (
        <div className={`mt-2 p-2 rounded text-xs ${result.success ? "bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-800" : "bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800"}`}>
          <div className="flex items-center gap-1.5 mb-1">
            {result.success ? <CheckCircle size={12} className="text-green-600" /> : <AlertTriangle size={12} className="text-red-500" />}
            <span className={result.success ? "text-green-700 dark:text-green-400" : "text-red-600 dark:text-red-400"}>
              {result.success ? "执行成功" : "执行失败"}
            </span>
            <span className="text-[var(--muted-foreground)]">({result.duration_ms}ms)</span>
          </div>
          {result.output && (
            <pre className="whitespace-pre-wrap font-mono text-[var(--foreground-dim)] bg-[var(--card)] p-2 max-h-[200px] overflow-y-auto">{result.output.slice(0, 2000)}</pre>
          )}
          {result.error && (
            <p className="text-red-500 mt-1">{result.error}</p>
          )}
        </div>
      )}
    </div>
  );
}

function emptySkill(): SkillDefinition {
  return {
    id: "",
    name: "",
    description: "",
    code_body: '{"type":"prompt","system_prompt":"","user_prompt_template":"{{input}}"}',
    parameter_schema: { type: "object", properties: {} },
    skill_type: "prompt",
    status: "active",
    metadata_json: {},
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
  };
}

export default function SkillPlatform() {
  const store = useAgentSkillStore();
  const [search, setSearch] = useState("");
  const [editing, setEditing] = useState<SkillDefinition | null>(null);
  const [isCreating, setIsCreating] = useState(false);
  const [saving, setSaving] = useState(false);
  const [deleteConfirm, setDeleteConfirm] = useState<string | null>(null);
  const [localError, setLocalError] = useState<string | null>(null);
  const [activeTab, setActiveTab] = useState<"config" | "code">("config");
  const [schemaError, setSchemaError] = useState<string | null>(null);

  const loadSkills = async () => {
    store.setLoading(true);
    store.setError(null);
    try {
      const list = await invoke<any[]>("list_skill_definitions");
      store.setSkills(
        list.map((s) => ({
          ...s,
          parameter_schema: typeof s.parameter_schema === "string"
            ? JSON.parse(s.parameter_schema)
            : s.parameter_schema,
        }))
      );
    } catch (e) {
      store.setError(String(e));
    } finally {
      store.setLoading(false);
    }
  };

  useEffect(() => {
    loadSkills();
    const unlisten = listen<{ action: string; skill_name: string }>(
      "skill-definition-changed",
      () => {
        loadSkills();
      }
    );
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const filtered = store.skills.filter(
    (s) =>
      !search ||
      s.name.toLowerCase().includes(search.toLowerCase()) ||
      s.description.toLowerCase().includes(search.toLowerCase())
  );

  const isHardcoded = (name: string) =>
    (HARDCODED_SKILLS as readonly string[]).includes(name);

  const startCreate = () => {
    setIsCreating(true);
    setEditing(emptySkill());
    setLocalError(null);
    setActiveTab("config");
  };

  const startEdit = (skill: SkillDefinition) => {
    if (isHardcoded(skill.name)) return;
    setIsCreating(false);
    setEditing({ ...skill });
    setLocalError(null);
    setActiveTab("config");
    setSchemaError(null);
  };

  const cancelEdit = () => {
    setEditing(null);
    setIsCreating(false);
    setLocalError(null);
    setSchemaError(null);
  };

  const handleSave = async () => {
    if (!editing) return;
    setSaving(true);
    setLocalError(null);
    try {
      // Validate code_body JSON
      const codeCheck = validateCodeBodyJson(editing.code_body);
      if (!codeCheck.valid) {
        setLocalError(codeCheck.error ?? null);
        setSaving(false);
        return;
      }

      // Validate parameter_schema
      await invoke("validate_skill_schema", {
        schemaJson: JSON.stringify(editing.parameter_schema),
      });

      const payload = {
        ...editing,
        id: editing.id || crypto.randomUUID(),
        created_at: editing.created_at || new Date().toISOString(),
        updated_at: new Date().toISOString(),
      };

      if (isCreating) {
        await invoke("create_skill_definition", { definition: payload });
      } else {
        await invoke("update_skill_definition", { id: payload.id, patch: payload });
      }
      setEditing(null);
      setIsCreating(false);
      await loadSkills();
    } catch (e) {
      setLocalError(String(e));
    } finally {
      setSaving(false);
    }
  };

  const handleDelete = async (id: string) => {
    try {
      await invoke("delete_skill_definition", { id });
      setDeleteConfirm(null);
      if (editing?.id === id) {
        setEditing(null);
        setIsCreating(false);
      }
      await loadSkills();
    } catch (e) {
      setLocalError(String(e));
    }
  };

  const handleValidateSchema = async () => {
    if (!editing) return;
    setSchemaError(null);
    try {
      await invoke("validate_skill_schema", {
        schemaJson: JSON.stringify(editing.parameter_schema),
      });
    } catch (e) {
      setSchemaError(String(e));
    }
  };

  const selectedId = editing?.id || "";

  // Pretty-print code_body for editing
  const codeBodyDisplay = (() => {
    if (!editing) return "";
    try {
      return JSON.stringify(JSON.parse(editing.code_body), null, 2);
    } catch {
      return editing.code_body;
    }
  })();

  const schemaDisplay = (() => {
    if (!editing) return "";
    try {
      return JSON.stringify(editing.parameter_schema, null, 2);
    } catch {
      return "{}";
    }
  })();

  return (
    <div className="flex h-full">
      {/* Left: Skill list */}
      <div className="w-60 shrink-0 border-r border-[var(--border)] flex flex-col">
        <div className="p-3 space-y-2">
          <div className="relative">
            <Search size={14} className="absolute left-2 top-1/2 -translate-y-1/2 text-[var(--muted-foreground)]" />
            <input
              className={`${INPUT_CLASS} pl-7`}
              placeholder="搜索 Skill..."
              value={search}
              onChange={(e) => setSearch(e.target.value)}
            />
          </div>
          <button type="button" onClick={startCreate} className={`${BTN_PRIMARY} w-full justify-center`}>
            <Plus size={14} /> 新建 Skill
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
              {search ? "无匹配结果" : "暂无用户自定义 Skill"}
            </p>
          )}
          {filtered.map((skill) => {
            const locked = isHardcoded(skill.name);
            return (
              <button
                key={skill.id}
                type="button"
                onClick={() => startEdit(skill)}
                className={`w-full text-left px-3 py-2.5 border-b border-[var(--border)] hover:bg-[var(--card-hover)] transition-colors ${
                  selectedId === skill.id ? "bg-[var(--card-hover)]" : ""
                } ${locked ? "opacity-80" : ""}`}
              >
                <div className="flex items-center gap-2">
                  {locked ? (
                    <Lock size={12} className="text-[var(--muted-foreground)] shrink-0" />
                  ) : (
                    <Wrench size={14} className="text-[var(--muted-foreground)] shrink-0" />
                  )}
                  <span className="text-sm font-medium text-[var(--foreground)] truncate">
                    {skill.name}
                  </span>
                </div>
                <div className="flex items-center gap-2 mt-1">
                  <span className="text-[10px] text-[var(--muted-foreground)] bg-[var(--card)] px-1.5 py-0.5 rounded">
                    {SKILL_TYPES.find((t) => t.value === skill.skill_type)?.label || skill.skill_type}
                  </span>
                  <span className="text-[10px] text-[var(--muted-foreground)] truncate">
                    {skill.description.slice(0, 30)}
                  </span>
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
            <button type="button" onClick={loadSkills} className="ml-2 underline">重试</button>
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
            <Wrench size={40} strokeWidth={1} />
            <p className="mt-4 text-sm">选择一个 Skill 查看或编辑</p>
            <p className="text-xs mt-1">系统内置 Skill 带有锁定标记，不可修改</p>
          </div>
        ) : (
          <div className="space-y-4 max-w-xl">
            <div className="flex items-center justify-between">
              <h2 className="text-lg font-semibold text-[var(--foreground)]">
                {isCreating ? "新建 Skill" : `编辑: ${editing.name}`}
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
                placeholder="Skill 名称"
              />
            </div>

            <div>
              <label className={LABEL_CLASS}>描述</label>
              <input
                className={INPUT_CLASS}
                value={editing.description}
                onChange={(e) => setEditing({ ...editing, description: e.target.value })}
                placeholder="描述此 Skill 的功能"
              />
            </div>

            <div>
              <label className={LABEL_CLASS}>Skill 类型</label>
              <select
                className={INPUT_CLASS}
                value={editing.skill_type}
                onChange={(e) =>
                  setEditing({ ...editing, skill_type: e.target.value as SkillDefinition["skill_type"] })
                }
                disabled={isHardcoded(editing.name)}
              >
                {SKILL_TYPES.map((t) => (
                  <option key={t.value} value={t.value}>{t.label}</option>
                ))}
              </select>
            </div>

            <div>
              <label className={LABEL_CLASS}>状态</label>
              <select
                className={INPUT_CLASS}
                value={editing.status}
                onChange={(e) =>
                  setEditing({ ...editing, status: e.target.value as "active" | "disabled" })
                }
              >
                <option value="active">激活</option>
                <option value="disabled">禁用</option>
              </select>
            </div>

            {/* Tab bar: Config / Code */}
            <div className="flex border-b border-[var(--border)]">
              <button
                type="button"
                onClick={() => setActiveTab("config")}
                className={`px-3 py-1.5 text-xs border-b-2 transition-colors ${
                  activeTab === "config"
                    ? "border-[var(--primary)] text-[var(--primary)]"
                    : "border-transparent text-[var(--muted-foreground)] hover:text-[var(--foreground)]"
                }`}
              >
                <Settings size={12} className="inline mr-1" /> 配置
              </button>
              <button
                type="button"
                onClick={() => setActiveTab("code")}
                className={`px-3 py-1.5 text-xs border-b-2 transition-colors ${
                  activeTab === "code"
                    ? "border-[var(--primary)] text-[var(--primary)]"
                    : "border-transparent text-[var(--muted-foreground)] hover:text-[var(--foreground)]"
                }`}
              >
                <Code2 size={12} className="inline mr-1" /> 代码编辑器
              </button>
            </div>

            {activeTab === "config" && (
              <div className="space-y-3">
                <div>
                  <label className={LABEL_CLASS}>Parameter Schema (JSON Schema)</label>
                  <textarea
                    className={`${INPUT_CLASS} font-mono text-xs`}
                    rows={6}
                    value={schemaDisplay}
                    onChange={(e) => {
                      try {
                        const parsed = JSON.parse(e.target.value);
                        setEditing({ ...editing, parameter_schema: parsed });
                        setSchemaError(null);
                      } catch {
                        // Allow editing even when JSON is temporarily invalid
                      }
                    }}
                    disabled={isHardcoded(editing.name)}
                  />
                  <div className="flex items-center gap-2 mt-1">
                    <button type="button" onClick={handleValidateSchema} className="text-[10px] text-[var(--primary)] hover:underline">
                      验证 Schema
                    </button>
                    {schemaError && (
                      <span className="text-[10px] text-red-500">{schemaError}</span>
                    )}
                  </div>
                </div>
              </div>
            )}

            {activeTab === "code" && (
              <div>
                <label className={LABEL_CLASS}>Code Body (JSON 配置)</label>
                <textarea
                  className={`${INPUT_CLASS} font-mono text-xs`}
                  rows={14}
                  value={codeBodyDisplay}
                  onChange={(e) => {
                    setEditing({ ...editing, code_body: e.target.value });
                  }}
                  disabled={isHardcoded(editing.name)}
                  spellCheck={false}
                />
                <p className="text-[10px] text-[var(--muted-foreground)] mt-1">
                  Prompt 类型需要: system_prompt, user_prompt_template（使用 {"{{变量名}}"} 模板语法）
                </p>
                {/* Mock 运行测试按钮 */}
                {editing.id && editing.skill_type !== "composite" && (
                  <MockTestButton
                    skillType={editing.skill_type}
                    codeBody={editing.code_body}
                  />
                )}
              </div>
            )}

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
                删除后不可恢复。确定要删除此 Skill 吗？
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
