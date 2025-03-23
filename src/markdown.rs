use anyhow::{Context, Result};
use log::debug;
use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::config::Config;
use crate::template;

/// Represents a header in the markdown document
#[derive(Debug, Clone, serde::Serialize)]
pub struct Header {
    /// Header text
    pub text: String,

    /// Header level (1-6)
    pub level: u8,

    /// Generated ID/anchor for the header
    pub id: String,
}

/// Collect all markdown files from the input directory
pub fn collect_markdown_files(input_dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    for entry in WalkDir::new(input_dir)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.is_file() && path.extension().map_or(false, |ext| ext == "md") {
            files.push(path.to_owned());
        }
    }

    Ok(files)
}

/// Process a single markdown file
pub fn process_markdown_file(config: &Config, file_path: &Path) -> Result<()> {
    debug!("Processing file: {}", file_path.display());

    // Read markdown
    let content = fs::read_to_string(file_path).context(format!(
        "Failed to read markdown file: {}",
        file_path.display()
    ))?;

    // Extract headers and title
    let (headers, title) = extract_headers(&content);

    // Determine output path
    let rel_path = file_path.strip_prefix(&config.input_dir).context(format!(
        "Failed to determine relative path for {}",
        file_path.display()
    ))?;

    let mut output_path = config.output_dir.join(rel_path);
    output_path.set_extension("html");

    // Create parent directories if needed
    if let Some(parent) = output_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)
                .context(format!("Failed to create directory: {}", parent.display()))?;
        }
    }

    // Convert to HTML
    let html_content = markdown_to_html(&content);

    // Render with template
    let html = template::render(
        config,
        &html_content,
        title.as_deref().unwrap_or(&config.title),
        &headers,
        rel_path,
    )?;

    // Write to output file
    fs::write(&output_path, html).context(format!(
        "Failed to write HTML file: {}",
        output_path.display()
    ))?;

    Ok(())
}

/// Convert markdown to HTML
fn markdown_to_html(markdown: &str) -> String {
    // Set up parser with extensions
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);

    let parser = Parser::new_ext(markdown, options);

    // Convert to HTML
    let mut html_output = String::new();
    pulldown_cmark::html::push_html(&mut html_output, parser);

    html_output
}

/// Extract headers from markdown content
fn extract_headers(content: &str) -> (Vec<Header>, Option<String>) {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);

    let parser = Parser::new_ext(content, options);

    let mut headers = Vec::new();
    let mut title = None;
    let mut current_header_text = String::new();
    let mut in_header = false;
    let mut current_level = 0;

    for event in parser {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                in_header = true;
                current_level = match level {
                    HeadingLevel::H1 => 1,
                    HeadingLevel::H2 => 2,
                    HeadingLevel::H3 => 3,
                    HeadingLevel::H4 => 4,
                    HeadingLevel::H5 => 5,
                    HeadingLevel::H6 => 6,
                };
                current_header_text.clear();
            }
            Event::Text(text) | Event::Code(text) if in_header => {
                current_header_text.push_str(&text);
            }
            Event::End(TagEnd::Heading(_)) => {
                in_header = false;

                // Generate ID
                let id = generate_id(&current_header_text);

                // Set title from first h1
                if current_level == 1 && title.is_none() {
                    title = Some(current_header_text.clone());
                }

                headers.push(Header {
                    text: current_header_text.clone(),
                    level: current_level,
                    id,
                });
            }
            _ => {}
        }
    }

    (headers, title)
}

/// Generate a header ID from header text
fn generate_id(text: &str) -> String {
    // Convert to lowercase, replace non-alphanumeric chars with hyphens
    let id = text
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>();

    // Collapse multiple hyphens and trim
    let id = id
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");

    id
}
