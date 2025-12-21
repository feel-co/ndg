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

# Search configuration
[search]
# Whether to generate a search index
enable = true

# Maximum heading level to index
max_heading_level = 3

# Depth of parent categories in options TOC
options_toc_depth = 2

# Whether to enable syntax highlighting for code blocks
highlight_code = true

# How to handle hard tabs in code blocks (one of "none", "warn", or "normalize")
tab_style = "none"

# GitHub revision for linking to source files (defaults to 'local')
revision = "main"

# OpenGraph tags to inject into the HTML head (example: { og:title = "My Project", og:image = "..." })
# opengraph = { og:title = "My Project", og:image = "https://example.com/image.png" }

# Additional meta tags to inject into the HTML head (example: { description = "Docs", keywords = "nix,docs" })
# meta_tags = { description = "Documentation for My Project", keywords = "nix,docs,example" }

# Sidebar configuration
# [sidebar]
# Enable numbering for sidebar items
# numbered = true

# Include special files in numbering sequence
# Only has effect when numbered = true.
# number_special_files = false

# Ordering algorithm for sidebar items: "alphabetical", "custom", or "filesystem"
# ordering = "alphabetical"

# Pattern-based sidebar matching rules
# Rules are evaluated in order, and the first matching rule is applied.
# All specified conditions are expected to match.

# Simple exact path match (shorthand syntax)
# [[sidebar.matches]]
# path = "getting-started.md"
# position = 1

# Exact title match with override (shorthand syntax)
# [[sidebar.matches]]
# title = "Release Notes"
# new_title = "What's New"
# position = 999

# Regex patterns for more advanced matching (nested syntax required)
# [[sidebar.matches]]
# path.regex = "^api/.*\\.md$"
# position = 50

# Combined conditions (path regex & title regex)
# [[sidebar.matches]]
# path.regex = "^guides/.*\\.md$"
# title.regex = "^Tutorial:.*"
# position = 10

# Options sidebar configuration
# [sidebar.options]
# Depth of parent categories in options TOC (defaults to root options_toc_depth)
# depth = 2

# Ordering algorithm for options: "alphabetical", "custom", or "filesystem"
# ordering = "alphabetical"

# Pattern-based options matching rules
# Rules are evaluated in order, and the first matching rule is applied.

# Hide internal options from the TOC
# [[sidebar.options.matches]]
# name.regex = "^myInternal\\..*"
# hidden = true

# Rename a category for display
# [[sidebar.options.matches]]
# name = "programs.git"
# new_name = "Git Configuration"
# position = 5

# Set custom depth for specific options
# [[sidebar.options.matches]]
# name.regex = "^services\\..*"
# depth = 3
# position = 10

# Basic exact name match (shorthand syntax)
# [[sidebar.options.matches]]
# name = "networking.firewall"
# new_name = "Firewall Settings"
# position = 1
"#;

/// Default configuration template in JSON format.
pub const DEFAULT_JSON_TEMPLATE: &str = r#"{
  "input_dir": "docs",
  "output_dir": "build",
  "title": "My Project Documentation",
  "footer_text": "Generated with ndg",
  "generate_anchors": true,
  "search": {
    "enable": true,
    "max_heading_level": 3
  },
  "options_toc_depth": 2,
  "highlight_code": true,
  "tab_style": "none",
  "revision": "main",
  "opengraph": {
    "og:title": "My Project"
  },
  "meta_tags": {
    "description": "Documentation for My Project",
    "keywords": "nix,docs,ndg,nixos"
  },
  "sidebar": {
    "numbered": true,
    "number_special_files": false,
    "ordering": "alphabetical",
    "matches": [
      {
        "path": "getting-started.md",
        "position": 1
      },
      {
        "title": "Release Notes",
        "new_title": "What's New",
        "position": 999
      },
      {
        "path": {
          "regex": "^api/.*\\.md$"
        },
        "position": 50
      }
    ],
    "options": {
      "depth": 2,
      "ordering": "alphabetical",
      "matches": [
        {
          "name": {
            "regex": "^myInternal\\..*"
          },
          "hidden": true
        },
        {
          "name": "programs.git",
          "new_name": "Git Configuration",
          "position": 5
        },
        {
          "name": {
            "regex": "^services\\..*"
          },
          "depth": 3,
          "position": 10
        }
      ]
    }
  }
}
"#;

/// Get the correct configuration template based on the requested format.
///
/// # Errors
///
/// Returns an error if the requested format is not supported.
pub fn get_template(format: &str) -> Result<&'static str, TemplateError> {
  match format.to_lowercase().as_str() {
    "toml" => Ok(DEFAULT_TOML_TEMPLATE),
    "json" => Ok(DEFAULT_JSON_TEMPLATE),
    _ => Err(TemplateError::UnsupportedFormat(format.to_string())),
  }
}
