use std::path::Path;

use kuchikikiki::{NodeRef, traits::TendrilSink};
use ndg_commonmark::utils::strip_markdown;
use rustc_hash::FxHashMap;

/// Calculate the relative path prefix needed to reach the root from a given
/// file path For example: "docs/subdir/file.html" would return "../"
///              "docs/subdir/nested/file.html" would return "../../"
#[must_use]
pub fn calculate_root_relative_path(file_rel_path: &Path) -> String {
  let depth = file_rel_path.components().count();
  if depth <= 1 {
    String::new() // file is at root level
  } else {
    "../".repeat(depth - 1)
  }
}

/// Generate proper asset paths for templates based on file location
#[must_use]
pub fn generate_asset_paths(
  file_rel_path: &Path,
) -> FxHashMap<&'static str, String> {
  let root_prefix = calculate_root_relative_path(file_rel_path);

  let mut paths = FxHashMap::default();
  paths.reserve(6);
  paths.insert("stylesheet_path", format!("{root_prefix}assets/style.css"));
  paths.insert("main_js_path", format!("{root_prefix}assets/main.js"));
  paths.insert("search_js_path", format!("{root_prefix}assets/search.js"));

  // Navigation paths
  paths.insert("index_path", format!("{root_prefix}index.html"));
  paths.insert("options_path", format!("{root_prefix}options.html"));
  paths.insert("search_path", format!("{root_prefix}search.html"));

  paths
}

/// Process content through the markdown pipeline and extract plain text.
///
/// Converts markdown to plain text while preserving document structure:
///
/// - Headings and paragraphs are separated by newlines
/// - Multiple consecutive blank lines are collapsed to single blank lines
/// - Whitespace within lines is normalized to single spaces
/// - Anchor markers like {#id} are removed
#[must_use]
pub fn content_to_plaintext(content: &str) -> String {
  normalize_plaintext(&strip_markdown(content), true)
}

/// Extract plain text from rendered HTML.
///
/// Uses the same output format as [`content_to_plaintext`], but without blank
/// lines. Block elements start a new line. The text of `<script>`, `<style>`
/// and `<template>` elements is not included. The text of code blocks is
/// included, so that a search can find commands and file names.
#[must_use]
pub fn html_to_plaintext(html: &str) -> String {
  fn walk(node: &NodeRef, out: &mut String) {
    if let Some(text) = node.as_text() {
      out.push_str(&text.borrow());
      return;
    }

    let name = node.as_element().map(|element| &*element.name.local);
    if matches!(name, Some("script" | "style" | "template")) {
      return;
    }
    let is_block = name.is_some_and(is_block_element);
    if name == Some("br") || is_block {
      out.push('\n');
    }
    for child in node.children() {
      walk(&child, out);
    }
    if is_block {
      out.push('\n');
    }
  }

  let mut text = String::with_capacity(html.len() / 2);
  walk(&kuchikikiki::parse_html().one(html), &mut text);
  normalize_plaintext(&text, false)
}

const fn is_block_element(name: &str) -> bool {
  matches!(
    name.as_bytes(),
    b"address"
      | b"article"
      | b"aside"
      | b"blockquote"
      | b"dd"
      | b"details"
      | b"div"
      | b"dl"
      | b"dt"
      | b"figcaption"
      | b"figure"
      | b"footer"
      | b"h1"
      | b"h2"
      | b"h3"
      | b"h4"
      | b"h5"
      | b"h6"
      | b"header"
      | b"hr"
      | b"li"
      | b"main"
      | b"nav"
      | b"ol"
      | b"p"
      | b"pre"
      | b"section"
      | b"summary"
      | b"table"
      | b"td"
      | b"th"
      | b"tr"
      | b"ul"
  )
}

/// Normalize extracted text:
///
/// - Multiple consecutive blank lines are collapsed to single blank lines, or
///   removed if `keep_blank_lines` is false
/// - Whitespace within lines is normalized to single spaces
/// - Anchor markers like {#id} are removed
fn normalize_plaintext(plain_text: &str, keep_blank_lines: bool) -> String {
  let mut result = String::with_capacity(plain_text.len());
  let mut line = String::new();
  let mut prev_empty = false;
  let mut has_output = false;

  for raw_line in plain_text.lines() {
    line.clear();

    // Remove anchor markers {#id} and collapse whitespace.
    for word in raw_line
      .split_whitespace()
      .filter(|word| !(word.starts_with("{#") && word.ends_with('}')))
    {
      if !line.is_empty() {
        line.push(' ');
      }
      line.push_str(word);
    }

    if line.is_empty() {
      if keep_blank_lines && !prev_empty && has_output {
        result.push('\n');
      }
      prev_empty = true;
    } else {
      if has_output {
        result.push('\n');
      }
      result.push_str(&line);
      has_output = true;
      prev_empty = false;
    }
  }

  result.trim().to_string()
}

#[cfg(test)]
mod tests {
  use super::html_to_plaintext;

  #[test]
  fn html_to_plaintext_separates_blocks() {
    let html = "<h1 id=\"a\">Title</h1><p>Some <em>text</em> and \
                <code>code</code>.</p><ul><li>One</li><li>Two</li></ul>";
    assert_eq!(
      html_to_plaintext(html),
      "Title\nSome text and code.\nOne\nTwo"
    );
  }

  #[test]
  fn html_to_plaintext_keeps_code_blocks_and_skips_scripts() {
    let html = "<p>Before<br>after</p><pre><code>ls -la\ncd \
                /</code></pre><script>alert(1)</script><style>p \
                {}</style><p>End</p>";
    assert_eq!(html_to_plaintext(html), "Before\nafter\nls -la\ncd /\nEnd");
  }
}
