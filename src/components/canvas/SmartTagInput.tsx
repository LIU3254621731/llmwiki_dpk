import { useState, useRef, useEffect, useCallback, KeyboardEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { X, Hash, Globe } from "lucide-react";
import { useKBStore } from "@/stores/useKBStore";
import { useCanvasStore } from "@/stores/useCanvasStore";
import WebSearchModal from "@/components/canvas/WebSearchModal";
import type { WebSourceItem } from "@/types/canvas";

export default function SmartTagInput() {
  const currentKB = useKBStore((s) => s.currentKB);
  const tags = useCanvasStore((s) => s.tags);
  const addTag = useCanvasStore((s) => s.addTag);
  const removeTag = useCanvasStore((s) => s.removeTag);
  const generationLock = useCanvasStore((s) => s.generationLock);

  const [inputValue, setInputValue] = useState("");
  const [suggestions, setSuggestions] = useState<string[]>([]);
  const [dropdownOpen, setDropdownOpen] = useState(false);
  const [highlightedIdx, setHighlightedIdx] = useState(0);
  const [showHashHint, setShowHashHint] = useState(false);
  const [showWebSearch, setShowWebSearch] = useState(false);
  const [webSearchQuery, setWebSearchQuery] = useState("");
  const generateFromWeb = useCanvasStore((s) => s.generateFromWeb);

  const inputRef = useRef<HTMLInputElement>(null);
  const dropdownRef = useRef<HTMLDivElement>(null);
  const debounceRef = useRef<number | null>(null);

  const fetchSuggestions = useCallback(
    async (query: string) => {
      if (!currentKB || !query) {
        setSuggestions([]);
        setDropdownOpen(false);
        return;
      }
      try {
        const results = await invoke<string[]>("get_canvas_tag_suggestions", {
          kbId: currentKB.id,
          query,
        });
        const filtered = results.filter((t) => !tags.includes(t));
        setSuggestions(filtered);
        // Always show dropdown: either local tags or web search option
        setDropdownOpen(true);
        setHighlightedIdx(0);
      } catch {
        setSuggestions([]);
        setDropdownOpen(true); // still show web search option
      }
    },
    [currentKB, tags],
  );

  const openWebSearch = () => {
    const hashIdx = inputValue.lastIndexOf("#");
    const query = hashIdx >= 0 ? inputValue.slice(hashIdx + 1).trim() : inputValue.trim();
    setWebSearchQuery(query);
    setShowWebSearch(true);
    setDropdownOpen(false);
  };

  const handleWebGenerate = (sources: WebSourceItem[]) => {
    setShowWebSearch(false);
    if (currentKB) {
      generateFromWeb(currentKB.id, sources);
    }
  };

  const handleInputChange = (value: string) => {
    setInputValue(value);

    // Detect # trigger
    if (value.includes("#")) {
      const hashIdx = value.lastIndexOf("#");
      const query = value.slice(hashIdx + 1).trim();
      setShowHashHint(false);

      if (debounceRef.current) clearTimeout(debounceRef.current);
      debounceRef.current = window.setTimeout(() => fetchSuggestions(query), 200);
    } else if (value.length > 0) {
      setShowHashHint(true);
      setDropdownOpen(false);
    } else {
      setShowHashHint(false);
      setDropdownOpen(false);
    }
  };

  const selectTag = (tag: string) => {
    addTag(tag);
    setInputValue("");
    setDropdownOpen(false);
    setSuggestions([]);
    inputRef.current?.focus();
  };

  const handleKeyDown = (e: KeyboardEvent<HTMLInputElement>) => {
    if (!dropdownOpen) return;

    switch (e.key) {
      case "ArrowDown":
        e.preventDefault();
        setHighlightedIdx((prev) => Math.min(prev + 1, suggestions.length - 1));
        break;
      case "ArrowUp":
        e.preventDefault();
        setHighlightedIdx((prev) => Math.max(prev - 1, 0));
        break;
      case "Enter":
        e.preventDefault();
        if (suggestions[highlightedIdx]) {
          selectTag(suggestions[highlightedIdx]);
        }
        break;
      case "Escape":
        setDropdownOpen(false);
        break;
    }
  };

  // Close dropdown on outside click
  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (
        dropdownRef.current &&
        !dropdownRef.current.contains(e.target as Node) &&
        inputRef.current &&
        !inputRef.current.contains(e.target as Node)
      ) {
        setDropdownOpen(false);
      }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, []);

  return (
    <div className="relative flex-1 min-w-0">
      <div className="flex items-center flex-wrap gap-1.5 px-3 py-2 bg-muted/50 rounded-lg border border-border min-h-[40px] focus-within:border-primary/50 focus-within:bg-background transition-colors">
        <Hash size={14} className="text-muted-foreground shrink-0" />
        {tags.map((tag) => (
          <span
            key={tag}
            className="inline-flex items-center gap-1 px-2 py-0.5 bg-primary/10 text-primary text-xs rounded-full font-medium"
          >
            {tag}
            <button
              type="button"
              onClick={() => removeTag(tag)}
              className="hover:text-destructive transition-colors"
              disabled={generationLock}
            >
              <X size={12} />
            </button>
          </span>
        ))}
        <input
          ref={inputRef}
          type="text"
          value={inputValue}
          onChange={(e) => handleInputChange(e.target.value)}
          onKeyDown={handleKeyDown}
          onFocus={() => {
            if (inputValue.includes("#")) {
              const hashIdx = inputValue.lastIndexOf("#");
              const query = inputValue.slice(hashIdx + 1).trim();
              if (query) fetchSuggestions(query);
            }
          }}
          placeholder={tags.length === 0 ? "输入 # 选择知识标签..." : "添加更多标签..."}
          className="flex-1 min-w-[120px] bg-transparent border-none outline-none text-sm text-foreground placeholder:text-muted-foreground"
          readOnly={generationLock}
        />
      </div>

      {/* Hash hint */}
      {showHashHint && (
        <div className="absolute top-full mt-1 text-xs text-muted-foreground">
          输入 <code className="px-1 py-0.5 bg-muted rounded">#</code>{" "}
          后跟关键字以搜索知识标签
        </div>
      )}

      {/* Dropdown */}
      {dropdownOpen && (
        <div
          ref={dropdownRef}
          className="absolute top-full mt-1 left-0 right-0 bg-popover border border-border rounded-lg shadow-lg z-50 max-h-48 overflow-y-auto"
        >
          {suggestions.map((tag, idx) => (
            <button
              key={tag}
              type="button"
              onClick={() => selectTag(tag)}
              className={`flex items-center gap-2 w-full px-3 py-2 text-sm text-left transition-colors ${
                idx === highlightedIdx
                  ? "bg-primary/10 text-primary"
                  : "text-foreground hover:bg-muted"
              }`}
            >
              <Hash size={12} className="text-muted-foreground shrink-0" />
              <span>{tag}</span>
            </button>
          ))}
          {/* Web search fallback — always available */}
          {suggestions.length === 0 && inputValue.includes("#") && (
            <button
              type="button"
              onClick={openWebSearch}
              className="flex items-center gap-2 w-full px-3 py-2 text-sm text-left text-brand-600 hover:bg-brand-50 dark:hover:bg-brand-900/20 transition-colors"
            >
              <Globe size={12} className="shrink-0" />
              <span>搜索网络: {inputValue.slice(inputValue.lastIndexOf("#") + 1).trim()}</span>
            </button>
          )}
          {/* If there are local results, still offer web search at the bottom */}
          {suggestions.length > 0 && inputValue.includes("#") && (
            <button
              type="button"
              onClick={openWebSearch}
              className="flex items-center gap-2 w-full px-3 py-2 text-sm text-left text-slate-500 hover:bg-brand-50 dark:hover:bg-brand-900/20 hover:text-brand-600 border-t border-border transition-colors"
            >
              <Globe size={12} className="shrink-0" />
              <span>搜索网络获取更多资料...</span>
            </button>
          )}
        </div>
      )}

      {/* Web Search Modal */}
      {showWebSearch && (
        <WebSearchModal
          initialQuery={webSearchQuery}
          onClose={() => setShowWebSearch(false)}
          onGenerate={handleWebGenerate}
        />
      )}
    </div>
  );
}
