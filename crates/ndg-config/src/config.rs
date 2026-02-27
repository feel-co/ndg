use std::{
  collections::HashMap,
  fs,
  path::{Path, PathBuf},
  sync::OnceLock,
};

use ndg_macros::Configurable;
use serde::{Deserialize, Serialize};

use crate::{assets, error::ConfigError, meta, postprocess, search, sidebar};

/// Configuration for the NDG documentation generator.
///
/// [`Config`] holds all configuration options for controlling documentation
/// generation, including input/output directories, template customization,
/// search, syntax highlighting, and more. Fields are typically loaded from a
/// TOML or JSON config file, but can also be set via CLI arguments.
#[derive(Debug, Clone, Serialize, Deserialize, Configurable)]
#[serde(default)]
pub struct Config {
  /// Input directory containing markdown files.
  #[config(key = "input_dir", allow_empty)]
  pub input_dir: Option<PathBuf>,

  /// Output directory for generated documentation.
  #[config(key = "output_dir")]
  pub output_dir: PathBuf,

  /// Path to options.json file (optional).
  #[config(key = "module_options", allow_empty)]
  pub module_options: Option<PathBuf>,

  /// Path to custom template file.
  #[config(key = "template_path", allow_empty)]
  pub template_path: Option<PathBuf>,

  /// Path to template directory containing all template files.
  #[config(key = "template_dir", allow_empty)]
  pub template_dir: Option<PathBuf>,

  /// Paths to custom stylesheets.
  pub stylesheet_paths: Vec<PathBuf>,

  /// Paths to custom JavaScript files.
  pub script_paths: Vec<PathBuf>,

  /// Directory containing additional assets.
  #[config(key = "assets_dir", allow_empty)]
  pub assets_dir: Option<PathBuf>,

  /// Options for copying custom assets.
  #[config(nested)]
  pub assets: Option<assets::AssetsConfig>,

  /// Path to manpage URL mappings JSON file.
  #[config(key = "manpage_urls_path", allow_empty)]
  pub manpage_urls_path: Option<PathBuf>,

  /// Title for the documentation.
  #[config(key = "title")]
  pub title: String,

  /// Number of threads to use for parallel processing.
  #[config(key = "jobs", allow_empty)]
  pub jobs: Option<usize>,

  /// Whether to generate anchors for headings.
  #[config(key = "generate_anchors")]
  pub generate_anchors: bool,

  /// Whether to generate a search index.
  #[deprecated(since = "2.5.0", note = "Use `search.enable` instead")]
  #[config(
    key = "generate_search",
    deprecated = "2.5.0",
    replacement = "search.enable"
  )]
  pub generate_search: bool,

  /// Search configuration
  #[config(nested)]
  pub search: Option<search::SearchConfig>,

  /// Text to be inserted in the footer.
  #[config(key = "footer_text")]
  pub footer_text: String,

  /// Depth of parent categories in options TOC.
  #[config(
    key = "options_toc_depth",
    deprecated = "2.5.0",
    replacement = "sidebar.options.depth"
  )]
  pub options_toc_depth: usize,

  /// Whether to enable syntax highlighting for code blocks.
  #[config(key = "highlight_code")]
  pub highlight_code: bool,

  /// GitHub revision for linking to source files.
  #[config(key = "revision")]
  pub revision: String,

  /// Files that are included via {=include=}, mapped to their parent input
  /// files
  #[serde(skip)]
  pub included_files:
    std::collections::HashMap<std::path::PathBuf, std::path::PathBuf>,

  /// How to handle hard tabs in code blocks.
  #[config(key = "tab_style")]
  pub tab_style: String,

  /// Meta tag configuration (`OpenGraph` and additional tags).
  #[config(nested)]
  pub meta: Option<meta::MetaConfig>,

  /// Sidebar configuration.
  #[config(nested)]
  pub sidebar: Option<sidebar::SidebarConfig>,

  /// Postprocessing configuration for HTML/CSS/JS minification
  #[config(nested)]
  pub postprocess: Option<postprocess::PostprocessConfig>,

  /// Nix files or directories to extract nixdoc comments from.
  ///
  /// Each entry may be a `.nix` file or a directory. Directories are scanned
  /// recursively for `.nix` files. Entries with nixdoc comments (`/** ... */`)
  /// are extracted and rendered as a library reference page (`lib.html`).
  #[serde(default)]
  pub nixdoc_inputs: Vec<PathBuf>,

  #[deprecated(since = "2.5.0", note = "Use `meta.opengraph` instead")]
  #[serde(skip_serializing_if = "Option::is_none")]
  pub opengraph: Option<HashMap<String, String>>,

  #[deprecated(since = "2.5.0", note = "Use `meta.tags` instead")]
  #[serde(skip_serializing_if = "Option::is_none")]
  pub meta_tags: Option<HashMap<String, String>>,
}

