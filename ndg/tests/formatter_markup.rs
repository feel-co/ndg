#![allow(clippy::expect_used, reason = "Fine in tests")]
use std::collections::HashSet;

use ndg_commonmark::{
  MarkdownOptions,
  MarkdownOptionsBuilder,
  MarkdownProcessor,
  processor,
};

#[test]
fn parses_basic_markdown_ast() {
  let md = "# Heading 1\n\nSome *italic* and **bold** text.";
  let processor = MarkdownProcessor::new(MarkdownOptions::default());
  let result = processor.render(md);
  let html = result.html;
  assert!(html.contains("<h1") && html.contains("Heading 1"));
  assert!(html.contains("<em>italic</em>"));
  assert!(html.contains("<strong>bold</strong>"));
}

#[test]
fn parses_list_with_inline_anchor() {
  let md = "- []{#item1} Item 1";
  let processor = MarkdownProcessor::new(MarkdownOptions::default());
  let result = processor.render(md);
  let html = result.html;

  let expected = r#"<span id="item1" class="nixos-anchor"></span> Item 1"#;
  assert!(
    html.contains(expected),
    "HTML did not contain expected span with anchor: {html}"
  );
}

#[test]
fn markup_role_pattern_matches() {
  let s = "{command}`ls -l`";
  let role_re = regex::Regex::new(r"\{([a-z]+)\}`([^`]+)`")
    .expect("Failed to compile role regex in formatter_markup test");
  let caps = role_re.captures(s).expect("Should match role pattern");
  assert_eq!(&caps[1], "command");
  assert_eq!(&caps[2], "ls -l");
}

#[test]
fn markdown_processor_handles_command_prompts() {
  let md = "`$ echo hi`";
  let processor = MarkdownProcessor::new(MarkdownOptions::default());
  let result = processor.render(md);

  // The processor should handle command prompts as code blocks
  assert!(result.html.contains("echo hi"));
}

#[test]
fn markdown_processor_handles_inline_code() {
  let md = "`inline code`";
  let processor = MarkdownProcessor::new(MarkdownOptions::default());
  let result = processor.render(md);
  assert!(result.html.contains("<code>inline code</code>"));
}

#[test]
fn safely_process_markup_handles_panic() {
  let result = processor::process_safe("foo", |_| panic!("fail"), "fallback");
  assert_eq!(result, "fallback");
}

#[test]
fn markdown_heading_anchor_regex() {
  let s = "## Section {#sec}";
  let processor = MarkdownProcessor::new(MarkdownOptions::default());
  let result = processor.render(s);
  assert!(result.html.contains("id=\"sec\""));
  assert!(result.html.contains("Section"));
}

#[test]
fn markdown_list_item_with_anchor_regex() {
  let s = "- []{#foo} Bar";
  let processor = MarkdownProcessor::new(MarkdownOptions::default());
  let result = processor.render(s);
  assert!(result.html.contains("id=\"foo\""));
  assert!(result.html.contains("Bar"));
}

#[test]
fn markdown_process_markdown_string_handles_links() {
  let processor = MarkdownProcessor::new(MarkdownOptions::default());
  let result = processor.render("[link](https://example.com)");
  assert!(result.html.contains("<a href=\"https://example.com\""));
}

#[test]
fn test_empty_markdown() {
  let md = "";
  let processor = MarkdownProcessor::new(MarkdownOptions::default());
  let result = processor.render(md);
  // Empty markdown produces a basic HTML structure
  assert!(result.html.contains("<html>") && result.html.contains("<body>"));
}

#[test]
fn test_markdown_with_only_whitespace() {
  let md = "   \n\t\n  ";
  let processor = MarkdownProcessor::new(MarkdownOptions::default());
  let result = processor.render(md);
  // Whitespace markdown produces basic HTML structure
  assert!(result.html.contains("<html>") && result.html.contains("<body>"));
}

#[test]
fn test_complex_nested_lists() {
  let md = r"- Item 1
  - Nested 1.1
  - Nested 1.2
    - Deep nested
- Item 2
  1. Numbered 2.1
  2. Numbered 2.2";
  let processor = MarkdownProcessor::new(MarkdownOptions::default());
  let result = processor.render(md);
  assert!(result.html.contains("<ul>") && result.html.contains("<ol>"));
  assert!(result.html.contains("Nested 1.1"));
  assert!(result.html.contains("Numbered 2.1"));
}

