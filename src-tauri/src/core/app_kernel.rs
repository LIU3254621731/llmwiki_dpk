use std::sync::Arc;
use tauri::{AppHandle, Manager};
use crate::core::database_service::DatabaseService;
use crate::core::config_service::ConfigService;
use crate::core::secret_service::SecretService;
use crate::core::workspace_service::WorkspaceService;
use crate::core::event_bus::EventBus;
use crate::core::task_queue::TaskQueue;
use crate::core::token_logger::TokenLogger;
use crate::core::agent_registry::AgentRegistry;
use crate::core::skill_registry::SkillRegistry;
use crate::embedding::vdb_service::VdbService;
use crate::recovery::recovery_check::RecoveryCheck;
use crate::core::health_monitor::HealthMonitor;

pub struct AppKernel {
    pub db: Arc<DatabaseService>,
    pub config: Arc<ConfigService>,
    pub secrets: Arc<SecretService>,
    pub workspace: Arc<WorkspaceService>,
    pub event_bus: Arc<EventBus>,
    pub token_logger: Arc<TokenLogger>,
    pub vdb: Arc<VdbService>,
    pub agent_registry: Arc<AgentRegistry>,
    pub skill_registry: Arc<SkillRegistry>,
    pub health_monitor: Arc<HealthMonitor>,
}

impl AppKernel {
    pub fn new(app: &AppHandle) -> Result<Self, String> {
        log::info!("[kernel] 获取应用数据目录...");
        let app_data_dir = app
            .path()
            .app_data_dir()
            .map_err(|e| format!("获取应用数据目录失败: {}", e))?;
        log::info!("[kernel] 应用数据目录: {:?}", app_data_dir);

        log::info!("[kernel] 创建应用数据目录...");
        std::fs::create_dir_all(&app_data_dir)
            .map_err(|e| format!("创建应用数据目录失败: {}", e))?;

        log::info!("[kernel] 初始化数据库...");
        let db = Arc::new(DatabaseService::new(&app_data_dir)?);
        log::info!("[kernel] 数据库初始化完成");

        log::info!("[kernel] 初始化配置服务...");
        let config = Arc::new(ConfigService::new(&app_data_dir));
        log::info!("[kernel] 初始化密钥服务...");
        let secrets = Arc::new(SecretService::new(config.get_config_dir()));
        log::info!("[kernel] 初始化工作区服务...");
        let workspace = Arc::new(WorkspaceService::new());
        log::info!("[kernel] 初始化事件总线...");
        let event_bus = Arc::new(EventBus::new(app.clone()));
        log::info!("[kernel] 初始化任务队列...");
        let tq = Arc::new(TaskQueue::new(db.clone(), event_bus.clone()));

        let db_for_recovery = db.clone();
        let tq_for_recovery = tq.clone();
        let event_bus_for_recovery = event_bus.clone();

        log::info!("[kernel] 启动异步恢复检查...");
        tauri::async_runtime::spawn(async move {
            if let Err(e) = Self::run_recovery_checks(&db_for_recovery, &tq_for_recovery, &event_bus_for_recovery).await {
                log::error!("恢复检查失败: {}", e);
            }
        });

        log::info!("[kernel] 初始化 Token 日志服务...");
        let token_logger = Arc::new(TokenLogger::new(db.clone(), config.clone()));

        log::info!("[kernel] 初始化向量数据库服务...");
        let resource_dir = app
            .path()
            .resource_dir()
            .map_err(|e| format!("获取资源目录失败: {}", e))?;
        // 生产环境: resource_dir/models/ (bundle.resources 直接映射到 models/)
        // 开发环境: resource_dir/resources/models/ (源文件路径)
        let model_dir_prod = resource_dir.join("models");
        let model_dir = if model_dir_prod.exists() {
            model_dir_prod
        } else {
            resource_dir.join("resources").join("models")
        };
        log::info!("[kernel] 模型目录: {:?}", model_dir);
        let vdb = Arc::new(VdbService::new(
            db.clone(),
            event_bus.clone(),
            &model_dir,
            config.get_config_dir(),
        )?);

        // 异步自动加载嵌入模型（不阻塞启动）
        let vdb_auto = vdb.clone();
        tauri::async_runtime::spawn(async move {
            match vdb_auto.get_config() {
                Ok(cfg) => {
                    if let Err(e) = vdb_auto.init_engine(&cfg) {
                        log::warn!("[kernel] 自动加载嵌入模型失败: {}", e);
                    } else {
                        log::info!("[kernel] 嵌入模型自动加载完成");
                    }
                }
                Err(e) => log::warn!("[kernel] 读取嵌入配置失败: {}", e),
            }
        });

        log::info!("[kernel] 初始化 Agent 注册表...");
        let agent_registry = Arc::new(AgentRegistry::new(db.clone(), event_bus.clone()));
        log::info!("[kernel] 初始化 Skill 注册表...");
        let skill_registry = Arc::new(SkillRegistry::new(db.clone(), event_bus.clone()));

        log::info!("[kernel] 初始化代码化健康监测引擎...");
        let health_monitor = Arc::new(HealthMonitor::new(
            db.clone(),
            config.clone(),
            event_bus.clone(),
        ));
        health_monitor.start();

        log::info!("[kernel] 内核初始化完成");
        Ok(Self {
            db,
            config,
            secrets,
            workspace,
            event_bus,
            token_logger,
            vdb,
            agent_registry,
            skill_registry,
            health_monitor,
        })
    }

    async fn run_recovery_checks(
        db: &Arc<DatabaseService>,
        task_queue: &Arc<TaskQueue>,
        event_bus: &Arc<EventBus>,
    ) -> Result<(), String> {
        let conn = db.connect().map_err(|e| format!("连接数据库失败: {}", e))?;
        let mut stmt = conn.prepare("SELECT id, name, path FROM knowledge_bases").map_err(|e| format!("准备查询失败: {}", e))?;
        let kbs: Vec<(String, String, String)> = {
            let mut kbs = Vec::new();
            let mut rows = stmt.query([]).map_err(|e| format!("查询失败: {}", e))?;
            while let Some(row) = rows.next().map_err(|e| format!("读取行失败: {}", e))? {
                let id: String = row.get(0).map_err(|e| format!("获取字段失败: {}", e))?;
                let name: String = row.get(1).map_err(|e| format!("获取字段失败: {}", e))?;
                let path: String = row.get(2).map_err(|e| format!("获取字段失败: {}", e))?;
                kbs.push((id, name, path));
            }
            kbs
        };

        // 清理上次会话可能遗留的卡在 cancelling 状态的任务
        if let Err(e) = task_queue.cleanup_stuck_cancelling() {
            log::error!("[recovery] 清理卡住的取消任务失败: {}", e);
        }

        for (kb_id, kb_name, kb_path) in &kbs {
            match RecoveryCheck::run(db, task_queue, kb_id, kb_path) {
                Ok(actions) => {
                    for action in &actions {
                        event_bus.emit_notification("info", "恢复检查", action);
                    }
                    if !actions.is_empty() {
                        event_bus.emit_notification("info", &format!("知识库: {}", kb_name), &format!("发现 {} 个问题已处理", actions.len()));
                    }
                }
                Err(e) => {
                    event_bus.emit_notification("warning", "恢复检查", &format!("检查失败: {}", e));
                }
            }
        }

        Ok(())
    }
}
