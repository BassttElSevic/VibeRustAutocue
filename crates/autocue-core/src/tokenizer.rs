//! 分词 / 分块器
//!
//! 将 Script 按视口尺寸切分为适合屏幕显示的 Chunk 列表。
//! 使用启发式算法估算字符宽度（不依赖实际字体度量）。

use crate::script::{Chunk, Heading, Script};
use crate::Result;

/// 分词器
pub struct Tokenizer {
    /// 视口宽度 (像素)
    viewport_width: f32,
    /// 字号 (pt)
    font_size: f32,
    /// 行间距倍数
    line_spacing: f32,
    /// 视口高度 (像素)
    viewport_height: f32,
}

impl Tokenizer {
    /// 创建分词器
    pub fn new(viewport_width: f32, viewport_height: f32, font_size: f32) -> Self {
        Self {
            viewport_width,
            font_size,
            line_spacing: 1.5,
            viewport_height,
        }
    }

    /// 创建带行间距的分词器
    pub fn with_line_spacing(
        viewport_width: f32,
        viewport_height: f32,
        font_size: f32,
        line_spacing: f32,
    ) -> Self {
        Self {
            viewport_width,
            font_size,
            line_spacing: line_spacing.max(1.0),
            viewport_height,
        }
    }

    /// 估算每行最大字符数
    fn chars_per_line(&self) -> usize {
        // 假设每个字符平均宽度约为 font_size * 0.6 像素（CJK 字符约 1.0，英文约 0.5）
        let char_width = self.font_size * 0.6;
        if char_width <= 0.0 {
            return 80; // 安全默认值
        }
        (self.viewport_width / char_width).max(1.0) as usize
    }

    /// 估算视口能容纳的行数
    fn lines_per_viewport(&self) -> usize {
        let line_height = self.font_size * self.line_spacing;
        if line_height <= 0.0 {
            return 15;
        }
        (self.viewport_height / line_height).max(1.0) as usize
    }

    /// 将 Script 切分为 Chunk 列表
    ///
    /// 每个 Chunk 包含不超过 `lines_per_viewport` 行文本。
    /// 长段落会先在 `chars_per_line` 处换行。
    pub fn tokenize(&self, script: &Script) -> Result<Vec<Chunk>> {
        let cpl = self.chars_per_line();
        let lpv = self.lines_per_viewport();

        // 第一阶段：将所有段落按字符宽度断行
        let mut all_lines: Vec<String> = Vec::new();
        let mut line_headings: Vec<Option<Heading>> = Vec::new();
        let mut line_offsets: Vec<usize> = Vec::new();
        let mut char_offset = 0usize;

        for para in &script.paragraphs {
            if para.is_blank {
                // 空段落变成空行
                all_lines.push(String::new());
                line_headings.push(para.heading.clone());
                line_offsets.push(char_offset);
                continue;
            }

            let wrapped = wrap_text(&para.text, cpl);
            for line in wrapped {
                all_lines.push(line);
                line_headings.push(para.heading.clone());
                line_offsets.push(char_offset);
            }
            char_offset += para.text.chars().count();
        }

        // 如果所有行都是空的，返回空列表
        if all_lines.is_empty() || all_lines.iter().all(|l| l.is_empty()) {
            return Ok(vec![]);
        }

        // 第二阶段：将行分组为 Chunk
        let mut chunks: Vec<Chunk> = Vec::new();
        let speed = 5.0; // 默认速度，用于 duration_estimate

        let mut i = 0;
        while i < all_lines.len() {
            let end = (i + lpv).min(all_lines.len());
            let chunk_lines: Vec<String> = all_lines[i..end].to_vec();
            // 使用该 Chunk 中第一行的标题
            let heading = line_headings[i].clone();
            let start_offset = line_offsets[i];

            chunks.push(Chunk::new(chunk_lines, speed, heading, start_offset));
            i = end;
        }

        Ok(chunks)
    }

    /// 更新视口参数并重新分词（可以在窗口 resize 或字体变化时调用）
    pub fn relayout(
        &mut self,
        viewport_width: f32,
        viewport_height: f32,
        font_size: f32,
    ) {
        self.viewport_width = viewport_width;
        self.viewport_height = viewport_height;
        self.font_size = font_size;
    }
}

