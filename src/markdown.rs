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
    options.insert(Options::ENABLE_SMART_PUNCTUATION);

    // Pre-process markdown for NixOS extensions
    let processed_markdown = preprocess_markdown(markdown);

    // Also apply special markup processing
    let processed_markdown = preprocess_special_markup(&processed_markdown);

    // Regular parser for standard markdown
    let parser = Parser::new_ext(&processed_markdown, options);

    // Convert to HTML
    let mut html_output = String::new();
    pulldown_cmark::html::push_html(&mut html_output, parser);

    // Post-process HTML for NixOS-specific elements
    process_nixpkgs_elements(&html_output, manpage_urls)
}

/// Preprocess markdown with NixOS extensions
fn preprocess_markdown(markdown: &str) -> String {
    lazy_static! {
        // Regex patterns (keeping existing ones)
        static ref HEADING_ANCHOR_RE: Regex = Regex::new(r"^(#+\s+.+?)(?:\s+\{#([a-zA-Z0-9_-]+)\})\s*$").unwrap();
        static ref INLINE_ANCHOR_RE: Regex = Regex::new(r"\[\]\{#([a-zA-Z0-9_-]+)\}").unwrap();
        static ref ADMONITION_START_RE: Regex = Regex::new(r"^:::\s*\{\.([a-zA-Z]+)(?:\s+#([a-zA-Z0-9_-]+))?\}(.*)$").unwrap();
        static ref ADMONITION_END_RE: Regex = Regex::new(r"^(.*?):::$").unwrap();
        static ref FIGURE_RE: Regex = Regex::new(r":::\s*\{\.figure(?:\s+#([a-zA-Z0-9_-]+))?\}\s*\n#\s+(.+)\n(!\[.*?\]\(.+?\))\s*:::").unwrap();
        static ref AUTO_LINK_RE: Regex = Regex::new(r"\[\]\((#[a-zA-Z0-9_-]+)\)").unwrap();
        static ref ROLE_MARKUP_RE: Regex = Regex::new(r"\{([a-zA-Z]+)\}`([^`]+)`").unwrap();
    }

    let mut result = String::new();
    let mut lines = markdown.lines().peekable();
    let mut in_admonition = false;

    while let Some(line) = lines.next() {
        // Check for definition list pattern
        if !line.is_empty() && !line.starts_with(':') {
            if let Some(next_line) = lines.peek() {
                if next_line.starts_with(":   ") {
                    // Found a term
                    let term = line;
                    // Consume the definition line
                    let def_line = lines.next().unwrap();
                    let definition = &def_line[4..]; // Skip ":   "

                    // Append formatted definition list
                    result.push_str(&format!(
                        "<dl>\n<dt>{}</dt>\n<dd>{}</dd>\n</dl>\n\n",
                        term, definition
                    ));
                    continue;
                }
            }
        }

        // Check for inline admonition (start and end on same line)
        if let Some(start_caps) = ADMONITION_START_RE.captures(line) {
            let admonition_type = &start_caps[1];
            let id_attr = if let Some(id) = start_caps.get(2) {
                format!(" id=\"{}\"", id.as_str())
            } else {
                String::new()
            };

            let content_part = start_caps.get(3).map_or("", |m| m.as_str());

            // Check if this is an inline admonition that ends on the same line
            if let Some(end_cap) = ADMONITION_END_RE.captures(content_part) {
                let content = end_cap.get(1).map_or("", |m| m.as_str()).trim();

                // Render inline admonition
                result.push_str(&format!(
                    "<div class=\"admonition {}\"{}>\n<p class=\"admonition-title\">{}</p>\n",
                    admonition_type,
                    id_attr,
                    admonition_type
                        .chars()
                        .next()
                        .unwrap()
                        .to_uppercase()
                        .collect::<String>()
                        + &admonition_type[1..]
                ));

                if !content.is_empty() {
                    result.push_str(&format!("<p>{}</p>\n", content));
                }

                result.push_str("</div>\n\n");
            } else {
                // Start a multi-line admonition
                result.push_str(&format!(
                    "<div class=\"admonition {}\"{}>\n<p class=\"admonition-title\">{}</p>\n",
                    admonition_type,
                    id_attr,
                    admonition_type
                        .chars()
                        .next()
                        .unwrap()
                        .to_uppercase()
                        .collect::<String>()
                        + &admonition_type[1..]
                ));

                // Add content if there's any after the opening tag
                if !content_part.trim().is_empty() {
                    result.push_str(&format!("<p>{}</p>\n", content_part.trim()));
                }

                in_admonition = true;
            }
            continue;
        }

        // Check for admonition end
        if in_admonition && ADMONITION_END_RE.is_match(line) {
            let content_before_end = ADMONITION_END_RE
                .captures(line)
                .and_then(|c| c.get(1))
                .map_or("", |m| m.as_str())
                .trim();

            if !content_before_end.is_empty() {
                result.push_str(&format!("<p>{}</p>\n", content_before_end));
            }

            result.push_str("</div>\n\n");
            in_admonition = false;
            continue;
        }

        // Process standard text line
        let processed_line = HEADING_ANCHOR_RE.replace_all(line, |caps: &regex::Captures| {
            let heading = &caps[1];
            let id = &caps[2];
            format!("{} <!-- anchor: {} -->", heading, id)
        });

        let processed_line =
            INLINE_ANCHOR_RE.replace_all(&processed_line, |caps: &regex::Captures| {
                let id = &caps[1];
                format!("<span id=\"{}\"></span>", id)
            });

        let processed_line = AUTO_LINK_RE.replace_all(&processed_line, |caps: &regex::Captures| {
            let anchor = &caps[1];
            format!("[{}]({})", anchor.trim_start_matches('#'), anchor)
        });

        let processed_line =
            ROLE_MARKUP_RE.replace_all(&processed_line, |caps: &regex::Captures| {
                let role_type = &caps[1];
                let content = &caps[2];
                format!("<span class=\"{}-markup\">{}</span>", role_type, content)
            });

        // If we're in an admonition, we need to make sure any raw HTML is properly wrapped
        // or suffer the consequences
        if in_admonition
            && !processed_line.trim().is_empty()
            && !processed_line.trim_start().starts_with('<')
            && !processed_line.trim_start().starts_with('#')
        {
            result.push_str(&format!("<p>{}</p>\n", processed_line.trim()));
        } else {
            result.push_str(&processed_line);
            result.push('\n');
        }
    }

    // Make sure we close any open admonition at the end of the document
    // XXX: This is *the* 2-line fix to a problem I spent an hour figuring out.
    // Leaving a comment here to forever immortalize my incompetence.
    if in_admonition {
        result.push_str("</div>\n\n");
    }

    // Process figures (keeping existing code)
    result = FIGURE_RE
        .replace_all(&result, |caps: &regex::Captures| {
            let id_attr = if let Some(id) = caps.get(1) {
                format!(" id=\"{}\"", id.as_str())
            } else {
                String::new()
            };
            let title = &caps[2];
            let image = &caps[3];
            format!(
                "<figure{}>\n<figcaption>{}</figcaption>\n{}\n</figure>",
                id_attr, title, image
            )
        })
        .to_string();

    result
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
        static ref MANPAGE_MARKUP: Regex = Regex::new(r"\{manpage\}`([^`]+)`").unwrap();
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

    // Replace {manpage} markup
    result = MANPAGE_MARKUP
        .replace_all(&result, |caps: &regex::Captures| {
            format!("<span class=\"manpage-markup\">{}</span>", &caps[1])
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
        static ref MANPAGE_RE: Regex = Regex::new(r#"<span class="manpage-markup">(.+?)</span>"#).unwrap();

        // Match option references like `option.path`
        static ref OPTION_RE: Regex = Regex::new(r"<code>([a-zA-Z][\w\.]+(\.[\w]+)+)</code>").unwrap();

        // Extract explicit header ID comments - updated regex for better reliability
        static ref HEADER_ID_RE: Regex = Regex::new(r"<h([1-6])>(.*?)\s*<!--\s*anchor:\s*([a-zA-Z0-9_-]+)\s*-->(.*?)</h[1-6]>").unwrap();

        // Process REPL inputs
        static ref REPL_RE: Regex = Regex::new(r"<code>nix-repl&gt;\s*(.*?)</code>").unwrap();

        // Process bracketed spans with attributes
        static ref BRACKETED_SPAN_RE: Regex = Regex::new(r#"<p><span id="([^"]+)"></span></p>"#).unwrap();

        // Process MyST roles
        static ref MYST_ROLE_RE: Regex = Regex::new(r#"<span class="([a-zA-Z]+)-markup">(.*?)</span>"#).unwrap();
    }

    // Process command prompts
    let with_prompts = PROMPT_RE.replace_all(html, |caps: &regex::Captures| {
        format!(
            "<code class=\"terminal\"><span class=\"prompt\">$</span> {}</code>",
            &caps[1]
        )
    });

    // Process manpage references
    let with_manpages = if let Some(urls) = manpage_urls {
        MANPAGE_RE
            .replace_all(&with_prompts, |caps: &regex::Captures| {
                let content = &caps[1];

                // Try to parse the manpage reference
                if let Some(cap) = Regex::new(r"(\w+)\((\d+)\)").unwrap().captures(content) {
                    let name = &cap[1];
                    let section = &cap[2];
                    let manpage_ref = format!("{}({})", name, section);

                    if let Some(url) = urls.get(&manpage_ref) {
                        format!(
                            "<a href=\"{}\" class=\"manpage-reference\">{}</a>",
                            url, &manpage_ref
                        )
                    } else {
                        format!("<span class=\"manpage-reference\">{}</span>", manpage_ref)
                    }
                } else {
                    // Not a valid manpage reference format
                    format!("<span class=\"manpage-markup\">{}</span>", content)
                }
            })
            .to_string()
    } else {
        with_prompts.to_string()
    };

    // Process option references
    let with_options = OPTION_RE.replace_all(&with_manpages, |caps: &regex::Captures| {
        let option_path = &caps[1];
        let option_id = format!("option-{}", option_path.replace('.', "-"));
        format!(
            "<a href=\"options.html#{}\" class=\"option-reference\"><code>{}</code></a>",
            option_id, option_path
        )
    });

    // Process heading anchors
    let with_header_anchors = HEADER_ID_RE.replace_all(&with_options, |caps: &regex::Captures| {
        let level = &caps[1];
        let prefix = &caps[2];
        let id = &caps[3];
        let suffix = &caps[4];

        // Header with built-in anchor
        format!(
            "<h{} id=\"{}\">{}{}<!-- anchor added --></h{}>",
            level, id, prefix, suffix, level
        )
    });

    // Process REPL inputs
    let with_repl = REPL_RE.replace_all(&with_header_anchors, |caps: &regex::Captures| {
        let content = &caps[1];
        format!(
            "<code class=\"nix-repl\"><span class=\"prompt\">nix-repl&gt;</span> {}</code>",
            content
        )
    });

    // Process inline anchors
    let with_inline_anchors =
        BRACKETED_SPAN_RE.replace_all(&with_repl, |caps: &regex::Captures| {
            let id = &caps[1];
            format!("<span id=\"{}\" class=\"nixos-anchor\"></span>", id)
        });

    // Process roles
    let with_roles = MYST_ROLE_RE.replace_all(&with_inline_anchors, |caps: &regex::Captures| {
        let role_type = &caps[1];
        let content = &caps[2];

        match role_type {
            "command" => format!("<code class=\"command\">{}</code>", content),
            "env" => format!("<code class=\"env-var\">{}</code>", content),
            "file" => format!("<code class=\"file-path\">{}</code>", content),
            "option" => format!("<code class=\"nixos-option\">{}</code>", content),
            "var" => format!("<code class=\"nix-var\">{}</code>", content),
            _ => format!("<span class=\"{}-role\">{}</span>", role_type, content),
        }
    });

    with_roles.to_string()
}

/// Extract headers from markdown content
fn extract_headers(content: &str) -> (Vec<Header>, Option<String>) {
    // First, pre-process the content to extract explicit anchors
    lazy_static! {
        static ref EXPLICIT_ANCHOR_RE: Regex =
            Regex::new(r"^(#+)\s+(.+?)(?:\s+\{#([a-zA-Z0-9_-]+)\})?\s*$").unwrap();
    }

    let mut headers = Vec::new();
    let mut title = None;

    // Process line by line to handle explicit anchors
    for line in content.lines() {
        if let Some(caps) = EXPLICIT_ANCHOR_RE.captures(line) {
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

    // If no headers found with regex, fall back to the parser
    if headers.is_empty() {
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

    // Collapse multiple hyphens, trim leading/trailing hyphens
    let segments: Vec<_> = id.split('-').filter(|s| !s.is_empty()).collect();

    // If no valid segments, provide a fallback
    if segments.is_empty() {
        return "section".to_string();
    }

    segments.join("-")
}

/// Process markdown string into HTML, useful for option descriptions
pub fn process_markdown_string(markdown: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_SMART_PUNCTUATION);

    // Pre-process for NixOS extensions
    let processed_markdown = preprocess_markdown(markdown);

    // Also apply special markup processing
    let processed_markdown = preprocess_special_markup(&processed_markdown);

    let parser = Parser::new_ext(&processed_markdown, options);
    let mut html = String::new();
    pulldown_cmark::html::push_html(&mut html, parser);

    // Apply nixpkgs-specific processing - no manpage URLs for option descriptions
    process_nixpkgs_elements(&html, None)
}
