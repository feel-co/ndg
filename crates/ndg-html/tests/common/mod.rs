use std::{fs, path::PathBuf};

use ndg_commonmark::MarkdownProcessor;
use ndg_html::search::ProcessedDocument;

/// Converts markdown files to `ProcessedDocuments` for
/// testing
pub fn files_to_processed_docs(
  files: &[PathBuf],
  input_dir: &std::path::Path,
  processor: &MarkdownProcessor,
) -> Vec<ProcessedDocument> {
  files
    .iter()
    .map(|file_path| {
      let content = fs::read_to_string(file_path).expect("Failed to read file");
      let base_dir = file_path.parent().unwrap_or(input_dir);
      let proc = processor.clone().with_base_dir(base_dir);
      let result = proc.render(&content);

      let rel_path = file_path
        .strip_prefix(input_dir)
        .expect("Failed to get relative path");
      let mut html_path = rel_path.to_path_buf();
      html_path.set_extension("html");

      ProcessedDocument {
        source_path:  file_path.clone(),
        html_content: result.html,
        headers:      result.headers,
        title:        result.title,
        html_path:    html_path.to_string_lossy().to_string(),
      }
    })
    .collect()
}
