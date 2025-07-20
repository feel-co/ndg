use std::collections::HashMap;

use comrak::{
    Arena, ComrakOptions,
    nodes::{AstNode, NodeHeading, NodeValue},
    parse_document,
};
use markup5ever::{local_name, namespace_url, ns};

use crate::{
    types::{Header, MarkdownResult},
    utils,
};

/// Options for configuring the Markdown processor.
#[derive(Debug, Clone)]
pub struct MarkdownOptions {
    /// Enable GitHub Flavored Markdown extensions.
    pub gfm: bool,

    /// Enable Nixpkgs/NixOS documentation extensions.
    pub nixpkgs: bool,

    /// Enable syntax highlighting for code blocks.
    pub highlight_code: bool,

    /// Optional: Path to manpage URL mappings (for {manpage} roles).
    pub manpage_urls_path: Option<String>,
}

impl Default for MarkdownOptions {
    fn default() -> Self {
        Self {
            gfm: cfg!(feature = "gfm"),
            nixpkgs: cfg!(feature = "nixpkgs"),
            highlight_code: true,
            manpage_urls_path: None,
        }
    }
}

/// Main Markdown processor struct.
pub struct MarkdownProcessor {
    options: MarkdownOptions,
    manpage_urls: Option<HashMap<String, String>>,
}

impl MarkdownProcessor {
    /// Create a new MarkdownProcessor with the given options.
    pub fn new(options: MarkdownOptions) -> Self {
        let manpage_urls = options
            .manpage_urls_path
            .as_ref()
            .and_then(|path| utils::load_manpage_urls(path).ok());
        Self {
            options,
            manpage_urls,
        }
    }

    /// Render Markdown to HTML, extracting headers and title.
    pub fn render(&self, markdown: &str) -> MarkdownResult {
        // 1. Preprocess (includes, block elements, headers, inline anchors, roles)
        let preprocessed = self.preprocess(markdown);

        // 2. Extract headers and title
        let (headers, title) = self.extract_headers(&preprocessed);

        // 3. Convert to HTML
        let html = self.convert_to_html(&preprocessed);

        // 4. Process option references
        let html = process_option_references(&html);

        MarkdownResult {
            html,
            headers,
            title,
        }
    }

    /// Preprocess the markdown content (includes, block elements, headers, roles, etc).
    fn preprocess(&self, content: &str) -> String {
        // Apply the legacy markdown processing pipeline for all extensions and roles.
        use crate::legacy_markdown::{
            preprocess_block_elements, preprocess_headers, preprocess_inline_anchors,
            process_file_includes,
        };

        let with_includes = process_file_includes(content, std::path::Path::new("."));
        let preprocessed = preprocess_block_elements(&with_includes);
        let with_headers = preprocess_headers(&preprocessed);
        let with_inline_anchors = preprocess_inline_anchors(&with_headers);

        self.process_role_markup(&with_inline_anchors)
    }

    /// Extract headers and title from the markdown content.
    fn extract_headers(&self, content: &str) -> (Vec<Header>, Option<String>) {
        let arena = Arena::new();
        let options = self.comrak_options();
        let root = parse_document(&arena, content, &options);

        let mut headers = Vec::new();
        let mut found_title = None;

        for node in root.descendants() {
            if let NodeValue::Heading(NodeHeading { level, .. }) = &node.data.borrow().value {
                let text = extract_inline_text(node);
                let id = utils::slugify(&text);
                if *level == 1 && found_title.is_none() {
                    found_title = Some(text.clone());
                }
                headers.push(Header {
                    text,
                    level: *level,
                    id,
                });
            }
        }

        (headers, found_title)
    }

    /// Convert markdown to HTML using comrak and configured options.
    fn convert_to_html(&self, content: &str) -> String {
        let arena = Arena::new();
        let options = self.comrak_options();
        let root = parse_document(&arena, content, &options);

        // TODO: Apply AST transformations (e.g., prompt highlighting)
        // let transformer = ...;
        // transformer.transform(root);

        let mut html_output = Vec::new();
        comrak::format_html(root, &options, &mut html_output).unwrap_or_default();
        String::from_utf8(html_output).unwrap_or_default()
    }

