import { useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useEditorStore } from "@/stores/useEditorStore";
import type { CitationMeta } from "@/components/common/CitationTag";

interface CitationTargetInfo {
  valid: boolean;
  file_name: string;
  file_path: string;
  file_type: string;
  reason?: string;
}

export function useCitationClick() {
  useEffect(() => {
    const handler = async (event: Event) => {
      const detail = (event as CustomEvent<CitationMeta>).detail;
      if (!detail?.sourceId) return;

      try {
        const info = await invoke<CitationTargetInfo>("validate_citation_target", {
          sourceId: detail.sourceId,
        });

        if (info.valid) {
          const tabType = info.file_type === "pdf" ? "pdf_viewer" : "file";
          useEditorStore.getState().openFile({
            path: info.file_path,
            title: info.file_name,
            type: tabType,
            content: "",
            sourceId: detail.sourceId,
            page: detail.page,
          });
        } else {
          window.dispatchEvent(
            new CustomEvent("notification", {
              detail: {
                level: "warning",
                title: "源文件已不存在",
                message: `引用 "${detail.fileName}" 指向的文件已被删除。可重新导入该文件或触发自愈修复。`,
              },
            })
          );
        }
      } catch (e) {
        console.error("[citation:click] 验证失败:", e);
      }
    };

    window.addEventListener("citation:click", handler);
    return () => window.removeEventListener("citation:click", handler);
  }, []);
}
