import { z } from 'zod';

export const agentDefinitionSchema = z.object({
  id: z.string().min(1),
  name: z.string().min(1, 'Agent 名称不能为空'),
  role: z.string().min(1),
  trigger_event: z.string().min(1),
  system_prompt: z.string().min(1, '系统提示词不能为空'),
  allowed_skills: z.array(z.string()),
  status: z.enum(['active', 'disabled', 'error']),
  max_depth: z.number().int().min(1).max(10),
  timeout_secs: z.number().int().min(1).max(600),
  metadata_json: z.record(z.string(), z.unknown()),
  created_at: z.string(),
  updated_at: z.string(),
});

export type AgentDefinitionZod = z.infer<typeof agentDefinitionSchema>;

export function validateAgentDefinition(data: unknown) {
  return agentDefinitionSchema.safeParse(data);
}
