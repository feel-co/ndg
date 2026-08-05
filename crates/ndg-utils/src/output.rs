use std::{
  fmt::Write,
  path::{Component, Path, PathBuf},
};

use color_eyre::eyre::{Result, bail};
use ndg_config::Config;

use crate::markdown::extract_page_title;

/// Join an output-relative path to an output directory.
///
/// # Errors
///
/// Returns an error when `relative` is absolute or escapes its output root.
pub fn output_path(output_dir: &Path, relative: &Path) -> Result<PathBuf> {
  if relative.components().any(|component| {
    matches!(
      component,
      Component::ParentDir | Component::RootDir | Component::Prefix(_)
    )
  }) {
    bail!(
      "output path must be relative and must not contain parent components: {}",
      relative.display()
    );
  }

  Ok(output_dir.join(relative))
}

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
          html_escape::encode_double_quoted_attribute(
            &html_path.to_string_lossy()
          ),
          html_escape::encode_text(&page_title)
        );
      }
    }

    file_list.push_str("</ul>");
    content.push_str(&file_list);
  }

  content
}

#[cfg(test)]
#[expect(clippy::expect_used, reason = "Fine in tests")]
mod tests {
  use std::path::Path;

  use super::output_path;

  #[test]
  fn output_path_rejects_paths_that_escape_the_output_directory() {
    assert!(
      output_path(Path::new("site"), Path::new("../outside.html")).is_err()
    );
    assert!(
      output_path(Path::new("site"), Path::new("/outside.html")).is_err()
    );
  }

  #[test]
  fn output_path_accepts_nested_relative_paths() {
    assert_eq!(
      output_path(Path::new("site"), Path::new("nested/page.html"))
        .expect("nested relative path is valid"),
      Path::new("site/nested/page.html")
    );
  }
}
