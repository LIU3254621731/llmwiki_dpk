use rusqlite::Connection;
use std::path::{Path, PathBuf};
use crate::db::migrations;

pub struct DatabaseService {
    db_path: PathBuf,
}

impl DatabaseService {
    pub fn new(app_data_dir: &Path) -> Result<Self, String> {
        let db_path = app_data_dir.join("app.sqlite");
        let db_exists = db_path.exists();

        let conn = Connection::open(&db_path)
            .map_err(|e| format!("无法打开数据库: {}", e))?;

        // 执行迁移
        migrations::run_migrations(&conn)?;

        // 初始化 model_profile 默认记录（如果不存在）
        let count: i64 = match conn
            .query_row(
                "SELECT COUNT(*) FROM model_profiles",
                [],
                |row| row.get(0),
            ) {
                Ok(c) => c,
                Err(rusqlite::Error::QueryReturnedNoRows) => 0,
                Err(e) => return Err(format!("查询 model_profiles 失败: {}", e)),
            };

        if count == 0 {
            let now = chrono::Utc::now().to_rfc3339();
            conn.execute(
                "INSERT INTO model_profiles (id, provider, name, base_url, model_name, encrypted_api_key_ref, role, temperature, max_tokens, timeout, retry_count, created_at, updated_at)
                 VALUES (?1, 'deepseek', '默认DeepSeek配置', 'https://api.deepseek.com', 'deepseek-chat', '', 'chat', 0.7, 4096, 120, 3, ?2, ?2)",
                rusqlite::params![uuid::Uuid::new_v4().to_string(), now],
            )
            .map_err(|e| format!("创建默认模型配置失败: {}", e))?;
        }

        if !db_exists {
            log::info!("数据库已创建: {:?}", db_path);
        }

        Ok(Self { db_path })
    }

    pub fn connect(&self) -> Result<Connection, String> {
        Connection::open(&self.db_path)
            .map_err(|e| format!("无法连接数据库: {}", e))
    }

    pub fn path(&self) -> &Path {
        &self.db_path
    }
}
