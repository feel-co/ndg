mod assets;

use std::{fmt::Write, fs};

use color_eyre::eyre::{self, Context};
use log::info;
use ndg_commonmark::{
  MarkdownOptionsBuilder,
  MarkdownProcessor,
  collect_markdown_files,
};

pub use crate::utils::assets::copy_assets;
use crate::{config::Config, formatter::options, html};

/// Collect all included files from markdown documents
pub fn collect_included_files(
  config: &Config,
) -> eyre::Result<std::collections::HashSet<std::path::PathBuf>> {
  use std::collections::HashSet;
  let mut all_included_files: HashSet<std::path::PathBuf> = HashSet::new();

  if let Some(ref input_dir) = config.input_dir {
    let files = collect_markdown_files(input_dir);

    for file_path in &files {
      let content = std::fs::read_to_string(file_path).wrap_err({
        format!("Failed to read markdown file: {}", file_path.display())
      })?;
      let processor = create_processor_from_config(config);
      let base_dir = file_path.parent().unwrap_or_else(|| {
        config
          .input_dir
          .as_deref()
          .unwrap_or(std::path::Path::new("."))
      });
      let processor = processor.with_base_dir(base_dir);
      let result = processor.render(&content);

      for inc in &result.included_files {
        // Include paths are relative to the file's directory, not input_dir
        let inc_path = base_dir.join(&inc.path);
        if let Ok(inc_rel) = inc_path.strip_prefix(input_dir) {
          all_included_files.insert(inc_rel.to_path_buf());
        }
      }
    }
  }

  Ok(all_included_files)
}

/// Process markdown files
type OutputEntry = (
  String,
  Vec<ndg_commonmark::Header>,
  Option<String>,
  std::path::PathBuf,
  bool,
);

pub fn process_markdown_files(
  config: &Config,
) -> eyre::Result<Vec<std::path::PathBuf>> {
  use std::collections::HashMap;
  if let Some(ref input_dir) = config.input_dir {
    info!("Input directory: {}", input_dir.display());
    let files = collect_markdown_files(input_dir);
    info!("Found {} markdown files", files.len());

    // Map: output html path -> OutputEntry
    let mut output_map: HashMap<String, OutputEntry> = HashMap::new();

    // Track custom outputs keyed by included file path
    let mut pending_custom_outputs: HashMap<std::path::PathBuf, Vec<String>> =
      HashMap::new();

    // First pass: collect all custom outputs for included files
    for file_path in &files {
      let content = std::fs::read_to_string(file_path).wrap_err({
        format!("Failed to read markdown file: {}", file_path.display())
      })?;
      let processor = create_processor_from_config(config);

      let base_dir = file_path.parent().unwrap_or_else(|| {
        config
          .input_dir
          .as_deref()
          .unwrap_or(std::path::Path::new("."))
      });
      let processor = processor.with_base_dir(base_dir);

      let result = processor.render(&content);

      // Track custom outputs for included files
      for inc in &result.included_files {
        if let Some(ref custom_output) = inc.custom_output {
          let inc_path = base_dir.join(&inc.path);
          if let Ok(inc_rel) = inc_path.strip_prefix(input_dir) {
            pending_custom_outputs
              .entry(inc_rel.to_path_buf())
              .or_default()
              .push(custom_output.clone());
          }
        }
      }
    }

    for file_path in &files {
      let content = std::fs::read_to_string(file_path).wrap_err({
        format!("Failed to read markdown file: {}", file_path.display())
      })?;
      let processor = create_processor_from_config(config);

      // Set base directory to file's parent directory for relative includes
      let base_dir = file_path.parent().unwrap_or_else(|| {
        config
          .input_dir
          .as_deref()
          .unwrap_or(std::path::Path::new("."))
      });
      let processor = processor.with_base_dir(base_dir);

      let result = processor.render(&content);

      let input_dir = config.input_dir.as_ref().expect("input_dir required");
      let rel_path = file_path
        .strip_prefix(input_dir)
        .expect("strip_prefix failed");
      let mut output_path = rel_path.to_path_buf();
      output_path.set_extension("html");
      let output_path_str = output_path.to_string_lossy().to_string();

      // Check if this file is included in another file
      let is_included = config.excluded_files.contains(rel_path);

      // Get any pending custom outputs for this file
      let custom_outputs =
        pending_custom_outputs.remove(rel_path).unwrap_or_default();

      // If this file has custom outputs via html:into-file, write to those
      // locations
      if custom_outputs.is_empty() {
        output_map.insert(
          output_path_str,
          (
            result.html,
            result.headers,
            result.title,
            file_path.clone(),
            is_included,
          ),
        );
      } else {
        for custom in custom_outputs {
          output_map.insert(
            custom.clone(),
            (
              result.html.clone(),
              result.headers.clone(),
              result.title.clone(),
              file_path.clone(),
              false,
            ),
          );
        }
      }
    }

    // Write outputs, skipping those marked as included (unless they have custom
    // output)
    use crate::html::template;

    for (out_path, (html_content, headers, title, _src_md, is_included)) in
      &output_map
    {
      if *is_included {
        continue;
      }

      let rel_path = std::path::Path::new(out_path);
      if rel_path.is_absolute() {
        log::warn!(
          "Output path '{out_path}' is absolute (would write to '{}'). \
           Attempting to normalize to relative.",
          rel_path.display()
        );
      }

      // Always force rel_path to be relative, never absolute
      let rel_path = rel_path
        .strip_prefix(std::path::MAIN_SEPARATOR_STR)
        .unwrap_or(rel_path);

      let html = template::render(
        config,
        html_content,
        title.as_deref().unwrap_or(&config.title),
        headers,
        rel_path,
      )?;

      let output_path = config.output_dir.join(rel_path);
      if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent).wrap_err({
          format!("Failed to create output directory: {}", parent.display())
        })?;
      }

      std::fs::write(&output_path, html).wrap_err({
        format!("Failed to write output HTML: {}", output_path.display())
      })?;
    }

    Ok(files)
  } else {
    info!("No input directory provided, skipping markdown processing");
    Ok(Vec::new())
  }
}

