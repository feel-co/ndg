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
pub(crate) fn validate_rendered_anchor_ids(
  headers: &[Header],
  html: &str,
) -> Result<(), DuplicateAnchorError> {
  validate_anchor_ids(headers)?;

  let mut seen = rustc_hash::FxHashMap::default();
  let mut duplicate_ids = Vec::new();
  let mut rest = html;
  while let Some(start) = rest.find("id=\"") {
    rest = &rest[start + 4..];
    let Some(end) = rest.find('"') else {
      break;
    };
    let id = &rest[..end];
    if seen.insert(id, ()).is_some()
      && !duplicate_ids.iter().any(|seen| seen == id)
    {
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
      let (first_text, first_level) = heading
        .map(|header| (header.text.clone(), header.level))
        .unwrap_or_else(|| ("non-heading anchor".to_owned(), 0));
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
}
