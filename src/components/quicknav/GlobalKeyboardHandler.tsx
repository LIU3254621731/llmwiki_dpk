import { useEffect } from "react";
import { useQuickNavStore } from "@/stores/useQuickNavStore";

export default function GlobalKeyboardHandler() {
  const openQuickSwitcher = useQuickNavStore((s) => s.openQuickSwitcher);
  const openCommandPalette = useQuickNavStore((s) => s.openCommandPalette);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      const mod = e.ctrlKey || e.metaKey;

      if (mod && e.shiftKey && e.key === "P") {
        e.preventDefault();
        openCommandPalette();
        return;
      }

      if (mod && (e.key === "o" || e.key === "O" || e.key === "p" || e.key === "P")) {
        e.preventDefault();
        if (e.key === "p" || e.key === "P") {
          openQuickSwitcher();
        } else {
          openQuickSwitcher();
        }
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [openQuickSwitcher, openCommandPalette]);

  useEffect(() => {
    const handler = () => openQuickSwitcher();
    window.addEventListener("editor-open-quick-switcher", handler);
    return () => window.removeEventListener("editor-open-quick-switcher", handler);
  }, [openQuickSwitcher]);

  return null;
}
