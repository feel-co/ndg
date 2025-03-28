use lazy_static::lazy_static;
use regex::Regex;
use std::borrow::Cow;

/// Preprocess markdown with `NixOS` extensions
pub fn preprocess_markdown(markdown: &str) -> String {
    // First process the special markup patterns
    let mut result = preprocess_special_markup(markdown);

    lazy_static! {
        // Core regex patterns
        static ref HEADING_ANCHOR_RE: Regex = Regex::new(r"^(#+\s+.+?)(?:\s+\{#([a-zA-Z0-9_-]+)\})\s*$").unwrap();
        static ref INLINE_ANCHOR_RE: Regex = Regex::new(r"\[\]\{#([a-zA-Z0-9_-]+)\}").unwrap();
        static ref AUTO_LINK_RE: Regex = Regex::new(r"\[\]\((#[a-zA-Z0-9_-]+)\)").unwrap();
        static ref ADMONITION_START_RE: Regex = Regex::new(r"^:::\s*\{\.([a-zA-Z]+)(?:\s+#([a-zA-Z0-9_-]+))?\}(.*)$").unwrap();
        static ref ADMONITION_END_RE: Regex = Regex::new(r"^(.*?):::$").unwrap();
        static ref FIGURE_RE: Regex = Regex::new(r":::\s*\{\.figure(?:\s+#([a-zA-Z0-9_-]+))?\}\s*\n#\s+(.+)\n(!\[.*?\]\(.+?\))\s*:::").unwrap();
    }

    // Process figures first (they span multiple lines)
    result = FIGURE_RE
        .replace_all(&result, |caps: &regex::Captures| {
            let id_attr = caps
                .get(1)
                .map_or(String::new(), |id| format!(" id=\"{}\"", id.as_str()));
            let title = &caps[2];
            let image = &caps[3];
            format!(
                "<figure{id_attr}>\n<figcaption>{title}</figcaption>\n{image}\n</figure>"
            )
        })
        .into_owned();

    // Process line by line for admonitions and other patterns
    let mut processed_lines = Vec::new();
    let mut lines = result.lines().peekable();
    let mut in_admonition = false;

    while let Some(line) = lines.next() {
        // Handle definition lists
        if !line.is_empty() && !line.starts_with(':') {
            if let Some(next_line) = lines.peek() {
                if next_line.starts_with(":   ") {
                    let term = line;
                    let def_line = lines.next().unwrap();
                    let definition = &def_line[4..]; // Skip ":   "

                    processed_lines.push(format!(
                        "<dl>\n<dt>{term}</dt>\n<dd>{definition}</dd>\n</dl>"
                    ));
                    continue;
                }
            }
        }

        // Handle admonitions
        if let Some(start_caps) = ADMONITION_START_RE.captures(line) {
            let admonition_type = &start_caps[1];
            let id_attr = start_caps
                .get(2)
                .map_or(String::new(), |id| format!(" id=\"{}\"", id.as_str()));
            let content_part = start_caps.get(3).map_or("", |m| m.as_str());

            let title = admonition_type
                .chars()
                .next().map_or_else(|| admonition_type.to_string(), |c| c.to_uppercase().collect::<String>() + &admonition_type[1..]);

            // Check if this is an inline admonition
            if let Some(end_cap) = ADMONITION_END_RE.captures(content_part) {
                let content = end_cap.get(1).map_or("", |m| m.as_str()).trim();

                let mut admon = format!(
                    "<div class=\"admonition {admonition_type}\"{id_attr}>\n<p class=\"admonition-title\">{title}</p>"
                );

                if !content.is_empty() {
                    admon.push_str(&format!("\n<p>{content}</p>"));
                }

                admon.push_str("\n</div>");
                processed_lines.push(admon);
            } else {
                // Start a multi-line admonition
                let mut admon = format!(
                    "<div class=\"admonition {admonition_type}\"{id_attr}>\n<p class=\"admonition-title\">{title}</p>"
                );

                if !content_part.trim().is_empty() {
                    admon.push_str(&format!("\n<p>{}</p>", content_part.trim()));
                }

                processed_lines.push(admon);
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
                processed_lines.push(format!("<p>{content_before_end}</p>"));
            }

            processed_lines.push("</div>".to_string());
            in_admonition = false;
            continue;
        }

        // Process other text transformations
        let processed_line = process_line_markup(line, in_admonition);
        processed_lines.push(processed_line);
    }

    // Close any open admonition at the end of the document
    if in_admonition {
        processed_lines.push("</div>".to_string());
    }

    processed_lines.join("\n")
}

/// Process inline markup in a single line
fn process_line_markup(line: &str, in_admonition: bool) -> String {
    lazy_static! {
        static ref HEADING_ANCHOR_RE: Regex =
            Regex::new(r"^(#+\s+.+?)(?:\s+\{#([a-zA-Z0-9_-]+)\})\s*$").unwrap();
        static ref INLINE_ANCHOR_RE: Regex = Regex::new(r"\[\]\{#([a-zA-Z0-9_-]+)\}").unwrap();
        static ref AUTO_LINK_RE: Regex = Regex::new(r"\[\]\((#[a-zA-Z0-9_-]+)\)").unwrap();
    }

    // Apply transformations in sequence
    let mut processed = Cow::from(line);

    // Handle heading anchors
    if HEADING_ANCHOR_RE.is_match(&processed) {
        processed = Cow::Owned(
            HEADING_ANCHOR_RE
                .replace_all(&processed, |caps: &regex::Captures| {
                    let heading = &caps[1];
                    let id = &caps[2];
                    format!("{heading} <!-- anchor: {id} -->")
                })
                .into_owned(),
        );
    }

    // Handle inline anchors
    if INLINE_ANCHOR_RE.is_match(&processed) {
        processed = Cow::Owned(
            INLINE_ANCHOR_RE
                .replace_all(&processed, |caps: &regex::Captures| {
                    let id = &caps[1];
                    format!("<span id=\"{id}\"></span>")
                })
                .into_owned(),
        );
    }

    // Handle auto links
    if AUTO_LINK_RE.is_match(&processed) {
        processed = Cow::Owned(
            AUTO_LINK_RE
                .replace_all(&processed, |caps: &regex::Captures| {
                    let anchor = &caps[1];
                    format!("[{}]({})", anchor.trim_start_matches('#'), anchor)
                })
                .into_owned(),
        );
    }

    let processed = processed.into_owned();

    // Special handling for text in admonitions
    if in_admonition
        && !processed.trim().is_empty()
        && !processed.trim_start().starts_with('<')
        && !processed.trim_start().starts_with('#')
    {
        format!("<p>{}</p>", processed.trim())
    } else {
        processed
    }
}

/// Preprocess special markup patterns like {file}`path` in markdown
pub fn preprocess_special_markup(markdown: &str) -> String {
    lazy_static! {
        static ref MARKUP_PATTERN: Regex =
            Regex::new(r"\{(command|env|file|option|var|manpage)\}`([^`]+)`").unwrap();
    }

    // Single pass replacement for all markup types
    MARKUP_PATTERN
        .replace_all(markdown, |caps: &regex::Captures| {
            let markup_type = &caps[1];
            let content = &caps[2];
            format!("<span class=\"{markup_type}-markup\">{content}</span>")
        })
        .into_owned()
}
