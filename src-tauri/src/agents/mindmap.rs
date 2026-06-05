// MindmapAgent — 将 Wiki 索引和内容作为上下文，
// 利用大模型生成标准树状 JSON，供前端 AntV X6 MicroCanvas 渲染交互式思维导图。

use std::sync::Arc;
use crate::core::database_service::DatabaseService;
use crate::core::config_service::ConfigService;
use crate::core::workspace_service::WorkspaceService;
use crate::model::model_gateway::ModelGateway;
use crate::schema::json_repair;
use crate::graph::topology_engine::TopologyEngine;

pub struct MindmapAgent;

/// MindmapAgent 输出的树节点结构
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MindmapTreeNode {
    pub id: String,
    pub topic: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<MindmapTreeNode>>,
}

impl MindmapAgent {
    /// 根据指定主题生成思维导图树
    /// - `kb_id`: 知识库 ID
    /// - `topic`: 根节点主题（如 "机器学习"）
    /// - `context_pages`: 可选的额外上下文页面内容
    pub async fn generate(
        db: &Arc<DatabaseService>,
        workspace: &Arc<WorkspaceService>,
        config: &Arc<ConfigService>,
        gateway: &Arc<ModelGateway>,
        kb_id: &str,
        topic: &str,
        context_pages: Option<&str>,
    ) -> Result<MindmapTreeNode, String> {
        let provider_config = config.get_provider_config()?;

        // 收集 Wiki 页面索引作为上下文
        let wiki_context = Self::collect_wiki_context(db, workspace, kb_id)?;

        let extra_context = context_pages.unwrap_or("");
        let system_prompt = Self::build_system_prompt();
        let user_prompt = Self::build_user_prompt(topic, &wiki_context, extra_context);

        let messages = vec![
            crate::model::model_gateway::ChatMessage {
                role: "system".to_string(),
                content: system_prompt,
            },
            crate::model::model_gateway::ChatMessage {
                role: "user".to_string(),
                content: user_prompt,
            },
        ];

        let result = gateway
            .chat(&provider_config, messages, true)
            .await
            .map_err(|e| format!("MindmapAgent LLM 调用失败: {}", e))?;

        // 修复并校验 JSON
        let json = json_repair::validate_and_repair_json(&result.content)?;

        let root: MindmapTreeNode = serde_json::from_value(json)
            .map_err(|e| format!("MindmapAgent 输出解析失败: {}", e))?;

        // 为所有节点补全 id（如果 LLM 未提供）
        let root = Self::ensure_ids(root);

        Ok(root)
    }

