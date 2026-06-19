use std::path::Path;

use ndg_commonmark::utils::strip_markdown;
use rustc_hash::FxHashMap;

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
) -> FxHashMap<&'static str, String> {
  let root_prefix = calculate_root_relative_path(file_rel_path);

  let mut paths = FxHashMap::default();
  paths.reserve(6);
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

  let mut result = String::with_capacity(plain_text.len());
  let mut line = String::new();
  let mut prev_empty = false;
  let mut has_output = false;

  for raw_line in plain_text.lines() {
    line.clear();

    // Remove anchor markers {#id} and collapse whitespace.
    for word in raw_line
      .split_whitespace()
      .filter(|word| !(word.starts_with("{#") && word.ends_with('}')))
    {
      if !line.is_empty() {
        line.push(' ');
      }
      line.push_str(word);
    }

    if line.is_empty() {
      if !prev_empty && has_output {
        result.push('\n');
      }
      prev_empty = true;
    } else {
      if has_output {
        result.push('\n');
      }
      result.push_str(&line);
      has_output = true;
      prev_empty = false;
    }
  }

  result.trim().to_string()
}
