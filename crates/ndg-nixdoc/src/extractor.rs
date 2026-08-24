//! Nix file extraction logic for finding attribute bindings with doc comments.
//!
//! This module walks a parsed `nixon` syntax tree to find all attribute
//! path-value bindings that are immediately preceded by a `/** ... */`
//! (Nixdoc-style) comment. Nested attribute sets are traversed recursively
//! so that deeply-nested paths (e.g. `lib.strings.concatStrings`) are
//! reported with their full dotted path.
use std::path::Path;

use nixon::{
  Element,
  Node,
  SyntaxKind,
  ast::{self, AstNode},
};

use crate::types::{Location, RawEntry};

/// Pre-computed line start positions for O(log n) line number lookups.
struct LineIndex {
  starts: Vec<usize>,
}

impl LineIndex {
  fn new(src: &str) -> Self {
    let mut starts = vec![0];
    for (byte_offset, c) in src.char_indices() {
      if c == '\n' {
        starts.push(byte_offset + 1);
      }
    }
    Self { starts }
  }

  fn line_of_offset(&self, byte_offset: usize) -> u32 {
    let offset = byte_offset.min(self.starts.last().copied().unwrap_or(0));
    match self.starts.binary_search(&offset) {
      Ok(line) => u32::try_from(line + 1).unwrap_or(u32::MAX),
      Err(insert_idx) => u32::try_from(insert_idx).unwrap_or(u32::MAX),
    }
  }
}

/// Extract all raw doc-comment entries from a Nix source string.
///
/// `file_path` is only used to populate [`Location`] metadata; the actual
/// parsing is done on `src`.
///
/// Parse errors from `nixon` are silently dropped at the node level: because
/// `nixon` is an error-tolerant parser it always returns a tree, but
/// individual nodes may be `None` when their structure could not be
/// determined. Those nodes are skipped.
pub fn extract_entries(src: &str, file_path: &Path) -> Vec<RawEntry> {
  let Ok(document) = nixon::parse_syntax(src) else {
    return Vec::new();
  };
  let Some(root) = ast::Root::cast(document.root()) else {
    return Vec::new();
  };
  let line_index = LineIndex::new(src);

  let mut entries = Vec::new();

  // nixon::Root wraps a single body expression. In flat assignment files (most
  // nixpkgs-style files) the body is itself an AttrSet; in others it may be a
  // `let … in` expression.  Rather than hardcoding the root as "has entries",
  // we recurse into the root's expression tree directly.
  if let Some(body) = root.expression() {
    collect_from_expr(body.syntax(), &[], file_path, &line_index, &mut entries);
  }

  entries
}

/// Recursively walk an expression node, collecting doc-commented bindings.
///
/// Only attribute-set and let-in nodes can directly contain attribute
/// entries. For any other node we descend into all children looking for nested
/// attrsets (e.g. the body of a lambda or a conditional).
///
/// `path_prefix` holds the attribute path segments accumulated from ancestor
/// attrsets so that nested entries are reported with their full dotted path.
fn collect_from_expr(
  node: Node<'_, '_>,
  path_prefix: &[String],
  file_path: &Path,
  line_index: &LineIndex,
  out: &mut Vec<RawEntry>,
) {
  use SyntaxKind::{AttributeSet, LetIn};

  match node.kind() {
    AttributeSet => {
      if let Some(attrset) = ast::AttributeSet::cast(node) {
        collect_entries(
          attrset.syntax(),
          path_prefix,
          file_path,
          line_index,
          out,
        );
      }
    },
    LetIn => {
      if let Some(let_in) = ast::LetIn::cast(node) {
        collect_entries(
          let_in.syntax(),
          path_prefix,
          file_path,
          line_index,
          out,
        );
        // Also recurse into the body expression (the part after `in`), since
        // it may be an attrset containing doc-commented bindings.
        if let Some(body) = let_in.body() {
          collect_from_expr(
            body.syntax(),
            path_prefix,
            file_path,
            line_index,
            out,
          );
        }
      }
    },
    _ => {
      // Not an entry container, so we descend into all children looking for
      // nested attribute sets (e.g. the body of a lambda or a with
      // expression).
      for child in node.child_nodes() {
        collect_from_expr(child, path_prefix, file_path, line_index, out);
      }
    },
  }
}

