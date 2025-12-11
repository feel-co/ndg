#![allow(clippy::expect_used, reason = "Fine in tests")]
use std::{fs, path::PathBuf};

use ndg::{
  config::Config,
  formatter::options::process_options,
  html::{search::generate_search_index, template::render},
};
use serde_json::json;
use tempfile::TempDir;

/// Test that search widget paths are resolved correctly from subdirectories
#[test]
fn test_search_path_resolution_from_subdirectory() {
  let temp_dir = TempDir::new().expect("Failed to create temp dir");
  let output_dir = temp_dir.path();

  // Create a nested directory structure
  let nested_dir = output_dir.join("docs");
  fs::create_dir_all(&nested_dir).expect("Failed to create nested directory");

  let options_file = output_dir.join("options.json");
  let options_data = json!({
      "test.option": {
          "type": "string",
          "description": "A test option",
          "default": "default"
      }
  });
  fs::write(&options_file, options_data.to_string())
    .expect("Failed to write options.json");

  let config = Config {
    output_dir: output_dir.to_path_buf(),
    module_options: Some(options_file.clone()),
    title: "Test".to_string(),
    generate_search: true,
    ..Default::default()
  };

  process_options(&config, &options_file).expect("Failed to process options");
  generate_search_index(&config, &[]).expect("Failed to generate search index");

  // Render a page in a subdirectory
  let nested_path = PathBuf::from("docs/test.html");
  let html = render(
    &config,
    "<p>Test content</p>",
    "Test Page",
    &[],
    &nested_path,
  )
  .expect("Failed to render page");

  // Verify that root_prefix is correctly set for subdirectory
  assert!(
    html.contains("window.searchNamespace.rootPath = \"../\";"),
    "HTML should contain root path prefix for subdirectory: {html}"
  );

  // Verify that search data is generated correctly
  let search_data =
    fs::read_to_string(output_dir.join("assets").join("search-data.json"))
      .expect("Failed to read search-data.json");
  assert!(
    search_data.contains("test.option"),
    "Search data should contain test option"
  );
}

#[test]
fn test_search_path_resolution_from_root() {
  let temp_dir = TempDir::new().expect("Failed to create temp dir");
  let output_dir = temp_dir.path();

  let options_file = output_dir.join("options.json");
  let options_data = json!({
      "root.option": {
          "type": "string",
          "description": "A root level option",
          "default": "default"
      }
  });
  fs::write(&options_file, options_data.to_string())
    .expect("Failed to write options.json");

  let config = Config {
    output_dir: output_dir.to_path_buf(),
    module_options: Some(options_file.clone()),
    title: "Test".to_string(),
    generate_search: true,
    ..Default::default()
  };

  process_options(&config, &options_file).expect("Failed to process options");
  generate_search_index(&config, &[]).expect("Failed to generate search index");

  // Render a page at root level
  let root_path = PathBuf::from("test.html");
  let html =
    render(&config, "<p>Test content</p>", "Test Page", &[], &root_path)
      .expect("Failed to render page");

  // Verify that root_prefix is empty for root level
  assert!(
    html.contains("window.searchNamespace.rootPath = \"\";"),
    "HTML should contain empty root path for root level: {html}"
  );

  // Verify that search data is generated correctly
  let search_data =
    fs::read_to_string(output_dir.join("assets").join("search-data.json"))
      .expect("Failed to read search-data.json");
  assert!(
    search_data.contains("root.option"),
    "Search data should contain root option"
  );
}
