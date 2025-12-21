#![allow(clippy::expect_used, clippy::panic, reason = "Fine in tests")]
use std::fs;

use ndg::{
  config::{Config, search::SearchConfig},
  formatter::options::process_options,
  html::search::generate_search_index,
};
use serde_json::json;
use tempfile::TempDir;

// Mock an options.json
// This doesn't have to be accurate, but it has to demonstrate a good enough
// reference for testing. <name>, for example, is important for knowing
// whether we've regressed the stupid HTML escaper again...
fn option_data() -> serde_json::Value {
  json!({
      "hjem.users.<name>.clobberFiles": {
          "type": "boolean",
          "description": "Files to clobber when creating home directory",
          "default": false
      },
      "system.<config>.path": {
          "type": "string",
          "description": "System configuration path",
          "default": "/etc/config"
      }
  })
}

#[test]
fn test_search_html_escape() {
  let temp_dir =
    TempDir::new().expect("Failed to create temp dir in search_widget test");
  let output_dir = temp_dir.path();

  let options_file = output_dir.join("options.json");
  fs::write(&options_file, option_data().to_string())
    .expect("Failed to write options.json in search_widget test");

  let config = Config {
    output_dir: output_dir.to_path_buf(),
    module_options: Some(options_file.clone()),
    title: "Test".to_string(),
    search: Some(SearchConfig {
      enable: true,
      ..Default::default()
    }),
    ..Default::default()
  };

  process_options(&config, &options_file)
    .expect("Failed to process options in search_widget test");
  generate_search_index(&config, &[])
    .expect("Failed to generate search index in search_widget test");

  let search_data =
    fs::read_to_string(output_dir.join("assets").join("search-data.json"))
      .expect("Failed to read search-data.json in search_widget test");
  let search_docs: Vec<serde_json::Value> = serde_json::from_str(&search_data)
    .expect("Failed to parse search-data.json in search_widget test");

  let options_html = fs::read_to_string(output_dir.join("options.html"))
    .expect("Failed to read options.html in search_widget test");

  for option_key in ["hjem.users.<name>.clobberFiles", "system.<config>.path"] {
    let escaped_key = option_key.replace('<', "&lt;").replace('>', "&gt;");

    let search_entry = search_docs
      .iter()
      .find(|doc| {
        doc
          .get("title")
          .and_then(|v| v.as_str())
          .is_some_and(|title| {
            title.contains(
              option_key
                .split('.')
                .next()
                .expect("option_key should have at least one segment"),
            )
          })
      })
      .unwrap_or_else(|| panic!("Search entry not found for {option_key}"));

    let title = search_entry
      .get("title")
      .expect("Search entry missing title field")
      .as_str()
      .expect("Search entry title is not a string");
    assert!(
      title.contains(&escaped_key),
      "Search title should contain HTML-escaped key. Expected: {escaped_key}, \
       Found: {title}"
    );

    assert!(
      options_html.contains(&escaped_key),
      "Options HTML should contain HTML-escaped key: {escaped_key}"
    );
  }
}