/// Walk one attrset/let-in level, extract doc-commented bindings, and recurse
/// into nested attrsets.
fn collect_entries(
  container: Node<'_, '_>,
  path_prefix: &[String],
  file_path: &Path,
  line_index: &LineIndex,
  out: &mut Vec<RawEntry>,
) {
  for element in container.children() {
    let Element::Node(node) = element else {
      continue;
    };

    let Some(binding) = ast::Binding::cast(node) else {
      continue;
    };

    let Some(attrpath) = binding.path() else {
      continue;
    };
    let components: Vec<ast::AttributeComponent<'_, '_>> =
      attrpath.components().collect();
    let segments: Vec<String> = components
      .iter()
      .copied()
      .filter_map(attribute_to_string)
      .collect();

    // Skip if any segment was dynamic (could not be statically resolved);
    // a partial path like ["foo", "bar"] for `foo.${expr}.bar` would be wrong.
    if segments.len() != components.len() || segments.is_empty() {
      continue;
    }

    let comment = leading_doc_comment(attrpath.syntax());

    // Only build full_path if we have a doc comment or need to recurse
    if comment.is_some() || binding.value().is_some() {
      let full_path: Vec<String> =
        path_prefix.iter().chain(segments.iter()).cloned().collect();

      if let Some(comment) = comment {
        let start = components
          .first()
          .copied()
          .map(attribute_start)
          .unwrap_or_default();
        let location = Location {
          file: file_path.to_path_buf(),
          line: line_index.line_of_offset(start),
        };

        out.push(RawEntry {
          attr_path: full_path.clone(),
          comment: comment.to_owned(),
          location,
        });
      }

      // recurse into RHS if it is an attrset
      if let Some(value) = binding.value() {
        collect_from_expr(
          value.syntax(),
          &full_path,
          file_path,
          line_index,
          out,
        );
      }
    }
  }
}

/// Find a doc comment among the trivia Nixon attaches to an attribute path.
fn leading_doc_comment<'src>(attrpath: Node<'_, 'src>) -> Option<&'src str> {
  let mut comment = None;
  scan_leading_trivia(attrpath, &mut comment);
  comment
}

fn scan_leading_trivia<'src>(
  node: Node<'_, 'src>,
  comment: &mut Option<&'src str>,
) {
  for element in node.children() {
    match element.kind() {
      SyntaxKind::Whitespace => {},
      SyntaxKind::DocComment => *comment = Some(element.text()),
      SyntaxKind::LineComment | SyntaxKind::BlockComment => *comment = None,
      _ => {
        if let Element::Node(child) = element {
          scan_leading_trivia(child, comment);
        }
        return;
      },
    }
  }
}

fn attribute_start(attribute: ast::AttributeComponent<'_, '_>) -> usize {
  match attribute {
    ast::AttributeComponent::Identifier(identifier) => {
      identifier.range().start().as_usize()
    },
    ast::AttributeComponent::String(string) => {
      first_non_trivia_start(string.syntax())
    },
    ast::AttributeComponent::Interpolation(interpolation) => {
      first_non_trivia_start(interpolation.syntax())
    },
  }
}

fn first_non_trivia_start(node: Node<'_, '_>) -> usize {
  for element in node.children() {
    match element {
      Element::Token(token) if token.kind().is_trivia() => {},
      Element::Token(token) => return token.range().start().as_usize(),
      Element::Node(child) => return first_non_trivia_start(child),
    }
  }
  node.range().start().as_usize()
}

fn first_non_trivia_kind(node: Node<'_, '_>) -> Option<SyntaxKind> {
  for element in node.children() {
    match element {
      Element::Token(token) if token.kind().is_trivia() => {},
      Element::Token(token) => return Some(token.kind()),
      Element::Node(child) => return first_non_trivia_kind(child),
    }
  }
  None
}

/// Convert an [`ast::AttributeComponent`] into a plain `String` segment.
///
/// Handles simple identifiers and string-literal keys. Dynamic keys
/// (e.g. `${ expr }`) cannot be statically resolved and are skipped
/// (returning `None`).
fn attribute_to_string(
  attribute: ast::AttributeComponent<'_, '_>,
) -> Option<String> {
  match attribute {
    ast::AttributeComponent::Identifier(identifier) => {
      Some(identifier.text().to_owned())
    },
    ast::AttributeComponent::String(string) => normalize_static_string(string),
    ast::AttributeComponent::Interpolation(_) => None,
  }
}

fn normalize_static_string(
  string: ast::StringExpression<'_, '_>,
) -> Option<String> {
  let mut contents = String::new();
  for part in string.parts() {
    match part {
      ast::StringPart::Fragment(fragment) => contents.push_str(fragment.text()),
      ast::StringPart::Interpolation(_) => return None,
    }
  }

  let multiline =
    first_non_trivia_kind(string.syntax()) == Some(SyntaxKind::IndentedQuote);
  if multiline {
    contents = strip_indented_string_whitespace(&contents);
  }
  Some(unescape_string(&contents, multiline))
}

