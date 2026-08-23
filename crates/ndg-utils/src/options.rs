//! Typed representation of the JSON emitted by `nixosOptionsDoc`.

use std::cmp::Ordering;

use indexmap::IndexMap;
use serde::{Deserialize, Deserializer};
use thiserror::Error;

/// A complete `nixosOptionsDoc` JSON document keyed by option name.
pub type NixOptionsDocument = IndexMap<String, NixOptionDocument>;

/// One option entry from `nixosOptionsDoc`.
///
/// Unknown fields remain accepted because `transformOptions` may add metadata.
/// Known fields are typed so malformed input fails at the field that caused the
/// problem instead of silently rendering an empty value.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NixOptionDocument {
  /// Human-readable Nix option type.
  #[serde(rename = "type")]
  pub type_name: String,

  /// Option description, either plain Markdown or a literal wrapper.
  #[serde(default)]
  pub description: Option<DocumentationText>,

  /// Rendered default value.
  #[serde(default, deserialize_with = "deserialize_documented_value")]
  pub default: Option<DocumentedValue>,

  /// Rendered example value.
  #[serde(default, deserialize_with = "deserialize_documented_value")]
  pub example: Option<DocumentedValue>,

  /// Locations that declare the option.
  #[serde(default)]
  pub declarations: Vec<OptionLocation>,

  /// Locations that define the option. This is an NDG-compatible extension.
  #[serde(default)]
  pub definitions: Vec<OptionLocation>,

  /// Option attribute path as separate components.
  pub loc: Vec<String>,

  /// Whether the option is internal.
  #[serde(default)]
  pub internal: bool,

  /// Whether the option is read-only.
  #[serde(default)]
  pub read_only: bool,

  /// Visibility metadata retained by custom `transformOptions` functions.
  #[serde(default)]
  pub visible: Option<OptionVisibility>,

  /// Legacy direct declaration URL.
  #[serde(default, rename = "declarationURL")]
  pub declaration_url: Option<String>,

  /// Markdown links for packages related to this option.
  #[serde(default)]
  pub related_packages: Option<String>,
}

impl NixOptionDocument {
  /// Return whether this entry should be treated as hidden or internal.
  #[must_use]
  pub fn is_hidden(&self) -> bool {
    self.internal
      || self
        .visible
        .as_ref()
        .is_some_and(OptionVisibility::hides_option)
  }
}

/// Markdown-bearing text used by descriptions and related package metadata.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum DocumentationText {
  /// Plain Markdown text.
  Plain(String),

  /// Markdown wrapped by `lib.mdDoc`.
  Markdown(MarkdownDocumentation),
}

impl DocumentationText {
  /// Return the contained text.
  #[must_use]
  pub fn text(&self) -> &str {
    match self {
      Self::Plain(text)
      | Self::Markdown(MarkdownDocumentation::Markdown { text }) => text,
    }
  }

  /// Return whether the text came from an explicit `mdDoc` wrapper.
  #[must_use]
  pub const fn is_markdown_documentation(&self) -> bool {
    matches!(self, Self::Markdown(_))
  }
}

/// Structured Markdown documentation produced by `lib.mdDoc`.
#[derive(Debug, Clone, Deserialize)]
#[serde(
  tag = "_type",
  deny_unknown_fields,
  expecting = "an mdDoc object with a string `text` field"
)]
pub enum MarkdownDocumentation {
  /// Markdown documentation.
  #[serde(rename = "mdDoc")]
  Markdown {
    /// Markdown contents.
    text: String,
  },
}

/// A rendered option value emitted by `nixosOptionsDoc`.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(
  tag = "_type",
  deny_unknown_fields,
  expecting = "a literalExpression or literalMD object with a string `text` field"
)]
pub enum DocumentedValue {
  /// Verbatim Nix source rendered as a Nix code block.
  #[serde(rename = "literalExpression")]
  LiteralExpression {
    /// Nix expression source.
    text: String,
  },

  /// Markdown rendered as documentation content.
  #[serde(rename = "literalMD")]
  LiteralMarkdown {
    /// Markdown contents.
    text: String,
  },
}

fn deserialize_documented_value<'de, D>(
  deserializer: D,
) -> Result<Option<DocumentedValue>, D::Error>
where
  D: Deserializer<'de>,
{
  DocumentedValue::deserialize(deserializer).map(Some)
}

