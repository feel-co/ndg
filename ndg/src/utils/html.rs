use std::{collections::HashMap, path::Path};

use ndg_commonmark::utils::strip_markdown;

/// Calculate the relative path prefix needed to reach the root from a given
/// file path For example: "docs/subdir/file.html" would return "../"
///              "docs/subdir/nested/file.html" would return "../../"
#[must_use]
pub fn calculate_root_relative_path(file_rel_path: &Path) -> String {
  let depth = file_rel_path.components().count();
  if depth <= 1 {
    String::new() // file is at root level
  } else {
    "../".repeat(depth - 1)
  }
}

/// Generate proper asset paths for templates based on file location
#[must_use]
pub fn generate_asset_paths(
  file_rel_path: &Path,
) -> HashMap<&'static str, String> {
  let root_prefix = calculate_root_relative_path(file_rel_path);

  let mut paths = HashMap::new();
  paths.insert("stylesheet_path", format!("{root_prefix}assets/style.css"));
  paths.insert("main_js_path", format!("{root_prefix}assets/main.js"));
  paths.insert("search_js_path", format!("{root_prefix}assets/search.js"));

  // Navigation paths
  paths.insert("index_path", format!("{root_prefix}index.html"));
  paths.insert("options_path", format!("{root_prefix}options.html"));
  paths.insert("search_path", format!("{root_prefix}search.html"));

  paths
}

/// Process content through the markdown pipeline and extract plain text
#[must_use]
pub fn content_to_plaintext(content: &str) -> String {
  // For search indexing, we want plain text from markdown, not full HTML with
  // templates Use the basic strip_markdown function which processes markdown
  // AST directly
  let plain_text = strip_markdown(content);

  // Clean up whitespace while preserving readability
  plain_text
    .split_whitespace()
    .collect::<Vec<_>>()
    .join(" ")
    .trim()
    .to_string()
}
