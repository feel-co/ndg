use anyhow::{Context, Result};
use log::debug;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::config::Config;
use crate::template;

/// Represents a NixOS option
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NixOption {
    /// Option name
    pub name: String,

    /// Option type
    #[serde(rename = "type")]
    pub type_name: String,

    /// Default value (if any)
    pub default: Option<Value>,

    /// Default value as text (if any)
    pub default_text: Option<String>,

    /// Option description
    pub description: String,

    /// Example value (if any)
    pub example: Option<Value>,

    /// Example text (if any)
    pub example_text: Option<String>,

    /// Declared in (file/module)
    pub declared_in: Option<String>,

    /// Whether the option is required
    pub required: bool,
}

/// Process options.json file
pub fn process_options(config: &Config, options_path: &Path) -> Result<()> {
    // Load and parse options.json
    let options_content = fs::read_to_string(options_path).context(format!(
        "Failed to read options file: {}",
        options_path.display()
    ))?;

    let options_raw: HashMap<String, Value> =
        serde_json::from_str(&options_content).context("Failed to parse options.json")?;

    // Convert to structured format
    let mut options = HashMap::new();

    for (name, option_value) in options_raw {
        if let Some(obj) = option_value.as_object() {
            let option = NixOption {
                name: name.clone(),
                type_name: obj
                    .get("type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string(),
                default: obj.get("default").cloned(),
                default_text: obj
                    .get("defaultText")
                    .and_then(|v| v.as_str())
                    .map(String::from),
                description: obj
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                example: obj.get("example").cloned(),
                example_text: obj
                    .get("exampleText")
                    .and_then(|v| v.as_str())
                    .map(String::from),
                declared_in: obj
                    .get("declaredIn")
                    .and_then(|v| v.as_str())
                    .map(String::from),
                required: obj
                    .get("required")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
            };

            options.insert(name, option);
        }
    }

    debug!("Processed {} options", options.len());

    // Render options page
    let html = template::render_options(config, &options)?;

    // Write options page
    let output_path = config.output_dir.join("options.html");
    fs::write(&output_path, html).context(format!(
        "Failed to write options HTML file: {}",
        output_path.display()
    ))?;

    // Save processed JSON for search
    let json_path = config.output_dir.join("assets/options.json");
    if let Some(parent) = json_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }

    fs::write(json_path, serde_json::to_string_pretty(&options)?)
        .context("Failed to write processed options.json")?;

    Ok(())
}
