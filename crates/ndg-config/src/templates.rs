//! Struct-derived starter configuration and annotated TOML rendering.

use std::{borrow::Cow, path::PathBuf};

use serde::Serialize;
use thiserror::Error;

use crate::Config;

/// Errors encountered while rendering a starter configuration.
#[derive(Debug, Error)]
pub enum TemplateError {
  /// The requested file format is unsupported.
  #[error("Unsupported config format: {0}")]
  UnsupportedFormat(String),
  /// A configuration value cannot be represented as TOML.
  #[error("Failed to serialize TOML configuration: {0}")]
  Toml(#[from] toml::ser::Error),
  /// A configuration value cannot be represented as JSON.
  #[error("Failed to serialize JSON configuration: {0}")]
  Json(#[from] serde_json::Error),
}

pub(crate) trait ConfigTemplate {
  fn write_template(
    &self,
    writer: &mut TemplateWriter,
  ) -> Result<(), TemplateError>;
}

#[derive(Default)]
pub(crate) struct TemplateWriter {
  output:    String,
  path:      String,
  value:     String,
  commented: bool,
}

impl TemplateWriter {
  fn comments(&mut self, docs: &str) {
    for line in docs.lines() {
      self.output.push_str("# ");
      self.output.push_str(line);
      self.output.push('\n');
    }
  }

  pub(crate) fn field<T: Serialize>(
    &mut self,
    key: &str,
    docs: &str,
    value: &T,
    commented: bool,
  ) -> Result<(), TemplateError> {
    self.value.clear();
    value.serialize(toml::ser::ValueSerializer::new(&mut self.value))?;
    self.comments(docs);
    let prefix = if self.commented || commented {
      "# "
    } else {
      ""
    };
    self.output.push_str(prefix);
    self.output.push_str(&toml_key(key));
    self.output.push_str(" = ");
    // TOML strings can contain literal newlines; comment every line of
    // examples.
    for (index, line) in self.value.lines().enumerate() {
      if index > 0 {
        self.output.push('\n');
        self.output.push_str(prefix);
      }
      self.output.push_str(line);
    }
    self.output.push_str("\n\n");
    Ok(())
  }

  pub(crate) fn section<T: ConfigTemplate>(
    &mut self,
    key: &str,
    docs: &str,
    value: &T,
    commented: bool,
    array: bool,
  ) -> Result<(), TemplateError> {
    let previous_len = self.path.len();
    let previous_commented = self.commented;
    if !self.path.is_empty() {
      self.path.push('.');
    }
    self.path.push_str(&toml_key(key));
    self.commented |= commented;
    self.comments(docs);
    if self.commented {
      self.output.push_str("# ");
    }
    self.output.push_str(if array { "[[" } else { "[" });
    self.output.push_str(&self.path);
    self.output.push_str(if array { "]]\n\n" } else { "]\n\n" });
    let result = value.write_template(self);
    self.path.truncate(previous_len);
    self.commented = previous_commented;
    result
  }
}

fn toml_key(key: &str) -> Cow<'_, str> {
  if !key.is_empty()
    && key
      .bytes()
      .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
  {
    Cow::Borrowed(key)
  } else {
    Cow::Owned(toml::Value::String(key.to_string()).to_string())
  }
}

fn starter_config() -> Config {
  Config {
    input_dir: Some(PathBuf::from("docs")),
    ..Config::default()
  }
}

/// Generate a starter configuration using the configuration types and defaults.
///
/// TOML includes field documentation and commented examples for absent optional
/// settings. JSON contains only the starter values, never the TOML examples.
///
/// # Errors
///
/// Returns an error for unsupported formats or values that cannot be
/// serialized.
///
/// # Examples
///
/// ```
/// let config = ndg_config::templates::get_template("json")?;
/// let _: ndg_config::Config = serde_json::from_str(&config)?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn get_template(format: &str) -> Result<String, TemplateError> {
  let config = starter_config();
  match format.to_lowercase().as_str() {
    "toml" => {
      let mut writer = TemplateWriter {
        output: "# NDG Configuration File\n# Optional settings and sections \
                 are commented out.\n\n"
          .to_string(),
        ..TemplateWriter::default()
      };
      config.write_template(&mut writer)?;
      Ok(writer.output)
    },
    "json" => Ok(serde_json::to_string_pretty(&config)? + "\n"),
    _ => Err(TemplateError::UnsupportedFormat(format.to_string())),
  }
}

#[cfg(test)]
#[expect(
  clippy::unwrap_used,
  reason = "Tests can unwrap generated configurations"
)]
mod tests {
  use tempfile::tempdir;

  use super::*;
  use crate::{options, sidebar};

  #[test]
  fn test_init_formats_load_the_same_starter() {
    let temp = tempdir().unwrap();
    let expected = serde_json::to_value(starter_config()).unwrap();
    for format in ["toml", "json"] {
      let path = temp.path().join(format!("ndg.{format}"));
      Config::generate_default_config(format, &path).unwrap();
      let config = Config::from_file(path).unwrap();
      assert_eq!(serde_json::to_value(config).unwrap(), expected);
    }
  }

  #[test]
  fn test_template_nested_tables_preserve_root_values_and_rule_names() {
    let config = Config {
      title: "Quotes \" and\na new line".to_string(),
      options: Some(options::OptionsConfig {
        filter: Some(options::FilterConfig {
          type_name: Some("boolean".to_string()),
          ..options::FilterConfig::default()
        }),
        ..options::OptionsConfig::default()
      }),
      sidebar: Some(sidebar::SidebarConfig {
        matches: vec![sidebar::SidebarMatch {
          path: Some(sidebar::PathMatch {
            exact: Some("guide.md".to_string()),
            ..sidebar::PathMatch::default()
          }),
          position: Some(2),
          ..sidebar::SidebarMatch::default()
        }],
        ..sidebar::SidebarConfig::default()
      }),
      ..starter_config()
    };
    let mut writer = TemplateWriter::default();
    config.write_template(&mut writer).unwrap();
    let mut parsed: Config = toml::from_str(&writer.output).unwrap();
    parsed.validate().unwrap();
    assert_eq!(
      serde_json::to_value(parsed).unwrap(),
      serde_json::to_value(config).unwrap()
    );
  }
}
