//! autocue-render — 渲染层
//!
//! 负责窗口创建、文本排版、GPU 绘制和展示模式调度。

pub mod display;
pub mod heading_bar;
pub mod layout;
pub mod mirror;
pub mod painter;
pub mod window;

/// Render error type.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("window error: {0}")]
    Window(String),
    #[error("rendering error: {0}")]
    Render(String),
}

/// Result alias for render operations.
pub type Result<T> = std::result::Result<T, Error>;
