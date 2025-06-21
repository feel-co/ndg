pub fn generate_id(text: &str) -> String {
    if text.is_empty() {
        return "section".to_string();
    }

    // Preallocate with approximate capacity to reduce reallocations
    let mut result = String::with_capacity(text.len());
    let mut last_was_hyphen = true; // To trim leading hyphens

    for c in text.chars() {
        if c.is_alphanumeric() {
            // Directly convert to lowercase at point of use
            if let Some(lowercase) = c.to_lowercase().next() {
                result.push(lowercase);
                last_was_hyphen = false;
            }
        } else if !last_was_hyphen {
            // Only add hyphen if previous char wasn't a hyphen
            result.push('-');
            last_was_hyphen = true;
        }
    }

    // Trim trailing hyphen if present
    if result.ends_with('-') {
        result.pop();
    }

    // Return fallback if needed
    if result.is_empty() {
        return "section".to_string();
    }

    result
}

/// Escape HTML special characters in a string
pub fn escape_html(input: &str, output: &mut String) {
    for c in input.chars() {
        match c {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '"' => output.push_str("&quot;"),
            '\'' => output.push_str("&#39;"),
            _ => output.push(c),
        }
    }
}

/// Strip markdown to get plain text
pub fn strip_markdown(content: &str) -> String {
    let mut options = pulldown_cmark::Options::empty();
    options.insert(pulldown_cmark::Options::ENABLE_TABLES);
    options.insert(pulldown_cmark::Options::ENABLE_FOOTNOTES);
    options.insert(pulldown_cmark::Options::ENABLE_STRIKETHROUGH);
    options.insert(pulldown_cmark::Options::ENABLE_TASKLISTS);

    let parser = pulldown_cmark::Parser::new_ext(content, options);

    let mut plain_text = String::new();
    let mut in_code_block = false;

    for event in parser {
        match event {
            pulldown_cmark::Event::Text(text) => {
                if !in_code_block {
                    plain_text.push_str(&text);
                    plain_text.push(' ');
                }
            }
            pulldown_cmark::Event::Start(pulldown_cmark::Tag::CodeBlock(_)) => {
                in_code_block = true;
            }
            pulldown_cmark::Event::End(pulldown_cmark::TagEnd::CodeBlock) => {
                in_code_block = false;
            }
            pulldown_cmark::Event::SoftBreak => {
                plain_text.push(' ');
            }
            pulldown_cmark::Event::HardBreak => {
                plain_text.push('\n');
            }
            _ => {}
        }
    }

    plain_text
}

/// Process content through the markdown pipeline and extract plain text
pub fn process_content_to_plain_text(content: &str, config: &crate::config::Config) -> String {
    let html = crate::formatter::markdown::process_markdown_string(content, config);
    strip_markdown(&html)
        .replace('\n', " ")
        .replace("  ", " ")
        .trim()
        .to_string()
}
