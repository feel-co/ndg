//! Core types and traits for syntax highlighting.

use std::collections::HashMap;

use super::error::{SyntaxError, SyntaxResult};

/// Trait for syntax highlighting backends.
///
/// Allows different syntax highlighting implementations to be used
/// interchangeably. Implementations should handle language detection, theme
/// management, and the actual highlighting process.
pub trait SyntaxHighlighter: Send + Sync {
  /// Get the name of this highlighter backend
  fn name(&self) -> &'static str;

  /// Get a list of supported languages
  fn supported_languages(&self) -> Vec<String>;

  /// Get a list of available themes
  fn available_themes(&self) -> Vec<String>;

  /// Check if a language is supported
  fn supports_language(&self, language: &str) -> bool {
    self
      .supported_languages()
      .iter()
      .any(|lang| lang.eq_ignore_ascii_case(language))
  }

  /// Check if a theme is available
  fn has_theme(&self, theme: &str) -> bool {
    self
      .available_themes()
      .iter()
      .any(|t| t.eq_ignore_ascii_case(theme))
  }

  /// Highlight code with the specified language and theme.
  ///
  /// # Arguments
  ///
  /// * `code` - The source code to highlight
  /// * `language` - The programming language (case-insensitive)
  /// * `theme` - The theme name (case-insensitive, optional)
  ///
  /// # Returns
  ///
  /// Highlighted HTML string on success
  fn highlight(
    &self,
    code: &str,
    language: &str,
    theme: Option<&str>,
  ) -> SyntaxResult<String>;

  /// Detect language from a file extension
  fn language_from_extension(&self, extension: &str) -> Option<String>;

  /// Detect language from a filename
  fn language_from_filename(&self, filename: &str) -> Option<String> {
    std::path::Path::new(filename)
      .extension()
      .and_then(|ext| ext.to_str())
      .and_then(|ext| self.language_from_extension(ext))
  }
}

/// Configuration for syntax highlighting
#[derive(Debug, Clone)]
pub struct SyntaxConfig {
  /// Default theme to use when none is specified
  pub default_theme: Option<String>,

  /// Language aliases for mapping common names to supported languages
  pub language_aliases: HashMap<String, String>,

  /// Whether to fall back to plain text for unsupported languages
  pub fallback_to_plain: bool,
}

impl Default for SyntaxConfig {
  fn default() -> Self {
    let mut language_aliases = HashMap::new();

    // Common aliases
    language_aliases.insert("js".to_string(), "javascript".to_string());
    language_aliases.insert("ts".to_string(), "typescript".to_string());
    language_aliases.insert("py".to_string(), "python".to_string());
    language_aliases.insert("rb".to_string(), "ruby".to_string());
    language_aliases.insert("sh".to_string(), "bash".to_string());
    language_aliases.insert("shell".to_string(), "bash".to_string());
    language_aliases.insert("yml".to_string(), "yaml".to_string());
    language_aliases.insert("nixos".to_string(), "nix".to_string());
    language_aliases.insert("md".to_string(), "markdown".to_string());

    Self {
      default_theme: None,
      language_aliases,
      fallback_to_plain: true,
    }
  }
}

/// High-level syntax highlighting manager.
///
/// Manages a syntax highlighting backend and provides a convenient
/// interface for highlighting code with configuration options.
pub struct SyntaxManager {
  highlighter: Box<dyn SyntaxHighlighter>,
  config:      SyntaxConfig,
}

impl SyntaxManager {
  /// Create a new syntax manager with the given highlighter and config
  #[must_use]
  pub fn new(
    highlighter: Box<dyn SyntaxHighlighter>,
    config: SyntaxConfig,
  ) -> Self {
    Self {
      highlighter,
      config,
    }
  }

  /// Create a new syntax manager with the default configuration
  #[must_use]
  pub fn with_highlighter(highlighter: Box<dyn SyntaxHighlighter>) -> Self {
    Self::new(highlighter, SyntaxConfig::default())
  }

  /// Get the underlying highlighter
  #[must_use]
  pub fn highlighter(&self) -> &dyn SyntaxHighlighter {
    self.highlighter.as_ref()
  }

  /// Get the configuration
  #[must_use]
  pub fn config(&self) -> &SyntaxConfig {
    &self.config
  }

  /// Update the configuration
  pub fn set_config(&mut self, config: SyntaxConfig) {
    self.config = config;
  }

  /// Resolve a language name using aliases
  #[must_use]
  pub fn resolve_language(&self, language: &str) -> String {
    self
      .config
      .language_aliases
      .get(language)
      .cloned()
      .unwrap_or_else(|| language.to_string())
  }

  /// Highlight code with automatic language resolution and fallback
  pub fn highlight_code(
    &self,
    code: &str,
    language: &str,
    theme: Option<&str>,
  ) -> SyntaxResult<String> {
    let resolved_language = self.resolve_language(language);
    let theme = theme.or(self.config.default_theme.as_deref());

    // Try to highlight with the resolved language
    if self.highlighter.supports_language(&resolved_language) {
      return self.highlighter.highlight(code, &resolved_language, theme);
    }

    // If language is not supported and fallback is enabled, try plain text
    if self.config.fallback_to_plain {
      if self.highlighter.supports_language("text") {
        return self.highlighter.highlight(code, "text", theme);
      }
      if self.highlighter.supports_language("plain") {
        return self.highlighter.highlight(code, "plain", theme);
      }
    }

    Err(SyntaxError::UnsupportedLanguage(resolved_language))
  }

  /// Highlight code from a filename
  pub fn highlight_from_filename(
    &self,
    code: &str,
    filename: &str,
    theme: Option<&str>,
  ) -> SyntaxResult<String> {
    if let Some(language) = self.highlighter.language_from_filename(filename) {
      self.highlight_code(code, &language, theme)
    } else if self.config.fallback_to_plain {
      self.highlight_code(code, "text", theme)
    } else {
      Err(SyntaxError::UnsupportedLanguage(format!(
        "from filename: {filename}"
      )))
    }
  }
}
