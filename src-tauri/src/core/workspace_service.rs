use std::fs;
use std::path::{Path, PathBuf};

pub struct WorkspaceService;

impl Default for WorkspaceService {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkspaceService {
    pub fn new() -> Self {
        Self
    }

    /// 清理旧 workspace 数据文件（raw/drafts/versions/.runtime/schema/db/config/wiki 目录下的所有文件）
    /// 在创建新 KB 前调用，确保旧安装残留的文件不会混入新 KB
    pub fn clean_old_workspace_data(&self, root_path: &Path) {
        // 这些是 workspace 管理的数据目录，可以安全清理
        let managed_dirs = [
            "raw",
            "drafts",
            "versions",
            ".runtime",
            "schema",
            "db",
            "config",
        ];

        for dir in &managed_dirs {
            let full = root_path.join(dir);
            if full.exists() {
                if let Err(e) = fs::remove_dir_all(&full) {
                    log::warn!("[WorkspaceService] 清理旧目录失败 ({}): {}", full.display(), e);
                } else {
                    log::info!("[WorkspaceService] 已清理旧目录: {}", full.display());
                }
            }
        }

        // wiki/ 目录：删除除 index.md/log.md/overview.md 以外的所有文件
        let wiki_dir = root_path.join("wiki");
        if wiki_dir.exists() {
            if let Ok(entries) = fs::read_dir(&wiki_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    // 保留这三个核心文件（后续 init_workspace 会按需覆盖）
                    if name != "index.md" && name != "log.md" && name != "overview.md" {
                        if path.is_dir() {
                            let _ = fs::remove_dir_all(&path);
                        } else {
                            let _ = fs::remove_file(&path);
                        }
                    }
                }
            }
        }
    }

    /// 初始化知识库 workspace 目录结构
    pub fn init_workspace(&self, root_path: &Path) -> Result<(), String> {
        let dirs = vec![
            "raw/sources/documents",
            "raw/sources/webclips",
            "raw/assets/images",
            "raw/assets/attachments",
            "wiki/sources",
            "wiki/concepts",
            "wiki/entities",
            "wiki/datasets",
            "wiki/methods",
            "wiki/topics",
            "wiki/questions",
            "wiki/reviews",
            "schema",
            "drafts/ingest",
            "drafts/wiki_updates",
            "versions/snapshots",
            "versions/pages",
            ".runtime/tasks",
            ".runtime/logs",
            ".runtime/source_previews",
            "db",
            "config",
        ];

        for dir in &dirs {
            let full_path = root_path.join(dir);
            fs::create_dir_all(&full_path)
                .map_err(|e| format!("创建目录失败 {}: {}", full_path.display(), e))?;
        }

        // 创建初始 index.md
        let index_path = root_path.join("wiki/index.md");
        if !index_path.exists() {
            let index_content = format!(
                "# 知识库索引\n\n> 创建时间: {}\n\n## 页面列表\n\n暂无页面。\n",
                chrono::Utc::now().format("%Y-%m-%d %H:%M:%S")
            );
            fs::write(&index_path, index_content)
                .map_err(|e| format!("创建 index.md 失败: {}", e))?;
        }

        // 创建初始 log.md
        let log_path = root_path.join("wiki/log.md");
        if !log_path.exists() {
            let log_content = format!(
                "# 知识库操作日志\n\n## {} - 知识库创建\n\n知识库已初始化。\n\n",
                chrono::Utc::now().format("%Y-%m-%d %H:%M:%S")
            );
            fs::write(&log_path, log_content)
                .map_err(|e| format!("创建 log.md 失败: {}", e))?;
        }

        // 创建初始 overview.md
        let overview_path = root_path.join("wiki/overview.md");
        if !overview_path.exists() {
            fs::write(&overview_path, "# 知识库概览\n\n欢迎使用 LLMWiki 知识库！\n")
                .map_err(|e| format!("创建 overview.md 失败: {}", e))?;
        }

        log::info!("Workspace 目录已初始化: {:?}", root_path);
        Ok(())
    }

    /// 检查 workspace 结构是否完整
    pub fn validate_workspace(&self, root_path: &Path) -> Vec<String> {
        let required_dirs = vec![
            "raw/sources/documents",
            "wiki",
            ".runtime/tasks",
            "db",
            "config",
        ];
        let mut missing = Vec::new();
        for dir in &required_dirs {
            if !root_path.join(dir).exists() {
                missing.push(dir.to_string());
            }
        }
        missing
    }

    /// 获取 .runtime/tasks 目录
    pub fn get_tasks_dir(&self, root_path: &Path) -> PathBuf {
        root_path.join(".runtime/tasks")
    }

    /// 获取 raw/sources/documents 目录
    pub fn get_documents_dir(&self, root_path: &Path) -> PathBuf {
        root_path.join("raw/sources/documents")
    }

    /// 获取 raw/assets/images 目录
    pub fn get_images_dir(&self, root_path: &Path) -> PathBuf {
        root_path.join("raw/assets/images")
    }

    /// 获取 versions 目录
    pub fn get_versions_dir(&self, root_path: &Path) -> PathBuf {
        root_path.join("versions")
    }

    /// 获取 wiki 目录
    pub fn get_wiki_dir(&self, root_path: &Path) -> PathBuf {
        root_path.join("wiki")
    }
}
