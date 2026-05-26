#![allow(clippy::expect_used, reason = "Fine in tests")]
use ndg_commonmark::{Header, MarkdownOptions, MarkdownProcessor};

/// Extract headers from markdown using the actual code.
fn extract_headers_from_markdown(md: &str) -> Vec<Header> {
  let mut options = MarkdownOptions::default();
  options.highlight_code = false;
  let processor = MarkdownProcessor::new(options);
  let (headers, _title) = processor.extract_headers(md);
  headers
}

#[test]
fn test_header_text_extraction() {
  let cases = [
    ("# Simple Header", "Simple Header"),
    ("# Install with `nix-env`", "Install with nix-env"),
    ("# See [the docs](https://example.com)", "See the docs"),
    (
      "# This is *important* and **bold**",
      "This is important and bold",
    ),
    (
      "# Try [*nix-shell*](https://nixos.org) now",
      "Try nix-shell now",
    ),
    ("# Use [`nix`](https://nixos.org)", "Use nix"),
    ("# Hello <span>world</span>", "Hello world"),
    (
      "# This is ~obsolete~ and super^script^",
      "This is obsolete and superscript",
    ),
    ("# Welcome ![logo](logo.png)", "Welcome "),
  ];

  for (markdown, expected) in cases {
    let headers = extract_headers_from_markdown(markdown);
    assert_eq!(headers.len(), 1, "unexpected header count for {markdown}");
    assert_eq!(headers[0].text, expected);
  }
}

#[test]
fn test_multiple_headers_various_types() {
  let md = r"
# First *header*
## Second with [link](#)
### Third with `code`
";
  let headers = extract_headers_from_markdown(md);
  assert_eq!(headers.len(), 3);
  assert_eq!(headers[0].text, "First header");
  assert_eq!(headers[1].text, "Second with link");
  assert_eq!(headers[2].text, "Third with code");
}

#[test]
fn test_no_headers_from_code_block() {
  let md = r#"- **Create memorable custom ID anchors** for important sections:

  ```markdown
  ## Installation {#my-epic-installation}

  Refer to the [installation instructions](#my-epic-installation) above.
  ```

## Building from Source
"#;
  let headers = extract_headers_from_markdown(md);
  assert_eq!(
    headers.len(),
    1,
    "Should only extract actual headings, not code block content"
  );
  assert_eq!(headers[0].text, "Building from Source");
}

#[test]
fn test_code_block_preserved_in_output() {
  let md = r#"- **Create memorable custom ID anchors** for important sections:

  ```markdown
  ## Installation {#my-epic-installation}

  Refer to the [installation instructions](#my-epic-installation) above.
  ```

## Building from Source
"#;
  let mut options = MarkdownOptions::default();
  options.highlight_code = false;
  let processor = MarkdownProcessor::new(options);
  let result = processor.render(md);
  let html = result.html;

  // The code block should be preserved as a single code block
  assert!(
    html.contains("<code class=\"language-markdown\">"),
    "Code block should be preserved in HTML output"
  );

  // The content inside should NOT be converted to actual headings
  assert!(
    !html.contains("<h2 id=\"my-epic-installation\">"),
    "Code block content should not be converted to actual headings"
  );

  // The actual heading should still be rendered
  assert!(
    html.contains("<h2 id=\"building-from-source\">"),
    "Actual heading should still be rendered"
  );
}
