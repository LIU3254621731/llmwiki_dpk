export interface TaskItem {
  id: string;
  kb_id?: string;
  task_type: string;
  task_name?: string;
  status: string;
  current_agent: string;
  error_message: string;
  failure_reason?: string;
  recoverable?: boolean;
  resume_from_stage?: string;
  last_success_stage?: string;
  next_action?: string;
  retry_count: number;
  cancel_reason?: string;
  created_at: string;
  updated_at: string;
  completed_at?: string;
  archived_at?: string;
  handled_at?: string;
  locked_at?: string;
  review_id?: string;
}

export interface TaskEvent {
  id: string;
  task_id: string;
  event_type: string;
  agent_name: string;
  message: string;
  created_at: string;
}

export interface TaskDetail extends TaskItem {
  input_ref: string;
  output_ref: string;
  events?: TaskEvent[];
  source_meta?: SourceMeta;
  review_id?: string;
}

export interface SourceMeta {
  file_name: string;
  file_type: string;
  file_size: number;
  file_hash: string;
  text_length: number;
  page_count: number | null;
}

export interface AgentStatusChangePayload {
  task_id: string;
  stage: string;
  stage_status: string;
  progress: number;
  prompt_text: string;
  response_text: string;
  log_message: string;
  timestamp: string;
}
