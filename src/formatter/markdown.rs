use std::{
    collections::HashMap,
    fmt::Write,
    fs,
    iter::Peekable,
    path::{Path, PathBuf},
    str::Lines,
    sync::LazyLock,
};

use anyhow::{Context, Result};
use log::{debug, error, trace};
use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};
use regex::Regex;
use walkdir::WalkDir;

use crate::{
    config::Config,
    formatter::markup,
    html::{
        highlight, template,
        utils::{escape_html, generate_id},
    },
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
// Role and markup patterns
pub static ROLE_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\{([a-z]+)\}`([^`]+)`").unwrap_or_else(|e| {
        error!("Failed to compile ROLE_PATTERN regex: {e}");
        markup::never_matching_regex()
    })
});

pub static MYST_ROLE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"<span class="([a-zA-Z]+)-markup">(.*?)</span>"#).unwrap_or_else(|e| {
        error!("Failed to compile MYST_ROLE_RE regex: {e}");
        markup::never_matching_regex()
    })
});

// Terminal and REPL patterns
pub static COMMAND_PROMPT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"`\s*\$\s+([^`]+)`").unwrap_or_else(|e| {
        error!("Failed to compile COMMAND_PROMPT regex: {e}");
        markup::never_matching_regex()
    })
});

pub static REPL_PROMPT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"`nix-repl>\s*([^`]+)`").unwrap_or_else(|e| {
        error!("Failed to compile REPL_PROMPT regex: {e}");
        markup::never_matching_regex()
    })
});

static PROMPT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"<code>\s*\$\s+(.+?)</code>").unwrap_or_else(|e| {
        error!("Failed to compile PROMPT_RE regex: {e}");
        markup::never_matching_regex()
    })
});

static REPL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"<code>nix-repl&gt;\s*(.*?)</code>").unwrap_or_else(|e| {
        error!("Failed to compile REPL_RE regex: {e}");
        markup::never_matching_regex()
    })
});

// Heading and anchor patterns
pub static HEADING_ANCHOR: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(#+)?\s*(.+?)(?:\s+\{#([a-zA-Z0-9_-]+)\})\s*$").unwrap_or_else(|e| {
        error!("Failed to compile HEADING_ANCHOR regex: {e}");
        markup::never_matching_regex()
    })
});
pub static INLINE_ANCHOR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[\]\{#([a-zA-Z0-9_-]+)\}").unwrap());
static AUTO_EMPTY_LINK_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[\]\((#[a-zA-Z0-9_-]+)\)").unwrap());
#[allow(dead_code)]
static AUTO_SECTION_LINK_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[([^\]]+)\]\((#[a-zA-Z0-9_-]+)\)").unwrap());
static HTML_EMPTY_LINK_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"<a href="(#[a-zA-Z0-9_-]+)"></a>"#).unwrap());
pub static RAW_INLINE_ANCHOR_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[\]\{#([a-zA-Z0-9_-]+)\}").unwrap());

// List item patterns
pub static LIST_ITEM_WITH_ANCHOR_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(\s*[-*+]|\s*\d+\.)\s+\[\]\{#([a-zA-Z0-9_-]+)\}(.*)$").unwrap());
static LIST_ITEM_ID_MARKER_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"<li><!-- nixos-anchor-id:([a-zA-Z0-9_-]+) -->").unwrap());
static LIST_ITEM_ANCHOR_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"<li>\[\]\{#([a-zA-Z0-9_-]+)\}(.*?)</li>").unwrap());

// Option reference patterns
#[allow(dead_code)]
static OPTION_REF: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"`([a-zA-Z][\w\.]+(\.[\w]+)+)`").unwrap());
static OPTION_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"<code>([a-zA-Z][\w\.]+(\.[\w]+)+)</code>").unwrap());

