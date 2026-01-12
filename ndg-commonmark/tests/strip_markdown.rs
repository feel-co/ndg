#![allow(clippy::expect_used, reason = "Fine in tests")]

use ndg_commonmark::utils::strip_markdown;

#[test]
fn test_strip_markdown_preserves_inline_code() {
  let md = "Use `grep` to search";
  let result = strip_markdown(md);
  assert!(
    result.contains("grep"),
    "Inline code should be preserved, got: {result}"
  );
}

#[test]
fn test_strip_markdown_multiple_inline_code() {
  let md = "Use `grep` and `sed` for text processing";
  let result = strip_markdown(md);
  assert!(
    result.contains("grep"),
    "First inline code should be preserved, got: {result}"
  );
  assert!(
    result.contains("sed"),
    "Second inline code should be preserved, got: {result}"
  );
}

#[test]
fn test_strip_markdown_inline_code_with_underscores() {
  let md = "Use `my_function` for processing";
  let result = strip_markdown(md);
  assert!(
    result.contains("my_function"),
    "Inline code with underscores should be preserved, got: {result}"
  );
}

#[test]
fn test_strip_markdown_code_block_not_preserved() {
  let md = r#"```bash
echo "hello"
```"#;
  let result = strip_markdown(md);
  assert!(
    !result.contains("hello"),
    "Code block content should not be in plain text, got: {result}"
  );
}

#[test]
fn test_strip_markdown_removes_formatting() {
  let md = "This is **bold** and *italic* text";
  let result = strip_markdown(md);
  assert!(!result.contains("**"), "Bold markers should be removed");
  assert!(
    !result.contains("*"),
    "Italic markers should be removed, got: {result}"
  );
  assert!(result.contains("bold"), "Bold text content should remain");
  assert!(
    result.contains("italic"),
    "Italic text content should remain"
  );
}

#[test]
fn test_strip_markdown_removes_links() {
  let md = "Click [here](https://example.com) for more";
  let result = strip_markdown(md);
  assert!(!result.contains("[here]"), "Link markup should be removed");
  assert!(
    !result.contains("(https://example.com)"),
    "Link URL should be removed"
  );
  assert!(result.contains("here"), "Link text content should remain");
}
