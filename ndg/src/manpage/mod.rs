mod options;

use std::{collections::HashMap, sync::LazyLock};

use regex::Regex;

use crate::formatter::{self, markup};
pub use crate::manpage::options::generate_manpage;

// These patterns need to be applied sequentially to preserve troff formatting codes
pub static TROFF_FORMATTING: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\\f[PBIR]").unwrap_or_else(|e| {
        log::error!("Failed to compile TROFF_FORMATTING regex: {e}");
        markup::never_matching_regex()
    })
});

pub static TROFF_ESCAPE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\\[\(\[\\\.]").unwrap_or_else(|e| {
        log::error!("Failed to compile TROFF_ESCAPE regex: {e}");
        markup::never_matching_regex()
    })
});

/// Map of characters that need to be escaped in manpages
#[must_use] pub fn get_roff_escapes() -> HashMap<char, &'static str> {
    let mut map = HashMap::new();
    map.insert('"', "\\(dq");
    map.insert('\'', "\\(aq");
    map.insert('-', "\\-");
    map.insert('.', "\\&.");
    map.insert('\\', "\\\\");
    map.insert('^', "\\(ha");
    map.insert('`', "\\(ga");
    map.insert('~', "\\(ti");
    map
}

/// Escapes a string for use in manpages
#[must_use] pub fn man_escape(s: &str) -> String {
    formatter::markup::safely_process_markup(
        s,
        |text| {
            let escapes = get_roff_escapes();
            let mut result = String::with_capacity(text.len() * 2);

            for c in text.chars() {
                if let Some(escape) = escapes.get(&c) {
                    result.push_str(escape);
                } else {
                    result.push(c);
                }
            }

            result
        },
        s,
    )
}

/// Escape a leading dot to prevent it from being interpreted as a troff command
#[must_use] pub fn escape_leading_dot(text: &str) -> String {
    formatter::markup::safely_process_markup(
        text,
        |text| {
            if text.starts_with('.') || text.starts_with('\'') {
                format!("\\&{text}")
            } else {
                text.to_string()
            }
        },
        text,
    )
}
