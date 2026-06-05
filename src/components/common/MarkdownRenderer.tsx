import { useState, useMemo, useEffect, useRef } from "react";
import { Hash, Link2, ChevronRight, ChevronDown } from "lucide-react";
import type { CitationMeta } from "@/components/common/CitationTag";
import { useEditorStore } from "@/stores/useEditorStore";
import { useKBStore } from "@/stores/useKBStore";
import { invoke } from "@tauri-apps/api/core";

interface MarkdownRendererProps {
  content: string;
  hideFrontmatter?: boolean;
  className?: string;
  citations?: CitationMeta[];
}

interface TocItem {
  level: number;
  text: string;
  id: string;
}

export default function MarkdownRenderer({ content, hideFrontmatter = true, className = "", citations }: MarkdownRendererProps) {
  const [tocCollapsed, setTocCollapsed] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);

  const { frontmatter, body } = useMemo(() => {
    let fm: Record<string, any> = {};
    let mainContent = content;

    if (content.startsWith("---")) {
      const endIdx = content.indexOf("---", 3);
      if (endIdx > 0) {
        const fmText = content.slice(3, endIdx).trim();
        mainContent = content.slice(endIdx + 3).trim();
        fmText.split("\n").forEach((line) => {
          const colonIdx = line.indexOf(":");
          if (colonIdx > 0) {
            const key = line.slice(0, colonIdx).trim();
            const value = line.slice(colonIdx + 1).trim();
            fm[key] = value;
          }
        });
      }
    }

    return { frontmatter: fm, body: mainContent };
  }, [content]);

  const renderMarkdown = (md: string): string => {
    const toc: TocItem[] = []; // 渲染过程中收集的标题，供外部可用但当前未使用
    const lines = md.split("\n");
    let html = "";
    let inCodeBlock = false;
    let codeLang = "";
    let codeContent = "";
    let inTable = false;
    let tableHtml = "";
    let inList = false;
    let listType = "";

    for (let i = 0; i < lines.length; i++) {
      const line = lines[i];

      // Code block
      if (line.trim().startsWith("```")) {
        if (inCodeBlock) {
          html += `<pre class="bg-slate-100 rounded-lg p-4 overflow-x-auto text-xs my-3"><code class="language-${codeLang}">${escapeHtml(codeContent.trimEnd())}</code></pre>`;
          codeContent = "";
          inCodeBlock = false;
        } else {
          codeLang = line.trim().slice(3).trim();
          inCodeBlock = true;
        }
        if (inTable) { html += renderTable(tableHtml); tableHtml = ""; inTable = false; }
        if (inList) { html += `</${listType}>`; inList = false; listType = ""; }
        continue;
      }

      if (inCodeBlock) {
        codeContent += line + "\n";
        continue;
      }

      // Table
      if (line.startsWith("|") && line.endsWith("|")) {
        if (!inTable) {
          inTable = true;
          if (inList) { html += `</${listType}>`; inList = false; listType = ""; }
        }
        tableHtml += line + "\n";
        continue;
      } else if (inTable && line.trim().length > 0 && !line.startsWith("|")) {
        html += renderTable(tableHtml);
        tableHtml = "";
        inTable = false;
      } else if (inTable && line.trim().length === 0) {
        html += renderTable(tableHtml);
        tableHtml = "";
        inTable = false;
        continue;
      }

      // Empty line
      if (line.trim().length === 0) {
        if (inList) { html += `</${listType}>`; inList = false; listType = ""; }
        html += "<div class='h-2'></div>";
        continue;
      }

      // Headers
      if (line.startsWith("### ")) {
        if (inList) { html += `</${listType}>`; inList = false; listType = ""; }
        const text = line.slice(4).trim();
        toc.push({ level: 3, text, id: slugify(text) });
        html += `<h3 class="text-base font-semibold text-slate-800 mt-5 mb-2" id="${slugify(text)}"><a href="#${slugify(text)}" class="text-brand-500 opacity-0 hover:opacity-100 absolute -ml-5 pr-1">#</a>${renderInline(text)}</h3>`;
        continue;
      }
      if (line.startsWith("## ")) {
        if (inList) { html += `</${listType}>`; inList = false; listType = ""; }
        const text = line.slice(3).trim();
        toc.push({ level: 2, text, id: slugify(text) });
        html += `<h2 class="text-lg font-semibold text-slate-800 mt-6 mb-2 pb-1 border-b border-slate-200" id="${slugify(text)}"><a href="#${slugify(text)}" class="text-brand-500 opacity-0 hover:opacity-100 absolute -ml-5 pr-1">#</a>${renderInline(text)}</h2>`;
        continue;
      }
      if (line.startsWith("# ")) {
        if (inList) { html += `</${listType}>`; inList = false; listType = ""; }
        const text = line.slice(2).trim();
        toc.push({ level: 1, text, id: slugify(text) });
        html += `<h1 class="text-xl font-bold text-slate-900 mt-4 mb-3" id="${slugify(text)}">${renderInline(text)}</h1>`;
        continue;
      }

      // Blockquote
      if (line.startsWith("> ")) {
        if (inList) { html += `</${listType}>`; inList = false; listType = ""; }
        html += `<blockquote class="border-l-4 border-brand-300 bg-brand-50/50 pl-4 py-1 my-2 text-sm text-slate-600">${renderInline(line.slice(2).trim())}</blockquote>`;
        continue;
      }

      // Horizontal rule
      if (line.trim() === "---" || line.trim() === "***") {
        if (inList) { html += `</${listType}>`; inList = false; listType = ""; }
        html += `<hr class="my-4 border-slate-200" />`;
        continue;
      }

      // Unordered list
      if (line.match(/^[\s]*[-*+]\s/)) {
        if (!inList || listType !== "ul") {
          if (inList) html += `</${listType}>`;
          html += `<ul class="list-disc list-inside space-y-1 my-2 text-sm text-slate-700">`;
          inList = true;
          listType = "ul";
        }
        const content = line.replace(/^[\s]*[-*+]\s/, "");
        html += `<li>${renderInline(content)}</li>`;
        continue;
      }

      // Ordered list
      if (line.match(/^[\s]*\d+\.\s/)) {
        if (!inList || listType !== "ol") {
          if (inList) html += `</${listType}>`;
          html += `<ol class="list-decimal list-inside space-y-1 my-2 text-sm text-slate-700">`;
          inList = true;
          listType = "ol";
        }
        const content = line.replace(/^[\s]*\d+\.\s/, "");
        html += `<li>${renderInline(content)}</li>`;
        continue;
      }

      // Regular paragraph
      if (inList) { html += `</${listType}>`; inList = false; listType = ""; }
      html += `<p class="text-sm text-slate-700 leading-relaxed my-1">${renderInline(line)}</p>`;
    }

    if (inCodeBlock) {
      html += `<pre class="bg-slate-100 rounded-lg p-4 overflow-x-auto text-xs my-3"><code>${escapeHtml(codeContent.trimEnd())}</code></pre>`;
    }
    if (inTable) { html += renderTable(tableHtml); }
    if (inList) { html += `</${listType}>`; }

    return html;
  };

  const renderTable = (tableText: string): string => {
    const rows = tableText.trim().split("\n");
    if (rows.length < 2) return "";
    let html = '<div class="overflow-x-auto my-3"><table class="min-w-full text-sm border-collapse">';
    rows.forEach((row, idx) => {
      const cells = row.split("|").filter((c) => c.trim().length > 0);
      if (idx === 1 && cells.every((c) => c.trim().match(/^[-:]+$/))) return; // separator
      const tag = idx === 0 ? "th" : "td";
      const cellClass = idx === 0 ? "bg-slate-50 font-medium text-slate-700 px-3 py-2 border border-slate-200" : "text-slate-600 px-3 py-2 border border-slate-200";
      html += "<tr>";
      cells.forEach((cell) => {
        html += `<${tag} class="${cellClass}">${renderInline(cell.trim())}</${tag}>`;
      });
      html += "</tr>";
    });
    html += "</table></div>";
    return html;
  };

  const renderInline = (text: string): string => {
    let result = escapeHtml(text);
    // Citation references: [1], [1-3], [1,2,3] — before code to avoid conflict with backtick pattern
    result = result.replace(/\[(\d+(?:[-,]\d+)*)\]/g, '<span class="citation-tag inline-flex items-center gap-0.5 bg-primary/10 text-primary rounded px-1 text-[11px] font-medium cursor-pointer align-text-top leading-none pt-px hover:bg-primary/20 active:bg-primary/30 transition-colors select-none" data-citation-ref="$1">[$1]</span>');
    // Code
    result = result.replace(/`([^`]+)`/g, '<code class="bg-slate-100 text-red-600 px-1 py-0.5 rounded text-xs font-mono">$1</code>');
    // Bold + Italic
    result = result.replace(/\*\*\*(.+?)\*\*\*/g, '<strong><em>$1</em></strong>');
    // Bold
    result = result.replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>');
    // Italic
    result = result.replace(/\*(.+?)\*/g, '<em>$1</em>');
    // Wiki links [[page]]
    result = result.replace(/\[\[([^\]]+)\]\]/g, '<span class="text-brand-600 underline cursor-pointer hover:text-brand-800" data-wiki-link="$1">$1</span>');
    // Links (filter javascript: URLs)
    result = result.replace(/\[([^\]]+)\]\(([^)]+)\)/g, (_m: string, label: string, href: string) => {
      const safe = /^\s*(javascript|data)\s*:/i.test(href) ? "#" : href;
      return `<a href="${safe}" class="text-brand-600 underline hover:text-brand-800" target="_blank" rel="noopener">${label}</a>`;
    });
    // Images
    result = result.replace(/!\[([^\]]*)\]\(([^)]+)\)/g, '<span class="inline-block text-xs text-slate-400 italic">[图片: $1]</span>');
    return result;
  };

  const slugify = (text: string): string => {
    return text.toLowerCase().replace(/[^\w\u4e00-\u9fa5]+/g, "-").replace(/^-|-$/g, "");
  };

  const escapeHtml = (text: string): string => {
    return text.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
  };

  const displayContent = hideFrontmatter ? body : content;
  const bodyHtml = renderMarkdown(displayContent);
  const bodyToc = useMemo(() => {
    const items: TocItem[] = [];
    const lines = displayContent.split("\n");
    for (const line of lines) {
      if (line.startsWith("### ")) items.push({ level: 3, text: line.slice(4).trim(), id: slugify(line.slice(4).trim()) });
      else if (line.startsWith("## ")) items.push({ level: 2, text: line.slice(3).trim(), id: slugify(line.slice(3).trim()) });
      else if (line.startsWith("# ")) items.push({ level: 1, text: line.slice(2).trim(), id: slugify(line.slice(2).trim()) });
    }
    return items;
  }, [displayContent]);

  // Post-render: wire click handlers for citation refs and wiki links
  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    const handleCitationClick = (el: HTMLElement) => {
      const ref = el.getAttribute("data-citation-ref");
      if (!ref || !citations) return;
      const idx = parseInt(ref.split(",")[0], 10);
      const meta = citations.find((c) => c.index === idx);
      if (meta) {
        window.dispatchEvent(new CustomEvent("citation:click", { detail: meta, bubbles: true }));
      }
    };

    const handleWikiLinkClick = async (el: HTMLElement) => {
      const pageName = el.getAttribute("data-wiki-link");
      if (!pageName) return;
      const kb = useKBStore.getState().currentKB;
      if (!kb) {
        // Fallback: slugify and open as wiki page
        const slug = pageName.toLowerCase().replace(/[^\w一-龥]+/g, "-").replace(/^-|-$/g, "");
        useEditorStore.getState().openFile({
          path: `wiki/concepts/${slug}.md`,
          title: pageName,
          type: "wiki",
          content: "",
        });
        return;
      }
      try {
        const resolved = await invoke<any>("resolve_wiki_link", {
          kbId: kb.id,
          kbPath: kb.path,
          linkText: pageName,
        });
        if (resolved) {
          useEditorStore.getState().openFile({
            path: resolved.path,
            title: resolved.title,
            type: "wiki",
            content: resolved.content || "",
          });
        } else {
          // Page doesn't exist yet — open a placeholder with slugified path
          const slug = pageName.toLowerCase().replace(/[^\w一-龥]+/g, "-").replace(/^-|-$/g, "");
          useEditorStore.getState().openFile({
            path: `wiki/concepts/${slug}.md`,
            title: pageName,
            type: "wiki",
            content: "",
          });
        }
      } catch {
        const slug = pageName.toLowerCase().replace(/[^\w一-龥]+/g, "-").replace(/^-|-$/g, "");
        useEditorStore.getState().openFile({
          path: `wiki/concepts/${slug}.md`,
          title: pageName,
          type: "wiki",
          content: "",
        });
      }
    };

    const onClick = (e: MouseEvent) => {
      const target = e.target as HTMLElement;
      const citationEl = target.closest("[data-citation-ref]") as HTMLElement | null;
      if (citationEl) {
        e.preventDefault();
        handleCitationClick(citationEl);
        return;
      }
      const wikiEl = target.closest("[data-wiki-link]") as HTMLElement | null;
      if (wikiEl) {
        e.preventDefault();
        handleWikiLinkClick(wikiEl);
      }
    };

    container.addEventListener("click", onClick);
    return () => container.removeEventListener("click", onClick);
  }, [bodyHtml, citations]);

  return (
    <div className={`${className}`}>
      {bodyToc.length > 0 && (
        <div className="mb-6 bg-slate-50 border border-slate-200 rounded-lg overflow-hidden">
          <button
            onClick={() => setTocCollapsed(!tocCollapsed)}
            className="flex items-center gap-2 px-4 py-2 w-full text-sm font-medium text-slate-600 hover:bg-slate-100"
          >
            {tocCollapsed ? <ChevronRight size={14} /> : <ChevronDown size={14} />}
            目录
          </button>
          {!tocCollapsed && (
            <div className="px-4 pb-3 space-y-0.5">
              {bodyToc.map((item) => (
                <a
                  key={item.id}
                  href={`#${item.id}`}
                  className={`block text-sm text-slate-500 hover:text-brand-600 truncate ${
                    item.level === 1 ? "font-medium" : item.level === 2 ? "pl-0" : "pl-6 text-xs"
                  }`}
                  onClick={(e) => {
                    e.preventDefault();
                    document.getElementById(item.id)?.scrollIntoView({ behavior: "smooth" });
                  }}
                >
                  {item.text}
                </a>
              ))}
            </div>
          )}
        </div>
      )}
      <div
        ref={containerRef}
        className="prose prose-slate max-w-none"
        dangerouslySetInnerHTML={{ __html: bodyHtml }}
      />
    </div>
  );
}

export function parseFrontmatter(content: string): { frontmatter: Record<string, any>; body: string } {
  let fm: Record<string, any> = {};
  let mainContent = content;

  if (content.startsWith("---")) {
    const endIdx = content.indexOf("---", 3);
    if (endIdx > 0) {
      const fmText = content.slice(3, endIdx).trim();
      mainContent = content.slice(endIdx + 3).trim();
      fmText.split("\n").forEach((line) => {
        const colonIdx = line.indexOf(":");
        if (colonIdx > 0) {
          const key = line.slice(0, colonIdx).trim();
          let value = line.slice(colonIdx + 1).trim();
          // 去掉引号
          if ((value.startsWith('"') && value.endsWith('"')) || (value.startsWith("'") && value.endsWith("'"))) {
            value = value.slice(1, -1);
          }
          // 处理数组
          if (value.startsWith("[") && value.endsWith("]")) {
            const arrStr = value.slice(1, -1);
            fm[key] = arrStr.split(",").map((s) => s.trim().replace(/['"]/g, ""));
          } else {
            fm[key] = value;
          }
        }
      });
    }
  }

  return { frontmatter: fm, body: mainContent };
}
