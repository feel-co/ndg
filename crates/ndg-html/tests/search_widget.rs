#![expect(clippy::expect_used, clippy::panic, reason = "Fine in tests")]
use std::fs;

mod common;

use common::test_config;
use ndg_config::options::{FilterConfig, OptionsConfig};
use ndg_html::{
  options::process_options,
  search::{SearchData, generate_search_index},
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
fn test_options_dot_filter_applies_to_html_and_search() {
  let temp_dir =
    TempDir::new().expect("Failed to create temp dir in options filter test");
  let output_dir = temp_dir.path();

  let options_file = output_dir.join("options.json");
  fs::write(&options_file, option_data().to_string())
    .expect("Failed to write options.json in options filter test");

  let config = ndg_config::Config {
    module_options: Some(options_file.clone()),
    options: Some(OptionsConfig {
      filter: Some(FilterConfig {
        prefix: Some("hjem.users".to_string()),
        ..Default::default()
      }),
    }),
    ..test_config(output_dir)
  };

  process_options(&config, &options_file)
    .expect("Failed to process filtered options");
  generate_search_index(&config, &[])
    .expect("Failed to generate filtered search index");

  let options_html = fs::read_to_string(output_dir.join("options.html"))
    .expect("Failed to read filtered options.html");
  assert!(options_html.contains("hjem.users.&lt;name&gt;.clobberFiles"));
  assert!(!options_html.contains("system.&lt;config&gt;.path"));

  let search_data =
    fs::read_to_string(output_dir.join("assets").join("search-data.json"))
      .expect("Failed to read filtered search-data.json");
  let search_data: SearchData = serde_json::from_str(&search_data)
    .expect("Failed to parse filtered search-data.json");
  let titles: Vec<_> = search_data
    .documents
    .iter()
    .map(|doc| doc.title.as_str())
    .collect();

  assert!(titles.iter().any(|title| title.contains("hjem.users")));
  assert!(!titles.iter().any(|title| title.contains("system.")));
}

#[test]
fn test_search_html_escape() {
  let temp_dir =
    TempDir::new().expect("Failed to create temp dir in search_widget test");
  let output_dir = temp_dir.path();

  let options_file = output_dir.join("options.json");
  fs::write(&options_file, option_data().to_string())
    .expect("Failed to write options.json in search_widget test");

  let config = ndg_config::Config {
    module_options: Some(options_file.clone()),
    ..test_config(output_dir)
  };

  process_options(&config, &options_file)
    .expect("Failed to process options in search_widget test");
  generate_search_index(&config, &[])
    .expect("Failed to generate search index in search_widget test");

  let search_data =
    fs::read_to_string(output_dir.join("assets").join("search-data.json"))
      .expect("Failed to read search-data.json in search_widget test");
  let search_data_parsed: SearchData = serde_json::from_str(&search_data)
    .expect("Failed to parse search-data.json in search_widget test");
  let search_docs: Vec<serde_json::Value> =
    serde_json::to_value(&search_data_parsed.documents)
      .expect("Failed to convert documents to Value")
      .as_array()
      .expect("Documents should be array")
      .clone();

  let options_html = fs::read_to_string(output_dir.join("options.html"))
    .expect("Failed to read options.html in search_widget test");

  for option_key in ["hjem.users.<name>.clobberFiles", "system.<config>.path"] {
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
      title.contains(option_key),
      "Search title should contain the raw option key. Expected: \
       {option_key}, Found: {title}"
    );

    let escaped_key = option_key.replace('<', "&lt;").replace('>', "&gt;");
    assert!(
      options_html.contains(&escaped_key),
      "Options HTML should contain HTML-escaped key: {escaped_key}"
    );
  }
}
