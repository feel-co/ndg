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

  /// Options specific to CSS minification
  pub css: Option<CssMinifyOptions>,

  /// Options specific to JavaScript minification
  pub js: Option<JsMinifyOptions>,
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

/// Options for CSS minification
///
/// These control `lightningcss` behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CssMinifyOptions {
  /// Enable minification
  ///
  /// When enabled, applies CSS minification including whitespace removal,
  /// shorthand properties, and other optimizations.
  pub minify: bool,
}

impl Default for CssMinifyOptions {
  fn default() -> Self {
    Self { minify: true }
  }
}

/// Options for JavaScript minification
///
/// These control `oxc_minifier` behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct JsMinifyOptions {
  /// Enable compression optimizations
  ///
  /// When enabled, applies transformations like dead code elimination,
  /// constant folding, and expression simplification.
  pub compress: bool,

  /// Enable name mangling
  ///
  /// When enabled, shortens variable and function names to reduce file size.
  pub mangle: bool,
}

impl Default for JsMinifyOptions {
  fn default() -> Self {
    Self {
      compress: true,
      mangle:   true,
    }
  }
}

impl PostprocessConfig {
  /// Get HTML minify options or default
  #[must_use]
  pub fn html_options(&self) -> HtmlMinifyOptions {
    self.html.clone().unwrap_or_default()
  }

  /// Get CSS minify options or default
  #[must_use]
  pub fn css_options(&self) -> CssMinifyOptions {
    self.css.clone().unwrap_or_default()
  }

  /// Get JavaScript minify options or default
  #[must_use]
  pub fn js_options(&self) -> JsMinifyOptions {
    self.js.clone().unwrap_or_default()
  }
}
