use std::{io, path::PathBuf};

use thiserror::Error;

/// Error type for ndg-config operations
#[derive(Debug, Error)]
pub enum ConfigError {
  #[error("Configuration error: {0}")]
  Config(String),

  #[error("Failed to read configuration file '{}': {source}", path.display())]
  Read {
    /// Configuration file that could not be read.
    path: PathBuf,

    /// Underlying file-system error.
    #[source]
    source: io::Error,
  },

  #[error("Failed to parse JSON configuration from '{}': {source}", path.display())]
  Json {
    /// Configuration file that could not be parsed.
    path: PathBuf,

    /// Parse error with the nested configuration field path.
    #[source]
    source: serde_path_to_error::Error<serde_json::Error>,
  },

  #[error("Failed to parse JSON configuration from '{}': {source}", path.display())]
  JsonSyntax {
    /// Configuration file that contains invalid JSON syntax.
    path: PathBuf,

    /// Underlying JSON syntax error.
    #[source]
    source: serde_json::Error,
  },

  #[error("Failed to parse TOML configuration from '{}': {source}", path.display())]
  TomlFile {
    /// Configuration file that could not be parsed.
    path: PathBuf,

    /// TOML parse error, including source span information.
    #[source]
    source: toml::de::Error,
  },

  #[error("Invalid configuration from {origin}: {message}")]
  Validation {
    /// File or command-line layer that introduced the invalid value.
    origin: String,

    /// Validation failure.
    message: String,
  },

  #[error("Invalid `--config` override '{value}': {source}")]
  Override {
    /// Full `KEY=VALUE` argument that failed.
    value: String,

    /// Field-specific override error.
    #[source]
    source: Box<Self>,
  },

  #[error("Unsupported configuration file format: '{}'", path.display())]
  UnsupportedFormat {
    /// Configuration path with an unsupported extension.
    path: PathBuf,
  },

  #[error("Configuration file has no extension: '{}'", path.display())]
  MissingExtension {
    /// Configuration path without an extension.
    path: PathBuf,
  },

  #[error("Template error: {0}")]
  Template(String),

  #[error("I/O error: {0}")]
  Io(#[from] io::Error),

  #[error("Serde error: {0}")]
  Serde(#[from] serde_json::Error),

  #[error("TOML error: {0}")]
  Toml(#[from] toml::de::Error),
}
