//! autocue-core — 核心引擎
//!
//! 负责文稿加载、分词分块、滚动状态管理。
//! 本 crate 保持无 GUI / 无平台依赖，可独立测试。

pub mod loader;
pub mod marker;
pub mod script;
pub mod scroll;
pub mod tokenizer;

/// Core error type used across the engine.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to load script: {0}")]
    Load(String),
    #[error("tokenization error: {0}")]
    Tokenize(String),
    #[error("scroll error: {0}")]
    Scroll(String),
}

/// Result alias for core operations.
pub type Result<T> = std::result::Result<T, Error>;
