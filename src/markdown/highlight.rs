use anyhow::{Context, Result};
use log::warn;
use std::sync::OnceLock;
use syntect::html::highlighted_html_for_string;

use crate::config::Config;

/// Cached syntax set with lazy init
fn syntax_set() -> &'static syntect::parsing::SyntaxSet {
    static SYNTAX_SET: OnceLock<syntect::parsing::SyntaxSet> = OnceLock::new();
    SYNTAX_SET.get_or_init(two_face::syntax::extra_newlines)
}

/// Cached theme set
fn theme_set() -> &'static two_face::theme::EmbeddedLazyThemeSet {
    static THEME_SET: OnceLock<two_face::theme::EmbeddedLazyThemeSet> = OnceLock::new();
    THEME_SET.get_or_init(two_face::theme::extra)
}

/// Apply syntax highlighting to codeblocks
pub fn highlight_code(code: &str, language: &str, _config: &Config) -> Result<String> {
    let syntax_set = syntax_set();

    // Try to find syntax for the specified language
    let syntax = syntax_set
        .find_syntax_by_token(language)
        .unwrap_or_else(|| {
            warn!(
                "Syntax for '{language}' not found, falling back to plain text"
            );
            syntax_set.find_syntax_plain_text()
        });

    // Get the theme
    let theme = theme_set().get(two_face::theme::EmbeddedThemeName::ColdarkDark);

    // Apply syntax highlighting
    highlighted_html_for_string(code, syntax_set, syntax, theme)
        .context("Failed to generate highlighted HTML")
}
