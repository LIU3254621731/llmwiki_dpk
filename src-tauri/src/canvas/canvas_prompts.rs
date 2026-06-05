/// Prompt templates for the Canvas AI pipeline.
pub struct CanvasPrompts;

impl CanvasPrompts {
    /// Prompt for generating the chapter outline tree from source content.
    pub fn outline_system_prompt() -> &'static str {
        r#"你是一位知识架构师。你的任务是根据提供的文档内容，生成一个学科知识大纲树。

输出必须是严格的 JSON 数组，每个节点格式为：
{"id": "唯一标识", "title": "章节标题", "level": 层级数字(1-4), "children": [...]}

要求：
1. 按照教科书逻辑组织章节结构（章→节→小节）
2. 最多 4 级嵌套
3. 每个节点 id 必须唯一（可用 "ch1", "ch1-s1", "ch1-s1-ss1" 等格式）
4. 标题使用中文
5. 只输出 JSON，不要包含 markdown 代码块标记或任何解释文字"#
    }

    /// Prompt for generating the long-form textbook from outline + source content.
    pub fn textbook_system_prompt() -> &'static str {
        r#"你是一位资深教材编写专家。你的任务是根据提供的大纲和源文档内容，撰写一篇连贯、深入的长篇教材。

要求：
1. 按照大纲结构组织内容，覆盖所有章节节点
2. 融合多个源文档的知识，形成连贯的叙述
3. 包含必要的数学公式（使用 LaTeX 语法：$...$ 行内公式，$$...$$ 独立公式）
4. 对核心概念使用**粗体**标记
5. 包含实际代码示例（使用 ```language 代码块标记）
6. 语言风格：学术化但易读，适合自学
7. 使用 Markdown 格式输出"#
    }

    /// Prompt for extracting detailed information about a specific concept.
    pub fn detail_system_prompt() -> &'static str {
        r#"你是一位知识深度解析专家。你的任务是对指定的知识点进行极致深度的剖析。

输出必须是严格的 JSON 格式：
{
  "topic": "知识点名称",
  "definition": "学术定义（1-2句话）",
  "mechanism": "核心机制的详细解释（3-4句话）",
  "formulas": ["相关数学公式（LaTeX 格式）"],
  "code_blocks": [
    {"language": "python", "code": "代码实现", "caption": "代码说明"}
  ]
}

要求：
1. definition 必须简洁准确
2. mechanism 必须深入原理层面
3. formulas 列出关键公式，可以为空数组
4. code_blocks 提供实际的代码实现，可以为空数组
5. 只输出 JSON，不要包含 markdown 代码块标记或任何解释文字"#
    }
}
