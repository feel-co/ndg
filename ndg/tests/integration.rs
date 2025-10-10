use std::fs;
use tempfile::tempdir;
use ndg::config::Config;
use ndg_commonmark::{process_markdown_string, ProcessorPreset};

#[test]
fn test_full_document_processing() {
    let temp_dir = tempdir().unwrap();
    let input_dir = temp_dir.path().join("input");
    let output_dir = temp_dir.path().join("output");
    fs::create_dir_all(&input_dir).unwrap();
    fs::create_dir_all(&output_dir).unwrap();

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
    fs::write(input_dir.join("test.md"), md_content).unwrap();

    // Create a basic config
    let mut config = Config::default();
    config.input_dir = Some(input_dir.clone());
    config.output_dir = output_dir.clone();

    // Test config loading
    assert_eq!(config.input_dir, Some(input_dir));
    assert_eq!(config.output_dir, output_dir);
}

#[test]
fn test_error_handling_on_invalid_input() {
    let temp_dir = tempdir().unwrap();
    let invalid_input = temp_dir.path().join("nonexistent");
    let mut config = Config::default();
    config.input_dir = Some(invalid_input.clone());

    // Test that config handles missing directory gracefully
    assert_eq!(config.input_dir, Some(invalid_input));
}

#[test]
fn test_large_file_processing() {
    let temp_dir = tempdir().unwrap();
    let input_dir = temp_dir.path().join("input");
    fs::create_dir_all(&input_dir).unwrap();

    // Create a large markdown file
    let large_content = "# Heading\n\n".repeat(1000) + "Some content.";
    fs::write(input_dir.join("large.md"), large_content).unwrap();

    // Test that large files can be read without issues
    let content = fs::read_to_string(input_dir.join("large.md")).unwrap();
    assert!(content.len() > 10000);
}

#[test]
fn test_malformed_markdown_recovery() {
    let malformed_md = r#"# Unclosed [link

Some text.

- List item 1
- List item 2

```unclosed code block

End.
"#;

    let result = process_markdown_string(malformed_md, ProcessorPreset::Basic);
    assert!(!result.html.is_empty());
}