    fn collect_wiki_context(
        db: &Arc<DatabaseService>,
        workspace: &Arc<WorkspaceService>,
        kb_id: &str,
    ) -> Result<String, String> {
        let conn = db.connect()?;
        let kb_path: String = conn
            .query_row(
                "SELECT path FROM knowledge_bases WHERE id = ?1",
                rusqlite::params![kb_id],
                |row| row.get(0),
            )
            .map_err(|_| "知识库不存在".to_string())?;

        let workspace_root = std::path::PathBuf::from(&kb_path);
        let wiki_dir = workspace.get_wiki_dir(&workspace_root);

        let mut context = String::new();

        // ── 1. 收集 index.md 摘要 ──
        let index_path = wiki_dir.join("index.md");
        if index_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&index_path) {
                context.push_str(&format!(
                    "# 知识库索引\n\n{}\n\n",
                    &content[..content.len().min(3000)]
                ));
            }
        }

        // ── 2. 收集 Wiki 页面标题列表 + 读内容提取 wikilink ──
        let mut stmt = conn
            .prepare(
                "SELECT title, page_type, path FROM wiki_pages WHERE kb_id = ?1 LIMIT 50",
            )
            .map_err(|e| format!("查询页面失败: {}", e))?;

        let pages: Vec<(String, String, String)> = stmt
            .query_map(rusqlite::params![kb_id], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })
            .map_err(|e| format!("映射页面失败: {}", e))?
            .filter_map(|r| r.ok())
            .collect();

        // 构建页面路径集合用于过滤死链
        let valid_paths: std::collections::HashSet<String> = pages
            .iter()
            .map(|(title, _, _)| title.to_lowercase())
            .collect();

        // ── 3. 收集目录结构 ──
        let dir_tree = Self::collect_directory_tree(&wiki_dir);
        if !dir_tree.is_empty() {
            context.push_str("# 知识库目录结构\n\n");
            context.push_str(&dir_tree);
            context.push_str("\n\n");
        }

        // ── 4. 收集页面双链关系 ──
        if !pages.is_empty() {
            context.push_str("# 现有 Wiki 页面\n\n");
            for (title, page_type, path) in &pages {
                context.push_str(&format!("- [{}] {} ({})\n", page_type, title, path));
            }
            context.push('\n');

            // 提取每个页面的 wikilink 并构建邻接表
            let topology = Self::collect_topology_context(&wiki_dir, &pages, &valid_paths);
            if !topology.is_empty() {
                context.push_str("# 页面双链关系\n\n");
                context.push_str(&topology);
                context.push_str("\n\n");
            }
        }

        // ── 5. 收集图谱边关系 ──
        let graph_ctx = Self::collect_graph_edges(db, kb_id)?;
        if !graph_ctx.is_empty() {
            context.push_str("# 知识图谱关系\n\n");
            context.push_str(&graph_ctx);
            context.push_str("\n\n");
        }

        Ok(context)
    }

    /// 扫描 wiki 目录树，生成缩进文本
    fn collect_directory_tree(wiki_dir: &std::path::Path) -> String {
        use std::collections::BTreeMap;
        use std::fs;

        if !wiki_dir.exists() {
            return String::new();
        }

        // 收集所有 .md 文件路径并构建树
        let mut entries: Vec<String> = Vec::new();
        if let Ok(dir_entries) = fs::read_dir(wiki_dir) {
            for entry in dir_entries.flatten() {
                let path = entry.path();
                let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                if name.starts_with('.') || name == "index.md" || name == "log.md" {
                    continue;
                }
                if path.is_dir() {
                    // 递归收集子目录中的 .md 文件
                    let sub_entries = Self::walk_dir_for_md(&path, &name);
                    entries.extend(sub_entries);
                } else if name.ends_with(".md") {
                    entries.push(name.trim_end_matches(".md").to_string());
                }
            }
        }

        if entries.is_empty() {
            return String::new();
        }

        // 按路径分组构建树状文本
        let mut tree = String::new();
        let mut grouped: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for entry in &entries {
            if let Some((dir, file)) = entry.rsplit_once('/') {
                grouped.entry(dir.to_string()).or_default().push(file.to_string());
            } else {
                grouped.entry(String::new()).or_default().push(entry.clone());
            }
        }

        for (dir, files) in &grouped {
            if dir.is_empty() {
                for f in files {
                    tree.push_str(&format!("├── {}.md\n", f));
                }
            } else {
                tree.push_str(&format!("{}/\n", dir));
                for f in files {
                    tree.push_str(&format!("  ├── {}.md\n", f));
                }
            }
        }

        tree
    }

    fn walk_dir_for_md(dir: &std::path::Path, prefix: &str) -> Vec<String> {
        let mut results = Vec::new();
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                if name.starts_with('.') {
                    continue;
                }
                let rel = format!("{}/{}", prefix, name);
                if path.is_dir() {
                    results.extend(Self::walk_dir_for_md(&path, &rel));
                } else if name.ends_with(".md") {
                    results.push(rel.trim_end_matches(".md").to_string());
                }
            }
        }
        results
    }

    /// 提取每个页面的 wikilink 并构建邻接表
    fn collect_topology_context(
        wiki_dir: &std::path::Path,
        pages: &[(String, String, String)],
        valid_paths: &std::collections::HashSet<String>,
    ) -> String {
        use std::collections::HashMap;

        // 每个页面的出链列表
        let mut adjacency: Vec<(String, Vec<String>)> = Vec::new();
        // 被引用计数
        let mut ref_count: HashMap<String, usize> = HashMap::new();

        for (_title, _page_type, rel_path) in pages.iter().take(30) {
            let file_path = wiki_dir.join(rel_path);
            if let Ok(content) = std::fs::read_to_string(&file_path) {
                let content_preview = &content[..content.len().min(5000)];
                let links = TopologyEngine::extract_wikilinks(content_preview);
                // 过滤：只保留指向实际存在页面的链接
                let valid_links: Vec<String> = links
                    .into_iter()
                    .filter(|l| {
                        let key = l.to_lowercase();
                        valid_paths.contains(&key)
                            || valid_paths.contains(&format!("{}.md", l).to_lowercase())
                    })
                    .collect();
                for link in &valid_links {
                    *ref_count.entry(link.to_lowercase()).or_insert(0) += 1;
                }
                if !valid_links.is_empty() {
                    let page_name = std::path::Path::new(rel_path)
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(|| rel_path.clone());
                    adjacency.push((page_name, valid_links));
                }
            }
        }

        if adjacency.is_empty() {
            return String::new();
        }

        // 按被引用次数降序排列（高连接度节点在前）
        adjacency.sort_by(|a, b| {
            let a_max = a.1.iter().map(|l| ref_count.get(&l.to_lowercase()).copied().unwrap_or(0)).max().unwrap_or(0);
            let b_max = b.1.iter().map(|l| ref_count.get(&l.to_lowercase()).copied().unwrap_or(0)).max().unwrap_or(0);
            b_max.cmp(&a_max)
        });

        let mut result = String::new();
        for (source, targets) in adjacency.iter().take(20) {
            let target_str = targets
                .iter()
                .map(|t| format!("[[{}]]", t))
                .collect::<Vec<_>>()
                .join(", ");
            result.push_str(&format!("{} → {}\n", source, target_str));
        }

        result
    }

    /// 查询 graph_edges 表获取已建立的知识图谱关系
    fn collect_graph_edges(
        db: &Arc<DatabaseService>,
        kb_id: &str,
    ) -> Result<String, String> {
        let conn = db.connect()?;

        let mut stmt = conn
            .prepare(
                "SELECT sn.label, tn.label, e.edge_type \
                 FROM graph_edges e \
                 JOIN graph_nodes sn ON e.source_node_id = sn.id \
                 JOIN graph_nodes tn ON e.target_node_id = tn.id \
                 WHERE e.kb_id = ?1 LIMIT 30",
            )
            .map_err(|e| format!("查询图谱边失败: {}", e))?;

        let edges: Vec<(String, String, String)> = stmt
            .query_map(rusqlite::params![kb_id], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })
            .map_err(|e| format!("映射图谱边失败: {}", e))?
            .filter_map(|r| r.ok())
            .collect();

        if edges.is_empty() {
            return Ok(String::new());
        }

        let mut result = String::new();
        for (source, target, edge_type) in &edges {
            result.push_str(&format!("{} --[{}]--> {}\n", source, edge_type, target));
        }

        Ok(result)
    }

    fn build_system_prompt() -> String {
        r#"你是一个知识结构专家。你的任务是根据给定的主题和知识库上下文，
生成一个标准的多层思维导图树状结构。

上下文包含以下信息（若存在则使用）：
- 知识库目录结构：按文件夹分组的页面组织方式，优先按此结构组织树的第一层
- 页面双链关系：`页面A → [[页面B]], [[页面C]]` 表示页面A引用了页面B和页面C，被引用最多的页面应作为一级子节点
- 知识图谱关系：`A --[包含]--> B` 表示A包含B，双向链接的页面在树中保持父子关系

输出格式要求（严格 JSON）：
{
  "id": "root",
  "topic": "根主题名称",
  "children": [
    {
      "id": "子节点唯一标识",
      "topic": "子主题名称",
      "children": [...]
    }
  ]
}

规则：
1. 根节点的 topic 使用用户指定的主题。
2. 深度控制在 2-4 层，每层 2-6 个子节点。
3. 子节点的 id 使用有意义的小写英文标识（如 "supervised_learning"）。
4. topic 使用中文，简洁明了（不超过 15 个字）。
5. 叶节点（没有 children 的节点）不需要输出 children 字段。
6. 仅输出 JSON，不要输出其他文本。
7. 若提供了双链关系或目录结构，优先使用这些信息决定树的组织方式。高连接度（被引用多）的页面优先作为一级子节点。
8. 若没有上下文信息，则基于主题自由生成合理的知识结构。"#
            .to_string()
    }

    fn build_user_prompt(topic: &str, wiki_context: &str, extra: &str) -> String {
        format!(
            "请为主题「{}」生成思维导图树。\n\n{}\n\n{}\n\n请生成 JSON：",
            topic, wiki_context, extra
        )
    }

    /// 确保树中所有节点都有 id（缺失则自动生成 UUID）
    fn ensure_ids(mut node: MindmapTreeNode) -> MindmapTreeNode {
        if node.id.is_empty() {
            node.id = uuid::Uuid::new_v4().to_string();
        }
        if let Some(ref mut children) = node.children {
            for child in children.iter_mut() {
                let child_owned = std::mem::replace(
                    child,
                    MindmapTreeNode {
                        id: String::new(),
                        topic: String::new(),
                        children: None,
                    },
                );
                *child = Self::ensure_ids(child_owned);
            }
        }
        node
    }
}