// Block element patterns
pub static ADMONITION_START_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^:::\s*\{\.([a-zA-Z]+)(?:\s+#([a-zA-Z0-9_-]+))?\}(.*)$").unwrap()
});
pub static ADMONITION_END_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(.*?):::$").unwrap());
pub static FIGURE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r":::\s*\{\.figure(?:\s+#([a-zA-Z0-9_-]+))?\}\s*\n#\s+(.+)\n([\s\S]*?)\s*:::")
        .unwrap()
});

// Definition list patterns
#[allow(dead_code)]
static DEF_LIST_TERM_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^([^:].+)$").unwrap());
#[allow(dead_code)]
static DEF_LIST_DEF_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^:   (.+)$").unwrap());

// Manpage patterns
#[allow(dead_code)]
static MANPAGE_ROLE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\{manpage\}`([^`]+)`").unwrap());
#[allow(dead_code)]
static MANPAGE_MARKUP_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"<span class="manpage-markup">([^<]+)</span>"#).unwrap());
#[allow(dead_code)]
static MANPAGE_REFERENCE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"<span class="manpage-reference">([^<]+)</span>"#).unwrap());

// Header patterns for post-processing
static HEADER_ID_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"<h([1-6])>(.*?)\s*<!--\s*anchor:\s*([a-zA-Z0-9_-]+)\s*-->(.*?)</h[1-6]>").unwrap()
});
static HEADER_H1_WITH_ID_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"<h1>(.*?)\s*\{#([a-zA-Z0-9_-]+)\}(.*?)</h1>").unwrap());
static HEADER_H2_WITH_ID_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"<h2>(.*?)\s*\{#([a-zA-Z0-9_-]+)\}(.*?)</h2>").unwrap());
static HEADER_H3_WITH_ID_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"<h3>(.*?)\s*\{#([a-zA-Z0-9_-]+)\}(.*?)</h3>").unwrap());
static HEADER_H4_WITH_ID_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"<h4>(.*?)\s*\{#([a-zA-Z0-9_-]+)\}(.*?)</h4>").unwrap());
static HEADER_H5_WITH_ID_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"<h5>(.*?)\s*\{#([a-zA-Z0-9_-]+)\}(.*?)</h5>").unwrap());
static HEADER_H6_WITH_ID_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"<h6>(.*?)\s*\{#([a-zA-Z0-9_-]+)\}(.*?)</h6>").unwrap());

// Other HTML post-processing patterns
static BRACKETED_SPAN_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"<p><span id="([^"]+)"></span></p>"#).unwrap());
static P_TAG_ANCHOR_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"<p>\[\]\{#([a-zA-Z0-9_-]+)\}(.*?)</p>").unwrap());
pub static EXPLICIT_ANCHOR_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(#+)\s+(.+?)(?:\s+\{#([a-zA-Z0-9_-]+)\})?\s*$").unwrap());

/// Collect all markdown files from the input directory
pub fn collect_markdown_files(input_dir: &Path) -> Vec<PathBuf> {
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
    files
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
        config.manpage_urls_path.as_ref().and_then(|mappings_path| {
            match load_manpage_urls(mappings_path) {
                Ok(mappings) => Some(mappings),
                Err(err) => {
                    debug!("Error loading manpage mappings: {err}");
                    None
                }
            }
        })
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
    // 1. Process includes
    let with_includes = process_file_includes(content, config);

    // 2. Process admonitions, figures, and definition lists
    let preprocessed = preprocess_block_elements(&with_includes);

    // 3. Process headers with explicit anchors
    let with_headers = preprocess_headers(&preprocessed);

    // 4. Process inline anchors in list items
    let with_inline_anchors = preprocess_inline_anchors(&with_headers);

    // 5. Process special roles
    let with_roles = process_role_markup(&with_inline_anchors, manpage_urls);

    // 6. Extract headers
    let (headers, title) = extract_headers(&with_roles);

    // 7. Convert standard markdown to HTML
    let html_output = convert_to_html(&with_roles, config);

    // 8. Process remaining special elements in the HTML
    let processed_html = post_process_html(html_output, manpage_urls);

    (processed_html, headers, title)
}

/// Process include directives in markdown files
fn process_file_includes(content: &str, config: &Config) -> String {
    let input_dir = match &config.input_dir {
        Some(dir) => dir,
        None => return content.to_string(), // no input directory, return original content
    };

    // Split content into lines for processing
    let lines: Vec<&str> = content.lines().collect();
    let mut processed_content = String::with_capacity(content.len() * 2);

    let mut i = 0;
    let mut in_code_block = false;

    while i < lines.len() {
        let line = lines[i].trim();

        // Check for code block start/end
        if line.starts_with("```") {
            // If this is a start of a code block that might be an include block
            if !in_code_block && line == "```{=include=}" {
                i += 1; // skip the opening line

                // Process file includes until we hit the closing backticks
                while i < lines.len() && lines[i].trim() != "```" {
                    let file_path = lines[i].trim();
                    if !file_path.is_empty() {
                        let full_path = input_dir.join(file_path);

                        match fs::read_to_string(&full_path) {
                            Ok(file_content) => {
                                debug!("Including file: {}", full_path.display());
                                processed_content
                                    .push_str(&format!("<!-- Begin include: {file_path} -->\n"));
                                processed_content.push_str(&file_content);
                                processed_content
                                    .push_str(&format!("\n<!-- End include: {file_path} -->\n\n"));
                            }
                            Err(err) => {
                                error!("Failed to include file {}: {}", full_path.display(), err);
                                processed_content.push_str(&format!(
                                    "**Error: Could not include file `{file_path}`: {err}**\n\n"
                                ));
                            }
                        }
                    }
                    i += 1;
                }

                // Skip the closing backticks
                if i < lines.len() && lines[i].trim() == "```" {
                    i += 1;
                }

                continue;
            }
            // Regular code block toggle
            in_code_block = !in_code_block;
        }

        // Add the line normally
        processed_content.push_str(lines[i]);
        processed_content.push('\n');

        i += 1;
    }

    processed_content
}

