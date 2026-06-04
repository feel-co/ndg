use std::path::PathBuf;

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

/// A generated module options page.
///
/// Each page points at one `options.json` file and controls where the rendered
/// HTML page is written. `slug` is a root-relative output path without the
/// `.html` extension, such as `options`, `projects/hjem/stable`, or
/// `projects/nvf/v0.8`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct ModuleOptionsPage {
  /// Path to an `options.json` file in nixos-render-docs format.
  pub path: PathBuf,

  /// Root-relative output slug without `.html`.
  pub slug: String,

  /// Display title for this options page.
  pub title: Option<String>,

  /// Optional project version label shown in headings and search results.
  pub version: Option<String>,
}

impl Default for ModuleOptionsPage {
  fn default() -> Self {
    Self {
      path:    PathBuf::new(),
      slug:    "options".to_string(),
      title:   None,
      version: None,
    }
  }
}

impl ModuleOptionsPage {
  /// Title used in navigation and document headings.
  #[must_use]
  pub fn display_title(&self) -> String {
    let title = self.title.as_deref().unwrap_or("Module Options");
    if let Some(version) = self.version.as_deref()
      && !version.is_empty()
    {
      format!("{title} ({version})")
    } else {
      title.to_string()
    }
  }
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