impl Default for Config {
  #[allow(deprecated)]
  fn default() -> Self {
    Self {
      input_dir:         None,
      output_dir:        PathBuf::from("build"),
      module_options:    None,
      template_path:     None,
      template_dir:      None,
      stylesheet_paths:  Vec::new(),
      script_paths:      Vec::new(),
      assets_dir:        None,
      assets:            None,
      manpage_urls_path: None,
      title:             "ndg documentation".to_string(),
      jobs:              None,
      generate_anchors:  true,
      generate_search:   true,
      search:            None,
      footer_text:       "Generated with ndg".to_string(),
      options_toc_depth: 2,
      highlight_code:    true,
      revision:          "local".to_string(),
      included_files:    std::collections::HashMap::new(),
      tab_style:         "none".to_string(),
      meta:              None,
      sidebar:           None,
      postprocess:       None,
      nixdoc_inputs:     Vec::new(),
      opengraph:         None,
      meta_tags:         None,
    }
  }
}

impl Config {
  /// Returns whether search is enabled, checking both new and deprecated
  /// configs.
  #[must_use]
  pub const fn is_search_enabled(&self) -> bool {
    // Priority: search.enable > deprecated generate_search > default true
    if let Some(ref search) = self.search {
      search.enable
    } else {
      #[allow(deprecated)]
      {
        self.generate_search
      }
    }
  }

  /// Returns the maximum heading level to index for search.
  #[must_use]
  pub fn search_max_heading_level(&self) -> u8 {
    self.search.as_ref().map_or(3, |s| s.max_heading_level)
  }

