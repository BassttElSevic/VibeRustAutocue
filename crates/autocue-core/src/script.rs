//! 脚本数据结构
//!
//! 定义文稿的完整数据模型：
//! - `DisplayMode` — 展示模式枚举
//! - `Heading` — Markdown 标题节点
//! - `Paragraph` — 文稿段落
//! - `Chunk` — 适合屏幕展示的文本块
//! - `Script` — 解析后的完整文稿
//! - `Marker` — 书签/标记点

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// DisplayMode
// ---------------------------------------------------------------------------

/// 展示模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DisplayMode {
    /// 匀速滚动 — 文本自动向上流动
    Scroll,
    /// 逐块推进 — 一次一屏，手动切块
    Chunk,
    /// 聚焦行 — 只高亮当前句
    Focus,
}

impl DisplayMode {
    /// 从字符串解析
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "scroll" => Some(Self::Scroll),
            "chunk" => Some(Self::Chunk),
            "focus" => Some(Self::Focus),
            _ => None,
        }
    }

    /// 转为配置字符串
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Scroll => "scroll",
            Self::Chunk => "chunk",
            Self::Focus => "focus",
        }
    }
}

// ---------------------------------------------------------------------------
// Heading
// ---------------------------------------------------------------------------

/// Markdown 标题节点
#[derive(Debug, Clone, PartialEq)]
pub struct Heading {
    /// 层级: 1 = `#`, 2 = `##`, 3 = `###`
    pub level: u8,
    /// 标题文本（不含 `#` 前缀和首尾空格）
    pub text: String,
    /// 在原稿中的起始行号 (0-based)
    pub line: usize,
}

impl Heading {
    pub fn new(level: u8, text: String, line: usize) -> Self {
        Self { level, text, line }
    }
}

// ---------------------------------------------------------------------------
// Paragraph
// ---------------------------------------------------------------------------

/// 文稿中的一个段落
#[derive(Debug, Clone, PartialEq)]
pub struct Paragraph {
    /// 段落文本（已去除首尾空白）
    pub text: String,
    /// 是否为空行
    pub is_blank: bool,
    /// 关联的标题（最内层匹配的 `##`），无标题则为 None
    pub heading: Option<Heading>,
}

impl Paragraph {
    pub fn new(text: String) -> Self {
        let is_blank = text.trim().is_empty();
        Self {
            text,
            is_blank,
            heading: None,
        }
    }

    pub fn with_heading(text: String, heading: Heading) -> Self {
        let is_blank = text.trim().is_empty();
        Self {
            text,
            is_blank,
            heading: Some(heading),
        }
    }
}

// ---------------------------------------------------------------------------
// Chunk
// ---------------------------------------------------------------------------

/// 分词后适合屏幕显示的"块"
#[derive(Debug, Clone, PartialEq)]
pub struct Chunk {
    /// 块包含的行（已按屏幕宽度断行）
    pub lines: Vec<String>,
    /// 预估朗读时长 (秒)，基于字数 ÷ 速度
    pub duration_estimate: f64,
    /// 所属标题
    pub heading: Option<Heading>,
    /// 在原稿中的起始字符偏移
    pub start_offset: usize,
    /// 本块总字符数（不含空白）
    pub char_count: usize,
}

impl Chunk {
    pub fn new(
        lines: Vec<String>,
        speed: f64,
        heading: Option<Heading>,
        start_offset: usize,
    ) -> Self {
        let char_count: usize = lines.iter().map(|l| l.chars().count()).sum();
        let duration_estimate = if speed > 0.0 {
            char_count as f64 / speed
        } else {
            f64::INFINITY
        };
        Self {
            lines,
            duration_estimate,
            heading,
            start_offset,
            char_count,
        }
    }
}

// ---------------------------------------------------------------------------
// Script
// ---------------------------------------------------------------------------

/// 解析后的完整文稿
#[derive(Debug, Clone, PartialEq)]
pub struct Script {
    /// 文件路径或来源标识
    pub source: String,
    /// 原始全文
    pub raw_text: String,
    /// 段落列表
    pub paragraphs: Vec<Paragraph>,
    /// 标题层级结构（按出现顺序）
    pub headings: Vec<Heading>,
    /// 总字符数（不含空白和 Markdown 标记）
    pub total_chars: usize,
    /// 预估总时长 (秒)
    pub estimated_duration: f64,
}

