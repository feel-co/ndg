use ndg_commonmark::{
  MarkdownOptionsBuilder,
  MarkdownProcessor,
  ProcessorFeature,
  ProcessorPreset,
  create_processor,
  process_markdown_string,
  process_with_recovery,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
  println!("NDG Commonmark Processor Examples\n");

  // 1. Using preset configurations
  example_presets();

  // 2. Using the builder pattern
  example_builder_pattern()?;

  // 3. Using processor methods
  example_processor_methods();

  // 4. Batch processing
  example_batch_processing();

  Ok(())
}

/// Demonstrate different processor presets
fn example_presets() {
  println!("=== Preset Examples ===");

  let markdown = r#"
# Example Document

This is a **bold** statement with `inline code`.

```rust
fn hello() {
    println!("Hello, world!");
}
```

- Item 1
- Item 2
- Item 3
"#;

  // Basic preset - GFM only
  println!("Basic preset:");
  let result = process_markdown_string(markdown, ProcessorPreset::Basic);
  println!("Title: {:?}", result.title);
  println!("Headers: {}", result.headers.len());

  // Enhanced preset - with syntax highlighting
  println!("\nEnhanced preset:");
  let result = process_markdown_string(markdown, ProcessorPreset::Ndg);
  println!("HTML length: {}", result.html.len());

  println!();
}

/// Demonstrate the builder pattern for configuration
fn example_builder_pattern() -> Result<(), Box<dyn std::error::Error>> {
  println!("=== Builder Pattern Example ===");

  // Build custom options using the builder pattern
  let options = MarkdownOptionsBuilder::new()
    .gfm(true)
    .highlight_code(true)
    .highlight_theme(Some("github"))
    .nixpkgs(false)
    .build();

  let processor = MarkdownProcessor::new(options);

  let markdown = r#"
# Custom Configuration

This processor was configured with:
- GitHub Flavored Markdown
- Syntax highlighting with GitHub theme
- No Nixpkgs extensions

```python
def greet(name):
    print(f"Hello, {name}!")
```
"#;

  let result = process_with_recovery(&processor, markdown);
  println!("Processed {} headers", result.headers.len());
  println!("Title: {:?}", result.title);
  println!();

  Ok(())
}

/// Demonstrate object-oriented processor methods
fn example_processor_methods() {
  println!("=== Object-Oriented Usage ===");

  let processor = create_processor(ProcessorPreset::Ndg);

  // Query processor features
  println!("Processor features:");
  println!("  GFM: {}", processor.has_feature(ProcessorFeature::Gfm));
  println!(
    "  Nixpkgs: {}",
    processor.has_feature(ProcessorFeature::Nixpkgs)
  );
  println!(
    "  Syntax highlighting: {}",
    processor.has_feature(ProcessorFeature::SyntaxHighlighting)
  );
  println!(
    "  Manpage URLs: {}",
    processor.has_feature(ProcessorFeature::ManpageUrls)
  );

  // Access options
  let options = processor.options();
  println!("\nOptions:");
  println!("  GFM enabled: {}", options.gfm);
  println!("  Highlight theme: {:?}", options.highlight_theme);

  // Process content
  let markdown = r#"
## Feature Demonstration

This shows how to use the ndg-commonmark processor:

1. Create a processor instance
2. Query its capabilities
3. Process content multiple times
4. Access configuration details

> **Note**: This is more efficient for batch processing!
"#;

  let result = processor.render(markdown);
  println!("\nProcessed content:");
  println!("  {} characters of HTML", result.html.len());
  println!("  {} headers found", result.headers.len());

  for header in &result.headers {
    println!("    H{}: {} (id: {})", header.level, header.text, header.id);
  }

  println!();
}

/// Demonstrate batch processing capabilities
fn example_batch_processing() {
  println!("=== Batch Processing Example ===");

  // Simulate multiple markdown files
  let documents = vec![
    ("doc1.md", "# Document One\n\nFirst document content."),
    ("doc2.md", "# Document Two\n\nSecond document content."),
    (
      "doc3.md",
      "# Document Three\n\n## Subsection\n\nThird document.",
    ),
  ];

  let processor = create_processor(ProcessorPreset::Basic);

  // Process all documents with the same processor instance
  let mut total_headers = 0;
  for (filename, content) in documents {
    let result = process_with_recovery(&processor, content);
    total_headers += result.headers.len();
    println!(
      "  {}: {} headers, title: {:?}",
      filename,
      result.headers.len(),
      result.title
    );
  }

  println!("Total headers across all documents: {}", total_headers);
  println!();
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_processor_features() {
    let basic = create_processor(ProcessorPreset::Basic);
    assert!(basic.has_feature(ProcessorFeature::Gfm));
    assert!(!basic.has_feature(ProcessorFeature::Nixpkgs));

    let nixpkgs = create_processor(ProcessorPreset::Nixpkgs);
    assert!(nixpkgs.has_feature(ProcessorFeature::Gfm));
    assert!(nixpkgs.has_feature(ProcessorFeature::Nixpkgs));
  }

  #[test]
  fn test_builder_pattern() {
    let options = MarkdownOptionsBuilder::new()
      .gfm(false)
      .nixpkgs(true)
      .highlight_code(false)
      .build();

    assert!(!options.gfm);
    assert!(options.nixpkgs);
    assert!(!options.highlight_code);
  }

  #[test]
  fn test_reusable_processor() {
    let processor = create_processor(ProcessorPreset::Basic);

    let content1 = "# First\nContent one";
    let content2 = "# Second\nContent two";

    let result1 = processor.render(content1);
    let result2 = processor.render(content2);

    assert_eq!(result1.title, Some("First".to_string()));
    assert_eq!(result2.title, Some("Second".to_string()));
  }
}