  /// Load configuration from a file (TOML or JSON).
  ///
  /// # Arguments
  ///
  /// * `path` - Path to the configuration file.
  ///
  /// # Errors
  ///
  /// Returns an error if the file cannot be read or parsed, or if the format is
  /// unsupported.
  #[allow(
    clippy::option_if_let_else,
    reason = "Clearer with explicit match on extension"
  )]
  pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
    let path = path.as_ref();
    let content = fs::read_to_string(path).map_err(|e| {
      ConfigError::Config(format!(
        "Failed to read config file: {}: {}",
        path.display(),
        e
      ))
    })?;

    match path.extension().and_then(|ext| ext.to_str()) {
      Some(ext) => {
        match ext.to_lowercase().as_str() {
          "json" => {
            serde_json::from_str(&content)
              .map_err(ConfigError::from)
              .map_err(|e| {
                ConfigError::Config(format!(
                  "Failed to parse JSON config from {}: {}",
                  path.display(),
                  e
                ))
              })
          },
          "toml" => {
            toml::from_str(&content)
              .map_err(ConfigError::from)
              .map_err(|e| {
                ConfigError::Config(format!(
                  "Failed to parse TOML config from {}: {}",
                  path.display(),
                  e
                ))
              })
          },
          _ => {
            Err(ConfigError::Config(format!(
              "Unsupported config file format: {}",
              path.display()
            )))
          },
        }
      },
      None => {
        Err(ConfigError::Config(format!(
          "Config file has no extension: {}",
          path.display()
        )))
      },
    }
  }

  /// Load configuration from file and CLI arguments, merging them.
  ///
  /// This function loads configuration from a file (if provided or discovered),
  /// then merges in CLI arguments, and validates required fields.
  ///
  /// # Arguments
  ///
  /// * `cli` - The parsed CLI arguments.
  ///
  /// # Errors
  ///
  /// Returns an error if required fields are missing or paths are invalid.
  pub fn load(
    config_files: &[PathBuf],
    config_overrides: &[String],
  ) -> Result<Self, ConfigError> {
    let mut config = if !config_files.is_empty() {
      // Config file(s) explicitly specified via CLI
      // Load and merge them in order
      let mut merged_config =
        Self::from_file(&config_files[0]).map_err(|e| {
          ConfigError::Config(format!(
            "Failed to load config from {}: {}",
            config_files[0].display(),
            e
          ))
        })?;

      // Merge additional config files if provided
      for config_path in &config_files[1..] {
        let additional_config = Self::from_file(config_path).map_err(|e| {
          ConfigError::Config(format!(
            "Failed to load config from {}: {}",
            config_path.display(),
            e
          ))
        })?;
        merged_config.merge(additional_config);
      }

      if config_files.len() > 1 {
        log::info!("Loaded and merged {} config files", config_files.len());
      }

      merged_config
    } else if let Some(discovered_config) = Self::find_config_file() {
      // Found a config file in a standard location
      log::info!(
        "Using discovered config file: {}",
        discovered_config.display()
      );
      Self::from_file(&discovered_config).map_err(|e| {
        ConfigError::Config(format!(
          "Failed to load discovered config from {}: {}",
          discovered_config.display(),
          e
        ))
      })?
    } else {
      Self::default()
    };

    // Apply config overrides from --config KEY=VALUE flags
    if !config_overrides.is_empty() {
      config.apply_overrides(config_overrides)?;
    }

    // We need *at least one* source of content
    if config.input_dir.is_none()
      && config.module_options.is_none()
      && config.nixdoc_inputs.is_empty()
    {
      return Err(ConfigError::Config(
        "At least one of input directory, module options, or nixdoc inputs \
         must be provided."
          .to_string(),
      ));
    }

    // Note: Path validation is deferred to after CLI merge to allow CLI args
    // to override config file values

    // Validate and compile sidebar configuration if present
    if let Some(ref mut sidebar) = config.sidebar {
      sidebar.validate().map_err(|e| {
        ConfigError::Config(format!(
          "Sidebar configuration validation failed: {e}"
        ))
      })?;
    }

    Ok(config)
  }

  /// Apply configuration overrides from KEY=VALUE strings.
  ///
  /// This method parses and applies configuration overrides specified via
  /// `--config KEY=VALUE` CLI flags. Each override must be in the format
  /// `KEY=VALUE` where KEY is a valid configuration field name.
  ///
  /// # Arguments
  ///
  /// * `overrides` - Vector of KEY=VALUE strings to apply
  ///
  /// # Errors
  ///
  /// Returns an error if:
  ///
  /// - An override string is not in KEY=VALUE format
  /// - A key is not recognized
  /// - A value cannot be parsed as the expected type
  ///
  /// # Example
  ///
  /// ```rust, ignore
  /// config.apply_overrides(&vec![
  ///     "search.enable=false".to_string(),
  ///     "title=My Documentation".to_string(),
  /// ])?;
  /// ```
  pub fn apply_overrides(
    &mut self,
    overrides: &[String],
  ) -> Result<(), ConfigError> {
    for override_str in overrides {
      let (key, value) = override_str.split_once('=').ok_or_else(|| {
        ConfigError::Config(format!(
          "Invalid config override format: '{override_str}'. Expected \
           KEY=VALUE"
        ))
      })?;

      self.apply_override(key.trim(), value.trim())?;
    }

    Ok(())
  }

  /// Merge another config into this one, with the other config's values taking
  /// precedence.
  ///
  /// This method is used when loading multiple config files - each successive
  /// config file is merged into the accumulated config, overriding values.
  ///
  /// # Merge Rules
  ///
  /// - [`Option<T>`] fields: Other's [`Some`] value replaces this config's
  ///   value
  /// - [`Vec<T>`] fields: Other's vec is appended to this config's vec
  /// - Plain fields (String, bool, etc.): Other's value always replaces
  /// - [`HashMap`] fields: Other's entries are merged in (can override
  ///   individual keys)
  ///
  /// # Arguments
  ///
  /// * `other` - The config to merge in (takes precedence)
  pub fn merge(&mut self, mut other: Self) {
    // Handle deprecated fields first before merge_fields consumes other
    #[allow(deprecated)]
    {
      if let Some(other_og) = other.opengraph.take() {
        log::warn!(
          "The 'opengraph' config field is deprecated. Use 'meta.opengraph' \
           instead."
        );
        if let Some(ref mut og) = self.opengraph {
          og.extend(other_og);
        } else {
          self.opengraph = Some(other_og);
        }
      }
      if let Some(other_meta) = other.meta_tags.take() {
        log::warn!(
          "The 'meta_tags' config field is deprecated. Use 'meta.tags' \
           instead."
        );
        if let Some(ref mut meta) = self.meta_tags {
          meta.extend(other_meta);
        } else {
          self.meta_tags = Some(other_meta);
        }
      }
    }

    // Use the generated merge method for most fields
    self.merge_fields(other);

    // XXX: excluded_files is runtime-only (#[serde(skip)]), so we don't merge
    // it
  }

  /// Get the template directory path, if available.
  ///
  /// # Returns
  ///
  /// Directory path that contains template files. Priority order:
  ///
  /// 1. Explicit `template_dir` configuration
  /// 2. If `template_path` is a directory, use that
  /// 3. [`None`] otherwise (single file templates don't provide a directory)
  #[must_use]
  pub fn get_template_path(&self) -> Option<PathBuf> {
    // Explicit template directory has highest priority
    if let Some(ref dir) = self.template_dir {
      return Some(dir.clone());
    }

    // Check if template_path is a directory
    if let Some(ref path) = self.template_path
      && path.is_dir()
    {
      return Some(path.clone());
    }

    None
  }

  /// Get the path to a specific template file by name.
  ///
  /// # Returns
  ///
  /// Path to the requested template file if it can be found in:
  ///
  /// 1. The template directory (if configured)
  /// 2. As a single file via `template_path` (if the filename matches)
  ///
  /// This method does not check if the returned path exists.
  #[must_use]
  pub fn get_template_file(&self, name: &str) -> Option<PathBuf> {
    // If we have a template directory, check for the file there
    if let Some(dir) = self.get_template_path() {
      return Some(dir.join(name));
    }

    // Check if template_path is a single file matching the requested name
    if let Some(ref path) = self.template_path
      && path.is_file()
      && let Some(filename) = path.file_name()
      && filename == name
    {
      return Some(path.clone());
    }

    None
  }

  /// Search for config files in common locations
  #[must_use]
  pub fn find_config_file() -> Option<PathBuf> {
    static RESULT: OnceLock<Option<PathBuf>> = OnceLock::new();
    RESULT
      .get_or_init(|| {
        let config_filenames = [
          "ndg.toml",
          "ndg.json",
          ".ndg.toml",
          ".ndg.json",
          ".config/ndg.toml",
          ".config/ndg.json",
        ];

        let current_dir = std::env::current_dir().ok()?;
        for filename in &config_filenames {
          let config_path = current_dir.join(filename);
          if config_path.exists() {
            return Some(config_path);
          }
        }

        if let Ok(xdg_config_home) = std::env::var("XDG_CONFIG_HOME") {
          let xdg_config_dir = PathBuf::from(xdg_config_home);
          for filename in &["ndg.toml", "ndg.json"] {
            let config_path = xdg_config_dir.join(filename);
            if config_path.exists() {
              return Some(config_path);
            }
          }
        }

        if let Ok(home) = std::env::var("HOME") {
          let home_config_dir = PathBuf::from(home).join(".config").join("ndg");
          for filename in &["config.toml", "config.json"] {
            let config_path = home_config_dir.join(filename);
            if config_path.exists() {
              return Some(config_path);
            }
          }
        }

        None
      })
      .clone()
  }

  /// Validate all paths specified in the configuration
  ///
  /// # Errors
  ///
  /// Returns an error if any configured path does not exist or is invalid.
  pub fn validate_paths(&self) -> Result<(), ConfigError> {
    let mut errors = Vec::new();

    // Module options file should exist if specified
    if let Some(ref module_options) = self.module_options {
      if !module_options.exists() {
        errors.push(format!(
          "Module options file does not exist: {}",
          module_options.display()
        ));
      } else if !module_options.is_file() {
        errors.push(format!(
          "Module options path is not a file: {}",
          module_options.display()
        ));
      }
    }

    // Template file should exist if specified
    if let Some(ref template_path) = self.template_path
      && !template_path.exists()
    {
      errors.push(format!(
        "Template file does not exist: {}",
        template_path.display()
      ));
    }

    // Template directory should exist if specified
    if let Some(ref template_dir) = self.template_dir {
      if !template_dir.exists() {
        errors.push(format!(
          "Template directory does not exist: {}",
          template_dir.display()
        ));
      } else if !template_dir.is_dir() {
        errors.push(format!(
          "Template directory path is not a directory: {}",
          template_dir.display()
        ));
      }
    }

    // Stylesheet files should exist if specified
    for (index, stylesheet_path) in self.stylesheet_paths.iter().enumerate() {
      if !stylesheet_path.exists() {
        errors.push(format!(
          "Stylesheet file {} does not exist: {}",
          index + 1,
          stylesheet_path.display()
        ));
      } else if !stylesheet_path.is_file() {
        errors.push(format!(
          "Stylesheet path {} is not a file: {}",
          index + 1,
          stylesheet_path.display()
        ));
      }
    }

    // Assets directory should exist if specified
    if let Some(ref assets_dir) = self.assets_dir {
      if !assets_dir.exists() {
        errors.push(format!(
          "Assets directory does not exist: {}",
          assets_dir.display()
        ));
      } else if !assets_dir.is_dir() {
        errors.push(format!(
          "Assets directory path is not a directory: {}",
          assets_dir.display()
        ));
      }
    }

    // Manpage URLs file should exist if specified
    if let Some(ref manpage_urls_path) = self.manpage_urls_path {
      if !manpage_urls_path.exists() {
        errors.push(format!(
          "Manpage URLs file does not exist: {}",
          manpage_urls_path.display()
        ));
      } else if !manpage_urls_path.is_file() {
        errors.push(format!(
          "Manpage URLs path is not a file: {}",
          manpage_urls_path.display()
        ));
      }
    }

    // Script files should exist if specified
    for (index, script_path) in self.script_paths.iter().enumerate() {
      if !script_path.exists() {
        errors.push(format!(
          "Script file {} does not exist: {}",
          index + 1,
          script_path.display()
        ));
      } else if !script_path.is_file() {
        errors.push(format!(
          "Script path {} is not a file: {}",
          index + 1,
          script_path.display()
        ));
      }
    }

    // Nixdoc input paths should exist if specified
    for (index, nixdoc_input) in self.nixdoc_inputs.iter().enumerate() {
      if !nixdoc_input.exists() {
        errors.push(format!(
          "Nixdoc input {} does not exist: {}",
          index + 1,
          nixdoc_input.display()
        ));
      }
    }

    // Return error if we found any issues
    if !errors.is_empty() {
      let error_message = errors.join("\n");
      return Err(ConfigError::Config(format!(
        "Configuration path validation errors:\n{error_message}"
      )));
    }

    Ok(())
  }

  /// Generate a default configuration file with commented explanations
  ///
  /// # Errors
  ///
  /// Returns an error if the template cannot be retrieved or the file cannot be
  /// written.
  pub fn generate_default_config(
    format: &str,
    path: &Path,
  ) -> Result<(), ConfigError> {
    // Get template from the templates module
    let config_content = crate::templates::get_template(format)
      .map_err(|e| ConfigError::Template(e.to_string()))?;

    fs::write(path, config_content).map_err(|e| {
      ConfigError::Config(format!(
        "Failed to write default config to {}: {}",
        path.display(),
        e
      ))
    })?;

    log::info!("Created default configuration file: {}", path.display());
    Ok(())
  }

  /// Export embedded templates to a directory for customization
  ///
  /// # Errors
  ///
  /// Returns an error if the output directory cannot be created or a template
  /// cannot be written.
  pub fn export_templates(
    output_dir: &Path,
    force: bool,
    templates: Option<Vec<String>>,
  ) -> Result<(), ConfigError> {
    // Create output directory if it doesn't exist
    fs::create_dir_all(output_dir).map_err(|e| {
      ConfigError::Config(format!(
        "Failed to create template directory: {}: {}",
        output_dir.display(),
        e
      ))
    })?;

    // Get all embedded template sources
    let all_templates = Self::get_template_sources();
    let templates_to_export = if let Some(specified) = templates {
      if specified.is_empty() {
        all_templates
      } else {
        all_templates
          .into_iter()
          .filter(|(name, _)| {
            specified
              .iter()
              .any(|t| name.ends_with(&format!(".{t}")) || *t == "all")
          })
          .collect()
      }
    } else {
      all_templates
    };

    for (filename, content) in templates_to_export {
      let file_path = output_dir.join(filename);

      // Check if file exists and force flag
      if file_path.exists() && !force {
        log::warn!(
          "File {} already exists. Use --force to overwrite.",
          file_path.display()
        );
        continue;
      }

      fs::write(&file_path, content).map_err(|e| {
        ConfigError::Config(format!(
          "Failed to write template file: {}: {}",
          file_path.display(),
          e
        ))
      })?;
      log::info!("Exported template: {}", file_path.display());
    }

    Ok(())
  }

  /// Get mapping of template filenames to their embedded content
  fn get_template_sources()
  -> std::collections::HashMap<&'static str, &'static str> {
    ndg_templates::all_templates()
  }
}

