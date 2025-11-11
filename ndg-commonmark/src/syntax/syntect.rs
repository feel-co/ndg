//! Syntect-based syntax highlighting backend enhanced with two-face.
//!
//! This module provides a syntax highlighter using the Syntect library,
//! which uses Sublime Text's syntax definitions (TextMate grammars),
//! significantly enhanced with the two-face crate for extended
//! syntax definitions and themes.

use std::sync::OnceLock;

use syntect::{
  highlighting::Theme,
  html::highlighted_html_for_string,
  parsing::SyntaxSet,
};
use two_face::{
  re_exports::syntect::highlighting::ThemeSet,
  theme::{EmbeddedLazyThemeSet, EmbeddedThemeName},
};

use super::{
  error::{SyntaxError, SyntaxResult},
  types::{SyntaxConfig, SyntaxHighlighter, SyntaxManager},
};

/// Syntect-based syntax highlighter
pub struct SyntectHighlighter {
  theme_name: String,
}

impl SyntectHighlighter {
  /// Create a new Syntect highlighter with the specified theme.
  pub fn new(theme_name: Option<String>) -> Self {
    Self {
      theme_name: theme_name.unwrap_or_else(|| "InspiredGitHub".to_string()),
    }
  }

  /// Get the syntect SyntaxSet.
  fn syntax_set() -> &'static SyntaxSet {
    static SYNTAX_SET: OnceLock<SyntaxSet> = OnceLock::new();
    SYNTAX_SET.get_or_init(two_face::syntax::extra_newlines)
  }

  /// Get the syntect ThemeSet with extended themes.
  fn theme_set() -> &'static EmbeddedLazyThemeSet {
    static THEME_SET: OnceLock<EmbeddedLazyThemeSet> = OnceLock::new();
    THEME_SET.get_or_init(two_face::theme::extra)
  }

  /// Get the default syntect ThemeSet for fallback themes.
  fn default_theme_set() -> &'static ThemeSet {
    static DEFAULT_THEME_SET: OnceLock<ThemeSet> = OnceLock::new();
    DEFAULT_THEME_SET.get_or_init(ThemeSet::load_defaults)
  }

  /// Get the theme by name.
  fn get_theme(&self, theme_name: Option<&str>) -> &'static Theme {
    let theme_set = Self::theme_set();
    let default_theme_set = Self::default_theme_set();
    let name = if theme_name.is_some() {
      theme_name.unwrap()
    } else if !self.theme_name.is_empty() {
      &self.theme_name
    } else {
      "InspiredGitHub" // guaranteed fallback
    };

    // Try to get theme from default syntect themes first
    if let Some(theme) = default_theme_set.themes.get(name) {
      return theme;
    }

    // Try to get theme from two-face themes by matching name
    let embedded_theme = match name {
      "Ansi" => Some(EmbeddedThemeName::Ansi),
      "Base16" => Some(EmbeddedThemeName::Base16),
      "Base16EightiesDark" => Some(EmbeddedThemeName::Base16EightiesDark),
      "Base16MochaDark" => Some(EmbeddedThemeName::Base16MochaDark),
      "Base16OceanDark" => Some(EmbeddedThemeName::Base16OceanDark),
      "Base16OceanLight" => Some(EmbeddedThemeName::Base16OceanLight),
      "Base16_256" => Some(EmbeddedThemeName::Base16_256),
      "ColdarkCold" => Some(EmbeddedThemeName::ColdarkCold),
      "ColdarkDark" => Some(EmbeddedThemeName::ColdarkDark),
      "DarkNeon" => Some(EmbeddedThemeName::DarkNeon),
      "Dracula" => Some(EmbeddedThemeName::Dracula),
      "Github" => Some(EmbeddedThemeName::Github),
      "GruvboxDark" => Some(EmbeddedThemeName::GruvboxDark),
      "GruvboxLight" => Some(EmbeddedThemeName::GruvboxLight),
      "InspiredGithub" => Some(EmbeddedThemeName::InspiredGithub),
      "Leet" => Some(EmbeddedThemeName::Leet),
      "MonokaiExtended" => Some(EmbeddedThemeName::MonokaiExtended),
      "MonokaiExtendedBright" => Some(EmbeddedThemeName::MonokaiExtendedBright),
      "MonokaiExtendedLight" => Some(EmbeddedThemeName::MonokaiExtendedLight),
      "MonokaiExtendedOrigin" => Some(EmbeddedThemeName::MonokaiExtendedOrigin),
      "Nord" => Some(EmbeddedThemeName::Nord),
      "OneHalfDark" => Some(EmbeddedThemeName::OneHalfDark),
      "OneHalfLight" => Some(EmbeddedThemeName::OneHalfLight),
      "SolarizedDark" => Some(EmbeddedThemeName::SolarizedDark),
      "SolarizedLight" => Some(EmbeddedThemeName::SolarizedLight),
      "SublimeSnazzy" => Some(EmbeddedThemeName::SublimeSnazzy),
      "TwoDark" => Some(EmbeddedThemeName::TwoDark),
      "VisualStudioDarkPlus" => Some(EmbeddedThemeName::VisualStudioDarkPlus),
      "Zenburn" => Some(EmbeddedThemeName::Zenburn),
      _ => None,
    };

    if let Some(embedded_name) = embedded_theme {
      return theme_set.get(embedded_name);
    }

    // Fall back to default theme
    default_theme_set
      .themes
      .get("InspiredGitHub")
      .unwrap_or_else(|| theme_set.get(EmbeddedThemeName::InspiredGithub))
  }
}

