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
use ndg_config::Config;

/// Output entry for processed markdown files
pub struct ProcessedMarkdown {
  pub html_content: String,
  pub headers:      Vec<Header>,
  pub title:        Option<String>,
  pub source_path:  PathBuf,
  pub output_path:  String,
  pub is_included:  bool,
}

/// Collects all included files from markdown documents in the input directory.
///
/// This scans all markdown files and collects the set of files that are
/// included via NDG's markdown processor.
///
/// # Arguments
///
/// * `config` - The loaded configuration for documentation generation.
/// * `processor` - Optional pre-created processor to reuse. If None, creates a
///   new one.
///
/// # Returns
///
/// A mapping of included file paths (relative to input directory) to the parent
/// input file path, which included that file
///
/// # Errors
///
/// Returns an error if any markdown file cannot be read.
#[allow(dead_code, reason = "Kept for backward compatibility")]
pub fn collect_included_files(
  config: &Config,
  processor: Option<&MarkdownProcessor>,
) -> Result<HashMap<PathBuf, PathBuf>> {
  let mut all_included_files: HashMap<PathBuf, PathBuf> = HashMap::new();

  if let Some(ref input_dir) = config.input_dir {
    let files = collect_markdown_files(input_dir);

    // Use provided processor or create a new one
    let base_processor = processor
      .map_or_else(|| create_processor(config, None), std::clone::Clone::clone);

    for file_path in &files {
      let content = fs::read_to_string(file_path).wrap_err_with(|| {
        format!("Failed to read markdown file: {}", file_path.display())
      })?;

      let base_dir = file_path.parent().unwrap_or_else(|| {
        config
          .input_dir
          .as_deref()
          .unwrap_or_else(|| Path::new("."))
      });
      let processor = base_processor.clone().with_base_dir(base_dir);

      let result = processor.render(&content);

      for inc in &result.included_files {
        // Include paths are relative to the file's directory, not input_dir
        let inc_path = base_dir.join(&inc.path);
        if let Ok(inc_rel) = inc_path.strip_prefix(input_dir) {
          let Ok(includer_rel) = file_path.strip_prefix(input_dir) else {
            continue;
          };

          all_included_files
            .entry(inc_rel.to_path_buf())
            .and_modify(|old| {
              // For deterministic output
              if includer_rel < old.as_path() {
                *old = {
                  let this = &includer_rel;
                  this.to_path_buf()
                };
              }
            })
            .or_insert_with(|| includer_rel.to_owned());
        }
      }
    }
  }

  Ok(all_included_files)
}

