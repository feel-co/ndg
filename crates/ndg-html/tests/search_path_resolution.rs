#![allow(clippy::expect_used, clippy::unwrap_used, reason = "Fine in tests")]
use std::{
  fs::{self, File, create_dir_all},
  path::PathBuf,
};

use ndg_commonmark::collect_markdown_files;
use ndg_config::{Config, search::SearchConfig};
use ndg_html::{
  options::process_options,
  search::{SearchData, generate_search_index},
  template::render,
};
use ndg_utils::{collect_included_files, markdown::create_processor};
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
    search: Some(SearchConfig {
      enable: true,
      ..Default::default()
    }),
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
    search: Some(SearchConfig {
      enable: true,
      ..Default::default()
    }),
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
  let deep_dir = included_dir.join("transitive");
  let output_dir = temp_dir.path().join("output");

  create_dir_all(&deep_dir).expect("failed to create input dir");
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

```{=include=}
transitive/included.md
```
";
  fs::write(included_dir.join("section_no_id.md"), no_anchor_id_content)
    .expect("Failed to write section_no_id.md");

  let transitive_included_content = "# Transitively included file

This file should be transitively included in `main.html`
";
  fs::write(deep_dir.join("included.md"), transitive_included_content)
    .expect("Failed to write section_no_id.md");

  let mut config = Config {
    input_dir: Some(input_dir.clone()),
    output_dir: output_dir.clone(),
    module_options: None,
    title: "Test".to_string(),
    search: Some(SearchConfig {
      enable: true,
      ..Default::default()
    }),
    ..Default::default()
  };

  let processor = Some(create_processor(&config, None));
  config.included_files = collect_included_files(&config, processor.as_ref())
    .expect("Failed to collect include files");

  let all_markdown_files = collect_markdown_files(&input_dir);

  // Filter out included files - only standalone files should be in search index
  let searchable_files: Vec<_> = all_markdown_files
    .iter()
    .filter(|file| {
      file
        .strip_prefix(&input_dir)
        .ok()
        .is_none_or(|rel| !config.included_files.contains_key(rel))
    })
    .cloned()
    .collect();

  generate_search_index(&config, &searchable_files)
    .expect("Failed to generate search index");

  // Verify that search data is generated correctly
  let index_file =
    File::open(output_dir.join("assets").join("search-data.json"))
      .expect("Failed to open search-data.json");
  let search_data_parsed: SearchData =
    serde_json::from_reader(index_file).expect("Failed to read index data");
  let search_data = &search_data_parsed.documents;

  // Only the main file should appear in search index
  // The included files' content is already in main.html, so they're searchable
  // through the main document
  let main_doc = search_data
    .iter()
    .find(|doc| doc.title == "Main file")
    .expect("main file not found in search-data.json");

  assert_eq!(main_doc.path, "main.html");

  // Included files should NOT have separate search entries
  assert!(
    search_data.iter().all(|doc| doc.title != "Included file"),
    "Included files should not have separate search entries"
  );
  assert!(
    search_data
      .iter()
      .all(|doc| doc.title != "Section without an anchor ID"),
    "Included files should not have separate search entries"
  );
  assert!(
    search_data
      .iter()
      .all(|doc| doc.title != "Transitively included file"),
    "Included files should not have separate search entries"
  );
}

