use anyhow::Result;
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
struct Cli {
    /// Path to a JSON file containing module options in the same format
    /// expected by nixos-render-docs.
    #[arg(short = 'j', long)]
    module_options: Option<PathBuf>,

    /// Path to the directory containing markdown files
    #[arg(short, long)]
    input: PathBuf,

    /// Output directory for generated documentation
    #[arg(short, long)]
    output: PathBuf,

    /// Path to custom template file
    #[arg(short, long)]
    template: Option<PathBuf>,

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
    title: String,

    /// Path to manpage URL mappings JSON file
    #[arg(long = "manpage-urls")]
    manpage_urls: Option<PathBuf>,

    /// Number of threads to use for parallel processing
    #[arg(short = 'p', long = "jobs")]
    jobs: Option<usize>,

    /// Enable verbose debug logging
    #[arg(short, long)]
    verbose: bool,

    /// Footer text for the documentation
    #[arg(short = 'f', long)]
    footer: Option<String>,
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

    // Create configuration from CLI args
    let config = Config {
        input_dir: cli.input,
        output_dir: cli.output,
        module_options: cli.module_options,
        template_path: cli.template,
        stylesheet_path: cli.stylesheet,
        script_paths: cli.script,
        manpage_urls_path: cli.manpage_urls,
        title: cli.title,
        jobs: cli.jobs,
        footer_text: cli
            .footer
            .unwrap_or_else(|| "Generated with ndg".to_string()),
        ..Default::default()
    };

    info!("Starting documentation generation...");

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

    // Generate search index
    search::generate_search_index(&config, &markdown_files)?;

    // Copy assets
    render::copy_assets(&config)?;

    info!(
        "Documentation generated successfully in {}",
        config.output_dir.display()
    );

    Ok(())
}
