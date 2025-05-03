use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use lazy_static::lazy_static;
use log::{debug, trace};
use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};
use regex::Regex;
use walkdir::WalkDir;

use crate::config::Config;
use crate::formatter::markup;
use crate::html::{
    highlight, template,
    utils::{escape_html, generate_id},
};

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

// Define regex pattern groups to organize them by functionality
lazy_static! {
    // Role and markup patterns
    pub static ref ROLE_PATTERN: Regex = Regex::new(r"\{([a-z]+)\}`([^`]+)`").unwrap();
    pub static ref MYST_ROLE_RE: Regex = Regex::new(r#"<span class="([a-zA-Z]+)-markup">(.*?)</span>"#).unwrap();

    // Terminal and REPL patterns
    pub static ref COMMAND_PROMPT: Regex = Regex::new(r"`\s*\$\s+([^`]+)`").unwrap();
    pub static ref REPL_PROMPT: Regex = Regex::new(r"`nix-repl>\s*([^`]+)`").unwrap();
    pub static ref PROMPT_RE: Regex = Regex::new(r"<code>\s*\$\s+(.+?)</code>").unwrap();
    pub static ref REPL_RE: Regex = Regex::new(r"<code>nix-repl&gt;\s*(.*?)</code>").unwrap();

    // Heading and anchor patterns
    pub static ref HEADING_ANCHOR: Regex = Regex::new(r"^(#+)?\s*(.+?)(?:\s+\{#([a-zA-Z0-9_-]+)\})\s*$").unwrap();
    pub static ref INLINE_ANCHOR: Regex = Regex::new(r"\[\]\{#([a-zA-Z0-9_-]+)\}").unwrap();
    pub static ref AUTO_EMPTY_LINK_RE: Regex = Regex::new(r"\[\]\((#[a-zA-Z0-9_-]+)\)").unwrap();
    pub static ref AUTO_SECTION_LINK_RE: Regex = Regex::new(r"\[([^\]]+)\]\((#[a-zA-Z0-9_-]+)\)").unwrap();
    pub static ref HTML_EMPTY_LINK_RE: Regex = Regex::new(r#"<a href="(#[a-zA-Z0-9_-]+)"></a>"#).unwrap();
    pub static ref RAW_INLINE_ANCHOR_RE: Regex = Regex::new(r"\[\]\{#([a-zA-Z0-9_-]+)\}").unwrap();

    // List item patterns
    pub static ref LIST_ITEM_WITH_ANCHOR_RE: Regex = Regex::new(r"^(\s*[-*+]|\s*\d+\.)\s+\[\]\{#([a-zA-Z0-9_-]+)\}(.*)$").unwrap();
    pub static ref LIST_ITEM_ID_MARKER_RE: Regex = Regex::new(r"<li><!-- nixos-anchor-id:([a-zA-Z0-9_-]+) -->").unwrap();
    pub static ref LIST_ITEM_ANCHOR_RE: Regex = Regex::new(r"<li>\[\]\{#([a-zA-Z0-9_-]+)\}(.*?)</li>").unwrap();

    // Option reference patterns
    pub static ref OPTION_REF: Regex = Regex::new(r"`([a-zA-Z][\w\.]+(\.[\w]+)+)`").unwrap();
    pub static ref OPTION_RE: Regex = Regex::new(r"<code>([a-zA-Z][\w\.]+(\.[\w]+)+)</code>").unwrap();

    // Block element patterns
    pub static ref ADMONITION_START_RE: Regex = Regex::new(r"^:::\s*\{\.([a-zA-Z]+)(?:\s+#([a-zA-Z0-9_-]+))?\}(.*)$").unwrap();
    pub static ref ADMONITION_END_RE: Regex = Regex::new(r"^(.*?):::$").unwrap();
    pub static ref FIGURE_RE: Regex = Regex::new(r":::\s*\{\.figure(?:\s+#([a-zA-Z0-9_-]+))?\}\s*\n#\s+(.+)\n([\s\S]*?)\s*:::").unwrap();

    // Definition list patterns
    pub static ref DEF_LIST_TERM_RE: Regex = Regex::new(r"^([^:].+)$").unwrap();
    pub static ref DEF_LIST_DEF_RE: Regex = Regex::new(r"^:   (.+)$").unwrap();

    // Manpage patterns
    pub static ref MANPAGE_ROLE_RE: Regex = Regex::new(r"\{manpage\}`([^`]+)`").unwrap();
    pub static ref MANPAGE_MARKUP_RE: Regex = Regex::new(r#"<span class="manpage-markup">([^<]+)</span>"#).unwrap();
    pub static ref MANPAGE_REFERENCE_RE: Regex = Regex::new(r#"<span class="manpage-reference">([^<]+)</span>"#).unwrap();

    // Header patterns for post-processing
    pub static ref HEADER_ID_RE: Regex = Regex::new(r"<h([1-6])>(.*?)\s*<!--\s*anchor:\s*([a-zA-Z0-9_-]+)\s*-->(.*?)</h[1-6]>").unwrap();
    pub static ref HEADER_H1_WITH_ID_RE: Regex = Regex::new(r"<h1>(.*?)\s*\{#([a-zA-Z0-9_-]+)\}(.*?)</h1>").unwrap();
    pub static ref HEADER_H2_WITH_ID_RE: Regex = Regex::new(r"<h2>(.*?)\s*\{#([a-zA-Z0-9_-]+)\}(.*?)</h2>").unwrap();
    pub static ref HEADER_H3_WITH_ID_RE: Regex = Regex::new(r"<h3>(.*?)\s*\{#([a-zA-Z0-9_-]+)\}(.*?)</h3>").unwrap();
    pub static ref HEADER_H4_WITH_ID_RE: Regex = Regex::new(r"<h4>(.*?)\s*\{#([a-zA-Z0-9_-]+)\}(.*?)</h4>").unwrap();
    pub static ref HEADER_H5_WITH_ID_RE: Regex = Regex::new(r"<h5>(.*?)\s*\{#([a-zA-Z0-9_-]+)\}(.*?)</h5>").unwrap();
    pub static ref HEADER_H6_WITH_ID_RE: Regex = Regex::new(r"<h6>(.*?)\s*\{#([a-zA-Z0-9_-]+)\}(.*?)</h6>").unwrap();

    // Other HTML post-processing patterns
    pub static ref BRACKETED_SPAN_RE: Regex = Regex::new(r#"<p><span id="([^"]+)"></span></p>"#).unwrap();
    pub static ref P_TAG_ANCHOR_RE: Regex = Regex::new(r"<p>\[\]\{#([a-zA-Z0-9_-]+)\}(.*?)</p>").unwrap();
    pub static ref EXPLICIT_ANCHOR_RE: Regex = Regex::new(r"^(#+)\s+(.+?)(?:\s+\{#([a-zA-Z0-9_-]+)\})?\s*$").unwrap();
}

/// Collect all markdown files from the input directory
pub fn collect_markdown_files(input_dir: &Path) -> Result<Vec<PathBuf>> {
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

    // Load manpage URL mappings if needed
    let manpage_urls = if content.contains("{manpage}")
        || content.contains("manpage-markup")
        || content.contains("manpage-reference")
    {
        if let Some(mappings_path) = &config.manpage_urls_path {
            match load_manpage_urls(mappings_path) {
                Ok(mappings) => Some(mappings),
                Err(err) => {
                    debug!("Error loading manpage mappings: {err}");
                    None
                }
            }
        } else {
            None
        }
    } else {
        None
    };

    // Process markdown with new unified processor
    let (html_content, headers, title) = process_markdown(&content, manpage_urls.as_ref(), config);

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

/// Process Markdown content with NixOS/nixpkgs extensions
pub fn process_markdown(
    content: &str,
    manpage_urls: Option<&HashMap<String, String>>,
    config: &Config,
) -> (String, Vec<Header>, Option<String>) {
    // 1. Process admonitions, figures, and definition lists
    let preprocessed = preprocess_block_elements(content);

    // 2. Process headers with explicit anchors
    let with_headers = preprocess_headers(&preprocessed);

    // 3. Process inline anchors in list items
    let with_inline_anchors = preprocess_inline_anchors(&with_headers);

    // 4. Process special roles
    let with_roles = process_role_markup(&with_inline_anchors, manpage_urls);

    // 5. Extract headers
    let (headers, title) = extract_headers(&with_roles);

    // 6. Convert standard markdown to HTML
    let html_output = convert_to_html(&with_roles, config);

    // 7. Process remaining special elements in the HTML
    let processed_html = post_process_html(html_output, manpage_urls);

    (processed_html, headers, title)
}

/// Process inline anchors by wrapping them in a span with appropriate id
fn preprocess_inline_anchors(content: &str) -> String {
    // First handle list items with anchors at the beginning
    let mut result = String::with_capacity(content.len() + 100);
    let lines = content.lines().peekable();

    for line in lines {
        if let Some(caps) = LIST_ITEM_WITH_ANCHOR_RE.captures(line) {
            let list_marker = caps.get(1).unwrap().as_str();
            let id = caps.get(2).unwrap().as_str();
            let content = caps.get(3).unwrap().as_str();

            // Use a span tag that will be preserved in the HTML output
            result.push_str(&format!(
                "{list_marker} <span id=\"{id}\" class=\"nixos-anchor\"></span>{content}\n"
            ));
        } else {
            result.push_str(line);
            result.push('\n');
        }
    }

    // Then process any remaining inline anchors
    INLINE_ANCHOR
        .replace_all(&result, |caps: &regex::Captures| {
            let id = &caps[1];
            format!("<span id=\"{id}\" class=\"nixos-anchor\"></span>")
        })
        .to_string()
}

/// Process headers with explicit anchors
fn preprocess_headers(content: &str) -> String {
    let mut lines = vec![];

    for line in content.lines() {
        // Look for headers with anchors but no level sign
        if let Some(caps) = HEADING_ANCHOR.captures(line) {
            let level_signs = caps.get(1).map_or("", |m| m.as_str());
            let text = caps.get(2).unwrap().as_str();
            let id = caps.get(3).unwrap().as_str();

            // If there's no level sign, assume it's an h2
            if level_signs.is_empty() {
                lines.push(format!("## {text} {{#{id}}}"));
            } else {
                // Keep as is if it has level signs
                lines.push(line.to_string());
            }
        } else {
            lines.push(line.to_string());
        }
    }

    lines.join("\n")
}

/// Process roles directly in the markdown before conversion to HTML
fn process_role_markup(content: &str, manpage_urls: Option<&HashMap<String, String>>) -> String {
    let mut result = ROLE_PATTERN
        .replace_all(content, |caps: &regex::Captures| {
            let role_type = &caps[1];
            let role_content = &caps[2];

            match role_type {
                "manpage" => {
                    if let Some(urls) = manpage_urls {
                        if let Some(url) = urls.get(role_content) {
                            format!(
                                "<a href=\"{url}\" class=\"manpage-reference\">{role_content}</a>"
                            )
                        } else {
                            format!("<span class=\"manpage-reference\">{role_content}</span>")
                        }
                    } else {
                        format!("<span class=\"manpage-reference\">{role_content}</span>")
                    }
                }
                "command" => format!("<code class=\"command\">{role_content}</code>"),
                "env" => format!("<code class=\"env-var\">{role_content}</code>"),
                "file" => format!("<code class=\"file-path\">{role_content}</code>"),
                "option" => format!("<code class=\"nixos-option\">{role_content}</code>"),
                "var" => format!("<code class=\"nix-var\">{role_content}</code>"),
                _ => format!("<span class=\"{role_type}-markup\">{role_content}</span>"),
            }
        })
        .into_owned();

    // Process command prompts
    result = COMMAND_PROMPT
        .replace_all(&result, |caps: &regex::Captures| {
            let command = &caps[1];
            format!("<code class=\"terminal\"><span class=\"prompt\">$</span> {command}</code>")
        })
        .into_owned();

    // Process REPL prompts
    result = REPL_PROMPT
        .replace_all(&result, |caps: &regex::Captures| {
            let expr = &caps[1];
            format!(
                "<code class=\"nix-repl\"><span class=\"prompt\">nix-repl&gt;</span> {expr}</code>"
            )
        })
        .into_owned();

    result
}

/// Preprocess block elements like admonitions, figures, and definition lists
fn preprocess_block_elements(content: &str) -> String {
    let mut processed_lines = Vec::new();
    let mut lines = content.lines().peekable();
    let mut in_admonition = false;
    let mut admonition_content = String::new();
    let mut current_adm_type = String::new();
    let mut current_adm_id = None;

    while let Some(line) = lines.next() {
        // Process figures
        if let Some(captures) = FIGURE_RE.captures(line) {
            // Collect figure content
            let id = captures.get(1).map(|m| m.as_str().to_string());
            let title = captures.get(2).unwrap().as_str().to_string();
            let content = captures.get(3).unwrap().as_str();

            // Convert the figure content to HTML
            let processed_content = process_markdown_content(content);

            // Create the figure element
            let id_attr = id
                .as_ref()
                .map_or(String::new(), |id| format!(" id=\"{id}\""));
            processed_lines.push(format!(
                "<figure{id_attr}>\n<figcaption>{title}</figcaption>\n{processed_content}\n</figure>"
            ));

            // Skip ahead to the end of the figure block
            while let Some(next_line) = lines.peek() {
                if next_line.trim() == ":::" {
                    lines.next(); // Consume the closing marker
                    break;
                }
                lines.next(); // Skip the content lines
            }
            continue;
        }

        // Check for definition lists
        if !line.is_empty()
            && !line.starts_with(':')
            && lines.peek().is_some_and(|next| next.starts_with(":   "))
        {
            let term = line;
            let def_line = lines.next().unwrap();
            let definition = &def_line[4..];

            processed_lines.push(format!(
                "<dl>\n<dt>{term}</dt>\n<dd>{definition}</dd>\n</dl>"
            ));
            continue;
        }

        // Process admonitions
        if let Some(caps) = ADMONITION_START_RE.captures(line) {
            // Start of a new admonition
            if in_admonition {
                // Close previous admonition
                let processed_adm = process_admonition(
                    &current_adm_type,
                    current_adm_id.as_deref(),
                    &admonition_content,
                );
                processed_lines.push(processed_adm);
                admonition_content.clear();
            }

            in_admonition = true;
            current_adm_type = caps.get(1).unwrap().as_str().to_string();
            current_adm_id = caps.get(2).map(|m| m.as_str().to_string());
            admonition_content.clear(); // Initialize as empty string

            // Check if it's a single-line admonition
            let content_part = caps.get(3).map_or("", |m| m.as_str());
            if let Some(end_caps) = ADMONITION_END_RE.captures(content_part) {
                // Single-line admonition
                let content = end_caps.get(1).map_or("", |m| m.as_str()).trim();
                if !content.is_empty() {
                    admonition_content.push_str(content);
                    admonition_content.push('\n');
                }

                // Process the admonition
                let processed_adm = process_admonition(
                    &current_adm_type,
                    current_adm_id.as_deref(),
                    &admonition_content,
                );
                processed_lines.push(processed_adm);
                admonition_content.clear();
                in_admonition = false;
            } else if !content_part.trim().is_empty() {
                // Add content to the admonition
                admonition_content.push_str(content_part.trim());
                admonition_content.push('\n');
            }
            continue;
        }

        if in_admonition {
            if let Some(end_caps) = ADMONITION_END_RE.captures(line) {
                // End of the admonition
                let content_before_end = end_caps.get(1).map_or("", |m| m.as_str()).trim();
                if !content_before_end.is_empty() {
                    admonition_content.push_str(content_before_end);
                    admonition_content.push('\n');
                }

                let processed_adm = process_admonition(
                    &current_adm_type,
                    current_adm_id.as_deref(),
                    &admonition_content,
                );
                processed_lines.push(processed_adm);
                admonition_content.clear();
                in_admonition = false;
            } else {
                // Content inside the admonition
                admonition_content.push_str(line);
                admonition_content.push('\n');
            }
            continue;
        }

        // Regular line
        processed_lines.push(line.to_string());
    }

    // Close any open admonition
    if in_admonition {
        let processed_adm = process_admonition(
            &current_adm_type,
            current_adm_id.as_deref(),
            &admonition_content,
        );
        processed_lines.push(processed_adm);
    }

    processed_lines.join("\n")
}

/// Process markdown content for use in admonitions and other block elements
fn process_markdown_content(content: &str) -> String {
    // Set up parser options for processing block content
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_SMART_PUNCTUATION);

    // Create parser for the content
    let parser = Parser::new_ext(content, options);

    // Convert to HTML
    let mut html_output = String::new();
    pulldown_cmark::html::push_html(&mut html_output, parser);

    // Return the HTML
    html_output
}

/// Process an admonition and convert it to HTML
fn process_admonition(admonition_type: &str, id: Option<&str>, content: &str) -> String {
    let id_attr = id.map_or(String::new(), |id| format!(" id=\"{id}\""));
    let title = markup::capitalize_first(admonition_type);

    // Process the content as Markdown to HTML
    // We need to make sure the content has proper line breaks for list items
    let formatted_content = content
        .trim()
        .replace("\n- ", "\n\n- ")
        .replace("\n* ", "\n\n* ")
        .replace("\n+ ", "\n\n+ ");
    let processed_content = process_markdown_content(&formatted_content);

    // Build the admonition HTML
    format!(
        "<div class=\"admonition {admonition_type}\"{id_attr}>\n<p class=\"admonition-title\">{title}</p>\n{processed_content}</div>"
    )
}

/// Convert markdown to HTML using `pulldown_cmark` with syntax highlighting
fn convert_to_html(markdown: &str, config: &Config) -> String {
    // Set up parser options
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_SMART_PUNCTUATION);

    // Create parser
    let parser = Parser::new_ext(markdown, options);

    // Estimate capacity for the output
    let estimated_capacity = markdown.len() * 3;
    let mut html_output = String::with_capacity(estimated_capacity);

    // Buffer for code blocks
    let mut in_code_block = false;
    let mut code_block_lang = String::with_capacity(16);
    let mut code_block_content = String::with_capacity(512);

    // Process each event
    for event in parser {
        match event {
            Event::Start(Tag::CodeBlock(kind)) => {
                in_code_block = true;
                code_block_content.clear();

                // Get language identifier for fenced code blocks
                code_block_lang.clear();
                if let CodeBlockKind::Fenced(lang) = kind {
                    code_block_lang.push_str(lang.as_ref());
                }
            }
            Event::End(TagEnd::CodeBlock) => {
                in_code_block = false;

                // Handle code blocks based on highlighting setting
                if code_block_lang.is_empty() || !config.highlight_code {
                    // Plain code block or highlighting disabled
                    html_output.push_str("<pre><code");

                    // Add language class if available, even when highlighting is disabled
                    if !code_block_lang.is_empty() {
                        html_output.push_str(" class=\"language-");
                        html_output.push_str(&code_block_lang);
                        html_output.push_str("\"");
                    }

                    html_output.push_str(">");
                    escape_html(&code_block_content, &mut html_output);
                    html_output.push_str("</code></pre>");
                } else if let Ok(highlighted) =
                    highlight::highlight_code(&code_block_content, &code_block_lang, config)
                {
                    html_output.push_str(&highlighted);
                } else {
                    // Fallback if highlighting fails
                    html_output.push_str("<pre><code class=\"language-");
                    html_output.push_str(&code_block_lang);
                    html_output.push_str("\">");
                    escape_html(&code_block_content, &mut html_output);
                    html_output.push_str("</code></pre>");
                }
            }
            Event::Text(text) => {
                if in_code_block {
                    code_block_content.push_str(text.as_ref());
                } else {
                    pulldown_cmark::html::push_html(
                        &mut html_output,
                        std::iter::once(Event::Text(text)),
                    );
                }
            }
            _ => {
                if !in_code_block {
                    pulldown_cmark::html::push_html(&mut html_output, std::iter::once(event));
                }
            }
        }
    }

    html_output
}

/// Process HTML to handle any remaining elements
fn post_process_html(html: String, manpage_urls: Option<&HashMap<String, String>>) -> String {
    let mut result = html;

    // Process list item ID markers
    result = process_html_elements(&result, &LIST_ITEM_ID_MARKER_RE, |caps| {
        let id = &caps[1];
        format!("<li><span id=\"{id}\" class=\"nixos-anchor\"></span>")
    });

    // Process manpage roles that were directly in HTML
    result = process_manpage_roles(result, manpage_urls);

    // Process command prompts
    result = process_html_elements(&result, &PROMPT_RE, |caps| {
        format!(
            "<code class=\"terminal\"><span class=\"prompt\">$</span> {} </code>",
            &caps[1]
        )
    });

    // Process REPL prompts
    result = process_html_elements(&result, &REPL_RE, |caps| {
        let content = &caps[1];
        format!(
            "<code class=\"nix-repl\"><span class=\"prompt\">nix-repl&gt;</span> {content}</code>"
        )
    });

    // Process option references
    result = process_html_elements(&result, &OPTION_RE, |caps| {
        let option_path = &caps[1];
        let option_id = format!("option-{}", option_path.replace('.', "-"));
        format!(
            "<a href=\"options.html#{option_id}\" class=\"option-reference\"><code>{option_path}</code></a>"
        )
    });

    // Process header anchors (both explicit and added by comments)
    result = process_html_elements(&result, &HEADER_ID_RE, |caps| {
        let level = &caps[1];
        let prefix = &caps[2];
        let id = &caps[3];
        let suffix = &caps[4];

        format!("<h{level} id=\"{id}\">{prefix}{suffix}<!-- anchor added --></h{level}>")
    });

    // Process any remaining header anchors (with {#id} notation in the header text)
    result = process_headers_with_inline_anchors(result);

    // Process MyST roles
    result = process_html_elements(&result, &MYST_ROLE_RE, |caps| {
        let role_type = &caps[1];
        let content = &caps[2];

        match role_type {
            "command" => format!("<code class=\"command\">{content}</code>"),
            "env" => format!("<code class=\"env-var\">{content}</code>"),
            "file" => format!("<code class=\"file-path\">{content}</code>"),
            "option" => format!("<code class=\"nixos-option\">{content}</code>"),
            "var" => format!("<code class=\"nix-var\">{content}</code>"),
            _ => format!("<span class=\"{role_type}-markup\">{content}</span>"),
        }
    });

    // Process any remaining inline anchors in list items
    result = process_html_elements(&result, &LIST_ITEM_ANCHOR_RE, |caps| {
        let id = &caps[1];
        let content = &caps[2];
        format!("<li><span id=\"{id}\" class=\"nixos-anchor\"></span>{content}</li>")
    });

    // Process inline anchors in paragraphs
    result = process_html_elements(&result, &P_TAG_ANCHOR_RE, |caps| {
        let id = &caps[1];
        let content = &caps[2];
        format!("<p><span id=\"{id}\" class=\"nixos-anchor\"></span>{content}</p>")
    });

    // Process any remaining inline anchors
    result = process_remaining_inline_anchors(result);

    // Process empty auto-links
    result = process_html_elements(&result, &AUTO_EMPTY_LINK_RE, |caps| {
        let anchor = &caps[1];
        let display_text = humanize_anchor_id(anchor);
        format!("<a href=\"{anchor}\">{display_text}</a>")
    });

    // Process empty links in HTML output
    result = process_html_elements(&result, &HTML_EMPTY_LINK_RE, |caps| {
        let anchor = &caps[1];
        let display_text = humanize_anchor_id(anchor);
        format!("<a href=\"{anchor}\">{display_text}</a>")
    });

    result
}

/// Apply a regex transformation to HTML content
fn process_html_elements<F>(html: &str, regex: &Regex, transform: F) -> String
where
    F: Fn(&regex::Captures) -> String,
{
    markup::process_html_elements(html, regex, transform)
}

/// Process headers that have {#id} within the header text
fn process_headers_with_inline_anchors(html: String) -> String {
    let header_types = [
        (&*HEADER_H1_WITH_ID_RE, 1),
        (&*HEADER_H2_WITH_ID_RE, 2),
        (&*HEADER_H3_WITH_ID_RE, 3),
        (&*HEADER_H4_WITH_ID_RE, 4),
        (&*HEADER_H5_WITH_ID_RE, 5),
        (&*HEADER_H6_WITH_ID_RE, 6),
    ];

    let mut result = html;

    for (regex, level) in header_types {
        result = process_html_elements(&result, regex, |caps: &regex::Captures| {
            let prefix = &caps[1];
            let id = &caps[2];
            let suffix = &caps[3];

            format!("<h{level} id=\"{id}\">{prefix}{suffix}</h{level}>")
        });
    }

    result
}

/// Convert an anchor ID to human-readable text
fn humanize_anchor_id(anchor: &str) -> String {
    // Strip the leading #
    let cleaned = anchor.trim_start_matches('#');

    // Remove common prefixes
    let without_prefix = cleaned
        .trim_start_matches("sec-")
        .trim_start_matches("ssec-")
        .trim_start_matches("opt-");

    // Replace separators with spaces
    let spaced = without_prefix.replace(['-', '_'], " ");

    // Capitalize each word
    spaced
        .split_whitespace()
        .map(markup::capitalize_first)
        .collect::<Vec<String>>()
        .join(" ")
}

/// Process manpage roles in the HTML
pub fn process_manpage_roles(
    html: String,
    manpage_urls: Option<&HashMap<String, String>>,
) -> String {
    markup::process_manpage_references(html, manpage_urls, true)
}

/// Process any remaining inline anchor syntax that wasn't caught earlier
fn process_remaining_inline_anchors(html: String) -> String {
    // First process bracketed spans
    let result = BRACKETED_SPAN_RE
        .replace_all(&html, |caps: &regex::Captures| {
            let id = &caps[1];
            format!("<span id=\"{id}\" class=\"nixos-anchor\"></span>")
        })
        .to_string();

    // Then catch any remaining raw inline anchors
    RAW_INLINE_ANCHOR_RE
        .replace_all(&result, |caps: &regex::Captures| {
            let id = &caps[1];
            format!("<span id=\"{id}\" class=\"nixos-anchor\"></span>")
        })
        .to_string()
}

/// Extract headers from markdown content
pub fn extract_headers(content: &str) -> (Vec<Header>, Option<String>) {
    lazy_static! {
        static ref EXPLICIT_ANCHOR_RE: Regex =
            Regex::new(r"^(#+)\s+(.+?)(?:\s+\{#([a-zA-Z0-9_-]+)\})?\s*$").unwrap();
    }

    let mut headers = Vec::new();
    let mut title = None;
    let mut found_headers_with_regex = false;

    // Process line by line for headers with explicit anchors
    for line in content.lines() {
        if let Some(caps) = EXPLICIT_ANCHOR_RE.captures(line) {
            found_headers_with_regex = true;
            let level = caps[1].len() as u8;
            let text = caps[2].trim().to_string();

            // Use explicit anchor if provided, otherwise generate one
            let id = if let Some(explicit_id) = caps.get(3) {
                explicit_id.as_str().to_string()
            } else {
                generate_id(&text)
            };

            // Set title from first h1
            if level == 1 && title.is_none() {
                title = Some(text.clone());
            }

            headers.push(Header { text, level, id });
        }
    }

    // If headers were found with regex, don't do the more expensive parser-based extraction
    if found_headers_with_regex {
        return (headers, title);
    }

    // Otherwise use pulldown_cmark to extract headers
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);

    let parser = Parser::new_ext(content, options);

    let mut current_header_text = String::new();
    let mut in_header = false;
    let mut current_level = 0;

    for event in parser {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                in_header = true;
                current_level = match level {
                    pulldown_cmark::HeadingLevel::H1 => 1,
                    pulldown_cmark::HeadingLevel::H2 => 2,
                    pulldown_cmark::HeadingLevel::H3 => 3,
                    pulldown_cmark::HeadingLevel::H4 => 4,
                    pulldown_cmark::HeadingLevel::H5 => 5,
                    pulldown_cmark::HeadingLevel::H6 => 6,
                };
                current_header_text.clear();
            }
            Event::Text(text) | Event::Code(text) if in_header => {
                current_header_text.push_str(text.as_ref());
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
    // Load manpage URL mappings if needed
    let manpage_urls = if markdown.contains("{manpage}") {
        if let Some(mappings_path) = &config.manpage_urls_path {
            match load_manpage_urls(mappings_path) {
                Ok(mappings) => Some(mappings),
                Err(err) => {
                    debug!("Error loading manpage mappings: {err}");
                    None
                }
            }
        } else {
            None
        }
    } else {
        None
    };

    let (html, _, _) = process_markdown(markdown, manpage_urls.as_ref(), config);
    html
}
