// ── Debounced persistence hook for canvas state ──

import { useRef, useCallback } from "react";
import { useCanvasEngineStore } from "@/stores/useCanvasEngineStore";

export function useCanvasPersistence(kbId: string) {
  const saveTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const pendingRef = useRef(false);

  const debouncedSaveMacro = useCallback(
    (schemaJson: string) => {
      pendingRef.current = true;
      if (saveTimerRef.current) clearTimeout(saveTimerRef.current);

      saveTimerRef.current = setTimeout(async () => {
        try {
          const { invoke } = await import("@tauri-apps/api/core");
          await invoke("save_canvas_state", {
            kbId,
            canvasType: "macro",
            canvasId: "default",
            schemaJson,
          });
          pendingRef.current = false;
        } catch (e) {
          console.error("[CanvasPersistence] Save failed:", e);
        }
      }, 300);
    },
    [kbId],
  );

  const debouncedSaveMicro = useCallback(
    (tagId: string, schemaJson: string) => {
      pendingRef.current = true;
      if (saveTimerRef.current) clearTimeout(saveTimerRef.current);

      saveTimerRef.current = setTimeout(async () => {
        try {
          const { invoke } = await import("@tauri-apps/api/core");
          await invoke("save_canvas_state", {
            kbId,
            canvasType: "micro",
            canvasId: tagId,
            schemaJson,
          });
          pendingRef.current = false;
        } catch (e) {
          console.error("[CanvasPersistence] Micro save failed:", e);
        }
      }, 300);
    },
    [kbId],
  );

  const flushPending = useCallback(async () => {
    if (saveTimerRef.current) {
      clearTimeout(saveTimerRef.current);
      saveTimerRef.current = null;
    }
    // Flush via store
    const store = useCanvasEngineStore.getState();
    if (store.macroDirty) {
      await store.saveMacroCanvas(kbId);
    }
  }, [kbId]);

  return { debouncedSaveMacro, debouncedSaveMicro, flushPending, isPending: () => pendingRef.current };
}
