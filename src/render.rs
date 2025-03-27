use anyhow::{Context, Result};
use log::debug;
use std::fs;

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

    // Copy script files to assets directory
    if !config.script_paths.is_empty() {
        for script_path in &config.script_paths {
            if script_path.exists() {
                let file_name = script_path.file_name().unwrap_or_default();
                let dest_path = assets_dir.join(file_name);

                // Read the content and write it rather than copying directly
                // NOTE: This is to work around a weird file permission bug
                // that I've experienced, and I'm not sure how necessary it
                // is. Either way its performance impact is negligible, so
                // I'm keeping it.
                match fs::read(script_path) {
                    Ok(content) => {
                        if let Err(e) = fs::write(&dest_path, content) {
                            return Err(anyhow::anyhow!(
                                "Failed to write script file to {}: {}",
                                dest_path.display(),
                                e
                            ));
                        }
                    }
                    Err(e) => {
                        return Err(anyhow::anyhow!(
                            "Failed to read script file {}: {}",
                            script_path.display(),
                            e
                        ));
                    }
                }
            }
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
            if stylesheet_path.extension().is_some_and(|ext| ext == "scss") {
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
