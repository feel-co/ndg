pub mod meta;
pub mod postprocess;
pub mod sidebar;
pub mod templates;

use std::{
  collections::HashMap,
  fs,
  path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::{
  cli::{Cli, Commands},
  error::NdgError,
};

/// Configuration for the NDG documentation generator.
///
/// [`Config`] holds all configuration options for controlling documentation
/// generation, including input/output directories, template customization,
/// search, syntax highlighting, and more. Fields are typically loaded from a
/// TOML or JSON config file, but can also be set via CLI arguments.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
  /// Input directory containing markdown files.
  pub input_dir: Option<PathBuf>,

  /// Output directory for generated documentation.
  pub output_dir: PathBuf,

  /// Path to options.json file (optional).
  pub module_options: Option<PathBuf>,

  /// Path to custom template file.
  pub template_path: Option<PathBuf>,

  /// Path to template directory containing all template files.
  pub template_dir: Option<PathBuf>,

  /// Paths to custom stylesheets.
  pub stylesheet_paths: Vec<PathBuf>,

  /// Paths to custom JavaScript files.
  pub script_paths: Vec<PathBuf>,

  /// Directory containing additional assets.
  pub assets_dir: Option<PathBuf>,

  /// Path to manpage URL mappings JSON file.
  pub manpage_urls_path: Option<PathBuf>,

  /// Title for the documentation.
  pub title: String,

  /// Number of threads to use for parallel processing.
  pub jobs: Option<usize>,

  /// Whether to generate anchors for headings.
  pub generate_anchors: bool,

  /// Whether to generate a search index.
  pub generate_search: bool,

  /// Text to be inserted in the footer.
  pub footer_text: String,

  /// Depth of parent categories in options TOC.
  pub options_toc_depth: usize,

  /// Whether to enable syntax highlighting for code blocks.
  pub highlight_code: bool,

  /// GitHub revision for linking to source files.
  pub revision: String,

  /// Files that are included via {=include=}, mapped to their parent input
  /// files
  #[serde(skip)]
  pub included_files:
    std::collections::HashMap<std::path::PathBuf, std::path::PathBuf>,

  /// How to handle hard tabs in code blocks.
  pub tab_style: String,

  /// Meta tag configuration (`OpenGraph` and additional tags).
  pub meta: Option<meta::MetaConfig>,

  /// Sidebar configuration.
  pub sidebar: Option<sidebar::SidebarConfig>,

  /// Postprocessing configuration for HTML/CSS/JS minification
  pub postprocess: Option<postprocess::PostprocessConfig>,

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
      manpage_urls_path: None,
      title:             "ndg documentation".to_string(),
      jobs:              None,
      generate_anchors:  true,
      generate_search:   true,
      footer_text:       "Generated with ndg".to_string(),
      options_toc_depth: 2,
      highlight_code:    true,
      revision:          "local".to_string(),
      included_files:    std::collections::HashMap::new(),
      tab_style:         "none".to_string(),
      meta:              None,
      sidebar:           None,
      postprocess:       None,
      opengraph:         None,
      meta_tags:         None,
    }
  }
}

