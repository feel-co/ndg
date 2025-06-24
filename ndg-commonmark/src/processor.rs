use std::collections::HashMap;

use comrak::{
    Arena, ComrakOptions,
    nodes::{AstNode, NodeHeading, NodeValue},
    parse_document,
};

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
            process_file_includes, preprocess_block_elements, preprocess_headers,
            preprocess_inline_anchors, process_role_markup,
        };

        let with_includes = process_file_includes(content);
        let preprocessed = preprocess_block_elements(&with_includes);
        let with_headers = preprocess_headers(&preprocessed);
        let with_inline_anchors = preprocess_inline_anchors(&with_headers);
        let with_roles = process_role_markup(&with_inline_anchors, self.manpage_urls.as_ref());

        with_roles
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
