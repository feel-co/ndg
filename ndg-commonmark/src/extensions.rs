#![allow(dead_code)]
//! Extension logic for ndg-commonmark: GFM and Nixpkgs/NixOS flavored markdown.
//!
//! This module is intended to house all logic for feature-flagged extensions
//! to the base `CommonMark` processor, including GitHub Flavored Markdown (GFM)
//! and Nixpkgs/NixOS documentation-specific syntax.

/// Apply GitHub Flavored Markdown (GFM) extensions to the input markdown.
///
/// This is a placeholder for future GFM-specific preprocessing or AST transformations.
/// In practice, most GFM features are enabled via comrak options, but additional
/// logic (such as custom tables, task lists, etc.) can be added here.
#[cfg(feature = "gfm")]
pub fn apply_gfm_extensions(markdown: &str) -> String {
    // XXX: Comrak already supports GFM, but if there is any feature in the spec
    // that is not implemented as we'd like for it to be, we can add it here.
    markdown.to_owned()
}

/// Apply Nixpkgs/NixOS documentation extensions to the input markdown.
#[cfg(feature = "nixpkgs")]
pub fn apply_nixpkgs_extensions(markdown: &str, base_dir: &std::path::Path) -> String {
    use std::{fs, path::Path};

    use log;

    // Check if a path is safe (no absolute, no ..)
    fn is_safe_path(path: &str) -> bool {
        let p = Path::new(path);
        !p.is_absolute() && !path.contains("..") && !path.contains('\\')
    }

    // Read included files, return concatenated content
    fn read_includes(listing: &str, base_dir: &Path) -> String {
        let mut result = String::new();
        for line in listing.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || !is_safe_path(trimmed) {
                continue;
            }
            let full_path = base_dir.join(trimmed);
            log::info!("Including file: {}", full_path.display());
            match fs::read_to_string(&full_path) {
                Ok(content) => {
                    result.push_str(&content);
                    if !content.ends_with('\n') {
                        result.push('\n');
                    }
                }
                Err(_) => {
                    // Insert a warning comment for missing files
                    result.push_str(&format!(
                        "<!-- ndg: could not include file: {} -->\n",
                        full_path.display()
                    ));
                }
            }
        }
        result
    }

    // Replace {=include=} code blocks with included file contents
    let mut output = String::new();
    let mut lines = markdown.lines().peekable();

    while let Some(line) = lines.next() {
        if line.trim_start().starts_with("```{=include=}") {
            // Start of an include block
            let mut include_listing = String::new();
            for next_line in lines.by_ref() {
                if next_line.trim_start().starts_with("```") {
                    break;
                }
                include_listing.push_str(next_line);
                include_listing.push('\n');
            }

            let included = read_includes(&include_listing, base_dir);
            output.push_str(&included);
        } else {
            output.push_str(line);
            output.push('\n');
        }
    }

    output
}