impl Config {
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
  pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, NdgError> {
    let path = path.as_ref();
    let content = fs::read_to_string(path).map_err(|e| {
      NdgError::Config(format!(
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
              .map_err(NdgError::from)
              .map_err(|e| {
                NdgError::Config(format!(
                  "Failed to parse JSON config from {}: {}",
                  path.display(),
                  e
                ))
              })
          },
          "toml" => {
            toml::from_str(&content)
              .map_err(NdgError::from)
              .map_err(|e| {
                NdgError::Config(format!(
                  "Failed to parse TOML config from {}: {}",
                  path.display(),
                  e
                ))
              })
          },
          _ => {
            Err(NdgError::Config(format!(
              "Unsupported config file format: {}",
              path.display()
            )))
          },
        }
      },
      None => {
        Err(NdgError::Config(format!(
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
  pub fn load(cli: &Cli) -> Result<Self, NdgError> {
    let mut config = if !cli.config_files.is_empty() {
      // Config file(s) explicitly specified via CLI
      // Load and merge them in order
      let mut merged_config =
        Self::from_file(&cli.config_files[0]).map_err(|e| {
          NdgError::Config(format!(
            "Failed to load config from {}: {}",
            cli.config_files[0].display(),
            e
          ))
        })?;

      // Merge additional config files if provided
      for config_path in &cli.config_files[1..] {
        let additional_config = Self::from_file(config_path).map_err(|e| {
          NdgError::Config(format!(
            "Failed to load config from {}: {}",
            config_path.display(),
            e
          ))
        })?;
        merged_config.merge(additional_config);
      }

      if cli.config_files.len() > 1 {
        log::info!("Loaded and merged {} config files", cli.config_files.len());
      }

      merged_config
    } else if let Some(discovered_config) = Self::find_config_file() {
      // Found a config file in a standard location
      log::info!(
        "Using discovered config file: {}",
        discovered_config.display()
      );
      Self::from_file(&discovered_config).map_err(|e| {
        NdgError::Config(format!(
          "Failed to load discovered config from {}: {}",
          discovered_config.display(),
          e
        ))
      })?
    } else {
      Self::default()
    };

    // Merge CLI arguments
    config.merge_with_cli(cli);

    // Apply config overrides from --config KEY=VALUE flags
    if !cli.config_overrides.is_empty() {
      config.apply_overrides(&cli.config_overrides)?;
    }

    // Get options from command if present
    if let Some(Commands::Html { .. }) = &cli.command {
      // Validation is handled in merge_with_cli
    } else {
      // No Html command, validate required fields if no config file was
      // specified
      if cli.config_files.is_empty() && Self::find_config_file().is_none() {
        // If there's no config file (explicit or discovered) and no Html
        // command, we're missing required data
        return Err(NdgError::Config(
          "Neither config file nor 'html' subcommand provided. Use 'ndg html' \
           or provide a config file with --config-file."
            .to_string(),
        ));
      }
    }

    // We need *at least one* source of content
    if config.input_dir.is_none() && config.module_options.is_none() {
      return Err(NdgError::Config(
        "At least one of input directory or module options must be provided."
          .to_string(),
      ));
    }

    // Validate input_dir if it's provided
    if let Some(ref input_dir) = config.input_dir {
      if !input_dir.exists() {
        return Err(NdgError::Config(format!(
          "Input directory does not exist: {}",
          input_dir.display()
        )));
      }
    }

    // Validate all paths
    config.validate_paths()?;

    // Validate and compile sidebar configuration if present
    if let Some(ref mut sidebar) = config.sidebar {
      sidebar.validate().map_err(|e| {
        NdgError::Config(format!(
          "Sidebar configuration validation failed: {e}"
        ))
      })?;
    }

    Ok(config)
  }

  /// Merge CLI arguments into this config, prioritizing CLI values when
  /// present
  pub fn merge_with_cli(&mut self, cli: &Cli) {
    // Handle options from the Html subcommand if present
    if let Some(Commands::Html {
      input_dir,
      output_dir,
      jobs,
      template,
      template_dir,
      stylesheet,
      script,
      title,
      footer,
      module_options,
      options_toc_depth,
      manpage_urls,
      generate_search,
      highlight_code,

      revision,
    }) = &cli.command
    {
      // Set input directory from CLI if provided
      if let Some(input_dir) = input_dir {
        self.input_dir = Some(input_dir.clone());
      }

      if let Some(output_dir) = output_dir {
        self.output_dir.clone_from(output_dir);
      }

      self.jobs = jobs.or(self.jobs);

      if let Some(template) = template {
        self.template_path = Some(template.clone());
      }

      if let Some(template_dir) = template_dir {
        self.template_dir = Some(template_dir.clone());
      }

      // Append stylesheet paths rather than replacing them
      if !stylesheet.is_empty() {
        self.stylesheet_paths.extend(stylesheet.iter().cloned());
      }

      // Append script paths rather than replacing them
      if !script.is_empty() {
        // Append CLI scripts to existing ones rather than replacing
        self.script_paths.extend(script.iter().cloned());
      }

      if let Some(title) = title {
        self.title.clone_from(title);
      }

      if let Some(footer) = footer {
        self.footer_text.clone_from(footer);
      }

      if let Some(module_options) = module_options {
        self.module_options = Some(module_options.clone());
      }

      if let Some(toc_depth) = options_toc_depth {
        log::warn!(
          "The --options-depth flag is deprecated and will be removed in a \
           future release. Use '--config sidebar.options.depth={toc_depth}' \
           or set 'sidebar.options.depth' in your config file instead."
        );
        self.options_toc_depth = *toc_depth;
      }

      if let Some(manpage_urls) = manpage_urls {
        self.manpage_urls_path = Some(manpage_urls.clone());
      }

      // Handle the generate-search flag - only override if flag was explicitly
      // provided
      if *generate_search {
        self.generate_search = true;
      }

      // Handle the highlight-code flag - only override if flag was explicitly
      // provided
      if *highlight_code {
        self.highlight_code = true;
      }

      if let Some(revision) = revision {
        self.revision.clone_from(revision);
      }
    }
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
  /// # Panics
  ///
  /// Panics if `strip_prefix` fails after a successful `starts_with` check for
  /// nested keys. This should never happen in practice as the guard condition
  /// ensures the prefix exists.
  ///
  /// # Example
  ///
  /// ```rust, ignore
  /// config.apply_overrides(&vec![
  ///     "generate_search=false".to_string(),
  ///     "title=My Documentation".to_string(),
  /// ])?;
  /// ```
  pub fn apply_overrides(
    &mut self,
    overrides: &[String],
  ) -> Result<(), NdgError> {
    for override_str in overrides {
      let (key, value) = override_str.split_once('=').ok_or_else(|| {
        NdgError::Config(format!(
          "Invalid config override format: '{override_str}'. Expected \
           KEY=VALUE"
        ))
      })?;

      let key = key.trim();
      let value = value.trim();

      match key {
        // Path fields
        "input_dir" => {
          self.input_dir = if value.is_empty() {
            None
          } else {
            Some(PathBuf::from(value))
          };
        },
        "output_dir" => {
          self.output_dir = PathBuf::from(value);
        },
        "module_options" => {
          self.module_options = if value.is_empty() {
            None
          } else {
            Some(PathBuf::from(value))
          };
        },
        "template_path" => {
          self.template_path = if value.is_empty() {
            None
          } else {
            Some(PathBuf::from(value))
          };
        },
        "template_dir" => {
          self.template_dir = if value.is_empty() {
            None
          } else {
            Some(PathBuf::from(value))
          };
        },
        "assets_dir" => {
          self.assets_dir = if value.is_empty() {
            None
          } else {
            Some(PathBuf::from(value))
          };
        },
        "manpage_urls_path" => {
          self.manpage_urls_path = if value.is_empty() {
            None
          } else {
            Some(PathBuf::from(value))
          };
        },

        // String fields
        "title" => {
          self.title = value.to_string();
        },
        "footer_text" => {
          self.footer_text = value.to_string();
        },
        "revision" => {
          self.revision = value.to_string();
        },
        "tab_style" => {
          self.tab_style = value.to_string();
        },

        // Boolean fields
        "generate_anchors" => {
          self.generate_anchors = Self::parse_bool(value, key)?;
        },
        "generate_search" => {
          self.generate_search = Self::parse_bool(value, key)?;
        },
        "highlight_code" => {
          self.highlight_code = Self::parse_bool(value, key)?;
        },

        // Numeric fields
        "jobs" => {
          self.jobs = if value.is_empty() {
            None
          } else {
            Some(value.parse::<usize>().map_err(|_| {
              NdgError::Config(format!(
                "Invalid value for 'jobs': '{value}'. Expected a positive \
                 integer"
              ))
            })?)
          };
        },
        "options_toc_depth" => {
          log::warn!(
            "The 'options_toc_depth' config key is deprecated. Use \
             'sidebar.options.depth' instead."
          );
          self.options_toc_depth = value.parse::<usize>().map_err(|_| {
            NdgError::Config(format!(
              "Invalid value for 'options_toc_depth': '{value}'. Expected a \
               positive integer"
            ))
          })?;
        },

        // Nested sidebar.options.depth
        "sidebar.options.depth" => {
          let depth = value.parse::<usize>().map_err(|_| {
            NdgError::Config(format!(
              "Invalid value for 'sidebar.options.depth': '{value}'. Expected \
               a positive integer"
            ))
          })?;

          // Create sidebar.options if it doesn't exist
          if self.sidebar.is_none() {
            self.sidebar = Some(sidebar::SidebarConfig::default());
          }
          if let Some(ref mut sidebar) = self.sidebar {
            if sidebar.options.is_none() {
              sidebar.options = Some(sidebar::OptionsConfig::default());
            }
            if let Some(ref mut options) = sidebar.options {
              options.depth = depth;
            }
          }
        },

        // `Vec<PathBuf>` fields - append single value
        "stylesheet_paths" => {
          self.stylesheet_paths.push(PathBuf::from(value));
        },

        "script_paths" => {
          self.script_paths.push(PathBuf::from(value));
        },

        // Nested meta.opengraph.* - set individual key=value
        key if key.starts_with("meta.opengraph.") => {
          #[allow(
            clippy::expect_used,
            reason = "Guard condition ensures strip_prefix cannot fail"
          )]
          let subkey = key.strip_prefix("meta.opengraph.").expect(
            "Key starts with 'meta.opengraph.' prefix, strip_prefix cannot \
             fail",
          );
          if self.meta.is_none() {
            self.meta = Some(meta::MetaConfig::default());
          }
          if let Some(ref mut meta) = self.meta {
            if meta.opengraph.is_none() {
              meta.opengraph = Some(HashMap::new());
            }
            if let Some(ref mut map) = meta.opengraph {
              map.insert(subkey.to_string(), value.to_string());
            }
          }
        },

        // Nested meta.tags.* - set individual key=value
        key if key.starts_with("meta.tags.") => {
          #[allow(
            clippy::expect_used,
            reason = "Guard condition ensures strip_prefix cannot fail"
          )]
          let subkey = key.strip_prefix("meta.tags.").expect(
            "Key starts with 'meta.tags.' prefix, strip_prefix cannot fail",
          );
          if self.meta.is_none() {
            self.meta = Some(meta::MetaConfig::default());
          }
          if let Some(ref mut meta) = self.meta {
            if meta.tags.is_none() {
              meta.tags = Some(HashMap::new());
            }
            if let Some(ref mut map) = meta.tags {
              map.insert(subkey.to_string(), value.to_string());
            }
          }
        },

        // Nested postprocess.* - boolean fields for minification control
        "postprocess.minify_html" => {
          let value = Self::parse_bool(value, key)?;
          if self.postprocess.is_none() {
            self.postprocess = Some(postprocess::PostprocessConfig::default());
          }
          if let Some(ref mut pp) = self.postprocess {
            pp.minify_html = value;
          }
        },
        "postprocess.minify_css" => {
          let value = Self::parse_bool(value, key)?;
          if self.postprocess.is_none() {
            self.postprocess = Some(postprocess::PostprocessConfig::default());
          }
          if let Some(ref mut pp) = self.postprocess {
            pp.minify_css = value;
          }
        },
        "postprocess.minify_js" => {
          let value = Self::parse_bool(value, key)?;
          if self.postprocess.is_none() {
            self.postprocess = Some(postprocess::PostprocessConfig::default());
          }
          if let Some(ref mut pp) = self.postprocess {
            pp.minify_js = value;
          }
        },

        // Nested postprocess.html.* - HTML minification options
        key if key.starts_with("postprocess.html.") => {
          #[allow(
            clippy::expect_used,
            reason = "Guard condition ensures strip_prefix cannot fail"
          )]
          let subkey = key.strip_prefix("postprocess.html.").expect(
            "Key starts with 'postprocess.html.' prefix, strip_prefix cannot \
             fail",
          );
          if self.postprocess.is_none() {
            self.postprocess = Some(postprocess::PostprocessConfig::default());
          }
          if let Some(ref mut pp) = self.postprocess {
            if pp.html.is_none() {
              pp.html = Some(postprocess::HtmlMinifyOptions::default());
            }
            if let Some(ref mut html_opts) = pp.html {
              match subkey {
                "collapse_whitespace" => {
                  html_opts.collapse_whitespace = Self::parse_bool(value, key)?;
                },
                "remove_comments" => {
                  html_opts.remove_comments = Self::parse_bool(value, key)?;
                },
                _ => {
                  return Err(NdgError::Config(format!(
                    "Unknown postprocess.html configuration key: '{key}'"
                  )));
                },
              }
            }
          }
        },

        // Deprecated: opengraph.* - redirect to meta.opengraph.*
        key if key.starts_with("opengraph.") => {
          #[allow(
            clippy::expect_used,
            reason = "Guard condition ensures strip_prefix cannot fail"
          )]
          let subkey = key.strip_prefix("opengraph.").expect(
            "Key starts with 'opengraph.' prefix, strip_prefix cannot fail",
          );
          log::warn!(
            "The 'opengraph.{subkey}' config override is deprecated. Use \
             'meta.opengraph.{subkey}' instead."
          );
          #[allow(deprecated)]
          {
            if self.opengraph.is_none() {
              self.opengraph = Some(HashMap::new());
            }
            if let Some(ref mut map) = self.opengraph {
              map.insert(subkey.to_string(), value.to_string());
            }
          }
        },

        // Deprecated: meta_tags.* - redirect to meta.tags.*
        key if key.starts_with("meta_tags.") => {
          #[allow(
            clippy::expect_used,
            reason = "Guard condition ensures strip_prefix cannot fail"
          )]
          let subkey = key.strip_prefix("meta_tags.").expect(
            "Key starts with 'meta_tags.' prefix, strip_prefix cannot fail",
          );
          log::warn!(
            "The 'meta_tags.{subkey}' config override is deprecated. Use \
             'meta.tags.{subkey}' instead."
          );
          #[allow(deprecated)]
          {
            if self.meta_tags.is_none() {
              self.meta_tags = Some(HashMap::new());
            }
            if let Some(ref mut map) = self.meta_tags {
              map.insert(subkey.to_string(), value.to_string());
            }
          }
        },

