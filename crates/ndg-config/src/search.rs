use ndg_macros::Configurable;
use serde::{Deserialize, Serialize};

/// Configuration for search functionality
#[derive(Debug, Clone, Serialize, Deserialize, Configurable)]
#[serde(default, deny_unknown_fields)]
pub struct SearchConfig {
  /// Whether search functionality is enabled
  #[config(key = "enable")]
  pub enable: bool,

  /// Maximum heading level to index (1-6)
  ///
  /// Controls which heading levels are included in the search index:
  /// - 1: Only H1 headings
  /// - 3: H1, H2, H3 headings (default)
  /// - 6: All headings H1-H6
  #[config(key = "max_heading_level")]
  pub max_heading_level: u8,

  /// Minimum word length for indexing
  ///
  /// Words shorter than this length will be excluded from the search index.
  /// Default is 2, meaning single-character words are ignored.
  #[config(key = "min_word_length")]
  pub min_word_length: usize,

  /// List of stopwords to exclude from indexing
  ///
  /// Common words that should be ignored during indexing.
  /// Empty by default - users can add their own stopwords.
  pub stopwords: Vec<String>,

  /// Score multiplier for title matches
  ///
  /// If not set, falls back to `boost` value. Default is 100.0 for fuzzy
  /// matches and 20.0 for exact matches.
  #[config(key = "boost_title")]
  pub boost_title: Option<f32>,

  /// Score multiplier for content matches
  ///
  /// If not set, falls back to `boost` value. Default is 30.0 for fuzzy
  /// matches and 2.0 for partial matches.
  #[config(key = "boost_content")]
  pub boost_content: Option<f32>,

  /// Score multiplier for anchor/heading matches
  ///
  /// If not set, falls back to `boost` value. This affects matching section
  /// headings.
  #[config(key = "boost_anchor")]
  pub boost_anchor: Option<f32>,

  /// Global score multiplier
  ///
  /// Sets all boost values at once. Individual boost_* settings override
  /// this for their respective categories.
  #[config(key = "boost")]
  pub boost: Option<f32>,
}

impl Default for SearchConfig {
  fn default() -> Self {
    Self {
      enable:            true,
      max_heading_level: 3,
      min_word_length:   2,
      stopwords:         Vec::new(),
      boost_title:       None,
      boost_content:     None,
      boost_anchor:      None,
      boost:             None,
    }
  }
}

impl SearchConfig {
  /// Validate search limits and score multipliers.
  ///
  /// # Errors
  ///
  /// Returns an error when a heading level is outside `1..=6` or a score
  /// multiplier is negative or non-finite.
  pub fn validate(&self) -> Result<(), String> {
    if !(1..=6).contains(&self.max_heading_level) {
      return Err(format!(
        "`search.max_heading_level` is {}; expected a value from 1 through 6",
        self.max_heading_level
      ));
    }

    for (field, value) in [
      ("boost_title", self.boost_title),
      ("boost_content", self.boost_content),
      ("boost_anchor", self.boost_anchor),
      ("boost", self.boost),
    ] {
      if let Some(value) = value
        && (!value.is_finite() || value < 0.0)
      {
        return Err(format!(
          "`search.{field}` is {value}; expected a finite, non-negative number"
        ));
      }
    }

    Ok(())
  }

  /// Get the effective title boost value
  #[must_use]
  pub fn get_title_boost(&self) -> f32 {
    self.boost_title.or(self.boost).unwrap_or(100.0)
  }

  /// Get the effective content boost value
  #[must_use]
  pub fn get_content_boost(&self) -> f32 {
    self.boost_content.or(self.boost).unwrap_or(30.0)
  }

  /// Get the effective anchor boost value
  #[must_use]
  pub fn get_anchor_boost(&self) -> f32 {
    self.boost_anchor.or(self.boost).unwrap_or(10.0)
  }
}

#[cfg(test)]
#[expect(clippy::unwrap_used, reason = "Fine in tests")]
mod tests {
  use super::*;

  #[test]
  fn test_search_config_apply_override_enable() {
    let mut config = SearchConfig::default();
    assert!(config.enable);

    config.apply_override("enable", "false").unwrap();
    assert!(!config.enable);
  }

  #[test]
  fn test_search_config_apply_override_max_heading_level() {
    let mut config = SearchConfig::default();
    config.apply_override("max_heading_level", "6").unwrap();
    assert_eq!(config.max_heading_level, 6);
  }

  #[test]
  fn test_search_config_apply_override_min_word_length() {
    let mut config = SearchConfig::default();
    config.apply_override("min_word_length", "3").unwrap();
    assert_eq!(config.min_word_length, 3);
  }

  #[test]
  fn test_search_config_apply_override_boost_values() {
    let mut config = SearchConfig::default();

    config.apply_override("boost", "1.5").unwrap();
    assert_eq!(config.boost, Some(1.5));

    config.apply_override("boost_title", "2.0").unwrap();
    assert_eq!(config.boost_title, Some(2.0));

    config.apply_override("boost_content", "0.5").unwrap();
    assert_eq!(config.boost_content, Some(0.5));

    config.apply_override("boost_anchor", "1.0").unwrap();
    assert_eq!(config.boost_anchor, Some(1.0));
  }

  #[test]
  fn test_search_config_apply_override_unknown_key() {
    let mut config = SearchConfig::default();

    let result = config.apply_override("unknown_field", "value");
    assert!(result.is_err());
    assert!(
      result
        .unwrap_err()
        .to_string()
        .contains("Unknown configuration key")
    );
  }

  #[test]
  fn test_search_config_merge_fields() {
    let mut config = SearchConfig {
      enable: false,
      max_heading_level: 2,
      ..Default::default()
    };

    let other = SearchConfig {
      enable: true,
      max_heading_level: 5,
      min_word_length: 4,
      ..Default::default()
    };

    config.merge_fields(other);

    assert!(config.enable);
    assert_eq!(config.max_heading_level, 5);
    assert_eq!(config.min_word_length, 4);
  }

  #[test]
  fn test_search_config_validate_rejects_heading_level_out_of_range() {
    let config = SearchConfig {
      max_heading_level: 7,
      ..Default::default()
    };

    let error = config.validate().expect_err("heading level 7 must fail");

    assert!(error.contains("search.max_heading_level"));
    assert!(error.contains("1 through 6"));
  }

  #[test]
  fn test_search_config_validate_rejects_invalid_boost() {
    let config = SearchConfig {
      boost: Some(f32::INFINITY),
      ..Default::default()
    };

    let error = config.validate().expect_err("infinite boost must fail");

    assert!(error.contains("search.boost"));
    assert!(error.contains("finite"));
  }
}
