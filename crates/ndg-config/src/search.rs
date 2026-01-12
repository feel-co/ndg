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

  /// Minimum word length for indexing
  ///
  /// Words shorter than this length will be excluded from the search index.
  /// Default is 2, meaning single-character words are ignored.
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
  pub boost_title: Option<f32>,

  /// Score multiplier for content matches
  ///
  /// If not set, falls back to `boost` value. Default is 30.0 for fuzzy
  /// matches and 2.0 for partial matches.
  pub boost_content: Option<f32>,

  /// Score multiplier for anchor/heading matches
  ///
  /// If not set, falls back to `boost` value. This affects matching section
  /// headings.
  pub boost_anchor: Option<f32>,

  /// Global score multiplier
  ///
  /// Sets all boost values at once. Individual boost_* settings override
  /// this for their respective categories.
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
