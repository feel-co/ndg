pub mod templates;

use std::{
  fs,
  path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::cli::{Cli, Commands};

// I know this looks silly, but my understanding is that this is the most
// type-correct and re-usable way. Functions allow for more complex default
// values that can't be expressed as literals. For example, creating a
// PathBuf would require execution, not just a literal value. Should be fine.
const fn default_input_dir() -> Option<PathBuf> {
  None
}

fn default_output_dir() -> PathBuf {
  PathBuf::from("build")
}

fn default_title() -> String {
  "ndg documentation".to_string()
}

fn default_footer_text() -> String {
  "Generated with ndg".to_string()
}

fn default_revision() -> String {
  "local".to_string()
}

const fn default_options_toc_depth() -> usize {
  2
}

const fn default_true() -> bool {
  true
}

/// Configuration options for ndg
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
  /// Input directory containing markdown files
  #[serde(default = "default_input_dir")]
  pub input_dir: Option<PathBuf>,

  /// Output directory for generated documentation
  #[serde(default = "default_output_dir")]
  pub output_dir: PathBuf,

  /// Path to options.json file (optional)
  #[serde(default)]
  pub module_options: Option<PathBuf>,

  /// Path to custom template file
  #[serde(default)]
  pub template_path: Option<PathBuf>,

  /// Path to template directory containing all template files
  #[serde(default)]
  pub template_dir: Option<PathBuf>,

  /// Paths to custom stylesheets
  #[serde(default)]
  pub stylesheet_paths: Vec<PathBuf>,

  /// Paths to custom JavaScript files
  #[serde(default)]
  pub script_paths: Vec<PathBuf>,

  /// Directory containing additional assets
  #[serde(default)]
  pub assets_dir: Option<PathBuf>,

  /// Path to manpage URL mappings JSON file
  #[serde(default)]
  pub manpage_urls_path: Option<PathBuf>,

  /// Title for the documentation
  #[serde(default = "default_title")]
  pub title: String,

  /// Number of threads to use for parallel processing
  #[serde(default)]
  pub jobs: Option<usize>,

  /// Whether to generate anchors for headings
  #[serde(default = "default_true")]
  pub generate_anchors: bool,

  /// Whether to generate a search index
  #[serde(default = "default_true")]
  pub generate_search: bool,

  /// Text to be inserted in the footer
  #[serde(default = "default_footer_text")]
  pub footer_text: String,

  /// Depth of parent categories in options TOC
  #[serde(default = "default_options_toc_depth")]
  pub options_toc_depth: usize,

  /// Whether to enable syntax highlighting for code blocks
  #[serde(default = "default_true")]
  pub highlight_code: bool,

  /// GitHub revision for linking to source files
  #[serde(default = "default_revision")]
  pub revision: String,

  /// `OpenGraph` tags to inject into the HTML head (e.g., {"og:title": "...",
  /// "og:image": "..."})
  #[serde(default)]
  pub opengraph: Option<std::collections::HashMap<String, String>>,

  /// Additional meta tags to inject into the HTML head (e.g., {"description":
  /// "...", "keywords": "..."})
  #[serde(default)]
  pub meta_tags: Option<std::collections::HashMap<String, String>>,
}

impl Config {
  /// Create a new configuration from a file
  /// Only TOML and JSON are supported for the time being.
  pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
    let path = path.as_ref();
    let content = fs::read_to_string(path).with_context(|| {
      format!("Failed to read config file: {}", path.display())
    })?;

