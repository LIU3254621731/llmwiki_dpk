export interface CitationMeta {
  index: number;
  sourceId: string;
  page?: number;
  targetType: "pdf" | "md" | "docx" | "wiki_page";
  fileName: string;
}

interface CitationTagProps {
  citation: CitationMeta;
}

export function CitationTag({ citation }: CitationTagProps) {
  const handleClick = (e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    window.dispatchEvent(
      new CustomEvent("citation:click", {
        detail: citation,
        bubbles: true,
      })
    );
  };

  const pageSuffix = citation.page ? `, Page ${citation.page}` : "";

  return (
    <span
      className="citation-tag inline-flex items-center gap-0.5 bg-primary/10 text-primary rounded px-1 text-[11px] font-medium cursor-pointer align-text-top leading-none pt-px hover:bg-primary/20 hover:text-primary-hover active:bg-primary/30 transition-colors select-none"
      data-source-id={citation.sourceId}
      data-page={citation.page}
      data-target-type={citation.targetType}
      onClick={handleClick}
      title={`${citation.fileName}${pageSuffix} — 点击跳转`}
      role="button"
      tabIndex={0}
      onKeyDown={(e) => { if (e.key === "Enter") handleClick(e as any); }}
    >
      [{citation.index}]
    </span>
  );
}
