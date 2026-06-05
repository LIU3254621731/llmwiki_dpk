import { useEffect, useState, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useQuickNavStore } from "@/stores/useQuickNavStore";
import { useKBStore } from "@/stores/useKBStore";

interface SearchResult {
  id: string;
  name: string;
  path: string;
  group: "wiki" | "workspace";
  title?: string;
}

function fuzzyMatch(query: string, target: string): boolean {
  if (!query) return true;
  const lowerQuery = query.toLowerCase();
  const lowerTarget = target.toLowerCase();
  let qi = 0;
  for (let ti = 0; ti < lowerTarget.length && qi < lowerQuery.length; ti++) {
    if (lowerQuery[qi] === lowerTarget[ti]) {
      qi++;
    }
  }
  return qi === lowerQuery.length;
}

function flattenFileTree(
  node: any,
  results: SearchResult[]
) {
  if (node.is_directory && Array.isArray(node.children)) {
    for (const child of node.children) {
      flattenFileTree(child, results);
    }
  } else if (!node.is_directory) {
    results.push({
      id: node.relative_path || node.name,
      name: node.name,
      path: node.relative_path || node.name,
      group: "workspace",
    });
  }
}

export default function QuickSwitcher() {
  const open = useQuickNavStore((s) => s.quickSwitcherOpen);
  const close = useQuickNavStore((s) => s.closeQuickSwitcher);
  const currentKB = useKBStore((s) => s.currentKB);

  const [query, setQuery] = useState("");
  const [wikiPages, setWikiPages] = useState<SearchResult[]>([]);
  const [workspaceFiles, setWorkspaceFiles] = useState<SearchResult[]>([]);
  const [filteredResults, setFilteredResults] = useState<SearchResult[]>([]);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [loading, setLoading] = useState(false);

  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);

  const loadData = useCallback(async () => {
    if (!currentKB) return;
    setLoading(true);
    try {
      const [pagesRaw, fileTreeRaw] = await Promise.all([
        invoke<any[]>("list_wiki_pages", { kbId: currentKB.id }),
        invoke<any>("get_file_tree", {
          kbId: currentKB.id,
          kbPath: currentKB.path,
        }),
      ]);

      const pages: SearchResult[] = pagesRaw.map((p: any) => ({
        id: p.id,
        name: p.title || p.path,
        path: p.path,
        title: p.title,
        group: "wiki" as const,
      }));
      setWikiPages(pages);

      const files: SearchResult[] = [];
      if (fileTreeRaw && (fileTreeRaw as any).root) {
        flattenFileTree((fileTreeRaw as any).root, files);
      }
      setWorkspaceFiles(files);
    } catch (e) {
      console.error("QuickSwitcher: 加载数据失败", e);
    }
    setLoading(false);
  }, [currentKB]);

  useEffect(() => {
    if (open) {
      setQuery("");
      setSelectedIndex(0);
      loadData();
      setTimeout(() => inputRef.current?.focus(), 50);
    }
  }, [open, loadData]);

  useEffect(() => {
    const allResults = [...wikiPages, ...workspaceFiles];
    const filtered = allResults.filter(
      (r) =>
        fuzzyMatch(query, r.name) ||
        fuzzyMatch(query, r.path) ||
        fuzzyMatch(query, r.title || "")
    );
    setFilteredResults(filtered);
    setSelectedIndex(0);
  }, [query, wikiPages, workspaceFiles]);

  const selectItem = useCallback(
    (item: SearchResult) => {
      if (item.group === "wiki") {
        window.location.href = `/wiki/${encodeURIComponent(item.path)}`;
      } else {
        window.location.href = `/files/${encodeURIComponent(item.path)}`;
      }
      close();
    },
    [close]
  );

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setSelectedIndex((prev) =>
        Math.min(prev + 1, filteredResults.length - 1)
      );
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setSelectedIndex((prev) => Math.max(prev - 1, 0));
    } else if (e.key === "Enter") {
      e.preventDefault();
      if (filteredResults[selectedIndex]) {
        selectItem(filteredResults[selectedIndex]);
      }
    } else if (e.key === "Escape") {
      close();
    }
  };

  useEffect(() => {
    if (listRef.current) {
      const selectedEl = listRef.current.children[selectedIndex] as HTMLElement;
      if (selectedEl) {
        selectedEl.scrollIntoView({ block: "nearest" });
      }
    }
  }, [selectedIndex]);

  if (!open) return null;

  const wikiResults = filteredResults.filter((r) => r.group === "wiki");
  const workspaceResults = filteredResults.filter(
    (r) => r.group === "workspace"
  );

  return (
    <div
      className="fixed inset-0 z-50 flex items-start justify-center pt-[15vh] bg-black/30"
      onClick={close}
    >
      <div
        className="bg-white rounded-lg shadow-xl border border-slate-200 w-[520px] max-h-[400px] flex flex-col overflow-hidden"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="px-4 py-3">
          <input
            ref={inputRef}
            type="text"
            className="w-full border-none outline-none text-base text-slate-900 placeholder:text-slate-400"
            placeholder="搜索文件..."
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={handleKeyDown}
            aria-label="搜索文件"
          />
        </div>

        <div className="border-t border-slate-100" />

        <div ref={listRef} className="flex-1 overflow-y-auto py-1">
          {loading && (
            <div className="px-4 py-6 text-center text-sm text-slate-400">
              加载中...
            </div>
          )}

          {!loading && filteredResults.length === 0 && (
            <div className="px-4 py-6 text-center text-sm text-slate-400">
              无匹配结果
            </div>
          )}

          {wikiResults.length > 0 && (
            <>
              <div className="px-4 py-1.5 text-[11px] font-medium text-slate-400 uppercase tracking-wide">
                Wiki 页面
              </div>
              {wikiResults.map((r, i) => {
                const globalIndex = filteredResults.indexOf(r);
                return (
                  <button
                    key={r.id}
                    type="button"
                    className={`w-full text-left px-4 py-1.5 text-sm flex items-center gap-3 ${
                      globalIndex === selectedIndex
                        ? "bg-slate-100"
                        : "hover:bg-slate-50"
                    }`}
                    onClick={() => selectItem(r)}
                    onMouseEnter={() => setSelectedIndex(globalIndex)}
                  >
                    <span className="text-slate-500 shrink-0 w-4 text-center text-xs">
                      W
                    </span>
                    <span className="text-slate-900 truncate">
                      {r.title || r.name}
                    </span>
                    <span className="text-xs text-slate-400 truncate ml-auto">
                      {r.path}
                    </span>
                  </button>
                );
              })}
            </>
          )}

          {workspaceResults.length > 0 && (
            <>
              <div className="px-4 py-1.5 text-[11px] font-medium text-slate-400 uppercase tracking-wide">
                工作区文件
              </div>
              {workspaceResults.map((r, i) => {
                const globalIndex = filteredResults.indexOf(r);
                return (
                  <button
                    key={r.path}
                    type="button"
                    className={`w-full text-left px-4 py-1.5 text-sm flex items-center gap-3 ${
                      globalIndex === selectedIndex
                        ? "bg-slate-100"
                        : "hover:bg-slate-50"
                    }`}
                    onClick={() => selectItem(r)}
                    onMouseEnter={() => setSelectedIndex(globalIndex)}
                  >
                    <span className="text-slate-500 shrink-0 w-4 text-center text-xs">
                      F
                    </span>
                    <span className="text-slate-900 truncate">{r.name}</span>
                    <span className="text-xs text-slate-400 truncate ml-auto">
                      {r.path}
                    </span>
                  </button>
                );
              })}
            </>
          )}
        </div>
      </div>
    </div>
  );
}
