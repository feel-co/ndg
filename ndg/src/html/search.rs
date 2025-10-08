use std::{collections::HashMap, fs, path::PathBuf};

use color_eyre::eyre::{Context, Result};
use log::info;
use serde::Serialize;
use serde_json::Value;

use crate::{config::Config, html};

/// Search document data structure
#[derive(Debug, Serialize)]
pub struct SearchDocument {
  id:      String,
  title:   String,
  content: String,
  path:    String,
}

/// Generate search index from markdown files
pub fn generate_search_index(
  config: &Config,
  markdown_files: &[PathBuf],
) -> Result<()> {
  if !config.generate_search {
    return Ok(());
  }

  info!("Generating search index...");

  // Create search directory
  let search_dir = config.output_dir.join("assets");
  fs::create_dir_all(&search_dir)?;

  // Create search index data
  let mut documents = Vec::new();
  let mut doc_id = 0;

  // Process markdown files if available and input_dir is provided
  if !markdown_files.is_empty() && config.input_dir.is_some() {
    let input_dir = config.input_dir.as_ref().unwrap();
    for file_path in markdown_files {
      let content = fs::read_to_string(file_path).wrap_err_with(|| {
        format!(
          "Failed to read file for search indexing: {}",
          file_path.display()
        )
      })?;

      let title = extract_title(&content).unwrap_or_else(|| {
        file_path
          .file_stem()
          .unwrap_or_default()
          .to_string_lossy()
          .to_string()
      });

      // Use the existing markdown processor to handle all NDG-specific markup
      let plain_text =
        crate::utils::html::process_content_to_plain_text(&content, config);

      let rel_path = file_path.strip_prefix(input_dir).wrap_err_with(|| {
        format!(
          "Failed to determine relative path for {}",
          file_path.display()
        )
      })?;

      let mut output_path = rel_path.to_owned();
      output_path.set_extension("html");

      documents.push(SearchDocument {
        id: doc_id.to_string(),
        title,
        content: plain_text,
        path: output_path.to_string_lossy().to_string(),
      });

      doc_id += 1;
    }
  }

  // Process options if available
  if let Some(options_path) = &config.module_options {
    // Try to load options data using serde_json::Value
    if let Ok(options_content) = fs::read_to_string(options_path) {
      if let Ok(options_data) = serde_json::from_str::<Value>(&options_content)
      {
        if let Some(options_obj) = options_data.as_object() {
          for (key, option_value) in options_obj {
            // Extract description from the option
            let raw_description =
              option_value["description"].as_str().unwrap_or("");

            // Use the same clean processing as for markdown files
            let plain_description =
              crate::utils::html::process_content_to_plain_text(
                raw_description,
                config,
              );

            // Create search entry for this option
            documents.push(SearchDocument {
              id:      doc_id.to_string(),
              title:   format!("Option: {key}"),
              content: plain_description,
              path:    format!("options.html#option-{}", key.replace('.', "-")),
            });
            doc_id += 1;
          }
        }
      }
    }
  }

  // Write search index data.
  // Always create a valid JSON array, even if empty.
  let search_data_path = search_dir.join("search-data.json");
  fs::write(&search_data_path, serde_json::to_string(&documents)?)
    .wrap_err_with(|| {
      format!(
        "Failed to write search data to {}",
        search_data_path.display()
      )
    })?;

  // Create search page
  create_search_page(config)?;

  info!("Search index generated successfully");

  Ok(())
}

/// Extract title from markdown content
/// Extract title from markdown content (first H1)
fn extract_title(content: &str) -> Option<String> {
  ndg_commonmark::utils::extract_title_from_markdown(content)
}

/// Create search page
pub fn create_search_page(config: &Config) -> Result<()> {
  if !config.generate_search {
    return Ok(());
  }

  info!("Creating search page...");

  // Create context for the search page
  let mut context = HashMap::new();
  context.insert("title", format!("{} - Search", config.title));

  // Render the search page using the template
  let html = html::template::render_search(config, &context)?;

  // Write the search page to the output directory
  let search_page_path = config.output_dir.join("search.html");
  fs::write(&search_page_path, &html).wrap_err_with(|| {
    format!(
      "Failed to write search page to {}",
      search_page_path.display()
    )
  })?;

  Ok(())
}
