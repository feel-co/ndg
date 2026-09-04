use std::str::FromStr;

use ndg_macros::Configurable;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, Configurable)]
#[serde(default, deny_unknown_fields)]
pub struct AnchorConfig {
  #[config(key = "legacy_option_id_format")]
  pub legacy_option_id_format: bool,

  #[config(key = "compatibility_anchors")]
  pub compatibility_anchors: bool,

  /// How to handle duplicate heading anchor IDs within a single page.
  #[config(key = "on_duplicate")]
  pub on_duplicate: DuplicateAnchorPolicy,
}

/// Policy for duplicate heading anchor IDs within a single page.
#[derive(
  Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum DuplicateAnchorPolicy {
  /// Fail the build on duplicate anchors (current behaviour).
  #[default]
  Error,
  /// Log a warning and keep the duplicate IDs as-is.
  Warn,
  /// Make anchors unique with deterministic `-1`, `-2`, ... suffixes.
  Deduplicate,
}

impl FromStr for DuplicateAnchorPolicy {
  type Err = String;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s.to_lowercase().as_str() {
      "error" => Ok(Self::Error),
      "warn" => Ok(Self::Warn),
      "deduplicate" => Ok(Self::Deduplicate),
      _ => {
        Err(format!(
          "Unknown duplicate anchor policy: {s}; expected `error`, `warn`, or \
           `deduplicate`"
        ))
      },
    }
  }
}

#[cfg(test)]
mod tests {
  #![allow(clippy::unwrap_used, reason = "Fine in tests")]

  use super::*;

  #[test]
  fn test_duplicate_policy_default_is_error() {
    let config = AnchorConfig::default();
    assert_eq!(config.on_duplicate, DuplicateAnchorPolicy::Error);
  }

  #[test]
  fn test_duplicate_policy_toml_deserialization() {
    let config: AnchorConfig =
      toml::from_str("on_duplicate = \"warn\"").unwrap();
    assert_eq!(config.on_duplicate, DuplicateAnchorPolicy::Warn);

    let config: AnchorConfig =
      toml::from_str("on_duplicate = \"deduplicate\"").unwrap();
    assert_eq!(config.on_duplicate, DuplicateAnchorPolicy::Deduplicate);
  }

  #[test]
  fn test_duplicate_policy_apply_override() {
    let mut config = AnchorConfig::default();
    config.apply_override("on_duplicate", "warn").unwrap();
    assert_eq!(config.on_duplicate, DuplicateAnchorPolicy::Warn);

    let err = config.apply_override("on_duplicate", "bogus").unwrap_err();
    assert!(err.to_string().contains("Unknown duplicate anchor policy"));
  }
}
