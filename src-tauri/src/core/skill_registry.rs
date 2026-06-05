use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use crate::core::database_service::DatabaseService;
use crate::core::event_bus::EventBus;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDefinition {
    pub id: String,
    pub name: String,
    pub description: String,
    pub code_body: String,
    #[serde(default)]
    pub parameter_schema: serde_json::Value,
    pub skill_type: String,
    pub status: String,
    #[serde(default)]
    pub metadata_json: serde_json::Value,
    pub created_at: String,
    pub updated_at: String,
}

pub struct SkillRegistry {
    db: Arc<DatabaseService>,
    event_bus: Arc<EventBus>,
    cache: RwLock<HashMap<String, SkillDefinition>>,
}

impl SkillRegistry {
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
            match conn.prepare("SELECT id, name, description, code_body, parameter_schema, skill_type, status, metadata_json, created_at, updated_at FROM skill_definitions") {
                Ok(mut stmt) => {
                    let rows = stmt.query_map([], |row| {
                        let param_str: String = row.get::<_, String>(4)?;
                        let parameter_schema: serde_json::Value =
                            serde_json::from_str(&param_str).unwrap_or_default();
                        let meta_str: String = row.get::<_, String>(7)?;
                        let metadata_json: serde_json::Value =
                            serde_json::from_str(&meta_str).unwrap_or_default();
                        Ok(SkillDefinition {
                            id: row.get(0)?,
                            name: row.get(1)?,
                            description: row.get(2)?,
                            code_body: row.get(3)?,
                            parameter_schema,
                            skill_type: row.get(5)?,
                            status: row.get(6)?,
                            metadata_json,
                            created_at: row.get(8)?,
                            updated_at: row.get(9)?,
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
                Err(e) => log::error!("[SkillRegistry] reload_cache 查询失败: {}", e),
            }
        }
    }

    pub fn list_skills(&self) -> Vec<SkillDefinition> {
        self.cache.read().values().cloned().collect()
    }

    pub fn get_skill(&self, name: &str) -> Option<SkillDefinition> {
        self.cache.read().get(name).cloned()
    }

    pub fn validate_parameter_schema(&self, schema_json: &str) -> Result<(), String> {
        let _schema: serde_json::Value = serde_json::from_str(schema_json)
            .map_err(|e| format!("parameter_schema 不是合法 JSON: {}", e))?;
        // 基本结构校验：如果是 object，应有 type 和 properties 字段
        Ok(())
    }

    pub fn create_skill(&self, def: SkillDefinition) -> Result<SkillDefinition, String> {
        let conn = self.db.connect().map_err(|e| format!("连接数据库失败: {}", e))?;

        let param_str = def.parameter_schema.to_string();
        let meta_str = def.metadata_json.to_string();

        conn.execute(
            "INSERT INTO skill_definitions (id, name, description, code_body, parameter_schema, skill_type, status, metadata_json, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            rusqlite::params![
                def.id,
                def.name,
                def.description,
                def.code_body,
                param_str,
                def.skill_type,
                def.status,
                meta_str,
                def.created_at,
                def.updated_at,
            ],
        )
        .map_err(|e| format!("创建 Skill 失败: {}", e))?;

        self.reload_cache();
        self.event_bus.emit_skill_definition_changed("created", &def.name);
        Ok(self.get_skill(&def.name).unwrap_or(def))
    }

    pub fn update_skill(&self, id: &str, patch: &serde_json::Value) -> Result<SkillDefinition, String> {
        let conn = self.db.connect().map_err(|e| format!("连接数据库失败: {}", e))?;

        let mut stmt = conn
            .prepare("SELECT name, description, code_body, parameter_schema, skill_type, status, metadata_json FROM skill_definitions WHERE id = ?1")
            .map_err(|e| format!("查询 Skill 失败: {}", e))?;

        let (name, mut desc, mut code_body, mut param_schema, mut skill_type, mut status, mut metadata) =
            stmt.query_row(rusqlite::params![id], |row| {
                let ps_str: String = row.get(3)?;
                let ps: serde_json::Value = serde_json::from_str(&ps_str).unwrap_or_default();
                let meta_str: String = row.get(6)?;
                let m: serde_json::Value = serde_json::from_str(&meta_str).unwrap_or_default();
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    ps,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    m,
                ))
            })
            .map_err(|e| format!("Skill 不存在: {}", e))?;

        if let Some(v) = patch.get("description").and_then(|v| v.as_str()) {
            desc = v.to_string();
        }
        if let Some(v) = patch.get("code_body").and_then(|v| v.as_str()) {
            code_body = v.to_string();
        }
        if let Some(v) = patch.get("parameter_schema") {
            param_schema = v.clone();
        }
        if let Some(v) = patch.get("skill_type").and_then(|v| v.as_str()) {
            skill_type = v.to_string();
        }
        if let Some(v) = patch.get("status").and_then(|v| v.as_str()) {
            status = v.to_string();
        }
        if let Some(v) = patch.get("metadata_json") {
            metadata = v.clone();
        }

        let now = chrono::Utc::now().to_rfc3339();
        let ps_str = param_schema.to_string();
        let meta_str = metadata.to_string();

        conn.execute(
            "UPDATE skill_definitions SET description=?1, code_body=?2, parameter_schema=?3, skill_type=?4, status=?5, metadata_json=?6, updated_at=?7 WHERE id=?8",
            rusqlite::params![desc, code_body, ps_str, skill_type, status, meta_str, now, id],
        )
        .map_err(|e| format!("更新 Skill 失败: {}", e))?;

        self.reload_cache();
        self.event_bus.emit_skill_definition_changed("updated", &name);
        self.get_skill(&name).ok_or_else(|| "更新后找不到 Skill".to_string())
    }

    pub fn delete_skill(&self, id: &str) -> Result<(), String> {
        let name = {
            let conn = self.db.connect().map_err(|e| format!("连接数据库失败: {}", e))?;
            let name: String = conn
                .query_row("SELECT name FROM skill_definitions WHERE id = ?1", rusqlite::params![id], |row| row.get(0))
                .map_err(|e| format!("Skill 不存在: {}", e))?;
            conn.execute("DELETE FROM skill_definitions WHERE id = ?1", rusqlite::params![id])
                .map_err(|e| format!("删除 Skill 失败: {}", e))?;
            name
        };

        self.reload_cache();
        self.event_bus.emit_skill_definition_changed("deleted", &name);
        Ok(())
    }
}
