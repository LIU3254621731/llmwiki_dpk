import { Handle, Position, type NodeProps } from "reactflow";

// ── Color palette by node type ──
const PALETTE: Record<string, { bg: string; border: string; text: string }> = {
  entity:   { bg: "#ecfdf5", border: "#34d399", text: "#065f46" },
  concept:  { bg: "#fffbeb", border: "#fbbf24", text: "#92400e" },
  topic:    { bg: "#f5f3ff", border: "#a78bfa", text: "#5b21b6" },
  person:   { bg: "#ecfeff", border: "#22d3ee", text: "#155e75" },
  source:   { bg: "#fff7ed", border: "#fb923c", text: "#9a3412" },
  question: { bg: "#fff1f2", border: "#fda4af", text: "#9f1239" },
  wikipage: { bg: "#eef2ff", border: "#818cf8", text: "#3730a3" },
  review:   { bg: "#faf5ff", border: "#c084fc", text: "#6b21a8" },
  dataset:  { bg: "#ecfdf5", border: "#6ee7b7", text: "#065f46" },
  method:   { bg: "#faf5ff", border: "#a78bfa", text: "#6b21a8" },
  default:  { bg: "#f8fafc", border: "#cbd5e1", text: "#475569" },
};

const TYPE_LABELS: Record<string, string> = {
  entity: "实体", concept: "概念", topic: "主题", person: "人物",
  source: "来源", wikipage: "Wiki", question: "问题", dataset: "数据集",
  method: "方法", review: "审阅",
};

function truncate(text: string, maxLen = 8): string {
  if (text.length <= maxLen) return text;
  return text.slice(0, maxLen) + "...";
}

/** A polished hierarchical node for the knowledge graph. */
export default function HierarchicalNode({ data, selected }: NodeProps) {
  const colors = PALETTE[data.nodeType] || PALETTE.default;
  const label: string = data.label || "?";
  const displayLabel = truncate(label);
  const needsReview = data.nodeType === "review" || data.status === "review";
  const isOrphan = data.status === "orphan";

  return (
    <div
      className="hierarchical-node group relative"
      title={label}
      style={{ minWidth: 90 }}
    >
      {/* ── Badges ── */}
      {needsReview && (
        <span
          className="absolute -top-1.5 -right-1.5 w-3 h-3 rounded-full z-10 animate-pulse"
          style={{
            background: "#f97316",
            boxShadow: "0 0 8px rgba(249,115,22,0.7)",
          }}
        />
      )}
      {isOrphan && !needsReview && (
        <span
          className="absolute -top-1 -right-1 w-2.5 h-2.5 rounded-full z-10"
          style={{
            background: "#fbbf24",
            boxShadow: "0 0 4px rgba(251,191,36,0.5)",
          }}
        />
      )}

      {/* ── Card body ── */}
      <div
        style={{
          background: `linear-gradient(135deg, ${colors.bg} 0%, #fff 100%)`,
          border: `2px solid ${selected ? "#3b82f6" : colors.border}`,
          borderRadius: 10,
          padding: "10px 14px",
          boxShadow: selected
            ? "0 0 0 3px rgba(59,130,246,0.25), 0 4px 12px rgba(0,0,0,0.1)"
            : "0 2px 6px rgba(0,0,0,0.06)",
          transition: "box-shadow 0.15s, border-color 0.15s",
        }}
      >
        {/* Handles */}
        <Handle
          type="target"
          position={Position.Top}
          style={{ background: colors.border, width: 8, height: 8, border: "2px solid white" }}
        />

        <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
          {/* Type dot */}
          <span
            style={{
              width: 8,
              height: 8,
              borderRadius: "50%",
              flexShrink: 0,
              background: colors.border,
            }}
          />

          {/* Label */}
          <span
            style={{
              fontSize: 12,
              fontWeight: 600,
              color: colors.text,
              maxWidth: 110,
              overflow: "hidden",
              textOverflow: "ellipsis",
              whiteSpace: "nowrap",
              lineHeight: 1.3,
            }}
          >
            {displayLabel}
          </span>
        </div>

        {/* Type tag + wiki indicator */}
        <div
          style={{
            fontSize: 9,
            color: "#94a3b8",
            marginTop: 4,
            display: "flex",
            alignItems: "center",
            gap: 4,
          }}
        >
          <span>{TYPE_LABELS[data.nodeType] || data.nodeType}</span>
          {data.path && (
            <span style={{ color: "#22c55e", fontWeight: 600 }}>W</span>
          )}
        </div>

        <Handle
          type="source"
          position={Position.Bottom}
          style={{ background: colors.border, width: 8, height: 8, border: "2px solid white" }}
        />
      </div>
    </div>
  );
}
