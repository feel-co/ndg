use serde::{Deserialize, Serialize};

/// Configuration for search functionality
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SearchConfig {
  /// Whether search functionality is enabled
  pub enable: bool,

  /// Maximum heading level to index (1-6)
  ///
  /// Controls which heading levels are included in the search index:
  /// - 1: Only H1 headings
  /// - 3: H1, H2, H3 headings (default)
  /// - 6: All headings H1-H6
  pub max_heading_level: u8,
}

impl Default for SearchConfig {
  fn default() -> Self {
    Self {
      enable:            true,
      max_heading_level: 3,
    }
  }
}
