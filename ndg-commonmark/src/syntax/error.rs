//! Error types for syntax highlighting operations.

/// Result type for syntax highlighting operations.
pub type SyntaxResult<T> = Result<T, SyntaxError>;

/// Errors that can occur during syntax highlighting.
#[derive(Debug, thiserror::Error)]
pub enum SyntaxError {
  #[error("Language '{0}' is not supported by this highlighter")]
  UnsupportedLanguage(String),
  #[error("Theme '{0}' is not available")]
  ThemeNotFound(String),
  #[error("Highlighting failed: {0}")]
  HighlightingFailed(String),
  #[error("Backend initialization failed: {0}")]
  BackendError(String),
  #[error(
    "Cannot enable both 'syntastica' and 'syntect' features simultaneously. \
     They are mutually exclusive."
  )]
  MutuallyExclusiveBackends,
  #[error(
    "No syntax highlighting backend available. Enable either 'syntastica' or \
     'syntect' feature."
  )]
  NoBackendAvailable,
}
