use std::{fs, path::PathBuf};

use color_eyre::eyre::{Context, Result, bail};
use log::info;
use ndg_config::Config;
use ndg_nixdoc::NixDocEntry;

use crate::{libdoc, search, template};

/// Process nixdoc inputs and write `lib.html`.
///
/// Returns the rendered HTML as a [`search::ProcessedDocument`] so the caller
/// can include the lib page in the search index, or `None` when there are no
/// nixdoc inputs or no entries were found.
pub fn process_nixdoc(
  config: &Config,
) -> Result<Option<search::ProcessedDocument>> {
  if config.nixdoc_inputs.is_empty() {
    info!("No nixdoc inputs configured, skipping lib generation");
    return Ok(None);
  }

  info!("Processing nixdoc inputs...");
  let mut all_entries: Vec<NixDocEntry> = Vec::new();
  let mut error_count = 0usize;

  for input in &config.nixdoc_inputs {
    if input.is_dir() {
      match ndg_nixdoc::extract_from_dir(input) {
        Ok(entries) => all_entries.extend(entries),
        Err(e) => {
          log::warn!("Failed to extract nixdoc from {}: {e}", input.display());
          error_count += 1;
        },
      }
    } else {
      match ndg_nixdoc::extract_from_file(input) {
        Ok(entries) => all_entries.extend(entries),
        Err(e) => {
          log::warn!("Failed to extract nixdoc from {}: {e}", input.display());
          error_count += 1;
        },
      }
    }
  }

  if error_count > 0 && error_count == config.nixdoc_inputs.len() {
    bail!(
      "All {} nixdoc input(s) failed to parse",
      config.nixdoc_inputs.len()
    );
  }

  if all_entries.is_empty() {
    log::warn!("No nixdoc entries found in configured inputs");
    return Ok(None);
  }

  let entries_html =
    libdoc::generate_lib_entries_html(&all_entries, &config.revision);
  let toc_html = libdoc::generate_lib_toc_html(&all_entries);
  let lib_html = template::render_lib(config, &entries_html, &toc_html)
    .wrap_err("Failed to render lib page")?;

  let lib_path = config.output_dir.join("lib.html");
  fs::write(&lib_path, &lib_html).wrap_err_with(|| {
    format!("Failed to write lib.html to {}", lib_path.display())
  })?;

  info!("Generated lib.html with {} entries", all_entries.len());

  // Return a ProcessedDocument so the caller can include lib.html in the
  // search index alongside markdown-derived pages.
  let doc = search::ProcessedDocument {
    source_path:  PathBuf::from("lib.html"),
    html_content: lib_html,
    headers:      vec![],
    title:        Some(format!("{} Library Reference", config.title)),
    html_path:    String::from("lib.html"),
  };
  Ok(Some(doc))
}
