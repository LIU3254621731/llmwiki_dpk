export interface TokenStats {
  total_input_tokens: number;
  total_output_tokens: number;
  total_cost_yuan: number;
  call_count: number;
}

export interface DailyTokenUsage {
  date: string;
  input_tokens: number;
  output_tokens: number;
}

export interface TokenLogEntry {
  id: string;
  task_id: string;
  task_name: string;
  agent_name: string;
  input_tokens: number;
  output_tokens: number;
  model_name: string;
  provider: string;
  created_at: string;
}

export interface PaginatedTokenLogs {
  entries: TokenLogEntry[];
  total: number;
  page: number;
  page_size: number;
}

export interface DailyTokenLimit {
  enabled: boolean;
  limit: number;
}

export interface TokenQuotaStatus {
  allowed: boolean;
  today_used: number;
  limit: number;
  remaining: number;
  message: string;
}
