import { useCanvasStore } from "@/stores/useCanvasStore";
import { Loader2, CheckCircle2, Circle } from "lucide-react";

const PHASES: { key: string; label: string }[] = [
  { key: "outline", label: "生成知识大纲" },
  { key: "textbook", label: "撰写教材长文" },
  { key: "done", label: "完成" },
];

export default function GenerationLockOverlay() {
  const generationPhase = useCanvasStore((s) => s.generationPhase);
  const resetGeneration = useCanvasStore((s) => s.resetGeneration);

  const currentIdx = PHASES.findIndex((p) => p.key === generationPhase);
  const isDone = generationPhase === "done";

  return (
    <div className="absolute inset-0 z-50 flex items-center justify-center bg-black/30 backdrop-blur-sm">
      <div className="bg-card border border-border rounded-2xl shadow-2xl px-8 py-6 min-w-[340px]">
        <h3 className="text-base font-semibold text-foreground mb-5 text-center">
          {isDone ? "生成完成" : "AI 知识重组进行中"}
        </h3>

        {/* Phase indicators */}
        <div className="space-y-3 mb-6">
          {PHASES.map((phase, idx) => {
            const isActive = idx === currentIdx;
            const isCompleted = idx < currentIdx || (isDone && idx <= currentIdx);

            return (
              <div
                key={phase.key}
                className={`flex items-center gap-3 px-3 py-2 rounded-lg transition-colors ${
                  isActive ? "bg-primary/10" : ""
                }`}
              >
                {isCompleted ? (
                  <CheckCircle2 size={18} className="text-green-500 shrink-0" />
                ) : isActive ? (
                  <Loader2 size={18} className="text-primary animate-spin shrink-0" />
                ) : (
                  <Circle size={18} className="text-muted-foreground/40 shrink-0" />
                )}
                <span
                  className={`text-sm ${
                    isActive
                      ? "text-primary font-medium"
                      : isCompleted
                        ? "text-green-600"
                        : "text-muted-foreground"
                  }`}
                >
                  {phase.label}
                </span>
              </div>
            );
          })}
        </div>

        {/* Note */}
        {!isDone && (
          <p className="text-xs text-muted-foreground text-center mb-4">
            生成过程中请勿切换页面或修改标签，以避免中断 AI 工作流
          </p>
        )}

        {/* Close button (only when done) */}
        {isDone && (
          <button
            type="button"
            onClick={resetGeneration}
            className="w-full py-2 bg-primary text-primary-foreground text-sm font-medium rounded-lg hover:bg-primary/90 transition-colors"
          >
            关闭
          </button>
        )}
      </div>
    </div>
  );
}
