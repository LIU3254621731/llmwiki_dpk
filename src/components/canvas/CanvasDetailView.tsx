import { useEffect } from "react";
import { useKBStore } from "@/stores/useKBStore";
import { useCanvasStore } from "@/stores/useCanvasStore";
import { invoke } from "@tauri-apps/api/core";
import { X, Copy, Check, Code, BookOpen, Lightbulb, Sigma } from "lucide-react";
import type { DetailData } from "@/types/canvas";
import { useState, useMemo } from "react";
import { renderLaTeX } from "@/components/canvas/LaTeXRenderer";

export default function CanvasDetailView() {
  const currentKB = useKBStore((s) => s.currentKB);
  const detailPanelVisible = useCanvasStore((s) => s.detailPanelVisible);
  const detailTopic = useCanvasStore((s) => s.detailTopic);
  const detailData = useCanvasStore((s) => s.detailData);
  const hideDetailPanel = useCanvasStore((s) => s.hideDetailPanel);
  const setDetailData = useCanvasStore((s) => s.setDetailData);
  const tags = useCanvasStore((s) => s.tags);

  const [copiedIdx, setCopiedIdx] = useState<number | null>(null);

  // Load detail data when panel becomes visible
  useEffect(() => {
    if (!detailPanelVisible || !detailTopic || !currentKB) return;

    let cancelled = false;
    const loadDetail = async () => {
      try {
        const data = await invoke<DetailData>("get_canvas_node_detail", {
          kbId: currentKB.id,
          topic: detailTopic,
          tags,
          cacheKey: "",
        });
        if (!cancelled) setDetailData(data);
      } catch (e) {
        console.error("加载知识详情失败:", e);
      }
    };
    loadDetail();
    return () => {
      cancelled = true;
    };
  }, [detailPanelVisible, detailTopic, currentKB?.id]);

  const handleCopy = async (code: string, idx: number) => {
    try {
      await navigator.clipboard.writeText(code);
      setCopiedIdx(idx);
      setTimeout(() => setCopiedIdx(null), 2000);
    } catch {
      // Clipboard API not available
    }
  };

  if (!detailPanelVisible) return null;

  return (
    <div className="w-[25%] min-w-[220px] border-l border-border bg-card h-full flex flex-col shrink-0 overflow-hidden animate-in slide-in-from-right">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-3 border-b border-border shrink-0">
        <h3 className="text-sm font-semibold text-foreground truncate flex-1">
          {detailTopic || "知识详情"}
        </h3>
        <button
          type="button"
          onClick={hideDetailPanel}
          className="p-1 rounded hover:bg-muted text-muted-foreground hover:text-foreground transition-colors shrink-0"
        >
          <X size={16} />
        </button>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto">
        {!detailData ? (
          <div className="flex items-center justify-center h-full">
            <div className="w-5 h-5 border-2 border-primary border-t-transparent rounded-full animate-spin" />
          </div>
        ) : (
          <div className="p-4 space-y-4">
            {/* Definition */}
            {detailData.definition && (
              <section>
                <div className="flex items-center gap-1.5 mb-2">
                  <BookOpen size={14} className="text-primary" />
                  <h4 className="text-xs font-semibold text-foreground uppercase tracking-wider">
                    学术定义
                  </h4>
                </div>
                <p className="text-sm text-muted-foreground leading-relaxed">
                  {detailData.definition}
                </p>
              </section>
            )}

            {/* Mechanism */}
            {detailData.mechanism && (
              <section>
                <div className="flex items-center gap-1.5 mb-2">
                  <Lightbulb size={14} className="text-amber-500" />
                  <h4 className="text-xs font-semibold text-foreground uppercase tracking-wider">
                    核心机制
                  </h4>
                </div>
                <p className="text-sm text-muted-foreground leading-relaxed">
                  {detailData.mechanism}
                </p>
              </section>
            )}

            {/* Formulas */}
            {detailData.formulas && detailData.formulas.length > 0 && (
              <section>
                <div className="flex items-center gap-1.5 mb-2">
                  <Sigma size={14} className="text-purple-500" />
                  <h4 className="text-xs font-semibold text-foreground uppercase tracking-wider">
                    数学公式
                  </h4>
                </div>
                <div className="space-y-2">
                  {detailData.formulas.map((formula, idx) => (
                    <div
                      key={idx}
                      className="px-3 py-2 bg-muted/50 rounded-lg text-sm text-foreground overflow-x-auto"
                      dangerouslySetInnerHTML={{
                        __html: renderLaTeX(formula.startsWith("$") ? formula : `$${formula}$`),
                      }}
                    />
                  ))}
                </div>
              </section>
            )}

            {/* Code blocks */}
            {detailData.code_blocks && detailData.code_blocks.length > 0 && (
              <section>
                <div className="flex items-center gap-1.5 mb-2">
                  <Code size={14} className="text-green-500" />
                  <h4 className="text-xs font-semibold text-foreground uppercase tracking-wider">
                    代码实现
                  </h4>
                </div>
                <div className="space-y-3">
                  {detailData.code_blocks.map((block, idx) => (
                    <div
                      key={idx}
                      className="rounded-lg border border-border overflow-hidden"
                    >
                      {block.caption && (
                        <div className="px-3 py-1.5 bg-muted/30 border-b border-border text-xs text-muted-foreground">
                          {block.caption}
                        </div>
                      )}
                      <div className="relative">
                        <div className="absolute top-2 right-2 z-10">
                          <button
                            type="button"
                            onClick={() => handleCopy(block.code, idx)}
                            className="p-1 rounded bg-muted hover:bg-muted/80 text-muted-foreground hover:text-foreground transition-colors"
                            title="复制代码"
                          >
                            {copiedIdx === idx ? (
                              <Check size={14} className="text-green-500" />
                            ) : (
                              <Copy size={14} />
                            )}
                          </button>
                        </div>
                        <pre className="px-4 py-3 bg-muted/20 text-xs font-mono text-foreground overflow-x-auto">
                          <code>{block.code}</code>
                        </pre>
                      </div>
                    </div>
                  ))}
                </div>
              </section>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
