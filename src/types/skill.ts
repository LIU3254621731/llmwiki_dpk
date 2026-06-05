export interface SkillDefinition {
  id: string;
  name: string;
  description: string;
  code_body: string;
  parameter_schema: Record<string, unknown>;
  skill_type: 'prompt' | 'transform' | 'composite';
  status: 'active' | 'disabled';
  metadata_json: Record<string, unknown>;
  created_at: string;
  updated_at: string;
}

export const HARDCODED_SKILLS = [
  'DocumentProcessor',
  'PdfSkill',
  'DocxSkill',
  'HtmlSkill',
  'MdSkill',
  'TxtSkill',
  'MarkitdownSkill',
  'PdfOcrSkill',
  'PptxSkill',
  'WebSearchSkill',
] as const;

export type HardcodedSkillName = (typeof HARDCODED_SKILLS)[number];

export const SKILL_TYPES = [
  { value: 'prompt', label: 'Prompt 模板 (LLM 调用)' },
  { value: 'transform', label: 'Transform (系统函数)' },
  { value: 'composite', label: 'Composite (组合调用)' },
] as const;
