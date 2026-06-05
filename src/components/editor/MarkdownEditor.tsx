import { useState, useRef, useEffect } from "react";
import { Compartment, EditorState } from "@codemirror/state";
import { EditorView, keymap, lineNumbers } from "@codemirror/view";
import { markdown } from "@codemirror/lang-markdown";
import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
import { autocompletion } from "@codemirror/autocomplete";
import { syntaxHighlighting, defaultHighlightStyle } from "@codemirror/language";
import MarkdownRenderer from "@/components/common/MarkdownRenderer";
import { wikiLinkCompletionSource, setCompletionKbId } from "./wikiLinkCompletion";
import { useKBStore } from "@/stores/useKBStore";

type ViewMode = "edit" | "preview" | "split";

interface MarkdownEditorProps {
  content: string;
  onChange: (value: string) => void;
  readOnly?: boolean;
  fileName?: string;
}

const editableCompartment = new Compartment();

const customTheme = EditorView.theme(
  {
    "&": {
      fontSize: "14px",
      fontFamily: "ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace",
      color: "#1e293b",
      backgroundColor: "#ffffff",
      height: "100%",
    },
    ".cm-content": {
      padding: "16px",
      lineHeight: "24px",
    },
    ".cm-gutters": {
      backgroundColor: "#f8fafc",
      color: "#cbd5e1",
      borderRight: "none",
      fontSize: "12px",
    },
    ".cm-activeLineGutter": {
      backgroundColor: "#f1f5f9",
      color: "#94a3b8",
    },
    ".cm-activeLine": {
      backgroundColor: "#f8fafc50",
    },
    ".cm-cursor": {
      borderLeftColor: "#1e293b",
    },
    ".cm-selectionBackground": {
      backgroundColor: "#bae6fd !important",
    },
    "&.cm-editor.cm-focused": {
      outline: "none",
    },
    "&.cm-editor .cm-scroll": {
      overscrollBehavior: "none",
    },
  },
  { dark: false }
);

export default function MarkdownEditor({
  content,
  onChange,
  readOnly = false,
  fileName,
}: MarkdownEditorProps) {
  const [viewMode, setViewMode] = useState<ViewMode>("split");
  const editorContainerRef = useRef<HTMLDivElement>(null);
  const editorViewRef = useRef<EditorView | null>(null);
  const currentKB = useKBStore((s) => s.currentKB);

  // Keep the completion source's kbId in sync
  useEffect(() => {
    setCompletionKbId(currentKB?.id ?? "");
  }, [currentKB?.id]);

  // Create CodeMirror editor once
  useEffect(() => {
    if (!editorContainerRef.current) return;

    const view = new EditorView({
      doc: content,
      extensions: [
        lineNumbers(),
        syntaxHighlighting(defaultHighlightStyle),
        markdown(),
        history(),
        keymap.of([...defaultKeymap, ...historyKeymap]),
        EditorView.updateListener.of((update) => {
          if (update.docChanged) {
            onChange(update.state.doc.toString());
          }
        }),
        autocompletion({
          override: [wikiLinkCompletionSource],
          closeOnBlur: true,
          defaultKeymap: true,
        }),
        editableCompartment.of(EditorView.editable.of(!readOnly)),
        customTheme,
      ],
      parent: editorContainerRef.current,
    });

    editorViewRef.current = view;

    return () => {
      view.destroy();
      editorViewRef.current = null;
    };
  }, []);

  // Sync external content changes into CodeMirror
  useEffect(() => {
    const view = editorViewRef.current;
    if (!view) return;
    const currentContent = view.state.doc.toString();
    if (content !== currentContent) {
      view.dispatch({
        changes: { from: 0, to: currentContent.length, insert: content },
      });
    }
  }, [content]);

  // Sync readOnly state
  useEffect(() => {
    const view = editorViewRef.current;
    if (!view) return;
    view.dispatch({
      effects: editableCompartment.reconfigure(
        EditorView.editable.of(!readOnly)
      ),
    });
  }, [readOnly]);

  // Request measure when editor becomes visible
  useEffect(() => {
    const view = editorViewRef.current;
    if (view && viewMode !== "preview") {
      requestAnimationFrame(() => view.requestMeasure());
    }
  }, [viewMode]);

  const showEdit = viewMode === "edit" || viewMode === "split";
  const showPreview = viewMode === "preview" || viewMode === "split";

  const toggleButton = (mode: ViewMode, label: string) => (
    <button
      type="button"
      onClick={() => setViewMode(mode)}
      className={`text-xs px-2.5 py-1 rounded transition-colors ${
        viewMode === mode
          ? "text-brand-600 bg-brand-50 font-medium"
          : "text-slate-500 hover:text-slate-700 hover:bg-slate-100"
      }`}
    >
      {label}
    </button>
  );

  return (
    <div className="flex flex-col h-full">
      {/* Header bar */}
      <div className="flex items-center justify-between h-9 px-3 bg-white border-b border-slate-100 flex-shrink-0">
        <div className="flex items-center gap-1.5">
          {fileName && (
            <span className="text-xs text-slate-400 font-mono truncate max-w-[300px]">
              {fileName}
            </span>
          )}
        </div>
        <div className="flex items-center gap-1 bg-slate-50 rounded-md p-0.5">
          {toggleButton("edit", "编辑")}
          {toggleButton("split", "分屏")}
          {toggleButton("preview", "预览")}
        </div>
      </div>

      {/* Editor + Preview panes */}
      <div className="flex-1 flex overflow-hidden">
        <div
          className={`overflow-hidden ${
            !showEdit ? "hidden" : ""
          } ${
            viewMode === "split" ? "w-1/2 border-r border-slate-200" : "flex-1"
          }`}
        >
          <div ref={editorContainerRef} className="h-full" />
        </div>

        {showPreview && (
          <div
            className={`overflow-y-auto ${
              viewMode === "split" ? "w-1/2" : "flex-1"
            }`}
          >
            <div className="p-6">
              {content.trim() ? (
                <MarkdownRenderer content={content} />
              ) : (
                <div className="flex items-center justify-center h-64 text-sm text-slate-300 select-none">
                  暂无内容
                </div>
              )}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
