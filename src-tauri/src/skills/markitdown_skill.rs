/// MarkItDown Skill — 通过 Python sidecar 调用 Microsoft MarkItDown
/// 将 PDF/DOCX/PPTX/XLSX/CSV/JSON/XML/Image 等格式统一转换为 Markdown
/// 文档: https://github.com/microsoft/markitdown
///
/// 首次使用自动通过 pip 安装 markitdown；若系统无 Python 则提示安装 Miniconda/Python。
use std::path::Path;
use std::process::Command;
use std::sync::Mutex;

/// 可用性缓存（首次检测后缓存结果，可通过 reset_cache 重置）
static AVAILABILITY: Mutex<Option<bool>> = Mutex::new(None);
/// 已尝试过自动安装（避免反复 pip install）
static INSTALL_ATTEMPTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// 创建无控制台窗口的子进程（Windows 下抑制 cmd 弹窗）
fn silent_command(program: &str) -> Command {
    let mut cmd = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

pub struct MarkitdownSkill;

impl MarkitdownSkill {
    /// 清除可用性缓存（安装 markitdown 后调用，允许重新检测）
    pub fn reset_cache() {
        if let Ok(mut cache) = AVAILABILITY.lock() {
            *cache = None;
        }
    }

    /// 重置安装状态（允许重新尝试自动安装）
    pub fn reset_install_state() {
        Self::reset_cache();
        INSTALL_ATTEMPTED.store(false, std::sync::atomic::Ordering::Relaxed);
    }

    /// 检查 Python 是否可用
    pub fn has_python() -> bool {
        Self::find_python().is_ok()
    }

    /// 检测 markitdown 是否可用（结果缓存；首次检测不可用时尝试自动安装）
    pub fn is_available() -> bool {
        // 先检查缓存
        if let Ok(cache) = AVAILABILITY.lock() {
            if let Some(cached) = *cache {
                return cached;
            }
        }

        if Self::find_command().is_ok() {
            if let Ok(mut cache) = AVAILABILITY.lock() {
                *cache = Some(true);
            }
            return true;
        }

        // 尝试自动安装
        if !INSTALL_ATTEMPTED.swap(true, std::sync::atomic::Ordering::Relaxed) {
            if let Err(e) = Self::auto_install() {
                log::error!("[markitdown] 自动安装失败: {}", e);
            }
            // 重新检测
            if Self::find_command().is_ok() {
                if let Ok(mut cache) = AVAILABILITY.lock() {
                    *cache = Some(true);
                }
                return true;
            }
        }

        if let Ok(mut cache) = AVAILABILITY.lock() {
            *cache = Some(false);
        }
        false
    }

    /// 自动安装 markitdown
    pub fn auto_install() -> Result<String, String> {
        let python = Self::find_python()?;

        // 尝试 pip install markitdown
        log::info!("[markitdown] 正在自动安装 markitdown (pip install markitdown)...");

        // 优先从本地源码安装（搜索多个可能的路径）
        let project_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .to_path_buf();

        let local_candidates = [project_root.join("markitdown-main").join("packages").join("markitdown"),
            project_root.join("markitdown-main")];

        let local_src = local_candidates.iter().find(|p| {
            p.join("pyproject.toml").exists() || p.join("setup.py").exists()
        });

        let output = if let Some(src) = local_src {
            log::info!("[markitdown] 使用本地源码安装: {}", src.display());
            silent_command(&python)
                .args(["-m", "pip", "install", "-q", "--no-input"])
                .arg(src.to_string_lossy().as_ref())
                .output()
                .map_err(|e| format!("无法启动 {}: {}", python, e))
        } else {
            silent_command(&python)
                .args(["-m", "pip", "install", "-q", "--no-input", "markitdown"])
                .output()
                .map_err(|e| format!("无法启动 {}: {}", python, e))
        }?;

        if output.status.success() {
            Ok("MarkItDown 安装成功".to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("pip install 失败: {}", stderr.trim()))
        }
    }

    /// 查找可用的 Python 解释器
    fn find_python() -> Result<String, String> {
        for candidate in &["python", "python3", "py"] {
            if let Ok(output) = silent_command(candidate).args(["--version"]).output() {
                if output.status.success() {
                    return Ok(candidate.to_string());
                }
            }
        }
        Err(
            "未找到 Python 运行环境。请安装 Python（https://python.org 或 Microsoft Store 搜索 Python 3.12）后重启应用。"
                .to_string(),
        )
    }

    /// 查找可用的 markitdown 命令，返回 (cmd_binary, extra_args)
    fn find_command() -> Result<(&'static str, Vec<&'static str>), String> {
        for (cmd, args) in &[
            ("markitdown", vec![]),
            ("python", vec!["-m", "markitdown"]),
            ("python3", vec!["-m", "markitdown"]),
        ] {
            let mut c = silent_command(cmd);
            for a in args {
                c.arg(a);
            }
            c.arg("--version");
            if let Ok(output) = c.output() {
                if output.status.success() {
                    return Ok((cmd, args.clone()));
                }
            }
        }
        Err("MarkItDown 未安装。请等待自动安装，或手动运行: pip install markitdown".to_string())
    }

    /// 将文件转换为 Markdown 文本
    pub fn convert(file_path: &Path) -> Result<String, String> {
        let (cmd, args) = Self::find_command()?;

        let mut command = silent_command(cmd);
        // 强制 Python 使用 UTF-8 编码输出，防止中文 Windows 上默认使用 GBK 导致乱码
        command.env("PYTHONIOENCODING", "utf-8");
        command.env("PYTHONUTF8", "1");
        for a in &args {
            command.arg(a);
        }
        command.arg(file_path);

        let output = command
            .output()
            .map_err(|e| format!("调用 MarkItDown 失败（{} 不可执行）: {}", cmd, e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            let detail = if !stderr.trim().is_empty() {
                stderr.trim().to_string()
            } else {
                stdout.trim().to_string()
            };
            return Err(format!("MarkItDown 转换失败: {}", detail));
        }

        let stdout_bytes = output.stdout;
        // 优先尝试 UTF-8；如果高字节字符占比异常（>30% 在 Latin-1 补充区），
        // 尝试 GBK 解码（中文 Windows 系统编码）
        let text = Self::decode_stdout(&stdout_bytes);
        if text.trim().is_empty() {
            return Err("MarkItDown 返回了空内容，该文件可能不包含可提取的文本。".to_string());
        }

        Ok(text)
    }

    /// 解码子进程 stdout 字节，UTF-8 优先，异常时回退到 GBK
    fn decode_stdout(bytes: &[u8]) -> String {
        let utf8_text = String::from_utf8_lossy(bytes);
        // 检测高比例 Latin-1 补充区字符（U+0080-U+0230），通常意味着 UTF-8 中文字节被误读
        let suspect_count = utf8_text
            .chars()
            .filter(|c| (*c as u32) > 0x007F && (*c as u32) < 0x0230)
            .count();
        let total = utf8_text.chars().count();
        if total > 0 && suspect_count > total / 3 {
            // 尝试用 GBK 解码
            let (gbk_text, _encoding, _malformed) = encoding_rs::GBK.decode(bytes);
            return gbk_text.trim().to_string();
        }
        utf8_text.trim().to_string()
    }
}
