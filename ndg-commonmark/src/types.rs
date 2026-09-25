//! Types for ndg-commonmark public API and internal use.
use serde::{Deserialize, Serialize};

/// Represents a header in a Markdown document.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Header {
  /// Header text (inline content, no markdown formatting).
  pub text:  String,
  /// Header level (1-6).
  pub level: u8,
  /// Generated or explicit anchor ID for the header.
  pub id:    String,
}

/// Represents a file that was included via `{=include=}` directive.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IncludedFile {
  /// Path to the included file.
  pub path:          String,
  /// Optional custom output path from `html:into-file` directive.
  pub custom_output: Option<String>,
}

/// A single duplicate anchor occurrence with context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuplicateAnchor {
  /// The duplicated anchor ID.
  pub id:           String,
  /// Text of the first heading with this ID.
  pub first_text:   String,
  /// Level of the first heading.
  pub first_level:  u8,
  /// Text of the second heading with this ID.
  pub second_text:  String,
  /// Level of the second heading.
  pub second_level: u8,
}

impl std::fmt::Display for DuplicateAnchor {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "Duplicate anchor ID '{}' found: '{}' (h{}) and '{}' (h{})",
      self.id,
      self.first_text,
      self.first_level,
      self.second_text,
      self.second_level
    )
  }
}

/// Error returned when duplicate anchor IDs are detected.
#[derive(Debug, Clone)]
pub struct DuplicateAnchorError {
  /// All duplicate anchors found in the document.
  pub duplicates: Vec<DuplicateAnchor>,
}

impl std::fmt::Display for DuplicateAnchorError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    writeln!(f, "Found {} duplicate anchor ID(s):", self.duplicates.len())?;
    for dup in &self.duplicates {
      writeln!(f, "  {dup}")?;
    }
    Ok(())
  }
}

impl std::error::Error for DuplicateAnchorError {}

/// Make all rendered anchor IDs unique with deterministic `-1`, `-2`, etc.
/// suffixes, updating matching heading metadata for navigation and search.
///
/// The first occurrence keeps its ID. Generated suffixes skip IDs already
/// present anywhere in the document, preserving explicit anchors even when
/// they occur after a duplicate.
///
/// Returns a list of `(old_id, new_id)` renames for logging.
pub fn deduplicate_anchor_ids(
  headers: &mut [Header],
  html: &mut String,
) -> Vec<(String, String)> {
  use std::collections::VecDeque;

  use rustc_hash::{FxHashMap, FxHashSet};

  let mut reserved = FxHashSet::default();
  let mut rest = html.as_str();
  let mut has_duplicates = false;
  while let Some(start) = rest.find("id=\"") {
    rest = &rest[start + 4..];
    let Some(end) = rest.find('"') else {
      break;
    };
    if !reserved.insert(&rest[..end]) {
      has_duplicates = true;
    }
    rest = &rest[end + 1..];
  }
  if !has_duplicates {
    return Vec::new();
  }

  // Only actual headings and generated inline anchors correspond to Header
  // entries. Raw HTML IDs must not consume a heading's occurrence.
  let mut header_positions: FxHashMap<(String, u8), VecDeque<usize>> =
    FxHashMap::default();
  for (index, header) in headers.iter().enumerate() {
    header_positions
      .entry((header.id.clone(), header.level))
      .or_default()
      .push_back(index);
  }

  let mut used = FxHashSet::default();
  let mut next_suffix: FxHashMap<&str, usize> = FxHashMap::default();
  let mut renames = Vec::new();
  let mut result = String::with_capacity(html.len());
  let mut rest = html.as_str();
  while let Some(start) = rest.find("id=\"") {
    result.push_str(&rest[..start + 4]);
    rest = &rest[start + 4..];
    let Some(end) = rest.find('"') else {
      result.push_str(rest);
      rest = "";
      break;
    };
    let old = &rest[..end];
    let tag = result.rsplit('<').next().unwrap_or("");
    let level = tag.as_bytes().get(0..2).and_then(|bytes| {
      match bytes {
        [b'h', digit @ b'1'..=b'6'] if tag.as_bytes().get(2) == Some(&b' ') => {
          Some(digit - b'0')
        },
        _ => None,
      }
    });
    let level = level.or_else(|| {
      (tag.starts_with("span ")
        && rest[end + 1..]
          .split_once('>')
          .is_some_and(|(attrs, _)| attrs.contains("nixos-anchor")))
      .then_some(2)
    });

    let new_id = if used.insert(old.to_owned()) {
      None
    } else {
      let suffix = next_suffix.entry(old).or_insert(1);
      let candidate = loop {
        let candidate = format!("{old}-{suffix}");
        *suffix += 1;
        if !reserved.contains(candidate.as_str()) && !used.contains(&candidate)
        {
          break candidate;
        }
      };
      used.insert(candidate.clone());
      renames.push((old.to_owned(), candidate.clone()));
      Some(candidate)
    };
    if let Some(level) = level
      && let Some(index) = header_positions
        .get_mut(&(old.to_owned(), level))
        .and_then(VecDeque::pop_front)
      && let Some(new_id) = &new_id
    {
      headers[index].id.clone_from(new_id);
    }
    result.push_str(new_id.as_deref().unwrap_or(old));
    rest = &rest[end..];
  }
  result.push_str(rest);
  *html = result;
  renames
}

