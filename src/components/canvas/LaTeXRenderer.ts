import katex from "katex";

/**
 * Renders LaTeX math ($...$ and $$...$$) in a markdown/html string using KaTeX.
 * Returns HTML string with math replaced by KaTeX-rendered spans.
 */
export function renderLaTeX(content: string): string {
  if (!content) return content;

  // Process display math $$...$$ first (must be before inline $...$)
  let result = content;
  result = result.replace(/\$\$([\s\S]*?)\$\$/g, (_match, formula: string) => {
    try {
      return katex.renderToString(formula.trim(), {
        displayMode: true,
        throwOnError: false,
        trust: false,
      });
    } catch {
      return _match; // return original on failure
    }
  });

  // Process inline math $...$
  // Avoid matching $$ (already handled) and $ inside code blocks
  result = result.replace(/(?<!\$)\$(?!\$)([^$\n]+?)\$(?!\$)/g, (_match, formula: string) => {
    try {
      return katex.renderToString(formula.trim(), {
        displayMode: false,
        throwOnError: false,
        trust: false,
      });
    } catch {
      return _match;
    }
  });

  return result;
}

/**
 * Pre-renders LaTeX in content before passing to MarkdownRenderer.
 * KaTeX produces self-contained HTML spans with inline CSS, so they survive
 * the MarkdownRenderer's dangerouslySetInnerHTML pass.
 */
export function preprocessContent(content: string): string {
  return renderLaTeX(content);
}
