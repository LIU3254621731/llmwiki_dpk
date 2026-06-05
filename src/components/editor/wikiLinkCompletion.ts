import type { CompletionContext, CompletionResult } from "@codemirror/autocomplete";
import { invoke } from "@tauri-apps/api/core";

let _kbId = "";

export function setCompletionKbId(kbId: string) {
  _kbId = kbId;
}

export async function wikiLinkCompletionSource(
  context: CompletionContext
): Promise<CompletionResult | null> {
  const match = context.matchBefore(/\[\[([^\]\n]*)$/);
  if (!match || !_kbId) return null;

  const partial = match.text.startsWith("[[") ? match.text.slice(2) : "";

  try {
    const pagesRaw = await invoke<any[]>("list_wiki_pages", { kbId: _kbId });
    const pages = pagesRaw.map((p: any) => ({
      label: p.title || p.path,
      detail: p.path,
      apply: p.title || p.path,
    }));

    const filtered = pages.filter(
      (p) =>
        p.label.toLowerCase().includes(partial.toLowerCase()) ||
        p.detail.toLowerCase().includes(partial.toLowerCase())
    );

    if (filtered.length === 0) return null;

    return {
      from: context.pos - partial.length,
      options: filtered.slice(0, 20).map((p) => ({
        label: p.label,
        detail: p.detail,
        apply: (view, completion, from, to) => {
          view.dispatch({
            changes: {
              from,
              to,
              insert: `[[${p.label}]]`,
            },
            selection: { anchor: from + p.label.length + 4 },
          });
        },
      })),
    };
  } catch {
    return null;
  }
}