/// Validate that all heading anchor IDs are unique.
///
/// Returns `Err(DuplicateAnchorError)` if any two headings share the same
/// anchor ID, which would produce invalid HTML and broken navigation links.
///
/// # Arguments
///
/// * `headers` - Slice of extracted headers to validate
///
/// # Errors
///
/// Returns an error listing all duplicate anchor IDs found.
pub fn validate_anchor_ids(
  headers: &[Header],
) -> Result<(), DuplicateAnchorError> {
  use rustc_hash::FxHashMap;

  let mut seen: FxHashMap<&str, (&str, u8)> = FxHashMap::default();
  let mut duplicates = Vec::new();

  for header in headers {
    if let Some(&(first_text, first_level)) = seen.get(header.id.as_str()) {
      duplicates.push(DuplicateAnchor {
        id: header.id.clone(),
        first_text: first_text.to_string(),
        first_level,
        second_text: header.text.clone(),
        second_level: header.level,
      });
    } else {
      seen.insert(&header.id, (&header.text, header.level));
    }
  }

  if duplicates.is_empty() {
    Ok(())
  } else {
    Err(DuplicateAnchorError { duplicates })
  }
}

/// Validate that all IDs in rendered HTML are unique.
///
/// # Errors
///
/// Returns an error listing all duplicate anchor IDs found.
pub fn validate_rendered_anchor_ids(
  headers: &[Header],
  html: &str,
) -> Result<(), DuplicateAnchorError> {
  validate_anchor_ids(headers)?;

  let mut seen = rustc_hash::FxHashSet::default();
  let mut duplicate_ids = Vec::new();
  let mut rest = html;
  while let Some(start) = rest.find("id=\"") {
    rest = &rest[start + 4..];
    let Some(end) = rest.find('"') else {
      break;
    };
    let id = &rest[..end];
    if !seen.insert(id) && !duplicate_ids.iter().any(|seen| seen == id) {
      duplicate_ids.push(id.to_owned());
    }
    rest = &rest[end + 1..];
  }

  if duplicate_ids.is_empty() {
    return Ok(());
  }

  let duplicates = duplicate_ids
    .into_iter()
    .map(|id| {
      let heading = headers.iter().find(|header| header.id == id);
      let (first_text, first_level) = heading.map_or_else(
        || ("non-heading anchor".to_owned(), 0),
        |header| (header.text.clone(), header.level),
      );
      DuplicateAnchor {
        id,
        first_text,
        first_level,
        second_text: "non-heading anchor".to_owned(),
        second_level: 0,
      }
    })
    .collect();

  Err(DuplicateAnchorError { duplicates })
}

/// Result of Markdown processing.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MarkdownResult {
  /// Rendered HTML output.
  pub html: String,

  /// Extracted headers (for `ToC`, navigation, etc).
  pub headers: Vec<Header>,

  /// Title of the document, if found (usually first H1).
  pub title: Option<String>,

  /// Files that were included via `{=include=}` directives.
  pub included_files: Vec<IncludedFile>,
}

#[cfg(test)]
#[expect(clippy::unwrap_used, reason = "Fine in tests")]
mod tests {
  use super::*;

  #[test]
  fn test_validate_anchor_ids_unique() {
    let headers = vec![
      Header {
        text:  "First".to_string(),
        level: 1,
        id:    "first".to_string(),
      },
      Header {
        text:  "Second".to_string(),
        level: 2,
        id:    "second".to_string(),
      },
    ];
    assert!(validate_anchor_ids(&headers).is_ok());
  }

  #[test]
  fn test_validate_anchor_ids_duplicate() {
    let headers = vec![
      Header {
        text:  "First".to_string(),
        level: 1,
        id:    "same".to_string(),
      },
      Header {
        text:  "Second".to_string(),
        level: 2,
        id:    "same".to_string(),
      },
    ];
    let err = validate_anchor_ids(&headers).unwrap_err();
    assert_eq!(err.duplicates.len(), 1);
    assert_eq!(err.duplicates[0].id, "same");
  }

