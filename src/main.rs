use std::fs;

use anyhow::Result;
use log::{info, LevelFilter};
use rayon::prelude::*;

mod cli;
mod completion;
mod config;
mod markdown;
mod options;
mod render;
mod search;
mod template;

use cli::{Cli, Commands};
use config::Config;

fn main() -> Result<()> {
    // Parse command line arguments
    let cli = Cli::parse_args();

    // Handle subcommands first, before loading config
    if let Some(command) = &cli.command {
        match command {
            Commands::Generate {
                output_dir,
                completions_only,
                manpage_only,
            } => {
                if *completions_only {
                    completion::generate_shell_completions(output_dir)?;
                    println!("Shell completions generated in {}", output_dir.display());
                } else if *manpage_only {
                    completion::generate_manpage(output_dir)?;
                    println!("Manpage generated in {}", output_dir.display());
                } else {
                    completion::generate_all_artifacts(output_dir)?;
                }
                return Ok(());
            }
        }
    }

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

    // Ensure output directory exists
    fs::create_dir_all(&config.output_dir)?;
    info!("Output directory: {}", config.output_dir.display());

    // Process markdown files if input directory is provided
    let markdown_files = if let Some(ref input_dir) = config.input_dir {
        info!("Input directory: {}", input_dir.display());
        let files = markdown::collect_markdown_files(input_dir)?;
        info!("Found {} markdown files", files.len());

        if !files.is_empty() {
            // Process files in parallel
            let thread_count = config.jobs.unwrap_or_else(num_cpus::get);
            rayon::ThreadPoolBuilder::new()
                .num_threads(thread_count)
                .build_global()?;

            files
                .par_iter()
                .try_for_each(|file_path| markdown::process_markdown_file(&config, file_path))?;
        }

        files
    } else {
        info!("No input directory provided, skipping markdown processing");
        Vec::new()
    };

    // Process options.json if provided
    let options_processed = if let Some(options_path) = &config.module_options {
        info!("Processing options.json from {}", options_path.display());
        options::process_options(&config, options_path)?;
        true
    } else {
        false
    };

    // Check if we need to create a fallback index.html
    ensure_index_html_exists(&config, &markdown_files, options_processed)?;

    // Generate search index if enabled
    if config.generate_search && !markdown_files.is_empty() {
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

fn ensure_index_html_exists(
    config: &Config,
    markdown_files: &[std::path::PathBuf],
    options_processed: bool,
) -> Result<()> {
    let index_path = config.output_dir.join("index.html");

    // Check if index.html already exists (already generated from index.md)
    if index_path.exists() {
        return Ok(());
    }

    // If options were processed, try to use options.html as index
    if options_processed {
        let options_path = config.output_dir.join("options.html");
        if options_path.exists() {
            info!("Using options.html as index.html");

            // Copy the content of options.html to index.html
            let options_content = fs::read(&options_path)?;
            fs::write(&index_path, options_content)?;
            return Ok(());
        }
    }

    // If no content was generated at all, create a basic welcome page
    if markdown_files.is_empty() && !options_processed {
        info!("No content found, creating fallback index.html");
        let content = format!(
            "<h1>{}</h1>\n<p>Welcome to the documentation.</p>\n<p>No content has been generated. Please provide either markdown files or options.json.</p>",
            &config.title
        );

        // Create a simple HTML document using our template system
        let headers = vec![markdown::Header {
            text: config.title.clone(),
            level: 1,
            id: "welcome".to_string(),
        }];

        let html = template::render(
            config,
            &content,
            &config.title,
            &headers,
            &std::path::PathBuf::from("index.html"),
        )?;

        fs::write(&index_path, html)?;
    }

    Ok(())
}
