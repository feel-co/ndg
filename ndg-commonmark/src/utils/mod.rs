use std::{collections::HashMap, sync::OnceLock};
pub mod codeblock;

use comrak::{
  Arena,
  nodes::{AstNode, NodeHeading, NodeValue},
  options::Options,
  parse_document,
};
use regex::Regex;

/// Error type for utility operations.
#[derive(Debug, thiserror::Error)]
pub enum UtilError {
  #[error("Regex compilation failed: {0}")]
  RegexError(#[from] regex::Error),
}

/// Result type for utility operations.
pub type UtilResult<T> = Result<T, UtilError>;

/// Slugify a string for use as an anchor ID.
/// Converts to lowercase, replaces non-alphanumeric characters with dashes,
/// and trims leading/trailing dashes.
#[must_use]
pub fn slugify(text: &str) -> String {
  text
    .to_lowercase()
    .replace(|c: char| !c.is_alphanumeric() && c != '-' && c != '_', "-")
    .trim_matches('-')
    .to_string()
}

/// Extract the first heading from markdown content as the page title.
/// Returns None if no heading is found.
#[must_use]
pub fn extract_markdown_title(content: &str) -> Option<String> {
  let arena = Arena::new();
  let mut options = Options::default();
  options.extension.table = true;
  options.extension.footnotes = true;
  options.extension.strikethrough = true;
  options.extension.tasklist = true;
  options.extension.superscript = true;
  options.render.r#unsafe = true;

  let root = parse_document(&arena, content, &options);

  for node in root.descendants() {
    if let NodeValue::Heading(_) = &node.data.borrow().value {
      let mut text = String::new();
      for child in node.children() {
        if let NodeValue::Text(t) = &child.data.borrow().value {
          text.push_str(t);
        }
        // Optionally handle inline formatting, code, etc.
        if let NodeValue::Code(t) = &child.data.borrow().value {
          text.push_str(&t.literal);
        }
      }
      if !text.trim().is_empty() {
        return Some(text.trim().to_string());
      }
    }
  }
  None
}

/// Extract the first H1 heading from markdown content as the document title.
/// Removes inline anchors and other markup from the title text.
///
/// # Returns
///
/// `None` if no H1 heading is found.
#[must_use]
pub fn extract_title_from_markdown(content: &str) -> Option<String> {
  let arena = Arena::new();
  let mut options = Options::default();
  options.extension.table = true;
  options.extension.footnotes = true;
  options.extension.strikethrough = true;
  options.extension.tasklist = true;
  options.render.r#unsafe = true;

  let root = parse_document(&arena, content, &options);

  // Use a static regex to avoid compilation failures at runtime
  #[allow(
    clippy::items_after_statements,
    reason = "Static is Scoped to function for clarity"
  )]
  static ANCHOR_RE: OnceLock<Regex> = OnceLock::new();
  let anchor_re = ANCHOR_RE.get_or_init(|| {
    Regex::new(r"(\[\]\{#.*?\}|\{#.*?\})")
      .unwrap_or_else(|_| never_matching_regex())
  });

  for node in root.descendants() {
    if let NodeValue::Heading(NodeHeading { level, .. }) =
      &node.data.borrow().value
    {
      if *level == 1 {
        let mut text = String::new();
        for child in node.children() {
          if let NodeValue::Text(ref t) = child.data.borrow().value {
            text.push_str(t);
          }
        }
        // Clean the title by removing inline anchors and other NDG markup
        let clean_title = anchor_re.replace_all(&text, "").trim().to_string();
        if !clean_title.is_empty() {
          return Some(clean_title);
        }
      }
    }
  }
  None
}

