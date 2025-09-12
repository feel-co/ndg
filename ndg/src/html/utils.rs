use std::{collections::HashMap, path::Path};

use comrak::nodes::AstNode;

/// Calculate the relative path prefix needed to reach the root from a given file path
/// For example: "docs/subdir/file.html" would return "../"
///              "docs/subdir/nested/file.html" would return "../../"
#[must_use]
pub fn calculate_root_relative_path(file_rel_path: &Path) -> String {
    let depth = file_rel_path.components().count();
    if depth <= 1 {
        String::new() // file is at root level
    } else {
        "../".repeat(depth - 1)
    }
}

/// Generate proper asset paths for templates based on file location
#[must_use]
pub fn generate_asset_paths(file_rel_path: &Path) -> HashMap<&'static str, String> {
    let root_prefix = calculate_root_relative_path(file_rel_path);

    let mut paths = HashMap::new();
    paths.insert("stylesheet_path", format!("{root_prefix}assets/style.css"));
    paths.insert("main_js_path", format!("{root_prefix}assets/main.js"));
    paths.insert("search_js_path", format!("{root_prefix}assets/search.js"));

    // Navigation paths
    paths.insert("index_path", format!("{root_prefix}index.html"));
    paths.insert("options_path", format!("{root_prefix}options.html"));
    paths.insert("search_path", format!("{root_prefix}search.html"));

    paths
}

/// Strip markdown to get plain text
#[must_use]
pub fn strip_markdown(content: &str) -> String {
    use comrak::{Arena, ComrakOptions};
    let arena = Arena::new();
    let mut options = ComrakOptions::default();
    options.extension.table = true;
    options.extension.footnotes = true;
    options.extension.strikethrough = true;
    options.extension.tasklist = true;
    options.render.unsafe_ = true;

    let root = comrak::parse_document(&arena, content, &options);

    let mut plain_text = String::new();
    fn extract_text<'a>(node: &'a AstNode<'a>, plain_text: &mut String, in_code_block: &mut bool) {
        use comrak::nodes::NodeValue;
        match &node.data.borrow().value {
            NodeValue::Text(t) => {
                if !*in_code_block {
                    plain_text.push_str(t);
                    plain_text.push(' ');
                }
            }
            NodeValue::CodeBlock(_) => {
                *in_code_block = true;
            }
            NodeValue::SoftBreak => {
                plain_text.push(' ');
            }
            NodeValue::LineBreak => {
                plain_text.push('\n');
            }
            _ => {}
        }
        for child in node.children() {
            extract_text(child, plain_text, in_code_block);
        }
        if let NodeValue::CodeBlock(_) = &node.data.borrow().value {
            *in_code_block = false;
        }
    }
    let mut in_code_block = false;
    extract_text(root, &mut plain_text, &mut in_code_block);
    plain_text
}

/// Process content through the markdown pipeline and extract plain text
#[must_use]
pub fn process_content_to_plain_text(content: &str, config: &crate::config::Config) -> String {
    let (html, ..) = ndg_commonmark::legacy_markdown::process_markdown(
        content,
        None,
        Some(&config.title),
        std::path::Path::new("."),
    );
    strip_markdown(&html)
        .replace('\n', " ")
        .replace("  ", " ")
        .trim()
        .to_string()
}
