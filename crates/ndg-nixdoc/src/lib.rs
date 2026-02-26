//! `ndg-nixdoc`.
//!
//! This crate walks one or more `.nix` source files, locates attribute
//! bindings preceded by `/** ... */` doc comments, and returns structured
//! documentation entries. The `nixdoc` crate is always used to parse
//! doc-comment content into [`ParsedDoc`].
//!
//! If you want to make Nix doc-comment support optional in a consumer crate,
//! gate the dependency on `ndg-nixdoc` itself behind a feature flag there.
//!
//! # Example
//!
//! ```no_run
//! use ndg_nixdoc::extract_from_file;
//!
//! let entries = extract_from_file("lib/strings.nix").unwrap();
//! for entry in &entries {
//!   println!("{}: {}", entry.attr_path.join("."), entry.raw_comment);
//! }
//! ```

pub mod error;
mod extractor;
mod types;

use std::path::Path;

pub use error::NixdocError;
pub use types::{
  Location,
  NixDocEntry,
  ParsedArgument,
  ParsedDoc,
  ParsedExample,
};

/// Extract all doc-commented attribute bindings from a single `.nix` file.
///
/// Returns one [`NixDocEntry`] per binding that has an immediately preceding
/// `/** ... */` doc comment. The entries are in source order.
///
/// # Errors
///
/// Returns [`NixdocError::ReadFile`] if the file cannot be read.
pub fn extract_from_file(
  path: impl AsRef<Path>,
) -> Result<Vec<NixDocEntry>, NixdocError> {
  let path = path.as_ref();
  let src = std::fs::read_to_string(path).map_err(|source| {
    NixdocError::ReadFile {
      path: path.to_path_buf(),
      source,
    }
  })?;

  let raw_entries = extractor::extract_entries(&src, path);
  Ok(raw_entries.into_iter().map(types::into_entry).collect())
}

/// Extract doc-commented bindings from every `.nix` file under `dir`,
/// walking the directory tree recursively.
///
/// Files that cannot be read are skipped; a warning is emitted via the `log`
/// crate.
///
/// # Errors
///
/// Returns [`NixdocError::ReadFile`] only when the top-level `dir` itself
/// cannot be accessed. Individual file read errors are logged and skipped.
pub fn extract_from_dir(
  dir: impl AsRef<Path>,
) -> Result<Vec<NixDocEntry>, NixdocError> {
  use walkdir::WalkDir;

  let dir = dir.as_ref();
  let mut entries = Vec::new();

  // WalkDir immediately tries to open the directory when iterating. If it
  // fails, the first entry will be an error. We convert that to our error
  // type.
  let walker = WalkDir::new(dir).follow_links(true).into_iter();

  let mut iter = walker.peekable();
  if matches!(iter.peek(), Some(Err(_))) {
    if let Some(Err(e)) = iter.next() {
      return Err(NixdocError::ReadFile {
        path:   dir.to_path_buf(),
        // Preserve the real OS ErrorKind via walkdir's From impl.
        source: e.into(),
      });
    }
  }

  for result in iter {
    let dent = match result {
      Ok(d) => d,
      Err(e) => {
        log::warn!("ndg-nixdoc: skipping unreadable directory entry: {e}");
        continue;
      },
    };

    if dent.file_type().is_dir() {
      continue;
    }

    if dent.path().extension().and_then(|e| e.to_str()) != Some("nix") {
      continue;
    }

    match extract_from_file(dent.path()) {
      Ok(file_entries) => entries.extend(file_entries),
      Err(e) => {
        log::warn!("ndg-nixdoc: skipping `{}`: {e}", dent.path().display());
      },
    }
  }

  Ok(entries)
}
