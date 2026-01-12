use std::io;

use thiserror::Error;

/// Error type for ndg-config operations
#[derive(Debug, Error)]
pub enum ConfigError {
  #[error("Configuration error: {0}")]
  Config(String),

  #[error("Template error: {0}")]
  Template(String),

  #[error("I/O error: {0}")]
  Io(#[from] io::Error),

  #[error("Serde error: {0}")]
  Serde(#[from] serde_json::Error),

  #[error("TOML error: {0}")]
  Toml(#[from] toml::de::Error),
}
