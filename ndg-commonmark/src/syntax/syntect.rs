//! Syntect-based syntax highlighting backend.
//!
//! This module provides a legacy regex-based syntax highlighter using the
//! Syntect library. It serves as a fallback when Syntastica is not available
//! or when the syntect feature is explicitly enabled.

use std::sync::OnceLock;

use syntect::{
    highlighting::{Theme, ThemeSet},
    html::highlighted_html_for_string,
    parsing::SyntaxSet,
};

use super::{
    error::{SyntaxError, SyntaxResult},
    types::{SyntaxConfig, SyntaxHighlighter, SyntaxManager},
};

/// Syntect-based syntax highlighter (legacy/fallback).
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

    /// Get the syntect SyntaxSet (cached, thread-safe).
    fn syntax_set() -> &'static SyntaxSet {
        static SYNTAX_SET: OnceLock<SyntaxSet> = OnceLock::new();
        SYNTAX_SET.get_or_init(SyntaxSet::load_defaults_newlines)
    }

    /// Get the syntect ThemeSet (cached, thread-safe).
    fn theme_set() -> &'static ThemeSet {
        static THEME_SET: OnceLock<ThemeSet> = OnceLock::new();
        THEME_SET.get_or_init(ThemeSet::load_defaults)
    }

    /// Get the theme by name.
    fn get_theme(&self, theme_name: Option<&str>) -> &'static Theme {
        let theme_set = Self::theme_set();
        let name = theme_name.unwrap_or(&self.theme_name);
        theme_set.themes.get(name).unwrap_or_else(|| {
            theme_set
                .themes
                .get("InspiredGitHub")
                .expect("Default theme missing")
        })
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
        Self::theme_set().themes.keys().cloned().collect()
    }

    fn highlight(&self, code: &str, language: &str, theme: Option<&str>) -> SyntaxResult<String> {
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

/// Create a Syntect-based syntax manager with default configuration.
///
/// Syntect provides legacy syntax highlighting using regex-based parsing.
/// Used as a fallback when Syntastica is not available.
pub fn create_syntect_manager() -> SyntaxResult<SyntaxManager> {
    let highlighter = Box::new(SyntectHighlighter::default());
    let mut config = SyntaxConfig::default();
    config.default_theme = Some("InspiredGitHub".to_string());
    Ok(SyntaxManager::new(highlighter, config))
}
