use ndg_commonmark::{Header, MarkdownOptions, MarkdownProcessor};

/// Extract headers from markdown using the actual code.
fn extract_headers_from_markdown(md: &str) -> Vec<Header> {
    let processor = MarkdownProcessor::new(MarkdownOptions::default());
    let (_headers, _title) = processor.extract_headers(md);
    _headers
}

#[test]
fn test_plain_text_header() {
    let md = "# Simple Header";
    let headers = extract_headers_from_markdown(md);
    assert_eq!(headers.len(), 1);
    assert_eq!(headers[0].text, "Simple Header");
}

#[test]
fn test_header_with_inline_code() {
    let md = "# Install with `nix-env`";
    let headers = extract_headers_from_markdown(md);
    assert_eq!(headers.len(), 1);
    assert_eq!(headers[0].text, "Install with nix-env");
}

#[test]
fn test_header_with_link() {
    let md = "# See [the docs](https://example.com)";
    let headers = extract_headers_from_markdown(md);
    assert_eq!(headers.len(), 1);
    assert_eq!(headers[0].text, "See the docs");
}

#[test]
fn test_header_with_emphasis_and_strong() {
    let md = "# This is *important* and **bold**";
    let headers = extract_headers_from_markdown(md);
    assert_eq!(headers.len(), 1);
    assert_eq!(headers[0].text, "This is important and bold");
}

#[test]
fn test_header_with_nested_inline_elements() {
    let md = "# Try [*nix-shell*](https://nixos.org) now";
    let headers = extract_headers_from_markdown(md);
    assert_eq!(headers.len(), 1);
    assert_eq!(headers[0].text, "Try nix-shell now");
}

#[test]
fn test_header_with_code_and_link() {
    let md = "# Use [`nix`](https://nixos.org)";
    let headers = extract_headers_from_markdown(md);
    assert_eq!(headers.len(), 1);
    assert_eq!(headers[0].text, "Use nix");
}

#[test]
fn test_header_with_html_inline() {
    let md = "# Hello <span>world</span>";
    let headers = extract_headers_from_markdown(md);
    assert_eq!(headers.len(), 1);
    // HTML inline is now included as visible text
    assert_eq!(headers[0].text, "Hello world");
}

#[test]
fn test_multiple_headers_various_types() {
    let md = r#"
# First *header*
## Second with [link](#)
### Third with `code`
"#;
    let headers = extract_headers_from_markdown(md);
    assert_eq!(headers.len(), 3);
    assert_eq!(headers[0].text, "First header");
    assert_eq!(headers[1].text, "Second with link");
    assert_eq!(headers[2].text, "Third with code");
}

#[test]
fn test_header_with_strikethrough_and_superscript() {
    let md = "# This is ~obsolete~ and super^script^";
    let headers = extract_headers_from_markdown(md);
    assert_eq!(headers.len(), 1);
    assert_eq!(headers[0].text, "This is obsolete and superscript");
}

#[test]
fn test_header_with_image() {
    let md = "# Welcome ![logo](logo.png)";
    let headers = extract_headers_from_markdown(md);
    assert_eq!(headers.len(), 1);
    // Image alt text is not included in header text extraction
    assert_eq!(headers[0].text, "Welcome ");
}
