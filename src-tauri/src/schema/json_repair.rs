/// JSON 修复工具
/// 尝试修复 LLM 返回的非法 JSON
///
/// 从任意文本中提取 JSON block
pub fn extract_json_block(text: &str) -> Option<String> {
    // 尝试提取 ```json ... ``` 代码块
    if let Some(start) = text.find("```json") {
        let after_start = &text[start + 7..];
        if let Some(end) = after_start.find("```") {
            return Some(after_start[..end].trim().to_string());
        }
    }

    // 尝试提取 ``` ... ``` 代码块
    if let Some(start) = text.find("```") {
        let after_start = &text[start + 3..];
        if let Some(end) = after_start.find("```") {
            let block = after_start[..end].trim();
            if block.starts_with('{') || block.starts_with('[') {
                return Some(block.to_string());
            }
        }
    }

    // 尝试提取 { ... } 或 [ ... ]
    let trimmed = text.trim();
    if let Some(start) = trimmed.find('{') {
        let mut depth = 0;
        let mut in_string = false;
        let mut escape = false;

        for (i, c) in trimmed[start..].char_indices() {
            if escape {
                escape = false;
                continue;
            }
            match c {
                '"' => in_string = !in_string,
                '\\' => escape = true,
                '{' if !in_string => depth += 1,
                '}' if !in_string => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(trimmed[start..start + i + 1].to_string());
                    }
                }
                _ => {}
            }
        }
    }

    None
}

/// JSON 修复
pub fn repair_json(text: &str) -> String {
    let mut result = text.to_string();

    // 仅去除开头/结尾的 markdown 代码围栏，避免破坏 JSON 字符串内的反引号
    if let Some(rest) = result.strip_prefix("```json") {
        result = rest.to_string();
    } else if let Some(rest) = result.strip_prefix("```") {
        result = rest.to_string();
    }
    if let Some(rest) = result.strip_suffix("```") {
        result = rest.to_string();
    }

    if let Some(start) = result.find('{') {
        if let Some(end) = result.rfind('}') {
            result = result[start..=end].to_string();
        } else {
            result = result[start..].to_string();
        }
    }

    result = result.trim().to_string();
    result = fix_python_literals(&result);
    result = fix_single_quoted_strings(&result);
    result = fix_missing_commas(&result);
    result = fix_unclosed_strings(&result);
    result = result.replace(",}", "}").replace(",]", "]");
    result = close_unclosed_brackets(&result);

    result
}

/// Replace Python-style literals outside of strings: True→true, False→false, None→null
fn fix_python_literals(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut in_double_string = false;
    let mut escape = false;
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();

    let mut i = 0;
    while i < len {
        let c = chars[i];
        if escape {
            escape = false;
            result.push(c);
            i += 1;
            continue;
        }
        match c {
            '"' => {
                in_double_string = !in_double_string;
                result.push(c);
            }
            '\\' => {
                escape = true;
                result.push(c);
            }
            'T' if !in_double_string && i + 3 < len => {
                if text[i..].starts_with("True") {
                    result.push_str("true");
                    i += 4;
                    continue;
                }
                result.push(c);
            }
            'F' if !in_double_string && i + 4 < len => {
                if text[i..].starts_with("False") {
                    result.push_str("false");
                    i += 5;
                    continue;
                }
                result.push(c);
            }
            'N' if !in_double_string && i + 3 < len => {
                if text[i..].starts_with("None") {
                    result.push_str("null");
                    i += 4;
                    continue;
                }
                result.push(c);
            }
            _ => { result.push(c); }
        }
        i += 1;
    }

    result
}

/// Convert single-quoted strings to double-quoted strings (LLMs often use Python-style quoting)
fn fix_single_quoted_strings(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut in_double_string = false;
    let mut in_single_string = false;
    let mut escape = false;

    for c in text.chars() {
        if escape {
            escape = false;
            if in_single_string {
                // In single-quoted strings, convert escaped single quotes
                if c == '\'' { result.push('\''); }
                else { result.push('\\'); result.push(c); }
            } else {
                result.push('\\'); result.push(c);
            }
            continue;
        }

        match c {
            '\\' => {
                escape = true;
                if !in_single_string { result.push(c); }
            }
            '"' => {
                if !in_single_string {
                    in_double_string = !in_double_string;
                }
                result.push(c);
            }
            '\'' => {
                if !in_double_string {
                    in_single_string = !in_single_string;
                    result.push('"'); // Replace single quote with double quote
                } else {
                    result.push(c); // Keep single quotes inside double-quoted strings
                }
            }
            _ => { result.push(c); }
        }
    }

    result
}

