export interface WikiPage {
  id: string;
  title: string;
  path: string;
  page_type: string;
  canonical_name: string;
  tags: string;
  created_at: string;
  updated_at: string;
}

export interface PageVersion {
  id: string;
  kb_id: string;
  page_path: string;
  content_hash: string;
  snapshot_path: string;
  task_id: string;
  operation_id: string;
  created_at: string;
}