/// Processes all markdown files in the input directory and returns processed
/// data.
///
/// This function renders all markdown files, handles custom output paths,
/// and returns structured data for each file. Files that are included in other
/// files are marked but still returned (caller decides whether to skip them).
///
/// As a side effect, this function populates `config.included_files` with the
/// mapping of included files to their parent files.
///
/// # Arguments
///
/// * `config` - The loaded configuration for documentation generation.
/// * `processor` - Optional pre-created processor to reuse. If None, creates a
///   new one.
///
/// # Returns
///
/// A vector of `ProcessedMarkdown` entries.
///
/// # Errors
///
/// Returns an error if any file cannot be read or rendered.
///
/// # Panics
///
/// Panics if `config.input_dir` is `None` when processing markdown files.
pub fn process_markdown_files(
  config: &mut Config,
  processor: Option<&MarkdownProcessor>,
) -> Result<Vec<ProcessedMarkdown>> {
  if let Some(ref input_dir) = config.input_dir {
    info!("Input directory: {}", input_dir.display());
    let files = collect_markdown_files(input_dir);
    info!("Found {} markdown files", files.len());

    // Map: output html path -> (html, headers, title, source_path, is_included)
    let mut output_map: HashMap<
      String,
      (String, Vec<Header>, Option<String>, PathBuf, bool),
    > = HashMap::new();

    // Track custom outputs keyed by included file path
    let mut pending_custom_outputs: HashMap<PathBuf, Vec<String>> =
      HashMap::new();

    // Track all included files across all markdown files
    let mut all_included_files: HashMap<PathBuf, PathBuf> = HashMap::new();

    // Use provided processor or create a new one
    let base_processor = processor
      .map_or_else(|| create_processor(config, None), std::clone::Clone::clone);

    // Store render results to avoid rendering twice
    #[allow(clippy::items_after_statements, reason = "Local type alias")]
    type RenderCache = HashMap<PathBuf, ndg_commonmark::MarkdownResult>;
    let mut render_cache: RenderCache = HashMap::new();

    // First pass: render all files once and collect metadata
    for file_path in &files {
      let content = fs::read_to_string(file_path).wrap_err_with(|| {
        format!("Failed to read markdown file: {}", file_path.display())
      })?;

      let base_dir = file_path.parent().unwrap_or_else(|| {
        config
          .input_dir
          .as_deref()
          .unwrap_or_else(|| Path::new("."))
      });
      let processor = base_processor.clone().with_base_dir(base_dir);

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

      // Collect all included files for this markdown file
      for inc in &result.included_files {
        let inc_path = base_dir.join(&inc.path);
        if let Ok(inc_rel) = inc_path.strip_prefix(input_dir) {
          let Ok(includer_rel) = file_path.strip_prefix(input_dir) else {
            continue;
          };

          all_included_files
            .entry(inc_rel.to_path_buf())
            .and_modify(|old| {
              // For deterministic output
              if includer_rel < old.as_path() {
                *old = {
                  let this = &includer_rel;
                  this.to_path_buf()
                };
              }
            })
            .or_insert_with(|| includer_rel.to_owned());
        }
      }

      // Cache the render result
      render_cache.insert(file_path.clone(), result);
    }

    // Second pass: use cached results to build output map
    for file_path in &files {
      // Retrieve cached result instead of rendering again
      #[allow(clippy::expect_used, reason = "File is guaranteed to be cached")]
      let result = render_cache
        .get(file_path)
        .expect("File should have cached result");

      let rel_path = file_path.strip_prefix(input_dir).wrap_err_with(|| {
        format!(
          "Failed to determine relative path for {}",
          file_path.display()
        )
      })?;
      let mut output_path = rel_path.to_path_buf();
      output_path.set_extension("html");
      let output_path_str = output_path.to_string_lossy().to_string();

      // Check if this file is included in another file
      let is_included = all_included_files.contains_key(rel_path);

      // Get any pending custom outputs for this file
      let custom_outputs =
        pending_custom_outputs.remove(rel_path).unwrap_or_default();

      // If this file has custom outputs via html:into-file, write to those
      // locations
      if custom_outputs.is_empty() {
        output_map.insert(
          output_path_str,
          (
            result.html.clone(),
            result.headers.clone(),
            result.title.clone(),
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

    // Populate config.included_files so caller can use it
    config.included_files.clone_from(&all_included_files);

    // Convert output_map to Vec<ProcessedMarkdown>
    let mut results: Vec<ProcessedMarkdown> = output_map
      .into_iter()
      .map(
        |(
          output_path,
          (html_content, headers, title, source_path, is_included),
        )| {
          ProcessedMarkdown {
            html_content,
            headers,
            title,
            source_path,
            output_path,
            is_included,
          }
        },
      )
      .collect();

    // Sort for deterministic output
    results.sort_by(|a, b| a.output_path.cmp(&b.output_path));

    Ok(results)
  } else {
    info!("No input directory provided, skipping markdown processing");
    Ok(Vec::new())
  }
}

/// Creates a markdown processor from the NDG configuration.
///
/// # Arguments
/// * `config` - The loaded configuration for documentation generation.
/// * `valid_options` - Optional set of valid option names for validation.
#[must_use]
#[allow(
  clippy::implicit_hasher,
  reason = "Standard HashSet sufficient for this use case"
)]
pub fn create_processor(
  config: &Config,
  valid_options: Option<HashSet<String>>,
) -> MarkdownProcessor {
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

  if let Some(options) = valid_options {
    builder = builder.valid_options(Some(options));
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
      ndg_commonmark::utils::extract_markdown_title_and_id(&content)
        .map_or(default_title, |(title, _)| title)
    },
    Err(_) => default_title,
  }
}