    path.extension().map_or_else(
      || {
        Err(anyhow::anyhow!(
          "Config file has no extension: {}",
          path.display()
        ))
      },
      |ext| {
        match ext.to_str().unwrap_or("").to_lowercase().as_str() {
          "json" => {
            serde_json::from_str(&content).with_context(|| {
              format!("Failed to parse JSON config from {}", path.display())
            })
          },
          "toml" => {
            toml::from_str(&content).with_context(|| {
              format!("Failed to parse TOML config from {}", path.display())
            })
          },
          _ => {
            Err(anyhow::anyhow!(
              "Unsupported config file format: {}",
              path.display()
            ))
          },
        }
      },
    )
  }

  /// Load config from file and CLI arguments
  pub fn load(cli: &Cli) -> Result<Self> {
    let mut config = if let Some(config_path) = &cli.config_file {
      // Config file explicitly specified via CLI
      Self::from_file(config_path).with_context(|| {
        format!("Failed to load config from {}", config_path.display())
      })?
    } else if let Some(discovered_config) = Self::find_config_file() {
      // Found a config file in a standard location
      log::info!(
        "Using discovered config file: {}",
        discovered_config.display()
      );
      Self::from_file(&discovered_config).with_context(|| {
        format!(
          "Failed to load discovered config from {}",
          discovered_config.display()
        )
      })?
    } else {
      Self::default()
    };

    // Merge CLI arguments
    config.merge_with_cli(cli);

    // Get options from command if present
    if let Some(Commands::Html { .. }) = &cli.command {
      // Validation is handled in merge_with_cli
    } else {
      // No Html command, validate required fields if no config file was
      // specified
      if cli.config_file.is_none() && Self::find_config_file().is_none() {
        // If there's no config file (explicit or discovered) and no Html
        // command, we're missing required data
        return Err(anyhow::anyhow!(
          "Neither config file nor 'html' subcommand provided. Use 'ndg html' \
           or provide a config file with --config."
        ));
      }
    }

    // We need *at least one* source of content
    if config.input_dir.is_none() && config.module_options.is_none() {
      return Err(anyhow::anyhow!(
        "At least one of input directory or module options must be provided."
      ));
    }

    // Validate input_dir if it's provided
    if let Some(ref input_dir) = config.input_dir {
      if !input_dir.exists() {
        return Err(anyhow::anyhow!(
          "Input directory does not exist: {}",
          input_dir.display()
        ));
      }
    }

    // Validate all paths
    config.validate_paths()?;

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
        self.options_toc_depth = *toc_depth;
      }

      if let Some(manpage_urls) = manpage_urls {
        self.manpage_urls_path = Some(manpage_urls.clone());
      }

      // Handle the generate-search flag when explicitly set
      if let Some(generate_search_val) = generate_search {
        self.generate_search = *generate_search_val;
      }

      // Handle the highlight-code flag when explicitly set
      if let Some(highlight_code_val) = highlight_code {
        self.highlight_code = *highlight_code_val;
      }

      if let Some(revision) = revision {
        self.revision.clone_from(revision);
      }
    }
  }

  /// Get template directory path or file parent
  #[must_use]
  pub fn get_template_path(&self) -> Option<PathBuf> {
    // First check explicit template directory
    self.template_dir
            .clone()
            // Then check if template_path is a directory or use its parent
            .or_else(|| {
                self.template_path.as_ref().and_then(|path| {
                    if path.is_dir() {
                        Some(path.clone())
                    } else {
                        path.parent().map(PathBuf::from)
                    }
                })
            })
  }

  /// Get template file path for a specific template name
  #[must_use]
  pub fn get_template_file(&self, name: &str) -> Option<PathBuf> {
    // First check if there's a direct template path and it matches the name
    if let Some(path) = &self.template_path {
      // Only use template_path if it's a file and its filename matches the
      // requested name
      if path.is_file() && path.file_name().is_some_and(|fname| fname == name) {
        return Some(path.clone());
      }
    }

    // Otherwise check template directory
    self.get_template_path().map(|dir| dir.join(name))
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
  pub fn validate_paths(&self) -> Result<()> {
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

    // Template path should exist if specified
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

    // Stylesheet should exist if specified
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

    // Check for template/template-dir conflict and log a warning
    if self.template_path.is_some() && self.template_dir.is_some() {
      log::warn!(
        "Both template file and template directory are specified. Template \
         file will only be used if it matches the requested template name."
      );
    }

    // Return error if we found any issues
    if !errors.is_empty() {
      let error_message = errors.join("\n");
      return Err(anyhow::anyhow!(
        "Configuration path validation errors:\n{}",
        error_message
      ));
    }

    Ok(())
  }

  /// Generate a default configuration file with commented explanations
  pub fn generate_default_config(format: &str, path: &Path) -> Result<()> {
    // Get template from the templates module
    let config_content = crate::config::templates::get_template(format)
      .map_err(|e| anyhow::anyhow!("{}", e))?;

    fs::write(path, config_content).with_context(|| {
      format!("Failed to write default config to {}", path.display())
    })?;

    log::info!("Created default configuration file: {}", path.display());
    Ok(())
  }

  /// Export embedded templates to a directory for customization
  pub fn export_templates(output_dir: &Path, force: bool) -> Result<()> {
    // Create output directory if it doesn't exist
    fs::create_dir_all(output_dir).with_context(|| {
      format!(
        "Failed to create template directory: {}",
        output_dir.display()
      )
    })?;

    // Get all embedded template sources
    let templates = Self::get_template_sources();

    for (filename, content) in templates {
      let file_path = output_dir.join(filename);

      // Check if file exists and force flag
      if file_path.exists() && !force {
        log::warn!(
          "Skipping existing file: {} (use --force to overwrite)",
          file_path.display()
        );
        continue;
      }

      fs::write(&file_path, content).with_context(|| {
        format!("Failed to write template file: {}", file_path.display())
      })?;

      log::info!("Exported template: {}", file_path.display());
    }

    log::info!("Templates exported to: {}", output_dir.display());
    log::info!(
      "Use --template-dir {} to customize your documentation",
      output_dir.display()
    );
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

    // CSS and JS assets
    templates
      .insert("default.css", include_str!("../../templates/default.css"));
    templates.insert("search.js", include_str!("../../templates/search.js"));
    templates.insert("main.js", include_str!("../../templates/main.js"));

    templates
  }
}
