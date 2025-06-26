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
use comrak::{
    Arena, ComrakOptions,
    nodes::{AstNode, NodeHeading, NodeValue},
    parse_document,
};
use log::{error, trace};
use regex::Regex;
use walkdir::WalkDir;

use crate::{
    legacy_markup::{capitalize_first, never_matching_regex, safely_process_markup},
    utils::process_html_elements,
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
        never_matching_regex()
    })
});

pub static MYST_ROLE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"<span class="([a-zA-Z]+)-markup">(.*?)</span>"#).unwrap_or_else(|e| {
        error!("Failed to compile MYST_ROLE_RE regex: {e}");
        never_matching_regex()
    })
});

// Heading and anchor patterns
pub static HEADING_ANCHOR: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(#+)?\s*(.+?)(?:\s+\{#([a-zA-Z0-9_-]+)\})\s*$").unwrap_or_else(|e| {
        error!("Failed to compile HEADING_ANCHOR regex: {e}");
        never_matching_regex()
    })
});
pub static INLINE_ANCHOR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[\]\{#([a-zA-Z0-9_-]+)\}").unwrap());
static AUTO_EMPTY_LINK_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[\]\((#[a-zA-Z0-9_-]+)\)").unwrap());

static HTML_EMPTY_LINK_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"<a href="(#[a-zA-Z0-9_-]+)"></a>"#).unwrap());
pub static RAW_INLINE_ANCHOR_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[\]\{#([a-zA-Z0-9_-]+)\}").unwrap());

// List item patterns
pub static LIST_ITEM_WITH_ANCHOR_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(\s*[-*+]|\\s*\d+\.)\s+\[\]\{#([a-zA-Z0-9_-]+)\}(.*)$").unwrap()
});
static LIST_ITEM_ID_MARKER_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"<li><!-- nixos-anchor-id:([a-zA-Z0-9_-]+) -->").unwrap());
static LIST_ITEM_ANCHOR_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"<li>\[\]\{#([a-zA-Z0-9_-]+)\}(.*?)</li>").unwrap());

// Option reference patterns

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

// Manpage patterns

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
/// Returns (html_content, headers, title) for the given file.
pub fn process_markdown_file(
    file_path: &Path,
    manpage_urls: Option<&HashMap<String, String>>,
    input_dir: Option<&Path>,
    output_dir: Option<&Path>,
    title: Option<&str>,
) -> Result<(String, Vec<Header>, Option<String>)> {
    let content = fs::read_to_string(file_path)
        .with_context(|| format!("Failed to read markdown file: {}", file_path.display()))?;
    let base_dir = file_path.parent().unwrap_or_else(|| Path::new("."));
    let (html_content, headers, found_title) =
        process_markdown(&content, manpage_urls, title, base_dir);

    // Optionally write output if output_dir is provided
    if let (Some(input_dir), Some(output_dir)) = (input_dir, output_dir) {
        let rel_path = file_path.strip_prefix(input_dir).with_context(|| {
            format!(
                "Failed to determine relative path for {}",
                file_path.display()
            )
        })?;
        let mut output_path = output_dir.join(rel_path);
        output_path.set_extension("html");
        if let Some(parent) = output_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
            }
        }
        fs::write(&output_path, &html_content)
            .with_context(|| format!("Failed to write HTML file: {}", output_path.display()))?;
    }

    Ok((html_content, headers, found_title))
}

/// Process Markdown content with NixOS/nixpkgs extensions
pub fn process_markdown(
    content: &str,
    manpage_urls: Option<&HashMap<String, String>>,
    title: Option<&str>,
    base_dir: &std::path::Path,
) -> (String, Vec<Header>, Option<String>) {
    // 1. Process includes (no config needed)
    let with_includes = process_file_includes(content, base_dir);

    // 2. Process admonitions, figures, and definition lists
    let preprocessed = preprocess_block_elements(&with_includes);

    // 3. Process headers with explicit anchors
    let with_headers = preprocess_headers(&preprocessed);

    // 4. Process inline anchors in list items
    let with_inline_anchors = preprocess_inline_anchors(&with_headers);

    // 5. Process special roles
    let with_roles = process_role_markup(&with_inline_anchors, manpage_urls);

    // 6. Extract headers
    let (headers, found_title) = extract_headers(&with_roles);

    // 7. Convert standard markdown to HTML
    let html_output = convert_to_html(&with_roles);

    // 8. Process remaining special elements in the HTML
    let processed_html = post_process_html(html_output, manpage_urls);

    (
        processed_html,
        headers,
        found_title.or_else(|| title.map(|s| s.to_string())),
    )
}

/// Process include directives in markdown files
pub fn process_file_includes(content: &str, base_dir: &std::path::Path) -> String {
    #[cfg(feature = "nixpkgs")]
    {
        crate::extensions::apply_nixpkgs_extensions(content, base_dir)
    }
    #[cfg(not(feature = "nixpkgs"))]
    {
        content.to_string()
    }
}

