use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Configuration options for ndg
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Input directory containing markdown files
    pub input_dir: PathBuf,

    /// Output directory for generated documentation
    pub output_dir: PathBuf,

    /// Path to options.json file (optional)
    pub module_options: Option<PathBuf>,

    /// Path to custom template file
    pub template_path: Option<PathBuf>,

    /// Path to custom stylesheet
    pub stylesheet_path: Option<PathBuf>,

    /// Paths to custom JavaScript files
    pub script_paths: Vec<PathBuf>,

    /// Directory containing additional assets
    pub assets_dir: Option<PathBuf>,

    /// Path to manpage URL mappings JSON file
    pub manpage_urls_path: Option<PathBuf>,

    /// Title for the documentation
    pub title: String,

    /// Number of threads to use for parallel processing
    pub jobs: Option<usize>,

    /// Whether to generate anchors for headings
    pub generate_anchors: bool,

    /// Whether to embed resources in HTML
    pub embed_resources: bool,

    /// Whether to generate a search index
    pub generate_search: bool,

    /// Text to be inserted in the footer
    pub footer_text: String,

    /// Depth of parent categories in options TOC
    pub options_toc_depth: Option<usize>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            input_dir: PathBuf::from("docs"),
            output_dir: PathBuf::from("build"),
            module_options: None,
            template_path: None,
            stylesheet_path: None,
            script_paths: Vec::new(),
            assets_dir: None,
            manpage_urls_path: None,
            title: "ndg documentation".to_string(),
            jobs: None,
            generate_anchors: true,
            embed_resources: false,
            generate_search: true,
            footer_text: "Generated with ndg".to_string(),
            options_toc_depth: Some(2),
        }
    }
}

impl Config {
    /// Create a new configuration from a file
    pub fn from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config = serde_json::from_str(&content)?;
        Ok(config)
    }

    /// Get default template path, either from config or from embedded template
    pub fn get_template_path(&self) -> Option<PathBuf> {
        self.template_path.clone()
    }
}
