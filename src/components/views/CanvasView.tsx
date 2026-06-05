import { useEffect, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import "katex/dist/katex.min.css";
import { useKBStore } from "@/stores/useKBStore";
import { useCanvasStore } from "@/stores/useCanvasStore";
import CanvasHeader from "@/components/canvas/CanvasHeader";
import CanvasOutlineTree from "@/components/canvas/CanvasOutlineTree";
import CanvasTextbookView from "@/components/canvas/CanvasTextbookView";
import CanvasDetailView from "@/components/canvas/CanvasDetailView";
import GenerationLockOverlay from "@/components/canvas/GenerationLockOverlay";
import MicroCanvas from "@/components/canvas-engine/micro/MicroCanvas";

export default function CanvasView() {
  const currentKB = useKBStore((s) => s.currentKB);
  const tags = useCanvasStore((s) => s.tags);
  const generationPhase = useCanvasStore((s) => s.generationPhase);
  const generationLock = useCanvasStore((s) => s.generationLock);
  const generationError = useCanvasStore((s) => s.generationError);
  const detailPanelVisible = useCanvasStore((s) => s.detailPanelVisible);
  const mindmapTree = useCanvasStore((s) => s.mindmapTree);
  const mindmapLoading = useCanvasStore((s) => s.mindmapLoading);
  const appendStreamingChunk = useCanvasStore((s) => s.appendStreamingChunk);
  const setTextbookContent = useCanvasStore((s) => s.setTextbookContent);
  const setGenerationPhase = useCanvasStore((s) => s.setGenerationPhase);
  const setGenerationLock = useCanvasStore((s) => s.setGenerationLock);
  const setGenerationError = useCanvasStore((s) => s.setGenerationError);

  // Track listener cleanup functions with a ref to survive async setup
  const listenerCleanupRef = useRef<(() => void)[]>([]);

  // Streaming event listeners
  useEffect(() => {
    if (generationPhase !== "textbook") return;

    let cancelled = false;

    // Clean up any stale listeners from previous runs
    listenerCleanupRef.current.forEach((fn) => fn());
    listenerCleanupRef.current = [];

    // Register all listeners concurrently, store cleanup fns when ready
    Promise.all([
      listen<{ chunk: string; accumulated: string }>(
        "canvas-stream-chunk",
        (e) => {
          if (!cancelled) appendStreamingChunk(e.payload.chunk);
        },
      ),
      listen<{ full_text: string }>(
        "canvas-stream-done",
        (e) => {
          if (!cancelled) {
            setTextbookContent(e.payload.full_text);
            setGenerationPhase("done");
            setGenerationLock(false);
          }
        },
      ),
      listen<{ error: string }>(
        "canvas-stream-error",
        (e) => {
          if (!cancelled) {
            setGenerationError(e.payload.error);
            setGenerationLock(false);
            setGenerationPhase("idle");
          }
        },
      ),
    ]).then((fns) => {
      if (cancelled) {
        // Cleanup already ran — unregister immediately
        fns.forEach((fn) => fn());
      } else {
        listenerCleanupRef.current = fns;
      }
    }).catch(() => {
      // listen() failed — nothing to clean up
    });

    return () => {
      cancelled = true;
      listenerCleanupRef.current.forEach((fn) => fn());
      listenerCleanupRef.current = [];
    };
  }, [generationPhase]);

  // Show empty state when no tags
  const showEmptyState = tags.length === 0 && !generationLock && generationPhase === "idle";

  return (
    <div className="h-full flex flex-col overflow-hidden bg-background relative">
      {/* Header */}
      <CanvasHeader />

      {/* Error banner */}
      {generationError && (
        <div className="mx-4 mt-2 px-3 py-2 bg-destructive/10 border border-destructive/20 rounded-lg text-sm text-destructive shrink-0">
          {generationError}
          <button
            type="button"
            onClick={() => setGenerationError("")}
            className="ml-2 underline hover:no-underline"
          >
            关闭
          </button>
        </div>
      )}

      {/* Body: three panels */}
      <div className="flex-1 flex overflow-hidden">
        {/* Left panel: outline tree (25%) */}
        <div className="w-[25%] min-w-[200px] border-r border-border overflow-y-auto bg-sidebar-bg/50">
          <CanvasOutlineTree />
        </div>

        {/* Center panel: textbook (50%) */}
        <div className="flex-1 overflow-hidden">
          {showEmptyState ? (
            <div className="h-full flex items-center justify-center">
              <div className="text-center px-8">
                <div className="w-16 h-16 rounded-2xl bg-primary/10 flex items-center justify-center mx-auto mb-4">
                  <svg className="w-8 h-8 text-primary" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={1.5}>
                    <path strokeLinecap="round" strokeLinejoin="round" d="M9.53 16.122a3 3 0 00-5.78 1.128 2.25 2.25 0 01-2.4 2.245 4.5 4.5 0 008.4-2.245c0-.399-.078-.78-.22-1.128zm0 0a15.998 15.998 0 003.388-1.62m-5.043-.025a15.994 15.994 0 011.622-3.395m3.42 3.42a15.995 15.995 0 004.764-4.648l3.876-5.814a1.151 1.151 0 00-1.597-1.597L14.146 6.32a15.996 15.996 0 00-4.649 4.763m3.42 3.42a6.776 6.776 0 00-3.42-3.42" />
                  </svg>
                </div>
                <p className="text-sm text-muted-foreground">
                  请在顶部搜索栏输入 <code className="px-1 py-0.5 bg-muted rounded text-xs">#</code> 选择知识标签，开始 AI 知识重组
                </p>
                {!currentKB && (
                  <p className="text-xs text-destructive mt-2">请先选择或创建一个知识库</p>
                )}
              </div>
            </div>
          ) : (
            <CanvasTextbookView />
          )}
        </div>

        {/* Right panel: detail wiki or mindmap (25%) */}
        {detailPanelVisible ? (
          <CanvasDetailView />
        ) : tags.length > 0 ? (
          <div className="w-[25%] min-w-[220px] border-l border-border h-full shrink-0">
            {mindmapLoading ? (
              <div className="flex items-center justify-center h-full">
                <div className="text-center">
                  <div className="w-5 h-5 border-2 border-primary border-t-transparent rounded-full animate-spin mx-auto mb-2" />
                  <p className="text-xs text-muted-foreground">生成思维导图...</p>
                </div>
              </div>
            ) : (
              <MicroCanvas
                tagId={tags[0]}
                rootTopic={tags[0]}
                initialTree={mindmapTree}
                onBack={undefined}
              />
            )}
          </div>
        ) : null}
      </div>

      {/* Generation lock overlay */}
      {generationLock && <GenerationLockOverlay />}
    </div>
  );
}