/// Process inline anchors by wrapping them in a span with appropriate id
pub fn preprocess_inline_anchors(content: &str) -> String {
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
pub fn preprocess_headers(content: &str) -> String {
    let mut lines = vec![];

    for line in content.lines() {
        // Look for headers with anchors but no level sign
        if let Some(caps) = HEADING_ANCHOR.captures(line) {
            let level_signs = caps.get(1).map_or("", |m| m.as_str());
            let text = caps.get(2).unwrap().as_str();
            let id = caps.get(3).map_or_else(
                || crate::utils::slugify(text),
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
pub fn process_role_markup(
    content: &str,
    manpage_urls: Option<&HashMap<String, String>>,
) -> String {
    let result = ROLE_PATTERN
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

    result
}

/// Preprocess block elements like admonitions, figures, and definition lists
pub fn preprocess_block_elements(content: &str) -> String {
    let mut processed_lines = Vec::new();
    let mut lines = content.lines().peekable();
    process_admonitions(&mut lines, &mut processed_lines);
    processed_lines.join("\n")
}

pub fn process_admonitions(lines: &mut Peekable<Lines>, processed_lines: &mut Vec<String>) {
    let mut in_admonition = false;
    let mut admonition_content = String::new();
    let mut current_adm_type = String::new();
    let mut current_adm_id = None;

    // For GitHub-style callouts
    let mut in_github_callout = false;
    let mut github_callout_type = String::new();
    let mut github_callout_content = String::new();
    let github_callout_pattern =
        regex::Regex::new(r"^\s*>\s*\[!(NOTE|TIP|IMPORTANT|WARNING|CAUTION|DANGER)\](.*)$")
            .unwrap();

    while let Some(line) = lines.next() {
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
                    lines.next(); // consume the closing marker
                    break;
                }
                lines.next(); // skip the content lines
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
            admonition_content.clear(); // initialize as empty string

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
    let arena = Arena::new();
    let mut options = ComrakOptions::default();
    options.extension.table = true;
    options.extension.footnotes = cfg!(feature = "gfm") || cfg!(feature = "nixpkgs");
    options.extension.strikethrough = true;
    options.extension.tasklist = true;
    options.render.unsafe_ = true;
    options.parse.smart = true;

    let root = parse_document(&arena, content, &options);

    let mut html_output = vec![];
    comrak::format_html(root, &options, &mut html_output).unwrap_or_default();

    String::from_utf8(html_output).unwrap_or_default()
}

/// Process an admonition and convert it to HTML
fn process_admonition(admonition_type: &str, id: Option<&str>, content: &str) -> String {
    let id_attr = id.map_or(String::new(), |id| format!(" id=\"{id}\""));
    let title = capitalize_first(admonition_type);

    // Process the content as Markdown to HTML
    // XXX: We need to make sure the content has proper line breaks for list items
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

/// Convert markdown to HTML using `comrak` with syntax highlighting
fn convert_to_html(text: &str) -> String {
    safely_process_markup(
        text,
        |text| {
            let arena = Arena::new();
            let mut options = ComrakOptions::default();
            options.extension.table = true;
            options.extension.footnotes = cfg!(feature = "gfm") || cfg!(feature = "nixpkgs");
            options.extension.strikethrough = true;
            options.extension.tasklist = true;
            options.extension.header_ids = Some("".to_string());
            options.render.unsafe_ = true;

            let root = parse_document(&arena, text, &options);

            // AST transformation for prompts
            let prompt_transformer = PromptTransformer;
            prompt_transformer.transform(root);

            // Custom code block handling for syntax highlighting
            let mut html_output = Vec::with_capacity(text.len() * 3);

            comrak::format_html(root, &options, &mut html_output).unwrap_or_default();
            String::from_utf8(html_output).unwrap_or_default()
        },
        // Fallback value
        "<div class=\"error\">Error processing markdown content</div>",
    )
}

/// Walk the AST and replace inline code nodes that match shell
/// or REPL prompts with HTML nodes.
pub trait AstTransformer {
    fn transform<'a>(&self, node: &'a AstNode<'a>);
}

/// Transformer for prompt code spans (shell and REPL).
pub struct PromptTransformer;

impl AstTransformer for PromptTransformer {
    fn transform<'a>(&self, node: &'a AstNode<'a>) {
        use comrak::nodes::NodeValue;
        for child in node.children() {
            {
                let mut data = child.data.borrow_mut();
                if let NodeValue::Code(ref code) = data.value {
                    let literal = code.literal.trim();

                    // Only match a single unescaped $ at the start, and trim whitespace after prompt
                    if literal.starts_with("$")
                        && !literal.starts_with("\\$")
                        && !literal.starts_with("$$")
                    {
                        let rest = literal.strip_prefix("$").unwrap().trim_start();
                        let html = format!(
                            "<code class=\"terminal\"><span class=\"prompt\">$</span> {}</code>",
                            rest
                        );
                        data.value = NodeValue::HtmlInline(html);
                    } else if literal.starts_with("nix-repl>") && !literal.starts_with("nix-repl>>")
                    {
                        let rest = literal.strip_prefix("nix-repl>").unwrap().trim_start();
                        let html = format!(
                            "<code class=\"nix-repl\"><span class=\"prompt\">nix-repl&gt;</span> {}</code>",
                            rest
                        );
                        data.value = NodeValue::HtmlInline(html);
                    }
                }
            }
            self.transform(child);
        }
    }
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
        .map(capitalize_first)
        .collect::<Vec<String>>()
        .join(" ")
}

/// Process manpage roles in the HTML
pub fn process_manpage_roles(
    html: String,
    manpage_urls: Option<&HashMap<String, String>>,
) -> String {
    crate::legacy_markup::process_manpage_references(html, manpage_urls, true)
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
    let title = None;

    // Always use comrak to extract headers to properly strip formatting
    let arena = Arena::new();
    let mut options = ComrakOptions::default();
    options.extension.table = true;
    options.extension.footnotes = cfg!(feature = "gfm") || cfg!(feature = "nixpkgs");
    options.extension.strikethrough = true;
    options.extension.tasklist = true;
    options.extension.superscript = true;
    options.render.unsafe_ = true;

    let root = parse_document(&arena, content, &options);

    let html_tag_re = regex::Regex::new(r"<[^>]*>").unwrap();

    // Collect all lines for anchor extraction
    let lines: Vec<&str> = content.lines().collect();

    let mut found_title = title;
    let mut line_idx = 0;
    for node in root.descendants() {
        if let NodeValue::Heading(NodeHeading { level, .. }) = &node.data.borrow().value {
            // Recursively extract all text from heading's inline children
            fn extract_inline_text<'a>(node: &'a AstNode<'a>) -> String {
                use comrak::nodes::NodeValue;
                let mut text = String::new();
                for child in node.children() {
                    match &child.data.borrow().value {
                        NodeValue::Text(t) => text.push_str(t),
                        NodeValue::Code(t) => text.push_str(&t.literal),
                        NodeValue::Link(..) => text.push_str(&extract_inline_text(child)),
                        NodeValue::Emph => text.push_str(&extract_inline_text(child)),
                        NodeValue::Strong => text.push_str(&extract_inline_text(child)),
                        NodeValue::Strikethrough => text.push_str(&extract_inline_text(child)),
                        NodeValue::Superscript => text.push_str(&extract_inline_text(child)),
                        NodeValue::Subscript => text.push_str(&extract_inline_text(child)),
                        NodeValue::FootnoteReference(..) => {
                            text.push_str(&extract_inline_text(child))
                        }
                        NodeValue::HtmlInline(_html) => {
                            // Skip HTML inline completely
                        }
                        NodeValue::Image(..) => {} // skip images
                        _ => {}
                    }
                }
                text
            }
            let mut text = extract_inline_text(node);

            // Strip any remaining HTML tags from the final text

            if text.contains('<') && text.contains('>') {
                text = html_tag_re.replace_all(&text, "").to_string();
            }

            // Strip trailing {#anchor} from extracted header text for anchor comparison
            let text_stripped = text
                .trim_end()
                .strip_suffix(|c| c == '}')
                .and_then(|s| {
                    let idx = s.rfind("{#");
                    idx.map(|i| s[..i].trim_end())
                })
                .unwrap_or_else(|| text.trim());

            // Try to extract explicit anchor from the corresponding markdown line
            // Find the next non-empty line that starts with the right number of #
            let mut id = crate::utils::slugify(&text);
            let mut tmp = line_idx..lines.len();
            while let Some(i) = tmp.next() {
                let line = lines[i].trim();
                if line.is_empty() {
                    continue;
                }
                // Match header line with explicit anchor
                if let Some(caps) = crate::legacy_markdown::EXPLICIT_ANCHOR_RE.captures(line) {
                    let level_signs = caps.get(1).unwrap().as_str();
                    let anchor_text = caps.get(2).unwrap().as_str();
                    let explicit_id = caps.get(3).map(|m| m.as_str());
                    if level_signs.len() as u8 == *level && anchor_text.trim() == text_stripped {
                        if let Some(explicit_id) = explicit_id {
                            id = explicit_id.to_string();
                        }
                        line_idx = i + 1;
                        break;
                    }
                }
            }

            headers.push(Header {
                text: text.clone(),
                level: *level,
                id,
            });

            // Set the first h1 as the title if not already set
            if found_title.is_none() && *level == 1 {
                found_title = Some(text);
            }
        }
    }

    (headers, found_title)
}

/// Process GitHub Flavored Markdown autolinks
fn process_autolinks(html: &str) -> String {
    // This should match the pattern used in the legacy markup module
    crate::legacy_markup::AUTOLINK_PATTERN
        .replace_all(html, |caps: &regex::Captures| {
            let url = &caps[1];
            format!("<a href=\"{url}\">{url}</a>")
        })
        .to_string()
}