/// 将文本按最大宽度断行
///
/// - CJK 字符（中/日/韩）：可在任何字符后断行
/// - 英文/数字：在空格处断行，长单词强制截断
fn wrap_text(text: &str, max_chars: usize) -> Vec<String> {
    if max_chars == 0 {
        return vec![text.to_string()];
    }

    let mut lines = Vec::new();
    let mut current_line = String::with_capacity(max_chars);
    let mut current_width = 0usize;

    for ch in text.chars() {
        let char_width = if is_cjk(ch) { 2 } else { 1 };

        if current_width + char_width > max_chars {
            // 当前行已满
            if !current_line.is_empty() {
                lines.push(current_line.clone());
                current_line.clear();
                current_width = 0;
            }
        }

        current_line.push(ch);
        current_width += char_width;
    }

    // 最后一行
    if !current_line.is_empty() {
        lines.push(current_line);
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}

/// 判断字符是否为 CJK（中文/日文/韩文）
fn is_cjk(ch: char) -> bool {
    matches!(
        ch,
        '\u{4E00}'..='\u{9FFF}'   // CJK 统一表意文字
        | '\u{3400}'..='\u{4DBF}' // CJK 扩展 A
        | '\u{F900}'..='\u{FAFF}' // CJK 兼容表意文字
        | '\u{3040}'..='\u{309F}' // 平假名
        | '\u{30A0}'..='\u{30FF}' // 片假名
        | '\u{AC00}'..='\u{D7AF}' // 韩文
        | '\u{FF01}'..='\u{FF60}' // 全角标点
        | '\u{FFE0}'..='\u{FFE6}' // 全角符号
        | '\u{3000}'..='\u{303F}' // CJK 标点
    )
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loader;

    #[test]
    fn test_is_cjk() {
        assert!(is_cjk('中'));
        assert!(is_cjk('文'));
        assert!(is_cjk('日'));
        assert!(is_cjk('あ')); // 平假名
        assert!(!is_cjk('A'));
        assert!(!is_cjk('1'));
        assert!(!is_cjk(' '));
    }

    #[test]
    fn test_wrap_text_short() {
        let result = wrap_text("Hello", 80);
        assert_eq!(result, vec!["Hello"]);
    }

    #[test]
    fn test_wrap_text_cjk() {
        // "这是一段中文测试文本" = 9 个字
        let result = wrap_text("这是一段中文测试文", 10);
        // 每个中文字宽度为 2，所以 10 宽度 = 5 个汉字/行
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].chars().count(), 5);
        assert_eq!(result[1].chars().count(), 4);
    }

    #[test]
    fn test_wrap_text_mixed() {
        let result = wrap_text("Hello世界Test测试", 8);
        // H(1)e(1)l(1)l(1)o(1) = 5, 世(2) = 7, 界(2) = 9 > 8 → 换行
        assert!(result.len() >= 2);
    }

    #[test]
    fn test_wrap_text_empty() {
        let result = wrap_text("", 40);
        assert_eq!(result, vec![""]);
    }

    #[test]
    fn test_tokenizer_basic() {
        let script = loader::from_string("test", "第一段文字内容\n\n第二段内容在这里".into());
        let tokenizer = Tokenizer::new(800.0, 600.0, 48.0);
        let chunks = tokenizer.tokenize(&script).unwrap();

        assert!(!chunks.is_empty());
        // 应该有 chunk 产出
        for chunk in &chunks {
            assert!(!chunk.lines.is_empty());
            assert!(chunk.char_count > 0 || chunk.lines.iter().all(|l| l.is_empty()));
        }
    }

    #[test]
    fn test_tokenizer_empty_script() {
        let script = loader::from_string("test", "".into());
        let tokenizer = Tokenizer::new(800.0, 600.0, 48.0);
        let chunks = tokenizer.tokenize(&script).unwrap();
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_relayout() {
        let mut tokenizer = Tokenizer::new(800.0, 600.0, 48.0);
        tokenizer.relayout(1024.0, 768.0, 56.0);

        assert_eq!(tokenizer.viewport_width, 1024.0);
        assert_eq!(tokenizer.viewport_height, 768.0);
        assert_eq!(tokenizer.font_size, 56.0);
    }

    #[test]
    fn test_tokenizer_preserves_headings() {
        let script = loader::from_markdown_string(
            "test.md",
            "## 第一节\n\n段落A的内容在这里\n\n## 第二节\n\n段落B的内容".into(),
        );
        let tokenizer = Tokenizer::new(800.0, 600.0, 48.0);
        let chunks = tokenizer.tokenize(&script).unwrap();

        // 应有至少两个 chunk，各自带不同标题
        let headings: Vec<_> = chunks
            .iter()
            .filter_map(|c| c.heading.as_ref().map(|h| h.text.clone()))
            .collect();

        // 第一个非空标题应该是 "第一节"
        assert!(headings.iter().any(|h| h == "第一节"));
    }

    #[test]
    fn test_with_line_spacing() {
        let tokenizer = Tokenizer::with_line_spacing(800.0, 600.0, 48.0, 2.0);
        assert_eq!(tokenizer.line_spacing, 2.0);
    }
}
