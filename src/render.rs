use anyhow::{Context, Result};
use log::debug;
use std::fs;
use std::path::Path;

use crate::config::Config;

// Template constants
const DEFAULT_CSS: &str = include_str!("../templates/default.css");
const SEARCH_JS: &str = include_str!("../templates/search.js");

/// Copy assets to output directory
pub fn copy_assets(config: &Config) -> Result<()> {
    // Create assets directory
    let assets_dir = config.output_dir.join("assets");
    fs::create_dir_all(&assets_dir)?;

    // Generate and write CSS
    let css = generate_css(config)?;
    fs::write(assets_dir.join("style.css"), css).context("Failed to write CSS file")?;

    // Copy custom assets if they exist
    copy_custom_assets(config, &assets_dir)?;

    // Copy script files to assets directory
    copy_script_files(config, &assets_dir)?;

    // Create search.js for search functionality
    if config.generate_search {
        // Use template from config if available, otherwise use embedded template
        let search_js = if let Some(template_path) = config.get_template_path() {
            let search_js_path = template_path.join("search.js");
            if search_js_path.exists() {
                fs::read_to_string(&search_js_path).with_context(|| {
                    format!(
                        "Failed to read search.js from: {}",
                        search_js_path.display()
                    )
                })?
            } else {
                SEARCH_JS.to_string()
            }
        } else {
            SEARCH_JS.to_string()
        };

        fs::write(assets_dir.join("search.js"), search_js).context("Failed to write search.js")?;
    }

    Ok(())
}

/// Copy custom assets from the configured directory
fn copy_custom_assets(config: &Config, assets_dir: &Path) -> Result<()> {
    if let Some(custom_assets_dir) = &config.assets_dir {
        if custom_assets_dir.exists() && custom_assets_dir.is_dir() {
            debug!("Copying custom assets from {}", custom_assets_dir.display());

            let options = fs_extra::dir::CopyOptions::new().overwrite(true);
            fs_extra::dir::copy(custom_assets_dir, assets_dir, &options)
                .context("Failed to copy custom assets")?;
        }
    }
    Ok(())
}

/// Copy script files to the assets directory
fn copy_script_files(config: &Config, assets_dir: &Path) -> Result<()> {
    for script_path in &config.script_paths {
        if script_path.exists() {
            let file_name = script_path
                .file_name()
                .ok_or_else(|| anyhow::anyhow!("Invalid script filename"))?;
            let dest_path = assets_dir.join(file_name);

            fs::read(script_path)
                .with_context(|| format!("Failed to read script file {}", script_path.display()))
                .and_then(|content| {
                    fs::write(&dest_path, content).with_context(|| {
                        format!("Failed to write script file to {}", dest_path.display())
                    })
                })?;
        }
    }
    Ok(())
}

/// Generate CSS from stylesheet
fn generate_css(config: &Config) -> Result<String> {
    // Use custom stylesheet if provided, otherwise use default
    if let Some(stylesheet_path) = &config.stylesheet_path {
        if !stylesheet_path.exists() {
            // Check if we have a template directory with default.css
            if let Some(template_path) = config.get_template_path() {
                let template_css_path = template_path.join("default.css");
                if template_css_path.exists() {
                    let content = fs::read_to_string(&template_css_path).with_context(|| {
                        format!(
                            "Failed to read template CSS: {}",
                            template_css_path.display()
                        )
                    })?;
                    return Ok(content);
                }
            }
            // Fall back to embedded default
            return Ok(DEFAULT_CSS.to_string());
        }

        let content = fs::read_to_string(stylesheet_path)
            .with_context(|| format!("Failed to read stylesheet: {}", stylesheet_path.display()))?;

        // Process SCSS if needed
        if stylesheet_path.extension().is_some_and(|ext| ext == "scss") {
            grass::from_string(content, &grass::Options::default())
                .context("Failed to compile SCSS to CSS")
        } else {
            Ok(content)
        }
    } else {
        // Check if we have a template directory with default.css
        if let Some(template_path) = config.get_template_path() {
            let template_css_path = template_path.join("default.css");
            if template_css_path.exists() {
                let content = fs::read_to_string(&template_css_path).with_context(|| {
                    format!(
                        "Failed to read template CSS: {}",
                        template_css_path.display()
                    )
                })?;
                return Ok(content);
            }
        }
        // Fall back to embedded default
        Ok(DEFAULT_CSS.to_string())
    }
}