/// Process options file
pub fn process_options_file(config: &Config) -> eyre::Result<bool> {
  if let Some(options_path) = &config.module_options {
    info!("Processing options.json from {}", options_path.display());
    options::process_options(config, options_path)?;
    Ok(true)
  } else {
    Ok(false)
  }
}

/// Create a fallback index page
#[must_use]
pub fn create_fallback_index(
  config: &Config,
  markdown_files: &[std::path::PathBuf],
) -> String {
  let mut content = format!(
    "<h1>{}</h1>\n<p>This is a fallback page created by ndg.</p>",
    &config.title
  );

  // Add file listing if we have an input directory
  if let Some(input_dir) = &config.input_dir {
    if !markdown_files.is_empty() {
      let mut file_list = String::with_capacity(markdown_files.len() * 100); // Preallocate based on estimated size
      file_list.push_str("<h2>Available Documents</h2>\n<ul>\n");

      for file_path in markdown_files {
        if let Ok(rel_path) = file_path.strip_prefix(input_dir) {
          let mut html_path = rel_path.to_path_buf();
          html_path.set_extension("html");

          // Get page title from first heading or filename
          let page_title = extract_page_title(file_path, &html_path);

          writeln!(
            file_list,
            "  <li><a href=\"{}\">{}</a></li>",
            html_path.to_string_lossy(),
            page_title
          )
          .unwrap();
        }
      }

      file_list.push_str("</ul>");
      content.push_str(&file_list);
    }
  }

  content
}

/// Create a markdown processor from ndg config
#[must_use]
pub fn create_processor_from_config(config: &Config) -> MarkdownProcessor {
  let mut builder = MarkdownOptionsBuilder::new()
    .gfm(true)
    .highlight_code(config.highlight_code);

  if let Some(mappings_path) = &config.manpage_urls_path {
    builder = builder
      .manpage_urls_path(Some(mappings_path.to_string_lossy().to_string()));
  }

  MarkdownProcessor::new(builder.build())
}

/// Extract the page title from a markdown file
#[must_use]
pub fn extract_page_title(
  file_path: &std::path::Path,
  html_path: &std::path::Path,
) -> String {
  let default_title = html_path
    .file_stem()
    .unwrap_or_default()
    .to_string_lossy()
    .to_string();

  match fs::read_to_string(file_path) {
    Ok(content) => {
      ndg_commonmark::utils::extract_markdown_title(&content)
        .unwrap_or(default_title)
    },
    Err(_) => default_title,
  }
}

/// Ensure that index.html exists in the output directory
pub fn ensure_index(
  config: &Config,
  options_processed: bool,
  markdown_files: &[std::path::PathBuf],
) -> eyre::Result<()> {
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
      let options_content = fs::read(&options_path).wrap_err({
        format!("Failed to read options.html: {}", options_path.display())
      })?;
      fs::write(&index_path, options_content).wrap_err({
        format!("Failed to write index.html: {}", index_path.display())
      })?;
      return Ok(());
    }
  }

  // Create a fallback index page if input directory was provided but no
  // index.md existed
  if config.input_dir.is_some() {
    info!("No index.md found, creating fallback index.html");

    let content = create_fallback_index(config, markdown_files);

    // Create a simple HTML document using our template system
    let headers = vec![ndg_commonmark::Header {
      text:  config.title.clone(),
      level: 1,
      id:    "welcome".to_string(),
    }];

    let html = html::template::render(
      config,
      &content,
      &config.title,
      &headers,
      &std::path::PathBuf::from("index.html"),
    )?;

    fs::write(&index_path, html).wrap_err({
      format!(
        "Failed to write fallback index.html: {}",
        index_path.display()
      )
    })?;
  }

  Ok(())
}
