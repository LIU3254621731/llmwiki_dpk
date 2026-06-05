use std::sync::Arc;
use serde::{Deserialize, Serialize};
use crate::core::database_service::DatabaseService;
use crate::core::config_service::ConfigService;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenContext {
    pub task_id: String,
    pub task_name: String,
    pub agent_name: String,
    pub model_name: String,
    pub provider: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenStats {
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_cost_yuan: f64,
    pub call_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyTokenUsage {
    pub date: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenLogEntry {
    pub id: String,
    pub task_id: String,
    pub task_name: String,
    pub agent_name: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub model_name: String,
    pub provider: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedTokenLogs {
    pub entries: Vec<TokenLogEntry>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyTokenLimit {
    pub enabled: bool,
    pub limit: u64,
}

impl Default for DailyTokenLimit {
    fn default() -> Self {
        Self {
            enabled: false,
            limit: 2_000_000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenQuotaStatus {
    pub allowed: bool,
    pub today_used: u64,
    pub limit: u64,
    pub remaining: u64,
    pub message: String,
}

pub struct TokenLogger {
    db: Arc<DatabaseService>,
    config: Arc<ConfigService>,
}

impl TokenLogger {
    pub fn new(db: Arc<DatabaseService>, config: Arc<ConfigService>) -> Self {
        Self { db, config }
    }

    /// 记录一次 API 调用的 Token 消耗
    pub fn log_usage(&self, ctx: &TokenContext, input_tokens: u32, output_tokens: u32) -> Result<(), String> {
        let conn = self.db.connect().map_err(|e| format!("连接数据库失败: {}", e))?;
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();

        conn.execute(
            "INSERT INTO token_logs (id, task_id, task_name, agent_name, input_tokens, output_tokens, model_name, provider, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![id, ctx.task_id, ctx.task_name, ctx.agent_name, input_tokens, output_tokens, ctx.model_name, ctx.provider, now],
        )
        .map_err(|e| format!("记录 Token 日志失败: {}", e))?;

        log::info!(
            "[TokenLogger] 记录消耗: task={}, agent={}, input={}, output={}",
            ctx.task_name, ctx.agent_name, input_tokens, output_tokens
        );
        Ok(())
    }

    /// 获取 Token 统计数据
    pub fn get_statistics(&self, range: &str) -> Result<TokenStats, String> {
        let conn = self.db.connect().map_err(|e| format!("连接数据库失败: {}", e))?;

        let date_filter = match range {
            "today" => "date(created_at) = date('now')",
            "7days" => "created_at >= datetime('now', '-7 days')",
            "month" => "strftime('%Y-%m', created_at) = strftime('%Y-%m', 'now')",
            _ => return Err(format!("未知的时间范围: {}", range)),
        };

        let sql = format!(
            "SELECT COALESCE(SUM(input_tokens), 0), COALESCE(SUM(output_tokens), 0), COUNT(*)
             FROM token_logs WHERE {}",
            date_filter
        );

        let (total_input, total_output, call_count): (u64, u64, u64) = conn
            .query_row(&sql, [], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })
            .map_err(|e| format!("查询 Token 统计失败: {}", e))?;

        // 费率: 输入 1元/百万token, 输出 2元/百万token
        let cost = (total_input as f64 / 1_000_000.0) * 1.0 + (total_output as f64 / 1_000_000.0) * 2.0;

        Ok(TokenStats {
            total_input_tokens: total_input,
            total_output_tokens: total_output,
            total_cost_yuan: (cost * 100.0).round() / 100.0,
            call_count,
        })
    }

    /// 获取最近 7 天的每日 Token 消耗趋势
    pub fn get_daily_trend(&self) -> Result<Vec<DailyTokenUsage>, String> {
        let conn = self.db.connect().map_err(|e| format!("连接数据库失败: {}", e))?;

        let mut stmt = conn
            .prepare(
                "SELECT date(created_at) as d, COALESCE(SUM(input_tokens), 0), COALESCE(SUM(output_tokens), 0)
                 FROM token_logs
                 WHERE created_at >= datetime('now', '-7 days')
                 GROUP BY d
                 ORDER BY d ASC",
            )
            .map_err(|e| format!("准备查询失败: {}", e))?;

        let rows = stmt
            .query_map([], |row| {
                Ok(DailyTokenUsage {
                    date: row.get(0)?,
                    input_tokens: row.get(1)?,
                    output_tokens: row.get(2)?,
                })
            })
            .map_err(|e| format!("查询每日趋势失败: {}", e))?;

        let mut result = Vec::new();
        for row in rows {
            result.push(row.map_err(|e| format!("读取行失败: {}", e))?);
        }
        Ok(result)
    }

    /// 获取分页的 Token 日志
    pub fn get_logs(&self, page: u64, page_size: u64) -> Result<PaginatedTokenLogs, String> {
        let conn = self.db.connect().map_err(|e| format!("连接数据库失败: {}", e))?;

        let total: u64 = conn
            .query_row("SELECT COUNT(*) FROM token_logs", [], |row| row.get(0))
            .map_err(|e| format!("查询日志总数失败: {}", e))?;

        let offset = (page.saturating_sub(1)) * page_size;
        let mut stmt = conn
            .prepare(
                "SELECT id, task_id, task_name, agent_name, input_tokens, output_tokens, model_name, provider, created_at
                 FROM token_logs
                 ORDER BY created_at DESC
                 LIMIT ?1 OFFSET ?2",
            )
            .map_err(|e| format!("准备查询失败: {}", e))?;

        let rows = stmt
            .query_map(rusqlite::params![page_size, offset], |row| {
                Ok(TokenLogEntry {
                    id: row.get(0)?,
                    task_id: row.get(1)?,
                    task_name: row.get(2)?,
                    agent_name: row.get(3)?,
                    input_tokens: row.get(4)?,
                    output_tokens: row.get(5)?,
                    model_name: row.get(6)?,
                    provider: row.get(7)?,
                    created_at: row.get(8)?,
                })
            })
            .map_err(|e| format!("查询日志失败: {}", e))?;

        let mut entries = Vec::new();
        for row in rows {
            entries.push(row.map_err(|e| format!("读取行失败: {}", e))?);
        }

        Ok(PaginatedTokenLogs {
            entries,
            total,
            page,
            page_size,
        })
    }

    /// 获取今日已消耗的总 Token（用于熔断检查）
    pub fn get_today_usage(&self) -> Result<u64, String> {
        let conn = self.db.connect().map_err(|e| format!("连接数据库失败: {}", e))?;

        let today_total: u64 = conn
            .query_row(
                "SELECT COALESCE(SUM(input_tokens) + SUM(output_tokens), 0) FROM token_logs WHERE date(created_at) = date('now')",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        Ok(today_total)
    }

    /// 获取每日限额配置
    pub fn get_daily_limit(&self) -> Result<DailyTokenLimit, String> {
        let path = self.config.get_config_dir().join("token_limit.json");
        if path.exists() {
            let content = std::fs::read_to_string(&path)
                .map_err(|e| format!("读取 Token 限额配置失败: {}", e))?;
            serde_json::from_str(&content)
                .map_err(|e| format!("解析 Token 限额配置失败: {}", e))
        } else {
            Ok(DailyTokenLimit::default())
        }
    }

    /// 保存每日限额配置
    pub fn save_daily_limit(&self, limit: &DailyTokenLimit) -> Result<(), String> {
        let path = self.config.get_config_dir().join("token_limit.json");
        let json = serde_json::to_string_pretty(limit)
            .map_err(|e| format!("序列化 Token 限额配置失败: {}", e))?;
        std::fs::write(&path, json)
            .map_err(|e| format!("保存 Token 限额配置失败: {}", e))?;
        Ok(())
    }

    /// 熔断检查：返回是否允许继续调用 API
    pub fn check_quota(&self) -> Result<TokenQuotaStatus, String> {
        let limit = self.get_daily_limit()?;

        if !limit.enabled {
            return Ok(TokenQuotaStatus {
                allowed: true,
                today_used: 0,
                limit: 0,
                remaining: 0,
                message: "每日限额未启用".to_string(),
            });
        }

        let today_used = self.get_today_usage()?;
        let remaining = if today_used >= limit.limit {
            0
        } else {
            limit.limit - today_used
        };
        let allowed = today_used < limit.limit;

        let message = if allowed {
            format!(
                "今日已消耗 {} tokens，剩余 {} tokens",
                today_used, remaining
            )
        } else {
            format!(
                "今日 Token 额度已耗尽！已消耗 {} / {} tokens",
                today_used, limit.limit
            )
        };

        Ok(TokenQuotaStatus {
            allowed,
            today_used,
            limit: limit.limit,
            remaining,
            message,
        })
    }
}
