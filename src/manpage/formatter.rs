/// Escape special characters for manpage format
pub fn escape_manpage(text: &str) -> String {
    // We only want to escape certain characters when they're in specific contexts
    // For example, a period at the beginning of a line needs escaping
    // but inside text it's usually fine
    let mut result = String::with_capacity(text.len());

    for c in text.chars() {
        match c {
            '\\' => result.push_str("\\\\"), // Escape backslashes
            '-' => result.push_str("\\-"),   // Escape hyphens to prevent interpretation as options
            '\'' => result.push_str("\\'"),  // Escape single quotes
            '\"' => result.push_str("\\\""), // Escape double quotes
            _ => result.push(c),
        }
    }

    result
}

/// Escape a dot at the beginning of a line to prevent it from being
/// interpreted as a troff command
pub fn escape_leading_dot(text: &str) -> String {
    if text.starts_with('.') {
        format!("\\&{}", text)
    } else {
        text.to_string()
    }
}
