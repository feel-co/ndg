pub mod assets;
pub mod html;
pub mod json;
pub mod markdown;
pub mod output;
pub mod postprocess;

use std::fs;

use color_eyre::eyre::{self, Context};
use log::info;

#[allow(unused_imports, reason = "Re-exported for backward compatibility")]
pub use crate::utils::{
  assets::copy_assets,
  markdown::{
    collect_included_files,
    create_processor,
    process_markdown_files,
  },
  output::{create_fallback_index, process_module_options},
};
use crate::{config::Config, html::template};

/// Ensure that index.html exists in the output directory
/// Ensure that index.html exists in the output directory
///
/// # Errors
///
/// Returns an error if index.html or options.html cannot be read or written.
pub fn ensure_index(
  config: &Config,
  options_processed: bool,
  markdown_files: &[std::path::PathBuf],
) -> eyre::Result<()> {
  let index_path = config.output_dir.join("index.html");

  // Check if index.html already exists (already generated from index.md)
  if index_path.exists() {
    return Ok(());
  }

  // If options were processed, try to use options.html as index
  if options_processed {
    let options_path = config.output_dir.join("options.html");
    if options_path.exists() {
      info!("Using options.html as index.html");

      // Copy the content of options.html to index.html
      let options_content = fs::read(&options_path).wrap_err({
        format!("Failed to read options.html: {}", options_path.display())
      })?;
      fs::write(&index_path, options_content).wrap_err({
        format!("Failed to write index.html: {}", index_path.display())
      })?;
      return Ok(());
    }
  }

  // Create a fallback index page if input directory was provided but no
  // index.md existed
  if config.input_dir.is_some() {
    info!("No index.md found, creating fallback index.html");

    let content = create_fallback_index(config, markdown_files);

    // Create a simple HTML document using our template system
    let headers = vec![ndg_commonmark::Header {
      text:  config.title.clone(),
      level: 1,
      id:    "welcome".to_string(),
    }];

    let html = template::render(
      config,
      &content,
      &config.title,
      &headers,
      &std::path::PathBuf::from("index.html"),
    )?;

    // Apply postprocessing if requested
    let processed_html = if let Some(ref postprocess) = config.postprocess {
      crate::utils::postprocess::process_html(&html, postprocess)?
    } else {
      html
    };

    fs::write(&index_path, processed_html).wrap_err({
      format!(
        "Failed to write fallback index.html: {}",
        index_path.display()
      )
    })?;
  }

  Ok(())
}