#[test]
fn test_code_blocks_with_syntax_highlighting() {
  let md = r#"```rust
fn main() {
    println!("Hello, world!");
}
```"#;
  let processor = MarkdownProcessor::new(MarkdownOptions::default());
  let result = processor.render(md);
  assert!(result.html.contains("println"));
  // Check for code-related tags
  assert!(
    result.html.contains("<code>")
      || result.html.contains("<pre>")
      || result.html.contains("code")
  );
}

#[test]
fn test_tables() {
  let md = r"| Header 1 | Header 2 |
|----------|----------|
| Cell 1   | Cell 2   |
| Cell 3   | Cell 4   |";
  let processor = MarkdownProcessor::new(MarkdownOptions::default());
  let result = processor.render(md);
  assert!(result.html.contains("<table>"));
  assert!(result.html.contains("Header 1"));
  assert!(result.html.contains("Cell 1"));
}

#[test]
fn test_blockquotes() {
  let md = r"> This is a blockquote
> with multiple lines
>
> > Nested blockquote";
  let processor = MarkdownProcessor::new(MarkdownOptions::default());
  let result = processor.render(md);
  assert!(result.html.contains("<blockquote>"));
  assert!(result.html.contains("Nested blockquote"));
}

#[test]
fn test_links_and_images() {
  let md = r"[Link text](https://example.com)
![Alt text](https://example.com/image.png)";
  let processor = MarkdownProcessor::new(MarkdownOptions::default());
  let result = processor.render(md);
  assert!(result.html.contains(r#"href="https://example.com""#));
  assert!(result.html.contains("Alt text"));
}

#[test]
fn test_emphasis_edge_cases() {
  let md = r"*italic* **bold** ***bold italic*** ~~strikethrough~~";
  let processor = MarkdownProcessor::new(MarkdownOptions::default());
  let result = processor.render(md);
  assert!(result.html.contains("<em>italic</em>"));
  assert!(result.html.contains("<strong>bold</strong>"));
  assert!(result.html.contains("<del>strikethrough</del>"));
}

#[test]
fn test_html_entities() {
  let md = r"&lt;script&gt; &amp; &quot;hello&quot;";
  let processor = MarkdownProcessor::new(MarkdownOptions::default());
  let result = processor.render(md);
  assert!(result.html.contains("&lt;script&gt;"));
  assert!(result.html.contains("&amp;"));
}

#[test]
fn test_option_validation_with_valid_options() {
  let mut valid_options = HashSet::new();
  valid_options.insert("services.nginx.enable".to_string());
  valid_options.insert("services.nginx.package".to_string());

  let options = MarkdownOptionsBuilder::new()
    .valid_options(Some(valid_options))
    .build();
  let processor = MarkdownProcessor::new(options);

  let md = "Use {option}`services.nginx.enable` to enable nginx.";
  let result = processor.render(md);

  // Valid option should be linked
  assert!(
    result.html.contains("nixos-option"),
    "Should contain nixos-option class"
  );
  assert!(
    result.html.contains("services.nginx.enable"),
    "Should contain the option name"
  );
}

#[test]
fn test_option_validation_with_invalid_options() {
  let mut valid_options = HashSet::new();
  valid_options.insert("services.nginx.enable".to_string());

  let options = MarkdownOptionsBuilder::new()
    .valid_options(Some(valid_options.clone()))
    .build();

  let processor = MarkdownProcessor::new(options);

  let md = "Use {option}`services.invalid.option` to configure something.";
  let result = processor.render(md);

  // Invalid option should still be rendered as code but not linked
  assert!(
    result.html.contains("services.invalid.option"),
    "Should contain the option name"
  );
  // Invalid options should NOT be linked, so no href should be present
  assert!(
    !result.html.contains("href="),
    "Invalid option should not be linked"
  );
}

#[test]
fn test_option_validation_disabled_links_all() {
  let options = MarkdownOptionsBuilder::new().build();
  let processor = MarkdownProcessor::new(options);

  let md = "Use {option}`services.any.option` to configure something.";
  let result = processor.render(md);

  // Without validation set, any option should be rendered
  assert!(
    result.html.contains("services.any.option"),
    "Should contain the option name"
  );
  assert!(
    result.html.contains("nixos-option"),
    "Should contain nixos-option class"
  );
}
