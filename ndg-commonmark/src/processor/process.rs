//! Main processing functions for Markdown content.
use std::{
  fs,
  io::Error,
  path::{Path, PathBuf},
};

use log::error;

use super::types::{MarkdownOptions, MarkdownProcessor, TabStyle};
use crate::types::MarkdownResult;

/// Process markdown content with error recovery.
///
/// Attempts to process the markdown content and falls back to
/// safe alternatives if processing fails at any stage.
///
/// # Arguments
///
/// * `processor` - The configured markdown processor
/// * `content` - The raw markdown content to process
///
/// # Returns
///
/// A `MarkdownResult` with processed HTML, headers, and title
#[must_use]
pub fn process_with_recovery(
  processor: &MarkdownProcessor,
  content: &str,
) -> MarkdownResult {
  match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
    processor.render(content)
  })) {
    Ok(result) => result,
    Err(panic_err) => {
      error!("Panic during markdown processing: {panic_err:?}");
      MarkdownResult {
        html: "<div class=\"error\">Critical error processing markdown \
               content</div>"
          .to_string(),

        headers:        Vec::new(),
        title:          None,
        included_files: Vec::new(),
      }
    },
  }
}

/// Safely process markup content with error recovery.
///
/// Provides a safe wrapper around markup processing operations
/// that may fail, and ensures that partial or fallback content is returned
/// rather than complete failure.
///
/// # Arguments
///
/// * `content` - The content to process
/// * `processor_fn` - The processing function to apply
/// * `fallback` - Fallback content to use if processing fails
///
/// # Returns
///
/// The processed content or fallback on error
pub fn process_safe<F>(content: &str, processor_fn: F, fallback: &str) -> String
where
  F: FnOnce(&str) -> String,
{
  // Avoid processing empty strings
  if content.is_empty() {
    return String::new();
  }

  // Catch any potential panics caused by malformed input or processing errors
  let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
    processor_fn(content)
  }));

  match result {
    Ok(processed_text) => processed_text,
    Err(e) => {
      // Log the error but allow the program to continue
      if let Some(error_msg) = e.downcast_ref::<String>() {
        error!("Error processing markup: {error_msg}");
      } else if let Some(error_msg) = e.downcast_ref::<&str>() {
        error!("Error processing markup: {error_msg}");
      } else {
        error!("Unknown error occurred while processing markup");
      }

      // Return the original text or default value to prevent breaking the
      // entire document
      if fallback.is_empty() {
        content.to_string()
      } else {
        fallback.to_string()
      }
    },
  }
}

/// Process a batch of markdown files with consistent error handling.
///
/// This function processes multiple markdown files using the same processor
/// configuration, collecting results and handling errors gracefully.
///
/// # Arguments
/// * `processor` - The configured markdown processor
/// * `files` - Iterator of file paths to process
/// * `read_file_fn` - Function to read file content from path
///
/// # Returns
/// Vector of tuples containing (`file_path`, `processing_result`)
pub fn process_batch<I, F>(
  processor: &MarkdownProcessor,
  files: I,
  read_file_fn: F,
) -> Vec<(String, Result<MarkdownResult, String>)>
where
  I: Iterator<Item = PathBuf>,
  F: Fn(&Path) -> Result<String, Error>,
{
  files
    .map(|path| {
      let path_str = path.display().to_string();
      let result = match read_file_fn(&path) {
        Ok(content) => Ok(process_with_recovery(processor, &content)),
        Err(e) => Err(format!("Failed to read file: {e}")),
      };
      (path_str, result)
    })
    .collect()
}

/// Create a processor with sensible defaults for library usage.
///
/// Provides a convenient way to create a processor with
/// commonly used settings for different use cases.
///
/// # Arguments
///
/// * `preset` - The preset configuration to use
///
/// # Returns
///
/// A configured `MarkdownProcessor`
#[must_use]
pub fn create_processor(preset: ProcessorPreset) -> MarkdownProcessor {
  let options = match preset {
    ProcessorPreset::Basic => {
      MarkdownOptions {
        gfm:               true,
        nixpkgs:           false,
        highlight_code:    true,
        highlight_theme:   None,
        manpage_urls_path: None,
        auto_link_options: true,
        tab_style:         TabStyle::None,
        valid_options:     None,
      }
    },
    ProcessorPreset::Ndg => {
      MarkdownOptions {
        gfm:               true,
        nixpkgs:           false,
        highlight_code:    true,
        highlight_theme:   Some("github".to_string()),
        manpage_urls_path: None,
        auto_link_options: true,
        tab_style:         TabStyle::None,
        valid_options:     None,
      }
    },
    ProcessorPreset::Nixpkgs => {
      MarkdownOptions {
        gfm:               true,
        nixpkgs:           true,
        highlight_code:    true,
        highlight_theme:   Some("github".to_string()),
        manpage_urls_path: None,
        auto_link_options: true,
        tab_style:         TabStyle::None,
        valid_options:     None,
      }
    },
  };

  MarkdownProcessor::new(options)
}

/// Preset configurations for common use cases. In some cases those presets will
/// require certain feature flags to be enabled.
#[derive(Debug, Clone, Copy)]
pub enum ProcessorPreset {
  /// Markdown processing with only Github Flavored Markdown (GFM) support
  Basic,
  /// Markdown processing with only Nixpkgs-flavored `CommonMark` support
  Nixpkgs,
  /// Enhanced Markdown processing with support for GFM and Nixpkgs-flavored
  /// `CommonMark` support
  Ndg,
}

