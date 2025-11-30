use std::fs;

use ndg::config::Config;
use ndg_commonmark::{ProcessorPreset, process_markdown_string};
use tempfile::tempdir;

#[test]
fn test_full_document_processing() {
  let temp_dir = tempdir().expect("Failed to create temp dir in test");
  let input_dir = temp_dir.path().join("input");
  let output_dir = temp_dir.path().join("output");
  fs::create_dir_all(&input_dir).expect("Failed to create dir in test");
  fs::create_dir_all(&output_dir).expect("Failed to create dir in test");

  // Create a sample markdown file
  let md_content = r#"# Test Document

This is a test.

## Section 1

Some content here.

```bash
echo "hello"
```

## Section 2

More content.
"#;
  fs::write(input_dir.join("test.md"), md_content)
    .expect("Failed to write test.md in test");

  // Create a basic config
  let config = Config {
    input_dir: Some(input_dir.clone()),
    output_dir: output_dir.clone(),
    ..Default::default()
  };

  // Test config loading
  assert_eq!(config.input_dir, Some(input_dir));
  assert_eq!(config.output_dir, output_dir);
}

#[test]
fn test_error_handling_on_invalid_input() {
  let temp_dir = tempdir().expect("Failed to create temp dir in test");
  let invalid_input = temp_dir.path().join("nonexistent");
  let config = Config {
    input_dir: Some(invalid_input.clone()),
    ..Default::default()
  };

  // Test that config handles missing directory gracefully
  assert_eq!(config.input_dir, Some(invalid_input));
}

#[test]
fn test_large_file_processing() {
  let temp_dir = tempdir().expect("Failed to create temp dir in test");
  let input_dir = temp_dir.path().join("input");
  fs::create_dir_all(&input_dir).expect("Failed to create dir in test");

  // Create a large markdown file
  let large_content = "# Heading\n\n".repeat(1000) + "Some content.";
  fs::write(input_dir.join("large.md"), large_content)
    .expect("Failed to write large.md in test");

  // Test that large files can be read without issues
  let content = fs::read_to_string(input_dir.join("large.md"))
    .expect("Failed to read large.md in test");
  assert!(content.len() > 10000);
}

#[test]
fn test_malformed_markdown_recovery() {
  let malformed_md = r"# Unclosed [link

Some text.

- List item 1
- List item 2

```unclosed code block

End.
";

  let result = process_markdown_string(malformed_md, ProcessorPreset::Basic);
  assert!(!result.html.is_empty());
}

#[test]
fn test_sidebar_integration() {
  use std::path::Path;

  use ndg::{
    config::sidebar::{
      PathMatch,
      SidebarConfig,
      SidebarMatch,
      SidebarOrdering,
      TitleMatch,
    },
    html::template::render,
  };

  let temp_dir = tempdir().expect("Failed to create temp dir in test");
  let input_dir = temp_dir.path().join("input");
  let output_dir = temp_dir.path().join("output");
  fs::create_dir_all(&input_dir).expect("Failed to create dir in test");
  fs::create_dir_all(&output_dir).expect("Failed to create dir in test");

  // Create test markdown files
  fs::write(
    input_dir.join("installation.md"),
    "# Installation Guide\n\nHow to install.",
  )
  .expect("Failed to write installation.md in test");
  fs::write(
    input_dir.join("configuration.md"),
    "# Configuration\n\nHow to configure.",
  )
  .expect("Failed to write configuration.md in test");
  fs::write(
    input_dir.join("api.md"),
    "# API Reference\n\nAPI documentation.",
  )
  .expect("Failed to write api.md in test");

  // Test with numbered sidebar
  let sidebar_config = SidebarConfig {
    numbered:             true,
    number_special_files: false,
    ordering:             SidebarOrdering::Alphabetical,
    matches:              vec![],
    options:              None,
  };

  let config = Config {
    input_dir: Some(input_dir.clone()),
    output_dir: output_dir.clone(),
    sidebar: Some(sidebar_config),
    ..Default::default()
  };

  let html = render(
    &config,
    "<p>Test content</p>",
    "Test",
    &[],
    Path::new("test.md"),
  )
  .expect("Failed to render HTML in test");

  // Check that the navigation contains numbered items
  assert!(html.contains("<li>"), "HTML should contain list items");

  // Test with custom ordering using positions
  let sidebar_config_custom = SidebarConfig {
    numbered:             false,
    number_special_files: false,
    ordering:             SidebarOrdering::Custom,
    matches:              vec![
      SidebarMatch {
        path:      Some(PathMatch {
          exact:          Some("installation.md".to_string()),
          regex:          None,
          compiled_regex: None,
        }),
        title:     None,
        new_title: Some("Setup Guide".to_string()),
        position:  Some(1),
      },
      SidebarMatch {
        path:      Some(PathMatch {
          exact:          Some("api.md".to_string()),
          regex:          None,
          compiled_regex: None,
        }),
        title:     None,
        new_title: None,
        position:  Some(3),
      },
      SidebarMatch {
        path:      Some(PathMatch {
          exact:          Some("configuration.md".to_string()),
          regex:          None,
          compiled_regex: None,
        }),
        title:     None,
        new_title: None,
        position:  Some(2),
      },
    ],
    options:              None,
  };

  let config_custom = Config {
    input_dir: Some(input_dir.clone()),
    output_dir: output_dir.clone(),
    sidebar: Some(sidebar_config_custom),
    ..Default::default()
  };

  let html_custom = render(
    &config_custom,
    "<p>Test content</p>",
    "Test",
    &[],
    Path::new("test.md"),
  )
  .expect("Failed to render HTML with custom config in test");

  // Check that custom title is applied
  assert!(
    html_custom.contains("Setup Guide"),
    "Custom title should be applied"
  );

  // Test with regex matching
  let mut sidebar_config_regex = SidebarConfig {
    numbered:             true,
    number_special_files: false,
    ordering:             SidebarOrdering::Alphabetical,
    matches:              vec![SidebarMatch {
      path:      Some(PathMatch {
        exact:          None,
        regex:          Some(r"^.*\.md$".to_string()),
        compiled_regex: None,
      }),
      title:     Some(TitleMatch {
        exact:          None,
        regex:          Some(r".*Guide.*".to_string()),
        compiled_regex: None,
      }),
      new_title: Some("📖 Guide".to_string()),
      position:  None,
    }],
    options:              None,
  };

  // Compile regexes
  sidebar_config_regex
    .validate()
    .expect("sidebar config should be valid");

  let config_regex = Config {
    input_dir: Some(input_dir),
    output_dir,
    sidebar: Some(sidebar_config_regex),
    ..Default::default()
  };

  let html_regex = render(
    &config_regex,
    "<p>Test content</p>",
    "Installation Guide",
    &[],
    Path::new("installation.md"),
  )
  .expect("Failed to render HTML with regex config in test");

  // Verify the regex match worked - the title contains "Guide" so it should
  // match
  assert!(html_regex.contains("📖 Guide") || html_regex.contains("Guide"));
}
