use std::path::PathBuf;

use clap::{
    Command,
    CommandFactory,
    Parser,
    Subcommand,
};

/// Command line interface for ndg
#[derive(Parser, Debug)]
#[command(author, version, about = "Nix Documentation Generator")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Enable verbose debug logging
    #[arg(short, long)]
    pub verbose: bool,

    /// Path to configuration file (TOML or JSON)
    #[arg(short = 'c', long = "config")]
    pub config_file: Option<PathBuf>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Initialize a new NDG configuration file
    Init {
        /// Path to create the configuration file at
        #[arg(short, long, default_value = "ndg.toml")]
        output: PathBuf,

        /// Format of the configuration file (toml or json)
        #[arg(short = 'F', long, default_value = "toml", value_parser = ["toml", "json"])]
        format: String,

        /// Force overwrite if file already exists
        #[arg(short, long)]
        force: bool,
    },

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

    /// Process documentation and generate HTML
    Html {
        /// Path to the directory containing markdown files
        #[arg(short, long)]
        input_dir: Option<PathBuf>,

        /// Output directory for generated documentation
        #[arg(short, long)]
        output_dir: Option<PathBuf>,

        /// Number of threads to use for parallel processing
        #[arg(short = 'p', long = "jobs")]
        jobs: Option<usize>,

        /// Path to custom template file
        #[arg(short, long)]
        template: Option<PathBuf>,

        /// Path to directory containing template files (default.html,
        /// options.html, etc.) For missing required files, ndg will fall
        /// back to its internal templates.
        #[arg(long = "template-dir")]
        template_dir: Option<PathBuf>,

        /// Path to custom stylesheet
        #[arg(short, long)]
        stylesheet: Option<PathBuf>,

        /// Path to custom Javascript file to include. This can be specified
        /// multiple times to create multiple script tags in order.
        #[arg(long, action = clap::ArgAction::Append)]
        script: Vec<PathBuf>,

        /// Title of the documentation. Will be used in various components via
        /// the templating options.
        #[arg(short = 'T', long)]
        title: Option<String>,

        /// Footer text for the documentation
        #[arg(short = 'f', long)]
        footer: Option<String>,

        /// Path to a JSON file containing module options in the same format
        /// expected by nixos-render-docs.
        #[arg(short = 'j', long)]
        module_options: Option<PathBuf>,

        /// Depth of parent categories in options TOC
        #[arg(long = "options-depth", value_parser = clap::value_parser!(usize))]
        options_toc_depth: Option<usize>,

        /// Path to manpage URL mappings JSON file
        #[arg(long = "manpage-urls")]
        manpage_urls: Option<PathBuf>,

        /// Whether to generate search functionality
        #[arg(short = 'S', long = "generate-search")]
        generate_search: Option<bool>,

        /// Whether to enable syntax highlighting for code blocks
        #[arg(long = "highlight-code")]
        highlight_code: Option<bool>,

        /// GitHub revision for linking to source files (defaults to 'local')
        #[arg(long)]
        revision: Option<String>,
    },

    /// Generate manpage from options
    Manpage {
        /// Path to a JSON file containing module options
        #[arg(short = 'j', long, required = true)]
        module_options: PathBuf,

        /// Output file for the generated manpage
        #[arg(short, long)]
        output_file: Option<PathBuf>,

        /// Header text to include at the beginning of the manpage
        #[arg(short = 'H', long)]
        header: Option<String>,

        /// Footer text to include at the end of the manpage
        #[arg(short = 'F', long)]
        footer: Option<String>,

        /// Title for the manpage
        #[arg(short = 'T', long)]
        title: Option<String>,

        /// Section number for the manpage
        #[arg(short = 's', long, default_value = "5")]
        section: u8,
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