/// Clean anchor patterns from text (removes {#anchor-id} patterns).
/// This is useful for cleaning titles and navigation text.
#[must_use]
pub fn clean_anchor_patterns(text: &str) -> String {
  static ANCHOR_PATTERN: OnceLock<Regex> = OnceLock::new();
  let anchor_pattern = ANCHOR_PATTERN.get_or_init(|| {
    Regex::new(r"\s*\{#[a-zA-Z0-9_-]+\}\s*$")
      .unwrap_or_else(|_| never_matching_regex())
  });
  anchor_pattern.replace_all(text.trim(), "").to_string()
}

/// Apply a regex transformation to HTML elements using the provided function.
/// Used by the markdown processor for HTML element transformations.
pub fn process_html_elements<F>(
  html: &str,
  regex: &Regex,
  transform: F,
) -> String
where
  F: Fn(&regex::Captures) -> String,
{
  match regex.replace_all(html, transform) {
    std::borrow::Cow::Borrowed(_) => html.to_string(),
    std::borrow::Cow::Owned(s) => s,
  }
}

/// Strip markdown formatting and return plain text.
///
/// This processes the markdown through the AST and extracts only text content,
/// excluding code blocks and other formatting.
#[must_use]
pub fn strip_markdown(content: &str) -> String {
  let arena = Arena::new();
  let mut options = Options::default();
  options.extension.table = true;
  options.extension.footnotes = true;
  options.extension.strikethrough = true;
  options.extension.tasklist = true;
  options.render.r#unsafe = true;

  let root = parse_document(&arena, content, &options);

  let mut plain_text = String::new();
  #[allow(clippy::items_after_statements, reason = "Helper scoped for clarity")]
  fn extract_text<'a>(
    node: &'a AstNode<'a>,
    plain_text: &mut String,
    in_code_block: &mut bool,
  ) {
    match &node.data.borrow().value {
      NodeValue::Text(t) => {
        if !*in_code_block {
          plain_text.push_str(t);
          plain_text.push(' ');
        }
      },
      NodeValue::CodeBlock(_) => {
        *in_code_block = true;
      },
      NodeValue::SoftBreak => {
        plain_text.push(' ');
      },
      NodeValue::LineBreak => {
        plain_text.push('\n');
      },
      _ => {},
    }
    for child in node.children() {
      extract_text(child, plain_text, in_code_block);
    }
    if let NodeValue::CodeBlock(_) = &node.data.borrow().value {
      *in_code_block = false;
    }
  }
  let mut in_code_block = false;
  extract_text(root, &mut plain_text, &mut in_code_block);
  plain_text
}

/// Capitalize the first letter of a string.
pub fn capitalize_first(s: &str) -> String {
  let mut chars = s.chars();
  chars.next().map_or_else(String::new, |c| {
    c.to_uppercase().collect::<String>() + chars.as_str()
  })
}

/// Return true if the string looks like a markdown header (starts with #).
#[must_use]
pub fn is_markdown_header(line: &str) -> bool {
  line.trim_start().starts_with('#')
}

/// Load manpage URL mappings from a JSON file.
///
/// # Errors
///
/// Returns an error if the file cannot be read or if the JSON is invalid.
pub fn load_manpage_urls(
  path: &str,
) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
  let content = std::fs::read_to_string(path)?;
  let mappings: HashMap<String, String> = serde_json::from_str(&content)?;
  Ok(mappings)
}

/// Create a regex that never matches anything.
///
/// This is used as a fallback pattern when a regex fails to compile.
/// It will never match any input, which is safer than using a trivial regex
/// like `^$` which would match empty strings.
///
/// # Panics
///
/// Panics if the fallback regex pattern `r"^\b$"` fails to compile, which
/// should never happen.
#[must_use]
pub fn never_matching_regex() -> regex::Regex {
  // Use a pattern that will never match anything because it asserts something
  // impossible - this pattern is guaranteed to be valid
  regex::Regex::new(r"[^\s\S]").unwrap_or_else(|_| {
    // As an ultimate fallback, use an empty pattern that matches nothing
    regex::Regex::new(r"^\b$").unwrap()
  })
}
