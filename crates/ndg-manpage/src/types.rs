use serde_json::Value;

/// Represents a NixOS configuration option.
///
/// This struct holds all relevant metadata for a single NixOS module option,
/// including its name, type, description, default and example values,
/// declaration location, and visibility flags. Used for rendering options
/// documentation.
#[derive(Debug, Clone, Default)]
pub struct NixOption {
  /// Option name (e.g., "services.nginx.enable")
  pub name: String,

  /// Option type (e.g., "boolean", "string", "signed integer", etc.)
  pub type_name: String,

  /// Option description
  pub description: String,

  /// Option default value (JSON)
  pub default: Option<Value>,

  /// Option default value as text
  pub default_text: Option<String>,

  /// Option example value (JSON)
  pub example: Option<Value>,

  /// Option example value as text
  pub example_text: Option<String>,

  /// Path to the file where the option is declared
  pub declared_in: Option<String>,

  /// Option declaration URL for hyperlink
  pub declared_in_url: Option<String>,

  /// Whether this option is internal
  pub internal: bool,

  /// Whether this option is read-only
  pub read_only: bool,
}
