export interface VdbStatus {
  kb_id: string;
  total_chunks: number;
  disk_size_bytes: number;
  vector_dimensions: number;
  status: "idle" | "indexing" | "error";
  error_message: string | null;
}

export interface EmbeddingConfig {
  engine_type: "builtin" | "high_perf" | "custom";
  model_path: string | null;
  num_threads: number;
  graph_opt_level: string;
  max_seq_len: number;
  pooling_strategy: string;
  l2_normalize: boolean;
}

export interface ReindexProgress {
  kb_id: string;
  current: number;
  total: number;
  message: string;
}

export const GRAPH_OPT_LEVELS: { value: string; label: string; desc: string }[] = [
  { value: "level1", label: "Level 1", desc: "基础图优化，加载最快" },
  { value: "level2", label: "Level 2", desc: "扩展图优化，平衡速度与内存" },
  { value: "level3", label: "Level 3", desc: "全部优化，推理最快" },
];

export const MAX_SEQ_LEN_OPTIONS: { value: number; label: string }[] = [
  { value: 128, label: "128 tokens" },
  { value: 256, label: "256 tokens" },
  { value: 512, label: "512 tokens (推荐)" },
];

export const POOLING_STRATEGIES: { value: string; label: string; desc: string }[] = [
  { value: "mean", label: "均值池化 (Mean)", desc: "对所有 token 加权平均，BGE 模型推荐" },
  { value: "cls", label: "CLS 向量", desc: "取 [CLS] 标记向量，部分模型适用" },
];
