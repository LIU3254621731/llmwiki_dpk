use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use crate::core::database_service::DatabaseService;
use crate::core::event_bus::EventBus;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDefinition {
    pub id: String,
    pub name: String,
    pub role: String,
    pub trigger_event: String,
    pub system_prompt: String,
    pub allowed_skills: Vec<String>,
    pub status: String,
    pub max_depth: u32,
    pub timeout_secs: u32,
    #[serde(default)]
    pub metadata_json: serde_json::Value,
    pub created_at: String,
    pub updated_at: String,
}

pub struct AgentRegistry {
    db: Arc<DatabaseService>,
    event_bus: Arc<EventBus>,
    cache: RwLock<HashMap<String, AgentDefinition>>,
}

impl AgentRegistry {
    pub fn new(db: Arc<DatabaseService>, event_bus: Arc<EventBus>) -> Self {
        let registry = Self {
            db,
            event_bus,
            cache: RwLock::new(HashMap::new()),
        };
        registry.reload_cache();
        registry
    }

    pub fn reload_cache(&self) {
        if let Ok(conn) = self.db.connect() {
            match conn.prepare("SELECT id, name, role, trigger_event, system_prompt, allowed_skills, status, max_depth, timeout_secs, metadata_json, created_at, updated_at FROM agent_definitions") {
                Ok(mut stmt) => {
                    let rows = stmt.query_map([], |row| {
                        let allowed_skills_str: String = row.get(4)?;
                        let allowed_skills: Vec<String> =
                            serde_json::from_str(&allowed_skills_str).unwrap_or_default();
                        let metadata_str: String = row.get(9)?;
                        let metadata_json: serde_json::Value =
                            serde_json::from_str(&metadata_str).unwrap_or_default();
                        Ok(AgentDefinition {
                            id: row.get(0)?,
                            name: row.get(1)?,
                            role: row.get(2)?,
                            trigger_event: row.get(3)?,
                            system_prompt: row.get(4)?,
                            allowed_skills,
                            status: row.get(6)?,
                            max_depth: row.get(7)?,
                            timeout_secs: row.get(8)?,
                            metadata_json,
                            created_at: row.get(10)?,
                            updated_at: row.get(11)?,
                        })
                    });
                    if let Ok(iter) = rows {
                        let mut cache = self.cache.write();
                        cache.clear();
                        for item in iter.flatten() {
                            cache.insert(item.name.clone(), item);
                        }
                    }
                }
                Err(e) => log::error!("[AgentRegistry] reload_cache 查询失败: {}", e),
            }
        }
    }

    pub fn list_agents(&self) -> Vec<AgentDefinition> {
        self.cache.read().values().cloned().collect()
    }

    pub fn get_agent(&self, name: &str) -> Option<AgentDefinition> {
        self.cache.read().get(name).cloned()
    }

    pub fn find_by_trigger(&self, event_type: &str) -> Vec<AgentDefinition> {
        self.cache
            .read()
            .values()
            .filter(|a| a.trigger_event == event_type && a.status == "active")
            .cloned()
            .collect()
    }

    pub fn create_agent(&self, def: AgentDefinition) -> Result<AgentDefinition, String> {
        let conn = self.db.connect().map_err(|e| format!("连接数据库失败: {}", e))?;

        let allowed_skills_json = serde_json::to_string(&def.allowed_skills).unwrap_or_else(|_| "[]".to_string());
        let metadata_str = def.metadata_json.to_string();

        conn.execute(
            "INSERT INTO agent_definitions (id, name, role, trigger_event, system_prompt, allowed_skills, status, max_depth, timeout_secs, metadata_json, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            rusqlite::params![
                def.id,
                def.name,
                def.role,
                def.trigger_event,
                def.system_prompt,
                allowed_skills_json,
                def.status,
                def.max_depth,
                def.timeout_secs,
                metadata_str,
                def.created_at,
                def.updated_at,
            ],
        )
        .map_err(|e| format!("创建 Agent 失败: {}", e))?;

