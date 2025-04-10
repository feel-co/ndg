use std::fs;

use anyhow::Result;
use log::{info, LevelFilter};

mod cli;
mod completion;
mod config;
mod formatter;
mod manpage;
mod search;
mod template;
mod utils;

use cli::{Cli, Commands};
use config::Config;

fn main() -> Result<()> {
    // Parse command line arguments
    let cli = Cli::parse_args();

    // Initialize logging first so we can log during command handling
    env_logger::Builder::new()
        .filter_level(if cli.verbose {
            LevelFilter::Debug
        } else {
            LevelFilter::Info
        })
        .write_style(env_logger::WriteStyle::Always)
        .init();

    // Handle subcommands
    if let Some(command) = &cli.command {
        match command {
            Commands::Generate {
                output_dir,
                completions_only,
                manpage_only,
            } => {
                return utils::handle_generate_command(
                    output_dir,
                    *completions_only,
                    *manpage_only,
                );
            }

            Commands::Manpage {
                input_dir,
                output_dir,
                section,
                manual,
                jobs,
            } => {
                return utils::handle_manpage_command(
                    input_dir,
                    output_dir,
                    (*section).into(),
                    manual,
                    jobs.unwrap_or(1),
                );
            }

            // Just passing the Options command to the Config handling
            Commands::Options { .. } => {
                // This is handled in Config::load and merge_with_cli
            }
        }
    }

    // Create configuration from CLI and/or config file
    let config = Config::load(&cli)?;

    info!("Starting documentation generation...");

    // Ensure output directory exists
    fs::create_dir_all(&config.output_dir)?;
    info!("Output directory: {}", config.output_dir.display());

    // Setup thread pool once for all parallel operations
    let thread_count = config.jobs.unwrap_or_else(num_cpus::get);
    rayon::ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .build_global()?;

    // Process markdown files if input directory is provided
    let markdown_files = utils::process_markdown_files(&config)?;

    // Process options.json if provided
    let options_processed = utils::process_options_file(&config)?;

    // Check if we need to create a fallback index.html
    utils::ensure_index_html_exists(&config, options_processed, &markdown_files)?;

    // Generate search index if enabled and there are markdown files
    if config.generate_search && !markdown_files.is_empty() {
        search::generate_search_index(&config, &markdown_files)?;
    }

    // Copy assets
    utils::copy_assets(&config)?;

    info!(
        "Documentation generated successfully in {}",
        config.output_dir.display()
    );

    Ok(())
}
