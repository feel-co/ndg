use std::path::PathBuf;

use clap::{Command, CommandFactory, Parser, Subcommand};

/// Command line interface for ndg
#[derive(Parser, Debug)]
#[command(author, version, about = "Nix Documentation Generator")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Path to the directory containing markdown files
    #[arg(short, long)]
    pub input: Option<PathBuf>,

    /// Output directory for generated documentation
    #[arg(short, long)]
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

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Generate shell completions and manpages
    Generate {
        /// Directory to output generated files
        #[arg(short, long, default_value = "dist")]
        output_dir: PathBuf,

        /// Only generate shell completions
        #[arg(long)]
        completions_only: bool,

        /// Only generate manpage
        #[arg(long)]
        manpage_only: bool,
    },
}

impl Cli {
    pub fn parse_args() -> Self {
        Self::parse()
    }

    pub fn command() -> Command {
        <Self as CommandFactory>::command()
    }
}
