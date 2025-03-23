use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Configuration options for the documentation generator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Input directory containing markdown files
    pub input_dir: PathBuf,

    /// Output directory for generated documentation
    pub output_dir: PathBuf,

    /// Path to options.json file (optional)
    pub options_json: Option<PathBuf>,

    /// Path to custom template file
    pub template_path: Option<PathBuf>,

    /// Path to custom stylesheet
    pub stylesheet_path: Option<PathBuf>,

    /// Directory containing additional assets
    pub assets_dir: Option<PathBuf>,

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
}

impl Default for Config {
    fn default() -> Self {
        Self {
            input_dir: PathBuf::from("docs"),
            output_dir: PathBuf::from("build"),
            options_json: None,
            template_path: None,
            stylesheet_path: None,
            assets_dir: None,
            title: "ndg documentation".to_string(),
            jobs: None,
            generate_anchors: true,
            embed_resources: false,
            generate_search: true,
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
