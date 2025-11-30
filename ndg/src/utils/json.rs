use serde_json::Value;

/// Extract string value from JSON structures including special types like
/// `literalExpression`
///
/// Handles both structured values such as `literalExpression`,
/// `literalDocBook`, `literalMD`) and simple scalar values (strings, numbers,
/// booleans and null).
///
/// # Arguments
///
/// * `value` - The JSON value to extract
/// * `wrap_code` - Whether to wrap `literalExpression` values in backticks
///
/// # Returns
///
/// Extracted string value, or `None` if not extractable
#[must_use]
#[inline]
pub fn extract_value(value: &Value, wrap_code: bool) -> Option<String> {
  if let Value::Object(obj) = value {
    if let Some(Value::String(type_name)) = obj.get("_type") {
      match type_name.as_str() {
        // literalDocBook and literalMD are deprecated as of 24.11
        // and supported only for backwards compatibility
        "literalExpression" | "literalDocBook" | "literalMD" => {
          if let Some(Value::String(text)) = obj.get("text") {
            if wrap_code && type_name.as_str() == "literalExpression" {
              return Some(format!("`{}`", text.clone()));
            }
            return Some(text.clone());
          }
        },
        _ => {},
      }
    }
  }

  // For simple scalar values, convert to string
  match value {
    Value::String(s) => Some(s.clone()),
    Value::Number(n) => Some(n.to_string()),
    Value::Bool(b) => Some(b.to_string()),
    Value::Null => Some("null".to_string()),
    _ => None,
  }
}
