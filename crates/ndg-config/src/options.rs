use ndg_macros::Configurable;
use serde::{Deserialize, Serialize};

const fn default_true() -> bool {
  true
}

/// Filters applied to module options before rendering options documentation.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Configurable)]
#[serde(default)]
pub struct OptionsConfig {
  /// Optional filtering configuration for module options.
  #[config(nested)]
  pub filter: Option<FilterConfig>,
}

/// Filters applied to module options before rendering options documentation.
#[derive(Debug, Clone, Serialize, Deserialize, Configurable)]
#[serde(default)]
pub struct FilterConfig {
  /// Include only options whose names start with this prefix.
  #[config(key = "prefix", allow_empty)]
  pub prefix: Option<String>,

  /// Include only options whose type contains this text, case-insensitively.
  #[serde(rename = "type")]
  #[config(key = "type", allow_empty)]
  pub type_name: Option<String>,

  /// Include only options whose names or descriptions contain this text.
  #[config(key = "search", allow_empty)]
  pub search: Option<String>,

  /// Include only options that define a default value.
  #[config(key = "has_default")]
  pub has_default: bool,

  /// Include only options that have a non-empty description.
  #[config(key = "has_description")]
  pub has_description: bool,

  /// Whether internal/hidden options should be included.
  #[serde(default = "default_true")]
  #[config(key = "include_internal")]
  pub include_internal: bool,
}

impl Default for FilterConfig {
  fn default() -> Self {
    Self {
      prefix:           None,
      type_name:        None,
      search:           None,
      has_default:      false,
      has_description:  false,
      include_internal: true,
    }
  }
}

impl FilterConfig {
  /// Check whether an option should be included after applying all filters.
  #[must_use]
  pub fn matches(
    &self,
    name: &str,
    type_name: &str,
    description: &str,
    has_default: bool,
    has_description: bool,
    internal: bool,
  ) -> bool {
    if !self.include_internal && internal {
      return false;
    }

    if let Some(prefix) = self.prefix.as_deref()
      && !prefix.is_empty()
      && !name.starts_with(prefix)
    {
      return false;
    }

    if let Some(type_filter) = self.type_name.as_deref()
      && !type_filter.is_empty()
      && !contains_ignore_ascii_case(type_name, type_filter)
    {
      return false;
    }

    if let Some(search) = self.search.as_deref()
      && !search.is_empty()
      && !contains_ignore_ascii_case(name, search)
      && !contains_ignore_ascii_case(description, search)
    {
      return false;
    }

    if self.has_default && !has_default {
      return false;
    }

    if self.has_description && !has_description {
      return false;
    }

    true
  }
}

fn contains_ignore_ascii_case(haystack: &str, needle: &str) -> bool {
  haystack
    .to_ascii_lowercase()
    .contains(&needle.to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn default_includes_internal_options() {
    assert!(FilterConfig::default().matches(
      "services.nginx.enable",
      "boolean",
      "Whether to enable nginx",
      true,
      true,
      true,
    ));
  }

  #[test]
  fn combines_filters() {
    let filter = FilterConfig {
      prefix:           Some("services.nginx".to_string()),
      type_name:        Some("bool".to_string()),
      search:           Some("enable".to_string()),
      has_default:      true,
      has_description:  true,
      include_internal: false,
    };

    assert!(filter.matches(
      "services.nginx.enable",
      "boolean",
      "Whether to enable nginx",
      true,
      true,
      false,
    ));
    assert!(!filter.matches(
      "services.httpd.enable",
      "boolean",
      "Whether to enable httpd",
      true,
      true,
      false,
    ));
    assert!(!filter.matches(
      "services.nginx.internal",
      "boolean",
      "Internal option",
      true,
      true,
      true,
    ));
  }
}
