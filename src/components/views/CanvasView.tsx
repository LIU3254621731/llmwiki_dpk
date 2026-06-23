import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import "katex/dist/katex.min.css";
import { useKBStore } from "@/stores/useKBStore";
import { useCanvasStore } from "@/stores/useCanvasStore";
import { useEditorStore } from "@/stores/useEditorStore";
import MindMapView from "@/components/graph/MindMapView";
import type { GraphData, GraphNode } from "@/types/graph";
import { invoke } from "@tauri-apps/api/core";
import { GitGraph, ArrowLeftRight } from "lucide-react";
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

  // ©¤©¤ Graph data state ©¤©¤
  const [graphData, setGraphData] = useState<GraphData | null>(null);
  const [graphLoading, setGraphLoading] = useState(true);
  const [graphError, setGraphError] = useState("");
  const graphRequestRef = useRef(0);

  // ©¤©¤ View mode toggle (textbook vs knowledge graph) ©¤©¤
  const [viewMode, setViewMode] = useState<"textbook" | "graph">("textbook");

  // ©¤©¤ Fetch graph data on KB change ©¤©¤
  useEffect(() => {
    if (!currentKB) return;
    setGraphLoading(true);
    setGraphError("");
    const reqId = ++graphRequestRef.current;
    (async () => {
      try {
        const data = await invoke<GraphData>("get_graph_data", { kbId: currentKB.id });
        if (graphRequestRef.current !== reqId) return;
        setGraphData(data);
      } catch (e) {
        if (graphRequestRef.current !== reqId) return;
        setGraphError(`Graph load failed: ${e}`);
      }
      if (graphRequestRef.current !== reqId) return;
      setGraphLoading(false);
    })();
  }, [currentKB?.id]);

  // Graph node click ¡ú open wiki page
  const handleGraphNodeClick = (node: GraphNode) => {
    if (node.path) {
      useEditorStore.getState().openFile({
        path: node.path,
        title: node.label,
        type: "wiki",
      });
    }
  };

  // Switch to import/review tab
  const handleGoToImport = () => {
    useEditorStore.getState().openFile({
      path: "import-review",
      title: "Import & Review",
      type: "import_review",
    });
  };

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
        // Cleanup already ran â€?unregister immediately
        fns.forEach((fn) => fn());
      } else {
        listenerCleanupRef.current = fns;
      }
    }).catch(() => {
      // listen() failed â€?nothing to clean up
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
      {/* Header with mode toggle */}
      <div className="flex items-center justify-between shrink-0">
        <CanvasHeader />
        <div className="flex items-center gap-2 px-3 py-1.5 mr-2">
          <button
            type="button"
            onClick={() => setViewMode("textbook")}
            className={`px-3 py-1 text-xs rounded transition-colors ${viewMode === "textbook" ? "bg-primary/10 text-primary font-medium" : "text-muted-foreground hover:bg-muted"}`}
          >
            Textbook
          </button>
          <button
            type="button"
            onClick={() => setViewMode("graph")}
            className={`px-3 py-1 text-xs rounded transition-colors flex items-center gap-1 ${viewMode === "graph" ? "bg-primary/10 text-primary font-medium" : "text-muted-foreground hover:bg-muted"}`}
          >
            <GitGraph size={13} />
            Knowledge Graph
            {graphData && graphData.nodes.length > 0 && (
              <span className="ml-0.5 px-1 py-0.5 rounded-full bg-primary/20 text-[10px] leading-none">{graphData.nodes.length}</span>
            )}
          </button>
        </div>
      </div>

      {/* Error banner */}
      {generationError && (
        <div className="mx-4 mt-2 px-3 py-2 bg-destructive/10 border border-destructive/20 rounded-lg text-sm text-destructive shrink-0">
          {generationError}
          <button
            type="button"
            onClick={() => setGenerationError("")}
            className="ml-2 underline hover:no-underline"
          >
            å…³é—­
          </button>
        </div>
      )}

      {/* ©¤©¤ Knowledge Graph View ©¤©¤ */}
      {viewMode === "graph" && (
        <div className="flex-1 flex overflow-hidden">
          {graphLoading ? (
            <div className="flex-1 flex items-center justify-center">
              <div className="w-5 h-5 border-2 border-primary border-t-transparent rounded-full animate-spin" />
            </div>
          ) : graphError ? (
            <div className="flex-1 flex items-center justify-center">
              <div className="text-center">
                <p className="text-sm text-destructive">{graphError}</p>
                <button type="button" onClick={() => { setGraphError(""); setGraphLoading(true); }} className="mt-2 text-xs text-primary underline">Retry</button>
              </div>
            </div>
          ) : !graphData || graphData.nodes.length === 0 ? (
            <div className="flex-1 flex items-center justify-center p-8">
              <div className="text-center max-w-md">
                <GitGraph size={48} className="mx-auto mb-4 text-slate-300 dark:text-slate-600" />
                <h3 className="text-base font-semibold text-slate-700 dark:text-slate-300 mb-2">
                  No knowledge graph data yet
                </h3>
                <p className="text-sm text-slate-500 dark:text-slate-400 mb-6">
                  Import documents and approve review items to build the graph.
                </p>
                <button
                  type="button"
                  onClick={handleGoToImport}
                  className="inline-flex items-center gap-2 px-4 py-2 rounded-lg bg-primary text-primary-foreground text-sm font-medium hover:bg-primary/90 transition-colors"
                >
                  <ArrowLeftRight size={14} />
                  Go to Import
                </button>
              </div>
            </div>
          ) : (
            <MindMapView
              nodes={graphData.nodes}
              edges={graphData.edges}
              kbName={currentKB?.name ?? "LLMWiki"}
              onNodeClick={handleGraphNodeClick}
            />
          )}
        </div>
      )}

      {/* Body: three panels */}
      {viewMode === "textbook" && (
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
                  è¯·åœ¨é¡¶éƒ¨æœç´¢æ è¾“å…?<code className="px-1 py-0.5 bg-muted rounded text-xs">#</code> é€‰æ‹©çŸ¥è¯†æ ‡ç­¾ï¼Œå¼€å§?AI çŸ¥è¯†é‡ç»„
                </p>
                {!currentKB && (
                  <p className="text-xs text-destructive mt-2">è¯·å…ˆé€‰æ‹©æˆ–åˆ›å»ºä¸€ä¸ªçŸ¥è¯†åº“</p>
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
                  <p className="text-xs text-muted-foreground">ç”Ÿæˆæ€ç»´å¯¼å›¾...</p>
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
      )}

      {/* Generation lock overlay */}
      {generationLock && <GenerationLockOverlay />}
    </div>
  );
}
