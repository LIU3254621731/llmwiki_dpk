use std::path::{Path, PathBuf};
use std::sync::Arc;

use parking_lot::Mutex;

use crate::core::database_service::DatabaseService;
use crate::core::event_bus::EventBus;
use crate::embedding::engine::EmbeddingEngine;
use crate::embedding::vdb_config::EmbeddingConfig;
use crate::embedding::vdb_status::{VdbState, VdbStatus};

pub struct VdbService {
    db: Arc<DatabaseService>,
    event_bus: Arc<EventBus>,
    engine: Arc<Mutex<Option<Arc<EmbeddingEngine>>>>,
    model_dir: PathBuf,
    config_dir: PathBuf,
    status: Arc<Mutex<VdbState>>,
    error_message: Arc<Mutex<Option<String>>>,
}

impl VdbService {
    pub fn new(
        db: Arc<DatabaseService>,
        event_bus: Arc<EventBus>,
        model_dir: &Path,
        config_dir: &Path,
    ) -> Result<Self, String> {
        Ok(Self {
            db,
            event_bus,
            engine: Arc::new(Mutex::new(None)),
            model_dir: model_dir.to_path_buf(),
            config_dir: config_dir.to_path_buf(),
            status: Arc::new(Mutex::new(VdbState::Idle)),
            error_message: Arc::new(Mutex::new(None)),
        })
    }

    pub fn init_engine(&self, config: &EmbeddingConfig) -> Result<(), String> {
        let model_path = match config.engine_type.as_str() {
            "builtin" => self.model_dir.join("bge-small-zh-v1.5.onnx"),
            "custom" => {
                if let Some(ref path) = config.model_path {
                    PathBuf::from(path)
                } else {
                    return Err("自定义模型需要提供模型路径".to_string());
                }
            }
            "high_perf" => {
                return Err("高性能模型需要先下载，请将模型文件放入自定义路径后选择「自定义模型」".to_string());
            }
            _ => return Err(format!("未知的引擎类型: {}", config.engine_type)),
        };

        if !model_path.exists() {
            return Err(format!("模型文件不存在: {:?}", model_path));
        }

        let tokenizer_path = self.model_dir.join("tokenizer.json");
        if !tokenizer_path.exists() {
            return Err(format!("分词器文件不存在: {:?}", tokenizer_path));
        }

        let engine = EmbeddingEngine::new(
            &model_path,
            &tokenizer_path,
            config.num_threads,
            &config.graph_opt_level,
            config.max_seq_len,
            &config.pooling_strategy,
            config.l2_normalize,
        )?;
        *self.engine.lock() = Some(Arc::new(engine));
        Ok(())
    }