#[cfg(test)]
mod tests {
  #![allow(
    clippy::useless_vec,
    clippy::unwrap_used,
    clippy::field_reassign_with_default,
    reason = "Fine in tests"
  )]

  use super::*;

  #[test]
  fn test_config_merge_option_fields() {
    let mut base = Config::default();
    base.input_dir = Some(PathBuf::from("base-input"));
    base.module_options = None;

    let mut override_config = Config::default();
    override_config.input_dir = None; // should not replace
    override_config.module_options = Some(PathBuf::from("override-options"));

    base.merge(override_config);

    // input_dir should remain from base (override had None)
    assert_eq!(base.input_dir, Some(PathBuf::from("base-input")));
    // module_options should come from override
    assert_eq!(base.module_options, Some(PathBuf::from("override-options")));
  }

  #[test]
  fn test_config_merge_vec_fields_append() {
    let mut base = Config::default();
    base.stylesheet_paths = vec![PathBuf::from("base.css")];
    base.script_paths = vec![PathBuf::from("base.js")];

    let mut override_config = Config::default();
    override_config.stylesheet_paths = vec![PathBuf::from("override.css")];
    override_config.script_paths = vec![PathBuf::from("override.js")];

    base.merge(override_config);

    // Vecs should be appended, not replaced
    assert_eq!(base.stylesheet_paths.len(), 2);
    assert_eq!(base.stylesheet_paths[0], PathBuf::from("base.css"));
    assert_eq!(base.stylesheet_paths[1], PathBuf::from("override.css"));

    assert_eq!(base.script_paths.len(), 2);
    assert_eq!(base.script_paths[0], PathBuf::from("base.js"));
    assert_eq!(base.script_paths[1], PathBuf::from("override.js"));
  }

  #[test]
  fn test_config_merge_boolean_fields() {
    let mut base = Config::default();
    base.search = Some(search::SearchConfig {
      enable: true,
      ..Default::default()
    });
    base.highlight_code = false;

    let mut override_config = Config::default();
    override_config.search = Some(search::SearchConfig {
      enable: false,
      ..Default::default()
    });
    override_config.highlight_code = true;

    base.merge(override_config);

    // Boolean fields should take override's value
    assert!(!base.is_search_enabled());
    assert!(base.highlight_code);
  }

  #[test]
  fn test_apply_overrides_boolean() {
    let mut config = Config::default();

    config
      .apply_overrides(&vec![
        "search.enable=false".to_string(),
        "highlight_code=yes".to_string(),
      ])
      .unwrap();

    assert!(!config.is_search_enabled());
    assert!(config.highlight_code);
  }

  #[test]
  fn test_apply_overrides_string() {
    let mut config = Config::default();

    config
      .apply_overrides(&vec![
        "title=Test Documentation".to_string(),
        "footer_text=Custom Footer".to_string(),
      ])
      .unwrap();

    assert_eq!(config.title, "Test Documentation");
    assert_eq!(config.footer_text, "Custom Footer");
  }

  #[test]
  fn test_apply_overrides_path() {
    let mut config = Config::default();

    config
      .apply_overrides(&vec![
        "output_dir=/tmp/output".to_string(),
        "input_dir=/tmp/input".to_string(),
      ])
      .unwrap();

    assert_eq!(config.output_dir, PathBuf::from("/tmp/output"));
    assert_eq!(config.input_dir, Some(PathBuf::from("/tmp/input")));
  }

  #[test]
  fn test_apply_overrides_numeric() {
    let mut config = Config::default();

    config
      .apply_overrides(&vec![
        "jobs=8".to_string(),
        "options_toc_depth=5".to_string(),
      ])
      .unwrap();

    assert_eq!(config.jobs, Some(8));
    assert_eq!(config.options_toc_depth, 5);
  }

  #[test]
  fn test_apply_overrides_invalid_format() {
    let mut config = Config::default();

    let result = config.apply_overrides(&vec!["no_equals_sign".to_string()]);

    assert!(result.is_err());
    assert!(
      result
        .unwrap_err()
        .to_string()
        .contains("Expected KEY=VALUE")
    );
  }

  #[test]
  fn test_apply_overrides_unknown_key() {
    let mut config = Config::default();

    let result = config.apply_overrides(&vec!["unknown_key=value".to_string()]);

    assert!(result.is_err());
    assert!(
      result
        .unwrap_err()
        .to_string()
        .contains("Unknown configuration key")
    );
  }

  #[test]
  fn test_apply_overrides_invalid_boolean() {
    let mut config = Config::default();

    let result =
      config.apply_overrides(&vec!["generate_search=maybe".to_string()]);

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Invalid boolean"));
  }

  #[test]
  fn test_apply_overrides_invalid_numeric() {
    let mut config = Config::default();

    let result = config.apply_overrides(&vec!["jobs=not_a_number".to_string()]);

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Invalid value"));
  }

  // Regression tests for proc-macro-based config system

  #[test]
  fn test_apply_override_string_field() {
    let mut config = Config::default();
    config.apply_override("title", "My Docs").unwrap();
    assert_eq!(config.title, "My Docs");
  }

  #[test]
  fn test_apply_override_bool_field() {
    let mut config = Config::default();

    // Test various boolean formats
    config.apply_override("generate_anchors", "false").unwrap();
    assert!(!config.generate_anchors);

    config.apply_override("generate_anchors", "yes").unwrap();
    assert!(config.generate_anchors);

    config.apply_override("generate_anchors", "0").unwrap();
    assert!(!config.generate_anchors);
  }

  #[test]
  fn test_apply_override_usize_field() {
    let mut config = Config::default();
    config.apply_override("options_toc_depth", "5").unwrap();
    assert_eq!(config.options_toc_depth, 5);
  }

  #[test]
  fn test_apply_override_pathbuf_field() {
    let mut config = Config::default();
    config
      .apply_override("output_dir", "/custom/output")
      .unwrap();
    assert_eq!(config.output_dir, PathBuf::from("/custom/output"));
  }

  #[test]
  fn test_apply_override_option_pathbuf_with_empty() {
    let mut config = Config::default();
    config.input_dir = Some(PathBuf::from("/existing"));

    // Empty string should set to None
    config.apply_override("input_dir", "").unwrap();
    assert!(config.input_dir.is_none());

    // Non-empty should set to Some
    config.apply_override("input_dir", "/new/path").unwrap();
    assert_eq!(config.input_dir, Some(PathBuf::from("/new/path")));
  }

  #[test]
  fn test_apply_override_nested_config() {
    let mut config = Config::default();

    // Enable search via nested key
    config.apply_override("search.enable", "false").unwrap();
    assert!(config.search.is_some());
    assert!(!config.search.as_ref().unwrap().enable);

    // Set nested numeric field
    config
      .apply_override("search.max_heading_level", "2")
      .unwrap();
    assert_eq!(config.search.as_ref().unwrap().max_heading_level, 2);

    // Set nested float field
    config.apply_override("search.boost", "1.5").unwrap();
    assert_eq!(config.search.as_ref().unwrap().boost, Some(1.5));
  }

  #[test]
  fn test_apply_override_postprocess_nested() {
    let mut config = Config::default();

    config
      .apply_override("postprocess.minify_html", "true")
      .unwrap();
    assert!(config.postprocess.is_some());
    assert!(config.postprocess.as_ref().unwrap().minify_html);

    config
      .apply_override("postprocess.minify_css", "true")
      .unwrap();
    assert!(config.postprocess.as_ref().unwrap().minify_css);
  }

  #[test]
  fn test_merge_fields_basic() {
    let mut config = Config::default();
    config.title = "Original".to_string();
    config.generate_anchors = false;

    let other = Config {
      title: "New Title".to_string(),
      generate_anchors: true,
      ..Default::default()
    };

    config.merge_fields(other);

    assert_eq!(config.title, "New Title");
    assert!(config.generate_anchors);
  }

  #[test]
  fn test_merge_fields_option_replacement() {
    let mut config = Config::default();
    assert!(config.search.is_none());

    let other = Config {
      search: Some(crate::search::SearchConfig {
        enable: false,
        ..Default::default()
      }),
      ..Default::default()
    };

    config.merge_fields(other);

    assert!(config.search.is_some());
    assert!(!config.search.as_ref().unwrap().enable);
  }

  #[test]
  fn test_merge_fields_vec_extension() {
    let mut config = Config::default();
    config.stylesheet_paths.push(PathBuf::from("style1.css"));

    let other = Config {
      stylesheet_paths: vec![
        PathBuf::from("style2.css"),
        PathBuf::from("style3.css"),
      ],
      ..Default::default()
    };

    config.merge_fields(other);

    assert_eq!(config.stylesheet_paths.len(), 3);
    assert!(
      config
        .stylesheet_paths
        .contains(&PathBuf::from("style1.css"))
    );
    assert!(
      config
        .stylesheet_paths
        .contains(&PathBuf::from("style2.css"))
    );
  }

  #[test]
  fn test_deprecated_field_still_works() {
    let mut config = Config::default();

    // This should work but log a deprecation warning
    config.apply_override("generate_search", "false").unwrap();

    #[allow(deprecated)]
    {
      assert!(!config.generate_search);
    }
  }

  #[test]
  fn test_f32_parsing() {
    let mut config = Config::default();
    config.apply_override("search.boost", "2.5").unwrap();

    assert!(config.search.is_some());
    assert_eq!(config.search.unwrap().boost, Some(2.5));
  }

  #[test]
  fn test_u8_parsing() {
    let mut config = Config::default();
    config
      .apply_override("search.max_heading_level", "6")
      .unwrap();

    assert!(config.search.is_some());
    assert_eq!(config.search.unwrap().max_heading_level, 6);
  }

  #[test]
  fn test_apply_override_creates_nested_config() {
    let mut config = Config::default();

    // Search config should be created on first nested override
    assert!(config.search.is_none());
    config.apply_override("search.enable", "true").unwrap();
    assert!(config.search.is_some());

    // Postprocess config should be created on first nested override
    assert!(config.postprocess.is_none());
    config
      .apply_override("postprocess.minify_html", "true")
      .unwrap();
    assert!(config.postprocess.is_some());
  }

  #[test]
  fn test_error_message_for_invalid_f32() {
    let mut config = Config::default();

    let result = config.apply_override("search.boost", "not_a_number");
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("Invalid value"));
    assert!(err_msg.contains("search.boost") || err_msg.contains("boost"));
  }

  #[test]
  fn test_option_usize_with_empty() {
    let mut config = Config::default();
    config.apply_override("jobs", "").unwrap();
    assert!(config.jobs.is_none());

    config.apply_override("jobs", "4").unwrap();
    assert_eq!(config.jobs, Some(4));
  }
}
