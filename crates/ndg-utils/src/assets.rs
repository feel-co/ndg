use std::{fs, path::Path};

use color_eyre::eyre::{self, Context, Result};
use log::debug;
use ndg_config::Config;
use ndg_templates as templates;
use walkdir::WalkDir;

use crate::postprocess;

const DEFAULT_CSS: &str = templates::DEFAULT_CSS;
const SEARCH_JS: &str = templates::SEARCH_JS;
const SEARCH_WORKER_JS: &str = templates::SEARCH_WORKER_JS;
const MAIN_JS: &str = templates::MAIN_JS;

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
  if config.is_search_enabled() {
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
/// Files are processed recursively:
///
/// - CSS files are minified if postprocessing is enabled
/// - JS files are minified if postprocessing is enabled
/// - Other files are copied as-is
///
/// Behavior can be configured via `assets` config:
///
/// - `follow_symlinks`: Whether to follow symbolic links (default: false)
/// - `max_depth`: Maximum directory depth to traverse (default: unlimited)
/// - `skip_hidden`: Skip hidden files/directories starting with '.' (default:
///   true)
///
/// # Errors
///
/// Returns an error if copying fails.
fn copy_custom_assets(config: &Config, assets_dir: &Path) -> eyre::Result<()> {
  let Some(custom_assets_dir) = &config.assets_dir else {
    return Ok(());
  };

  if !custom_assets_dir.exists() || !custom_assets_dir.is_dir() {
    return Ok(());
  }

  debug!("Copying custom assets from {}", custom_assets_dir.display());

  let assets_opts = config.assets.clone().unwrap_or_default();

  let mut walker =
    WalkDir::new(custom_assets_dir).follow_links(assets_opts.follow_symlinks);

  if let Some(depth) = assets_opts.max_depth {
    walker = walker.max_depth(depth);
  }

  let iter = walker.into_iter();

  let iter: Box<dyn Iterator<Item = walkdir::Result<walkdir::DirEntry>>> =
    if assets_opts.skip_hidden {
      Box::new(iter.filter_entry(|e| {
        e.file_name().to_str().is_none_or(|s| !s.starts_with('.'))
      }))
    } else {
      Box::new(iter)
    };

  for entry in iter.filter_map(std::result::Result::ok) {
    let path = entry.path();

    // Skip the root directory itself
    if path == custom_assets_dir {
      continue;
    }

    // Calculate relative path and destination
    let rel_path = path
      .strip_prefix(custom_assets_dir)
      .wrap_err("Failed to compute relative path")?;
    let dest_path = assets_dir.join(rel_path);

    let file_type = entry.file_type();

    if file_type.is_dir() {
      fs::create_dir_all(&dest_path).wrap_err_with(|| {
        format!("Failed to create directory {}", dest_path.display())
      })?;
    } else {
      #[allow(
        clippy::filetype_is_file,
        reason = "Explicitly checking for regular files to skip symlinks and \
                  special files"
      )]
      if file_type.is_file() {
        // Parent directory already created by walkdir's depth-first traversal
        process_asset_file(path, &dest_path, config)?;
      }
    }
    // Symlinks and other special files are silently skipped
  }

  Ok(())
}

/// Process a single asset file, applying minification to CSS/JS files if
/// enabled.
///
/// # Errors
///
/// Returns an error if the file cannot be read, processed, or written.
fn process_asset_file(
  path: &Path,
  dest_path: &Path,
  config: &Config,
) -> eyre::Result<()> {
  let extension = path.extension().and_then(|e| e.to_str());
  let is_css = extension.is_some_and(|e| e.eq_ignore_ascii_case("css"));
  let is_js = extension.is_some_and(|e| e.eq_ignore_ascii_case("js"));

  if is_css || is_js {
    let content = fs::read_to_string(path)
      .wrap_err_with(|| format!("Failed to read asset {}", path.display()))?;

    let processed = if let Some(ref postprocess) = config.postprocess {
      if is_css {
        postprocess::process_css(&content, postprocess)?
      } else {
        postprocess::process_js(&content, postprocess)?
      }
    } else {
      content
    };

    fs::write(dest_path, processed).wrap_err_with(|| {
      format!("Failed to write asset to {}", dest_path.display())
    })?;
  } else {
    // Binary copy for non-processable files
    fs::copy(path, dest_path).wrap_err_with(|| {
      format!(
        "Failed to copy asset from {} to {}",
        path.display(),
        dest_path.display()
      )
    })?;
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
