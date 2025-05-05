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
