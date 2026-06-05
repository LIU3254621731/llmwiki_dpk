import { useState, useMemo } from "react";

interface HeadingItem {
  level: number;
  text: string;
  id: string;
}

interface OutlineViewProps {
  content: string;
  activeHeadingId?: string;
  onHeadingClick?: (headingId: string) => void;
}

function parseHeadings(content: string): HeadingItem[] {
  const regex = /^(#{1,6})\s+(.+)$/gm;
  const headings: HeadingItem[] = [];
  let match: RegExpExecArray | null;
  while ((match = regex.exec(content)) !== null) {
    const level = match[1].length;
    const text = match[2].trim();
    const id = text
      .toLowerCase()
      .replace(/[^\w一-鿿]+/g, "-")
      .replace(/^-+|-+$/g, "");
    headings.push({ level, text, id });
  }
  return headings;
}

function headingColor(level: number): string {
  if (level === 1) return "text-slate-700";
  if (level === 2) return "text-slate-600";
  return "text-slate-500";
}

function headingSize(level: number): string {
  if (level === 1) return "text-sm";
  if (level === 2) return "text-xs";
  return "text-[11px]";
}

export default function OutlineView({ content, activeHeadingId, onHeadingClick }: OutlineViewProps) {
  const headings = useMemo(() => parseHeadings(content), [content]);

  if (!content || headings.length === 0) {
    return (
      <div className="py-4 text-center">
        <span className="text-xs text-slate-400 italic">无标题</span>
      </div>
    );
  }

  return (
    <div className="py-1">
      {headings.map((h) => {
        const isActive = activeHeadingId === h.id;
        return (
          <button
            key={h.id + "-" + headings.indexOf(h)}
            type="button"
            onClick={() => onHeadingClick?.(h.id)}
            className={`flex items-center w-full text-left py-0.5 group ${
              isActive ? "bg-slate-100" : "hover:bg-slate-50"
            }`}
            style={{ paddingLeft: `${(h.level - 1) * 16 + 8}px` }}
          >
            <div
              className={`border-l-2 mr-2 self-stretch shrink-0 ${
                isActive ? "border-brand-500" : "border-slate-200"
              }`}
            />
            <span
              className={`truncate ${headingColor(h.level)} ${headingSize(h.level)} ${
                isActive ? "text-slate-900 font-medium" : ""
              }`}
            >
              {h.text}
            </span>
          </button>
        );
      })}
    </div>
  );
}
