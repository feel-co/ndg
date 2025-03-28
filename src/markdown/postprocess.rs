use lazy_static::lazy_static;
use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use regex::Regex;
use std::collections::HashMap;

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

    // Process any remaining raw {manpage} roles first
    let with_raw_manpage_roles = MANPAGE_ROLE_RE
        .replace_all(html, |caps: &regex::Captures| {
            let manpage_ref = &caps[1];
            if let Some(urls) = manpage_urls {
                if let Some(url) = urls.get(manpage_ref) {
                    return format!(
                        "<a href=\"{}\" class=\"manpage-reference\">{}</a>",
                        url, manpage_ref
                    );
                }
            }
            format!("<span class=\"manpage-reference\">{}</span>", manpage_ref)
        })
        .to_string();

    // Process spans with class="manpage-markup"
    let with_manpage_markup = if let Some(urls) = manpage_urls {
        MANPAGE_MARKUP_RE
            .replace_all(&with_raw_manpage_roles, |caps: &regex::Captures| {
                let manpage_ref = &caps[1]; // this should be "nix.conf(5)"
                if let Some(url) = urls.get(manpage_ref) {
                    format!(
                        "<a href=\"{}\" class=\"manpage-reference\">{}</a>",
                        url, manpage_ref
                    )
                } else {
                    format!("<span class=\"manpage-reference\">{}</span>", manpage_ref)
                }
            })
            .to_string()
    } else {
        with_raw_manpage_roles
    };

    // Process spans with class="manpage-reference"
    let with_manpage_spans = if let Some(urls) = manpage_urls {
        MANPAGE_REFERENCE_RE
            .replace_all(&with_manpage_markup, |caps: &regex::Captures| {
                let span_content = &caps[1]; // this might be "conf(5)"

                // Direct lookup first
                if let Some(url) = urls.get(span_content) {
                    return format!(
                        "<a href=\"{}\" class=\"manpage-reference\">{}</a>",
                        url, span_content
                    );
                }

                // Special case handling
                if span_content == "conf(5)" {
                    let full_ref = "nix.conf(5)";
                    if let Some(url) = urls.get(full_ref) {
                        return format!(
                            "<a href=\"{}\" class=\"manpage-reference\">{}</a>",
                            url,
                            full_ref // show, e.g., "nix.conf(5)" instead of "conf(5)"
                        );
                    }
                }

                format!("<span class=\"manpage-reference\">{}</span>", span_content)
            })
            .to_string()
    } else {
        with_manpage_markup
    };

    // Process command prompts
    let with_prompts = PROMPT_RE.replace_all(&with_manpage_spans, |caps: &regex::Captures| {
        format!(
            "<code class=\"terminal\"><span class=\"prompt\">$</span> {}</code>",
            &caps[1]
        )
    });

    let with_options = OPTION_RE.replace_all(&with_prompts, |caps: &regex::Captures| {
        let option_path = &caps[1];
        let option_id = format!("option-{}", option_path.replace('.', "-"));
        format!(
            "<a href=\"options.html#{}\" class=\"option-reference\"><code>{}</code></a>",
            option_id, option_path
        )
    });

    let with_header_anchors = HEADER_ID_RE.replace_all(&with_options, |caps: &regex::Captures| {
        let level = &caps[1];
        let prefix = &caps[2];
        let id = &caps[3];
        let suffix = &caps[4];

        format!(
            "<h{} id=\"{}\">{}{}<!-- anchor added --></h{}>",
            level, id, prefix, suffix, level
        )
    });

    let with_repl = REPL_RE.replace_all(&with_header_anchors, |caps: &regex::Captures| {
        let content = &caps[1];
        format!(
            "<code class=\"nix-repl\"><span class=\"prompt\">nix-repl&gt;</span> {}</code>",
            content
        )
    });

    let with_inline_anchors =
        BRACKETED_SPAN_RE.replace_all(&with_repl, |caps: &regex::Captures| {
            let id = &caps[1];
            format!("<span id=\"{}\" class=\"nixos-anchor\"></span>", id)
        });

    let with_roles = MYST_ROLE_RE.replace_all(&with_inline_anchors, |caps: &regex::Captures| {
        let role_type = &caps[1];
        let content = &caps[2];

        match role_type {
            "command" => format!("<code class=\"command\">{}</code>", content),
            "env" => format!("<code class=\"env-var\">{}</code>", content),
            "file" => format!("<code class=\"file-path\">{}</code>", content),
            "option" => format!("<code class=\"nixos-option\">{}</code>", content),
            "var" => format!("<code class=\"nix-var\">{}</code>", content),
            _ => format!("<span class=\"{}-markup\">{}</span>", role_type, content),
        }
    });

    with_roles.to_string()
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
