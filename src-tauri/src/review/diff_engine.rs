// DiffEngine - 生成文本 Diff 数据

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DiffHunk {
    pub old_start: u32,
    pub old_lines: u32,
    pub new_start: u32,
    pub new_lines: u32,
    pub content: String,
    pub change_type: String, // "add", "delete", "change"
}

pub struct DiffEngine;

impl DiffEngine {
    /// 生成两个文本之间的 diff（行比较，窗口搜索同步点）
    pub fn generate_diff(old_text: &str, new_text: &str) -> Vec<DiffHunk> {
        let old_lines: Vec<&str> = old_text.lines().collect();
        let new_lines: Vec<&str> = new_text.lines().collect();

        let mut hunks = Vec::new();
        let mut i = 0usize;
        let mut j = 0usize;
        let max_window = 20;

        while i < old_lines.len() || j < new_lines.len() {
            if i < old_lines.len() && j < new_lines.len() && old_lines[i] == new_lines[j] {
                i += 1;
                j += 1;
                continue;
            }

            // Joint search for next matching point (i+d, j+a) minimizing d+a
            let mut del_count = 0;
            let mut add_count = 0;
            let mut found = false;

            'search: for w in 0..=max_window {
                for d in 0..=w {
                    let a = w - d;
                    if i + d < old_lines.len()
                        && j + a < new_lines.len()
                        && old_lines[i + d] == new_lines[j + a]
                    {
                        del_count = d;
                        add_count = a;
                        found = true;
                        break 'search;
                    }
                }
            }

            if !found {
                // No match within window — bulk change for remaining lines
                del_count = if i < old_lines.len() {
                    std::cmp::min(old_lines.len() - i, max_window)
                } else {
                    0
                };
                add_count = if j < new_lines.len() {
                    std::cmp::min(new_lines.len() - j, max_window)
                } else {
                    0
                };
            }

            let mut content = String::new();
            if del_count > 0 {
                for k in 0..del_count {
                    content.push_str(&format!("- {}\n", old_lines[i + k]));
                }
            }
            if add_count > 0 {
                for k in 0..add_count {
                    content.push_str(&format!("+ {}\n", new_lines[j + k]));
                }
            }

            let change_type = match (del_count > 0, add_count > 0) {
                (true, true) => "change",
                (true, false) => "delete",
                (false, true) => "add",
                (false, false) => continue, // no change, skip
            };

            hunks.push(DiffHunk {
                old_start: i as u32,
                old_lines: del_count as u32,
                new_start: j as u32,
                new_lines: add_count as u32,
                content,
                change_type: change_type.to_string(),
            });

            i += del_count;
            j += add_count;
        }

        hunks
    }
}
