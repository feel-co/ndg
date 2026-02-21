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
  #[cfg(feature = "nixdoc")]
  let nixdoc_processed = {
    if config.nixdoc_inputs.is_empty() {
      info!("No nixdoc inputs configured, skipping lib generation");
      false
    } else {
      info!("Processing nixdoc inputs...");
      let mut all_entries: Vec<ndg_nixdoc::NixDocEntry> = Vec::new();
      for input in &config.nixdoc_inputs {
        if input.is_dir() {
          match ndg_nixdoc::extract_from_dir(input) {
            Ok(entries) => all_entries.extend(entries),
            Err(e) => {
              log::warn!(
                "Failed to extract nixdoc from {}: {e}",
                input.display()
              )
            },
          }
        } else {
          match ndg_nixdoc::extract_from_file(input) {
            Ok(entries) => all_entries.extend(entries),
            Err(e) => {
              log::warn!(
                "Failed to extract nixdoc from {}: {e}",
                input.display()
              )
            },
          }
        }
      }
      if all_entries.is_empty() {
        log::warn!("No nixdoc entries found in configured inputs");
        false
      } else {
        let entries_html = generate_lib_entries_html(&all_entries);
        let toc_html = generate_lib_toc_html(&all_entries);
        let lib_html =
          html::template::render_lib(config, &entries_html, &toc_html)
            .wrap_err("Failed to render lib page")?;
        let lib_path = config.output_dir.join("lib.html");
        fs::write(&lib_path, lib_html).wrap_err_with(|| {
          format!("Failed to write lib.html to {}", lib_path.display())
        })?;
        info!("Generated lib.html with {} entries", all_entries.len());
        true
      }
    }
  };
  #[cfg(not(feature = "nixdoc"))]
  let nixdoc_processed = false;

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

/// Generate HTML content for all nixdoc library entries.
///
/// Each entry is rendered as a `<section>` block containing the attribute
/// path, type signature, description, arguments, and examples.
#[cfg(feature = "nixdoc")]
fn generate_lib_entries_html(entries: &[ndg_nixdoc::NixDocEntry]) -> String {
  use std::fmt::Write;

  let mut html = String::new();
  for entry in entries {
    let attr_path = entry.attr_path.join(".");
    let id_raw = attr_path.replace('.', "-");
    let id = html_escape::encode_safe(&id_raw);
    let attr_path_escaped = html_escape::encode_text(&attr_path);
    let _ = write!(html, "<section class=\"lib-entry\" id=\"{id}\">");
    let _ = write!(
      html,
      "<h2 class=\"lib-entry-name\"><a \
       href=\"#{id}\">{attr_path_escaped}</a></h2>"
    );

    if let Some(doc) = &entry.doc {
      if let Some(type_sig) = &doc.type_sig {
        let escaped = html_escape::encode_text(type_sig);
        let _ = write!(
          html,
          "<div class=\"lib-entry-type\"><code>{escaped}</code></div>"
        );
      }

      let description = html_escape::encode_text(&doc.description);
      let _ = write!(
        html,
        "<div class=\"lib-entry-description\">{description}</div>"
      );

      if !doc.arguments.is_empty() {
        let _ = write!(
          html,
          "<div class=\"lib-entry-arguments\"><h3>Arguments</h3><dl>"
        );
        for arg in &doc.arguments {
          let name = html_escape::encode_text(&arg.name);
          let arg_desc = html_escape::encode_text(&arg.description);
          let _ =
            write!(html, "<dt><code>{name}</code></dt><dd>{arg_desc}</dd>",);
        }
        let _ = write!(html, "</dl></div>");
      }

      if !doc.examples.is_empty() {
        let _ =
          write!(html, "<div class=\"lib-entry-examples\"><h3>Examples</h3>");
        for ex in &doc.examples {
          let lang =
            html_escape::encode_safe(ex.language.as_deref().unwrap_or("nix"));
          let code = html_escape::encode_text(&ex.code);
          let _ = write!(
            html,
            "<pre><code class=\"language-{lang}\">{code}</code></pre>"
          );
        }
        let _ = write!(html, "</div>");
      }
    }

    let file_str = entry.location.file.display().to_string();
    let file_display = html_escape::encode_text(&file_str);
    let line = entry.location.line;
    let _ = write!(
      html,
      "<div class=\"lib-entry-location\">Defined in \
       <code>{file_display}</code>, line {line}</div>"
    );
    let _ = write!(html, "</section>");
  }
  html
}

/// Generate a table-of-contents HTML list for all nixdoc library entries.
#[cfg(feature = "nixdoc")]
fn generate_lib_toc_html(entries: &[ndg_nixdoc::NixDocEntry]) -> String {
  use std::fmt::Write;

  let mut html = String::new();
  for entry in entries {
    let attr_path = entry.attr_path.join(".");
    let id_raw = attr_path.replace('.', "-");
    let id = html_escape::encode_safe(&id_raw);
    let attr_path_escaped = html_escape::encode_text(&attr_path);
    let _ =
      writeln!(html, "<li><a href=\"#{id}\">{attr_path_escaped}</a></li>");
  }
  html
}
