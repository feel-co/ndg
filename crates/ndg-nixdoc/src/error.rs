use std::path::PathBuf;

use thiserror::Error;

/// Errors that can occur when extracting Nixdoc comments.
#[derive(Debug, Error)]
pub enum NixdocError {
  /// The Nix file could not be read from disk.
  #[error("failed to read Nix file `{path}`: {source}")]
  ReadFile {
    path:   PathBuf,
    #[source]
    source: std::io::Error,
  },
}
