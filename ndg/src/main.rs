use std::fs;

use color_eyre::eyre::{Context, Result};
use log::{LevelFilter, info};

mod cli;
mod config;
mod error;
mod formatter;
mod html;
mod manpage;
mod utils;

use cli::{Cli, Commands};
use color_eyre::eyre::bail;
use config::Config;

fn main() -> Result<()> {
  color_eyre::install()?;

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
      Commands::Init {
        output,
        format,
        force,
      } => {
        // Check if file already exists and that we're not forcing overwrite
        if output.exists() && !force {
          bail!(
            "Configuration file already exists: {}. Use --force to overwrite.",
            output.display()
          );
        }

        // Create parent directories if needed
        if let Some(parent) = output.parent() {
          if !parent.exists() {
            fs::create_dir_all(parent).wrap_err_with(|| {
              format!("Failed to create directory: {}", parent.display())
            })?;
            info!("Created directory: {}", parent.display());
          }
        }

        // Generate the config file
        Config::generate_default_config(format, output).wrap_err_with(
          || {
            format!(
              "Failed to generate configuration file: {}",
              output.display()
            )
          },
        )?;

        info!(
          "Configuration file created successfully. Edit it to customize your \
           documentation generation."
        );
        return Ok(());
      },

      Commands::ExportTemplates { output_dir, force } => {
        Config::export_templates(output_dir, *force).wrap_err_with(|| {
          format!("Failed to export templates to {}", output_dir.display())
        })?;
        return Ok(());
      },

      Commands::Manpage {
        module_options,
        output_file,
        header,
        footer,
        title,
        section,
      } => {
        return manpage::generate_manpage(
          module_options,
          output_file.as_deref(),
          title.as_deref(),
          header.as_deref(),
          footer.as_deref(),
          *section,
        );
      },

      // The Html command is handled in Config::load and merge_with_cli
      Commands::Html { .. } => {},
    }
  }

  // Create configuration from CLI and/or config file
  let mut config = Config::load(&cli)?;

  // Run the main documentation generation process
  generate_documentation(&mut config)
}

/// Main documentation generation process
fn generate_documentation(config: &mut Config) -> Result<()> {
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
  // Collect all included files first
  let excluded_files = utils::collect_included_files(config)?;
  config.excluded_files = excluded_files;

  // Process markdown files
  let markdown_files = utils::process_markdown_files(config)?;

  // Process options if provided
  let options_processed = utils::process_module_options(config)?;

  // Check if we need to create a fallback index.html
  utils::ensure_index(config, options_processed, &markdown_files)?;

  // Generate search index if enabled, regardless of whether there are markdown
  // files
  if config.generate_search {
    html::search::generate_search_index(config, &markdown_files)?;
  }

  // Copy assets
  utils::copy_assets(config)?;

  info!(
    "Documentation generated successfully in {}",
    config.output_dir.display()
  );

  Ok(())
}
