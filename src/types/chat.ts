export interface Conversation {
  id: string;
  kb_id: string;
  title: string;
  created_at: string;
  updated_at: string;
}

export interface ChatMessage {
  id: string;
  conversation_id: string;
  role: string;
  content: string;
  citations: string;
  created_at: string;
}
