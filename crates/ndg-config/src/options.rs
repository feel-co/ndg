use ndg_macros::Configurable;
use serde::{Deserialize, Serialize};

use crate::matchers::OptionNameMatch;

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

  /// Optional multi-page rendering configuration for module options.
  #[config(nested)]
  pub pages: Option<OptionsPagesConfig>,
}

impl OptionsConfig {
  /// Validate and compile all regex patterns in the options configuration.
  ///
  /// # Errors
  ///
  /// Returns an error if any regex pattern is invalid.
  pub fn validate(&mut self) -> Result<(), String> {
    if let Some(ref mut pages) = self.pages {
      pages.validate()?;
    }

    Ok(())
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

const fn default_pages_depth() -> usize {
  1
}

fn default_pages_root() -> String {
  "options".to_string()
}

/// Configuration for splitting option documentation across generated pages.
#[derive(Debug, Clone, Serialize, Deserialize, Configurable)]
#[serde(default)]
pub struct OptionsPagesConfig {
  /// Whether multi-page option documentation is enabled.
  #[config(key = "enabled")]
  pub enabled: bool,

  /// Default option-name component depth used as the owning page prefix.
  #[serde(default = "default_pages_depth")]
  #[config(key = "depth")]
  pub depth: usize,

  /// Output directory for generated option group pages.
  #[serde(default = "default_pages_root")]
  #[config(key = "root")]
  pub root: String,

  /// Pattern-based matching rules for deep or custom option page groups.
  pub matches: Vec<OptionsPageMatch>,
}

impl Default for OptionsPagesConfig {
  fn default() -> Self {
    Self {
      enabled: false,
      depth:   default_pages_depth(),
      root:    default_pages_root(),
      matches: Vec::new(),
    }
  }
}

impl OptionsPagesConfig {
  /// Validate and compile all regex patterns in page routing rules.
  ///
  /// # Errors
  ///
  /// Returns an error if any regex pattern is invalid.
  pub fn validate(&mut self) -> Result<(), String> {
    for (idx, m) in self.matches.iter_mut().enumerate() {
      m.compile_regexes()
        .map_err(|e| format!("Options page match #{}: {}", idx + 1, e))?;
    }

    Ok(())
  }

  /// Find the first page routing rule matching the given option name.
  #[must_use]
  pub fn find_match(&self, option_name: &str) -> Option<&OptionsPageMatch> {
    self.matches.iter().find(|m| m.matches(option_name))
  }
}

/// Matching rule for custom option page routing.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct OptionsPageMatch {
  /// Option name matching criteria.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub name: Option<OptionNameMatch>,

  /// Custom page prefix depth for matching options.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub depth: Option<usize>,

  /// Override display title for the generated page group.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub title: Option<String>,

  /// Custom page position in the options index.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub position: Option<usize>,
}

impl OptionsPageMatch {
  /// Compile all regex patterns in this routing rule.
  ///
  /// # Errors
  ///
  /// Returns an error if a regex pattern is invalid.
  pub fn compile_regexes(&mut self) -> Result<(), String> {
    if let Some(ref mut name_match) = self.name
      && let Some(ref pattern) = name_match.regex
    {
      name_match.compiled_regex = Some(
        regex::Regex::new(pattern)
          .map_err(|e| format!("Invalid name regex '{pattern}': {e}"))?,
      );
    }

    Ok(())
  }

  /// Check if this rule matches the given option name.
  ///
  /// # Panics
  ///
  /// Panics if `compile_regexes()` was not called after deserialization and a
  /// regex pattern is present. Configs loaded through [`OptionsConfig`] are
  /// validated before use.
  #[must_use]
  pub fn matches(&self, option_name: &str) -> bool {
    let Some(ref name_match) = self.name else {
      return false;
    };

    if let Some(ref exact_name) = name_match.exact
      && option_name != exact_name
    {
      return false;
    }

    if let Some(ref _pattern) = name_match.regex {
      #[expect(
        clippy::expect_used,
        reason = "invariant guaranteed during OptionsConfig validation"
      )]
      let re = name_match
        .compiled_regex
        .as_ref()
        .expect("internal error: invalid regex configuration");
      if !re.is_match(option_name) {
        return false;
      }
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
  #![allow(clippy::expect_used, reason = "Fine in tests")]

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

  #[test]
  fn parses_options_pages_config() {
    let toml = r#"
[pages]
enabled = true
depth = 2
root = "module-options"

[[pages.matches]]
name.regex = "^foo\\.bar\\.baz(\\.|$)"
depth = 3
title = "Foo Bar Baz"
position = 10
"#;

    let mut config: OptionsConfig =
      toml::from_str(toml).expect("Failed to parse options pages config");
    config
      .validate()
      .expect("options pages config should validate");

    let pages = config.pages.expect("pages config should be present");
    assert!(pages.enabled);
    assert_eq!(pages.depth, 2);
    assert_eq!(pages.root, "module-options");
    assert_eq!(pages.matches.len(), 1);
    assert!(pages.matches[0].matches("foo.bar.baz"));
    assert!(pages.matches[0].matches("foo.bar.baz.quz.enable"));
    assert!(!pages.matches[0].matches("foo.bar.bazzz.enable"));
    assert!(!pages.matches[0].matches("foo.bar.other.enable"));
  }
}
