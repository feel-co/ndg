use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

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

fn default_options_toc_depth() -> Option<usize> {
    Some(2)
}

fn default_true() -> bool {
    true
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

    /// Merge CLI arguments into this config, prioritizing CLI values when present
    pub fn merge_with_cli(&mut self, cli: &crate::Cli) {
        // Only override values from CLI if they are explicitly provided
        if let Some(input) = &cli.input {
            self.input_dir = input.clone();
        }

        if let Some(output) = &cli.output {
            self.output_dir = output.clone();
        }

        if cli.jobs.is_some() {
            self.jobs = cli.jobs;
        }

        if cli.template.is_some() {
            self.template_path = cli.template.clone();
        }

        if cli.stylesheet.is_some() {
            self.stylesheet_path = cli.stylesheet.clone();
        }

        if !cli.script.is_empty() {
            self.script_paths = cli.script.clone();
        }

        if let Some(title) = &cli.title {
            self.title = title.clone();
        }

        if cli.footer.is_some() {
            self.footer_text = cli.footer.clone().unwrap();
        }

        if cli.module_options.is_some() {
            self.module_options = cli.module_options.clone();
        }

        if let Some(depth) = cli.options_toc_depth {
            self.options_toc_depth = Some(depth);
        }

        if cli.manpage_urls.is_some() {
            self.manpage_urls_path = cli.manpage_urls.clone();
        }

        if cli.template.is_some() {
            self.template_path = cli.template.clone();
        }

        if cli.template_dir.is_some() {
            self.template_dir = cli.template_dir.clone();
        }
    }

    /// Get template directory path
    pub fn get_template_path(&self) -> Option<PathBuf> {
        // Prioritize template_dir if available
        if let Some(dir) = &self.template_dir {
            Some(dir.clone())
        } else if let Some(path) = &self.template_path {
            // For backward compatibility
            // Check if path is a directory
            if path.is_dir() {
                Some(path.clone())
            } else {
                // If template_path is a file, return its parent directory
                path.parent().map(|p| p.to_path_buf())
            }
        } else {
            None
        }
    }
}
