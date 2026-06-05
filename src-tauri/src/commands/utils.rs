use crate::skills::markitdown_skill::MarkitdownSkill;

#[tauri::command]
pub async fn shell_open(path: String) -> Result<(), String> {
    open::that(&path).map_err(|e| format!("打开文件/文件夹失败: {}", e))
}

/// 获取 MarkItDown 当前状态（可用于健康检查/设置页展示）
#[tauri::command]
pub async fn get_markitdown_status() -> Result<serde_json::Value, String> {
    let available = MarkitdownSkill::is_available();
    Ok(serde_json::json!({
        "available": available,
        "python_found": MarkitdownSkill::has_python(),
        "description": if available {
            "MarkItDown 可用，支持 PDF/DOCX/PPTX/XLSX/CSV/JSON/XML 等格式转换"
        } else if MarkitdownSkill::has_python() {
            "Python 已安装但 MarkItDown 未安装。请点击安装按钮。"
        } else {
            "未找到 Python 运行环境。请安装 Python 后再试。"
        },
    }))
}

/// 手动重新安装 MarkItDown（重置缓存后重新检测）
#[tauri::command]
pub async fn retry_markitdown_install() -> Result<serde_json::Value, String> {
    MarkitdownSkill::reset_install_state();
    let result = MarkitdownSkill::auto_install()?;
    // 重新检测
    let available = MarkitdownSkill::is_available();
    Ok(serde_json::json!({
        "install_result": result,
        "available": available,
    }))
}
