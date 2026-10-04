#![allow(clippy::expect_used, reason = "Fine in tests")]
use std::path::PathBuf;

mod common;

use common::test_config;
use ndg_config::mermaid::{DEFAULT_MERMAID_SCRIPT, MermaidConfig};
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

fn render_with_mermaid(
  script: Option<&str>,
  output_dir: &std::path::Path,
) -> String {
  let config = ndg_config::Config {
    mermaid: Some(MermaidConfig {
      enable: true,
      script: script.map(str::to_owned),
    }),
    ..test_config(output_dir)
  };

  render(
    &config,
    "<p>Test content</p>",
    "Test Page",
    &[],
    &PathBuf::from("docs/test.html"),
    None,
  )
  .expect("Failed to render page")
}

#[test]
fn test_mermaid_loader_uses_default_cdn() {
  let temp_dir = TempDir::new().expect("Failed to create temp dir");
  let html = render_with_mermaid(None, temp_dir.path());

  assert!(
    html.contains(&format!(
      "<script defer src=\"../assets/mermaid-init.js\" \
       data-mermaid-src=\"{DEFAULT_MERMAID_SCRIPT}\"></script>"
    )),
    "Loader should point to the default Mermaid script"
  );
}

#[test]
fn test_mermaid_loader_links_to_copied_script() {
  let temp_dir = TempDir::new().expect("Failed to create temp dir");
  let html = render_with_mermaid(
    Some("/nix/store/abc-mermaid/mermaid.min.js"),
    temp_dir.path(),
  );

  assert!(
    html.contains("data-mermaid-src=\"../assets/mermaid.min.js\""),
    "Loader should point to the copied Mermaid script"
  );
  assert!(
    !html.contains("/nix/store/abc-mermaid"),
    "Loader should not point to the source path of the script"
  );
}

#[test]
fn test_mermaid_loader_absent_when_disabled() {
  let temp_dir = TempDir::new().expect("Failed to create temp dir");
  let config = ndg_config::Config {
    mermaid: Some(MermaidConfig {
      enable: false,
      script: None,
    }),
    ..test_config(temp_dir.path())
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

  assert!(
    !html.contains("mermaid-init.js"),
    "Loader should not be added when Mermaid is disabled"
  );
}