        self.reload_cache();
        self.event_bus.emit_agent_definition_changed("created", &def.name);
        Ok(self.get_agent(&def.name).unwrap_or(def))
    }

    pub fn update_agent(&self, id: &str, patch: &serde_json::Value) -> Result<AgentDefinition, String> {
        let conn = self.db.connect().map_err(|e| format!("连接数据库失败: {}", e))?;

        // 读取现有记录
        let mut stmt = conn
            .prepare("SELECT name, role, trigger_event, system_prompt, allowed_skills, status, max_depth, timeout_secs, metadata_json FROM agent_definitions WHERE id = ?1")
            .map_err(|e| format!("查询 Agent 失败: {}", e))?;

        let (name, mut role, mut trigger, mut prompt, mut skills, mut status, mut max_depth, mut timeout_secs, mut metadata) =
            stmt.query_row(rusqlite::params![id], |row| {
                let skills_str: String = row.get(4)?;
                let s: Vec<String> = serde_json::from_str(&skills_str).unwrap_or_default();
                let meta_str: String = row.get(8)?;
                let m: serde_json::Value = serde_json::from_str(&meta_str).unwrap_or_default();
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    s,
                    row.get::<_, String>(5)?,
                    row.get::<_, u32>(6)?,
                    row.get::<_, u32>(7)?,
                    m,
                ))
            })
            .map_err(|e| format!("Agent 不存在: {}", e))?;

        if let Some(v) = patch.get("role").and_then(|v| v.as_str()) {
            role = v.to_string();
        }
        if let Some(v) = patch.get("trigger_event").and_then(|v| v.as_str()) {
            trigger = v.to_string();
        }
        if let Some(v) = patch.get("system_prompt").and_then(|v| v.as_str()) {
            prompt = v.to_string();
        }
        if let Some(arr) = patch.get("allowed_skills").and_then(|v| v.as_array()) {
            skills = arr.iter().filter_map(|v| v.as_str().map(String::from)).collect();
        }
        if let Some(v) = patch.get("status").and_then(|v| v.as_str()) {
            status = v.to_string();
        }
        if let Some(v) = patch.get("max_depth").and_then(|v| v.as_u64()) {
            max_depth = v as u32;
        }
        if let Some(v) = patch.get("timeout_secs").and_then(|v| v.as_u64()) {
            timeout_secs = v as u32;
        }
        if let Some(v) = patch.get("metadata_json") {
            metadata = v.clone();
        }

        let now = chrono::Utc::now().to_rfc3339();
        let skills_json = serde_json::to_string(&skills).unwrap_or_else(|_| "[]".to_string());
        let meta_str = metadata.to_string();

        conn.execute(
            "UPDATE agent_definitions SET role=?1, trigger_event=?2, system_prompt=?3, allowed_skills=?4, status=?5, max_depth=?6, timeout_secs=?7, metadata_json=?8, updated_at=?9 WHERE id=?10",
            rusqlite::params![role, trigger, prompt, skills_json, status, max_depth, timeout_secs, meta_str, now, id],
        )
        .map_err(|e| format!("更新 Agent 失败: {}", e))?;

        self.reload_cache();
        self.event_bus.emit_agent_definition_changed("updated", &name);
        self.get_agent(&name).ok_or_else(|| "更新后找不到 Agent".to_string())
    }

    pub fn delete_agent(&self, id: &str) -> Result<(), String> {
        let name = {
            let conn = self.db.connect().map_err(|e| format!("连接数据库失败: {}", e))?;
            let name: String = conn
                .query_row("SELECT name FROM agent_definitions WHERE id = ?1", rusqlite::params![id], |row| row.get(0))
                .map_err(|e| format!("Agent 不存在: {}", e))?;
            conn.execute("DELETE FROM agent_definitions WHERE id = ?1", rusqlite::params![id])
                .map_err(|e| format!("删除 Agent 失败: {}", e))?;
            name
        };

        self.reload_cache();
        self.event_bus.emit_agent_definition_changed("deleted", &name);
        Ok(())
    }
}
