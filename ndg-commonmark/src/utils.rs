use std::collections::HashMap;

use comrak::{Arena, ComrakOptions, nodes::NodeValue, parse_document};
use regex::Regex;

/// Slugify a string for use as an anchor ID.
/// Converts to lowercase, replaces non-alphanumeric characters with dashes,
/// and trims leading/trailing dashes.
#[must_use]
pub fn slugify(text: &str) -> String {
    text.to_lowercase()
        .replace(|c: char| !c.is_alphanumeric() && c != '-' && c != '_', "-")
        .trim_matches('-')
        .to_string()
}

/// Extract the first heading from markdown content as the page title.
/// Returns None if no heading is found.
#[must_use]
pub fn extract_markdown_title(content: &str) -> Option<String> {
    let arena = Arena::new();
    let mut options = ComrakOptions::default();
    options.extension.table = true;
    options.extension.footnotes = true;
    options.extension.strikethrough = true;
    options.extension.tasklist = true;
    options.extension.superscript = true;
    options.render.unsafe_ = true;

    let root = parse_document(&arena, content, &options);

    for node in root.descendants() {
        if let NodeValue::Heading(_) = &node.data.borrow().value {
            let mut text = String::new();
            for child in node.children() {
                if let NodeValue::Text(t) = &child.data.borrow().value {
                    text.push_str(t);
                }
                // Optionally handle inline formatting, code, etc.
                if let NodeValue::Code(t) = &child.data.borrow().value {
                    text.push_str(&t.literal);
                }
            }
            if !text.trim().is_empty() {
                return Some(text.trim().to_string());
            }
        }
    }
    None
}

/// Apply a regex transformation to HTML elements using the provided function.
/// Used by both `legacy_markup` and `legacy_markdown` modules.
pub fn process_html_elements<F>(html: &str, regex: &Regex, transform: F) -> String
where
    F: Fn(&regex::Captures) -> String,
{
    match regex.replace_all(html, transform) {
        std::borrow::Cow::Borrowed(_) => html.to_string(),
        std::borrow::Cow::Owned(s) => s,
    }
}

/// Capitalize the first letter of a string.
pub fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    chars.next().map_or_else(String::new, |c| {
        c.to_uppercase().collect::<String>() + chars.as_str()
    })
}

/// Return true if the string looks like a markdown header (starts with #).
#[must_use]
pub fn is_markdown_header(line: &str) -> bool {
    line.trim_start().starts_with('#')
}

/// Load manpage URL mappings from a JSON file.
pub fn load_manpage_urls(
    path: &str,
) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(path)?;
    let mappings: HashMap<String, String> = serde_json::from_str(&content)?;
    Ok(mappings)
}

/// Safely process markup with panic recovery and graceful error handling.
///
/// Wraps potentially panicking operations and provides
/// graceful degradation when processing fails, ensuring that malformed
/// input doesn't crash the entire document processor.
pub fn safely_process_markup<F>(text: &str, process_fn: F, default_on_error: &str) -> String
where
    F: FnOnce(&str) -> String,
{
    // Avoid processing empty strings
    if text.is_empty() {
        return String::new();
    }

    // Catch any potential panics caused by malformed input or processing errors
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| process_fn(text)));

    match result {
        Ok(processed_text) => processed_text,
        Err(e) => {
            // Log the error but allow the program to continue
            if let Some(error_msg) = e.downcast_ref::<String>() {
                log::error!("Error processing markup: {error_msg}");
            } else if let Some(error_msg) = e.downcast_ref::<&str>() {
                log::error!("Error processing markup: {error_msg}");
            } else {
                log::error!("Unknown error occurred while processing markup");
            }

            // Return the original text or default value to prevent breaking the entire document
            if default_on_error.is_empty() {
                text.to_string()
            } else {
                default_on_error.to_string()
            }
        }
    }
}

/// Create a regex that never matches anything.
///
/// This is used as a fallback pattern when a regex fails to compile.
/// It will never match any input, which is safer than using a trivial regex
/// like `^$` which would match empty strings.
#[must_use]
pub fn never_matching_regex() -> regex::Regex {
    // Use a pattern that will never match anything because it asserts something impossible
    regex::Regex::new(r"[^\s\S]").expect("Failed to compile never-matching regex")
}

/// Process text with error recovery, converting any processing errors to log messages.
///
/// This provides a more lightweight alternative to `safely_process_markup` for operations
/// that are unlikely to panic but may have logical errors.
pub fn process_with_error_recovery<F, T>(
    operation_name: &str,
    input: T,
    process_fn: F,
) -> Result<String, String>
where
    F: FnOnce(T) -> Result<String, Box<dyn std::error::Error>>,
{
    match process_fn(input) {
        Ok(result) => Ok(result),
        Err(e) => {
            let error_msg = format!("Error in {operation_name}: {e}");
            log::error!("{error_msg}");
            Err(error_msg)
        }
    }
}
