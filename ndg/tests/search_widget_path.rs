#![allow(clippy::expect_used, reason = "Fine in tests")]
use std::{fs, path::PathBuf};

use ndg::{
  config::Config,
  formatter::options::process_options,
  html::{search::generate_search_index, template::render},
};
use serde_json::json;
use tempfile::TempDir;

#[test]
fn test_search_widget_path_resolution() {
  let temp_dir = TempDir::new().expect("Failed to create temp dir");
  let output_dir = temp_dir.path();

  let nested_dir = output_dir.join("docs");
  fs::create_dir_all(&nested_dir).expect("Failed to create nested directory");

  let options_file = output_dir.join("options.json");
  let options_data = json!({
      "test.option": {
          "type": "string",
          "description": "A test option for search widget",
          "default": "default"
      }
  });

  #[allow(clippy::expect_used, reason = "Fine in tests")]
  fs::write(&options_file, options_data.to_string())
    .expect("Failed to write options.json");

  let config = Config {
    output_dir: output_dir.to_path_buf(),
    module_options: Some(options_file.clone()),
    title: "Test".to_string(),
    generate_search: true,
    ..Default::default()
  };

  // Process options and generate search
  process_options(&config, &options_file).expect("Failed to process options");
  generate_search_index(&config, &[]).expect("Failed to generate search index");

  // Render a page in a subdirectory. This will have the search widget
  // so we can see if it has regressed (again).
  let nested_path = PathBuf::from("docs/test.html");
  let html = render(
    &config,
    "<p>Test content</p>",
    "Test Page",
    &[],
    &nested_path,
  )
  .expect("Failed to render page");

  // Verify that search widget is present
  assert!(
    html.contains(r#"<input type="text" id="search-input""#),
    "Page should contain search widget input"
  );

  // Verify that root_prefix is correctly set for subdirectory
  assert!(
    html.contains("window.searchNamespace.rootPath = \"../\";"),
    "HTML should contain root path prefix for search widget: {html}"
  );

  // Verify search script is included
  assert!(
    html.contains("search.js"),
    "Page should include search script for widget"
  );

  // Also verify search data exists for the widget to use
  let search_data =
    fs::read_to_string(output_dir.join("assets").join("search-data.json"))
      .expect("Failed to read search-data.json");
  assert!(
    search_data.contains("test.option"),
    "Search data should contain test option for widget"
  );
}

#[test]
fn test_search_widget_at_root_level() {
  let temp_dir = TempDir::new().expect("Failed to create temp dir");
  let output_dir = temp_dir.path();

  let options_file = output_dir.join("options.json");
  let options_data = json!({
      "root.option": {
          "type": "string",
          "description": "A root level option for search widget",
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

  let root_path = PathBuf::from("test.html");
  let html =
    render(&config, "<p>Test content</p>", "Test Page", &[], &root_path)
      .expect("Failed to render page");

  // Verify that search widget is present
  assert!(
    html.contains(r#"<input type="text" id="search-input""#),
    "Page should contain search widget input"
  );

  // Verify that root_prefix is empty for root level
  assert!(
    html.contains("window.searchNamespace.rootPath = \"\";"),
    "HTML should contain empty root path for search widget at root: {html}"
  );

  // Verify search script is included
  assert!(
    html.contains("search.js"),
    "Page should include search script for widget"
  );
}
