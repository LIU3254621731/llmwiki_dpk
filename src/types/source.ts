export interface SourceItem {
  id: string;
  file_name: string;
  file_path: string;
  file_type: string;
  file_size: number;
  file_hash: string;
  status: string;
  created_at: string;
  updated_at: string;
  ai_summary?: string;
  coverage_report?: string;
  linked_pages_count?: number;
  review_count?: number;
  recent_task_id?: string;
}

export interface KnowledgeItem {
  id: string;
  canonical_name: string;
  item_type: string;
  page_path?: string;
  page_id?: string;
  linked_page_path?: string;
  summary?: string;
  source_id?: string;
}

export interface SourceDetail extends SourceItem {
  kb_id: string;
  extracted_text: string;
  preview_path?: string;
  preview_status?: string;
  preview_generated_at?: string;
  preview_error?: string;
  summary_json_path?: string;
  coverage_json_path?: string;
  linked_pages_count?: number;
  linked_relations_count?: number;
  review_count?: number;
  entity_count?: number;
  concept_count?: number;
  relation_count?: number;
  linked_wiki_pages?: LinkedWikiPage[];
  tasks?: LinkedTask[];
  graph_nodes?: GraphNodeRef[];
  knowledge_items?: KnowledgeItem[];
}

export interface LinkedWikiPage {
  id: string;
  title: string;
  path: string;
  page_type: string;
}

export interface LinkedTask {
  id: string;
  task_type: string;
  status: string;
  created_at: string;
  error_message: string;
}

export interface GraphNodeRef {
  id: string;
  label: string;
  node_type: string;
}

export interface FileType {
  extension: string;
  mime_type: string;
  description: string;
  is_document: boolean;
}

export interface FileTreeNode {
  name: string;
  relative_path: string;
  file_type: string;
  file_size: number;
  modified_at: string;
  is_directory: boolean;
  children?: FileTreeNode[];
  record_type: string;
  linked_record_id: string;
  status: string;
}

export interface FileScanResult {
  root: FileTreeNode;
  total_files: number;
  total_dirs: number;
  warnings: string[];
}

export interface FileDetail {
  file_name: string;
  relative_path: string;
  absolute_path: string;
  file_type: string;
  file_size: number;
  file_hash: string | null;
  created_at: string;
  modified_at: string;
  record_type: string;
  linked_record_id: string;
  status: string;
  linked_wiki_pages: LinkedWikiPage[];
  linked_tasks: LinkedTask[];
  linked_graph_nodes: GraphNodeRef[];
}

export interface SourcePreview {
  source_id: string;
  preview_path: string;
  preview_status: string;
  content: string;
  file_type: string;
}

export interface ImportCandidate {
  relative_path: string;
  file_name: string;
  file_type: string;
  file_size: number;
  is_supported: boolean;
}

export interface ImportFolderScanResult {
  folder_name: string;
  folder_path: string;
  total_files: number;
  supported_files: number;
  skipped_files: number;
  total_size: number;
  directory_count: number;
  files: ImportCandidate[];
  skipped_items: { relative_path: string; file_name: string; reason: string }[];
}

export interface ImportProgressEvent {
  current: number;
  total: number;
  file_name: string;
  relative_path: string;
  status: "importing" | "complete";
  success_count: number;
  fail_count: number;
}

export interface ImportFolderResult {
  total: number;
  success: number;
  failed: number;
  results: { file_path: string; relative_path?: string; status: string; result?: any; error?: string }[];
}

export interface HashCheckResult {
  file_hash: string;
  file_size: number;
  text_length: number;
  is_duplicate: boolean;
  existing_source: {
    id: string;
    file_name: string;
    file_type: string;
    status: string;
    created_at: string;
  } | null;
}
