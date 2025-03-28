use anyhow::{Context, Result};
use clap::Parser;
use log::{info, LevelFilter};
use rayon::prelude::*;
use std::fs;
use std::path::PathBuf;

mod config;
mod markdown;
mod options;
mod render;
mod search;
mod template;

use config::Config;

#[derive(Parser, Debug)]
#[command(author, version)]
pub struct Cli {
    /// Path to the directory containing markdown files
    #[arg(short, long, required_unless_present = "config_file")]
    input: Option<PathBuf>,

    /// Output directory for generated documentation
    #[arg(short, long, required_unless_present = "config_file")]
    output: Option<PathBuf>,

    /// Number of threads to use for parallel processing
    #[arg(short = 'p', long = "jobs")]
    jobs: Option<usize>,

    /// Enable verbose debug logging
    #[arg(short, long)]
    verbose: bool,

    /// Path to custom template file
    #[arg(short, long)]
    template: Option<PathBuf>,

    /// Path to directory containing template files (default.html, options.html, etc.)
    /// For missing required files, ndg will fall back to its internal templates.
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

    /// Path to configuration file (TOML or JSON)
    #[arg(short = 'c', long = "config")]
    config_file: Option<PathBuf>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    env_logger::Builder::new()
        .filter_level(if cli.verbose {
            LevelFilter::Debug
        } else {
            LevelFilter::Info
        })
        .init();

    // Create configuration - first load from file if specified, then merge with CLI args
    let mut config = if let Some(config_path) = &cli.config_file {
        info!("Loading configuration from {}", config_path.display());
        Config::from_file(config_path)
            .with_context(|| format!("Failed to load config from {}", config_path.display()))?
    } else {
        Config::default()
    };

    // Merge CLI arguments, which take precedence over config file
    config.merge_with_cli(&cli);

    info!("Starting documentation generation...");
    info!("Input directory: {}", config.input_dir.display());
    info!("Output directory: {}", config.output_dir.display());

    // Process markdown files
    let markdown_files = markdown::collect_markdown_files(&config.input_dir)?;
    info!("Found {} markdown files", markdown_files.len());

    // Ensure output directory exists
    fs::create_dir_all(&config.output_dir)?;

    // Process files in parallel
    let thread_count = config.jobs.unwrap_or_else(num_cpus::get);
    rayon::ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .build_global()?;

    markdown_files
        .par_iter()
        .try_for_each(|file_path| markdown::process_markdown_file(&config, file_path))?;

    // Process options.json if provided
    if let Some(options_path) = &config.module_options {
        info!("Processing options.json from {}", options_path.display());
        options::process_options(&config, options_path)?;
    }

    // Generate search index if enabled
    if config.generate_search {
        search::generate_search_index(&config, &markdown_files)?;
    }

    // Copy assets
    render::copy_assets(&config)?;

    info!(
        "Documentation generated successfully in {}",
        config.output_dir.display()
    );

    Ok(())
}
