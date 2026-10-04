#![allow(clippy::expect_used, reason = "Fine in tests")]
use std::{
  fs,
  path::{Path, PathBuf},
};

mod common;

use common::test_config;
use ndg_html::template::render;
use ndg_utils::assets::copy_assets;
use tempfile::{TempDir, tempdir_in};

#[test]
fn test_custom_script_links_to_copied_asset() {
  let sources = TempDir::new().expect("Failed to create script sources");
  let cwd = std::env::current_dir().expect("Failed to resolve test directory");
  let relative_sources =
    tempdir_in(&cwd).expect("Failed to create relative sources");
  let output = TempDir::new().expect("Failed to create output");
  let absolute_script = sources.path().join("absolute.js");
  let relative_script = relative_sources
    .path()
    .strip_prefix(&cwd)
    .expect("Relative source must be under the test directory")
    .join("relative.js");
  fs::write(&absolute_script, "window.absoluteScript = true;")
    .expect("Write absolute script");
  fs::write(&relative_script, "window.relativeScript = true;")
    .expect("Write relative script");
  let config = ndg_config::Config {
    script_paths: vec![absolute_script, relative_script],
    ..test_config(output.path())
  };
  copy_assets(&config).expect("Failed to copy scripts");

  for page in [Path::new("index.html"), Path::new("docs/test.html")] {
    let html =
      render(&config, "<p>Test content</p>", "Test Page", &[], page, None)
        .expect("Failed to render page");
    let page_dir = output
      .path()
      .join(page)
      .parent()
      .expect("Page parent")
      .to_path_buf();
    fs::create_dir_all(&page_dir).expect("Create page directory");
    for (filename, content) in [
      ("absolute.js", "window.absoluteScript = true;"),
      ("relative.js", "window.relativeScript = true;"),
    ] {
      let src = html
        .split("<script defer src=\"")
        .filter_map(|tag| tag.split_once('"').map(|(src, _)| src))
        .find(|src| src.ends_with(filename))
        .expect("Missing custom script link");
      assert_eq!(
        fs::read_to_string(page_dir.join(src))
          .expect("Script link must resolve to a copied file"),
        content
      );
    }
  }
}

#[test]
fn test_custom_scripts_reject_paths_without_filenames() {
  let output = TempDir::new().expect("Failed to create output");
  let config = ndg_config::Config {
    script_paths: vec![PathBuf::from("/")],
    ..test_config(output.path())
  };
  let render_error =
    render(&config, "", "Test", &[], Path::new("index.html"), None)
      .expect_err("Rendering must reject a script without a filename")
      .to_string();
  let copy_error = copy_assets(&config)
    .expect_err("Copying must reject a script without a filename")
    .to_string();
  assert_eq!(render_error, copy_error);
  assert!(render_error.contains("Invalid script_paths entry '/'"));
  assert!(render_error.contains("expected a path with a filename"));
  assert!(!output.path().join("assets").exists());
}

#[test]
fn test_custom_scripts_reject_duplicate_destination_filenames() {
  let temp = TempDir::new().expect("Failed to create sources");
  let first = temp.path().join("first/custom.js");
  let second = temp.path().join("second/custom.js");
  fs::create_dir_all(first.parent().expect("First parent"))
    .expect("Create first source directory");
  fs::create_dir_all(second.parent().expect("Second parent"))
    .expect("Create second source directory");
  fs::write(&first, "window.first = true;").expect("Write first script");
  fs::write(&second, "window.second = true;").expect("Write second script");
  let output = temp.path().join("output");
  fs::create_dir_all(output.join("assets")).expect("Create existing assets");
  fs::write(output.join("assets/custom.js"), "previous script")
    .expect("Write existing destination");
  let config = ndg_config::Config {
    script_paths: vec![first.clone(), second.clone()],
    ..test_config(&output)
  };
  let render_error =
    render(&config, "", "Test", &[], Path::new("index.html"), None)
      .expect_err("Rendering must reject conflicting scripts")
      .to_string();
  let copy_error = copy_assets(&config)
    .expect_err("Copying must reject conflicting scripts")
    .to_string();
  let config_error = config
    .validate_paths()
    .expect_err("Config validation must reject conflicting scripts")
    .to_string();
  assert_eq!(render_error, copy_error);
  assert_eq!(render_error, config_error);
  assert!(render_error.contains(&first.display().to_string()));
  assert!(render_error.contains(&second.display().to_string()));
  assert!(render_error.contains("both copy to 'assets/custom.js'"));
  assert_eq!(
    fs::read_to_string(output.join("assets/custom.js"))
      .expect("Read original destination"),
    "previous script"
  );
}
