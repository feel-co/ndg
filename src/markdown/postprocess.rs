use std::collections::HashMap;

use lazy_static::lazy_static;
use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use regex::Regex;

use crate::markdown::parser::Header;
use crate::markdown::utils::generate_id;

pub fn process_nixpkgs_elements(
    html: &str,
    manpage_urls: Option<&HashMap<String, String>>,
) -> String {
    lazy_static! {
        // Match command prompts
        static ref PROMPT_RE: Regex = Regex::new(r"<code>\s*\$\s+(.+?)</code>").unwrap();

        // Match manpage references with markup class
        static ref MANPAGE_MARKUP_RE: Regex = Regex::new(r#"<span class="manpage-markup">([^<]+)</span>"#).unwrap();

        // Match already processed manpage references
        static ref MANPAGE_REFERENCE_RE: Regex = Regex::new(r#"<span class="manpage-reference">([^<]+)</span>"#).unwrap();

        // Match option references like `option.path`
        static ref OPTION_RE: Regex = Regex::new(r"<code>([a-zA-Z][\w\.]+(\.[\w]+)+)</code>").unwrap();

        // Extract explicit header ID comments
        static ref HEADER_ID_RE: Regex = Regex::new(r"<h([1-6])>(.*?)\s*<!--\s*anchor:\s*([a-zA-Z0-9_-]+)\s*-->(.*?)</h[1-6]>").unwrap();

        // Process REPL inputs
        static ref REPL_RE: Regex = Regex::new(r"<code>nix-repl&gt;\s*(.*?)</code>").unwrap();

        // Process bracketed spans with attributes
        static ref BRACKETED_SPAN_RE: Regex = Regex::new(r#"<p><span id="([^"]+)"></span></p>"#).unwrap();

        // Process MyST roles
        static ref MYST_ROLE_RE: Regex = Regex::new(r#"<span class="([a-zA-Z]+)-markup">(.*?)</span>"#).unwrap();

        // Match manpage roles in raw markdown format
        static ref MANPAGE_ROLE_RE: Regex = Regex::new(r"\{manpage\}`([^`]+)`").unwrap();
    }

    // Estimate HTML output size
    let estimated_capacity = html.len() * 2;
    let mut result = String::with_capacity(estimated_capacity);

    // Process HTML in chunks to avoid full string copies for each replacement
    let mut cursor = 0;

    // Find matches for all patterns we want to replace
    let mut matches = Vec::new();

    // Collect manpage role matches
    for mat in MANPAGE_ROLE_RE.find_iter(html) {
        matches.push((mat.start(), mat.end(), 0));
    }

    // Collect manpage markup matches
    for mat in MANPAGE_MARKUP_RE.find_iter(html) {
        matches.push((mat.start(), mat.end(), 1));
    }

    // Collect manpage reference matches
    for mat in MANPAGE_REFERENCE_RE.find_iter(html) {
        matches.push((mat.start(), mat.end(), 2));
    }

    // Collect prompt matches
    for mat in PROMPT_RE.find_iter(html) {
        matches.push((mat.start(), mat.end(), 3));
    }

    // Collect option matches
    for mat in OPTION_RE.find_iter(html) {
        matches.push((mat.start(), mat.end(), 4));
    }

    // Collect header ID matches
    for mat in HEADER_ID_RE.find_iter(html) {
        matches.push((mat.start(), mat.end(), 5));
    }

    // Collect REPL matches
    for mat in REPL_RE.find_iter(html) {
        matches.push((mat.start(), mat.end(), 6));
    }

    // Collect bracketed span matches
    for mat in BRACKETED_SPAN_RE.find_iter(html) {
        matches.push((mat.start(), mat.end(), 7));
    }

    // Collect MyST role matches
    for mat in MYST_ROLE_RE.find_iter(html) {
        matches.push((mat.start(), mat.end(), 8));
    }

    // Sort matches by start position
    matches.sort_by_key(|(start, _, _)| *start);

    // Process chunks between matches
    for (start, end, _type) in &matches {
        // Add unmodified content between last match and this one
        if *start > cursor {
            result.push_str(&html[cursor..*start]);
        }

        // Process the matched chunk
        let chunk = &html[*start..*end];
        let processed = match _type {
            0 => process_manpage_role(chunk, manpage_urls),
            1 => process_manpage_markup(chunk, manpage_urls),
            2 => process_manpage_reference(chunk, manpage_urls),
            3 => process_prompt(chunk),
            4 => process_option(chunk),
            5 => process_header_id(chunk),
            6 => process_repl(chunk),
            7 => process_bracketed_span(chunk),
            8 => process_myst_role(chunk),
            _ => chunk.to_string(),
        };

        result.push_str(&processed);
        cursor = *end;
    }

    // Add remaining content after last match
    if cursor < html.len() {
        result.push_str(&html[cursor..]);
    }

    result
}

// Helper functions for individual transformations
fn process_manpage_role(chunk: &str, manpage_urls: Option<&HashMap<String, String>>) -> String {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"\{manpage\}`([^`]+)`").unwrap();
    }

    RE.replace(chunk, |caps: &regex::Captures| {
        let manpage_ref = &caps[1];
        if let Some(urls) = manpage_urls {
            if let Some(url) = urls.get(manpage_ref) {
                return format!("<a href=\"{url}\" class=\"manpage-reference\">{manpage_ref}</a>");
            }
        }
        format!("<span class=\"manpage-reference\">{manpage_ref}</span>")
    })
    .to_string()
}

fn process_manpage_markup(chunk: &str, manpage_urls: Option<&HashMap<String, String>>) -> String {
    lazy_static! {
        static ref RE: Regex =
            Regex::new(r#"<span class="manpage-markup">([^<]+)</span>"#).unwrap();
    }

    if let Some(urls) = manpage_urls {
        RE.replace(chunk, |caps: &regex::Captures| {
            let manpage_ref = &caps[1];
            if let Some(url) = urls.get(manpage_ref) {
                format!("<a href=\"{url}\" class=\"manpage-reference\">{manpage_ref}</a>")
            } else {
                format!("<span class=\"manpage-reference\">{manpage_ref}</span>")
            }
        })
        .to_string()
    } else {
        chunk.to_string()
    }
}

fn process_manpage_reference(
    chunk: &str,
    manpage_urls: Option<&HashMap<String, String>>,
) -> String {
    lazy_static! {
        static ref RE: Regex =
            Regex::new(r#"<span class="manpage-reference">([^<]+)</span>"#).unwrap();
    }

    if let Some(urls) = manpage_urls {
        RE.replace(chunk, |caps: &regex::Captures| {
            let span_content = &caps[1];

            // Direct lookup first
            if let Some(url) = urls.get(span_content) {
                return format!("<a href=\"{url}\" class=\"manpage-reference\">{span_content}</a>");
            }

            // Special case handling
            if span_content == "conf(5)" {
                let full_ref = "nix.conf(5)";
                if let Some(url) = urls.get(full_ref) {
                    return format!("<a href=\"{url}\" class=\"manpage-reference\">{full_ref}</a>");
                }
            }

            format!("<span class=\"manpage-reference\">{span_content}</span>")
        })
        .to_string()
    } else {
        chunk.to_string()
    }
}

fn process_prompt(chunk: &str) -> String {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"<code>\s*\$\s+(.+?)</code>").unwrap();
    }

    RE.replace(chunk, |caps: &regex::Captures| {
        format!(
            "<code class=\"terminal\"><span class=\"prompt\">$</span> {}</code>",
            &caps[1]
        )
    })
    .to_string()
}

fn process_option(chunk: &str) -> String {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"<code>([a-zA-Z][\w\.]+(\.[\w]+)+)</code>").unwrap();
    }

    RE.replace(chunk, |caps: &regex::Captures| {
        let option_path = &caps[1];
        let option_id = format!("option-{}", option_path.replace('.', "-"));
        format!(
            "<a href=\"options.html#{option_id}\" class=\"option-reference\"><code>{option_path}</code></a>"
        )
    }).to_string()
}

