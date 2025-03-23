use anyhow::Result;
use clap::Parser;
use log::{info, LevelFilter};
use num_cpus;
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
    /// Path to options.json file
    #[arg(short = 'j', long)]
    options_json: Option<PathBuf>,

    /// Path to markdown files directory
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

    /// Title of the documentation
    #[arg(short = 'T', long)] // Changed from 't' to 'T'
    title: String,

    /// Number of threads to use for parallel processing
    #[arg(short = 'p', long = "jobs")]
    jobs: Option<usize>,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
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
        options_json: cli.options_json,
        template_path: cli.template,
        stylesheet_path: cli.stylesheet,
        title: cli.title,
        jobs: cli.jobs,
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
    if let Some(options_path) = &config.options_json {
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
