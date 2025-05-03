use std::{
   collections::HashMap,
   fs,
   path::Path,
};

use anyhow::{
   Context,
   Result,
};
use log::debug;
use serde_json::{
   self,
   Value,
};

use crate::{
   config::Config,
   formatter::markdown,
   html::template,
};

/// Represents a `NixOS` configuration option
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

   /// Option declaration URL for hyperlink
   pub declared_in_url: Option<String>,

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
               name:            key.clone(),
               type_name:       String::new(),
               description:     String::new(),
               default:         None,
               default_text:    None,
               example:         None,
               example_text:    None,
               declared_in:     None,
               declared_in_url: None,
               internal:        false,
               read_only:       false,
            };

            // Process fields from the option data
            if let Some(Value::String(type_name)) = option_data.get("type") {
               option.type_name.clone_from(type_name);
            }

            if let Some(Value::String(desc)) = option_data.get("description") {
               let processed_desc = escape_html_in_markdown(desc);
               option.description = markdown::process_markdown_string(&processed_desc, config);
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

            // Process declarations with location handling
            if let Some(Value::Array(decls)) = option_data.get("declarations") {
               if !decls.is_empty() {
                  let (display, url) = format_location(&decls[0], &config.revision);
                  option.declared_in = display;
                  option.declared_in_url = url;
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
                  if !loc_data.is_empty() {
                     // For loc data, join the parts to form a path
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
               debug!("Using fallback declared_in for {key}");
            }

            options.insert(key, option);
         }
      }
   }

   // Sort options by priority (enable > package > other)
   let mut sorted_options: Vec<_> = options.into_iter().collect();
   sorted_options.sort_by(|(name_a, _), (name_b, _)| {
      // Calculate priority for name_a
      let priority_a = if name_a.starts_with("enable") {
         0
      } else if name_a.starts_with("package") {
         1
      } else {
         2
      };

      // Calculate priority for name_b
      let priority_b = if name_b.starts_with("enable") {
         0
      } else if name_b.starts_with("package") {
         1
      } else {
         2
      };

      // Compare by priority first, then by name
      priority_a.cmp(&priority_b).then_with(|| name_a.cmp(name_b))
   });

   // Convert back to HashMap preserving the new order
   let customized_options = sorted_options
      .iter()
      .map(|(key, opt)| {
         let option_clone = opt.clone();
         (key.clone(), option_clone)
      })
      .map(|(key, mut opt)| {
         opt.name = opt.name.replace('<', "&lt;").replace('>', "&gt;");
         (key, opt)
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

/// Format location with URL handling based on nixos-render-docs
fn format_location(loc_value: &Value, revision: &str) -> (Option<String>, Option<String>) {
   match loc_value {
      // Handle string path
      Value::String(path) => {
         let path_str = path.as_str();

         if path_str.starts_with('/') {
            // Local filesystem path handling
            let url = format!("file://{path_str}");

            if path_str.contains("nixops") && path_str.contains("/nix/") {
               let suffix_index = path_str.find("/nix/").map_or(0, |i| i + 5);
               let suffix = &path_str[suffix_index..];
               (Some(format!("<nixops/{suffix}>")), Some(url))
            } else {
               (Some(path_str.to_string()), Some(url))
            }
         } else {
            // Path is relative to nixpkgs repo
            let url = if revision == "local" {
               format!("https://github.com/NixOS/nixpkgs/blob/master/{path_str}")
            } else {
               format!("https://github.com/NixOS/nixpkgs/blob/{revision}/{path_str}")
            };

            // Format display name
            let display = format!("<nixpkgs/{path_str}>");
            (Some(display), Some(url))
         }
      },

      // Handle object with name and url
      Value::Object(obj) => {
         let display = if let Some(Value::String(name)) = obj.get("name") {
            Some(name.replace('<', "&lt;").replace('>', "&gt;"))
         } else {
            None
         };

         let url = obj.get("url").and_then(|u| {
            if let Value::String(url_str) = u {
               Some(url_str.clone())
            } else {
               None
            }
         });

         (display, url)
      },

      // For other values, return None
      _ => (None, None),
   }
}

/// Escape HTML tags in markdown text before it's processed by the markdown
/// processor
fn escape_html_in_markdown(text: &str) -> String {
   text.replace('<', "&lt;").replace('>', "&gt;")
}

/// Extract the value from special JSON structures like literalExpression
fn extract_value_from_json(value: &Value) -> Option<String> {
   if let Value::Object(obj) = value {
      // literalExpression and similar structured values
      if let Some(Value::String(type_name)) = obj.get("_type") {
         match type_name.as_str() {
            "literalExpression" | "literalDocBook" | "literalMD" => {
               if let Some(Value::String(text)) = obj.get("text") {
                  return Some(text.clone());
               }
            },
            _ => {},
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
