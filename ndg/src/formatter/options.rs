use std::{
  collections::{HashMap, HashSet},
  fs,
  path::Path,
};

use color_eyre::eyre::{Context, Result};
use html_escape;
use log::debug;
use serde_json::{self, Value};

use crate::{
  config::Config,
  html::template,
  utils::{create_processor, json::extract_value},
};

/// Represents a `NixOS` configuration option.
///
/// This struct holds all relevant metadata for a single `NixOS` module option,
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

/// Process options from a JSON file and generate the options documentation
/// page.
///
/// Reads the options JSON file, parses all options, sorts and formats them,
/// and writes the rendered HTML to the output directory specified in the
/// config.
///
/// # Arguments
///
/// * `config` - The loaded configuration for documentation generation.
/// * `options_path` - Path to the options.json file.
///
/// # Errors
///
/// Returns an error if the file cannot be read, parsed, or written.
#[allow(
  clippy::cognitive_complexity,
  reason = "Main processing logic with multiple steps"
)]
pub fn process_options(config: &Config, options_path: &Path) -> Result<()> {
  // Read options JSON
  let json_content = fs::read_to_string(options_path).wrap_err_with(|| {
    format!("Failed to read options file: {}", options_path.display())
  })?;

  let options_data: Value = serde_json::from_str(&json_content)
    .wrap_err("Failed to parse options JSON")?;

  // First pass: collect all option names for validation
  let mut valid_options = HashSet::new();
  if let Value::Object(ref map) = options_data {
    for key in map.keys() {
      valid_options.insert(key.clone());
    }
  }

  // Create processor once with validation enabled
  let processor = create_processor(config, Some(valid_options));

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
          let result = processor.render(&processed_desc);
          option.description = result.html;
        }

        // Handle default values
        if let Some(default_val) = option_data.get("default") {
          if let Some(extracted_value) = extract_value(default_val, true) {
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
          if let Some(extracted_value) = extract_value(example_val, true) {
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
      opt.name = html_escape::encode_text(&opt.name).to_string();
      (key, opt)
    })
    .collect();

  // Render options page
  let html = template::render_options(config, &customized_options)?;
  let output_path = config.output_dir.join("options.html");
  fs::write(&output_path, html).wrap_err_with(|| {
    format!("Failed to write options file: {}", output_path.display())
  })?;

  Ok(())
}

/// Format location with URL handling based on nixos-render-docs.
///
/// Converts a location value from the options JSON into a display string and
/// an optional URL, supporting both local and GitHub-based references.
///
/// # Arguments
///
/// * `loc_value` - The JSON value representing the location.
/// * `revision` - The git revision for GitHub links.
///
/// # Returns
///
/// A tuple of (display string, URL).
fn format_location(
  loc_value: &Value,
  revision: &str,
) -> (Option<String>, Option<String>) {
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
        Some(html_escape::encode_text(name).to_string())
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
/// processor, but preserve code blocks.
///
/// This function ensures that angle brackets are escaped outside of code
/// blocks, preventing accidental HTML injection in documentation.
///
/// # Arguments
///
/// * `text` - The markdown text to escape.
///
/// # Returns
///
/// The escaped string, safe for markdown rendering.
fn escape_html_in_markdown(text: &str) -> String {
  // Split the text on backticks (`) to separate code blocks from regular text
  let mut result = String::with_capacity(text.len());

  // Track code block state
  let mut in_code_block = false;
  let mut in_inline_code = false;
  let mut backquote_counter = 0;

  for c in text.chars() {
    if c == '`' {
      // Manage backtick counting for code blocks
      backquote_counter += 1;

      if backquote_counter == 3 && !in_inline_code {
        // Toggle fenced code block state
        in_code_block = !in_code_block;
        backquote_counter = 0; // Reset counter after handling triple
      // backticks
      } else if backquote_counter == 1 && !in_code_block {
        // Toggle inline code state
        in_inline_code = !in_inline_code;
      }

      result.push(c);
    } else {
      // Reset backtick counter when not a backtick
      if backquote_counter > 0 && backquote_counter < 3 {
        // We've seen some backticks but not enough for a code block
        // These are either inline code delimiters or just literal backticks
        backquote_counter = 0;
      }

      // Handle character based on context
      if c == '<' && !in_code_block && !in_inline_code {
        result.push_str("&lt;");
      } else if c == '>' && !in_code_block && !in_inline_code {
        result.push_str("&gt;");
      } else {
        result.push(c);
      }
    }
  }

  result
}
