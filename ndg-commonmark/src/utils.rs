use std::collections::HashMap;

/// Slugify a string for use as an anchor ID.
/// Converts to lowercase, replaces non-alphanumeric characters with dashes,
/// and trims leading/trailing dashes.
pub fn slugify(text: &str) -> String {
    text.to_lowercase()
        .replace(|c: char| !c.is_alphanumeric() && c != '-' && c != '_', "-")
        .trim_matches('-')
        .to_string()
}

/// Capitalize the first letter of a string.
pub fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    chars.next().map_or_else(String::new, |c| {
        c.to_uppercase().collect::<String>() + chars.as_str()
    })
}

/// Return true if the string looks like a markdown header (starts with #).
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