    /// Build comrak options from MarkdownOptions and feature flags.
    fn comrak_options(&self) -> ComrakOptions {
        let mut options = ComrakOptions::default();
        if self.options.gfm {
            options.extension.table = true;
            options.extension.footnotes = true;
            options.extension.strikethrough = true;
            options.extension.tasklist = true;
            options.extension.superscript = true;
        }
        options.render.unsafe_ = true;
        options
    }

    /// Process role markup
    /// Handles patterns like {command}`ls -l` and {option}`services.nginx.enable`.
    fn process_role_markup(&self, content: &str) -> String {
        let mut result = String::with_capacity(content.len());
        let mut chars = content.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '{' {
                // Try to parse a role markup
                if let Some(role_markup) = self.parse_role_markup(&mut chars) {
                    result.push_str(&role_markup);
                } else {
                    // Not a valid role markup, keep the original character
                    result.push(ch);
                }
            } else {
                result.push(ch);
            }
        }

        result
    }

    /// Parse a role markup from the character iterator.
    /// Returns Some(html) if a valid role markup is found, None otherwise.
    fn parse_role_markup(
        &self,
        chars: &mut std::iter::Peekable<std::str::Chars>,
    ) -> Option<String> {
        let mut role_name = String::new();

        // Parse role name (lowercase letters only)
        while let Some(&ch) = chars.peek() {
            if ch.is_ascii_lowercase() {
                role_name.push(ch);
                chars.next();
            } else {
                break;
            }
        }

        // Must have a non-empty role name
        if role_name.is_empty() {
            return None;
        }

        // Expect closing brace
        if chars.peek() != Some(&'}') {
            return None;
        }
        chars.next(); // consume '}'

        // Expect opening backtick
        if chars.peek() != Some(&'`') {
            return None;
        }
        chars.next(); // consume '`'

        // Parse content until closing backtick
        let mut content = String::new();
        for ch in chars.by_ref() {
            if ch == '`' {
                // Found closing backtick, process the role
                return Some(self.format_role_markup(&role_name, &content));
            } else {
                content.push(ch);
            }
        }

        // No closing backtick found
        None
    }

    /// Format the role markup as HTML based on the role type and content.
    fn format_role_markup(&self, role_type: &str, content: &str) -> String {
        match role_type {
            "manpage" => {
                if let Some(ref urls) = self.manpage_urls {
                    if let Some(url) = urls.get(content) {
                        let clean_url = extract_url_from_html(url);
                        format!("<a href=\"{clean_url}\" class=\"manpage-reference\">{content}</a>")
                    } else {
                        format!("<span class=\"manpage-reference\">{content}</span>")
                    }
                } else {
                    format!("<span class=\"manpage-reference\">{content}</span>")
                }
            }
            "command" => format!("<code class=\"command\">{content}</code>"),
            "env" => format!("<code class=\"env-var\">{content}</code>"),
            "file" => format!("<code class=\"file-path\">{content}</code>"),
            "option" => format!("<code>{content}</code>"),
            "var" => format!("<code class=\"nix-var\">{content}</code>"),
            _ => format!("<span class=\"{role_type}-markup\">{content}</span>"),
        }
    }
}

/// Trait for AST transformations (e.g., prompt highlighting).
pub trait AstTransformer {
    fn transform<'a>(&self, node: &'a AstNode<'a>);
}

/// Extract all inline text from a heading node.
fn extract_inline_text<'a>(node: &'a AstNode<'a>) -> String {
    let mut text = String::new();
    for child in node.children() {
        match &child.data.borrow().value {
            NodeValue::Text(t) => text.push_str(t),
            NodeValue::Code(t) => text.push_str(&t.literal),
            NodeValue::Link(..) => text.push_str(&extract_inline_text(child)),
            NodeValue::Emph => text.push_str(&extract_inline_text(child)),
            NodeValue::Strong => text.push_str(&extract_inline_text(child)),
            NodeValue::Strikethrough => text.push_str(&extract_inline_text(child)),
            NodeValue::Superscript => text.push_str(&extract_inline_text(child)),
            NodeValue::Subscript => text.push_str(&extract_inline_text(child)),
            NodeValue::FootnoteReference(..) => text.push_str(&extract_inline_text(child)),
            NodeValue::HtmlInline(_) => {}
            NodeValue::Image(..) => {}
            _ => {}
        }
    }
    text
}

