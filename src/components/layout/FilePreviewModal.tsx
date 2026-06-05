import { useEffect, useRef, useState, useCallback } from "react";
import { useKBStore } from "@/stores/useKBStore";
import type { FileTreeNode } from "@/stores/useFileTreeStore";
import MarkdownRenderer from "@/components/common/MarkdownRenderer";
import { formatSize } from "@/lib/utils";
import {
  X, FileText, Loader2, ExternalLink, Image,
} from "lucide-react";

interface FilePreviewModalProps {
  node: FileTreeNode;
  onClose: () => void;
}

interface PreviewResponse {
  content: string;
  preview_type: string;
  render_hint: {
    can_render_markdown: boolean;
    can_show_source: boolean;
    can_format_json: boolean;
    is_large_file: boolean;
    truncated: boolean;
    truncated_length: number;
  };
  size?: number;
  modified_at?: string;
  error?: string | null;
}

const IMAGE_EXT = new Set(["png", "jpg", "jpeg", "gif", "svg", "webp", "bmp", "ico"]);

export default function FilePreviewModal({ node, onClose }: FilePreviewModalProps) {
  const currentKB = useKBStore((s) => s.currentKB);
  const [preview, setPreview] = useState<PreviewResponse | null>(null);
  const [imageUrl, setImageUrl] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");

  const ext = (node.file_type || node.extension || "").toLowerCase();
  const isImage = IMAGE_EXT.has(ext);
  const abortedRef = useRef(false);

  // Focus management
  const modalRef = useRef<HTMLDivElement>(null);
  const previousFocusRef = useRef<HTMLElement | null>(null);

  useEffect(() => {
    previousFocusRef.current = document.activeElement as HTMLElement | null;
    const closeBtn = modalRef.current?.querySelector<HTMLButtonElement>(
      'button[aria-label="关闭"]'
    );
    (closeBtn || modalRef.current)?.focus();
    return () => {
      previousFocusRef.current?.focus();
    };
  }, []);

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Escape") {
      onClose();
      return;
    }
    if (e.key !== "Tab") return;
    const container = modalRef.current;
    if (!container) return;
    const focusable = container.querySelectorAll<HTMLElement>(
      'button:not([disabled]), [tabindex]:not([tabindex="-1"])'
    );
    if (focusable.length === 0) return;
    const first = focusable[0];
    const last = focusable[focusable.length - 1];
    const active = document.activeElement;
    if (e.shiftKey) {
      if (active === first) {
        e.preventDefault();
        last.focus();
      }
    } else {
      if (active === last) {
        e.preventDefault();
        first.focus();
      }
    }
  };

  const loadPreview = useCallback(async () => {
    if (!currentKB) return;
    abortedRef.current = false;
    setLoading(true);
    setError("");

    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const result = await invoke<PreviewResponse>("get_workspace_file_preview", {
        kbId: currentKB.id,
        kbPath: currentKB.path,
        relativePath: node.relative_path,
      });
      if (abortedRef.current) return;
      setPreview(result);

      if (result.error) {
        setError(result.error);
      }

      // 图片：通过 Tauri asset 协议加载本地文件
      if (result.preview_type === "image") {
        const fullPath = `${currentKB.path}/${node.relative_path}`;
        try {
          const { convertFileSrc } = await import("@tauri-apps/api/core");
          const url = convertFileSrc(fullPath);
          setImageUrl(url);
        } catch {
          setError("无法加载图片文件");
        }
      }
    } catch (e) {
      if (abortedRef.current) return;
      setError(`加载文件失败: ${e}`);
    }
    if (!abortedRef.current) setLoading(false);
  }, [node.relative_path, currentKB?.id, currentKB?.path]);

  useEffect(() => {
    if (!currentKB) return;
    loadPreview();
    return () => { abortedRef.current = true; };
  }, [loadPreview]);

  const handleOpenExternally = async () => {
    if (!currentKB) return;
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const fullPath = `${currentKB.path}/${node.relative_path}`;
      await invoke("shell_open", { path: fullPath });
    } catch (e) {
      console.error("打开文件失败:", e);
    }
  };

  return (
    <div
      ref={modalRef}
      className="fixed inset-0 z-[60] bg-black/50 flex items-center justify-center p-8"
      onKeyDown={handleKeyDown}
      onClick={(e) => { if (e.target === e.currentTarget) onClose(); }}
    >
      <div
        tabIndex={-1}
        className="bg-white dark:bg-slate-900 rounded-xl shadow-2xl w-full max-w-3xl max-h-[85vh] flex flex-col overflow-hidden"
      >
        {/* Header */}
        <div className="flex items-center gap-3 px-5 py-3 border-b border-slate-200 dark:border-slate-800 shrink-0">
          <FileText size={18} className="text-slate-400 shrink-0" />
          <div className="flex-1 min-w-0">
            <h3 className="text-sm font-medium text-slate-800 dark:text-slate-200 truncate">{node.name}</h3>
            <p className="text-xs text-slate-400 font-mono truncate">{node.relative_path}</p>
          </div>
          <button
            type="button"
            onClick={handleOpenExternally}
            className="p-1.5 rounded hover:bg-slate-100 dark:hover:bg-slate-800 text-slate-400 hover:text-slate-600"
            title="在外部程序中打开"
          >
            <ExternalLink size={16} />
          </button>
          <button
            type="button"
            onClick={onClose}
            aria-label="关闭"
            className="p-1.5 rounded hover:bg-slate-100 dark:hover:bg-slate-800 text-slate-400 hover:text-slate-600"
          >
            <X size={18} />
          </button>
        </div>

        {/* Metadata bar */}
        <div className="flex items-center gap-4 px-5 py-2 bg-slate-50 dark:bg-slate-950 border-b border-slate-200 dark:border-slate-800 text-xs text-slate-500 dark:text-slate-400 shrink-0">
          <span>{ext || "unknown"}</span>
          {preview && <span>{formatSize(preview.size || node.file_size || node.size || 0)}</span>}
          {preview?.modified_at && <span>{preview.modified_at}</span>}
        </div>

        {/* Content */}
        <div className="flex-1 overflow-y-auto">
          {loading ? (
            <div className="flex items-center justify-center py-20">
              <Loader2 size={24} className="text-slate-400 animate-spin" />
            </div>
          ) : error && !preview?.content ? (
            <div className="flex flex-col items-center justify-center py-20 px-8 text-center">
              <FileText size={40} className="text-slate-300 dark:text-slate-600 mb-4" />
              <p className="text-sm text-red-500 mb-2">{error}</p>
              <p className="text-xs text-slate-400">可在外部程序中打开查看</p>
            </div>
          ) : isImage && imageUrl ? (
            <div className="flex items-center justify-center p-4">
              <img
                src={imageUrl}
                alt={node.name}
                className="max-w-full max-h-[60vh] object-contain rounded"
                onError={() => setError("图片加载失败，尝试在外部程序中打开")}
              />
            </div>
          ) : isImage && !imageUrl && !loading ? (
            <div className="flex flex-col items-center justify-center py-20 px-8 text-center">
              <Image size={48} className="text-slate-300 dark:text-slate-600 mb-4" />
              <p className="text-sm text-slate-500">图片文件</p>
              <p className="text-xs text-slate-400 mt-1">点击右上角按钮在外部程序中打开</p>
            </div>
          ) : preview?.content ? (
            <div className="px-5 py-4">
              {preview.render_hint.truncated && (
                <div className="mb-3 px-3 py-1.5 bg-amber-50 dark:bg-amber-900/20 border border-amber-200 dark:border-amber-800 rounded text-xs text-amber-700 dark:text-amber-400">
                  文件较大，仅显示前 {preview.render_hint.truncated_length.toLocaleString()} 字符
                </div>
              )}
              {preview.preview_type === "markdown" || preview.render_hint.can_render_markdown ? (
                <MarkdownRenderer content={preview.content} hideFrontmatter={false} />
              ) : (
                <pre className="text-sm text-slate-700 dark:text-slate-300 whitespace-pre-wrap font-mono">
                  {preview.content}
                </pre>
              )}
            </div>
          ) : preview && !preview.content ? (
            <div className="flex flex-col items-center justify-center py-20 px-8 text-center">
              <FileText size={40} className="text-slate-300 dark:text-slate-600 mb-4" />
              <p className="text-sm text-slate-500">内容为空</p>
            </div>
          ) : (
            <div className="flex flex-col items-center justify-center py-20 px-8 text-center">
              <FileText size={40} className="text-slate-300 dark:text-slate-600 mb-4" />
              <p className="text-sm text-slate-500">暂不支持预览此文件类型</p>
              <p className="text-xs text-slate-400 mt-1">可在外部程序中打开查看</p>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
