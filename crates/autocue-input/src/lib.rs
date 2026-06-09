//! autocue-input — 输入控制
//!
//! 键盘快捷键映射和远程控制 (WebSocket) 支持。

pub mod keyboard;
pub mod remote;

/// Input error type.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("keyboard error: {0}")]
    Keyboard(String),
    #[error("remote control error: {0}")]
    Remote(String),
}

/// Result alias for input operations.
pub type Result<T> = std::result::Result<T, Error>;
