use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use log::{debug, trace};
use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};
use walkdir::WalkDir;

use crate::config::Config;
use crate::markdown::highlight;
use crate::markdown::postprocess::{extract_headers, process_nixpkgs_elements};
use crate::markdown::preprocess::{preprocess_markdown, FormatType};
use crate::markdown::utils::escape_html;
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
    // Preallocate with a reasonable capacity to avoid reallocations
    let mut files = Vec::with_capacity(100);

    for entry in WalkDir::new(input_dir)
        .follow_links(true)
        .into_iter()
        .filter_map(Result::ok)
    {
        let path = entry.path();
        if path.is_file() && path.extension().is_some_and(|ext| ext == "md") {
            files.push(path.to_owned());
        }
    }

    trace!("Found {} markdown files to process", files.len());
    Ok(files)
}

/// Process a single markdown file
pub fn process_markdown_file(config: &Config, file_path: &Path) -> Result<()> {
    debug!("Processing file: {}", file_path.display());

    // Read markdown
    let content = fs::read_to_string(file_path)
        .with_context(|| format!("Failed to read markdown file: {}", file_path.display()))?;

    // Load manpage URL mappings if configured
    let manpage_urls = if let Some(mappings_path) = &config.manpage_urls_path {
        match load_manpage_urls(mappings_path) {
            Ok(mappings) => Some(mappings),
            Err(err) => {
                debug!("Error loading manpage mappings: {err}");
                None
            }
        }
    } else {
        None
    };

    // Extract headers and title
    let (headers, title) = extract_headers(&content);

    // Get the input directory from config or return an error
    let input_dir = config.input_dir.as_ref().ok_or_else(|| {
        anyhow::anyhow!("Cannot process markdown file: input directory is not configured")
    })?;

    // Determine output path
    let rel_path = file_path.strip_prefix(input_dir).with_context(|| {
        format!(
            "Failed to determine relative path for {}",
            file_path.display()
        )
    })?;

    let mut output_path = config.output_dir.join(rel_path);
    output_path.set_extension("html");

    // Create parent directories if needed
    if let Some(parent) = output_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }
    }

    // Convert to HTML with nixpkgs flavored markdown
    let html_content = markdown_to_html(&content, manpage_urls.as_ref(), config);

    // Render with template
    let html = template::render(
        config,
        &html_content,
        title.as_deref().unwrap_or(&config.title),
        &headers,
        rel_path,
    )?;

    // Write to output file
    fs::write(&output_path, html)
        .with_context(|| format!("Failed to write HTML file: {}", output_path.display()))?;

    Ok(())
}

/// Convert markdown to HTML with `NixOS` flavored extensions and syntax highlighting
fn markdown_to_html(
    markdown: &str,
    manpage_urls: Option<&HashMap<String, String>>,
    config: &Config,
) -> String {
    // Set up parser with extensions
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_SMART_PUNCTUATION);

    // Pre-process markdown for NixOS extensions - use HTML format type
    let processed_markdown = preprocess_markdown(markdown, FormatType::Html);

    // Estimate capacity needed for HTML output
    // I don't have clear metrics, but typically HTML is 2-3x larger than markdown
    // for the same document.
    let estimated_capacity = processed_markdown.len() * 3;
    let mut html_output = String::with_capacity(estimated_capacity);

    // Allocate reasonable capacity for code blocks too
    let mut code_block_lang = String::with_capacity(16);
    let mut code_block_content = String::with_capacity(512);
    let mut in_code_block = false;

    // Create parser
    let parser = Parser::new_ext(&processed_markdown, options);

    for event in parser {
        match event {
            Event::Start(Tag::CodeBlock(kind)) => {
                in_code_block = true;
                code_block_content.clear();

                // Get language identifier if it's a fenced code block
                code_block_lang.clear();
                if let CodeBlockKind::Fenced(lang) = kind {
                    code_block_lang.push_str(lang.as_ref());
                }
            }
            Event::End(TagEnd::CodeBlock) => {
                in_code_block = false;

                // Apply syntax highlighting if we have a language specified
                if code_block_lang.is_empty() {
                    // Regular code block without language specification
                    html_output.push_str("<pre><code>");

                    // Use the standalone escape function
                    escape_html(&code_block_content, &mut html_output);

                    html_output.push_str("</code></pre>");
                } else if let Ok(highlighted_html) =
                    highlight::highlight_code(&code_block_content, &code_block_lang, config)
                {
                    html_output.push_str(&highlighted_html);
                } else {
                    // Fallback to regular HTML if highlighting fails
                    html_output.push_str("<pre><code class=\"language-");
                    html_output.push_str(&code_block_lang);
                    html_output.push_str("\">");

                    // Use the standalone escape function
                    escape_html(&code_block_content, &mut html_output);

                    html_output.push_str("</code></pre>");
                }
            }
            Event::Text(text) => {
                if in_code_block {
                    // Collect code content
                    code_block_content.push_str(text.as_ref());
                } else {
                    // Regular text handling
                    pulldown_cmark::html::push_html(
                        &mut html_output,
                        std::iter::once(Event::Text(text)),
                    );
                }
            }
            _ => {
                // For all other events, if we're not in a code block, push to HTML as usual
                if !in_code_block {
                    pulldown_cmark::html::push_html(&mut html_output, std::iter::once(event));
                }
            }
        }
    }

    // Post-process HTML for NixOS-specific elements
    process_nixpkgs_elements(&html_output, manpage_urls)
}

/// Load manpage URL mappings from JSON file
fn load_manpage_urls(path: &Path) -> Result<HashMap<String, String>> {
    debug!("Loading manpage URL mappings from {}", path.display());

    // Read file content
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read manpage mappings file: {}", path.display()))?;

    // Parse JSON
    let mappings: HashMap<String, String> =
        serde_json::from_str(&content).context("Failed to parse manpage mappings JSON")?;

    debug!("Loaded {} manpage URL mappings", mappings.len());
    Ok(mappings)
}

/// Process markdown string into HTML, useful for option descriptions
pub fn process_markdown_string(markdown: &str, config: &Config) -> String {
    // Pre-process for NixOS extensions - use HTML format type
    let processed_markdown = preprocess_markdown(markdown, FormatType::Html);

    // Convert to HTML with syntax highlighting
    let html = markdown_to_html(&processed_markdown, None, config);

    // Apply nixpkgs-specific processing
    process_nixpkgs_elements(&html, None)
}