impl Script {
    /// 从段落和标题创建 Script
    pub fn new(
        source: impl Into<String>,
        raw_text: String,
        paragraphs: Vec<Paragraph>,
        headings: Vec<Heading>,
    ) -> Self {
        let total_chars: usize = paragraphs
            .iter()
            .filter(|p| !p.is_blank)
            .map(|p| p.text.chars().count())
            .sum();
        // 按默认速度 5 字/秒估算
        let estimated_duration = total_chars as f64 / 5.0;
        Self {
            source: source.into(),
            raw_text,
            paragraphs,
            headings,
            total_chars,
            estimated_duration,
        }
    }

    /// 获取非空段落数量
    pub fn non_blank_paragraph_count(&self) -> usize {
        self.paragraphs.iter().filter(|p| !p.is_blank).count()
    }
}

// ---------------------------------------------------------------------------
// Marker
// ---------------------------------------------------------------------------

/// 书签 / 标记点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Marker {
    /// 书签名称
    pub name: String,
    /// 在文稿中的字符偏移
    pub offset: usize,
}

impl Marker {
    pub fn new(name: String, offset: usize) -> Self {
        Self { name, offset }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_mode_from_str() {
        assert_eq!(DisplayMode::from_str("scroll"), Some(DisplayMode::Scroll));
        assert_eq!(DisplayMode::from_str("CHUNK"), Some(DisplayMode::Chunk));
        assert_eq!(DisplayMode::from_str("Focus"), Some(DisplayMode::Focus));
        assert_eq!(DisplayMode::from_str("invalid"), None);
    }

    #[test]
    fn test_display_mode_as_str() {
        assert_eq!(DisplayMode::Scroll.as_str(), "scroll");
        assert_eq!(DisplayMode::Chunk.as_str(), "chunk");
        assert_eq!(DisplayMode::Focus.as_str(), "focus");
    }

    #[test]
    fn test_heading_new() {
        let h = Heading::new(2, "Introduction".into(), 5);
        assert_eq!(h.level, 2);
        assert_eq!(h.text, "Introduction");
        assert_eq!(h.line, 5);
    }

    #[test]
    fn test_paragraph_new() {
        let p = Paragraph::new("Hello world".into());
        assert_eq!(p.text, "Hello world");
        assert!(!p.is_blank);
        assert!(p.heading.is_none());
    }

    #[test]
    fn test_paragraph_blank() {
        let p = Paragraph::new("   ".into());
        assert!(p.is_blank);
    }

    #[test]
    fn test_paragraph_with_heading() {
        let h = Heading::new(1, "Title".into(), 0);
        let p = Paragraph::with_heading("Text".into(), h);
        assert_eq!(p.heading.unwrap().text, "Title");
    }

    #[test]
    fn test_chunk_new() {
        let lines = vec!["Hello world".into(), "第二行".into()];
        let h = Heading::new(2, "Section".into(), 0);
        let chunk = Chunk::new(lines.clone(), 5.0, Some(h), 100);

        assert_eq!(chunk.lines, lines);
        assert_eq!(chunk.char_count, 14); // "Hello world"(11) + "第二行"(3)
        assert!(chunk.duration_estimate > 0.0);
        assert_eq!(chunk.start_offset, 100);
        assert_eq!(chunk.heading.unwrap().text, "Section");
    }

    #[test]
    fn test_chunk_zero_speed() {
        let chunk = Chunk::new(vec!["text".into()], 0.0, None, 0);
        assert!(chunk.duration_estimate.is_infinite());
    }

    #[test]
    fn test_script_new() {
        let paragraphs = vec![
            Paragraph::new("第一段".into()),
            Paragraph::new("第二段".into()),
        ];
        let headings = vec![Heading::new(1, "Title".into(), 0)];
        let script = Script::new(
            "test.md",
            "# Title\n\n第一段\n\n第二段".into(),
            paragraphs,
            headings,
        );

        assert_eq!(script.source, "test.md");
        assert_eq!(script.total_chars, 6); // 3 + 3
        assert!(script.estimated_duration > 0.0);
        assert_eq!(script.non_blank_paragraph_count(), 2);
    }

    #[test]
    fn test_marker_new() {
        let m = Marker::new("bookmark1".into(), 42);
        assert_eq!(m.name, "bookmark1");
        assert_eq!(m.offset, 42);
    }
}