// ponytail: remove these two helpers once Nixon exposes normalized static
// string contents.
fn strip_indented_string_whitespace(input: &str) -> String {
  let input = input
    .find('\n')
    .filter(|&newline| {
      input[..newline].chars().all(|character| character == ' ')
    })
    .map_or(input, |newline| &input[newline + 1..]);

  let min_indent = input
    .lines()
    .filter(|line| line.chars().any(|character| character != ' '))
    .map(|line| {
      line
        .chars()
        .take_while(|&character| character == ' ')
        .count()
    })
    .min()
    .unwrap_or(usize::MAX);

  let mut normalized = String::with_capacity(input.len());
  for segment in input.split_inclusive('\n') {
    let (line, newline) = segment
      .strip_suffix('\n')
      .map_or((segment, ""), |line| (line, "\n"));
    let drop = line
      .chars()
      .take_while(|&character| character == ' ')
      .count()
      .min(min_indent);
    normalized.extend(line.chars().skip(drop));
    normalized.push_str(newline);
  }

  if let Some(newline) = normalized.rfind('\n')
    && normalized[newline + 1..]
      .chars()
      .all(|character| character == ' ')
  {
    normalized.truncate(newline + 1);
  }

  normalized
}

fn unescape_string(input: &str, multiline: bool) -> String {
  let mut output = String::with_capacity(input.len());
  let mut characters = input.chars().peekable();

  while let Some(character) = characters.next() {
    match character {
      '\\' if !multiline => push_escape(&mut output, characters.next()),
      '\'' if multiline && characters.next_if_eq(&'\'').is_some() => {
        match characters.next() {
          Some('\'') | None => output.push_str("''"),
          Some('$') => output.push('$'),
          Some('\\') => push_escape(&mut output, characters.next()),
          Some(character) => {
            output.push('\'');
            output.push('\'');
            output.push(character);
          },
        }
      },
      character => output.push(character),
    }
  }

  output
}

fn push_escape(output: &mut String, escaped: Option<char>) {
  match escaped {
    Some('n') => output.push('\n'),
    Some('r') => output.push('\r'),
    Some('t') => output.push('\t'),
    Some(character) => output.push(character),
    None => {},
  }
}

#[cfg(test)]
mod tests {
  use std::path::PathBuf;

  use super::*;

  fn path() -> PathBuf {
    PathBuf::from("test.nix")
  }

  #[test]
  fn test_extract_top_level_doc_comment() {
    let src = r"{
/**
  A top-level function.

  # Arguments

  - [x] The input value.
*/
identity = x: x;
}";
    let entries = extract_entries(src, &path());
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].attr_path, vec!["identity"]);
    assert!(entries[0].comment.contains("A top-level function"));
  }

  #[test]
  fn test_extract_skips_plain_line_comment() {
    let src = r"{
# Not a doc comment
identity = x: x;
}";
    let entries = extract_entries(src, &path());
    assert!(
      entries.is_empty(),
      "plain # comments should not be extracted"
    );
  }

  #[test]
  fn test_extract_skips_plain_block_comment() {
    let src = r"{
/* Not a doc comment either */
identity = x: x;
}";
    let entries = extract_entries(src, &path());
    assert!(
      entries.is_empty(),
      "/* */ comments without ** should be skipped"
    );
  }

  #[test]
  fn test_extract_nested_attrset() {
    let src = r"{
lib = {
  /**
    Concatenates two strings.
  */
  concatStrings = a: b: a + b;
};
}";
    let entries = extract_entries(src, &path());
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].attr_path, vec!["lib", "concatStrings"]);
  }

  #[test]
  fn test_extract_multiple_bindings() {
    let src = r"{
/** First function. */
first = x: x;

/** Second function. */
second = x: x + 1;

notDocumented = x: x;
}";
    let entries = extract_entries(src, &path());
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].attr_path, vec!["first"]);
    assert_eq!(entries[1].attr_path, vec!["second"]);
  }

  #[test]
  fn test_line_number_is_populated() {
    let src = "{ /** Doc. */\nfoo = 1;\n}";
    let entries = extract_entries(src, &path());
    assert_eq!(entries.len(), 1);
    // "foo = 1;" is on line 2.
    assert_eq!(entries[0].location.line, 2);
  }

  #[test]
  fn test_extract_normalizes_static_string_attributes() {
    let src = r#"{
/** Static. */
"line\nbreak" = 1;

/** Indented. */
''
  multi
  line
'' = 2;

/** Dynamic. */
"${name}" = 3;
}"#;
    let entries = extract_entries(src, &path());
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].attr_path, ["line\nbreak"]);
    assert_eq!(entries[1].attr_path, ["multi\nline\n"]);
  }

  #[test]
  fn test_extracts_let_bindings_and_body() {
    let src = r"let
/** Local. */
local = 1;
in {
  /** Body. */
  body = local;
}";
    let entries = extract_entries(src, &path());
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].attr_path, ["local"]);
    assert_eq!(entries[1].attr_path, ["body"]);
  }
}
