use std::path::PathBuf;

use clap::{Parser, Subcommand};
use clap_verbosity_flag::Verbosity;

/// Command line interface for ndg
#[derive(Parser, Debug)]
#[command(author, version, about = "NDG: Not a Docs Generator")]
pub struct Cli {
  /// Subcommand to execute (see [`Commands`])
  #[command(subcommand)]
  pub command: Option<Commands>,

  /// Logging verbosity (-v for info, -vv for debug, -vvv for trace, -q for
  /// quiet)
  #[command(flatten)]
  pub verbose: Verbosity,

  /// Path to configuration file(s) (TOML or JSON, can be specified multiple
  /// times) Multiple files are merged in order, with later files overriding
  /// earlier ones
  #[arg(short = 'c', long = "config-file", action = clap::ArgAction::Append)]
  pub config_files: Vec<PathBuf>,

  /// Override configuration values (KEY=VALUE format, can be used multiple
  /// times)
  #[arg(long = "config", action = clap::ArgAction::Append)]
  pub config_overrides: Vec<String>,
}

/// All supported subcommands for the ndg CLI.
#[derive(Subcommand, Debug)]
#[expect(
  clippy::large_enum_variant,
  reason = "Clap subcommands keep their parsed fields inline for \
            straightforward access"
)]
pub enum Commands {
  /// Initialize a new NDG configuration file
  Init {
    /// Path to create the configuration file at
    #[arg(short, long, default_value = "ndg.toml")]
    output: PathBuf,

    /// Format of the configuration file.
    #[arg(short = 'F', long, default_value = "toml", value_parser = ["toml", "json"])]
    format: String,

    /// Force overwrite if file already exists
    #[arg(short, long)]
    force: bool,
  },

  /// Export default templates to a directory for customization.
  Export {
    /// Output directory for template files.
    #[arg(short, long, default_value = "templates")]
    output_dir: PathBuf,

    /// Whether to overwrite existing files.
    #[arg(long)]
    force: bool,

    /// Specific templates to export (e.g., html, css, js). If not specified,
    /// exports all.
    #[arg(short, long, action = clap::ArgAction::Append)]
    templates: Vec<String>,
  },

  /// Process documentation and generate HTML.
  Html {
    /// Path to the directory containing markdown files.
    #[arg(short, long)]
    input_dir: Option<PathBuf>,

    /// Output directory for generated documentation.
    #[arg(short, long, alias = "dest-dir")]
    output_dir: Option<PathBuf>,

    /// Number of threads to use for parallel processing.
    #[arg(short = 'p', long = "jobs")]
    jobs: Option<usize>,

    /// Path to custom template file.
    #[arg(short, long)]
    template: Option<PathBuf>,

    /// Path to directory containing template files. Templates override
    /// built-in ones (default.html, options.html, etc.)
    #[arg(long = "template-dir")]
    template_dir: Option<PathBuf>,

    /// Path to custom stylesheet (can be specified multiple times)
    #[arg(short, long, action = clap::ArgAction::Append)]
    stylesheet: Vec<PathBuf>,

    /// Path to custom JavaScript file (can be specified multiple times)
    #[arg(long, action = clap::ArgAction::Append)]
    script: Vec<PathBuf>,

    /// Title of the documentation. Will be used in various components via
    /// the templating options.
    #[arg(short = 'T', long)]
    title: Option<String>,

    /// Footer text for the documentation.
    #[arg(short = 'f', long)]
    footer: Option<String>,

    /// Path to a JSON file containing module options in the same format
    /// expected by nixos-render-docs.
    #[arg(short = 'j', long)]
    module_options: Option<PathBuf>,

    /// Include only module options whose names start with this prefix.
    #[arg(long = "options-filter-prefix")]
    option_filter_prefix: Option<String>,

    /// Include only module options whose type contains this text.
    #[arg(long = "options-filter-type")]
    option_filter_type: Option<String>,

    /// Include only module options whose name or description contains this
    /// text.
    #[arg(long = "options-filter-search")]
    option_filter_search: Option<String>,

    /// Include only module options that define a default value.
    #[arg(long = "options-filter-has-default", action = clap::ArgAction::SetTrue)]
    options_has_default: bool,

    /// Include only module options that have a non-empty description.
    #[arg(long = "options-filter-has-description", action = clap::ArgAction::SetTrue)]
    options_has_description: bool,

    /// Hide module options marked internal or invisible.
    #[arg(long = "options-filter-hide-internal", action = clap::ArgAction::SetTrue)]
    options_hide_internal: bool,

    /// Path to manpage URL mappings JSON file.
    #[arg(long = "manpage-urls")]
    manpage_urls: Option<PathBuf>,

    /// Whether to enable syntax highlighting for code blocks.
    #[arg(long = "highlight-code", action = clap::ArgAction::SetTrue)]
    highlight_code: bool,

    /// GitHub revision for linking to source files.
    #[arg(long)]
    revision: Option<String>,

    /// Serve the generated documentation locally after generation
    #[cfg(feature = "serve")]
    #[arg(long = "serve", action = clap::ArgAction::SetTrue)]
    serve: bool,

    /// Port to serve documentation on (only used with --serve).
    #[cfg(feature = "serve")]
    #[arg(long = "serve-port", default_value = "3000")]
    serve_port: u16,

    /// Open the generated documentation in the default browser.
    #[arg(long = "open", action = clap::ArgAction::SetTrue)]
    open: bool,
  },

  /// Generate manpage from options.
  #[command(alias = "manpage")]
  Man {
    /// Path to a JSON file containing module options.
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

  /// Generate PDF from options.
  Pdf {
    /// Path to a JSON file containing module options.
    #[arg(short = 'j', long, required = true)]
    module_options: PathBuf,

    /// Output file for the generated PDF.
    #[arg(short, long)]
    output_file: Option<PathBuf>,

    /// Header text to include at the beginning of the document.
    #[arg(short = 'H', long)]
    header: Option<String>,

    /// Footer text to include at the end of the document.
    #[arg(short = 'F', long)]
    footer: Option<String>,

    /// Title for the PDF document.
    #[arg(short = 'T', long)]
    title: Option<String>,
  },
}

impl Cli {
  /// Parse command line arguments into a [`Cli`] struct.
  #[must_use]
  pub fn parse_args() -> Self {
    Self::parse()
  }
}
