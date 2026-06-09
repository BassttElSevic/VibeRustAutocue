//! 展示模式调度
//!
//! 管理三种展示模式 (Scroll/Chunk/Focus)，计算当前应渲染的文本区域。

use autocue_core::DisplayMode;
use autocue_core::script::Chunk;
use autocue_core::scroll::ScrollState;

/// 展示模式控制器
pub struct DisplayController {
    pub mode: DisplayMode,
}

/// 计算出的可见区域
pub struct VisibleRegion {
    /// 可见行索引范围 (在 Chunk.lines 中的索引)
    pub line_range: std::ops::Range<usize>,
    /// 高亮行索引 (Focus 模式使用)
    pub highlight_line: Option<usize>,
    /// 当前所属标题文本 (.md)
    pub heading: Option<String>,
}

impl DisplayController {
    pub fn new(mode: DisplayMode) -> Self {
        Self { mode }
    }

    /// 根据模式和 ScrollState 计算可见区域
    pub fn compute_visible(
        &self,
        state: &ScrollState,
        chunks: &[Chunk],
        viewport_lines: usize,
    ) -> VisibleRegion {
        match self.mode {
            DisplayMode::Scroll => self.compute_scroll(state, chunks, viewport_lines),
            DisplayMode::Chunk => self.compute_chunk(state, chunks, viewport_lines),
            DisplayMode::Focus => self.compute_focus(state, chunks, viewport_lines),
        }
    }

    /// 匀速滚动模式：以当前 Chunk 为中心，显示 viewport_lines 行
    fn compute_scroll(
        &self,
        state: &ScrollState,
        chunks: &[Chunk],
        viewport_lines: usize,
    ) -> VisibleRegion {
        let chunk = chunks.get(state.current_index);
        let heading = chunk
            .and_then(|c| c.heading.as_ref())
            .map(|h| h.text.clone());

        // 默认显示当前 chunk 的全部行
        let line_range = if let Some(c) = chunk {
            let total = c.lines.len();
            let start = 0usize;
            let end = total.min(viewport_lines);
            start..end
        } else {
            0..0
        };

        VisibleRegion {
            line_range,
            highlight_line: None,
            heading,
        }
    }

    /// 逐块推进：显示当前 chunk 的全部行
    fn compute_chunk(
        &self,
        state: &ScrollState,
        chunks: &[Chunk],
        viewport_lines: usize,
    ) -> VisibleRegion {
        // 与 Scroll 模式相同（逐块模式在 Phase 4 会有差异）
        self.compute_scroll(state, chunks, viewport_lines)
    }

    /// 聚焦行：居中高亮当前行
    fn compute_focus(
        &self,
        state: &ScrollState,
        chunks: &[Chunk],
        viewport_lines: usize,
    ) -> VisibleRegion {
        let chunk = chunks.get(state.current_index);
        let heading = chunk
            .and_then(|c| c.heading.as_ref())
            .map(|h| h.text.clone());

        let (line_range, highlight) = if let Some(c) = chunk {
            let total = c.lines.len();
            let total_signed = total as isize;
            let vp_signed = viewport_lines as isize;

            let center = (state.line_offset * (total_signed - 1).max(0) as f32) as isize;
            let half = vp_signed / 2;
            let start = (center - half).max(0).min(total_signed - vp_signed).max(0) as usize;
            let end = (start + viewport_lines).min(total);

            (start..end, Some(center.max(0) as usize))
        } else {
            (0..0, None)
        };

        VisibleRegion {
            line_range,
            highlight_line: highlight,
            heading,
        }
    }
}
