use serde::{Deserialize, Serialize};

/// Configuration for HTML/CSS/JS postprocessing
///
/// Controls minification of generated output files.
///
/// All minification is performed using the [`minifier`] crate, which provides
/// basic minification without excessive configuration options.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct PostprocessConfig {
  /// Whether to minify HTML output
  pub minify_html: bool,

  /// Whether to minify CSS output
  pub minify_css: bool,

  /// Whether to minify JavaScript output
  pub minify_js: bool,

  /// Options specific to HTML minification
  pub html: Option<HtmlMinifyOptions>,
}

/// Options for HTML minification
///
/// These control post-processing behavior applied after the `minifier` crate's
/// basic HTML minification.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct HtmlMinifyOptions {
  /// Collapse whitespace in text nodes
  ///
  /// When enabled, uses the minifier crate's output which collapses multiple
  /// spaces into single spaces.
  pub collapse_whitespace: bool,

  /// Remove HTML comments
  ///
  /// When enabled, applies a custom comment removal pass after minification.
  pub remove_comments: bool,
}

impl Default for HtmlMinifyOptions {
  fn default() -> Self {
    Self {
      collapse_whitespace: true,
      remove_comments:     true,
    }
  }
}

impl PostprocessConfig {
  /// Get HTML minify options or default
  #[must_use]
  pub fn html_options(&self) -> HtmlMinifyOptions {
    self.html.clone().unwrap_or_default()
  }
}
