use anyhow::{Context, Result};
use log::debug;
use serde_json::{self, Value};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::config::Config;
use crate::markdown;
use crate::template;

/// Represents a NixOS configuration option
#[derive(Debug, Clone)]
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

    /// Whether this option is internal
    pub internal: bool,

    /// Whether this option is read-only
    pub read_only: bool,
}

/// Process options from a JSON file
pub fn process_options(config: &Config, options_path: &Path) -> Result<()> {
    // Read options JSON
    let json_content = fs::read_to_string(options_path).context(format!(
        "Failed to read options file: {}",
        options_path.display()
    ))?;

    let options_data: Value =
        serde_json::from_str(&json_content).context("Failed to parse options JSON")?;

    // Extract options
    let mut options = HashMap::new();

    if let Value::Object(map) = options_data {
        for (key, value) in map {
            if let Value::Object(option_data) = &value {
                let mut option = NixOption {
                    name: key.clone(),
                    type_name: "".to_string(),
                    description: "".to_string(),
                    default: None,
                    default_text: None,
                    example: None,
                    example_text: None,
                    declared_in: None,
                    internal: false,
                    read_only: false,
                };

                // Process fields from the option data
                if let Some(Value::String(type_name)) = option_data.get("type") {
                    option.type_name = type_name.clone();
                }

                if let Some(Value::String(desc)) = option_data.get("description") {
                    option.description = markdown::process_markdown_string(desc);
                }

                // Handle default values
                if let Some(default_val) = option_data.get("default") {
                    if let Some(extracted_value) = extract_value_from_json(default_val) {
                        option.default_text = Some(extracted_value);
                    } else {
                        option.default = Some(default_val.clone());
                    }
                }

                if let Some(Value::String(text)) = option_data.get("defaultText") {
                    option.default_text = Some(text.clone());
                }

                // Handle example values
                if let Some(example_val) = option_data.get("example") {
                    if let Some(extracted_value) = extract_value_from_json(example_val) {
                        option.example_text = Some(extracted_value);
                    } else {
                        option.example = Some(example_val.clone());
                    }
                }

                if let Some(Value::String(text)) = option_data.get("exampleText") {
                    option.example_text = Some(text.clone());
                }

                // Process declarations
                if let Some(Value::Array(decls)) = option_data.get("declarations") {
                    if !decls.is_empty() {
                        if let Value::Object(decl_obj) = &decls[0] {
                            if let Some(Value::String(name)) = decl_obj.get("name") {
                                debug!("Found declaration name: {}", name);
                                // Convert angle brackets to HTML entities explicitly
                                // This is the only way to retain `Declared by: < ... >`
                                let path = name.replace("<", "&lt;").replace(">", "&gt;");
                                option.declared_in = Some(path);
                            } else if let Some(Value::String(url)) = decl_obj.get("url") {
                                debug!("Found declaration url: {}", url);
                                option.declared_in = Some(url.clone());
                            }
                        } else if let Value::String(path) = &decls[0] {
                            debug!("Found declaration string: {}", path);
                            // Convert angle brackets to HTML entities explicitly
                            let path = path.replace("<", "&lt;").replace(">", "&gt;");
                            option.declared_in = Some(path);
                        }
                    }
                }

                // Read-only status
                if let Some(Value::Bool(read_only)) = option_data.get("readOnly") {
                    option.read_only = *read_only;
                }

                // Internal status
                if let Some(Value::Bool(internal)) = option_data.get("internal") {
                    option.internal = *internal;
                }

                if let Some(Value::Bool(visible)) = option_data.get("visible") {
                    if !visible {
                        option.internal = true;
                    }
                }

                // Use loc as fallback if no declaration
                if option.declared_in.is_none() {
                    if let Some(Value::Array(loc_data)) = option_data.get("loc") {
                        if loc_data.len() >= 2 {
                            // For loc data, join the parts with dots to form a path
                            let parts: Vec<String> = loc_data
                                .iter()
                                .filter_map(|v| {
                                    if let Value::String(s) = v {
                                        Some(s.clone())
                                    } else {
                                        None
                                    }
                                })
                                .collect();

                            if !parts.is_empty() {
                                option.declared_in = Some(parts.join("."));
                                debug!("Set declared_in from loc: {}", parts.join("."));
                            }
                        }
                    }
                }

                // Always ensure a fallback for declared_in
                if option.declared_in.is_none() {
                    option.declared_in = Some("configuration.nix".to_string());
                    debug!("Using fallback declared_in for {}", key);
                }

                options.insert(key, option);
            }
        }
    }

    let customized_options = options
        .iter()
        .map(|(key, opt)| {
            let option_clone = opt.clone();
            (key.clone(), option_clone)
        })
        .collect();

    // Render options page
    let html = template::render_options(config, &customized_options)?;
    let output_path = config.output_dir.join("options.html");
    fs::write(&output_path, html).context(format!(
        "Failed to write options file: {}",
        output_path.display()
    ))?;

    Ok(())
}

/// Extract the value from special JSON structures like literalExpression
fn extract_value_from_json(value: &Value) -> Option<String> {
    if let Value::Object(obj) = value {
        // literalExpression and similar structured values
        if let Some(Value::String(type_name)) = obj.get("_type") {
            match type_name.as_str() {
                "literalExpression" => {
                    if let Some(Value::String(text)) = obj.get("text") {
                        return Some(text.clone());
                    }
                }
                "literalDocBook" => {
                    if let Some(Value::String(text)) = obj.get("text") {
                        return Some(text.clone());
                    }
                }
                "literalMD" => {
                    if let Some(Value::String(text)) = obj.get("text") {
                        return Some(text.clone());
                    }
                }
                _ => {}
            }
        }
    }

    // For simple scalar values, just convert them to string
    match value {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        Value::Null => Some("null".to_string()),
        _ => None,
    }
}
