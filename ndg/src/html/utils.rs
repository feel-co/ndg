#![allow(dead_code)]
use std::{collections::HashMap, path::Path};

use comrak::nodes::AstNode;
use ndg_commonmark::legacy_markdown::Header;

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

/// Strip markdown to get plain text
pub fn strip_markdown(content: &str) -> String {
    use comrak::{Arena, ComrakOptions};
    let arena = Arena::new();
    let mut options = ComrakOptions::default();
    options.extension.table = true;
    options.extension.footnotes = true;
    options.extension.strikethrough = true;
    options.extension.tasklist = true;
    options.render.unsafe_ = true;

    let root = comrak::parse_document(&arena, content, &options);

    let mut plain_text = String::new();
    fn extract_text<'a>(node: &'a AstNode<'a>, plain_text: &mut String, in_code_block: &mut bool) {
        use comrak::nodes::NodeValue;
        match &node.data.borrow().value {
            NodeValue::Text(t) => {
                if !*in_code_block {
                    plain_text.push_str(t);
                    plain_text.push(' ');
                }
            }
            NodeValue::CodeBlock(_) => {
                *in_code_block = true;
            }
            NodeValue::SoftBreak => {
                plain_text.push(' ');
            }
            NodeValue::LineBreak => {
                plain_text.push('\n');
            }
            _ => {}
        }
        for child in node.children() {
            extract_text(child, plain_text, in_code_block);
        }
        if let NodeValue::CodeBlock(_) = &node.data.borrow().value {
            *in_code_block = false;
        }
    }
    let mut in_code_block = false;
    extract_text(root, &mut plain_text, &mut in_code_block);
    plain_text
}

/// Process content through the markdown pipeline and extract plain text
pub fn process_content_to_plain_text(content: &str, config: &crate::config::Config) -> String {
    let (html, ..) = ndg_commonmark::legacy_markdown::process_markdown(content, None, Some(&config.title));
    strip_markdown(&html)
        .replace('\n', " ")
        .replace("  ", " ")
        .trim()
        .to_string()
}

/// Process manpage roles in the HTML
pub fn process_manpage_roles(
    html: String,
    manpage_urls: Option<&HashMap<String, String>>,
) -> String {
    ndg_commonmark::legacy_markdown::process_manpage_roles(html, manpage_urls)
}

/// Extract headers from markdown content
pub fn extract_headers(content: &str) -> (Vec<Header>, Option<String>) {
    ndg_commonmark::legacy_markdown::extract_headers(content)
}
