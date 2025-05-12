use std::sync::OnceLock;

use anyhow::{Context, Result};
use log::warn;
use syntect::{
    highlighting::{Theme, ThemeSet},
    html::highlighted_html_for_string,
};

use crate::config::Config;

/// Cached syntax set with lazy init
fn syntax_set() -> &'static syntect::parsing::SyntaxSet {
    static SYNTAX_SET: OnceLock<syntect::parsing::SyntaxSet> = OnceLock::new();
    SYNTAX_SET.get_or_init(two_face::syntax::extra_newlines)
}

/// Cached theme with no background
fn modified_theme() -> &'static Theme {
    static THEME: OnceLock<Theme> = OnceLock::new();
    THEME.get_or_init(|| {
        let mut themes = ThemeSet::load_defaults();
        let mut theme = themes
            .themes
            .remove("InspiredGitHub")
            .expect("Theme not found");

        theme.settings.background = None;
        theme
    })
}

/// Apply syntax highlighting to codeblocks
pub fn highlight_code(code: &str, language: &str, config: &Config) -> Result<String> {
    // Skip highlighting if it's disabled in config
    if !config.highlight_code {
        return Err(anyhow::anyhow!("Syntax highlighting is disabled"));
    }

    let syntax_set = syntax_set();

    let syntax = syntax_set
        .find_syntax_by_token(language)
        .unwrap_or_else(|| {
            warn!("Syntax for '{language}' not found, falling back to plain text");
            syntax_set.find_syntax_plain_text()
        });

    let theme = modified_theme();

    highlighted_html_for_string(code, syntax_set, syntax, theme)
        .context("Failed to generate highlighted HTML")
}
