#![allow(clippy::expect_used, reason = "Fine in tests")]
use std::{
  fs::{self, File, create_dir_all},
  path::PathBuf,
};

use ndg::{
  config::Config,
  formatter::options::process_options,
  html::{
    search::{SearchDocument, generate_search_index},
    template::render,
  },
  utils::{collect_included_files, create_processor},
};
use ndg_commonmark::collect_markdown_files;
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

#[test]
fn test_search_path_resolution_of_included_file() {
  let temp_dir = TempDir::new().expect("Failed to create temp dir");
  let input_dir = temp_dir.path().join("input");
  let included_dir = input_dir.join("included");
  let output_dir = temp_dir.path().join("output");

  create_dir_all(&included_dir).expect("failed to create input dir");
  create_dir_all(&output_dir).expect("failed to create output dir");

  let main_content = "# Main file

Some content.

```{=include=}
included/file.md
included/section_no_id.md
```
";
  fs::write(input_dir.join("main.md"), main_content)
    .expect("Failed to write options.json");

  let included_content = "# Included file {#included-file-heading}

Some included content.
";
  fs::write(included_dir.join("file.md"), included_content)
    .expect("Failed to write included/file.md");

  let no_anchor_id_content = "# Section without an anchor ID

Some text.
";
  fs::write(included_dir.join("section_no_id.md"), no_anchor_id_content)
    .expect("Failed to write section_no_id.md");

  let mut config = Config {
    input_dir: Some(input_dir.clone()),
    output_dir: output_dir.clone(),
    module_options: None,
    title: "Test".to_string(),
    generate_search: true,
    ..Default::default()
  };

  let processor = Some(create_processor(&config, None));
  config.included_files = collect_included_files(&config, processor.as_ref())
    .expect("Failed to collect include files");

  let markdown_files = collect_markdown_files(&input_dir);
  generate_search_index(&config, &markdown_files)
    .expect("Failed to generate search index");

  // Verify that search data is generated correctly
  let index_file =
    File::open(output_dir.join("assets").join("search-data.json"))
      .expect("Failed to open search-data.json");
  let search_data: Vec<SearchDocument> =
    serde_json::from_reader(index_file).expect("Failed to read index data");
  let included_doc = search_data
    .iter()
    .find(|doc| doc.title == "Included file")
    .expect("included file not found in search-data.json");

  assert_eq!(included_doc.path, "main.html#included-file-heading");

  // println!("{}", temp_dir.keep().display());
  let no_id_doc = search_data
    .iter()
    .find(|doc| doc.title == "Section without an anchor ID")
    .expect("section_no_id file not found in search-data.json");

  assert_eq!(no_id_doc.path, "main.html#section-without-an-anchor-id");
}
