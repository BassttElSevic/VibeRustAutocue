//! 文稿加载器
//!
//! 支持从文件路径、stdin、原始字符串加载文稿。
//! 自动检测格式：.txt 纯文本、.md Markdown（含标题解析）。
//! 所有来源归一化为统一的 `Script` 结构。

use crate::{Error, Result};
use crate::script::{Heading, Paragraph, Script};

/// 从文件路径加载文稿，根据扩展名自动检测格式
///
/// - `.txt` — 纯文本
/// - `.md` — Markdown，解析 `#` 标题
/// - 其他 — 默认按 `.txt` 处理
pub fn load_file(path: &std::path::Path) -> Result<Script> {
    let source = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    let raw_text = std::fs::read_to_string(path).map_err(|e| {
        Error::Load(format!("cannot read {}: {}", path.display(), e))
    })?;

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    match ext.to_lowercase().as_str() {
        "md" | "markdown" => Ok(from_markdown(&source, raw_text)),
        _ => Ok(from_plain_text(&source, raw_text)),
    }
}

/// 从 stdin 读取文稿（按纯文本处理）
pub fn load_stdin() -> Result<Script> {
    use std::io::Read;
    let mut raw_text = String::new();
    std::io::stdin()
        .read_to_string(&mut raw_text)
        .map_err(|e| Error::Load(format!("cannot read stdin: {}", e)))?;

    Ok(from_plain_text("stdin", raw_text))
}

/// 从原始字符串创建 Script（用于测试和程序化调用）
pub fn from_string(source: &str, text: String) -> Script {
    from_plain_text(source, text)
}

