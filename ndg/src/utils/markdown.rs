use std::{
  collections::{HashMap, HashSet},
  fs,
  path::{Path, PathBuf},
};

use color_eyre::eyre::{Context, Result};
use log::info;
use ndg_commonmark::{
  Header,
  MarkdownOptionsBuilder,
  MarkdownProcessor,
  collect_markdown_files,
  processor::types::TabStyle,
};

use crate::{config::Config, html::template};

/// Output entry for processed markdown files.
type OutputEntry = (String, Vec<Header>, Option<String>, PathBuf, bool);

/// Collects all included files from markdown documents in the input directory.
///
/// This scans all markdown files and collects the set of files that are
/// included via NDG's markdown processor.
///
/// # Arguments
///
/// * `config` - The loaded configuration for documentation generation.
///
/// # Returns
///
/// A set of relative paths (to the input directory) of all included files.
///
/// # Errors
///
/// Returns an error if any markdown file cannot be read.
pub fn collect_included_files(config: &Config) -> Result<HashSet<PathBuf>> {
  let mut all_included_files: HashSet<PathBuf> = HashSet::new();

  if let Some(ref input_dir) = config.input_dir {
    let files = collect_markdown_files(input_dir);

    for file_path in &files {
      let content = fs::read_to_string(file_path).wrap_err_with(|| {
        format!("Failed to read markdown file: {}", file_path.display())
      })?;
      let processor = create_processor_from_config(config);
      let base_dir = file_path.parent().unwrap_or_else(|| {
        config.input_dir.as_deref().unwrap_or(Path::new("."))
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

/// Processes all markdown files in the input directory and writes HTML output.
///
/// This function renders all markdown files, handles custom output paths,
/// and writes the resulting HTML files to the output directory. Files that are
/// included in other files are skipped unless they have custom output paths.
///
/// # Arguments
///
/// * `config` - The loaded configuration for documentation generation.
///
/// # Returns,
///
/// A vector of all processed markdown file paths.
///
/// # Errors
///
/// Returns an error if any file cannot be read, rendered, or written.
/// Processes all markdown files in the input directory and writes HTML output.
///
/// This function renders all markdown files, handles custom output paths,
/// and writes the resulting HTML files to the output directory. Files that are
/// included in other files are skipped unless they have custom output paths.
///
/// # Errors
///
/// Returns an error if any file cannot be read, rendered, or written.
///
/// # Panics
///
/// Panics if `config.input_dir` is `None` when processing markdown files.
pub fn process_markdown_files(config: &Config) -> Result<Vec<PathBuf>> {
  if let Some(ref input_dir) = config.input_dir {
    info!("Input directory: {}", input_dir.display());
    let files = collect_markdown_files(input_dir);
    info!("Found {} markdown files", files.len());

    // Map: output html path -> OutputEntry
    let mut output_map: HashMap<String, OutputEntry> = HashMap::new();

    // Track custom outputs keyed by included file path
    let mut pending_custom_outputs: HashMap<PathBuf, Vec<String>> =
      HashMap::new();

    // First pass: collect all custom outputs for included files
    for file_path in &files {
      let content = fs::read_to_string(file_path).wrap_err_with(|| {
        format!("Failed to read markdown file: {}", file_path.display())
      })?;
      let processor = create_processor_from_config(config);

      let base_dir = file_path.parent().unwrap_or_else(|| {
        config.input_dir.as_deref().unwrap_or(Path::new("."))
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
      let content = fs::read_to_string(file_path).wrap_err_with(|| {
        format!("Failed to read markdown file: {}", file_path.display())
      })?;
      let processor = create_processor_from_config(config);

      // Set base directory to file's parent directory for relative includes
      let base_dir = file_path.parent().unwrap_or_else(|| {
        config.input_dir.as_deref().unwrap_or(Path::new("."))
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
    for (out_path, (html_content, headers, title, _src_md, is_included)) in
      &output_map
    {
      if *is_included {
        continue;
      }

      let rel_path = Path::new(out_path);
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
        fs::create_dir_all(parent).wrap_err_with(|| {
          format!("Failed to create output directory: {}", parent.display())
        })?;
      }

      fs::write(&output_path, html).wrap_err_with(|| {
        format!("Failed to write output HTML: {}", output_path.display())
      })?;
    }

    Ok(files)
  } else {
    info!("No input directory provided, skipping markdown processing");
    Ok(Vec::new())
  }
}

/// Creates a markdown processor from the NDG configuration.
///
/// This configures the processor with GFM, code highlighting, and manpage URL
/// mappings as specified in the configuration.
///
/// # Arguments
/// * `config` - The loaded configuration for documentation generation.
///
/// # Returns
/// A configured `MarkdownProcessor`.
#[must_use]
pub fn create_processor_from_config(config: &Config) -> MarkdownProcessor {
  let tab_style = match config.tab_style.as_str() {
    "warn" => TabStyle::Warn,
    "normalize" => TabStyle::Normalize,
    _ => TabStyle::None,
  };

  let mut builder = MarkdownOptionsBuilder::new()
    .gfm(true)
    .highlight_code(config.highlight_code)
    .tab_style(tab_style);

  if let Some(mappings_path) = &config.manpage_urls_path {
    builder = builder
      .manpage_urls_path(Some(mappings_path.to_string_lossy().to_string()));
  }

  MarkdownProcessor::new(builder.build())
}

/// Extracts the page title from a markdown file.
///
/// This attempts to extract the first heading as the title, falling back to the
/// file name if no heading is found or the file cannot be read.
///
/// # Arguments
///
/// * `file_path` - The path to the markdown file.
/// * `html_path` - The corresponding HTML output path (used for fallback).
///
/// # Returns
///
/// The extracted title or the file stem as a fallback.
#[must_use]
pub fn extract_page_title(file_path: &Path, html_path: &Path) -> String {
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
