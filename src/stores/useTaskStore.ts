import { create } from "zustand";

export interface TaskInfo {
  id: string;
  fileName: string;
  stage: string;
  progress: number;
  status: "running" | "completed" | "failed";
  error?: string;
}

interface TaskState {
  tasks: TaskInfo[];
  addTask: (task: TaskInfo) => void;
  updateTask: (id: string, updates: Partial<TaskInfo>) => void;
  removeTask: (id: string) => void;
  clearCompleted: () => void;
}

export const useTaskStore = create<TaskState>((set) => ({
  tasks: [],

  addTask: (task) =>
    set((s) => ({ tasks: [...s.tasks, task] })),

  updateTask: (id, updates) =>
    set((s) => ({
      tasks: s.tasks.map((t) => (t.id === id ? { ...t, ...updates } : t)),
    })),

  removeTask: (id) =>
    set((s) => ({ tasks: s.tasks.filter((t) => t.id !== id) })),

  clearCompleted: () =>
    set((s) => ({ tasks: s.tasks.filter((t) => t.status !== "completed") })),
}));
