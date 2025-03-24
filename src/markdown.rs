use anyhow::{Context, Result};
use lazy_static::lazy_static;
use log::debug;
use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use regex::Regex;
use std::collections::HashMap;
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
        if path.is_file() && path.extension().is_some_and(|ext| ext == "md") {
            files.push(path.to_owned());
        }
    }

    Ok(files)
}

/// Load manpage URL mappings from JSON file
fn load_manpage_urls(path: &Path) -> Result<HashMap<String, String>> {
    debug!("Loading manpage URL mappings from {}", path.display());
    let content = fs::read_to_string(path).context(format!(
        "Failed to read manpage mappings file: {}",
        path.display()
    ))?;
    let mappings: HashMap<String, String> =
        serde_json::from_str(&content).context("Failed to parse manpage mappings JSON")?;
    debug!("Loaded {} manpage URL mappings", mappings.len());
    Ok(mappings)
}

/// Process a single markdown file
pub fn process_markdown_file(config: &Config, file_path: &Path) -> Result<()> {
    debug!("Processing file: {}", file_path.display());

    // Read markdown
    let content = fs::read_to_string(file_path).context(format!(
        "Failed to read markdown file: {}",
        file_path.display()
    ))?;

    // Load manpage URL mappings if configured
    let manpage_urls = if let Some(mappings_path) = &config.manpage_urls_path {
        match load_manpage_urls(mappings_path) {
            Ok(mappings) => Some(mappings),
            Err(err) => {
                debug!("Error loading manpage mappings: {}", err);
                None
            }
        }
    } else {
        None
    };

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

    // Convert to HTML with nixpkgs flavored markdown
    let html_content = markdown_to_html(&content, manpage_urls.as_ref());

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

/// Convert markdown to HTML with NixOS flavored extensions
fn markdown_to_html(markdown: &str, manpage_urls: Option<&HashMap<String, String>>) -> String {
    // Set up parser with extensions
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);

    // Pre-process NixOS special markup patterns
    let processed_markdown = preprocess_special_markup(markdown);

    // Regular parser for standard markdown
    let parser = Parser::new_ext(&processed_markdown, options);

    // Convert to HTML
    let mut html_output = String::new();
    pulldown_cmark::html::push_html(&mut html_output, parser);

    // Post-process HTML for NixOS-specific elements
    process_nixpkgs_elements(&html_output, manpage_urls)
}

/// Preprocess special markup patterns like {file}`path` in markdown
fn preprocess_special_markup(markdown: &str) -> String {
    lazy_static! {
        // Match inline NixOS-specific markup patterns
        static ref COMMAND_MARKUP: Regex = Regex::new(r"\{command\}`([^`]+)`").unwrap();
        static ref ENV_MARKUP: Regex = Regex::new(r"\{env\}`([^`]+)`").unwrap();
        static ref FILE_MARKUP: Regex = Regex::new(r"\{file\}`([^`]+)`").unwrap();
        static ref OPTION_MARKUP: Regex = Regex::new(r"\{option\}`([^`]+)`").unwrap();
        static ref VAR_MARKUP: Regex = Regex::new(r"\{var\}`([^`]+)`").unwrap();
        static ref SPAN_MARKUP: Regex = Regex::new(r#"<span class="(\w+)-markup">([^<]+)</span>"#).unwrap();
    }

    // First convert any existing span markup to our special format
    let mut result = SPAN_MARKUP
        .replace_all(markdown, |caps: &regex::Captures| {
            let markup_type = &caps[1];
            let content = &caps[2];
            format!("{{{}}}`{}`", markup_type, content)
        })
        .to_string();

    // Replace {command} markup
    result = COMMAND_MARKUP
        .replace_all(&result, |caps: &regex::Captures| {
            format!("<span class=\"command-markup\">{}</span>", &caps[1])
        })
        .to_string();

    // Replace {env} markup
    result = ENV_MARKUP
        .replace_all(&result, |caps: &regex::Captures| {
            format!("<span class=\"env-markup\">{}</span>", &caps[1])
        })
        .to_string();

    // Replace {file} markup
    result = FILE_MARKUP
        .replace_all(&result, |caps: &regex::Captures| {
            format!("<span class=\"file-markup\">{}</span>", &caps[1])
        })
        .to_string();

    // Replace {option} markup
    result = OPTION_MARKUP
        .replace_all(&result, |caps: &regex::Captures| {
            format!("<span class=\"option-markup\">{}</span>", &caps[1])
        })
        .to_string();

    // Replace {var} markup
    result = VAR_MARKUP
        .replace_all(&result, |caps: &regex::Captures| {
            format!("<span class=\"var-markup\">{}</span>", &caps[1])
        })
        .to_string();

    result
}

/// Process NixOS-specific elements in the generated HTML
fn process_nixpkgs_elements(html: &str, manpage_urls: Option<&HashMap<String, String>>) -> String {
    lazy_static! {
        // Match command prompts
        static ref PROMPT_RE: Regex = Regex::new(r"<code>\s*\$\s+(.+?)</code>").unwrap();
        // Match manpage references
        static ref MANPAGE_RE: Regex = Regex::new(r"(\w+)\((\d+)\)").unwrap();
        // Match option references like `option.path`
        static ref OPTION_RE: Regex = Regex::new(r"<code>([a-zA-Z][\w\.]+(\.[\w]+)+)</code>").unwrap();
    }

    // Process command prompts
    let with_prompts = PROMPT_RE.replace_all(html, |caps: &regex::Captures| {
        format!(
            "<code class=\"command\"><span class=\"prompt\">$</span> {}</code>",
            &caps[1]
        )
    });

    // Process manpage references - only if URL mappings are provided
    let with_manpages = if let Some(urls) = manpage_urls {
        MANPAGE_RE
            .replace_all(&with_prompts, |caps: &regex::Captures| {
                let name = &caps[1];
                let section = &caps[2];
                let manpage_ref = format!("{}({})", name, section);
                if let Some(url) = urls.get(&manpage_ref) {
                    format!("<a href=\"{}\" class=\"manpage\">{}</a>", url, &manpage_ref)
                } else {
                    // No URL mapping found, leave as plain text
                    manpage_ref
                }
            })
            .to_string()
    } else {
        // No manpage URL mappings provided, leave as plain text
        with_prompts.to_string()
    };

    // Process option references
    let with_options = OPTION_RE.replace_all(&with_manpages, |caps: &regex::Captures| {
        let option_path = &caps[1];
        let option_id = format!("option-{}", option_path.replace(".", "-"));
        format!(
            "<a href=\"options.html#{}\" class=\"option-reference\"><code>{}</code></a>",
            option_id, option_path
        )
    });

    with_options.to_string()
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

/// Process markdown string into HTML, useful for option descriptions
pub fn process_markdown_string(markdown: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);

    // Pre-process NixOS special markup patterns
    let processed_markdown = preprocess_special_markup(markdown);

    let parser = Parser::new_ext(&processed_markdown, options);
    let mut html = String::new();
    pulldown_cmark::html::push_html(&mut html, parser);

    // Apply nixpkgs-specific processing - no manpage URLs for option descriptions
    process_nixpkgs_elements(&html, None)
}