    pub fn get_status(&self, kb_id: &str) -> Result<VdbStatus, String> {
        let conn = self.db.connect()?;

        let (total_chunks, disk_size_bytes): (u64, u64) = conn
            .query_row(
                "SELECT COUNT(*), COALESCE(SUM(LENGTH(chunk_text) + LENGTH(embedding_json)), 0)
                 FROM vdb_chunks WHERE kb_id = ?1",
                rusqlite::params![kb_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|e| format!("查询 VDB 状态失败: {}", e))?;

        let status = self.status.lock().clone();
        let error_message = self.error_message.lock().clone();

        let vector_dimensions = self
            .engine
            .lock()
            .as_ref()
            .map(|e| e.model_dim() as u32)
            .unwrap_or(0);

        Ok(VdbStatus {
            kb_id: kb_id.to_string(),
            total_chunks,
            disk_size_bytes,
            vector_dimensions,
            status,
            error_message,
        })
    }

    /// 对单个 source 做增量索引（不清空其他 source 的已有 chunk）
    pub fn index_source(&self, kb_id: &str, source_id: &str) -> Result<(), String> {
        let engine_guard = self.engine.lock();
        if engine_guard.is_none() {
            return Err("请先在推理引擎设置中保存配置以加载模型".to_string());
        }
        drop(engine_guard);

        let kb_id_owned = kb_id.to_string();
        let source_id_owned = source_id.to_string();
        let db = self.db.clone();
        let engine = self.engine.clone();
        let eb = self.event_bus.clone();

        // Check that this source actually has extracted_text
        {
            let conn = self.db.connect()?;
            let text: Option<String> = conn
                .query_row(
                    "SELECT extracted_text FROM sources WHERE id = ?1 AND extracted_text != ''",
                    rusqlite::params![source_id],
                    |row| row.get(0),
                )
                .ok();
            if text.is_none() {
                return Err("该文档尚未提取文本，请等待处理完成".to_string());
            }
        }

        let kb_id_for_status = kb_id_owned.clone();
        let db2 = db.clone();

        tauri::async_runtime::spawn(async move {
            let result = tokio::task::spawn_blocking(move || {
                    let engine_guard = engine.lock();
                    let engine = match engine_guard.as_ref() {
                        Some(e) => e,
                        None => return Err("引擎未初始化".to_string()),
                    };

                    let conn = db.connect()?;

                    // Only delete chunks for this specific source
                    conn.execute(
                        "DELETE FROM vdb_chunks WHERE kb_id = ?1 AND source_id = ?2",
                        rusqlite::params![kb_id_owned, source_id_owned],
                    )
                    .map_err(|e| format!("清除旧索引失败: {}", e))?;

                    // Get the source's extracted text
                    let text: String = conn
                        .query_row(
                            "SELECT extracted_text FROM sources WHERE id = ?1",
                            rusqlite::params![source_id_owned],
                            |row| row.get(0),
                        )
                        .map_err(|e| format!("查询 source 文本失败: {}", e))?;

                    let chunks = chunk_text(&text, 512);
                    let mut chunks_stored: u64 = 0;

                    for (ci, chunk) in chunks.iter().enumerate() {
                        match engine.embed(chunk) {
                            Ok(embedding) => {
                                let embedding_json = serde_json::to_string(&embedding)
                                    .unwrap_or_else(|_| "[]".to_string());
                                let id = uuid::Uuid::new_v4().to_string();
                                let now = chrono::Utc::now().to_rfc3339();
                                if let Err(e) = conn.execute(
                                    "INSERT INTO vdb_chunks (id, kb_id, chunk_text, embedding_json, source_id, page_path, chunk_index, created_at)
                                     VALUES (?1, ?2, ?3, ?4, ?5, '', ?6, ?7)",
                                    rusqlite::params![id, kb_id_owned, chunk, embedding_json, source_id_owned, ci as i32, now],
                                ) {
                                    log::error!("存储 chunk 失败: {}", e);
                                } else {
                                    chunks_stored += 1;
                                }
                            }
                            Err(e) => {
                                log::error!("embed 失败 for source {} chunk {}: {}", source_id_owned, ci, e);
                            }
                        }
                    }

                    log::info!(
                        "[VDB] 增量索引完成: source={}, {} chunks stored",
                        source_id_owned, chunks_stored
                    );

                    Ok((source_id_owned, chunks_stored))
                })
                .await;

            match &result {
                Ok(Ok((_source_id, chunks_stored))) => {
                    eb.emit_notification(
                        "info",
                        "向量库",
                        &format!("增量索引完成: {} 个 chunks 已添加", chunks_stored),
                    );
                    // Emit updated status for UI
                    if let Ok(conn) = db2.connect() {
                        let (total, disk): (u64, u64) = conn
                            .query_row(
                                "SELECT COUNT(*), COALESCE(SUM(LENGTH(chunk_text) + LENGTH(embedding_json)), 0)
                                 FROM vdb_chunks WHERE kb_id = ?1",
                                rusqlite::params![kb_id_for_status],
                                |row| Ok((row.get(0)?, row.get(1)?)),
                            )
                            .unwrap_or((0, 0));
                        let status = crate::embedding::vdb_status::VdbStatus {
                            kb_id: kb_id_for_status,
                            total_chunks: total,
                            disk_size_bytes: disk,
                            vector_dimensions: 0,
                            status: crate::embedding::vdb_status::VdbState::Idle,
                            error_message: None,
                        };
                        eb.emit_vdb_status_changed(&status);
                    }
                }
                Ok(Err(e)) => {
                    eb.emit_notification("error", "向量库", &format!("增量索引失败: {}", e));
                }
                Err(e) => {
                    eb.emit_notification("error", "向量库", &format!("后台任务异常: {}", e));
                }
            }
        });

        Ok(())
    }

    pub fn start_reindex(&self, kb_id: &str) -> Result<(), String> {
        let engine_guard = self.engine.lock();
        if engine_guard.is_none() {
            return Err("请先在推理引擎设置中保存配置以加载模型".to_string());
        }
        drop(engine_guard);

        let kb_id_owned = kb_id.to_string();
        let kb_id_for_status = kb_id_owned.clone();
        let db = self.db.clone();
        let db2 = self.db.clone();
        let eb = self.event_bus.clone();
        let eb2 = self.event_bus.clone();
        let engine = self.engine.clone();
        let status_clone = self.status.clone();
        let err_msg_clone = self.error_message.clone();

        // Set status to Indexing, clear previous error
        *self.status.lock() = VdbState::Indexing;
        *self.error_message.lock() = None;

        tauri::async_runtime::spawn(async move {
            let result = tokio::task::spawn_blocking(move || {
                let engine_guard = engine.lock();
                let engine = match engine_guard.as_ref() {
                    Some(e) => e,
                    None => return Err("引擎未初始化".to_string()),
                };

                let conn = db.connect()?;

                // Clear existing chunks for this KB before reindexing
                conn.execute(
                    "DELETE FROM vdb_chunks WHERE kb_id = ?1",
                    rusqlite::params![kb_id_owned],
                )
                .map_err(|e| format!("清除旧索引失败: {}", e))?;

                // Check how many total sources exist vs how many have text
                let total_sources: u64 = conn
                    .query_row(
                        "SELECT COUNT(*) FROM sources WHERE kb_id = ?1",
                        rusqlite::params![kb_id_owned],
                        |row| row.get(0),
                    )
                    .unwrap_or(0);

                // Get all sources with extracted text
                let mut stmt = conn
                    .prepare(
                        "SELECT id, extracted_text FROM sources WHERE kb_id = ?1 AND extracted_text != ''",
                    )
                    .map_err(|e| format!("查询 sources 失败: {}", e))?;

                let sources: Vec<(String, String)> = stmt
                    .query_map(rusqlite::params![kb_id_owned], |row| {
                        Ok((row.get(0)?, row.get(1)?))
                    })
                    .map_err(|e| format!("映射 sources 失败: {}", e))?
                    .filter_map(|r| r.ok())
                    .collect();

                let total = sources.len() as u64;

                log::info!(
                    "[VDB] KB {}: {} 个文档中有 {} 个已提取文本",
                    kb_id_owned, total_sources, total
                );

                if total == 0 {
                    if total_sources == 0 {
                        return Err("该知识库尚无文档，请先上传并处理文档".to_string());
                    } else {
                        return Err(format!(
                            "{} 个文档中均无已提取的文本，请先运行文档处理任务",
                            total_sources
                        ));
                    }
                }

                let model_dim = engine.model_dim() as u32;
                let mut chunks_stored: u64 = 0;
                let mut embed_errors: u64 = 0;

                for (i, (source_id, text)) in sources.iter().enumerate() {
                    eb.emit_reindex_progress(&kb_id_owned, i as u64, total, "正在分块并嵌入...");

                    let chunks = chunk_text(text, 512);
                    for (ci, chunk) in chunks.iter().enumerate() {
                        match engine.embed(chunk) {
                            Ok(embedding) => {
                                let embedding_json = serde_json::to_string(&embedding)
                                    .unwrap_or_else(|_| "[]".to_string());
                                let id = uuid::Uuid::new_v4().to_string();
                                let now = chrono::Utc::now().to_rfc3339();
                                if let Err(e) = conn.execute(
                                    "INSERT INTO vdb_chunks (id, kb_id, chunk_text, embedding_json, source_id, page_path, chunk_index, created_at)
                                     VALUES (?1, ?2, ?3, ?4, ?5, '', ?6, ?7)",
                                    rusqlite::params![id, kb_id_owned, chunk, embedding_json, source_id, ci as i32, now],
                                ) {
                                    log::error!("存储 chunk 失败: {}", e);
                                } else {
                                    chunks_stored += 1;
                                }
                            }
                            Err(e) => {
                                embed_errors += 1;
                                if embed_errors <= 3 {
                                    log::error!("embed 失败 for source {} chunk {}: {}", source_id, ci, e);
                                }
                            }
                        }
                    }
                }

                if embed_errors > 0 {
                    log::warn!(
                        "[VDB] KB {}: 共 {} 次 embedding 失败, {} chunks 已存储",
                        kb_id_owned, embed_errors, chunks_stored
                    );
                }

                log::info!(
                    "[VDB] KB {}: 索引完成 — {} sources, {} chunks stored, {} embed errors",
                    kb_id_owned, total, chunks_stored, embed_errors
                );

                Ok::<_, String>((sources.len() as u64, chunks_stored, embed_errors, model_dim))
            })
            .await;

            // Update status and error message
            {
                let mut s = status_clone.lock();
                let mut em = err_msg_clone.lock();
                match &result {
                    Ok(Ok((_, _, _, _))) => {
                        *s = VdbState::Idle;
                        *em = None;
                    }
                    Ok(Err(e)) => {
                        *s = VdbState::Error;
                        *em = Some(e.clone());
                    }
                    Err(e) => {
                        *s = VdbState::Error;
                        *em = Some(format!("后台任务异常: {}", e));
                    }
                }
            }

            // Emit final notification with stats
            let (sources_count, chunks_stored, embed_errors, model_dim) = match &result {
                Ok(Ok(stats)) => (stats.0, stats.1, stats.2, stats.3),
                _ => (0, 0, 0, 0u32),
            };

            match result {
                Ok(Ok(_)) => {
                    let msg = if embed_errors > 0 {
                        format!(
                            "索引构建完成: {} 个文档 → {} chunks, {} 次失败",
                            sources_count, chunks_stored, embed_errors
                        )
                    } else {
                        format!(
                            "索引构建完成: {} 个文档 → {} chunks",
                            sources_count, chunks_stored
                        )
                    };
                    eb2.emit_notification("info", "向量库", &msg);
                }
                Ok(Err(e)) => {
                    eb2.emit_notification("error", "向量库", &format!("索引构建失败: {}", e));
                }
                Err(e) => {
                    eb2.emit_notification("error", "向量库", &format!("后台任务异常: {}", e));
                }
            }

            // Emit updated status for UI refresh — read actual chunk count from DB
            let (actual_chunks, actual_disk): (u64, u64) = if sources_count > 0 {
                match db2.connect() {
                    Ok(conn) => conn
                        .query_row(
                            "SELECT COUNT(*), COALESCE(SUM(LENGTH(chunk_text) + LENGTH(embedding_json)), 0)
                             FROM vdb_chunks WHERE kb_id = ?1",
                            rusqlite::params![kb_id_for_status],
                            |row| Ok((row.get(0)?, row.get(1)?)),
                        )
                        .unwrap_or((0, 0)),
                    Err(_) => (0, 0),
                }
            } else {
                (0, 0)
            };

            let status = status_clone.lock();
            let err_msg = err_msg_clone.lock();
            let latest_status = VdbStatus {
                kb_id: kb_id_for_status,
                total_chunks: actual_chunks,
                disk_size_bytes: actual_disk,
                vector_dimensions: model_dim,
                status: status.clone(),
                error_message: err_msg.clone(),
            };
            drop(status);
            drop(err_msg);
            eb2.emit_vdb_status_changed(&latest_status);
        });

        Ok(())
    }

    pub fn flush(&self, kb_id: &str) -> Result<(), String> {
        let conn = self.db.connect()?;
        conn.execute(
            "DELETE FROM vdb_chunks WHERE kb_id = ?1",
            rusqlite::params![kb_id],
        )
        .map_err(|e| format!("清空向量库失败: {}", e))?;

        self.event_bus.emit_notification("info", "向量库", "向量库已清空");
        Ok(())
    }

    pub fn get_config(&self) -> Result<EmbeddingConfig, String> {
        let config_path = self.config_dir.join("embedding.json");
        if config_path.exists() {
            let content =
                std::fs::read_to_string(&config_path).map_err(|e| format!("读取配置失败: {}", e))?;
            serde_json::from_str(&content)
                .map_err(|e| format!("解析配置失败: {}", e))
        } else {
            Ok(EmbeddingConfig::default())
        }
    }

    /// 语义搜索：将查询文本向量化，与 vdb_chunks 中所有向量计算余弦相似度，
    /// 返回 Top-K 结果 (chunk_text, similarity_score)。
    pub fn search_similar(
        &self,
        kb_id: &str,
        query_text: &str,
        top_k: usize,
    ) -> Result<Vec<(String, f64)>, String> {
        let engine_guard = self.engine.lock();
        let engine = engine_guard
            .as_ref()
            .ok_or("嵌入引擎未初始化，请先在设置中配置向量数据库")?;

        let query_embedding = engine.embed(query_text)?;

        let conn = self.db.connect()?;
        let mut stmt = conn
            .prepare("SELECT chunk_text, embedding_json FROM vdb_chunks WHERE kb_id = ?1")
            .map_err(|e| format!("查询 vdb_chunks 失败: {}", e))?;

        let rows: Vec<(String, String)> = stmt
            .query_map(rusqlite::params![kb_id], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .map_err(|e| format!("映射 vdb_chunks 失败: {}", e))?
            .filter_map(|r| r.ok())
            .collect();

        if rows.is_empty() {
            return Ok(Vec::new());
        }

        let mut scored: Vec<(String, f64)> = rows
            .iter()
            .filter_map(|(chunk_text, emb_json)| {
                let emb: Vec<f32> = serde_json::from_str(emb_json).ok()?;
                if emb.len() != query_embedding.len() {
                    return None;
                }
                let sim = cosine_similarity(&query_embedding, &emb);
                Some((chunk_text.clone(), sim))
            })
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(top_k);

        Ok(scored)
    }

    pub fn save_config(&self, config: &EmbeddingConfig) -> Result<(), String> {
        let config_path = self.config_dir.join("embedding.json");
        let content =
            serde_json::to_string_pretty(config).map_err(|e| format!("序列化配置失败: {}", e))?;
        std::fs::write(&config_path, content).map_err(|e| format!("保存配置失败: {}", e))?;
        Ok(())
    }
}

fn chunk_text(text: &str, max_chars: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut start = 0usize;
    while start < chars.len() {
        let end = (start + max_chars).min(chars.len());
        let mut break_point = end;
        if end < chars.len() {
            for i in (start..end).rev() {
                if matches!(chars[i], '。' | '！' | '？' | '\n' | '；') {
                    break_point = i + 1;
                    break;
                }
            }
        }
        let chunk: String = chars[start..break_point].iter().collect();
        if !chunk.trim().is_empty() {
            chunks.push(chunk);
        }
        start = break_point;
    }
    chunks
}

/// 余弦相似度计算: dot(a,b) / (|a| * |b|)
fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    let eps = 1e-12f32;
    (dot as f64) / ((norm_a * norm_b + eps) as f64)
}
