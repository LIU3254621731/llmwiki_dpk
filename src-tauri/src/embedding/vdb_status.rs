use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum VdbState {
    Idle,
    Indexing,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VdbStatus {
    pub kb_id: String,
    pub total_chunks: u64,
    pub disk_size_bytes: u64,
    pub vector_dimensions: u32,
    pub status: VdbState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReindexProgress {
    pub kb_id: String,
    pub current: u64,
    pub total: u64,
    pub message: String,
}