/// Process option references
/// Rewrites NixOS/Nix option references in HTML output.
///
/// This scans the HTML for `<code>option.path</code>` elements that look like NixOS/Nix option references
/// and replaces them with option reference links. Only processes plain `<code>` elements that don't
/// already have specific role classes.
///
/// # Arguments
///
/// * `html` - The HTML string to process.
///
/// # Returns
///
/// The HTML string with option references rewritten as links.
pub fn process_option_references(html: &str) -> String {
    use kuchiki::{Attribute, ExpandedName, NodeRef};
    use markup5ever::{QualName, local_name, namespace_url, ns};
    use tendril::TendrilSink;

    let document = kuchiki::parse_html().one(html);

    let mut to_replace = vec![];

    for code_node in document.select("code").unwrap() {
        let code_el = code_node.as_node();
        let code_text = code_el.text_contents();

        // Skip if this code element already has a role-specific class
        if let Some(element) = code_el.as_element() {
            if let Some(class_attr) = element.attributes.borrow().get(local_name!("class")) {
                if class_attr.contains("command")
                    || class_attr.contains("env-var")
                    || class_attr.contains("file-path")
                    || class_attr.contains("nixos-option")
                    || class_attr.contains("nix-var")
                {
                    continue;
                }
            }
        }

        // Skip if this code element is already inside an option-reference link
        let mut is_already_option_ref = false;
        let mut current = code_el.parent();
        while let Some(parent) = current {
            if let Some(element) = parent.as_element() {
                if element.name.local == local_name!("a") {
                    if let Some(class_attr) = element.attributes.borrow().get(local_name!("class"))
                    {
                        if class_attr.contains("option-reference") {
                            is_already_option_ref = true;
                            break;
                        }
                    }
                }
            }
            current = parent.parent();
        }

        if !is_already_option_ref && is_nixos_option_reference(&code_text) {
            let option_id = format!("option-{}", code_text.replace('.', "-"));
            let attrs = vec![
                (
                    ExpandedName::new("", "href"),
                    Attribute {
                        prefix: None,
                        value: format!("options.html#{option_id}"),
                    },
                ),
                (
                    ExpandedName::new("", "class"),
                    Attribute {
                        prefix: None,
                        value: "option-reference".into(),
                    },
                ),
            ];
            let a = NodeRef::new_element(QualName::new(None, ns!(html), local_name!("a")), attrs);
            let code =
                NodeRef::new_element(QualName::new(None, ns!(html), local_name!("code")), vec![]);
            code.append(NodeRef::new_text(code_text.clone()));
            a.append(code);
            to_replace.push((code_el.clone(), a));
        }
    }

    for (old, new) in to_replace {
        old.insert_before(new);
        old.detach();
    }

    let mut out = Vec::new();
    document.serialize(&mut out).ok();
    String::from_utf8(out).unwrap_or_default()
}

/// Check if a string looks like a NixOS option reference
fn is_nixos_option_reference(text: &str) -> bool {
    // Must have at least 2 dots and no whitespace
    let dot_count = text.chars().filter(|&c| c == '.').count();
    if dot_count < 2 || text.chars().any(|c| c.is_whitespace()) {
        return false;
    }

    // Must not contain special characters that indicate it's not an option
    if text.contains('<') || text.contains('>') || text.contains('$') || text.contains('/') {
        return false;
    }

    // Must start with a letter (options don't start with numbers or special chars)
    if !text.chars().next().is_some_and(|c| c.is_alphabetic()) {
        return false;
    }

    // Must look like a structured option path (letters, numbers, dots, dashes, underscores)
    text.chars()
        .all(|c| c.is_alphanumeric() || c == '.' || c == '-' || c == '_')
}

/// Extract URL from HTML anchor tag or return the string as-is if it's a plain URL
fn extract_url_from_html(url_or_html: &str) -> &str {
    // Check if it looks like HTML (starts with <a href=")
    if url_or_html.starts_with("<a href=\"") {
        // Extract the URL from href attribute
        if let Some(start) = url_or_html.find("href=\"") {
            let start = start + 6; // Skip 'href="'
            if let Some(end) = url_or_html[start..].find('"') {
                return &url_or_html[start..start + end];
            }
        }
    }

    // Return as-is if not HTML or if extraction fails
    url_or_html
}