/// Compare option locations using `nixos-render-docs` ordering.
///
/// Components beginning with `enable` sort first, followed by components
/// beginning with `package`, then all other components alphabetically.
#[must_use]
pub fn compare_option_locs(
  left: &[String],
  right: &[String],
) -> Ordering {
  fn priority(component: &str) -> u8 {
    if component.starts_with("enable") {
      0
    } else if component.starts_with("package") {
      1
    } else {
      2
    }
  }

  left
    .iter()
    .map(|component| (priority(component), component))
    .cmp(
      right
        .iter()
        .map(|component| (priority(component), component)),
    )
}

/// A declaration or definition location.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum OptionLocation {
  /// A path relative to Nixpkgs, or an absolute local path.
  Path(String),

  /// A named source link emitted by `transformOptions`.
  Link {
    /// Human-readable source name.
    #[serde(default)]
    name: Option<String>,

    /// Source URL.
    #[serde(default)]
    url: Option<String>,
  },
}

/// Visibility forms accepted by the Nix module system.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum OptionVisibility {
  /// Boolean visibility used by `nixosOptionsDoc` output.
  Boolean(bool),

  /// Raw module-system visibility mode retained by custom exporters.
  Mode(VisibilityMode),
}

impl OptionVisibility {
  fn hides_option(&self) -> bool {
    match self {
      Self::Boolean(visible) => !visible,
      Self::Mode(VisibilityMode::Transparent) => true,
      Self::Mode(VisibilityMode::Shallow) => false,
    }
  }
}

/// Named module-system visibility modes.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VisibilityMode {
  /// Show this option while hiding its sub-options.
  Shallow,

  /// Hide this option while retaining visible sub-options.
  Transparent,
}

/// Error returned while parsing an options JSON document.
#[derive(Debug, Error)]
pub enum OptionsParseError {
  /// A value did not match the schema at the reported JSON path.
  #[error("{0}")]
  Schema(serde_path_to_error::Error<serde_json::Error>),

  /// Extra non-whitespace data followed the JSON document.
  #[error("{0}")]
  Trailing(serde_json::Error),
}

/// Parse and validate a `nixosOptionsDoc` JSON document.
///
/// # Errors
///
/// Returns an error with the exact option and field path when a known field has
/// the wrong type, a required field is absent, or the JSON syntax is invalid.
pub fn parse_options_json(
  input: &str,
) -> Result<NixOptionsDocument, OptionsParseError> {
  let mut deserializer = serde_json::Deserializer::from_str(input);
  let document = serde_path_to_error::deserialize(&mut deserializer)
    .map_err(OptionsParseError::Schema)?;
  deserializer.end().map_err(OptionsParseError::Trailing)?;
  Ok(document)
}

#[cfg(test)]
mod tests {
  #![allow(clippy::expect_used, reason = "Fine in tests")]

  use super::*;

  #[test]
  fn test_options_parser_accepts_current_nixos_options_doc_schema() {
    let input = r#"{
  "services.example.enable": {
    "declarations": [
      "nixos/modules/services/example.nix",
      {"name": "module.nix", "url": "https://example.test/module.nix"}
    ],
    "default": {"_type": "literalExpression", "text": "false"},
    "description": {"_type": "mdDoc", "text": "Enable **example**."},
    "example": {"_type": "literalExpression", "text": "true"},
    "loc": ["services", "example", "enable"],
    "readOnly": false,
    "relatedPackages": "- [`pkgs.example`](https://example.test)",
    "type": "boolean"
  }
}"#;

    let document = parse_options_json(input).expect("valid options JSON");
    let option = document
      .get("services.example.enable")
      .expect("option entry");