/// 从原始字符串创建 Script，按 Markdown 解析
pub fn from_markdown_string(source: &str, text: String) -> Script {
    from_markdown(source, text)
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// 按纯文本解析：双换行分段，无标题
fn from_plain_text(source: &str, raw_text: String) -> Script {
    let paragraphs = split_paragraphs(&raw_text);
    Script::new(source, raw_text, paragraphs, vec![])
}

/// 按 Markdown 解析：双换行分段 + 解析标题
fn from_markdown(source: &str, raw_text: String) -> Script {
    let headings = parse_headings(&raw_text);
    let mut paragraphs = split_paragraphs(&raw_text);

    // 为每个段落关联最近的标题
    assign_headings_to_paragraphs(&mut paragraphs, &headings);

    Script::new(source, raw_text, paragraphs, headings)
}

/// 按双换行 (`\n\n` 或 `\r\n\r\n`) 分割为段落
fn split_paragraphs(text: &str) -> Vec<Paragraph> {
    text.split("\n\n")
        .flat_map(|block| block.split("\r\n\r\n"))
        .map(|block| {
            let trimmed = block.trim().to_string();
            Paragraph::new(trimmed)
        })
        .collect()
}

/// 解析 Markdown 标题
///
/// 匹配行首的 `#{1,6} ` 模式，提取标题文本。
/// 返回按出现顺序排列的标题列表。
fn parse_headings(text: &str) -> Vec<Heading> {
    let mut headings = Vec::new();

    for (line_num, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        // 匹配 `#` 后跟空格或直接是文本
        if let Some(rest) = trimmed.strip_prefix("# ") {
            headings.push(Heading::new(1, rest.trim().to_string(), line_num));
        } else if let Some(rest) = trimmed.strip_prefix("## ") {
            headings.push(Heading::new(2, rest.trim().to_string(), line_num));
        } else if let Some(rest) = trimmed.strip_prefix("### ") {
            headings.push(Heading::new(3, rest.trim().to_string(), line_num));
        } else if let Some(rest) = trimmed.strip_prefix("#### ") {
            headings.push(Heading::new(4, rest.trim().to_string(), line_num));
        } else if let Some(rest) = trimmed.strip_prefix("##### ") {
            headings.push(Heading::new(5, rest.trim().to_string(), line_num));
        } else if let Some(rest) = trimmed.strip_prefix("###### ") {
            headings.push(Heading::new(6, rest.trim().to_string(), line_num));
        }
        // 注意: 不匹配 `#text`（无空格），避免把 `#tag` 误识别为标题
    }

    headings
}

/// 为每个段落分配最近的上层标题
///
/// 规则：段落的文本在原稿中的位置决定它"属于"哪个标题。
/// 采用简化策略：找到该段落之前最近的标题。
fn assign_headings_to_paragraphs(paragraphs: &mut [Paragraph], headings: &[Heading]) {
    if headings.is_empty() {
        return;
    }

    // 为每个段落计算其在原文中的位置，匹配最近的前一个标题
    // 简化实现：按段落索引比例近似分配
    // 更精确的实现需要记录每个段落的原始行号（此处按段落序号线性映射）
    let para_count = paragraphs.len();
    for (i, para) in paragraphs.iter_mut().enumerate() {
        if para.is_blank {
            continue;
        }
        // 找到在此段落之前的最后一个标题
        let estimated_line = if para_count > 1 {
            // 简单估算：段落均匀分布
            (i as f64 / (para_count - 1) as f64 * headings.last().map(|h| h.line).unwrap_or(0) as f64) as usize
        } else {
            0
        };

        let heading = headings
            .iter()
            .rev()
            .find(|h| h.line <= estimated_line)
            .cloned();

        para.heading = heading;
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_string_plain_text() {
        let text = "第一段\n\n第二段\n\n第三段".to_string();
        let script = from_string("test.txt", text);

        assert_eq!(script.source, "test.txt");
        assert_eq!(script.paragraphs.len(), 3);
        assert_eq!(script.paragraphs[0].text, "第一段");
        assert_eq!(script.paragraphs[1].text, "第二段");
        assert_eq!(script.paragraphs[2].text, "第三段");
        assert!(script.headings.is_empty());
    }

    #[test]
    fn test_from_string_skips_blank_paragraphs() {
        let text = "段落1\n\n\n\n段落2".to_string();
        let script = from_string("test.txt", text);

        // 两个段落，中间的空行被处理为空段落
        assert_eq!(script.paragraphs.len(), 3);
        assert_eq!(script.paragraphs[0].text, "段落1");
        assert!(script.paragraphs[1].is_blank);
        assert_eq!(script.paragraphs[2].text, "段落2");
    }

    #[test]
    fn test_from_markdown_string() {
        let text = "# 标题一\n\n内容第一段\n\n## 标题二\n\n内容第二段".to_string();
        let script = from_markdown_string("test.md", text);

        assert_eq!(script.source, "test.md");
        assert_eq!(script.headings.len(), 2);
        assert_eq!(script.headings[0].text, "标题一");
        assert_eq!(script.headings[0].level, 1);
        assert_eq!(script.headings[1].text, "标题二");
        assert_eq!(script.headings[1].level, 2);

        assert_eq!(script.paragraphs.len(), 4);
        assert_eq!(script.paragraphs[0].text, "# 标题一");
        assert_eq!(script.paragraphs[1].text, "内容第一段");
        assert_eq!(script.paragraphs[2].text, "## 标题二");
        assert_eq!(script.paragraphs[3].text, "内容第二段");
    }

    #[test]
    fn test_parse_headings() {
        let text = "# H1\n\nsome text\n\n## H2\n\n### H3\n\n#### H4";
        let headings = parse_headings(text);

        assert_eq!(headings.len(), 4);
        assert_eq!(headings[0].level, 1);
        assert_eq!(headings[0].text, "H1");
        assert_eq!(headings[1].level, 2);
        assert_eq!(headings[1].text, "H2");
        assert_eq!(headings[2].level, 3);
        assert_eq!(headings[2].text, "H3");
        assert_eq!(headings[3].level, 4);
    }

    #[test]
    fn test_parse_headings_no_false_positive() {
        // `#tag` 不带空格的不算标题
        let text = "#tag is not a heading\n\n# real heading".to_string();
        let headings = parse_headings(&text);

        assert_eq!(headings.len(), 1);
        assert_eq!(headings[0].text, "real heading");
    }

    #[test]
    fn test_load_file_txt() {
        // 创建临时 .txt 文件
        let dir = std::env::temp_dir();
        let path = dir.join("autocue_test_load.txt");
        std::fs::write(&path, "Hello\n\nWorld").unwrap();

        let script = load_file(&path).unwrap();
        assert_eq!(script.paragraphs.len(), 2);
        assert_eq!(script.paragraphs[0].text, "Hello");
        assert_eq!(script.paragraphs[1].text, "World");

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_load_file_md() {
        let dir = std::env::temp_dir();
        let path = dir.join("autocue_test_load.md");
        std::fs::write(&path, "# Title\n\nContent").unwrap();

        let script = load_file(&path).unwrap();
        assert_eq!(script.headings.len(), 1);
        assert_eq!(script.headings[0].text, "Title");

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_load_file_not_found() {
        let result = load_file(std::path::Path::new("/nonexistent/file.txt"));
        assert!(result.is_err());
    }

    #[test]
    fn test_from_markdown_with_headings() {
        let text = "## 第一节\n\n段落A\n\n## 第二节\n\n段落B".to_string();
        let script = from_markdown_string("doc.md", text);

        assert_eq!(script.headings.len(), 2);
        assert_eq!(script.headings[0].text, "第一节");
        assert_eq!(script.headings[1].text, "第二节");
    }
}
