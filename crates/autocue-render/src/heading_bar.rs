//! .md 标题栏叠加层
//!
//! 在屏幕顶部渲染当前章节的 Markdown 标题。

/// 标题叠加条
pub struct HeadingBar {
    pub visible: bool,
    pub text: String,
    /// 相对于正文字号的比例 (0.6 = 60%)
    pub font_size_ratio: f32,
}

impl HeadingBar {
    pub fn new(font_size_ratio: f32) -> Self {
        Self {
            visible: false,
            text: String::new(),
            font_size_ratio,
        }
    }

    /// 更新当前标题
    pub fn update(&mut self, heading: Option<&str>) {
        match heading {
            Some(h) if !h.is_empty() => {
                self.text = h.to_string();
                self.visible = true;
            }
            _ => {
                self.visible = false;
                self.text.clear();
            }
        }
    }

    /// 标题叠加条占用的行数（用于偏移正文区域）
    pub fn line_height(&self, font_size: f32) -> f32 {
        if self.visible {
            font_size * self.font_size_ratio * 1.5
        } else {
            0.0
        }
    }
}
