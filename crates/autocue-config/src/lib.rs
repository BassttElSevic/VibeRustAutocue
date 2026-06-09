//! autocue-config — 配置层
//!
//! 读取和解析 `autocue.toml`，提供主题预设和默认值。

pub mod defaults;
pub mod theme;

/// Configuration error type.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to read config file: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to parse config: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("invalid config value: {0}")]
    Invalid(String),
}

/// Result alias for config operations.
pub type Result<T> = std::result::Result<T, Error>;
