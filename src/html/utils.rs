use std::{collections::HashMap, path::Path};

/// Generate a unique ID from text
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

/// Calculate the relative path prefix needed to reach the root from a given file path
/// For example: "docs/subdir/file.html" would return "../"
///              "docs/subdir/nested/file.html" would return "../../"
pub fn calculate_root_relative_path(file_rel_path: &Path) -> String {
    let depth = file_rel_path.components().count();
    if depth <= 1 {
        String::new() // file is at root level
    } else {
        "../".repeat(depth - 1)
    }
}

/// Generate proper asset paths for templates based on file location
pub fn generate_asset_paths(file_rel_path: &Path) -> HashMap<&'static str, String> {
    let root_prefix = calculate_root_relative_path(file_rel_path);

    let mut paths = HashMap::new();
    paths.insert(
        "stylesheet_path",
        format!("{}assets/style.css", root_prefix),
    );
    paths.insert("main_js_path", format!("{}assets/main.js", root_prefix));
    paths.insert("search_js_path", format!("{}assets/search.js", root_prefix));

    // Navigation paths
    paths.insert("index_path", format!("{}index.html", root_prefix));
    paths.insert("options_path", format!("{}options.html", root_prefix));
    paths.insert("search_path", format!("{}search.html", root_prefix));

    paths
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
