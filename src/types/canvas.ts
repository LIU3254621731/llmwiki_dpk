// Canvas (画布) — AI knowledge recombination workspace types

export interface OutlineNode {
  id: string;
  title: string;
  level: number;
  children: OutlineNode[];
}

export interface CanvasScope {
  id: string;
  kb_id: string;
  name: string;
  tags: string[];
  last_scroll_position: number;
  created_at: string;
  updated_at: string;
}

export interface CodeBlock {
  language: string;
  code: string;
  caption?: string;
}

export interface DetailData {
  topic: string;
  definition: string;
  mechanism: string;
  formulas: string[];
  code_blocks: CodeBlock[];
}

export type GenerationPhase = "idle" | "outline" | "textbook" | "done";

export interface ScopeCheckResult {
  total_words: number;
  matched_file_count: number;
  cache_key: string;
  blocked: boolean;
  message?: string;
}

export interface OutlineGenerationResult {
  nodes: OutlineNode[];
  cache_key: string;
}

export interface CanvasStreamChunk {
  chunk: string;
  accumulated: string;
}

export interface CanvasStreamDone {
  full_text: string;
}

export interface CanvasStreamError {
  error: string;
}

export interface WebSourceItem {
  title: string;
  url: string;
  content: string;
  selected: boolean;
}
