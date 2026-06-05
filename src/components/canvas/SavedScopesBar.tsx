import { useState, useRef, useEffect } from "react";
import { useKBStore } from "@/stores/useKBStore";
import { useCanvasStore } from "@/stores/useCanvasStore";
import { Bookmark, MoreHorizontal, Pencil, Trash2, Check, X } from "lucide-react";

export default function SavedScopesBar() {
  const currentKB = useKBStore((s) => s.currentKB);
  const savedScopes = useCanvasStore((s) => s.savedScopes);
  const loadingScopes = useCanvasStore((s) => s.loadingScopes);
  const loadSavedScopes = useCanvasStore((s) => s.loadSavedScopes);
  const deleteScope = useCanvasStore((s) => s.deleteScope);
  const renameScope = useCanvasStore((s) => s.renameScope);
  const setTags = useCanvasStore((s) => s.setTags);
  const setScrollPosition = useCanvasStore((s) => s.setScrollPosition);
  const checkScope = useCanvasStore((s) => s.checkScope);
  const generateOutline = useCanvasStore((s) => s.generateOutline);
  const setGenerationError = useCanvasStore((s) => s.setGenerationError);
  const setGenerationPhase = useCanvasStore((s) => s.setGenerationPhase);

  const [menuOpenId, setMenuOpenId] = useState<string | null>(null);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editName, setEditName] = useState("");
  const menuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (currentKB) loadSavedScopes(currentKB.id);
  }, [currentKB?.id]);

  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        setMenuOpenId(null);
      }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, []);

  const handleRestoreScope = async (scope: (typeof savedScopes)[number]) => {
    if (!currentKB) return;
    setTags(scope.tags);
    setScrollPosition(scope.last_scroll_position);

    // Auto-restore from cache (no LLM call — cache_key is deterministic)
    try {
      const result = await checkScope(currentKB.id, scope.tags);
      if (result.blocked) {
        setGenerationError(result.message || "视域过大");
        return;
      }
      // The backend checks canvas_cache first, so this will restore instantly from SQLite
      await generateOutline(currentKB.id, result.cache_key);
    } catch (e) {
      setGenerationError(String(e));
      setGenerationPhase("idle");
    }
  };

  const handleStartRename = (scope: (typeof savedScopes)[number]) => {
    setEditingId(scope.id);
    setEditName(scope.name);
    setMenuOpenId(null);
  };

  const handleConfirmRename = async (scopeId: string) => {
    if (editName.trim()) {
      await renameScope(scopeId, editName.trim());
      if (currentKB) loadSavedScopes(currentKB.id);
    }
    setEditingId(null);
  };

  const handleDelete = async (scopeId: string) => {
    await deleteScope(scopeId);
    if (currentKB) loadSavedScopes(currentKB.id);
    setMenuOpenId(null);
  };

  if (loadingScopes) {
    return (
      <div className="flex items-center gap-1.5 px-4 py-2 text-xs text-muted-foreground">
        <Bookmark size={12} />
        加载书签中...
      </div>
    );
  }

  if (savedScopes.length === 0) {
    return (
      <div className="flex items-center gap-1.5 px-4 py-2 text-xs text-muted-foreground">
        <Bookmark size={12} />
        暂无保存的画布视域 — 选择标签并生成后，可保存为快捷书签
      </div>
    );
  }

  return (
    <div className="flex items-center gap-1.5 px-4 py-2 overflow-x-auto shrink-0" ref={menuRef}>
      <Bookmark size={12} className="text-muted-foreground shrink-0" />
      {savedScopes.map((scope) => (
        <div key={scope.id} className="relative shrink-0">
          {editingId === scope.id ? (
            <div className="flex items-center gap-1 px-2 py-1 bg-muted rounded-full">
              <input
                type="text"
                value={editName}
                onChange={(e) => setEditName(e.target.value)}
                className="w-24 bg-transparent border-none outline-none text-xs"
                autoFocus
                onKeyDown={(e) => {
                  if (e.key === "Enter") handleConfirmRename(scope.id);
                  if (e.key === "Escape") setEditingId(null);
                }}
              />
              <button onClick={() => handleConfirmRename(scope.id)}>
                <Check size={12} className="text-green-500" />
              </button>
              <button onClick={() => setEditingId(null)}>
                <X size={12} className="text-muted-foreground" />
              </button>
            </div>
          ) : (
            <button
              type="button"
              onClick={() => handleRestoreScope(scope)}
              onContextMenu={(e) => {
                e.preventDefault();
                setMenuOpenId(menuOpenId === scope.id ? null : scope.id);
              }}
              className="flex items-center gap-1 px-2.5 py-1 bg-muted hover:bg-muted/80 rounded-full text-xs text-foreground transition-colors whitespace-nowrap"
              title={`标签: ${scope.tags.join(", ")}`}
            >
              {scope.name}
              <button
                type="button"
                onClick={(e) => {
                  e.stopPropagation();
                  setMenuOpenId(menuOpenId === scope.id ? null : scope.id);
                }}
                className="ml-0.5 hover:text-foreground text-muted-foreground"
              >
                <MoreHorizontal size={12} />
              </button>
            </button>
          )}

          {/* Dropdown menu */}
          {menuOpenId === scope.id && (
            <div className="absolute top-full mt-1 right-0 w-32 bg-popover border border-border rounded-lg shadow-lg z-50 py-1">
              <button
                type="button"
                onClick={() => handleStartRename(scope)}
                className="flex items-center gap-2 w-full px-3 py-1.5 text-xs hover:bg-muted text-left"
              >
                <Pencil size={12} /> 重命名
              </button>
              <button
                type="button"
                onClick={() => handleDelete(scope.id)}
                className="flex items-center gap-2 w-full px-3 py-1.5 text-xs hover:bg-destructive/10 text-destructive text-left"
              >
                <Trash2 size={12} /> 删除
              </button>
            </div>
          )}
        </div>
      ))}
    </div>
  );
}