impl Default for SyntectHighlighter {
  fn default() -> Self {
    Self::new(None)
  }
}

impl SyntaxHighlighter for SyntectHighlighter {
  fn name(&self) -> &'static str {
    "Syntect"
  }

  fn supported_languages(&self) -> Vec<String> {
    Self::syntax_set()
      .syntaxes()
      .iter()
      .flat_map(|syntax| {
        std::iter::once(syntax.name.to_lowercase())
          .chain(syntax.file_extensions.iter().map(|ext| ext.to_lowercase()))
      })
      .collect()
  }

  fn available_themes(&self) -> Vec<String> {
    let default_theme_set = Self::default_theme_set();
    let mut themes: Vec<String> =
      default_theme_set.themes.keys().cloned().collect();

    // Add all embedded themes from two-face
    let embedded_themes: Vec<String> = vec![
      "Ansi".to_string(),
      "Base16".to_string(),
      "Base16EightiesDark".to_string(),
      "Base16MochaDark".to_string(),
      "Base16OceanDark".to_string(),
      "Base16OceanLight".to_string(),
      "Base16_256".to_string(),
      "ColdarkCold".to_string(),
      "ColdarkDark".to_string(),
      "DarkNeon".to_string(),
      "Dracula".to_string(),
      "Github".to_string(),
      "GruvboxDark".to_string(),
      "GruvboxLight".to_string(),
      "InspiredGithub".to_string(),
      "Leet".to_string(),
      "MonokaiExtended".to_string(),
      "MonokaiExtendedBright".to_string(),
      "MonokaiExtendedLight".to_string(),
      "MonokaiExtendedOrigin".to_string(),
      "Nord".to_string(),
      "OneHalfDark".to_string(),
      "OneHalfLight".to_string(),
      "SolarizedDark".to_string(),
      "SolarizedLight".to_string(),
      "SublimeSnazzy".to_string(),
      "TwoDark".to_string(),
      "VisualStudioDarkPlus".to_string(),
      "Zenburn".to_string(),
    ];

    themes.extend(embedded_themes);
    themes.sort();
    themes.dedup();
    themes
  }

  fn highlight(
    &self,
    code: &str,
    language: &str,
    theme: Option<&str>,
  ) -> SyntaxResult<String> {
    let syntax_set = Self::syntax_set();
    let syntax = syntax_set
      .find_syntax_by_token(language)
      .unwrap_or_else(|| syntax_set.find_syntax_plain_text());

    let theme = self.get_theme(theme);

    highlighted_html_for_string(code, syntax_set, syntax, theme)
      .map_err(|e| SyntaxError::HighlightingFailed(e.to_string()))
  }

  fn language_from_extension(&self, extension: &str) -> Option<String> {
    let syntax_set = Self::syntax_set();
    syntax_set
      .find_syntax_by_extension(extension)
      .map(|syntax| syntax.name.to_lowercase())
  }
}

/// Create a Syntect-based syntax manager with configuration
pub fn create_syntect_manager() -> SyntaxResult<SyntaxManager> {
  let highlighter = Box::new(SyntectHighlighter::default());
  let mut config = SyntaxConfig::default();
  config.default_theme = Some("InspiredGitHub".to_string());
  Ok(SyntaxManager::new(highlighter, config))
}
