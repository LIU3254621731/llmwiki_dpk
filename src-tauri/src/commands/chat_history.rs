use std::sync::Arc;
use tauri::State;
use crate::core::app_kernel::AppKernel;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Conversation {
    pub id: String,
    pub kb_id: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    pub id: String,
    pub conversation_id: String,
    pub role: String,
    pub content: String,
    pub citations: String,
    pub created_at: String,
}

#[tauri::command]
pub async fn list_conversations(
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
) -> Result<Vec<Conversation>, String> {
    let conn = kernel.db.connect()?;
    let mut stmt = conn
        .prepare("SELECT id, kb_id, title, created_at, updated_at FROM chat_conversations WHERE kb_id = ?1 ORDER BY updated_at DESC")
        .map_err(|e| format!("查询对话列表失败: {}", e))?;

    let rows = stmt
        .query_map(rusqlite::params![kb_id], |row| {
            Ok(Conversation {
                id: row.get(0)?,
                kb_id: row.get(1)?,
                title: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
            })
        })
        .map_err(|e| format!("映射对话列表失败: {}", e))?;

    let mut conversations = Vec::new();
    for row in rows {
        conversations.push(row.map_err(|e| format!("读取对话行失败: {}", e))?);
    }
    Ok(conversations)
}

#[tauri::command]
pub async fn create_conversation(
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
    title: Option<String>,
) -> Result<Conversation, String> {
    let conn = kernel.db.connect()?;
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let title = title.unwrap_or_else(|| "新对话".to_string());

    conn.execute(
        "INSERT INTO chat_conversations (id, kb_id, title, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![id, kb_id, title, now, now],
    )
    .map_err(|e| format!("创建对话失败: {}", e))?;

    Ok(Conversation {
        id,
        kb_id,
        title,
        created_at: now.clone(),
        updated_at: now,
    })
}

#[tauri::command]
pub async fn get_conversation_messages(
    kernel: State<'_, Arc<AppKernel>>,
    conversation_id: String,
) -> Result<Vec<ChatMessage>, String> {
    let conn = kernel.db.connect()?;
    let mut stmt = conn
        .prepare("SELECT id, conversation_id, role, content, COALESCE(citations, ''), created_at FROM chat_messages WHERE conversation_id = ?1 ORDER BY created_at ASC")
        .map_err(|e| format!("查询消息失败: {}", e))?;

    let rows = stmt
        .query_map(rusqlite::params![conversation_id], |row| {
            Ok(ChatMessage {
                id: row.get(0)?,
                conversation_id: row.get(1)?,
                role: row.get(2)?,
                content: row.get(3)?,
                citations: row.get(4)?,
                created_at: row.get(5)?,
            })
        })
        .map_err(|e| format!("映射消息失败: {}", e))?;

    let mut messages = Vec::new();
    for row in rows {
        messages.push(row.map_err(|e| format!("读取消息行失败: {}", e))?);
    }
    Ok(messages)
}

#[tauri::command]
pub async fn save_message(
    kernel: State<'_, Arc<AppKernel>>,
    conversation_id: String,
    role: String,
    content: String,
    citations: Option<String>,
) -> Result<ChatMessage, String> {
    let conn = kernel.db.connect()?;
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let citations = citations.unwrap_or_default();

    conn.execute(
        "INSERT INTO chat_messages (id, conversation_id, role, content, citations, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![id, conversation_id, role, content, citations, now],
    )
    .map_err(|e| format!("保存消息失败: {}", e))?;

    conn.execute(
        "UPDATE chat_conversations SET updated_at = ?1 WHERE id = ?2",
        rusqlite::params![now, conversation_id],
    )
    .map_err(|e| format!("更新对话时间失败: {}", e))?;

    Ok(ChatMessage {
        id,
        conversation_id,
        role,
        content,
        citations,
        created_at: now,
    })
}

#[tauri::command]
pub async fn delete_conversation(
    kernel: State<'_, Arc<AppKernel>>,
    conversation_id: String,
) -> Result<(), String> {
    let conn = kernel.db.connect()?;
    conn.execute(
        "DELETE FROM chat_messages WHERE conversation_id = ?1",
        rusqlite::params![conversation_id],
    )
    .map_err(|e| format!("删除消息失败: {}", e))?;

    conn.execute(
        "DELETE FROM chat_conversations WHERE id = ?1",
        rusqlite::params![conversation_id],
    )
    .map_err(|e| format!("删除对话失败: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn update_conversation_title(
    kernel: State<'_, Arc<AppKernel>>,
    conversation_id: String,
    title: String,
) -> Result<(), String> {
    let conn = kernel.db.connect()?;
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE chat_conversations SET title = ?1, updated_at = ?2 WHERE id = ?3",
        rusqlite::params![title, now, conversation_id],
    )
    .map_err(|e| format!("更新对话标题失败: {}", e))?;
    Ok(())
}