/// Process markdown content from a string with error recovery.
///
/// This is a convenience function that combines processor creation and
/// content processing in a single call.
///
/// # Arguments
/// * `content` - The markdown content to process
/// * `preset` - The processor preset to use
///
/// # Returns
/// A `MarkdownResult` with processed content
#[must_use]
pub fn process_markdown_string(
  content: &str,
  preset: ProcessorPreset,
) -> MarkdownResult {
  let processor = create_processor(preset);
  process_with_recovery(&processor, content)
}

/// Process markdown content from a file with error recovery.
///
/// This function reads a markdown file and processes it with the specified
/// configuration, handling both file I/O and processing errors.
///
/// # Arguments
/// * `file_path` - Path to the markdown file
/// * `preset` - The processor preset to use
///
/// # Returns
/// A `Result` containing the `MarkdownResult` or an error message
///
/// # Errors
///
/// Returns an error if the file cannot be read.
pub fn process_markdown_file(
  file_path: &Path,
  preset: ProcessorPreset,
) -> Result<MarkdownResult, String> {
  let content = fs::read_to_string(file_path).map_err(|e| {
    format!("Failed to read file {}: {}", file_path.display(), e)
  })?;

  let base_dir = file_path.parent().unwrap_or_else(|| Path::new("."));
  let processor = create_processor(preset).with_base_dir(base_dir);
  Ok(process_with_recovery(&processor, &content))
}

/// Process markdown content from a file with custom base directory.
///
/// This function reads a markdown file and processes it with the specified
/// configuration and base directory for resolving includes.
///
/// # Arguments
///
/// * `file_path` - Path to the markdown file
/// * `base_dir` - Base directory for resolving relative includes
/// * `preset` - The processor preset to use
///
/// # Returns
///
/// A `Result` containing the `MarkdownResult` or an error message
///
/// # Errors
///
/// Returns an error if the file cannot be read.
pub fn process_markdown_file_with_basedir(
  file_path: &Path,
  base_dir: &Path,
  preset: ProcessorPreset,
) -> Result<MarkdownResult, String> {
  let content = fs::read_to_string(file_path).map_err(|e| {
    format!("Failed to read file {}: {}", file_path.display(), e)
  })?;

  let processor = create_processor(preset).with_base_dir(base_dir);
  Ok(process_with_recovery(&processor, &content))
}

#[cfg(test)]
mod tests {
  use std::path::Path;

  use super::*;

  #[test]
  fn test_safely_process_markup_success() {
    let content = "test content";
    let result =
      process_safe(content, |s| format!("processed: {s}"), "fallback");
    assert_eq!(result, "processed: test content");
  }

  #[test]
  #[allow(clippy::panic)]
  fn test_safely_process_markup_fallback() {
    let content = "test content";
    let result = process_safe(content, |_| panic!("test panic"), "fallback");
    assert_eq!(result, "fallback");
  }

  #[test]
  fn test_process_markdown_string() {
    let content = "# Test Header\n\nSome content.";
    let result = process_markdown_string(content, ProcessorPreset::Basic);

    assert!(result.html.contains("<h1"));
    assert!(result.html.contains("Test Header"));
    assert_eq!(result.title, Some("Test Header".to_string()));
    assert_eq!(result.headers.len(), 1);
  }

  #[test]
  fn test_create_processor_presets() {
    let basic = create_processor(ProcessorPreset::Basic);
    assert!(basic.options.gfm);
    assert!(!basic.options.nixpkgs);
    assert!(basic.options.highlight_code);

    let enhanced = create_processor(ProcessorPreset::Ndg);
    assert!(enhanced.options.gfm);
    assert!(!enhanced.options.nixpkgs);
    assert!(enhanced.options.highlight_code);

    let nixpkgs = create_processor(ProcessorPreset::Nixpkgs);
    assert!(nixpkgs.options.gfm);
    assert!(nixpkgs.options.nixpkgs);
    assert!(nixpkgs.options.highlight_code);
  }

  #[test]
  #[allow(clippy::panic)]
  fn test_process_batch() {
    let processor = create_processor(ProcessorPreset::Basic);
    let paths = vec![Path::new("test1.md"), Path::new("test2.md")];

    let read_fn = |path: &Path| -> Result<String, std::io::Error> {
      match path.file_name().and_then(|n| n.to_str()) {
        Some("test1.md") => Ok("# Test 1".to_string()),
        Some("test2.md") => Ok("# Test 2".to_string()),
        _ => {
          Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "File not found",
          ))
        },
      }
    };

    let results = process_batch(
      &processor,
      paths.into_iter().map(std::path::Path::to_path_buf),
      read_fn,
    );
    assert_eq!(results.len(), 2);

    for (path, result) in results {
      match result {
        Ok(markdown_result) => {
          assert!(markdown_result.html.contains("<h1"));
          if path.contains("test1") {
            assert!(markdown_result.html.contains("Test 1"));
          } else {
            assert!(markdown_result.html.contains("Test 2"));
          }
        },
        Err(e) => panic!("Unexpected error for path {path}: {e}"),
      }
    }
  }
}
