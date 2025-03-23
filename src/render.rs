use anyhow::{Context, Result};
use log::debug;
use std::fs;
use syntect::highlighting::ThemeSet;
use syntect::html::highlighted_html_for_string;
use syntect::parsing::SyntaxSet;

use crate::config::Config;

/// Copy assets to output directory
pub fn copy_assets(config: &Config) -> Result<()> {
    // Create assets directory
    let assets_dir = config.output_dir.join("assets");
    fs::create_dir_all(&assets_dir)?;

    // Generate CSS
    let css = generate_css(config)?;
    fs::write(assets_dir.join("style.css"), css).context("Failed to write CSS file")?;

    // Copy custom assets if they exist
    if let Some(custom_assets_dir) = &config.assets_dir {
        if custom_assets_dir.exists() && custom_assets_dir.is_dir() {
            debug!("Copying custom assets from {}", custom_assets_dir.display());

            let options = fs_extra::dir::CopyOptions::new().overwrite(true);

            fs_extra::dir::copy(custom_assets_dir, &assets_dir, &options)
                .context("Failed to copy custom assets")?;
        }
    }

    // Create search.js for search functionality
    if config.generate_search {
        fs::write(
            assets_dir.join("search.js"),
            include_str!("../templates/search.js"),
        )
        .context("Failed to write search.js")?;
    }

    Ok(())
}

/// Generate CSS from stylesheet
fn generate_css(config: &Config) -> Result<String> {
    // Use custom stylesheet if provided, otherwise use default
    let css = if let Some(stylesheet_path) = &config.stylesheet_path {
        if stylesheet_path.exists() {
            let content = fs::read_to_string(stylesheet_path).context(format!(
                "Failed to read stylesheet: {}",
                stylesheet_path.display()
            ))?;

            // Process SCSS if needed
            if stylesheet_path
                .extension()
                .map_or(false, |ext| ext == "scss")
            {
                grass::from_string(content, &grass::Options::default())
                    .context("Failed to compile SCSS to CSS")?
            } else {
                content
            }
        } else {
            include_str!("../templates/default.css").to_string()
        }
    } else {
        include_str!("../templates/default.css").to_string()
    };

    Ok(css)
}

/// Highlight code block with syntax highlighting
pub fn highlight_code(code: &str, language: &str) -> Result<String> {
    let syntax_set = SyntaxSet::load_defaults_newlines();
    let theme_set = ThemeSet::load_defaults();

    let syntax = syntax_set
        .find_syntax_by_token(language)
        .unwrap_or_else(|| syntax_set.find_syntax_plain_text());

    let theme = &theme_set.themes["base16-ocean-dark"];

    let html = highlighted_html_for_string(code, &syntax_set, syntax, theme)
        .context("Failed to highlight code")?;

    Ok(html)
}
