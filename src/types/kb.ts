export interface KnowledgeBase {
  id: string;
  name: string;
  path: string;
  created_at: string;
  updated_at: string;
}

export interface KBStats {
  page_count: number;
  source_count: number;
  review_count: number;
  relationship_count: number;
  broken_page_count: number;
  failed_task_count: number;
  knowledge_item_count?: number;
  graph_node_count?: number;
  severe_issue_count?: number;
  warning_issue_count?: number;
  issue_count?: number;
  health_status: string;
  language?: string;
  review_mode?: string;
}
