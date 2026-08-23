//! Typed representation of the JSON emitted by `nixosOptionsDoc`.

use indexmap::IndexMap;
use serde::{Deserialize, Deserializer};
use serde_json::Value;
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
  #[serde(default, deserialize_with = "deserialize_present_value")]
  pub default: Option<Value>,

  /// Legacy textual default value.
  #[serde(default, deserialize_with = "deserialize_present_value")]
  pub default_text: Option<Value>,

  /// Rendered example value.
  #[serde(default, deserialize_with = "deserialize_present_value")]
  pub example: Option<Value>,

  /// Legacy textual example value.
  #[serde(default, deserialize_with = "deserialize_present_value")]
  pub example_text: Option<Value>,

  /// Locations that declare the option.
  #[serde(default)]
  pub declarations: Vec<OptionLocation>,

  /// Locations that define the option. This is an NDG-compatible extension.
  #[serde(default)]
  pub definitions: Vec<OptionLocation>,

  /// Option attribute path as separate components.
  #[serde(default)]
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
  pub related_packages: Option<DocumentationText>,
}

fn deserialize_present_value<'de, D>(
  deserializer: D,
) -> Result<Option<Value>, D::Error>
where
  D: Deserializer<'de>,
{
  Value::deserialize(deserializer).map(Some)
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

  /// Text wrapped by a Nix documentation literal helper.
  Literal(LiteralText),
}

impl DocumentationText {
  /// Return the contained text.
  #[must_use]
  pub fn text(&self) -> &str {
    match self {
      Self::Plain(text) => text,
      Self::Literal(literal) => &literal.text,
    }
  }

  /// Return whether the text is explicitly marked as literal Markdown.
  #[must_use]
  pub fn is_literal_markdown(&self) -> bool {
    matches!(self, Self::Literal(literal) if literal.kind == "literalMD")
  }
}

/// Structured documentation literal produced by Nixpkgs helpers.
#[derive(Debug, Clone, Deserialize)]
pub struct LiteralText {
  /// Literal helper name, such as `literalMD` or `literalExpression`.
  #[serde(rename = "_type")]
  pub kind: String,

  /// Literal contents.
  pub text: String,
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
    "description": {"_type": "literalMD", "text": "Enable **example**."},
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
    assert!(
      option
        .description
        .as_ref()
        .is_some_and(DocumentationText::is_literal_markdown)
    );
    assert_eq!(
      option
        .related_packages
        .as_ref()
        .map(DocumentationText::text),
      Some("- [`pkgs.example`](https://example.test)")
    );
  }

  #[test]
  fn test_options_parser_preserves_explicit_null_default() {
    let input = r#"{"services.example.package":{"default":null,"type":"null or package"}}"#;

    let document = parse_options_json(input).expect("valid options JSON");
    let option = document
      .get("services.example.package")
      .expect("option entry");

    assert_eq!(option.default, Some(Value::Null));
    assert!(option.example.is_none());
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
    let input = r#"{"services.example.enable":{"readOnly":false}}"#;

    let error = parse_options_json(input).expect_err("missing type");
    let message = error.to_string();

    assert!(message.contains("services.example.enable"));
    assert!(message.contains("type"));
  }
}