/// 关闭未闭合的括号/引号（处理 LLM 截断）
fn close_unclosed_brackets(text: &str) -> String {
    let mut result = text.to_string();
    let mut stack = Vec::new();
    let mut in_string = false;
    let mut escape = false;

    for c in text.chars() {
        if escape { escape = false; continue; }
        match c {
            '\\' => escape = true,
            '"' => in_string = !in_string,
            '{' if !in_string => stack.push('}'),
            '[' if !in_string => stack.push(']'),
            '}' | ']' if !in_string => { stack.pop(); }
            _ => {}
        }
    }

    // 未闭合的字符串先补引号
    if in_string {
        result.push('"');
    }

    // 关闭未闭合的括号
    while let Some(b) = stack.pop() {
        result.push(b);
    }

    result
}

fn fix_unclosed_strings(text: &str) -> String {
    let mut result = String::new();
    let mut in_string = false;
    let mut escape = false;

    for c in text.chars() {
        if escape {
            escape = false;
            result.push(c);
            continue;
        }

        match c {
            '"' => {
                in_string = !in_string;
                result.push(c);
            }
            '\\' => {
                escape = true;
                result.push(c);
            }
            '\n' if in_string => {
                result.push_str("\\n");
            }
            _ => {
                result.push(c);
            }
        }
    }

    if in_string {
        result.push('"');
    }

    result
}

/// 修复 JSON 中 } 或 ] 或 " 后直接跟换行和 { 或 [ 或 " 前缺少逗号的情况
fn fix_missing_commas(text: &str) -> String {
    let mut result = String::new();
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        let c = chars[i];
        result.push(c);

        let can_follow = c == '}' || c == ']' || c == '"';
        if can_follow && i + 1 < len {
            let mut j = i + 1;
            let mut found_newline = false;

            while j < len && (chars[j] == ' ' || chars[j] == '\t') {
                j += 1;
            }

            if j < len && chars[j] == ',' {
                j += 1;
            }

            if j < len && chars[j] == '\n' {
                found_newline = true;
                j += 1;
            } else if j < len && chars[j] == '\r' && j + 1 < len && chars[j + 1] == '\n' {
                found_newline = true;
                j += 2;
            }

            if found_newline {
                while j < len && (chars[j] == ' ' || chars[j] == '\t') {
                    j += 1;
                }

                if j < len && (chars[j] == '{' || chars[j] == '[' || chars[j] == '"') {
                    result.push(',');
                    result.push('\n');
                    i = j;
                    continue;
                }
            }
        }

        i += 1;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repair_unclosed_brace() {
        let input = r#"{"name": "test""#;
        let result = repair_json(input);
        let val: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(val["name"], "test");
    }

    #[test]
    fn test_repair_trailing_comma() {
        let input = r#"{"name": "test",}"#;
        let result = repair_json(input);
        let val: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(val["name"], "test");
    }

    #[test]
    fn test_extract_json_from_markdown() {
        let input = "```json\n{\"key\": \"value\"}\n```";
        let block = extract_json_block(input).unwrap();
        assert_eq!(block, r#"{"key": "value"}"#);
    }

    #[test]
    fn test_validate_and_repair_valid_json() {
        let result = validate_and_repair_json(r#"{"a": 1}"#).unwrap();
        assert_eq!(result["a"], 1);
    }

    #[test]
    fn test_validate_and_repair_broken_json() {
        let result = validate_and_repair_json(r#"{"a": 1"#).unwrap();
        assert_eq!(result["a"], 1);
    }
}

/// 验证并修复 JSON：返回最终可用的 JSON Value
pub fn validate_and_repair_json(text: &str) -> Result<serde_json::Value, String> {
    // 1. 直接解析
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(text) {
        return Ok(val);
    }

    // 2. 提取 JSON block
    if let Some(block) = extract_json_block(text) {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&block) {
            return Ok(val);
        }

        // 3. 修复后再试
        let repaired = repair_json(&block);
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&repaired) {
            return Ok(val);
        }
    }

    // 4. 直接修复原始文本
    let repaired = repair_json(text);
    serde_json::from_str::<serde_json::Value>(&repaired)
        .map_err(|e| format!("JSON 解析失败（已尝试修复）: {}", e))
}
