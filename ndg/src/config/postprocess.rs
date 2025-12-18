use serde::{Deserialize, Serialize};

/// Configuration for HTML/CSS/JS postprocessing
///
/// Controls minification of generated output files using specialized
/// minification libraries:
///
/// - HTML: `minify-html` - Fast, spec-compliant HTML minification
/// - CSS: `lightningcss` - Production-grade CSS parsing and minification
/// - JavaScript: `oxc_minifier` - Production-grade JavaScript minification with
///   17+ optimization passes
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
/// These control `minify-html` behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct HtmlMinifyOptions {
  /// Remove HTML comments
  ///
  /// When enabled, removes all HTML comments from the output.
  pub remove_comments: bool,
}

impl Default for HtmlMinifyOptions {
  fn default() -> Self {
    Self {
      remove_comments: true,
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