#[test]
fn test_nested_directory_include_search_paths() {
  // This test replicates a real-world scenario where:
  // - index.md includes installation/modules.md
  // - installation/modules.md includes installation/modules/nixos.md
  // The search index should NOT create entries for included files as
  // standalone pages (e.g., installation/modules/nixos.html), but should
  // index their content under the root document with anchors.

  let temp_dir = TempDir::new().expect("Failed to create temp dir");
  let input_dir = temp_dir.path().join("input");
  let installation_dir = input_dir.join("installation");
  let modules_dir = installation_dir.join("modules");
  let output_dir = temp_dir.path().join("output");

  create_dir_all(&modules_dir).expect("failed to create modules dir");
  create_dir_all(&output_dir).expect("failed to create output dir");

  // Root document that includes a file from a subdirectory
  let index_content = "# Documentation Index

Welcome to the documentation.

```{=include=}
installation/modules.md
```
";
  fs::write(input_dir.join("index.md"), index_content)
    .expect("Failed to write index.md");

  // Intermediate file that includes deeper nested files
  let modules_content = "# Module Installation {#ch-module-installation}

The below chapters describe module installation.

```{=include=}
modules/nixos.md
modules/home-manager.md
```
";
  fs::write(installation_dir.join("modules.md"), modules_content)
    .expect("Failed to write installation/modules.md");

  // Deeply nested included files
  let nixos_content = "# NixOS Module {#ch-nixos-module}

This describes the NixOS module installation.
";
  fs::write(modules_dir.join("nixos.md"), nixos_content)
    .expect("Failed to write installation/modules/nixos.md");

  let hm_content = "# Home Manager Module {#ch-home-manager-module}

This describes the Home Manager module installation.
";
  fs::write(modules_dir.join("home-manager.md"), hm_content)
    .expect("Failed to write installation/modules/home-manager.md");

  let mut config = Config {
    input_dir: Some(input_dir.clone()),
    output_dir: output_dir.clone(),
    module_options: None,
    title: "Test Documentation".to_string(),
    search: Some(SearchConfig {
      enable: true,
      ..Default::default()
    }),
    ..Default::default()
  };

  let processor = Some(create_processor(&config, None));

  let all_markdown_files = collect_markdown_files(&input_dir);

  // Process markdown files to generate HTML.
  // This also populates config.included_files as a side effect.
  let _processed_files =
    ndg_utils::process_markdown_files(&mut config, processor.as_ref())
      .expect("Failed to process markdown files");

  // Filter out included files - only standalone files should be in search index
  let searchable_files: Vec<_> = all_markdown_files
    .iter()
    .filter(|file| {
      file
        .strip_prefix(&input_dir)
        .ok()
        .is_none_or(|rel| !config.included_files.contains_key(rel))
    })
    .cloned()
    .collect();

  // Generate search index with only standalone files
  generate_search_index(&config, &searchable_files)
    .expect("Failed to generate search index");

  // Verify that search data is generated correctly
  let index_file =
    File::open(output_dir.join("assets").join("search-data.json"))
      .expect("Failed to open search-data.json");
  let search_data_parsed: SearchData =
    serde_json::from_reader(index_file).expect("Failed to read index data");
  let search_data = &search_data_parsed.documents;

  // The index document should be in search results
  let index_doc = search_data
    .iter()
    .find(|doc| doc.title == "Documentation Index");
  assert!(
    index_doc.is_some(),
    "Index document should be in search results"
  );
  assert_eq!(index_doc.unwrap().path, "index.html");

  // Included files should NOT appear as separate search entries
  // Their content is already in index.html
  assert!(
    search_data
      .iter()
      .all(|doc| doc.title != "Module Installation"),
    "Included files should not have separate search entries"
  );
  assert!(
    search_data.iter().all(|doc| doc.title != "NixOS Module"),
    "Included files should not have separate search entries"
  );
  assert!(
    search_data
      .iter()
      .all(|doc| doc.title != "Home Manager Module"),
    "Included files should not have separate search entries"
  );

  // Verify that the included files are NOT created as standalone HTML files
  assert!(
    !output_dir.join("installation/modules.html").exists(),
    "installation/modules.html should not be created (file is included)"
  );
  assert!(
    !output_dir.join("installation/modules/nixos.html").exists(),
    "installation/modules/nixos.html should not be created (file is included)"
  );
  assert!(
    !output_dir
      .join("installation/modules/home-manager.html")
      .exists(),
    "installation/modules/home-manager.html should not be created (file is \
     included)"
  );
}
