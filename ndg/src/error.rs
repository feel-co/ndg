use std::{fmt, io};

use thiserror::Error;

/// Top-level error type for the ndg crate.
#[derive(Debug, Error)]
#[allow(dead_code)]
pub enum NdgError {
  #[error("Configuration error: {0}")]
  Config(String),

  #[error("Template error: {0}")]
  Template(String),

  #[error("I/O error: {0}")]
  Io(#[from] io::Error),

  #[allow(dead_code)]
  #[error("Rendering error: {0}")]
  Render(String),

  #[error("External library error: {0}")]
  External(String),

  #[error("Serde error: {0}")]
  Serde(#[from] serde_json::Error),

  #[error("TOML error: {0}")]
  Toml(#[from] toml::de::Error),

  #[error("Regex error: {0}")]
  Regex(#[from] regex::Error),

  #[allow(dead_code)]
  #[error("Unknown error: {0}")]
  Unknown(String),
}

impl From<tera::Error> for NdgError {
  fn from(e: tera::Error) -> Self {
    Self::Template(e.to_string())
  }
}

impl From<fs_extra::error::Error> for NdgError {
  fn from(e: fs_extra::error::Error) -> Self {
    Self::Io(io::Error::other(e.to_string()))
  }
}

impl From<std::string::FromUtf8Error> for NdgError {
  fn from(e: std::string::FromUtf8Error) -> Self {
    Self::External(e.to_string())
  }
}

impl From<std::num::ParseIntError> for NdgError {
  fn from(e: std::num::ParseIntError) -> Self {
    Self::Config(e.to_string())
  }
}

impl From<std::fmt::Error> for NdgError {
  fn from(e: fmt::Error) -> Self {
    Self::External(e.to_string())
  }
}
