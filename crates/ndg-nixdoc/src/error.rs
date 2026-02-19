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

  /// The Nix source could not be parsed.
  ///
  /// Unlike typical parse errors, `rnix` is error-tolerant and always
  /// produces a tree. This variant is reserved for cases where the parser
  /// signals hard parse errors that make the file unusable.
  #[error("failed to parse `{path}` as Nix: {message}")]
  ParseNix { path: PathBuf, message: String },
}
