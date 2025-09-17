use std::fmt;

/// Error type for template operations.
///
/// Represents the various errors that can occur during template
/// operations, providing clear error messages and context for debugging.
#[derive(Debug)]
pub enum TemplateError {
  /// Indicates that the requested configuration format is not supported.
  /// Contains the name of the unsupported format.
  UnsupportedFormat(String),
}

impl fmt::Display for TemplateError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::UnsupportedFormat(format) => {
        write!(f, "Unsupported config format: {format}")
      },
    }
  }
}

impl std::error::Error for TemplateError {}

/// Default configuration template in TOML. We've also included sufficient
/// amount of comments to explain each field so that the user is not immediately
/// lost. Yuck though, I do not like having to embed a string one bit.
pub const DEFAULT_TOML_TEMPLATE: &str = r#"# NDG Configuration File

# Input directory containing markdown files
input_dir = "docs"

# Output directory for generated documentation
output_dir = "build"

# Path to options.json file (optional)
# module_options = "options.json"

# Title for the documentation
title = "My Project Documentation"

# Footer text for the documentation
footer_text = "Generated with ndg"

# Number of threads to use for parallel processing (defaults to number of CPU cores)
# jobs = 4

# Template customization
# Path to custom template file
# template_path = "templates/custom.html"

# Path to template directory containing all template files
# template_dir = "templates"

# Path to custom stylesheet
# stylesheet_path = "assets/custom.css"

# Paths to custom JavaScript files
# script_paths = ["assets/custom.js", "assets/search.js"]

# Directory containing additional assets
# assets_dir = "assets"

# Path to manpage URL mappings JSON file
# manpage_urls_path = "manpage-urls.json"

# Whether to generate anchors for headings
generate_anchors = true

# Whether to generate a search index
generate_search = true

# Depth of parent categories in options TOC
options_toc_depth = 2

# Whether to enable syntax highlighting for code blocks
highlight_code = true

# GitHub revision for linking to source files (defaults to 'local')
revision = "main"

# OpenGraph tags to inject into the HTML head (example: { og:title = "My Project", og:image = "..." })
# opengraph = { og:title = "My Project", og:image = "https://example.com/image.png" }

# Additional meta tags to inject into the HTML head (example: { description = "Docs", keywords = "nix,docs" })
# meta_tags = { description = "Documentation for My Project", keywords = "nix,docs,example" }
"#;

/// Default configuration template in JSON format.
pub const DEFAULT_JSON_TEMPLATE: &str = r#"{
  "input_dir": "docs",
  "output_dir": "build",
  "title": "My Project Documentation",
  "footer_text": "Generated with ndg",
  "generate_anchors": true,
  "generate_search": true,
  "options_toc_depth": 2,
  "highlight_code": true,
  "revision": "main",
  "opengraph": {
    "og:title": "My Project",
  },
  "meta_tags": {
    "description": "Documentation for My Project",
    "keywords": "nix,docs,ndg,nixos"
  }
}
"#;

/// Get the correct configuration template based on the requested format.
pub fn get_template(format: &str) -> Result<&'static str, TemplateError> {
  match format.to_lowercase().as_str() {
    "toml" => Ok(DEFAULT_TOML_TEMPLATE),
    "json" => Ok(DEFAULT_JSON_TEMPLATE),
    _ => Err(TemplateError::UnsupportedFormat(format.to_string())),
  }
}
