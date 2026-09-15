use ndg_macros::{ConfigTemplate, Configurable};
use serde::{Deserialize, Serialize};

/// Configuration for the aggregate printable document.
#[derive(
  Debug, Clone, Serialize, Deserialize, Configurable, ConfigTemplate,
)]
#[serde(default, deny_unknown_fields)]
pub struct PrintConfig {
  /// Whether to generate `print.html`.
  pub enable: bool,

  /// Whether printed chapters should start on a new page.
  pub page_break: bool,
}

impl Default for PrintConfig {
  fn default() -> Self {
    Self {
      enable:     true,
      page_break: true,
    }
  }
}
