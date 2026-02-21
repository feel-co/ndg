use std::fs;

use color_eyre::eyre::{Context, Result};
use log::info;
use ndg_config::Config;
use ndg_nixdoc::NixDocEntry;

use crate::{libdoc, template};

pub fn process_nixdoc(config: &Config) -> Result<bool> {
  if config.nixdoc_inputs.is_empty() {
    info!("No nixdoc inputs configured, skipping lib generation");
    return Ok(false);
  }

  info!("Processing nixdoc inputs...");
  let mut all_entries: Vec<NixDocEntry> = Vec::new();

  for input in &config.nixdoc_inputs {
    if input.is_dir() {
      match ndg_nixdoc::extract_from_dir(input) {
        Ok(entries) => all_entries.extend(entries),
        Err(e) => {
          log::warn!("Failed to extract nixdoc from {}: {e}", input.display())
        },
      }
    } else {
      match ndg_nixdoc::extract_from_file(input) {
        Ok(entries) => all_entries.extend(entries),
        Err(e) => {
          log::warn!("Failed to extract nixdoc from {}: {e}", input.display())
        },
      }
    }
  }

  if all_entries.is_empty() {
    log::warn!("No nixdoc entries found in configured inputs");
    return Ok(false);
  }

  let entries_html =
    libdoc::generate_lib_entries_html(&all_entries, &config.revision);
  let toc_html = libdoc::generate_lib_toc_html(&all_entries);
  let lib_html = template::render_lib(config, &entries_html, &toc_html)
    .wrap_err("Failed to render lib page")?;

  let lib_path = config.output_dir.join("lib.html");
  fs::write(&lib_path, lib_html).wrap_err_with(|| {
    format!("Failed to write lib.html to {}", lib_path.display())
  })?;

  info!("Generated lib.html with {} entries", all_entries.len());
  Ok(true)
}