  #[test]
  fn test_validate_anchor_ids_multiple_duplicates() {
    let headers = vec![
      Header {
        text:  "A".to_string(),
        level: 1,
        id:    "x".to_string(),
      },
      Header {
        text:  "B".to_string(),
        level: 2,
        id:    "x".to_string(),
      },
      Header {
        text:  "C".to_string(),
        level: 1,
        id:    "y".to_string(),
      },
      Header {
        text:  "D".to_string(),
        level: 2,
        id:    "y".to_string(),
      },
    ];
    let err = validate_anchor_ids(&headers).unwrap_err();
    assert_eq!(err.duplicates.len(), 2);
  }

  #[test]
  fn test_validate_anchor_ids_empty() {
    assert!(validate_anchor_ids(&[]).is_ok());
  }

  #[test]
  fn test_deduplicate_anchor_ids_suffixes() {
    let mut headers = vec![
      Header {
        text:  "Inputs".to_string(),
        level: 3,
        id:    "inputs".to_string(),
      },
      Header {
        text:  "Inputs".to_string(),
        level: 3,
        id:    "inputs".to_string(),
      },
      Header {
        text:  "Type".to_string(),
        level: 3,
        id:    "type".to_string(),
      },
    ];
    let mut html = String::from(
      "<h3 id=\"inputs\">Inputs</h3><h3 id=\"inputs\">Inputs</h3><h3 \
       id=\"type\">Type</h3>",
    );

    let renames = deduplicate_anchor_ids(&mut headers, &mut html);
    assert_eq!(headers[0].id, "inputs");
    assert_eq!(headers[1].id, "inputs-1");
    assert_eq!(headers[2].id, "type");
    assert!(html.contains("id=\"inputs-1\""));
    assert_eq!(renames, vec![(
      "inputs".to_string(),
      "inputs-1".to_string()
    )]);
    assert!(validate_anchor_ids(&headers).is_ok());
    assert!(validate_rendered_anchor_ids(&headers, &html).is_ok());
  }

  #[test]
  fn test_deduplicate_skips_existing_suffixed_id() {
    let mut headers = vec![
      Header {
        text:  "A".to_string(),
        level: 2,
        id:    "x".to_string(),
      },
      Header {
        text:  "B".to_string(),
        level: 2,
        id:    "x-1".to_string(),
      },
      Header {
        text:  "C".to_string(),
        level: 2,
        id:    "x".to_string(),
      },
    ];
    let mut html = String::from(
      "<h2 id=\"x\">A</h2><h2 id=\"x-1\">B</h2><h2 id=\"x\">C</h2>",
    );

    deduplicate_anchor_ids(&mut headers, &mut html);
    // Third header must skip the taken `x-1` and become `x-2`.
    assert_eq!(headers[2].id, "x-2");
    assert!(html.contains("id=\"x-2\""));
  }

  #[test]
  fn test_deduplicate_preserves_heading_links_with_non_heading_collision() {
    let mut headers = vec![
      Header {
        text:  "Inputs".to_string(),
        level: 2,
        id:    "inputs".to_string(),
      },
      Header {
        text:  "Inputs".to_string(),
        level: 2,
        id:    "inputs".to_string(),
      },
      Header {
        text:  "Reserved".to_string(),
        level: 2,
        id:    "inputs-1".to_string(),
      },
    ];
    let mut html = String::from(
      "<span id=\"inputs\"></span><h2 id=\"inputs\">Inputs</h2><h2 \
       id=\"inputs\">Inputs</h2><h2 id=\"inputs-1\">Reserved</h2>",
    );

    deduplicate_anchor_ids(&mut headers, &mut html);
    assert_eq!(
      headers
        .iter()
        .map(|header| header.id.as_str())
        .collect::<Vec<_>>(),
      ["inputs-2", "inputs-3", "inputs-1"]
    );
    assert!(html.contains("<h2 id=\"inputs-2\">Inputs</h2>"));
    assert!(html.contains("<h2 id=\"inputs-3\">Inputs</h2>"));
    assert!(html.contains("<h2 id=\"inputs-1\">Reserved</h2>"));
    validate_rendered_anchor_ids(&headers, &html).unwrap();
  }

  #[test]
  fn test_deduplicate_updates_inline_anchor_metadata() {
    let mut headers = vec![Header {
      text:  "Inline anchor".to_string(),
      level: 2,
      id:    "shared".to_string(),
    }];
    let mut html = String::from(
      "<span id=\"shared\"></span><span id=\"shared\" \
       class=\"nixos-anchor\"></span>",
    );

    deduplicate_anchor_ids(&mut headers, &mut html);
    assert_eq!(headers[0].id, "shared-1");
    assert!(html.contains("<span id=\"shared-1\" class=\"nixos-anchor\">"));
    validate_rendered_anchor_ids(&headers, &html).unwrap();
  }
}
