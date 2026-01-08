use std::{fmt::Write, path::PathBuf};

use ndg_config::Config;

use crate::markdown::extract_page_title;

/// Creates a fallback index page listing available documents.
///
/// This is used when no `index.md` is present. It generates a simple HTML
/// listing of all processed markdown files, using their first heading or
/// filename as the display title.
///
/// # Arguments
///
/// * `config` - The loaded configuration for documentation generation.
/// * `markdown_files` - The list of processed markdown file paths.
///
/// # Returns
///
/// A [`String`] containing the fallback HTML content.
#[must_use]
pub fn create_fallback_index(
  config: &Config,
  markdown_files: &[PathBuf],
) -> String {
  let mut content = format!(
    "<h1>{}</h1>\n<p>This is a fallback page created by ndg.</p>",
    &config.title
  );

  // Add file listing if we have an input directory
  if let Some(input_dir) = &config.input_dir
    && !markdown_files.is_empty()
  {
    let mut file_list = String::with_capacity(markdown_files.len() * 100); // preallocate based on estimated size
    file_list.push_str("<h2>Available Documents</h2>\n<ul>\n");

    for file_path in markdown_files {
      if let Ok(rel_path) = file_path.strip_prefix(input_dir) {
        // Skip included files that are not generated as HTML
        if config.included_files.contains_key(rel_path) {
          continue;
        }

        let mut html_path = rel_path.to_path_buf();
        html_path.set_extension("html");

        // Get page title from first heading or filename
        let page_title = extract_page_title(file_path, &html_path);

        // Writing to String is infallible
        let _ = writeln!(
          file_list,
          "  <li><a href=\"{}\">{}</a></li>",
          html_path.to_string_lossy(),
          page_title
        );
      }
    }

    file_list.push_str("</ul>");
    content.push_str(&file_list);
  }

  content
}
