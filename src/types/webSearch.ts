export interface WebSearchConfig {
  engine: string;
  max_results: number;
  searxng_url: string;
  brave_api_key: string;
  bing_api_key: string;
  bing_endpoint: string;
}

export interface SearchResult {
  title: string;
  url: string;
  snippet: string;
}

export interface WebPageContent {
  title: string;
  url: string;
  content: string;
  content_length: number;
}

export interface SaveWebResultResponse {
  source_id: string;
  task_id: string;
  file_name: string;
  file_type: string;
  file_size: number;
  created_at: string;
}
