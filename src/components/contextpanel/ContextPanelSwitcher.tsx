import { ListTree, ArrowLeftRight, GitGraph } from "lucide-react";
import type { ContextPanelMode } from "@/stores/useContextPanelStore";

interface ContextPanelSwitcherProps {
  activeMode: ContextPanelMode;
  onModeChange: (mode: ContextPanelMode) => void;
}

const tabs: { mode: ContextPanelMode; label: string; icon: typeof ListTree }[] = [
  { mode: "outline", label: "大纲", icon: ListTree },
  { mode: "backlinks", label: "反向链接", icon: ArrowLeftRight },
  { mode: "local_graph", label: "关系", icon: GitGraph },
];

export default function ContextPanelSwitcher({ activeMode, onModeChange }: ContextPanelSwitcherProps) {
  return (
    <div className="flex items-center border-b border-slate-200">
      {tabs.map(({ mode, label, icon: Icon }) => (
        <button
          key={mode}
          type="button"
          onClick={() => onModeChange(mode)}
          className={`flex items-center gap-1 px-3 py-1.5 text-xs transition-colors ${
            activeMode === mode
              ? "text-brand-600 border-b-2 border-brand-500"
              : "text-slate-400 hover:text-slate-600"
          }`}
          title={label}
        >
          <Icon size={14} />
          <span>{label}</span>
        </button>
      ))}
    </div>
  );
}
