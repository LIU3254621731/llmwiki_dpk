import { useRef, useEffect, useCallback } from "react";
import { useCanvasStore } from "@/stores/useCanvasStore";
import type { OutlineNode } from "@/types/canvas";
import MarkdownRenderer from "@/components/common/MarkdownRenderer";
import { preprocessContent } from "@/components/canvas/LaTeXRenderer";

// Simple concept detection: wraps known outline node titles in clickable spans
function highlightConcepts(html: string, outlineNodes: OutlineNode[]): string {
  const titles = collectNodeTitles(outlineNodes);
  if (titles.length === 0) return html;

  let result = html;
  for (const title of titles) {
    if (title.length < 2) continue;
    // Escape regex special chars
    const escaped = title.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
    const regex = new RegExp(
      `(?<!["'=])(?<!<[^>]*)(?<!data-concept="[^"]*)${escaped}(?![^<]*>)(?!"')`,
      "g",
    );
    result = result.replace(
      regex,
      `<span class="canvas-concept" data-concept="${title}" style="text-decoration:underline;text-decoration-style:dotted;text-underline-offset:3px;cursor:pointer;color:var(--color-primary)">${title}</span>`,
    );
  }
  return result;
}

function collectNodeTitles(nodes: OutlineNode[]): string[] {
  const titles: string[] = [];
  function walk(list: OutlineNode[]) {
    for (const n of list) {
      if (n.title.length >= 2) titles.push(n.title);
      if (n.children) walk(n.children);
    }
  }
  walk(nodes);
  // Remove duplicates, sort by length descending (longer matches first)
  return [...new Set(titles)].sort((a, b) => b.length - a.length);
}

export default function CanvasTextbookView() {
  const textbookContent = useCanvasStore((s) => s.textbookContent);
  const streamingText = useCanvasStore((s) => s.streamingText);
  const generationPhase = useCanvasStore((s) => s.generationPhase);
  const outlineNodes = useCanvasStore((s) => s.outlineNodes);
  const scrollPosition = useCanvasStore((s) => s.scrollPosition);
  const setScrollPosition = useCanvasStore((s) => s.setScrollPosition);
  const showDetailPanel = useCanvasStore((s) => s.showDetailPanel);

  const containerRef = useRef<HTMLDivElement>(null);

  // Restore scroll position on mount
  useEffect(() => {
    if (containerRef.current && scrollPosition > 0) {
      containerRef.current.scrollTop = scrollPosition;
    }
  }, []);

  // Save scroll position on scroll
  const handleScroll = useCallback(() => {
    if (containerRef.current) {
      setScrollPosition(containerRef.current.scrollTop);
    }
  }, [setScrollPosition]);

  // Handle concept clicks via event delegation
  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    const handler = (e: MouseEvent) => {
      const target = e.target as HTMLElement;
      const conceptSpan = target.closest("[data-concept]");
      if (conceptSpan) {
        const concept = conceptSpan.getAttribute("data-concept");
        if (concept) {
          showDetailPanel(concept);
        }
      }
    };

    container.addEventListener("click", handler);
    return () => container.removeEventListener("click", handler);
  }, [showDetailPanel, outlineNodes, textbookContent, streamingText]);

  const displayContent = textbookContent || streamingText;
  const isLoading = generationPhase === "textbook" && !textbookContent;
  const isStreaming = generationPhase === "textbook" && streamingText;

  if (!displayContent && !isLoading) {
    return (
      <div className="h-full flex items-center justify-center">
        <p className="text-sm text-muted-foreground">
          选择标签并点击"生成教材"以开始
        </p>
      </div>
    );
  }

  // Apply LaTeX rendering + concept highlighting
  const processedContent = displayContent
    ? preprocessContent(displayContent)
    : "";
  const enhancedHtml = processedContent
    ? highlightConcepts(processedContent, outlineNodes)
    : "";

  return (
    <div className="h-full flex flex-col">
      {/* Streaming indicator */}
      {isStreaming && (
        <div className="px-4 py-2 bg-primary/5 border-b border-primary/10 shrink-0 flex items-center gap-2">
          <div className="w-3 h-3 border-2 border-primary border-t-transparent rounded-full animate-spin" />
          <span className="text-xs text-primary font-medium">AI 正在撰写教材...</span>
        </div>
      )}

      {/* Loading skeleton */}
      {isLoading && !streamingText && (
        <div className="flex-1 flex items-center justify-center">
          <div className="text-center">
            <div className="w-8 h-8 border-3 border-primary border-t-transparent rounded-full animate-spin mx-auto mb-3" />
            <p className="text-sm text-muted-foreground">准备教材内容...</p>
          </div>
        </div>
      )}

      {/* Rendered markdown */}
      {displayContent && (
        <div
          ref={containerRef}
          onScroll={handleScroll}
          className="flex-1 overflow-y-auto canvas-textbook-content"
        >
          <div className="max-w-3xl mx-auto px-8 py-6">
            <MarkdownRenderer content={enhancedHtml} />
          </div>
        </div>
      )}

      <style>{`
        .canvas-concept {
          text-decoration: underline;
          text-decoration-style: dotted;
          text-underline-offset: 3px;
          cursor: pointer;
          color: var(--color-primary, #3b82f6);
          transition: opacity 0.15s;
        }
        .canvas-concept:hover {
          opacity: 0.8;
        }
      `}</style>
    </div>
  );
}
