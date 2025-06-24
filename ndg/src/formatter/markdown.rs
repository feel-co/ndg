use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use anyhow::Result;
pub use ndg_commonmark::legacy_markdown::Header;

use crate::config::Config;

/// Collect all markdown files from the input directory
pub fn collect_markdown_files(input_dir: &Path) -> Vec<PathBuf> {
    ndg_commonmark::legacy_markdown::collect_markdown_files(input_dir)
}

/// Process a single markdown file
/// Process a single markdown file and return (html_content, headers, title).
/// The caller is responsible for rendering the template and writing the file.
pub fn process_markdown_file(
    config: &Config,
    file_path: &Path,
) -> Result<(String, Vec<Header>, Option<String>)> {
    let manpage_urls = config.manpage_urls_path.as_ref().and_then(|mappings_path| {
        ndg_commonmark::utils::load_manpage_urls(mappings_path.to_str().unwrap()).ok()
    });
    let content = std::fs::read_to_string(file_path)?;
    let (html_content, headers, title) = ndg_commonmark::legacy_markdown::process_markdown(
        &content,
        manpage_urls.as_ref(),
        Some(&config.title),
    );
    Ok((html_content, headers, title))
}

/// Process Markdown content with NixOS/nixpkgs extensions
pub fn process_markdown(
    content: &str,
    manpage_urls: Option<&HashMap<String, String>>,
    config: &Config,
) -> (String, Vec<Header>, Option<String>) {
    ndg_commonmark::legacy_markdown::process_markdown(content, manpage_urls, Some(&config.title))
}
