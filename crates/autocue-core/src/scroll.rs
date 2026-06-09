//! 滚动状态机
//!
//! 管理文稿的滚动位置：当前 Chunk 索引、行内偏移、全局进度。
//! 支持匀速滚动、跳跃、速度调节。

/// 滚动状态
#[derive(Debug, Clone)]
pub struct ScrollState {
    /// 当前所在 Chunk 索引
    pub current_index: usize,
    /// 当前 Chunk 内的行偏移 (0.0 = 第一行, 用于平滑滚动)
    pub line_offset: f32,
    /// 总 Chunk 数
    pub total_chunks: usize,
    /// 累计已消耗字符数
    chars_consumed: usize,
    /// 所有 Chunk 的字符数 (缓存)
    chunk_char_counts: Vec<usize>,
}

impl ScrollState {
    /// 创建滚动状态
    ///
    /// `chunk_char_counts` 是各 Chunk 的字符数列表
    pub fn new(chunk_char_counts: Vec<usize>) -> Self {
        let total_chunks = chunk_char_counts.len();
        Self {
            current_index: 0,
            line_offset: 0.0,
            total_chunks,
            chars_consumed: 0,
            chunk_char_counts,
        }
    }

    /// 向前滚动 delta 秒
    ///
    /// `speed` — 当前速度 (字/秒)
    pub fn advance(&mut self, delta: f64, speed: f64) {
        if self.total_chunks == 0 || speed <= 0.0 || delta <= 0.0 {
            return;
        }

        let chars_to_advance = (delta * speed) as usize;
        if chars_to_advance == 0 {
            return;
        }

        let mut remaining = chars_to_advance;

        while remaining > 0 && self.current_index < self.total_chunks {
            let current_chunk_chars = self
                .chunk_char_counts
                .get(self.current_index)
                .copied()
                .unwrap_or(1)
                .max(1);

            let chars_left_in_chunk = current_chunk_chars.saturating_sub(
                (self.line_offset * current_chunk_chars as f32) as usize,
            );

            if remaining >= chars_left_in_chunk {
                // 跳过当前 Chunk 剩余部分
                remaining -= chars_left_in_chunk;
                self.current_index += 1;
                self.line_offset = 0.0;
                self.chars_consumed += chars_left_in_chunk;
            } else {
                // 在当前 Chunk 内前进
                self.line_offset += remaining as f32 / current_chunk_chars as f32;
                self.chars_consumed += remaining;
                remaining = 0;
            }
        }

        // 到达末尾后不再前进
        if self.current_index >= self.total_chunks {
            self.current_index = self.total_chunks.saturating_sub(1);
            self.line_offset = 1.0;
        }
    }

    /// 跳转到指定 Chunk
    pub fn seek_to(&mut self, index: usize) {
        self.current_index = index.min(self.total_chunks.saturating_sub(1));
        self.line_offset = 0.0;
        // 重新计算 chars_consumed
        self.chars_consumed = self.chunk_char_counts
            .iter()
            .take(self.current_index)
            .sum();
    }

    /// 前进 n 个 Chunk（用于 SeekForward）
    pub fn seek_forward(&mut self, n: usize) {
        let target = self.current_index.saturating_add(n);
        self.seek_to(target);
    }

    /// 后退 n 个 Chunk（用于 SeekBackward）
    pub fn seek_backward(&mut self, n: usize) {
        let target = self.current_index.saturating_sub(n);
        self.seek_to(target);
    }

    /// 全局进度 0.0 ~ 1.0
    pub fn progress(&self) -> f32 {
        if self.chunk_char_counts.is_empty() {
            return 0.0;
        }
        let total: usize = self.chunk_char_counts.iter().sum();
        if total == 0 {
            return 0.0;
        }
        (self.chars_consumed as f32 / total as f32).clamp(0.0, 1.0)
    }

    /// 是否已播完
    pub fn is_finished(&self) -> bool {
        if self.total_chunks == 0 {
            return true;
        }
        self.current_index >= self.total_chunks.saturating_sub(1)
            && self.line_offset >= 1.0
    }

    /// 总字符数
    pub fn total_chars(&self) -> usize {
        self.chunk_char_counts.iter().sum()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_state() -> ScrollState {
        // 3 个 Chunk: 100 chars, 200 chars, 150 chars
        ScrollState::new(vec![100, 200, 150])
    }

    #[test]
    fn test_new_state() {
        let state = make_state();
        assert_eq!(state.current_index, 0);
        assert_eq!(state.total_chunks, 3);
        assert_eq!(state.total_chars(), 450);
        assert!((state.progress() - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_advance_within_chunk() {
        let mut state = make_state();
        // 速度 10 字/秒, delta 5 秒 = 50 字, 在第一个 Chunk (100 字) 内
        state.advance(5.0, 10.0);
        assert_eq!(state.current_index, 0);
        assert!(state.line_offset > 0.0);
        assert!(state.line_offset < 1.0);
    }

    #[test]
    fn test_advance_cross_chunk() {
        let mut state = make_state();
        // 速度 10 字/秒, delta 11 秒 = 110 字
        // 第一个 Chunk 100 字 → 跳到第二个
        state.advance(11.0, 10.0);
        assert_eq!(state.current_index, 1);
    }

    #[test]
    fn test_advance_to_end() {
        let mut state = make_state();
        // 速度 100 字/秒, delta 10 秒 = 1000 字, 总共 450 字
        state.advance(10.0, 100.0);
        assert!(state.is_finished());
        assert!((state.progress() - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_seek_to() {
        let mut state = make_state();
        state.seek_to(1);
        assert_eq!(state.current_index, 1);
        assert_eq!(state.line_offset, 0.0);
    }

    #[test]
    fn test_seek_forward_backward() {
        let mut state = make_state();
        state.seek_forward(2);
        assert_eq!(state.current_index, 2);

        state.seek_backward(1);
        assert_eq!(state.current_index, 1);
    }

    #[test]
    fn test_progress() {
        let mut state = make_state();
        // 450 总字符, 250 已消耗 → 250/450 ≈ 0.556
        state.advance(25.0, 10.0); // 250 chars
        let p = state.progress();
        assert!(p > 0.5 && p < 0.6);
    }

    #[test]
    fn test_empty_state() {
        let state = ScrollState::new(vec![]);
        assert_eq!(state.total_chunks, 0);
        assert_eq!(state.progress(), 0.0);
        assert!(state.is_finished());
    }
}
