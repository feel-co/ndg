//! Types for ndg-commonmark public API and internal use.
use serde::{Deserialize, Serialize};

/// Represents a header in a Markdown document.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Header {
  /// Header text (inline content, no markdown formatting).
  pub text:  String,
  /// Header level (1-6).
  pub level: u8,
  /// Generated or explicit anchor ID for the header.
  pub id:    String,
}

/// Represents a file that was included via `{=include=}` directive.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IncludedFile {
  /// Path to the included file.
  pub path:          String,
  /// Optional custom output path from `html:into-file` directive.
  pub custom_output: Option<String>,
}

/// Result of Markdown processing.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MarkdownResult {
  /// Rendered HTML output.
  pub html: String,

  /// Extracted headers (for `ToC`, navigation, etc).
  pub headers: Vec<Header>,

  /// Title of the document, if found (usually first H1).
  pub title: Option<String>,

  /// Files that were included via `{=include=}` directives.
  pub included_files: Vec<IncludedFile>,
}
