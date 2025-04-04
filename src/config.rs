use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::cli::Cli;

// I know this looks silly, but my understanding is that this is the most
// type-correct and re-usable way. Functions allow for more complex default
// values that can't be expressed as literals. For example, creating a
// PathBuf would require execution, not just a literal value. Should be fine.
fn default_input_dir() -> PathBuf {
    PathBuf::from("docs")
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

const fn default_options_toc_depth() -> Option<usize> {
    Some(2)
}

const fn default_true() -> bool {
    true
}

/// Configuration options for ndg
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// Input directory containing markdown files
    #[serde(default = "default_input_dir")]
    pub input_dir: PathBuf,

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

    /// Path to custom stylesheet
    #[serde(default)]
    pub stylesheet_path: Option<PathBuf>,

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

    /// Whether to embed resources in HTML
    #[serde(default)]
    pub embed_resources: bool,

    /// Whether to generate a search index
    #[serde(default = "default_true")]
    pub generate_search: bool,

    /// Text to be inserted in the footer
    #[serde(default = "default_footer_text")]
    pub footer_text: String,

    /// Depth of parent categories in options TOC
    #[serde(default = "default_options_toc_depth")]
    pub options_toc_depth: Option<usize>,
}

impl Config {
    /// Create a new configuration from a file
    /// Only TOML and JSON are supported for the time being.
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;

        if let Some(ext) = path.extension() {
            match ext.to_str().unwrap_or("").to_lowercase().as_str() {
                "json" => serde_json::from_str(&content).with_context(|| {
                    format!("Failed to parse JSON config from {}", path.display())
                }),
                "toml" => toml::from_str(&content).with_context(|| {
                    format!("Failed to parse TOML config from {}", path.display())
                }),
                _ => Err(anyhow::anyhow!(
                    "Unsupported config file format: {}",
                    path.display()
                )),
            }
        } else {
            Err(anyhow::anyhow!(
                "Config file has no extension: {}",
                path.display()
            ))
        }
    }

    /// Create a config from CLI arguments
    pub fn from_cli(cli: &Cli) -> Self {
        let mut config = Self::default();
        config.merge_with_cli(cli);
        config
    }

    /// Load config from file and CLI arguments
    pub fn load(cli: &Cli) -> Result<Self> {
        if let Some(config_path) = &cli.config_file {
            let mut config = Self::from_file(config_path)
                .with_context(|| format!("Failed to load config from {}", config_path.display()))?;
            config.merge_with_cli(cli);
            Ok(config)
        } else {
            Ok(Self::from_cli(cli))
        }
    }

    /// Merge CLI arguments into this config, prioritizing CLI values when present
    pub fn merge_with_cli(&mut self, cli: &Cli) {
        if let Some(input) = &cli.input {
            self.input_dir = input.clone();
        }

        if let Some(output) = &cli.output {
            self.output_dir = output.clone();
        }

        self.jobs = cli.jobs.or(self.jobs);

        if let Some(template) = &cli.template {
            self.template_path = Some(template.clone());
        }

        if let Some(template_dir) = &cli.template_dir {
            self.template_dir = Some(template_dir.clone());
        }

        if let Some(stylesheet) = &cli.stylesheet {
            self.stylesheet_path = Some(stylesheet.clone());
        }

        if !cli.script.is_empty() {
            self.script_paths = cli.script.clone();
        }

        if let Some(title) = &cli.title {
            self.title = title.clone();
        }

        if let Some(footer) = &cli.footer {
            self.footer_text = footer.clone();
        }

        if let Some(module_options) = &cli.module_options {
            self.module_options = Some(module_options.clone());
        }

        self.options_toc_depth = cli.options_toc_depth.or(self.options_toc_depth);

        if let Some(manpage_urls) = &cli.manpage_urls {
            self.manpage_urls_path = Some(manpage_urls.clone());
        }
    }

    /// Get template directory path
    pub fn get_template_path(&self) -> Option<PathBuf> {
        // First check explicit template directory
        self.template_dir
            .clone()
            // Then check if template_path is a directory
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
}
