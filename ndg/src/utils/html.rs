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

/// Process content through the markdown pipeline and extract plain text.
///
/// Converts markdown to plain text while preserving document structure:
///
/// - Headings and paragraphs are separated by newlines
/// - Multiple consecutive blank lines are collapsed to single blank lines
/// - Whitespace within lines is normalized to single spaces
/// - Anchor markers like {#id} are removed
#[must_use]
pub fn content_to_plaintext(content: &str) -> String {
  let plain_text = strip_markdown(content);

  // Normalize whitespace: collapse multiple blank lines and trim lines
  let lines: Vec<String> = plain_text
    .lines()
    .map(|line| {
      // Remove anchor markers {#id} and collapse whitespace
      line
        .split_whitespace()
        .filter(|word| !(word.starts_with("{#") && word.ends_with('}')))
        .collect::<Vec<_>>()
        .join(" ")
    })
    .collect();

  // Remove consecutive empty lines
  let mut result = Vec::new();
  let mut prev_empty = false;

  for line in lines {
    if line.is_empty() {
      if !prev_empty && !result.is_empty() {
        result.push(line);
      }
      prev_empty = true;
    } else {
      result.push(line);
      prev_empty = false;
    }
  }

  result.join("\n").trim().to_string()
}
