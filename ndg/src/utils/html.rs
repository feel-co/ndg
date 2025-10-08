use std::{collections::HashMap, path::Path};

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

/// Strip markdown to get plain text
#[must_use]
pub fn strip_markdown(content: &str) -> String {
  ndg_commonmark::utils::strip_markdown(content)
}

/// Process content through the markdown pipeline and extract plain text
#[must_use]
pub fn process_content_to_plain_text(
  content: &str,
  config: &crate::config::Config,
) -> String {
  let processor = crate::utils::create_processor_from_config(config);
  let result = processor.render(content);
  strip_markdown(&result.html)
    .replace('\n', " ")
    .replace("  ", " ")
    .trim()
    .to_string()
}
