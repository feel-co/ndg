use std::fs;

use anyhow::Result;
use log::{info, LevelFilter};
use rayon::prelude::*;

mod cli;
mod completion;
mod config;
mod manpage;
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
                if *completions_only {
                    completion::generate_comp(output_dir)?;
                    println!("Shell completions generated in {}", output_dir.display());
                } else if *manpage_only {
                    completion::generate_manpage(output_dir)?;
                    println!("Manpage generated in {}", output_dir.display());
                } else {
                    completion::generate_all(output_dir)?;
                }
                return Ok(());
            }

            Commands::Manpage {
                input_dir,
                output_dir,
                section,
                manual,
                jobs,
            } => {
                info!("Generating manpages from markdown files");

                // Ensure output directory exists
                fs::create_dir_all(output_dir)?;

                // Use config's title as manual name if not specified
                let config = config::Config::default(); // Create minimal config
                let manual_name = manual.as_deref().unwrap_or(&config.title);

                // Process markdown files to manpages - now using the dedicated module
                manpage::generate_manpages_from_directory(
                    input_dir,
                    output_dir,
                    *section,
                    manual_name,
                    *jobs,
                )?;

                info!(
                    "Manpages generated successfully in {}",
                    output_dir.display()
                );

                return Ok(());
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
    ensure_index_html_exists(&config, options_processed)?;

    // Generate search index if enabled and there are markdown files
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

fn ensure_index_html_exists(config: &Config, options_processed: bool) -> Result<()> {
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

    // Create a fallback index page if input directory was provided but no index.md existed
    if config.input_dir.is_some() {
        info!("No index.md found, creating fallback index.html");

        let mut content = format!(
            "<h1>{}</h1>\n<p>This is a fallback page created by ndg.</p>",
            &config.title
        );

        // List other markdown files if they exist
        if let Some(input_dir) = &config.input_dir {
            let entries: Vec<_> = walkdir::WalkDir::new(input_dir)
                .follow_links(true)
                .into_iter()
                .filter_map(std::result::Result::ok)
                .filter(|e| {
                    e.path().is_file() && e.path().extension().is_some_and(|ext| ext == "md")
                })
                .collect();

            if !entries.is_empty() {
                let mut file_list = String::from("<h2>Available Documents</h2>\n<ul>\n");

                for entry in entries {
                    if let Ok(rel_path) = entry.path().strip_prefix(input_dir) {
                        let mut html_path = rel_path.to_path_buf();
                        html_path.set_extension("html");

                        // Get page title from first heading or filename
                        let page_title = if let Ok(content) = fs::read_to_string(entry.path()) {
                            if let Some(first_line) = content.lines().next() {
                                if let Some(title) = first_line.strip_prefix("# ") {
                                    title.trim().to_string()
                                } else {
                                    html_path
                                        .file_stem()
                                        .unwrap_or_default()
                                        .to_string_lossy()
                                        .to_string()
                                }
                            } else {
                                html_path
                                    .file_stem()
                                    .unwrap_or_default()
                                    .to_string_lossy()
                                    .to_string()
                            }
                        } else {
                            html_path
                                .file_stem()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .to_string()
                        };

                        file_list.push_str(&format!(
                            "  <li><a href=\"{}\">{}</a></li>\n",
                            html_path.to_string_lossy(),
                            page_title
                        ));
                    }
                }

                file_list.push_str("</ul>");
                content.push_str(&file_list);
            }
        }

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
