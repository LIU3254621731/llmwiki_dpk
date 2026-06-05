import { z } from 'zod';

export const skillDefinitionSchema = z.object({
  id: z.string().min(1),
  name: z.string().min(1, 'Skill 名称不能为空'),
  description: z.string(),
  code_body: z.string(),
  parameter_schema: z.record(z.string(), z.unknown()),
  skill_type: z.enum(['prompt', 'transform', 'composite']),
  status: z.enum(['active', 'disabled']),
  metadata_json: z.record(z.string(), z.unknown()),
  created_at: z.string(),
  updated_at: z.string(),
});

export type SkillDefinitionZod = z.infer<typeof skillDefinitionSchema>;

export function validateSkillDefinition(data: unknown) {
  return skillDefinitionSchema.safeParse(data);
}

export function validateCodeBodyJson(codeBody: string): { valid: boolean; error?: string } {
  try {
    const parsed = JSON.parse(codeBody);
    if (typeof parsed !== 'object' || parsed === null) {
      return { valid: false, error: 'code_body 必须是一个 JSON 对象' };
    }
    return { valid: true };
  } catch (e) {
    return { valid: false, error: `JSON 解析错误: ${(e as Error).message}` };
  }
}
