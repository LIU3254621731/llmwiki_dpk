import { create } from 'zustand';
import type { AgentDefinition } from '@/types/agent';
import type { SkillDefinition } from '@/types/skill';

interface AgentSkillState {
  agents: AgentDefinition[];
  skills: SkillDefinition[];
  selectedAgentId: string | null;
  selectedSkillId: string | null;
  loading: boolean;
  error: string | null;

  setAgents: (agents: AgentDefinition[]) => void;
  setSkills: (skills: SkillDefinition[]) => void;
  setSelectedAgentId: (id: string | null) => void;
  setSelectedSkillId: (id: string | null) => void;
  setLoading: (v: boolean) => void;
  setError: (e: string | null) => void;

  addAgent: (a: AgentDefinition) => void;
  updateAgent: (id: string, patch: Partial<AgentDefinition>) => void;
  removeAgent: (id: string) => void;
  addSkill: (s: SkillDefinition) => void;
  updateSkill: (id: string, patch: Partial<SkillDefinition>) => void;
  removeSkill: (id: string) => void;
}

export const useAgentSkillStore = create<AgentSkillState>((set) => ({
  agents: [],
  skills: [],
  selectedAgentId: null,
  selectedSkillId: null,
  loading: false,
  error: null,

  setAgents: (agents) => set({ agents }),
  setSkills: (skills) => set({ skills }),
  setSelectedAgentId: (id) => set({ selectedAgentId: id }),
  setSelectedSkillId: (id) => set({ selectedSkillId: id }),
  setLoading: (v) => set({ loading: v }),
  setError: (e) => set({ error: e }),

  addAgent: (a) => set((s) => ({ agents: [...s.agents, a] })),
  updateAgent: (id, patch) =>
    set((s) => ({
      agents: s.agents.map((a) => (a.id === id ? { ...a, ...patch } : a)),
    })),
  removeAgent: (id) =>
    set((s) => ({
      agents: s.agents.filter((a) => a.id !== id),
      selectedAgentId: s.selectedAgentId === id ? null : s.selectedAgentId,
    })),
  addSkill: (sk) => set((s) => ({ skills: [...s.skills, sk] })),
  updateSkill: (id, patch) =>
    set((s) => ({
      skills: s.skills.map((sk) => (sk.id === id ? { ...sk, ...patch } : sk)),
    })),
  removeSkill: (id) =>
    set((s) => ({
      skills: s.skills.filter((sk) => sk.id !== id),
      selectedSkillId: s.selectedSkillId === id ? null : s.selectedSkillId,
    })),
}));