/// Process inline anchors by wrapping them in a span with appropriate id
fn preprocess_inline_anchors(content: &str) -> String {
    // First handle list items with anchors at the beginning
    let mut result = String::with_capacity(content.len() + 100);
    let lines = content.lines();

    for line in lines {
        if let Some(caps) = LIST_ITEM_WITH_ANCHOR_RE.captures(line) {
            let list_marker = caps.get(1).unwrap().as_str();
            let id = caps.get(2).unwrap().as_str();
            let content = caps.get(3).unwrap().as_str();

            // Use a span tag that will be preserved in the HTML output
            writeln!(
                result,
                "{list_marker} <span id=\"{id}\" class=\"nixos-anchor\"></span>{content}"
            )
            .unwrap();
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
            let id = caps.get(3).map_or_else(
                || generate_id(text),
                |explicit_id| explicit_id.as_str().to_string(),
            );

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
                    manpage_urls.map_or_else(|| format!("<span class=\"manpage-reference\">{role_content}</span>"), |urls| {
                        urls.get(role_content).map_or_else(|| format!("<span class=\"manpage-reference\">{role_content}</span>"), |url| {
                            format!("<a href=\"{url}\" class=\"manpage-reference\">{role_content}</a>")
                        })
                    })
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
    process_admonitions(&mut lines, &mut processed_lines);
    processed_lines.join("\n")
}

fn process_admonitions(lines: &mut Peekable<Lines>, processed_lines: &mut Vec<String>) {
    // Extracted logic for processing admonitions
    let mut in_admonition = false;
    let mut admonition_content = String::new();
    let mut current_adm_type = String::new();
    let mut current_adm_id = None;

    // For GitHub-style callouts
    // XXX: This was, by far, the cheapest implementation for this. Surprisingly.
    let mut in_github_callout = false;
    let mut github_callout_type = String::new();
    let mut github_callout_content = String::new();
    let github_callout_pattern =
        regex::Regex::new(r"^\s*>\s*\[!(NOTE|TIP|IMPORTANT|WARNING|CAUTION|DANGER)\](.*)$")
            .unwrap();

    while let Some(line) = lines.next() {
        // Process GitHub-style callouts (> [!NOTE] format)
        if let Some(caps) = github_callout_pattern.captures(line) {
            if in_github_callout {
                // Close previous callout
                let processed_callout = process_admonition(
                    &github_callout_type.to_lowercase(),
                    None,
                    &github_callout_content,
                );
                processed_lines.push(processed_callout);
                github_callout_content.clear();
            }

            in_github_callout = true;
            github_callout_type = caps.get(1).unwrap().as_str().to_string();
            github_callout_content.clear();

            // Add any content on the same line as the callout
            let content_part = caps.get(2).map_or("", |m| m.as_str()).trim();
            if !content_part.is_empty() {
                github_callout_content.push_str(content_part);
                github_callout_content.push('\n');
            }
            continue;
        }

        // Continue a GitHub-style callout
        if in_github_callout {
            // Check if line continues the callout (starts with >)
            if line.trim_start().starts_with('>') {
                // Extract content after the >
                let content = line.trim_start().trim_start_matches('>').trim_start();
                github_callout_content.push_str(content);
                github_callout_content.push('\n');
                continue;
            }
            // End of callout
            let processed_callout = process_admonition(
                &github_callout_type.to_lowercase(),
                None,
                &github_callout_content,
            );
            processed_lines.push(processed_callout);
            github_callout_content.clear();
            in_github_callout = false;
        }

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
            let mut lines = content.lines().peekable();
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

    // Close any open GitHub callout
    if in_github_callout {
        let processed_callout = process_admonition(
            &github_callout_type.to_lowercase(),
            None,
            &github_callout_content,
        );
        processed_lines.push(processed_callout);
    }
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
        "<div class=\"admonition {admonition_type}\"{id_attr}>\n<p \
         class=\"admonition-title\">{title}</p>\n{processed_content}</div>"
    )
}

/// Convert markdown to HTML using `pulldown_cmark` with syntax highlighting
fn convert_to_html(markdown: &str, config: &Config) -> String {
    // Implement safe conversion with recovery mechanisms
    markup::safely_process_markup(
        markdown,
        |text| {
            // Set up parser options
            let mut options = Options::empty();
            options.insert(Options::ENABLE_TABLES);
            options.insert(Options::ENABLE_FOOTNOTES);
            options.insert(Options::ENABLE_STRIKETHROUGH);
            options.insert(Options::ENABLE_TASKLISTS);
            // GitHub Flavored Markdown specific options
            options.insert(Options::ENABLE_HEADING_ATTRIBUTES);

            // Create parser
            let parser = Parser::new_ext(text, options);

            // Estimate capacity for the output
            let estimated_capacity = text.len() * 3;
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
                                html_output.push('"');
                            }

                            html_output.push('>');
                            escape_html(&code_block_content, &mut html_output);
                            html_output.push_str("</code></pre>");
                        } else {
                            // Try syntax highlighting with fallback if it fails
                            match highlight::highlight_code(
                                &code_block_content,
                                &code_block_lang,
                                config,
                            ) {
                                Ok(highlighted) => {
                                    html_output.push_str(&highlighted);
                                }
                                Err(err) => {
                                    // Log the error and fallback to non-highlighted code
                                    debug!("Syntax highlighting failed (lang: {code_block_lang}): {err}");
                                    html_output.push_str("<pre><code class=\"language-");
                                    html_output.push_str(&code_block_lang);
                                    html_output.push_str("\">");
                                    escape_html(&code_block_content, &mut html_output);
                                    html_output.push_str("</code></pre>");
                                }
                            }
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
                            pulldown_cmark::html::push_html(
                                &mut html_output,
                                std::iter::once(event),
                            );
                        }
                    }
                }
            }

            html_output
        },
        "<div class=\"error\">Error processing markdown content</div>",
    )
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
    result = process_remaining_inline_anchors(&result);

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

    // Process GitHub Flavored Markdown autolinks
    result = process_autolinks(&result);

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
fn process_remaining_inline_anchors(html: &str) -> String {
    // First process bracketed spans
    let result = BRACKETED_SPAN_RE
        .replace_all(html, |caps: &regex::Captures| {
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
    let mut headers = Vec::new();
    let mut title = None;
    let mut found_headers_with_regex = false;

    // Process line by line for headers with explicit anchors
    let mut in_code_block = false;
    for line in content.lines() {
        // Check if we're entering or leaving a code block
        if line.trim().starts_with("```") {
            in_code_block = !in_code_block;
            continue;
        }

        // Skip processing headers inside code blocks
        if in_code_block {
            continue;
        }

        if let Some(caps) = EXPLICIT_ANCHOR_RE.captures(line) {
            found_headers_with_regex = true;
            let level = u8::try_from(caps[1].len()).expect("Header level should fit in u8");
            let text = caps[2].trim().to_string();

            // Use explicit anchor if provided, otherwise generate one
            let id = caps.get(3).map_or_else(
                || generate_id(&text),
                |explicit_id| explicit_id.as_str().to_string(),
            );

            // Set title from first h1
            if level == 1 && title.is_none() {
                title = Some(text.clone());
            }

            headers.push(Header { text, level, id });
        }
    }

    // If headers were found with regex, don't do the more expensive parser-based
    // extraction
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
    let mut in_code_block = false; // Track if we're inside a code block
    let mut current_level = 0;

    for event in parser {
        match event {
            Event::Start(Tag::CodeBlock(_)) => {
                in_code_block = true;
            }
            Event::End(TagEnd::CodeBlock) => {
                in_code_block = false;
            }
            Event::Start(Tag::Heading { level, .. }) if !in_code_block => {
                // Only process headers outside of code blocks
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
            Event::Text(text) | Event::Code(text) if in_header && !in_code_block => {
                current_header_text.push_str(text.as_ref());
            }
            Event::End(TagEnd::Heading(_)) if in_header && !in_code_block => {
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
        config.manpage_urls_path.as_ref().and_then(|mappings_path| {
            match load_manpage_urls(mappings_path) {
                Ok(mappings) => Some(mappings),
                Err(err) => {
                    debug!("Error loading manpage mappings: {err}");
                    None
                }
            }
        })
    } else {
        None
    };

    let (html, ..) = process_markdown(markdown, manpage_urls.as_ref(), config);
    html
}

/// Process GitHub Flavored Markdown autolinks
fn process_autolinks(html: &str) -> String {
    // Process any text outside of HTML tags and code blocks
    let mut result = String::with_capacity(html.len());
    let mut in_tag = false;
    let mut in_code = false;
    let mut current_text = String::new();

    // Uses char iterator instead of indexing to for proper UTF-8 handling
    // FIXME: this is probably not how we want to handle this.
    let chars = html.chars();

    for c in chars {
        match c {
            '<' => {
                // Process any accumulated text before starting a tag
                if !in_tag && !in_code && !current_text.is_empty() {
                    let processed = markup::AUTOLINK_PATTERN.replace_all(
                        &current_text,
                        |caps: &regex::Captures| {
                            let url = &caps[1];
                            format!("<a href=\"{url}\">{url}</a>")
                        },
                    );
                    result.push_str(&processed);
                    current_text.clear();
                }
                in_tag = true;
                result.push(c);
            }
            '>' => {
                in_tag = false;
                result.push(c);

                // Check if we're entering or leaving a code block
                if result.ends_with("<code") {
                    in_code = true;
                } else if result.ends_with("</code>") {
                    in_code = false;
                }
            }
            _ => {
                if in_tag || in_code {
                    // Inside tags or code, don't process
                    result.push(c);
                } else {
                    // Collect text for processing
                    current_text.push(c);
                }
            }
        }
    }

    // Process any remaining text
    if !current_text.is_empty() {
        let processed =
            markup::AUTOLINK_PATTERN.replace_all(&current_text, |caps: &regex::Captures| {
                let url = &caps[1];
                format!("<a href=\"{url}\">{url}</a>")
            });
        result.push_str(&processed);
    }

    result
}
