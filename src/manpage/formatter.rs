pub fn escape_leading_dot(text: &str) -> String {
    if text.starts_with('.') {
        format!("\\&{text}")
    } else {
        text.to_string()
    }
}

pub fn escape_manpage(text: &str) -> String {
    if text.contains("\\f") {
        // Don't escape troff formatting codes
        return text.to_string();
    }

    let mut result = text.to_string();

    result = result
        .replace('-', "\\-")
        .replace('\'', "\\'")
        .replace('.', "\\.")
        .replace('"', "\\\"");

    result
}
