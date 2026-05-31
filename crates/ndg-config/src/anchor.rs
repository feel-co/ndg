use ndg_macros::Configurable;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, Configurable)]
#[serde(default)]
pub struct AnchorConfig {
  #[config(key = "legacy_option_id_format")]
  pub legacy_option_id_format: bool,

  #[config(key = "compatibility_anchors")]
  pub compatibility_anchors: bool,
}
