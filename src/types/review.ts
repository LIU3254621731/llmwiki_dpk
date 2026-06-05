export interface ReviewItem {
  id: string;
  review_id?: string;
  task_id?: string;
  operation: string;
  operation_type: string;
  target_path: string;
  old_content: string;
  new_content: string;
  status: string;
  risk_level: string;
  reason: string;
  source_id: string;
  citation_status: string;
  summary: string;
  confidence: string;
  created_at: string;
  page_type: string;
  title: string;
  apply_error?: string;
  // v0.2.1: 额外标记
  duplicate_candidate?: boolean;
  merge_candidate?: boolean;
  missing_target?: boolean;
  manual_required?: boolean;
  auto_converted_from_update?: boolean;
  matched_page?: string;
  matched_path?: string;
}

export interface Review {
  id: string;
  kb_id: string;
  task_id: string;
  status: string;
  summary: string;
  risk_level: string;
  created_at: string;
  items: ReviewItem[];
  // Computed stats (spread by frontend computeStats)
  total_pending?: number;
  total_accepted?: number;
  total_rejected?: number;
  total_failed?: number;
  total_skipped?: number;
  total_manual?: number;
  total_path_errors?: number;
  createCount?: number;
  updateCount?: number;
  aliasCount?: number;
  relationCount?: number;
  mergeCount?: number;
  skipCount?: number;
  directlyApplicable?: number;
  needsManual?: number;
}
