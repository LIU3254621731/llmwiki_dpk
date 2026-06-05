import { useState, useCallback, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useKBStore } from "@/stores/useKBStore";

interface WikiPageLink {
  title: string;
  path: string;
}

export function useWikiLinkAutocomplete() {
  const currentKB = useKBStore((s) => s.currentKB);
  const [showAutocomplete, setShowAutocomplete] = useState(false);
  const [autocompleteResults, setAutocompleteResults] = useState<
    WikiPageLink[]
  >([]);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [autocompletePosition, setAutocompletePosition] = useState({
    top: 0,
    left: 0,
  });
  const [allPages, setAllPages] = useState<WikiPageLink[]>([]);
  const [pagesLoaded, setPagesLoaded] = useState(false);

  const loadPages = useCallback(async (): Promise<WikiPageLink[]> => {
    if (!currentKB) return [];
    try {
      const pagesRaw = await invoke<any[]>("list_wiki_pages", {
        kbId: currentKB.id,
      });
      const pages: WikiPageLink[] = pagesRaw.map((p: any) => ({
        title: p.title || p.path,
        path: p.path,
      }));
      setAllPages(pages);
      setPagesLoaded(true);
      return pages;
    } catch (e) {
      console.error("WikiLinkAutocomplete: 加载页面列表失败", e);
      return [];
    }
  }, [currentKB]);

  const handleInput = useCallback(
    (
      textareaValue: string,
      cursorPos: number,
      textareaBounds: DOMRect
    ) => {
      const textBeforeCursor = textareaValue.slice(0, cursorPos);

      // Find the most recent [[ that hasn't been closed
      const lastOpen = textBeforeCursor.lastIndexOf("[[");
      if (lastOpen === -1) {
        setShowAutocomplete(false);
        setSelectedIndex(0);
        return;
      }

      // Check if [[ is closed before cursor
      const afterOpen = textareaValue.slice(lastOpen + 2, cursorPos);
      const closeBeforeCursor = afterOpen.indexOf("]]");
      if (closeBeforeCursor !== -1) {
        setShowAutocomplete(false);
        setSelectedIndex(0);
        return;
      }

      // Don't show if [[ is followed by newline before cursor
      if (afterOpen.includes("\n")) {
        setShowAutocomplete(false);
        setSelectedIndex(0);
        return;
      }

      const partial = afterOpen;

      const doFilter = (pages: WikiPageLink[]) => {
        const filtered = pages.filter(
          (p) =>
            p.title.toLowerCase().includes(partial.toLowerCase()) ||
            p.path.toLowerCase().includes(partial.toLowerCase())
        );
        setAutocompleteResults(filtered);
        setSelectedIndex(0);
        setShowAutocomplete(true);
      };

      if (pagesLoaded) {
        doFilter(allPages);
      } else {
        loadPages().then((pages) => doFilter(pages));
      }

      // Position the dropdown at cursor
      // Approximate: ~8px per char, ~20px per line
      const linesBefore = textBeforeCursor.split("\n");
      const currentLineChars = linesBefore[linesBefore.length - 1].length;
      setAutocompletePosition({
        top: textareaBounds.top + 24 + (linesBefore.length - 1) * 20,
        left: textareaBounds.left + currentLineChars * 8,
      });
    },
    [allPages, loadPages]
  );

  // The actual insertion is done by the editor; hook provides link info
  const insertLink = useCallback(
    (link: WikiPageLink): { newValue: string; newCursorPos: number } | null => {
    return null;
  }, []);

  const navigateAutocomplete = useCallback(
    (direction: "up" | "down") => {
      if (direction === "down") {
        setSelectedIndex((prev) =>
          Math.min(prev + 1, autocompleteResults.length - 1)
        );
      } else {
        setSelectedIndex((prev) => Math.max(prev - 1, 0));
      }
    },
    [autocompleteResults.length]
  );

  const selectAutocompleteItem = useCallback(
    (index: number): WikiPageLink | null => {
      if (autocompleteResults[index]) {
        setShowAutocomplete(false);
        setSelectedIndex(0);
        return autocompleteResults[index];
      }
      return null;
    },
    [autocompleteResults]
  );

  const closeAutocomplete = useCallback(() => {
    setShowAutocomplete(false);
    setSelectedIndex(0);
  }, []);

  return {
    showAutocomplete,
    autocompleteResults,
    selectedIndex,
    autocompletePosition,
    handleInput,
    insertLink,
    navigateAutocomplete,
    selectAutocompleteItem,
    closeAutocomplete,
  };
}

// Autocomplete dropdown component
export function WikiLinkAutocompleteDropdown({
  results,
  selectedIndex,
  position,
  onSelect,
  onHover,
}: {
  results: WikiPageLink[];
  selectedIndex: number;
  position: { top: number; left: number };
  onSelect: (index: number) => void;
  onHover: (index: number) => void;
}) {
  if (results.length === 0) return null;

  return (
    <div
      className="fixed z-50 bg-white border border-slate-200 rounded shadow-lg max-h-40 overflow-y-auto w-64"
      style={{ top: position.top, left: position.left }}
    >
      {results.map((page, i) => (
        <button
          key={page.path}
          type="button"
          className={`w-full text-left px-3 py-1.5 text-sm ${
            i === selectedIndex ? "bg-slate-100" : "hover:bg-slate-50"
          }`}
          onClick={() => onSelect(i)}
          onMouseEnter={() => onHover(i)}
        >
          <span className="text-slate-900">{page.title}</span>
          <span className="text-xs text-slate-400 ml-2">{page.path}</span>
        </button>
      ))}
    </div>
  );
}
