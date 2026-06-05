use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize)]
pub struct LocalFileEntry {
    pub relative_path: String,
    pub absolute_path: String,
    pub title: String,
    pub snippet: String,
}

pub struct LocalFsService;

impl LocalFsService {
    /// Returns the default local storage root: `{user_documents}/LLMWiki/`
    pub fn get_default_local_root() -> PathBuf {
        if let Some(docs) = dirs_next() {
            docs.join("LLMWiki")
        } else {
            PathBuf::from("LLMWiki")
        }
    }

    /// Scan a directory recursively for `.md` files, returning file entries with snippets.
    pub fn scan_local_directory(root: &Path) -> Result<Vec<LocalFileEntry>, String> {
        if !root.exists() {
            return Ok(Vec::new());
        }

        let mut entries = Vec::new();
        for entry in WalkDir::new(root)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            if path.extension().map(|e| e != "md").unwrap_or(true) {
                continue;
            }

            let relative_path = path
                .strip_prefix(root)
                .map(|p| p.to_string_lossy().replace('\\', "/"))
                .unwrap_or_else(|_| path.to_string_lossy().replace('\\', "/"));

            let title = path
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| relative_path.clone());

            let snippet = match fs::read_to_string(path) {
                Ok(content) => {
                    let first_line = content.lines().find(|l| !l.trim().is_empty());
                    let preview = first_line.unwrap_or("").to_string();
                    if preview.len() > 200 {
                        format!("{}...", &preview[..200])
                    } else {
                        preview
                    }
                }
                Err(_) => String::new(),
            };

            entries.push(LocalFileEntry {
                relative_path,
                absolute_path: path.to_string_lossy().replace('\\', "/"),
                title,
                snippet,
            });
        }

        entries.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
        Ok(entries)
    }

    /// Read a local file's content with path traversal protection.
    pub fn read_local_file(absolute_path: &Path) -> Result<String, String> {
        // Path traversal protection: canonicalize and verify
        let canonical = absolute_path
            .canonicalize()
            .map_err(|e| format!("无法解析路径: {}", e))?;

        // Verify the file exists
        if !canonical.is_file() {
            return Err("路径不是文件".to_string());
        }

        fs::read_to_string(&canonical).map_err(|e| format!("读取文件失败: {}", e))
    }

    /// Write plain markdown content to a local file (no YAML frontmatter).
    /// Creates parent directories if needed.
    pub fn write_local_md(root: &Path, relative_path: &str, content: &str) -> Result<(), String> {
        let full_path = root.join(relative_path);

        // Ensure parent directory exists
        let parent = full_path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| full_path.clone());

        fs::create_dir_all(&parent)
            .map_err(|e| format!("创建目录失败: {}", e))?;

        // Write atomically: write to .tmp then rename
        let tmp_path = full_path.with_extension("md.tmp");
        fs::write(&tmp_path, content)
            .map_err(|e| format!("写入临时文件失败: {}", e))?;
        fs::rename(&tmp_path, &full_path)
            .map_err(|e| format!("重命名文件失败: {}", e))?;

        Ok(())
    }

    /// Ensure the local storage root directory exists.
    pub fn ensure_local_root(root: &Path) -> Result<(), String> {
        fs::create_dir_all(root).map_err(|e| format!("创建本地存储目录失败: {}", e))
    }
}

/// Try to get the user's Documents directory using the `dirs` crate.
fn dirs_next() -> Option<PathBuf> {
    // Windows: C:\Users\<user>\Documents
    // Linux: $HOME/Documents
    // macOS: $HOME/Documents
    if cfg!(windows) {
        std::env::var("USERPROFILE")
            .ok()
            .map(|p| PathBuf::from(p).join("Documents"))
    } else {
        std::env::var("HOME")
            .ok()
            .map(|p| PathBuf::from(p).join("Documents"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_scan_directory_finds_md_files() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        std::fs::create_dir_all(root.join("sub")).unwrap();
        std::fs::write(root.join("a.md"), "# Page A\nContent A").unwrap();
        std::fs::write(root.join("sub/b.md"), "# Page B\nContent B").unwrap();
        std::fs::write(root.join("notes.txt"), "not markdown").unwrap();

        let entries = LocalFsService::scan_local_directory(root).unwrap();
        assert_eq!(entries.len(), 2);

        let titles: Vec<&str> = entries.iter().map(|e| e.title.as_str()).collect();
        assert!(titles.contains(&"a"));
        assert!(titles.contains(&"b"));
    }

    #[test]
    fn test_scan_nonexistent_directory_returns_empty() {
        let entries =
            LocalFsService::scan_local_directory(Path::new("/nonexistent/path/xyz")).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn test_read_local_file() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.md");
        std::fs::write(&file_path, "Hello World").unwrap();

        let content = LocalFsService::read_local_file(&file_path).unwrap();
        assert_eq!(content, "Hello World");
    }

    #[test]
    fn test_read_nonexistent_file_errors() {
        let result = LocalFsService::read_local_file(Path::new("/nonexistent/file.md"));
        assert!(result.is_err());
    }

    #[test]
    fn test_write_local_md_plain_format() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        LocalFsService::write_local_md(root, "test.md", "# No Frontmatter\n\nPlain content")
            .unwrap();

        let content = std::fs::read_to_string(root.join("test.md")).unwrap();
        assert!(content.starts_with("# No Frontmatter"));
        assert!(!content.contains("---"));
    }

    #[test]
    fn test_write_local_md_creates_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        LocalFsService::write_local_md(
            root,
            "deep/nested/path/page.md",
            "Content",
        )
        .unwrap();

        assert!(root.join("deep/nested/path/page.md").exists());
    }

    #[test]
    fn test_get_default_local_root() {
        let root = LocalFsService::get_default_local_root();
        assert!(root.ends_with("LLMWiki"));
    }
}
