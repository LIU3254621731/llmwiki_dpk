import { useState, useCallback, useEffect, useRef } from "react";
import { useKBStore } from "@/stores/useKBStore";
import { useCanvasStore } from "@/stores/useCanvasStore";
import { useAppStore } from "@/stores/useAppStore";
import SmartTagInput from "@/components/canvas/SmartTagInput";
import SavedScopesBar from "@/components/canvas/SavedScopesBar";
import { Save, GitGraph } from "lucide-react";

export default function CanvasHeader() {
  const currentKB = useKBStore((s) => s.currentKB);
  const tags = useCanvasStore((s) => s.tags);
  const generationLock = useCanvasStore((s) => s.generationLock);
  const generationPhase = useCanvasStore((s) => s.generationPhase);
  const outlineNodes = useCanvasStore((s) => s.outlineNodes);
  const checkScope = useCanvasStore((s) => s.checkScope);
  const generateOutline = useCanvasStore((s) => s.generateOutline);
  const triggerTextbookGeneration = useCanvasStore((s) => s.triggerTextbookGeneration);
  const saveScope = useCanvasStore((s) => s.saveScope);
  const setGenerationError = useCanvasStore((s) => s.setGenerationError);
  const setGenerationPhase = useCanvasStore((s) => s.setGenerationPhase);
  const setCanvasBadgeDot = useAppStore((s) => s.setCanvasBadgeDot);
  const generateMindmap = useCanvasStore((s) => s.generateMindmap);
  const generateMindmapFromTextbook = useCanvasStore((s) => s.generateMindmapFromTextbook);
  const mindmapLoading = useCanvasStore((s) => s.mindmapLoading);
  const webGenerationActive = useCanvasStore((s) => s.webGenerationActive);
  const textbookContent = useCanvasStore((s) => s.textbookContent);

  const [cacheKey, setCacheKey] = useState("");
  const [saving, setSaving] = useState(false);
  const autoTriggeredRef = useRef(false);

  const handleGenerate = useCallback(async () => {
    if (!currentKB || tags.length === 0) return;
    setGenerationError("");
    autoTriggeredRef.current = false;

    try {
      const result = await checkScope(currentKB.id, tags);
      if (result.blocked) {
        setGenerationError(result.message || "视域过大");
        return;
      }
      const ck = result.cache_key;
      setCacheKey(ck);
      await generateOutline(currentKB.id, ck);
      setCanvasBadgeDot(true);
      // After outline succeeds, the generationPhase becomes "done" for the outline stage,
      // but we now auto-trigger textbook. Check if we should.
      // The generateOutline action now sets phase=outline and lock=true, then phase stays "outline"
      // after completion. We need to auto-trigger textbook.
    } catch (e) {
      setGenerationError(String(e));
      setGenerationPhase("idle");
    }
  }, [currentKB, tags, checkScope, generateOutline, setGenerationError, setGenerationPhase, setCanvasBadgeDot]);

  // Auto-trigger textbook generation after outline is complete
  useEffect(() => {
    if (
      generationPhase === "outline" &&
      outlineNodes.length > 0 &&
      currentKB &&
      !autoTriggeredRef.current
    ) {
      autoTriggeredRef.current = true;
      triggerTextbookGeneration(currentKB.id);
    }
  }, [generationPhase, outlineNodes.length, currentKB?.id, triggerTextbookGeneration]);

  const handleGenerateMindmap = useCallback(async () => {
    if (!currentKB || tags.length === 0) return;
    if (webGenerationActive && textbookContent) {
      await generateMindmapFromTextbook(currentKB.id);
    } else {
      await generateMindmap(currentKB.id);
    }
  }, [currentKB, tags, generateMindmap, generateMindmapFromTextbook, webGenerationActive, textbookContent]);

  const handleSaveScope = async () => {
    if (!currentKB || tags.length === 0) return;
    setSaving(true);
    const name = tags.join("+");
    await saveScope(currentKB.id, name);
    setSaving(false);
  };

  return (
    <div className="shrink-0 border-b border-border">
      {/* Top row: tag input + actions */}
      <div className="flex items-center gap-2 px-4 py-3">
        <SmartTagInput />

        <button
          type="button"
          onClick={handleGenerate}
          disabled={generationLock || tags.length === 0 || !currentKB}
          className="px-4 py-2 bg-primary text-primary-foreground text-sm font-medium rounded-lg hover:bg-primary/90 disabled:opacity-50 disabled:cursor-not-allowed transition-colors shrink-0"
        >
          {generationLock ? "生成中..." : "生成教材"}
        </button>

        <button
          type="button"
          onClick={handleGenerateMindmap}
          disabled={generationLock || tags.length === 0 || !currentKB || mindmapLoading}
          className="flex items-center gap-1 px-3 py-2 bg-emerald-600 text-white text-sm font-medium rounded-lg hover:bg-emerald-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors shrink-0"
        >
          {mindmapLoading ? "生成中..." : <><GitGraph size={16} /> 思维导图</>}
        </button>

        <button
          type="button"
          onClick={handleSaveScope}
          disabled={tags.length === 0 || saving}
          className="flex items-center gap-1 px-3 py-2 bg-muted text-sm rounded-lg hover:bg-muted/80 disabled:opacity-50 transition-colors shrink-0"
          title="保存当前画布视域"
        >
          <Save size={16} />
        </button>
      </div>

      {/* Bottom row: saved scopes bar */}
      <SavedScopesBar />
    </div>
  );
}
