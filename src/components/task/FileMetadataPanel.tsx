import { Eye, FileText, Hash, Weight, Type, Puzzle } from "lucide-react";
import { formatSize } from "@/lib/utils";
import type { SourceMeta } from "@/types/task";

interface FileMetadataPanelProps {
  sourceMeta: SourceMeta | null | undefined;
  onPreview: () => void;
}

function truncateHash(hash: string): string {
  if (hash.length <= 12) return hash;
  return `${hash.slice(0, 8)}...${hash.slice(-4)}`;
}

export default function FileMetadataPanel({ sourceMeta, onPreview }: FileMetadataPanelProps) {
  if (!sourceMeta) {
    return (
      <div className="px-4 py-3 bg-card border-b border-border text-sm text-muted-foreground">
        暂无文件元数据
      </div>
    );
  }

  const items = [
    { icon: FileText, label: "文件名", value: sourceMeta.file_name },
    { icon: Hash, label: "Hash", value: truncateHash(sourceMeta.file_hash), mono: true },
    { icon: Weight, label: "大小", value: formatSize(sourceMeta.file_size) },
    { icon: Type, label: "字符数", value: sourceMeta.text_length.toLocaleString() },
  ];

  if (sourceMeta.page_count && sourceMeta.page_count > 0) {
    items.push({ icon: Puzzle, label: "页数", value: String(sourceMeta.page_count) });
  }

  return (
    <div className="flex items-center flex-wrap gap-4 px-5 py-3 bg-card border-b border-border">
      {items.map((item) => {
        const Icon = item.icon;
        return (
          <div key={item.label} className="flex items-center gap-1.5 text-sm">
            <Icon size={14} className="text-muted-foreground" />
            <span className="text-muted-foreground">{item.label}:</span>
            <span className={item.mono ? "font-mono text-xs" : "font-medium"}>
              {item.value}
            </span>
          </div>
        );
      })}
      <button
        onClick={onPreview}
        className="ml-auto inline-flex items-center gap-1.5 px-3 py-1.5 rounded-md border border-border hover:bg-accent text-sm transition-colors"
      >
        <Eye size={14} />
        预览文件
      </button>
    </div>
  );
}
