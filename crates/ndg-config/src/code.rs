use ndg_macros::{ConfigTemplate, Configurable};
use serde::{Deserialize, Serialize};

/// Presentation controls for fenced code examples.
#[derive(
  Debug, Clone, Serialize, Deserialize, Configurable, ConfigTemplate,
)]
#[serde(default, deny_unknown_fields)]
pub struct CodeConfig {
  /// Add a copy button to rendered code blocks.
  pub copy_button: bool,

  /// Collapse blocks with more than this many lines. `None` disables folding.
  #[template(example = 40usize)]
  pub collapse_lines: Option<usize>,
}

impl Default for CodeConfig {
  fn default() -> Self {
    Self {
      copy_button:    true,
      collapse_lines: None,
    }
  }
}