    assert_eq!(option.type_name, "boolean");
    assert_eq!(option.declarations.len(), 2);
    assert_eq!(option.loc, ["services", "example", "enable"]);
    assert_eq!(
      option.description.as_ref().map(DocumentationText::text),
      Some("Enable **example**.")
    );
    assert_eq!(
      option
        .related_packages
        .as_deref(),
      Some("- [`pkgs.example`](https://example.test)")
    );
    assert_eq!(
      option.default,
      Some(DocumentedValue::LiteralExpression {
        text: "false".to_string()
      })
    );
  }

  #[test]
  fn test_options_parser_accepts_normalized_null_default() {
    let input = r#"{
  "services.example.package": {
    "default": {"_type": "literalExpression", "text": "null"},
    "loc": ["services", "example", "package"],
    "type": "null or package"
  }
}"#;

    let document = parse_options_json(input).expect("valid options JSON");
    let option = document
      .get("services.example.package")
      .expect("option entry");

    assert_eq!(
      option.default,
      Some(DocumentedValue::LiteralExpression {
        text: "null".to_string()
      })
    );
    assert!(option.example.is_none());
  }

  #[test]
  fn test_options_parser_rejects_json_null_default() {
    let input = r#"{
  "services.example.package": {
    "default": null,
    "loc": ["services", "example", "package"],
    "type": "null or package"
  }
}"#;

    let error = parse_options_json(input).expect_err("raw null default");
    let message = error.to_string();

    assert!(message.contains("services.example.package.default"));
    assert!(message.contains("null"));
  }

  #[test]
  fn test_options_parser_reports_option_and_field_for_wrong_type() {
    let input = r#"{
  "services.example.enable": {
    "declarations": [],
    "loc": ["services", "example", "enable"],
    "readOnly": "sometimes",
    "type": "boolean"
  }
}"#;

    let error = parse_options_json(input).expect_err("invalid readOnly type");
    let message = error.to_string();

    assert!(message.contains("services.example.enable"));
    assert!(message.contains("readOnly"));
    assert!(message.contains("boolean"));
  }

  #[test]
  fn test_options_parser_rejects_missing_type() {
    let input = r#"{
  "services.example.enable": {
    "loc": ["services", "example", "enable"],
    "readOnly": false
  }
}"#;

    let error = parse_options_json(input).expect_err("missing type");
    let message = error.to_string();

    assert!(message.contains("services.example.enable"));
    assert!(message.contains("type"));
  }

  #[test]
  fn test_options_parser_rejects_missing_loc() {
    let input = r#"{"services.example.enable":{"type":"boolean"}}"#;

    let error = parse_options_json(input).expect_err("missing loc");
    let message = error.to_string();

    assert!(message.contains("services.example.enable"));
    assert!(message.contains("loc"));
  }

  #[test]
  fn test_options_parser_rejects_unknown_documented_value_type() {
    let input = r#"{
  "services.example.enable": {
    "default": {"_type": "literalDocBook", "text": "<literal>false</literal>"},
    "loc": ["services", "example", "enable"],
    "type": "boolean"
  }
}"#;

    let error = parse_options_json(input).expect_err("unknown default type");
    let message = error.to_string();

    assert!(message.contains("services.example.enable.default"));
    assert!(message.contains("literalDocBook"));
    assert!(message.contains("literalExpression"));
    assert!(message.contains("literalMD"));
  }

  #[test]
  fn test_options_parser_rejects_documented_value_without_text() {
    let input = r#"{
  "services.example.enable": {
    "example": {"_type": "literalExpression"},
    "loc": ["services", "example", "enable"],
    "type": "boolean"
  }
}"#;

    let error = parse_options_json(input).expect_err("missing example text");
    let message = error.to_string();

    assert!(message.contains("services.example.enable.example"));
    assert!(message.contains("text"));
  }

  #[test]
  fn test_options_parser_rejects_raw_default_value() {
    let input = r#"{
  "services.example.enable": {
    "default": false,
    "loc": ["services", "example", "enable"],
    "type": "boolean"
  }
}"#;

    let error = parse_options_json(input).expect_err("raw default");
    let message = error.to_string();

    assert!(message.contains("services.example.enable.default"));
    assert!(message.contains("boolean"));
    assert!(message.contains("literalExpression"));
    assert!(message.contains("literalMD"));
  }

  #[test]
  fn test_compare_option_locs_matches_nixos_render_docs_priority() {
    let mut locations = [
      vec!["services".to_string(), "zebra".to_string()],
      vec!["services".to_string(), "package".to_string()],
      vec!["services".to_string(), "enableFeature".to_string()],
      vec!["programs".to_string(), "alpha".to_string()],
    ];

    locations.sort_by(|left, right| compare_option_locs(left, right));

    assert_eq!(
      locations,
      [
        vec!["programs".to_string(), "alpha".to_string()],
        vec!["services".to_string(), "enableFeature".to_string()],
        vec!["services".to_string(), "package".to_string()],
        vec!["services".to_string(), "zebra".to_string()],
      ]
    );
  }
}
