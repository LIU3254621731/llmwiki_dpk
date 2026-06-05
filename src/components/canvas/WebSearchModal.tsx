import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useKBStore } from "@/stores/useKBStore";
import { useCanvasStore } from "@/stores/useCanvasStore";
import {
  X, Search, Loader2, CheckSquare, Square,
  ExternalLink, Globe, Download,
} from "lucide-react";
import { cn } from "@/lib/utils";

interface SearchResult {
  title: string;
  url: string;
  snippet: string;
  engine: string;
}

interface WebSourceItem {
  title: string;
  url: string;
  content: string;
  selected: boolean;
}

interface Props {
  initialQuery: string;
  onClose: () => void;
  onGenerate: (sources: WebSourceItem[]) => void;
}

export default function WebSearchModal({ initialQuery, onClose, onGenerate }: Props) {
  const currentKB = useKBStore((s) => s.currentKB);
  const generationLock = useCanvasStore((s) => s.generationLock);

  const [query, setQuery] = useState(initialQuery);
  const [searchResults, setSearchResults] = useState<SearchResult[]>([]);
  const [selectedUrls, setSelectedUrls] = useState<Set<string>>(new Set());
  const [searching, setSearching] = useState(false);
  const [searchError, setSearchError] = useState("");
  const [fetchingContent, setFetchingContent] = useState(false);
  const [phase, setPhase] = useState<"search" | "select" | "fetching">("search");

  const handleSearch = useCallback(async () => {
    if (!query.trim()) return;
    setSearching(true);
    setSearchError("");
    setPhase("search");
    try {
      const results = await invoke<SearchResult[]>("web_search", {
        query: query.trim(),
        maxResults: 10,
      });
      setSearchResults(results);
      if (results.length === 0) {
        setSearchError("未找到相关搜索结果");
      }
    } catch (e) {
      setSearchError(`搜索失败: ${e}`);
      setSearchResults([]);
    } finally {
      setSearching(false);
    }
  }, [query]);

  const toggleSelect = (url: string) => {
    setSelectedUrls((prev) => {
      const next = new Set(prev);
      if (next.has(url)) {
        next.delete(url);
      } else {
        next.add(url);
      }
      return next;
    });
  };

  const selectAll = () => {
    if (selectedUrls.size === searchResults.length) {
      setSelectedUrls(new Set());
    } else {
      setSelectedUrls(new Set(searchResults.map((r) => r.url)));
    }
  };

  const handleGenerate = useCallback(async () => {
    if (selectedUrls.size === 0) return;
    setFetchingContent(true);
    setPhase("fetching");
    try {
      const sources: WebSourceItem[] = [];
      for (const r of searchResults) {
        if (!selectedUrls.has(r.url)) continue;
        try {
          const content = await invoke<string>("fetch_web_page_content", { url: r.url });
          sources.push({ title: r.title, url: r.url, content, selected: true });
        } catch {
          // Use snippet as fallback if fetch fails
          sources.push({ title: r.title, url: r.url, content: r.snippet, selected: true });
        }
      }
      onGenerate(sources);
    } catch (e) {
      setSearchError(`获取网页内容失败: ${e}`);
      setPhase("select");
    } finally {
      setFetchingContent(false);
    }
  }, [selectedUrls, searchResults, onGenerate]);

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 backdrop-blur-sm"
         onClick={(e) => { if (e.target === e.currentTarget) onClose(); }}>
      <div className="bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-700 rounded-xl shadow-2xl w-[680px] max-h-[85vh] flex flex-col overflow-hidden"
           onClick={(e) => e.stopPropagation()}>
        {/* Header */}
        <div className="flex items-center justify-between px-5 py-3 border-b border-slate-200 dark:border-slate-700 shrink-0">
          <div className="flex items-center gap-2">
            <Globe size={16} className="text-slate-500" />
            <span className="text-sm font-medium text-slate-700 dark:text-slate-300">网络搜索参考资料</span>
          </div>
          <button onClick={onClose} className="p-1 text-slate-400 hover:text-slate-600 rounded" disabled={generationLock}>
            <X size={16} />
          </button>
        </div>

        {/* Search bar */}
        <div className="flex items-center gap-2 px-5 py-3 border-b border-slate-100 dark:border-slate-800 shrink-0">
          <div className="flex-1 flex items-center gap-2 px-3 py-2 bg-slate-50 dark:bg-slate-800 border border-slate-200 dark:border-slate-700 rounded-lg">
            <Search size={14} className="text-slate-400 shrink-0" />
            <input
              type="text"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              onKeyDown={(e) => { if (e.key === "Enter") handleSearch(); }}
              placeholder="输入搜索关键词..."
              className="flex-1 bg-transparent border-none outline-none text-sm text-slate-700 dark:text-slate-300 placeholder:text-slate-400"
              autoFocus
            />
          </div>
          <button
            onClick={handleSearch}
            disabled={searching || !query.trim()}
            className="px-4 py-2 bg-brand-500 text-white text-sm font-medium rounded-lg hover:bg-brand-600 disabled:opacity-50 shrink-0"
          >
            {searching ? <Loader2 size={14} className="animate-spin" /> : "搜索"}
          </button>
        </div>

        {/* Error */}
        {searchError && (
          <div className="px-5 py-2 text-xs text-red-600 bg-red-50 dark:bg-red-900/20 border-b border-red-100 dark:border-red-900/30 shrink-0">
            {searchError}
          </div>
        )}

        {/* Results */}
        <div className="flex-1 overflow-y-auto">
          {searching || fetchingContent ? (
            <div className="flex items-center justify-center py-20">
              <div className="text-center">
                <Loader2 size={24} className="animate-spin mx-auto mb-2 text-slate-400" />
                <p className="text-sm text-slate-500">
                  {fetchingContent ? "正在获取选中网页内容..." : "搜索中..."}
                </p>
              </div>
            </div>
          ) : phase === "fetching" ? (
            <div className="flex items-center justify-center py-20">
              <div className="text-center">
                <Loader2 size={24} className="animate-spin mx-auto mb-2 text-slate-400" />
                <p className="text-sm text-slate-500">
                  {fetchingContent ? "正在获取选中网页内容..." : "搜索中..."}
                </p>
              </div>
            </div>
          ) : searchResults.length === 0 ? (
            <div className="flex flex-col items-center justify-center py-20">
              <Search size={48} className="text-slate-300 mb-4" />
              <p className="text-sm text-slate-500">输入关键词搜索网络参考资料</p>
              <p className="text-xs text-slate-400 mt-1">支持 DuckDuckGo 等搜索引擎</p>
            </div>
          ) : (
            <div>
              {/* Select all */}
              <div className="flex items-center gap-2 px-5 py-2 border-b border-slate-100 dark:border-slate-800 shrink-0">
                <button
                  onClick={selectAll}
                  className="flex items-center gap-1.5 text-xs text-slate-500 hover:text-slate-700"
                >
                  {selectedUrls.size === searchResults.length && searchResults.length > 0
                    ? <CheckSquare size={14} className="text-brand-500" />
                    : <Square size={14} />}
                  {selectedUrls.size === searchResults.length ? "取消全选" : "全选"}
                </button>
                <span className="text-xs text-slate-400 ml-auto">
                  已选 {selectedUrls.size}/{searchResults.length} 项
                </span>
              </div>
              {/* Result list */}
              {searchResults.map((r) => (
                <div
                  key={r.url}
                  className={cn(
                    "flex gap-3 px-5 py-3 border-b border-slate-50 dark:border-slate-800/50 hover:bg-slate-50 dark:hover:bg-slate-800/30 cursor-pointer transition-colors",
                    selectedUrls.has(r.url) && "bg-brand-50/50 dark:bg-brand-900/10",
                  )}
                  onClick={() => toggleSelect(r.url)}
                >
                  <button className="shrink-0 mt-0.5">
                    {selectedUrls.has(r.url)
                      ? <CheckSquare size={16} className="text-brand-500" />
                      : <Square size={16} className="text-slate-300" />}
                  </button>
                  <div className="flex-1 min-w-0">
                    <div className="flex items-start gap-2">
                      <span className="text-sm font-medium text-slate-700 dark:text-slate-300 truncate">
                        {r.title}
                      </span>
                      <a
                        href={r.url}
                        target="_blank"
                        rel="noopener noreferrer"
                        onClick={(e) => e.stopPropagation()}
                        className="shrink-0 text-slate-400 hover:text-brand-500"
                      >
                        <ExternalLink size={13} />
                      </a>
                    </div>
                    <p className="text-xs text-slate-500 dark:text-slate-400 mt-0.5 line-clamp-2">
                      {r.snippet}
                    </p>
                    <p className="text-[10px] text-slate-400 truncate mt-1">{r.url}</p>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>

        {/* Footer */}
        {searchResults.length > 0 && (
          <div className="flex items-center justify-between px-5 py-3 border-t border-slate-200 dark:border-slate-700 shrink-0">
            <span className="text-xs text-slate-500">
              已选择 <span className="font-semibold text-brand-600">{selectedUrls.size}</span> 个网页作为参考资料
            </span>
            <div className="flex items-center gap-2">
              <button
                onClick={onClose}
                className="px-3 py-1.5 text-xs border border-slate-200 dark:border-slate-700 text-slate-600 dark:text-slate-400 rounded-md hover:bg-slate-50 dark:hover:bg-slate-800"
              >
                取消
              </button>
              <button
                onClick={handleGenerate}
                disabled={selectedUrls.size === 0 || fetchingContent || generationLock}
                className="flex items-center gap-1.5 px-4 py-1.5 bg-primary text-primary-foreground text-xs font-medium rounded-md hover:bg-primary/90 disabled:opacity-50"
              >
                {fetchingContent ? <Loader2 size={12} className="animate-spin" /> : <Download size={12} />}
                生成教材
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
