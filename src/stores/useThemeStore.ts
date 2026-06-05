import { create } from "zustand";

type Theme = "light" | "dark";

interface ThemeState {
  theme: Theme;
  toggleTheme: () => void;
  setTheme: (theme: Theme) => void;
}

export const useThemeStore = create<ThemeState>((set) => ({
  theme: (() => {
    const stored = localStorage.getItem("llmwiki-theme");
    if (stored === "dark" || stored === "light") return stored;
    if (window.matchMedia?.("(prefers-color-scheme: dark)")?.matches ?? false) return "dark";
    return "light";
  })(),
  toggleTheme: () =>
    set((s) => {
      const next = s.theme === "dark" ? "light" : "dark";
      localStorage.setItem("llmwiki-theme", next);
      document.documentElement.classList.toggle("dark", next === "dark");
      return { theme: next };
    }),
  setTheme: (theme) =>
    set(() => {
      localStorage.setItem("llmwiki-theme", theme);
      document.documentElement.classList.toggle("dark", theme === "dark");
      return { theme };
    }),
}));
