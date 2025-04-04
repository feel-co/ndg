use std::fs;

use anyhow::Result;
use log::{info, LevelFilter};
use rayon::prelude::*;

mod cli;
mod config;
mod markdown;
mod options;
mod render;
mod search;
mod template;

use cli::Cli;
use config::Config;

fn main() -> Result<()> {
    // Parse command line arguments
    let cli = Cli::parse_args();

    // Create configuration from CLI and/or config file
    let config = Config::load(&cli)?;

    // Initialize logging
    env_logger::Builder::new()
        .filter_level(if cli.verbose {
            LevelFilter::Debug
        } else {
            LevelFilter::Info
        })
        .write_style(env_logger::WriteStyle::Always)
        .init();

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
