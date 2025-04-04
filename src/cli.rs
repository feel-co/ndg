use std::path::PathBuf;

use clap::Parser;

/// Command line interface for ndg
#[derive(Parser, Debug)]
#[command(author, version)]
pub struct Cli {
    /// Path to the directory containing markdown files
    #[arg(short, long, required_unless_present = "config_file")]
    pub input: Option<PathBuf>,

    /// Output directory for generated documentation
    #[arg(short, long, required_unless_present = "config_file")]
    pub output: Option<PathBuf>,

    /// Number of threads to use for parallel processing
    #[arg(short = 'p', long = "jobs")]
    pub jobs: Option<usize>,

    /// Enable verbose debug logging
    #[arg(short, long)]
    pub verbose: bool,

    /// Path to custom template file
    #[arg(short, long)]
    pub template: Option<PathBuf>,

    /// Path to directory containing template files (default.html, options.html, etc.)
    /// For missing required files, ndg will fall back to its internal templates.
    #[arg(long = "template-dir")]
    pub template_dir: Option<PathBuf>,

    /// Path to custom stylesheet
    #[arg(short, long)]
    pub stylesheet: Option<PathBuf>,

    /// Path to custom Javascript file to include. This can be specified
    /// multiple times to create multiple script tags in order.
    #[arg(long, action = clap::ArgAction::Append)]
    pub script: Vec<PathBuf>,

    /// Title of the documentation. Will be used in various components via
    /// the templating options.
    #[arg(short = 'T', long)]
    pub title: Option<String>,

    /// Footer text for the documentation
    #[arg(short = 'f', long)]
    pub footer: Option<String>,

    /// Path to a JSON file containing module options in the same format
    /// expected by nixos-render-docs.
    #[arg(short = 'j', long)]
    pub module_options: Option<PathBuf>,

    /// Depth of parent categories in options TOC
    #[arg(long = "options-depth", value_parser = clap::value_parser!(usize))]
    pub options_toc_depth: Option<usize>,

    /// Path to manpage URL mappings JSON file
    #[arg(long = "manpage-urls")]
    pub manpage_urls: Option<PathBuf>,

    /// Path to configuration file (TOML or JSON)
    #[arg(short = 'c', long = "config")]
    pub config_file: Option<PathBuf>,
}

impl Cli {
    pub fn parse_args() -> Self {
        Self::parse()
    }
}
