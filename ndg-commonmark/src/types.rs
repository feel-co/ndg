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

/// Result of Markdown processing.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MarkdownResult {
  /// Rendered HTML output.
  pub html: String,

  /// Extracted headers (for `ToC`, navigation, etc).
  pub headers: Vec<Header>,

  /// Title of the document, if found (usually first H1).
  pub title: Option<String>,
}
