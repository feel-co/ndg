use std::{collections::HashMap, sync::LazyLock};

use regex::Regex;

// Common regex patterns used across markdown and manpage generation
// Role patterns for syntax like {command}`ls -l`
pub static ROLE_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\{([a-z]+)\}`([^`]+)`").unwrap());

// Terminal command prompt patterns
pub static COMMAND_PROMPT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"`\s*\$\s+([^`]+)`").unwrap());
pub static REPL_PROMPT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"`nix-repl>\s*([^`]+)`").unwrap());
#[allow(dead_code)]
static PROMPT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"<code>\s*\$\s+(.+?)</code>").unwrap());
#[allow(dead_code)]
static REPL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"<code>nix-repl&gt;\s*(.*?)</code>").unwrap());

// Inline code pattern
pub static INLINE_CODE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"`([^`]+)`").unwrap());

// Manpage reference patterns
#[allow(dead_code)]
static MANPAGE_ROLE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\{manpage\}`([^`]+)`").unwrap());
pub static MANPAGE_MARKUP_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"<span class="manpage-markup">([^<]+)</span>"#).unwrap());
pub static MANPAGE_REFERENCE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"<span class="manpage-reference">([^<]+)</span>"#).unwrap());

/// Apply a regex transformation to HTML elements using the provided function
pub fn process_html_elements<F>(html: &str, regex: &Regex, transform: F) -> String
where
    F: Fn(&regex::Captures) -> String,
{
    regex.replace_all(html, transform).to_string()
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
        MANPAGE_MARKUP_RE
            .replace_all(&text, |caps: &regex::Captures| {
                let manpage_ref = &caps[1];
                manpage_urls.map_or_else(
                    || format!("<span class=\"manpage-reference\">{manpage_ref}</span>"),
                    |urls| {
                        urls.get(manpage_ref).map_or_else(
                     || format!("<span class=\"manpage-reference\">{manpage_ref}</span>"),
                     |url| {
                        format!("<a href=\"{url}\" class=\"manpage-reference\">{manpage_ref}</a>")
                     },
                  )
                    },
                )
            })
            .to_string()
    } else {
        text
    };

    // Process manpage references that have already been identified
    MANPAGE_REFERENCE_RE
        .replace_all(&result, |caps: &regex::Captures| {
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
        .to_string()
}

/// Process role-based formatting in text (like {command}`ls -l`)
///
/// Works with both HTML and troff outputs
pub fn process_roles(
    text: &str,
    manpage_urls: Option<&HashMap<String, String>>,
    is_html: bool,
) -> String {
    ROLE_PATTERN
        .replace_all(text, |caps: &regex::Captures| {
            let role_type = &caps[1];
            let content = &caps[2];

            if is_html {
                // HTML formatting
                match role_type {
                    "manpage" => manpage_urls.map_or_else(
                        || format!("<span class=\"manpage-reference\">{content}</span>"),
                        |urls| {
                            urls.get(content).map_or_else(
                           || format!("<span class=\"manpage-reference\">{content}</span>"),
                           |url| {
                              format!("<a href=\"{url}\" class=\"manpage-reference\">{content}</a>")
                           },
                        )
                        },
                    ),
                    "command" => format!("<code class=\"command\">{content}</code>"),
                    "env" => format!("<code class=\"env-var\">{content}</code>"),
                    "file" => format!("<code class=\"file-path\">{content}</code>"),
                    "option" => format!("<code class=\"nixos-option\">{content}</code>"),
                    "var" => format!("<code class=\"nix-var\">{content}</code>"),
                    _ => format!("<span class=\"{role_type}-markup\">{content}</span>"),
                }
            } else {
                // Troff formatting for man pages
                match role_type {
                    "command" | "option" => format!("\\fB{content}\\fP"),
                    "env" | "file" | "var" => format!("\\fI{content}\\fP"),
                    "manpage" => {
                        if let Some((page, section)) = content.rsplit_once('(') {
                            let page = page.trim();
                            let section = section.trim_end_matches(')');
                            format!("\\fB{page}\\fP({section})")
                        } else {
                            format!("\\fB{content}\\fP")
                        }
                    }
                    _ => format!("\\fI{content}\\fP"),
                }
            }
        })
        .to_string()
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