        _ => {
          return Err(NdgError::Config(format!(
            "Unknown configuration key: '{key}'. See documentation for \
             supported keys."
          )));
        },
      }
    }

    Ok(())
  }

  /// Parse a boolean value from a string.
  ///
  /// Accepts: true/false, yes/no, 1/0 (case-insensitive)
  fn parse_bool(value: &str, key: &str) -> Result<bool, NdgError> {
    match value.to_lowercase().as_str() {
      "true" | "yes" | "1" => Ok(true),
      "false" | "no" | "0" => Ok(false),
      _ => {
        Err(NdgError::Config(format!(
          "Invalid boolean value for '{key}': '{value}'. Expected true/false, \
           yes/no, or 1/0"
        )))
      },
    }
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
  pub fn merge(&mut self, other: Self) {
    // Option fields - replace if other has Some
    if other.input_dir.is_some() {
      self.input_dir = other.input_dir;
    }
    if other.module_options.is_some() {
      self.module_options = other.module_options;
    }
    if other.template_path.is_some() {
      self.template_path = other.template_path;
    }
    if other.template_dir.is_some() {
      self.template_dir = other.template_dir;
    }
    if other.assets_dir.is_some() {
      self.assets_dir = other.assets_dir;
    }
    if other.manpage_urls_path.is_some() {
      self.manpage_urls_path = other.manpage_urls_path;
    }
    if other.jobs.is_some() {
      self.jobs = other.jobs;
    }
    if other.sidebar.is_some() {
      self.sidebar = other.sidebar;
    }
    if other.meta.is_some() {
      self.meta = other.meta;
    }
    if other.postprocess.is_some() {
      self.postprocess = other.postprocess;
    }

    // Vec fields - append
    self.stylesheet_paths.extend(other.stylesheet_paths);
    self.script_paths.extend(other.script_paths);

    // Plain fields - always replace with other's value
    // We only replace if other's value is not the default, to allow proper
    // layering
    if other.output_dir.as_os_str() != "build" {
      self.output_dir = other.output_dir;
    }
    if other.title != "ndg documentation" {
      self.title = other.title;
    }
    if other.footer_text != "Generated with ndg" {
      self.footer_text = other.footer_text;
    }
    if other.revision != "local" {
      self.revision = other.revision;
    }
    if other.tab_style != "none" {
      self.tab_style = other.tab_style;
    }

    // Numeric/boolean fields - always use other's value
    // These don't have a meaningful "unset" state, so we always take the
    // value
    self.options_toc_depth = other.options_toc_depth;
    self.generate_anchors = other.generate_anchors;
    self.generate_search = other.generate_search;
    self.highlight_code = other.highlight_code;

    // Deprecated fields - merge if present, with warnings
    #[allow(deprecated)]
    {
      if let Some(other_og) = other.opengraph {
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
      if let Some(other_meta) = other.meta_tags {
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

    // excluded_files is runtime-only (#[serde(skip)]), so we don't merge it
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
    if let Some(ref path) = self.template_path {
      if path.is_dir() {
        return Some(path.clone());
      }
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
    if let Some(ref path) = self.template_path {
      if path.is_file() {
        if let Some(filename) = path.file_name() {
          if filename == name {
            return Some(path.clone());
          }
        }
      }
    }

    None
  }

  /// Search for config files in common locations
  #[must_use]
  pub fn find_config_file() -> Option<PathBuf> {
    // Common config file names to look for
    let config_filenames = [
      "ndg.toml",
      "ndg.json",
      ".ndg.toml",
      ".ndg.json",
      ".config/ndg.toml",
      ".config/ndg.json",
    ];

    // First try current directory
    let current_dir = std::env::current_dir().ok()?;

    // Check each filename in the current directory
    for filename in &config_filenames {
      let config_path = current_dir.join(filename);
      if config_path.exists() {
        return Some(config_path);
      }
    }

    // If we have a $XDG_CONFIG_HOME environment variable, check there too
    if let Ok(xdg_config_home) = std::env::var("XDG_CONFIG_HOME") {
      let xdg_config_dir = PathBuf::from(xdg_config_home);
      for filename in &["ndg.toml", "ndg.json"] {
        let config_path = xdg_config_dir.join(filename);
        if config_path.exists() {
          return Some(config_path);
        }
      }
    }

    // Then check $HOME/.config/ndg/
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
  }

  /// Validate all paths specified in the configuration
  ///
  /// # Errors
  ///
  /// Returns an error if any configured path does not exist or is invalid.
  pub fn validate_paths(&self) -> Result<(), NdgError> {
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
    if let Some(ref template_path) = self.template_path {
      if !template_path.exists() {
        errors.push(format!(
          "Template file does not exist: {}",
          template_path.display()
        ));
      }
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

    // Return error if we found any issues
    if !errors.is_empty() {
      let error_message = errors.join("\n");
      return Err(NdgError::Config(format!(
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
  ) -> Result<(), NdgError> {
    // Get template from the templates module
    let config_content = crate::config::templates::get_template(format)
      .map_err(|e| NdgError::Template(e.to_string()))?;

    fs::write(path, config_content).map_err(|e| {
      NdgError::Config(format!(
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
  ) -> Result<(), NdgError> {
    // Create output directory if it doesn't exist
    fs::create_dir_all(output_dir).map_err(|e| {
      NdgError::Config(format!(
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
        NdgError::Config(format!(
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
    let mut templates = std::collections::HashMap::new();

    // HTML templates
    templates
      .insert("default.html", include_str!("../../templates/default.html"));
    templates
      .insert("options.html", include_str!("../../templates/options.html"));
    templates
      .insert("search.html", include_str!("../../templates/search.html"));
    templates.insert(
      "options_toc.html",
      include_str!("../../templates/options_toc.html"),
    );
    templates
      .insert("navbar.html", include_str!("../../templates/navbar.html"));
    templates
      .insert("footer.html", include_str!("../../templates/footer.html"));

    // CSS and JS assets
    templates
      .insert("default.css", include_str!("../../templates/default.css"));
    templates.insert("search.js", include_str!("../../templates/search.js"));
    templates.insert(
      "search-worker.js",
      include_str!("../../templates/search-worker.js"),
    );
    templates.insert("main.js", include_str!("../../templates/main.js"));

    templates
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
    base.generate_search = true;
    base.highlight_code = false;

    let mut override_config = Config::default();
    override_config.generate_search = false;
    override_config.highlight_code = true;

    base.merge(override_config);

    // Boolean fields should take override's value
    assert!(!base.generate_search);
    assert!(base.highlight_code);
  }

  #[test]
  fn test_apply_overrides_boolean() {
    let mut config = Config::default();

    config
      .apply_overrides(&vec![
        "generate_search=false".to_string(),
        "highlight_code=yes".to_string(),
      ])
      .unwrap();

    assert!(!config.generate_search);
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
    assert!(
      result
        .unwrap_err()
        .to_string()
        .contains("Expected a positive integer")
    );
  }
}
