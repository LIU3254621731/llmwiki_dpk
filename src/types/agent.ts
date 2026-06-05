export interface AgentDefinition {
  id: string;
  name: string;
  role: string;
  trigger_event: string;
  system_prompt: string;
  allowed_skills: string[];
  status: 'active' | 'disabled' | 'error';
  max_depth: number;
  timeout_secs: number;
  metadata_json: Record<string, unknown>;
  created_at: string;
  updated_at: string;
}

export const HARDCODED_AGENTS = [
  'AdminAgent',
  'CoordinatorAgent',
  'SourceIngestAgent',
  'ResolutionAgent',
  'RelationshipAgent',
  'WikiUpdateAgent',
  'HealthCheckAgent',
  'QueryAgent',
] as const;

export type HardcodedAgentName = (typeof HARDCODED_AGENTS)[number];

export const TRIGGER_EVENTS = [
  { value: 'manual', label: '手动触发' },
  { value: 'source_ingested', label: '文档导入完成' },
  { value: 'ingest_completed', label: '文档解析完成' },
  { value: 'resolution_completed', label: '实体消歧完成' },
  { value: 'relationship_completed', label: '关系发现完成' },
  { value: 'wiki_update_completed', label: 'Wiki 更新完成' },
  { value: 'health_check_completed', label: '健康检查完成' },
] as const;

export const AGENT_ROLES = [
  { value: 'custom', label: '自定义' },
  { value: 'orchestrator', label: '协调调度' },
  { value: 'ingestor', label: '文档解析' },
  { value: 'resolver', label: '实体消歧' },
  { value: 'connector', label: '关系发现' },
  { value: 'writer', label: 'Wiki 更新' },
  { value: 'diagnostician', label: '健康检查' },
  { value: 'assistant', label: '智能问答' },
] as const;
