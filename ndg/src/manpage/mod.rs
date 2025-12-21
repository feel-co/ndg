mod options;

use std::{collections::HashMap, sync::LazyLock};

use ndg_commonmark::{process_safe, utils::never_matching_regex};
use regex::Regex;

pub use crate::manpage::options::generate_manpage;

// These patterns need to be applied sequentially to preserve troff formatting
// codes
pub static TROFF_FORMATTING: LazyLock<Regex> = LazyLock::new(|| {
  Regex::new(r"\\f[PBIR]").unwrap_or_else(|e| {
    log::error!("Failed to compile TROFF_FORMATTING regex: {e}");
    never_matching_regex().unwrap_or_else(|_| {
      #[allow(
        clippy::expect_used,
        reason = "This pattern is guaranteed to be valid"
      )]
      Regex::new(r"[^\s\S]")
        .expect("regex pattern [^\\s\\S] should always compile")
    })
  })
});

pub static TROFF_ESCAPE: LazyLock<Regex> = LazyLock::new(|| {
  Regex::new(r"\\[\(\[\\\.]").unwrap_or_else(|e| {
    log::error!("Failed to compile TROFF_ESCAPE regex: {e}");
    never_matching_regex().unwrap_or_else(|_| {
      #[allow(
        clippy::expect_used,
        reason = "This pattern is guaranteed to be valid"
      )]
      Regex::new(r"[^\s\S]")
        .expect("regex pattern [^\\s\\S] should always compile")
    })
  })
});

/// Map of characters that need to be escaped in manpages
pub static ROFF_ESCAPES: LazyLock<HashMap<char, &'static str>> =
  LazyLock::new(|| {
    let mut map = HashMap::with_capacity(8);
    map.insert('"', "\\(dq");
    map.insert('\'', "\\(aq");
    map.insert('-', "\\-");
    map.insert('.', "\\&.");
    map.insert('\\', "\\\\");
    map.insert('^', "\\(ha");
    map.insert('`', "\\(ga");
    map.insert('~', "\\(ti");
    map
  });

/// Escapes a string for use in manpages
#[must_use]
pub fn man_escape(s: &str) -> String {
  process_safe(
    s,
    |text| {
      let mut result = String::with_capacity(text.len() * 2);

      for c in text.chars() {
        if let Some(escape) = ROFF_ESCAPES.get(&c) {
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
#[must_use]
pub fn escape_leading_dot(text: &str) -> String {
  process_safe(
    text,
    |text| {
      if text.starts_with('.')
        || text.starts_with('\'')
        || text.starts_with("\\&'")
        || text.starts_with("\\[aq]")
      {
        format!("\\&{text}")
      } else {
        text.to_string()
      }
    },
    text,
  )
}

/// Escape lines except those starting with man macros we emit (e.g., .IP)
#[must_use]
pub fn escape_non_macro_lines(text: &str) -> String {
  text
    .lines()
    .map(|line| {
      if line.starts_with(".IP ")
        || line.starts_with(".RS")
        || line.starts_with(".RE")
      {
        line.to_string()
      } else {
        escape_leading_dot(line)
      }
    })
    .collect::<Vec<_>>()
    .join("\n")
}
