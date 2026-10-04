use ndg_commonmark::MarkdownExtension;
use ndg_macros::{ConfigTemplate, Configurable};
use serde::{Deserialize, Serialize};

/// Configuration for Markdown rendering.
#[derive(
  Debug, Clone, Default, Serialize, Deserialize, Configurable, ConfigTemplate,
)]
#[serde(default, deny_unknown_fields)]
pub struct MarkdownConfig {
  /// Additional Comrak Markdown extensions to enable.
  pub extensions: Vec<MarkdownExtension>,
}
