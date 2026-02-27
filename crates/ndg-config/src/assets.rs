use ndg_macros::Configurable;
use serde::{Deserialize, Serialize};

/// Configuration for custom assets copying
///
/// Controls how custom assets from the assets directory are copied.
#[derive(Debug, Clone, Serialize, Deserialize, Configurable)]
#[serde(default)]
pub struct AssetsConfig {
  /// Follow symbolic links when copying assets
  ///
  /// When enabled, symlinks are followed. When disabled, symlinks are skipped.
  #[config(key = "follow_symlinks")]
  pub follow_symlinks: bool,

  /// Maximum directory depth to traverse
  ///
  /// Limits how deep the walker will descend into subdirectories.
  /// Use `None` for unlimited depth.
  #[config(key = "max_depth", allow_empty)]
  pub max_depth: Option<usize>,

  /// Skip hidden files and directories
  ///
  /// When enabled, files and directories starting with '.' are skipped.
  #[config(key = "skip_hidden")]
  pub skip_hidden: bool,
}

impl Default for AssetsConfig {
  fn default() -> Self {
    Self {
      follow_symlinks: false,
      max_depth:       None,
      skip_hidden:     true,
    }
  }
}