fn process_header_id(chunk: &str) -> String {
    lazy_static! {
        static ref RE: Regex =
            Regex::new(r"<h([1-6])>(.*?)\s*<!--\s*anchor:\s*([a-zA-Z0-9_-]+)\s*-->(.*?)</h[1-6]>")
                .unwrap();
    }

    RE.replace(chunk, |caps: &regex::Captures| {
        let level = &caps[1];
        let prefix = &caps[2];
        let id = &caps[3];
        let suffix = &caps[4];

        format!("<h{level} id=\"{id}\">{prefix}{suffix}<!-- anchor added --></h{level}>")
    })
    .to_string()
}

fn process_repl(chunk: &str) -> String {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"<code>nix-repl&gt;\s*(.*?)</code>").unwrap();
    }

    RE.replace(chunk, |caps: &regex::Captures| {
        let content = &caps[1];
        format!(
            "<code class=\"nix-repl\"><span class=\"prompt\">nix-repl&gt;</span> {content}</code>"
        )
    })
    .to_string()
}

fn process_bracketed_span(chunk: &str) -> String {
    lazy_static! {
        static ref RE: Regex = Regex::new(r#"<p><span id="([^"]+)"></span></p>"#).unwrap();
    }

    RE.replace(chunk, |caps: &regex::Captures| {
        let id = &caps[1];
        format!("<span id=\"{id}\" class=\"nixos-anchor\"></span>")
    })
    .to_string()
}

fn process_myst_role(chunk: &str) -> String {
    lazy_static! {
        static ref RE: Regex =
            Regex::new(r#"<span class="([a-zA-Z]+)-markup">(.*?)</span>"#).unwrap();
    }

    RE.replace(chunk, |caps: &regex::Captures| {
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
    })
    .to_string()
}

/// Extract headers from markdown content
pub fn extract_headers(content: &str) -> (Vec<Header>, Option<String>) {
    // First, pre-process the content to extract explicit anchors
    lazy_static! {
        static ref EXPLICIT_ANCHOR_RE: Regex =
            Regex::new(r"^(#+)\s+(.+?)(?:\s+\{#([a-zA-Z0-9_-]+)\})?\s*$").unwrap();
    }

    let mut headers = Vec::new();
    let mut title = None;
    let mut found_headers_with_regex = false;

    // Process line by line to handle explicit anchors
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

    // If headers were found with regex, don't do the more expensive
    // parser-based extraction
    if found_headers_with_regex {
        return (headers, title);
    }

    // If no headers found with regex, fall back to the parser
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

    (headers, title)
}
