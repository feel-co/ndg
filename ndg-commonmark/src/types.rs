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

/// Make all heading anchor IDs unique with deterministic `-1`, `-2`, etc.
/// suffixes.
///
/// The first occurrence of an ID is kept as-is; subsequent occurrences get
/// `"{id}-1"`, `"{id}-2"`, and so on (skipping IDs that already exist, e.g. an
/// author-written `{#inputs-1}`). Empty IDs are ignored.
///
/// Both `headers` and the `id="..."` attributes in `html` are rewritten in
/// document order so they stay in sync. Any remaining duplicate non-heading
/// IDs in `html` are deduplicated with the same scheme.
///
/// Returns a list of `(old_id, new_id)` renames for logging.
pub fn deduplicate_anchor_ids(
  headers: &mut [Header],
  html: &mut String,
) -> Vec<(String, String)> {
  use rustc_hash::{FxHashMap, FxHashSet};

  let mut used: FxHashSet<String> = FxHashSet::default();
  let mut next_suffix: FxHashMap<String, usize> = FxHashMap::default();
  // (old_id, occurrence_index) -> new_id, to keep headers and HTML in sync.
  let mut mapping: FxHashMap<(String, usize), String> = FxHashMap::default();
  let mut header_occ: FxHashMap<String, usize> = FxHashMap::default();
  let mut renames = Vec::new();

  for header in headers.iter_mut() {
    if header.id.is_empty() {
      continue;
    }
    let old = header.id.clone();
    let occ = header_occ.get(&old).copied().unwrap_or(0);
    header_occ.insert(old.clone(), occ + 1);

    if !used.contains(&old) {
      used.insert(old.clone());
      next_suffix.entry(old.clone()).or_insert(1);
      mapping.insert((old.clone(), occ), old);
      continue;
    }

    let mut suffix = next_suffix.get(&old).copied().unwrap_or(1);
    let new_id = loop {
      let candidate = format!("{old}-{suffix}");
      suffix += 1;
      if !used.contains(&candidate) {
        break candidate;
      }
    };
    next_suffix.insert(old.clone(), suffix);
    used.insert(new_id.clone());
    mapping.insert((old.clone(), occ), new_id.clone());
    renames.push((old, new_id.clone()));
    header.id = new_id;
  }

  // Rewrite `id="..."` attributes in document order to match the header
  // mapping. IDs not present in the mapping (e.g. syntax-highlight extras)
  // are deduplicated globally with the same suffix scheme.
  let mut html_occ: FxHashMap<String, usize> = FxHashMap::default();
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
    let occ = html_occ.get(old).copied().unwrap_or(0);
    html_occ.insert(old.to_owned(), occ + 1);

    if let Some(mapped) = mapping.get(&(old.to_owned(), occ)) {
      result.push_str(mapped);
    } else if used.contains(old) {
      // Extra HTML-only duplicate: find the next free suffix.
      let mut suffix = next_suffix.get(old).copied().unwrap_or(1);
      let new_id = loop {
        let candidate = format!("{old}-{suffix}");
        suffix += 1;
        if !used.contains(&candidate) {
          break candidate;
        }
      };
      next_suffix.insert(old.to_owned(), suffix);
      used.insert(new_id.clone());
      renames.push((old.to_owned(), new_id.clone()));
      result.push_str(&new_id);
    } else {
      used.insert(old.to_owned());
      next_suffix.entry(old.to_owned()).or_insert(1);
      result.push_str(old);
    }
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
}
