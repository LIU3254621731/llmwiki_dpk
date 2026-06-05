export interface BacklinkItem {
  title: string;
  path: string;
  snippet: string;
}

interface BacklinksPanelProps {
  backlinks: BacklinkItem[];
  onNavigate?: (path: string) => void;
}

export default function BacklinksPanel({ backlinks, onNavigate }: BacklinksPanelProps) {
  if (!backlinks || backlinks.length === 0) {
    return (
      <div className="py-4 text-center">
        <span className="text-xs text-slate-400 italic">无反向链接</span>
      </div>
    );
  }

  return (
    <div className="py-1">
      {backlinks.map((bl, i) => (
        <div key={bl.path + "-" + i} className="px-2 py-1.5 border-b border-slate-100 last:border-b-0">
          <button
            type="button"
            onClick={() => onNavigate?.(bl.path)}
            className="text-sm text-brand-600 hover:text-brand-800 text-left w-full truncate"
          >
            {bl.title}
          </button>
          {bl.snippet && (
            <p className="text-[11px] text-slate-400 mt-0.5 line-clamp-2 break-all">
              {bl.snippet}
            </p>
          )}
        </div>
      ))}
    </div>
  );
}
