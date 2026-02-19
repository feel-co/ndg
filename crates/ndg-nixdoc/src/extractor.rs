//! Nix file extraction logic for finding attribute bindings with doc comments.
//!
//! This module walks a parsed `rnix` syntax tree to find all attribute
//! path-value bindings that are immediately preceded by a `/** ... */`
//! (Nixdoc-style) comment. Nested attribute sets are traversed recursively
//! so that deeply-nested paths (e.g. `lib.strings.concatStrings`) are
//! reported with their full dotted path.
use std::path::Path;

use rnix::{
  SyntaxKind,
  ast::{self, HasEntry},
};
use rowan::ast::AstNode;

use crate::types::{Location, RawEntry};

/// Extract all raw doc-comment entries from a Nix source string.
///
/// `file_path` is only used to populate [`Location`] metadata; the actual
/// parsing is done on `src`.
///
/// Parse errors from `rnix` are silently dropped at the node level: because
/// `rnix` is an error-tolerant parser it always returns a tree, but
/// individual nodes may be `None` when their structure could not be
/// determined. Those nodes are skipped.
pub fn extract_entries(src: &str, file_path: &Path) -> Vec<RawEntry> {
  let root = rnix::Root::parse(src).tree();

  let mut entries = Vec::new();

  // rnix::Root wraps a single body expression. In flat assignment files (most
  // nixpkgs-style files) the body is itself an AttrSet; in others it may be a
  // `let … in` expression.  Rather than hardcoding the root as "has entries",
  // we recurse into the root's expression tree directly.
  if let Some(body) = root.expr() {
    collect_from_expr(body.syntax(), &[], file_path, src, &mut entries);
  }

  entries
}

/// Recursively walk an expression node, collecting doc-commented bindings.
///
/// Only `NODE_ATTR_SET` and `NODE_LET_IN` nodes can directly contain attribute
/// entries. For any other node we descend into all children looking for nested
/// attrsets (e.g. the body of a lambda or a conditional).
///
/// `path_prefix` holds the attribute path segments accumulated from ancestor
/// attrsets so that nested entries are reported with their full dotted path.
fn collect_from_expr(
  node: &rnix::SyntaxNode,
  path_prefix: &[String],
  file_path: &Path,
  src: &str,
  out: &mut Vec<RawEntry>,
) {
  use SyntaxKind::{NODE_ATTR_SET, NODE_LET_IN};

  match node.kind() {
    NODE_ATTR_SET => {
      if let Some(attrset) = ast::AttrSet::cast(node.clone()) {
        collect_entries(attrset.entries(), path_prefix, file_path, src, out);
      }
    },
    NODE_LET_IN => {
      if let Some(let_in) = ast::LetIn::cast(node.clone()) {
        collect_entries(let_in.entries(), path_prefix, file_path, src, out);
      }
    },
    _ => {
      // Not an entry container — descend into all children looking for nested
      // attribute sets (e.g. the body of a lambda or a with expression).
      for child in node.children() {
        collect_from_expr(&child, path_prefix, file_path, src, out);
      }
    },
  }
}

/// Walk the [`ast::Entry`] iterator for one attrset/let-in level, extract
/// doc-commented bindings and recurse into nested attrsets.
fn collect_entries(
  entries: impl Iterator<Item = ast::Entry>,
  path_prefix: &[String],
  file_path: &Path,
  src: &str,
  out: &mut Vec<RawEntry>,
) {
  for entry in entries {
    let ast::Entry::AttrpathValue(binding) = entry else {
      continue;
    };

    let Some(attrpath) = binding.attrpath() else {
      continue;
    };
    let segments: Vec<String> =
      attrpath.attrs().filter_map(attr_to_string).collect();

    if segments.is_empty() {
      continue;
    }

    // Build the full dotted path (e.g. ["lib", "strings"] + ["concat"] →
    // "lib.strings.concat").
    let full_path: Vec<String> =
      path_prefix.iter().chain(segments.iter()).cloned().collect();

    if let Some(comment) = preceding_doc_comment(binding.syntax()) {
      let location = Location {
        file: file_path.to_path_buf(),
        line: line_of_offset(src, binding.syntax().text_range().start().into()),
      };

      out.push(RawEntry {
        attr_path: full_path.clone(),
        comment,
        location,
      });
    }

    // recurse into RHS if it is an attrset
    if let Some(value) = binding.value() {
      collect_from_expr(value.syntax(), &full_path, file_path, src, out);
    }
  }
}

/// Convert an [`ast::Attr`] node into a plain `String` segment.
///
/// Handles simple identifiers and string-literal keys. Dynamic keys
/// (e.g. `${ expr }`) cannot be statically resolved and are skipped
/// (returning `None`).
fn attr_to_string(attr: ast::Attr) -> Option<String> {
  match attr {
    ast::Attr::Ident(id) => Some(id.to_string()),
    ast::Attr::Str(s) => {
      // Only static string keys (no interpolations).
      let parts = s.normalized_parts();
      if parts.len() == 1
        && let ast::InterpolPart::Literal(lit) = &parts[0]
      {
        return Some(lit.clone());
      }
      None
    },
    ast::Attr::Dynamic(_) => None,
  }
}

/// Walk the token stream backwards from `node` to find the immediately
/// preceding `/** ... */` doc comment, skipping whitespace tokens.
///
/// Only a single Nixdoc block comment is returned. If the nearest non-
/// whitespace token is a plain `# ...` line comment or a regular `/* */`
/// block comment, `None` is returned.
fn preceding_doc_comment(node: &rnix::SyntaxNode) -> Option<String> {
  use rnix::SyntaxKind::{TOKEN_COMMENT, TOKEN_WHITESPACE};
  use rowan::Direction;

  // Walk backwards through siblings/tokens from just before this node.
  // `.skip(1)` skips the node itself (the first element of the iterator).
  let mut cursor = node.siblings_with_tokens(Direction::Prev).skip(1);

  let mut found: Option<String> = None;

  for sibling in cursor.by_ref() {
    match sibling.kind() {
      TOKEN_WHITESPACE => {},
      TOKEN_COMMENT => {
        let text = sibling.to_string();
        if is_doc_comment(&text) {
          // Take only the innermost (closest) doc comment.
          if found.is_none() {
            found = Some(text);
          }
        }
        // Stop after any comment (doc or plain): a plain `# ...` or `/* */`
        // separates this binding from any doc comment further up.
        break;
      },
      _ => break,
    }
  }

  found
}

/// Return `true` when `text` is a Nixdoc block comment (`/** ... */`).
///
/// Nixdoc block comments start with `/**` (double asterisk), distinguishing
/// them from regular `/* ... */` block comments.
fn is_doc_comment(text: &str) -> bool {
  let trimmed = text.trim_start();
  trimmed.starts_with("/**") && trimmed.ends_with("*/")
}

/// Return the 1-based line number for a byte offset within `src`.
fn line_of_offset(src: &str, offset: usize) -> u32 {
  let safe_offset = offset.min(src.len());
  let line = src[..safe_offset].chars().filter(|&c| c == '\n').count();
  u32::try_from(line + 1).unwrap_or(u32::MAX)
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
  fn test_is_doc_comment() {
    assert!(is_doc_comment("/** hello */"));
    assert!(is_doc_comment("/**\n  body\n*/"));
    assert!(!is_doc_comment("/* regular block */"));
    assert!(!is_doc_comment("# line comment"));
  }
}
