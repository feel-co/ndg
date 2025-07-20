use std::{collections::HashMap, sync::LazyLock};

use log::error;
use regex::Regex;

use crate::utils::process_html_elements;

/// Common regex patterns used across markdown and manpage generation

/// Terminal command prompt patterns
pub static COMMAND_PROMPT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"`\s*\$\s+([^`]+)`").unwrap_or_else(|e| {
        error!("Failed to compile COMMAND_PROMPT regex: {e}");
        never_matching_regex()
    })
});

pub static REPL_PROMPT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"`nix-repl>\s*([^`]+)`").unwrap_or_else(|e| {
        error!("Failed to compile REPL_PROMPT regex: {e}");
        never_matching_regex()
    })
});

/// GitHub Flavored Markdown autolink patterns
pub static AUTOLINK_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(https?://[^\s<>"')\}]+)"#).unwrap_or_else(|e| {
        error!("Failed to compile AUTOLINK_PATTERN regex: {e}");
        never_matching_regex()
    })
});

/// Inline code pattern
pub static INLINE_CODE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"`([^`\n]+)`").unwrap_or_else(|e| {
        error!("Failed to compile INLINE_CODE regex: {e}");
        never_matching_regex()
    })
});

pub static MANPAGE_MARKUP_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"<span class="manpage-markup">([^<]+)</span>"#).unwrap_or_else(|e| {
        error!("Failed to compile MANPAGE_MARKUP_RE regex: {e}");
        never_matching_regex()
    })
});

pub static MANPAGE_REFERENCE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"<span class="manpage-reference">([^<]+)</span>"#).unwrap_or_else(|e| {
        error!("Failed to compile MANPAGE_REFERENCE_RE regex: {e}");
        never_matching_regex()
    })
});

/// Create a regex that never matches anything
///
/// This is used as a fallback pattern when a regex fails to compile.
/// It will never match any input, which is safer than using a trivial regex
/// like `^$` which would match empty strings.
pub fn never_matching_regex() -> Regex {
    // Use a pattern that will never match anything because it asserts something impossible
    Regex::new(r"[^\s\S]").expect("Failed to compile never-matching regex")
}

/// Capitalize the first letter of a string
pub fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    chars.next().map_or_else(String::new, |c| {
        c.to_uppercase().collect::<String>() + chars.as_str()
    })
}

/// Process manpage references, optionally with URL links
///
/// Handles formatting differently for HTML vs troff output
pub fn process_manpage_references(
    text: String,
    manpage_urls: Option<&HashMap<String, String>>,
    is_html: bool,
) -> String {
    let result = if is_html {
        // Process HTML manpage markup
        process_html_elements(&text, &MANPAGE_MARKUP_RE, |caps: &regex::Captures| {
            let manpage_ref = &caps[1];
            manpage_urls.map_or_else(
                || format!("<span class=\"manpage-reference\">{manpage_ref}</span>"),
                |urls| {
                    urls.get(manpage_ref).map_or_else(
                        || format!("<span class=\"manpage-reference\">{manpage_ref}</span>"),
                        |url| {
                            format!(
                                "<a href=\"{url}\" class=\"manpage-reference\">{manpage_ref}</a>"
                            )
                        },
                    )
                },
            )
        })
    } else {
        text
    };

    // Process manpage references that have already been identified
    process_html_elements(&result, &MANPAGE_REFERENCE_RE, |caps: &regex::Captures| {
        let span_content = &caps[1];

        manpage_urls.map_or_else(
            || {
                // No manpage URLs available
                if is_html {
                    format!("<span class=\"manpage-reference\">{span_content}</span>")
                } else {
                    format!("\\fB{span_content}\\fP")
                }
            },
            |urls| {
                urls.get(span_content).map_or_else(
                    || {
                        // URL not found for this manpage reference
                        // Special case for conf(5)
                        if span_content == "conf(5)" {
                            let full_ref = "nix.conf(5)";
                            if let Some(url) = urls.get(full_ref) {
                                if is_html {
                                    return format!(
                                        "<a href=\"{url}\" class=\"manpage-reference\">{full_ref}</a>"
                                    );
                                }
                            }
                        }

                        if is_html {
                            format!("<span class=\"manpage-reference\">{span_content}</span>")
                        } else {
                            format!("\\fB{span_content}\\fP")
                        }
                    },
                    |url| {
                        // URL found
                        if is_html {
                            format!("<a href=\"{url}\" class=\"manpage-reference\">{span_content}</a>")
                        } else {
                            // Format for troff/manpage
                            format!("\\fB{span_content}\\fP")
                        }
                    },
                )
            },
        )
    })
}

/// Format terminal command prompts ($ command)
pub fn process_command_prompts(text: &str, is_html: bool) -> String {
    if is_html {
        COMMAND_PROMPT
            .replace_all(text, |caps: &regex::Captures| {
                let command = &caps[1];
                format!("<code class=\"terminal\"><span class=\"prompt\">$</span> {command}</code>")
            })
            .to_string()
    } else {
        COMMAND_PROMPT
            .replace_all(text, |caps: &regex::Captures| {
                let cmd = &caps[1];
                format!("\\f[C]$ {cmd}\\fP")
            })
            .to_string()
    }
}

/// Format nix REPL prompts (nix-repl> expr)
pub fn process_repl_prompts(text: &str, is_html: bool) -> String {
    if is_html {
        REPL_PROMPT
            .replace_all(text, |caps: &regex::Captures| {
                let expr = &caps[1];
                format!(
                    "<code class=\"nix-repl\"><span class=\"prompt\">nix-repl&gt;</span> {expr}</code>"
                )
            })
            .to_string()
    } else {
        REPL_PROMPT
            .replace_all(text, |caps: &regex::Captures| {
                let expr = &caps[1];
                format!("\\f[C]nix-repl> {expr}\\fP")
            })
            .to_string()
    }
}

/// Format inline code blocks with backticks
pub fn process_inline_code(text: &str, is_html: bool) -> String {
    if is_html {
        // For HTML, the markdown processor typically handles this
        text.to_string()
    } else {
        INLINE_CODE
            .replace_all(text, |caps: &regex::Captures| {
                let code = &caps[1];
                format!("\\fR\\(oq{code}\\(cq\\fP")
            })
            .to_string()
    }
}

/// Apply markup processing to text with safe handling of potential errors
///
/// This is proof that I can learn from my mistakes sometimes.
pub fn safely_process_markup<F>(text: &str, process_fn: F, default_on_error: &str) -> String
where
    F: FnOnce(&str) -> String,
{
    // Avoid processing empty strings
    if text.is_empty() {
        return String::new();
    }

    // Catch any potential panics caused by malformed regex matches or other issues
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| process_fn(text)));

    match result {
        Ok(processed_text) => processed_text,
        Err(e) => {
            // Log the error but allow the program to continue
            if let Some(error_msg) = e.downcast_ref::<String>() {
                error!("Error processing markup: {error_msg}");
            } else if let Some(error_msg) = e.downcast_ref::<&str>() {
                error!("Error processing markup: {error_msg}");
            } else {
                error!("Unknown error occurred while processing markup");
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
