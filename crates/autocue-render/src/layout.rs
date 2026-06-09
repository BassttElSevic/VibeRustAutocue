//! 文本排版引擎 — 基于 cosmic-text 0.13

use cosmic_text::{Attrs, Buffer, FontSystem, Metrics, Shaping, Wrap};

pub struct LayoutEngine {
    pub font_system: FontSystem,
    pub buffer: Buffer,
    font_size: f32,
}

impl LayoutEngine {
    pub fn new(font_size: f32) -> Self {
        let mut font_system = FontSystem::new();
        let mut buffer = Buffer::new(&mut font_system, Metrics::new(font_size, font_size * 1.5));

        buffer.set_wrap(&mut font_system, Wrap::Word);

        Self {
            font_system,
            buffer,
            font_size,
        }
    }

    /// 设置文本内容并重新布局
    pub fn set_text(&mut self, text: &str) {
        self.buffer.set_text(
            &mut self.font_system,
            text,
            Attrs::new(),
            Shaping::Advanced,
        );
    }

    /// 设置字体大小
    pub fn set_font_size(&mut self, font_size: f32) {
        self.font_size = font_size;
        self.buffer
            .set_size(&mut self.font_system, Some(font_size), Some(font_size * 1.5));
    }

    pub fn font_size(&self) -> f32 {
        self.font_size
    }

    pub fn line_count(&self) -> usize {
        self.buffer.lines.len()
    }

    pub fn needs_redraw(&self) -> bool {
        self.buffer.redraw()
    }
}
