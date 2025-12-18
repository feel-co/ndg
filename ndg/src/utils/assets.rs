use std::{fs, path::Path};

use color_eyre::eyre::{self, Context, Result};
use log::debug;

use crate::{config::Config, utils::postprocess};

/// Template constants for default assets
const DEFAULT_CSS: &str = include_str!("../../templates/default.css");
const SEARCH_JS: &str = include_str!("../../templates/search.js");
const SEARCH_WORKER_JS: &str = include_str!("../../templates/search-worker.js");
const MAIN_JS: &str = include_str!("../../templates/main.js");

/// Copies all required assets (CSS, JS, custom assets, scripts) to the output
/// directory.
///
/// This includes:
/// - The main stylesheet (default or template/custom)
/// - Main JavaScript files (main.js, search.js only if enabled)
/// - Any custom assets from the configured assets directory
/// - Any custom script files specified in the configuration
///
/// Assets are postprocessed (minified) if configured via `config.postprocess`
///
/// # Errors
///
/// Returns an error if any asset cannot be read or written.
pub fn copy_assets(config: &Config) -> Result<()> {
  // Create assets directory
  let assets_dir = config.output_dir.join("assets");
  fs::create_dir_all(&assets_dir)?;

  // Generate and write CSS
  let css = generate_css(config)?;
  let processed_css = if let Some(ref postprocess) = config.postprocess {
    postprocess::process_css(&css, postprocess)?
  } else {
    css
  };
  fs::write(assets_dir.join("style.css"), processed_css)
    .context("Failed to write CSS file")?;

  // Copy main.js, which is always needed for the default templates
  copy_template_asset(config, &assets_dir, "main.js", MAIN_JS)?;

  // Copy custom assets if they exist
  copy_custom_assets(config, &assets_dir)?;

  // Copy script files to assets directory
  copy_script_files(config, &assets_dir)?;

  // Create search.js for search functionality
  if config.generate_search {
    copy_template_asset(config, &assets_dir, "search.js", SEARCH_JS)?;

    // Only copy search-worker.js if using default templates or if custom
    // template provides it
    if config.get_template_path().is_none()
      || config
        .get_template_file("search-worker.js")
        .is_some_and(|path| path.exists())
    {
      copy_template_asset(
        config,
        &assets_dir,
        "search-worker.js",
        SEARCH_WORKER_JS,
      )?;
    }
  }

  Ok(())
}

/// Copies a template asset to the assets directory, allowing user override if
/// present.
///
/// If the user has provided a custom template file for the asset, it is used.
/// Otherwise, the provided fallback content is written.
///
/// # Errors
///
/// Returns an error if the asset cannot be read or written.
fn copy_template_asset(
  config: &Config,
  assets_dir: &Path,
  filename: &str,
  fallback_content: &str,
) -> eyre::Result<()> {
  let content = if let Some(path) = config.get_template_file(filename) {
    if path.exists() {
      fs::read_to_string(&path).wrap_err({
        format!("Failed to read {} from: {}", filename, path.display())
      })?
    } else {
      fallback_content.to_string()
    }
  } else {
    fallback_content.to_string()
  };

  // Apply postprocessing based on file type
  let processed_content = if let Some(ref postprocess) = config.postprocess {
    if std::path::Path::new(filename)
      .extension()
      .is_some_and(|ext| ext.eq_ignore_ascii_case("js"))
    {
      postprocess::process_js(&content, postprocess)?
    } else {
      content
    }
  } else {
    content
  };

  fs::write(assets_dir.join(filename), processed_content)
    .wrap_err_with(|| format!("Failed to write {filename} to assets directory"))
}

/// Copies custom assets from the configured assets directory, if any, into the
/// output assets directory.
///
/// # Errors
///
/// Returns an error if copying fails.
fn copy_custom_assets(config: &Config, assets_dir: &Path) -> eyre::Result<()> {
  if let Some(custom_assets_dir) = &config.assets_dir {
    if custom_assets_dir.exists() && custom_assets_dir.is_dir() {
      debug!("Copying custom assets from {}", custom_assets_dir.display());

      let options = fs_extra::dir::CopyOptions::new().overwrite(true);
      fs_extra::dir::copy(custom_assets_dir, assets_dir, &options)
        .wrap_err("Failed to copy custom assets")?;
    }
  }
  Ok(())
}

/// Copies custom script files to the assets directory.
///
/// # Errors
///
/// Returns an error if any script file cannot be read or written.
fn copy_script_files(config: &Config, assets_dir: &Path) -> eyre::Result<()> {
  for script_path in &config.script_paths {
    if script_path.exists() {
      let file_name = script_path
        .file_name()
        .ok_or_else(|| eyre::eyre!("Invalid script filename"))?;
      let dest_path = assets_dir.join(file_name);

      let content = fs::read_to_string(script_path).wrap_err_with(|| {
        format!("Failed to read script file {}", script_path.display())
      })?;

      // Apply postprocessing if enabled
      let processed_content = if let Some(ref postprocess) = config.postprocess
      {
        postprocess::process_js(&content, postprocess)?
      } else {
        content
      };

      fs::write(&dest_path, processed_content).wrap_err_with(|| {
        format!("Failed to write script file to {}", dest_path.display())
      })?;
    }
  }
  Ok(())
}

/// Generates the combined CSS for the documentation output.
///
/// This includes:
/// - The default or template CSS
/// - Any custom stylesheets specified in the configuration (including SCSS
///   processing)
///
/// # Errors
///
/// Returns an error if any stylesheet cannot be read or processed.
fn generate_css(config: &Config) -> eyre::Result<String> {
  // Use template CSS if available, otherwise use default CSS
  let mut combined_css = if let Some(template_path) = config.get_template_path()
  {
    let template_css_path = template_path.join("default.css");
    if template_css_path.exists() {
      fs::read_to_string(&template_css_path).wrap_err({
        format!(
          "Failed to read template CSS: {}",
          template_css_path.display()
        )
      })?
    } else {
      // No template CSS found, use default
      String::from(DEFAULT_CSS)
    }
  } else {
    // No template directory specified, use default CSS
    String::from(DEFAULT_CSS)
  };

  // Add any custom stylesheets provided via --stylesheet
  if !config.stylesheet_paths.is_empty() {
    // Process each stylesheet in order
    for (index, stylesheet_path) in config.stylesheet_paths.iter().enumerate() {
      if stylesheet_path.exists() {
        let content = fs::read_to_string(stylesheet_path).wrap_err({
          format!(
            "Failed to read stylesheet {}: {}",
            index + 1,
            stylesheet_path.display()
          )
        })?;

        // Process SCSS if needed
        let processed_content =
          if stylesheet_path.extension().is_some_and(|ext| ext == "scss") {
            grass::from_string(content.clone(), &grass::Options::default())
              .wrap_err({
                format!(
                  "Failed to compile SCSS to CSS for stylesheet {}",
                  index + 1
                )
              })?
          } else {
            content
          };

        // Add a comment to separate multiple stylesheets
        combined_css.push_str("\n\n/* Custom Stylesheet ");
        combined_css.push_str(&(index + 1).to_string());
        combined_css.push_str(": ");
        combined_css.push_str(&stylesheet_path.display().to_string());
        combined_css.push_str(" */\n");
        combined_css.push_str(&processed_content);
      }
    }
  }

  Ok(combined_css)
}
