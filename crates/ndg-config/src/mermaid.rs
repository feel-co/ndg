use std::path::Path;

use ndg_macros::{ConfigTemplate, Configurable};
use serde::{Deserialize, Serialize};

/// The Mermaid library that ndg loads when `mermaid.script` is not set.
pub const DEFAULT_MERMAID_SCRIPT: &str =
  "https://cdn.jsdelivr.net/npm/mermaid@11/dist/mermaid.min.js";

/// Configuration for Mermaid diagrams
#[derive(
  Debug, Clone, Default, Serialize, Deserialize, Configurable, ConfigTemplate,
)]
#[serde(default, deny_unknown_fields)]
pub struct MermaidConfig {
  /// Whether to render `mermaid` fenced code blocks as diagrams
  #[config(key = "enable")]
  pub enable: bool,

  /// Where to load the Mermaid library from
  ///
  /// A URL (`https://`, `http://` or `//`) is linked as-is. Any other value
  /// is a path to a local file, which ndg copies to the `assets` directory.
  /// Defaults to Mermaid 11 from jsDelivr.
  #[config(key = "script", allow_empty)]
  #[template(example = DEFAULT_MERMAID_SCRIPT.to_string())]
  pub script: Option<String>,
}

/// The source of the Mermaid library.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MermaidScript<'a> {
  /// A URL that pages link to as-is.
  Url(&'a str),
  /// A local file that ndg copies to the `assets` directory.
  File(&'a Path),
}

impl MermaidConfig {
  /// Returns the configured source of the Mermaid library.
  #[must_use]
  pub fn script_source(&self) -> MermaidScript<'_> {
    let script = self.script.as_deref().unwrap_or(DEFAULT_MERMAID_SCRIPT);
    if ["https://", "http://", "//"]
      .iter()
      .any(|prefix| script.starts_with(prefix))
    {
      MermaidScript::Url(script)
    } else {
      MermaidScript::File(Path::new(script))
    }
  }
}

#[cfg(test)]
#[expect(clippy::unwrap_used, reason = "Fine in tests")]
mod tests {
  use super::*;

  #[test]
  fn test_mermaid_config_defaults_to_disabled_cdn_script() {
    let config = MermaidConfig::default();
    assert!(!config.enable);
    assert_eq!(
      config.script_source(),
      MermaidScript::Url(DEFAULT_MERMAID_SCRIPT)
    );
  }

  #[test]
  fn test_mermaid_config_apply_override() {
    let mut config = MermaidConfig::default();
    config.apply_override("enable", "true").unwrap();
    config
      .apply_override("script", "vendor/mermaid.min.js")
      .unwrap();
    assert!(config.enable);
    assert_eq!(
      config.script_source(),
      MermaidScript::File(Path::new("vendor/mermaid.min.js"))
    );
  }

  #[test]
  fn test_mermaid_config_script_source_url() {
    for url in [
      "https://example.com/mermaid.min.js",
      "http://example.com/mermaid.min.js",
      "//example.com/mermaid.min.js",
    ] {
      let config = MermaidConfig {
        script: Some(url.to_string()),
        ..Default::default()
      };
      assert_eq!(config.script_source(), MermaidScript::Url(url));
    }
  }
}
