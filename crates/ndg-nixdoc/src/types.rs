use std::path::PathBuf;

/// Internal representation of a located attribute binding with its raw
/// comment string, before doc-comment parsing is applied.
pub struct RawEntry {
  /// Full attribute path segments (e.g. `["lib", "strings", "concat"]`).
  pub attr_path: Vec<String>,

  /// The raw `/** ... */` comment text as it appears in the source.
  pub comment: String,

  /// Source location of the attribute binding.
  pub location: Location,
}

/// Source location of a Nix attribute binding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Location {
  /// Path to the `.nix` source file.
  pub file: PathBuf,
  /// 1-based line number of the attribute binding.
  pub line: u32,
}

/// A fully-parsed doc comment, as interpreted by the `nixdoc` crate.
#[derive(Debug, Clone)]
pub struct ParsedDoc {
  /// Short one-line title (first non-empty line of the description).
  pub title:              Option<String>,
  /// Full description text (everything before the first `#` section).
  pub description:        String,
  /// Type signature from a `# Type` section or an inline `id :: type`
  /// annotation.
  pub type_sig:           Option<String>,
  /// Named arguments from a `# Arguments` / `# Args` section.
  pub arguments:          Vec<ParsedArgument>,
  /// Code examples from `# Example` / `# Examples` sections.
  pub examples:           Vec<ParsedExample>,
  /// Notes from `# Note` / `# Notes` sections.
  pub notes:              Vec<String>,
  /// Warnings from `# Warning` / `# Warnings` / `# Caution` sections.
  pub warnings:           Vec<String>,
  /// Whether a `# Deprecated` section is present.
  pub is_deprecated:      bool,
  /// Content of the `# Deprecated` section, if any.
  pub deprecation_notice: Option<String>,
}

/// A documented function argument from a `# Arguments` section.
#[derive(Debug, Clone)]
pub struct ParsedArgument {
  /// Argument name (from `- [name] description` syntax).
  pub name:        String,
  /// Human-readable description of the argument.
  pub description: String,
}

/// A documentation code example from an `# Example` / `# Examples` section.
#[derive(Debug, Clone)]
pub struct ParsedExample {
  /// Optional language hint from the fenced code block (e.g. `"nix"`).
  pub language: Option<String>,
  /// The raw code content.
  pub code:     String,
}

/// A Nix attribute binding that has an associated `/** ... */` doc comment.
#[derive(Debug, Clone)]
pub struct NixDocEntry {
  /// Fully-qualified attribute path segments.
  ///
  /// For a binding `concatStrings = ...;` nested inside `lib.strings`,
  /// this will be `["lib", "strings", "concatStrings"]`.
  pub attr_path:   Vec<String>,
  /// The raw comment text, including the `/**` and `*/` delimiters.
  pub raw_comment: String,
  /// Structured doc comment, parsed by the `nixdoc` crate.
  ///
  /// `None` only when the `nixdoc` crate cannot parse the comment (the raw
  /// text is still available via [`Self::raw_comment`]).
  pub doc:         Option<ParsedDoc>,
  /// Source location of the attribute binding.
  pub location:    Location,
}

/// Convert a [`RawEntry`] into a [`NixDocEntry`], parsing the doc comment
/// via the `nixdoc` crate.
pub fn into_entry(raw: RawEntry, file_path: &std::path::Path) -> NixDocEntry {
  let doc = parse_doc_comment(&raw.comment, file_path, &raw.attr_path);

  NixDocEntry {
    attr_path: raw.attr_path,
    raw_comment: raw.comment,
    doc,
    location: raw.location,
  }
}

/// Parse a raw `/** ... */` comment string using the `nixdoc` crate.
///
/// Parse warnings are logged at `WARN` level but do not prevent a result from
/// being returned. Parse errors are logged and cause `None` to be returned.
fn parse_doc_comment(
  raw: &str,
  file_path: &std::path::Path,
  attr_path: &[String],
) -> Option<ParsedDoc> {
  let parsed = match nixdoc::DocComment::parse(raw) {
    Ok(p) => p,
    Err(e) => {
      log::warn!(
        "ndg-nixdoc: could not parse doc comment for `{}` in `{}`: {e}",
        attr_path.join("."),
        file_path.display(),
      );
      return None;
    },
  };

  for warn in &parsed.warnings {
    log::warn!(
      "ndg-nixdoc: doc comment warning for `{}` in `{}`: {warn:?}",
      attr_path.join("."),
      file_path.display(),
    );
  }

  let arguments = parsed
    .arguments()
    .into_iter()
    .map(|a| {
      ParsedArgument {
        name:        a.name,
        description: a.description,
      }
    })
    .collect();

  let examples = parsed
    .examples()
    .into_iter()
    .map(|e| {
      ParsedExample {
        language: e.language,
        code:     e.code,
      }
    })
    .collect();

  let notes = parsed.notes().into_iter().map(str::to_owned).collect();
  let warnings = parsed
    .warnings_content()
    .into_iter()
    .map(str::to_owned)
    .collect();

  Some(ParsedDoc {
    title: parsed.title().map(str::to_owned),
    description: parsed.description().to_owned(),
    type_sig: parsed.type_sig(),
    arguments,
    examples,
    notes,
    warnings,
    is_deprecated: parsed.is_deprecated(),
    deprecation_notice: parsed.deprecation_notice().map(str::to_owned),
  })
}
