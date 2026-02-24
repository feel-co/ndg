use ndg_macros::Configurable;
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
#[derive(Debug, Clone, Serialize, Deserialize, Default, Configurable)]
#[serde(default)]
pub struct PostprocessConfig {
  /// Whether to minify HTML output
  #[config(key = "minify_html")]
  pub minify_html: bool,

  /// Whether to minify CSS output
  #[config(key = "minify_css")]
  pub minify_css: bool,

  /// Whether to minify JavaScript output
  #[config(key = "minify_js")]
  pub minify_js: bool,

  /// Options specific to HTML minification
  #[config(nested)]
  pub html: Option<HtmlMinifyOptions>,

  /// Options specific to CSS minification
  #[config(nested)]
  pub css: Option<CssMinifyOptions>,

  /// Options specific to JavaScript minification
  #[config(nested)]
  pub js: Option<JsMinifyOptions>,
}

/// Options for HTML minification
///
/// These control `minify-html` behavior.
#[derive(Debug, Clone, Serialize, Deserialize, Configurable)]
#[serde(default)]
pub struct HtmlMinifyOptions {
  /// Remove HTML comments
  ///
  /// When enabled, removes all HTML comments from the output.
  #[config(key = "remove_comments")]
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
#[derive(Debug, Clone, Serialize, Deserialize, Configurable)]
#[serde(default)]
pub struct CssMinifyOptions {
  /// Enable minification
  ///
  /// When enabled, applies CSS minification including whitespace removal,
  /// shorthand properties, and other optimizations.
  #[config(key = "minify")]
  pub minify: bool,

  /// Project root for resolving relative paths
  ///
  /// Used to resolve relative imports and URL references in CSS.
  /// If `None`, paths are resolved relative to the CSS file location.
  #[config(key = "project_root")]
  pub project_root: Option<String>,
}

impl Default for CssMinifyOptions {
  fn default() -> Self {
    Self {
      minify:       true,
      project_root: None,
    }
  }
}

/// Options for JavaScript minification
///
/// These control `oxc_minifier` behavior.
#[derive(Debug, Clone, Serialize, Deserialize, Configurable)]
#[serde(default)]
pub struct JsMinifyOptions {
  /// Enable compression optimizations
  ///
  /// When enabled, applies transformations like dead code elimination,
  /// constant folding, and expression simplification.
  #[config(key = "compress")]
  pub compress: bool,

  /// Enable name mangling
  ///
  /// When enabled, shortens variable and function names to reduce file size.
  #[config(key = "mangle")]
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

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_postprocess_config_apply_override_bools() {
    let mut config = PostprocessConfig::default();
    assert!(!config.minify_html);

    config.apply_override("minify_html", "true").unwrap();
    assert!(config.minify_html);

    config.apply_override("minify_css", "true").unwrap();
    assert!(config.minify_css);

    config.apply_override("minify_js", "true").unwrap();
    assert!(config.minify_js);
  }

  #[test]
  fn test_postprocess_config_nested_html() {
    let mut config = PostprocessConfig::default();

    config
      .apply_override("html.remove_comments", "false")
      .unwrap();
    assert!(config.html.is_some());
    assert!(!config.html.unwrap().remove_comments);
  }

  #[test]
  fn test_postprocess_config_nested_css() {
    let mut config = PostprocessConfig::default();

    config.apply_override("css.minify", "false").unwrap();
    assert!(config.css.is_some());
    assert!(!config.css.as_ref().unwrap().minify);

    config
      .apply_override("css.project_root", "/custom/path")
      .unwrap();
    assert_eq!(
      config.css.as_ref().unwrap().project_root,
      Some("/custom/path".to_string())
    );
  }

  #[test]
  fn test_postprocess_config_nested_js() {
    let mut config = PostprocessConfig::default();

    config.apply_override("js.compress", "false").unwrap();
    assert!(config.js.is_some());
    assert!(!config.js.as_ref().unwrap().compress);

    config.apply_override("js.mangle", "false").unwrap();
    assert!(!config.js.as_ref().unwrap().mangle);
  }

  #[test]
  fn test_postprocess_config_merge_fields() {
    let mut config = PostprocessConfig::default();
    config.minify_html = true;

    let other = PostprocessConfig {
      minify_html: false,
      minify_css: true,
      minify_js: true,
      ..Default::default()
    };

    config.merge_fields(other);

    assert!(!config.minify_html);
    assert!(config.minify_css);
    assert!(config.minify_js);
  }

  #[test]
  fn test_postprocess_config_nested_merge() {
    let mut config = PostprocessConfig::default();
    config.html = Some(HtmlMinifyOptions {
      remove_comments: true,
    });

    let other = PostprocessConfig {
      html: Some(HtmlMinifyOptions {
        remove_comments: false,
      }),
      ..Default::default()
    };

    config.merge_fields(other);

    assert!(config.html.is_some());
    assert!(!config.html.unwrap().remove_comments);
  }
}
