use std::{fs, path::PathBuf};

use color_eyre::eyre::{Context, Result, bail};
use log::{LevelFilter, info};
use ndg::{config, html, manpage, utils};

mod cli;
use cli::{Cli, Commands};
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
        if let Some(parent) = output.parent()
          && !parent.exists()
        {
          fs::create_dir_all(parent).wrap_err_with(|| {
            format!("Failed to create directory: {}", parent.display())
          })?;
          info!("Created directory: {}", parent.display());
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

      Commands::Export {
        output_dir,
        force,
        templates,
      } => {
        Config::export_templates(output_dir, *force, Some(templates.clone()))
          .wrap_err_with(|| {
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

      // Html command handled below via merge_cli_into_config
      Commands::Html { .. } => {},
    }
  }

  // Create configuration from CLI and/or config file
  let mut config = Config::load(&cli.config_files, &cli.config_overrides)?;

  // Merge CLI command options into config
  merge_cli_into_config(&mut config, &cli);

  // Run the main documentation generation process
  generate_documentation(&mut config)
}

/// Merge CLI command options into the configuration
fn merge_cli_into_config(config: &mut Config, cli: &Cli) {
  // Handle options from the Html subcommand if present
  if let Some(Commands::Html {
    input_dir,
    output_dir,
    jobs,
    template,
    template_dir,
    stylesheet,
    script,
    title,
    footer,
    module_options,
    options_toc_depth,
    manpage_urls,
    generate_search,
    highlight_code,
    revision,
  }) = &cli.command
  {
    if let Some(input_dir) = input_dir {
      config.input_dir = Some(input_dir.clone());
    }
    if let Some(output_dir) = output_dir {
      config.output_dir.clone_from(output_dir);
    }
    config.jobs = jobs.or(config.jobs);
    if let Some(template) = template {
      config.template_path = Some(template.clone());
    }
    if let Some(template_dir) = template_dir {
      config.template_dir = Some(template_dir.clone());
    }
    if !stylesheet.is_empty() {
      config.stylesheet_paths.extend(stylesheet.iter().cloned());
    }
    if !script.is_empty() {
      config.script_paths.extend(script.iter().cloned());
    }
    if let Some(title) = title {
      config.title.clone_from(title);
    }
    if footer.is_some() {
      // Config may not have footer field anymore, skip it
    }
    if let Some(module_options) = module_options {
      config.module_options = Some(module_options.clone());
    }
    if let Some(depth) = options_toc_depth {
      config.options_toc_depth = *depth;
    }
    if let Some(manpage_urls) = manpage_urls {
      config.manpage_urls_path = Some(manpage_urls.clone());
    }
    if *generate_search {
      #[allow(deprecated)]
      {
        config.generate_search = true;
      }
    }
    if *highlight_code {
      config.highlight_code = true;
    }
    if let Some(revision) = revision {
      config.revision.clone_from(revision);
    }
  }
}

/// Main documentation generation process
fn generate_documentation(config: &mut Config) -> Result<()> {
  info!("Starting documentation generation...");

  // Validate required paths after CLI merge
  if let Some(ref input_dir) = config.input_dir
    && !input_dir.exists()
  {
    bail!("Input directory does not exist: {}", input_dir.display());
  }

  // Validate other paths (module_options, template_path, etc.)
  config
    .validate_paths()
    .map_err(|e| color_eyre::eyre::eyre!("Configuration error: {e}"))?;

  // Ensure output directory exists
  fs::create_dir_all(&config.output_dir)?;
  info!("Output directory: {}", config.output_dir.display());

  // Setup thread pool once for all parallel operations
  let thread_count = config.jobs.unwrap_or_else(num_cpus::get);
  rayon::ThreadPoolBuilder::new()
    .num_threads(thread_count)
    .build_global()?;

  // Process markdown files if input directory is provided
  //
  // Create processor once and reuse for all markdown operations
  let processor = if config.input_dir.is_some() {
    Some(utils::create_processor(config, None))
  } else {
    None
  };

  // Process markdown files and collect included files in one pass.
  // As a side effect, this populates config.included_files so that the
  // sidebar navigation can correctly filter out included files.
  let processed_markdown =
    utils::process_markdown_files(config, processor.as_ref())?;

  // Render templates and write HTML files
  for item in &processed_markdown {
    // Skip included files unless they have custom output paths
    if item.is_included {
      continue;
    }

    let rel_path = std::path::Path::new(&item.output_path);
    if rel_path.is_absolute() {
      log::warn!(
        "Output path '{}' is absolute. Normalizing to relative.",
        item.output_path
      );
    }

    // Force rel_path to be relative
    let rel_path = rel_path
      .strip_prefix(std::path::MAIN_SEPARATOR_STR)
      .unwrap_or(rel_path);

    let html = html::template::render(
      config,
      &item.html_content,
      item.title.as_deref().unwrap_or(&config.title),
      &item.headers,
      rel_path,
    )?;

    // Apply postprocessing if requested
    let processed_html =
      if let Some(ref postprocess_config) = config.postprocess {
        utils::postprocess::process_html(&html, postprocess_config)?
      } else {
        html
      };

    let output_path = config.output_dir.join(rel_path);
    if let Some(parent) = output_path.parent() {
      fs::create_dir_all(parent).wrap_err_with(|| {
        format!("Failed to create output directory: {}", parent.display())
      })?;
    }

    fs::write(&output_path, processed_html).wrap_err_with(|| {
      format!("Failed to write output HTML: {}", output_path.display())
    })?;
  }

  // Extract source file paths for later use
  let markdown_files: Vec<PathBuf> = processed_markdown
    .iter()
    .map(|item| item.source_path.clone())
    .collect();

  // Process options if provided
  let options_processed = if let Some(options_path) = &config.module_options {
    info!("Processing options.json from {}", options_path.display());
    html::options::process_options(config, options_path)?;
    true
  } else {
    info!("Module options were not set, skipping options processing");
    false
  };

  // Process nixdoc library documentation if configured
  let nixdoc_processed = html::nixdoc::process_nixdoc(config)?;

  // Check if we need to create a fallback index.html
  let index_path = config.output_dir.join("index.html");
  if !index_path.exists()
    && (options_processed || nixdoc_processed || !markdown_files.is_empty())
  {
    info!("Creating fallback index.html");
    let fallback_content =
      utils::create_fallback_index(config, &markdown_files);
    let html = html::template::render(
      config,
      &fallback_content,
      &config.title,
      &[],
      std::path::Path::new("index.html"),
    )?;
    fs::write(&index_path, html).wrap_err_with(|| {
      format!("Failed to write index.html to {}", index_path.display())
    })?;
  }

  // Generate search index if enabled, regardless of whether there are markdown
  // files. Even if there's only an options.html, search can continue to
  // function.
  if config.is_search_enabled() {
    // Filter out included files - they should not appear as standalone entries
    // in search results. Their content is indexed as part of the parent
    // document.
    let searchable_docs: Vec<html::search::ProcessedDocument> =
      processed_markdown
        .iter()
        .filter(|item| !item.is_included)
        .map(|item| {
          html::search::ProcessedDocument {
            source_path:  item.source_path.clone(),
            html_content: item.html_content.clone(),
            headers:      item.headers.clone(),
            title:        item.title.clone(),
            html_path:    item.output_path.clone(),
          }
        })
        .collect();

    html::search::generate_search_index(config, &searchable_docs)?;
  }

  // Copy assets
  utils::copy_assets(config)?;

  info!(
    "Documentation generated successfully in {}",
    config.output_dir.display()
  );

  Ok(())
}
