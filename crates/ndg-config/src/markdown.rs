use ndg_commonmark::MarkdownExtension;
use ndg_macros::Configurable;
use serde::{Deserialize, Serialize};

/// Configuration for Markdown rendering.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Configurable)]
#[serde(default)]
pub struct MarkdownConfig {
  /// Additional Comrak Markdown extensions to enable.
  pub extensions: Vec<MarkdownExtension>,
}
