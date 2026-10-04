#![allow(clippy::expect_used, reason = "Fine in tests")]
use std::path::PathBuf;

mod common;

use common::test_config;
use ndg_html::template::render;
use tempfile::TempDir;

#[test]
fn test_custom_script_links_to_copied_asset() {
  let temp_dir = TempDir::new().expect("Failed to create temp dir");
  let output_dir = temp_dir.path();

  // A script outside the output directory, as with a path in the Nix store.
  let config = ndg_config::Config {
    script_paths: vec![PathBuf::from("/nix/store/abc-scripts/custom.js")],
    ..test_config(output_dir)
  };

  let html = render(
    &config,
    "<p>Test content</p>",
    "Test Page",
    &[],
    &PathBuf::from("docs/test.html"),
    None,
  )
  .expect("Failed to render page");

  // The script is copied to `assets/<file name>`, so the page must link there.
  assert!(
    html.contains("<script defer src=\"../assets/custom.js\"></script>"),
    "Script should link to the copied asset"
  );
  assert!(
    !html.contains("/nix/store/abc-scripts/custom.js"),
    "Script should not link to its source path"
  );
}
