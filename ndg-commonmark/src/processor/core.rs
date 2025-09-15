//! Core implementation of the Markdown processor.
//!
//! This module contains the main implementation of `MarkdownProcessor` and its methods,
//! along with core processing utilities and helper functions.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use comrak::{
    Arena, ComrakOptions,
    nodes::{AstNode, NodeHeading, NodeValue},
    parse_document,
};
use log::trace;
use markup5ever::{local_name, ns};
use walkdir::WalkDir;

use super::types::{AstTransformer, MarkdownOptions, MarkdownProcessor, PromptTransformer};
use crate::{
    syntax::create_default_manager,
    types::{Header, MarkdownResult},
    utils::{self, safely_process_markup},
};

impl MarkdownProcessor {
    /// Create a new `MarkdownProcessor` with the given options.
    #[must_use]
    pub fn new(options: MarkdownOptions) -> Self {
        let manpage_urls = options
            .manpage_urls_path
            .as_ref()
            .and_then(|path| crate::utils::load_manpage_urls(path).ok());

        let syntax_manager = if options.highlight_code {
            create_default_manager().ok()
        } else {
            None
        };

        Self {
            options,
            manpage_urls,
            syntax_manager,
        }
    }

    /// Highlight all code blocks in HTML using the configured syntax highlighter
    #[must_use]
    pub fn highlight_codeblocks(&self, html: &str) -> String {
        if !self.options.highlight_code || self.syntax_manager.is_none() {
            return html.to_string();
        }

        use kuchikikiki::parse_html;
        use tendril::TendrilSink;

        let document = parse_html().one(html);

        // Collect all code blocks first to avoid DOM modification during iteration
        let mut code_blocks = Vec::new();
        for pre_node in document.select("pre > code").unwrap() {
            let code_node = pre_node.as_node();
            if let Some(element) = code_node.as_element() {
                let class_attr = element
                    .attributes
                    .borrow()
                    .get("class")
                    .map(std::string::ToString::to_string);
                let language = class_attr
                    .as_deref()
                    .and_then(|s| s.strip_prefix("language-"))
                    .unwrap_or("text");
                let code_text = code_node.text_contents();

                if let Some(pre_parent) = code_node.parent() {
                    code_blocks.push((pre_parent.clone(), code_text, language.to_string()));
                }
            }
        }

        // Process each code block
        for (pre_element, code_text, language) in code_blocks {
            if let Some(highlighted) = self.highlight_code_html(&code_text, &language) {
                // Replace the entire <pre><code>...</code></pre> with highlighted HTML
                let fragment = parse_html().one(highlighted.as_str());
                pre_element.insert_after(fragment);
                pre_element.detach();
            }
        }

        let mut buf = Vec::new();
        document.serialize(&mut buf).unwrap();
        String::from_utf8(buf).unwrap_or_default()
    }

    /// Highlight code using the configured syntax highlighter, returns HTML string
    fn highlight_code_html(&self, code: &str, language: &str) -> Option<String> {
        if !self.options.highlight_code {
            return None;
        }

        let syntax_manager = self.syntax_manager.as_ref()?;

        syntax_manager
            .highlight_code(code, language, self.options.highlight_theme.as_deref())
            .ok()
    }

    /// Render Markdown to HTML, extracting headers and title.
    #[must_use]
    pub fn render(&self, markdown: &str) -> MarkdownResult {
        // 1. Preprocess (includes, block elements, headers, inline anchors, roles)
        let preprocessed = self.preprocess(markdown);

        // 2. Extract headers and title
        let (headers, title) = self.extract_headers(&preprocessed);

        // 3. Convert to HTML
        let html = self.convert_to_html(&preprocessed);

        // 4. Process option references
        let html = if cfg!(feature = "ndg-flavored") {
            #[cfg(feature = "ndg-flavored")]
            {
                super::extensions::process_option_references(&html)
            }
            #[cfg(not(feature = "ndg-flavored"))]
            {
                html
            }
        } else {
            html
        };

        // 5. Autolinks are handled natively by comrak when GFM is enabled

        // 6. Post-process manpage references
        let html = if self.options.nixpkgs {
            self.process_manpage_references_html(&html)
        } else {
            html
        };

        // 7. Apply syntax highlighting to code blocks
        let html = if self.options.highlight_code {
            self.highlight_codeblocks(&html)
        } else {
            html
        };

        // 8. Complete HTML post-processing
        let html = self.kuchiki_postprocess(&html);

        MarkdownResult {
            html,
            headers,
            title,
        }
    }

    /// Preprocess the markdown content (includes, block elements, headers, roles, etc).
    fn preprocess(&self, content: &str) -> String {
        // 1. Process file includes if nixpkgs feature is enabled
        let with_includes = if self.options.nixpkgs {
            #[cfg(feature = "nixpkgs")]
            {
                super::extensions::process_file_includes(content, std::path::Path::new("."))
            }
            #[cfg(not(feature = "nixpkgs"))]
            {
                content.to_string()
            }
        } else {
            content.to_string()
        };

        // 2. Process block elements (admonitions, figures, definition lists)
        let preprocessed = if self.options.nixpkgs {
            super::extensions::process_block_elements(&with_includes)
        } else {
            with_includes
        };

        // 3. Process inline anchors
        let with_inline_anchors = if self.options.nixpkgs {
            super::extensions::process_inline_anchors(&preprocessed)
        } else {
            preprocessed
        };

        // 4. Process role markup
        if self.options.nixpkgs || cfg!(feature = "ndg-flavored") {
            super::extensions::process_role_markup(&with_inline_anchors, self.manpage_urls.as_ref())
        } else {
            with_inline_anchors
        }
    }

    /// Extract headers and title from the markdown content.
    #[must_use]
    pub fn extract_headers(&self, content: &str) -> (Vec<Header>, Option<String>) {
        let arena = Arena::new();
        let options = self.comrak_options();

        // Normalize custom anchors with no heading level to h2
        let mut normalized = String::with_capacity(content.len());
        for line in content.lines() {
            let trimmed = line.trim_end();
            if !trimmed.starts_with('#') {
                if let Some(anchor_start) = trimmed.rfind("{#") {
                    if let Some(anchor_end) = trimmed[anchor_start..].find('}') {
                        let text = trimmed[..anchor_start].trim_end();
                        let id = &trimmed[anchor_start + 2..anchor_start + anchor_end];
                        normalized.push_str(&format!("## {text} {{#{id}}}\n"));
                        continue;
                    }
                }
            }
            normalized.push_str(line);
            normalized.push('\n');
        }

        let root = parse_document(&arena, &normalized, &options);

        let mut headers = Vec::new();
        let mut found_title = None;

        for node in root.descendants() {
            if let NodeValue::Heading(NodeHeading { level, .. }) = &node.data.borrow().value {
                let mut text = String::new();
                let mut explicit_id = None;

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
                        NodeValue::FootnoteReference(..) => {
                            text.push_str(&extract_inline_text(child));
                        }
                        NodeValue::HtmlInline(html) => {
                            // Look for explicit anchor in HTML inline node: {#id}
                            let html_str = html.as_str();
                            if let Some(start) = html_str.find("{#") {
                                if let Some(end) = html_str[start..].find('}') {
                                    let anchor = &html_str[start + 2..start + end];
                                    explicit_id = Some(anchor.to_string());
                                }
                            }
                        }
                        NodeValue::Image(..) => {}
                        _ => {}
                    }
                }

                // Check for trailing {#id} in heading text
                let trimmed = text.trim_end();
                let (final_text, id) = if let Some(start) = trimmed.rfind("{#") {
                    if let Some(end) = trimmed[start..].find('}') {
                        let anchor = &trimmed[start + 2..start + end];
                        (trimmed[..start].trim_end().to_string(), anchor.to_string())
                    } else {
                        (
                            text.clone(),
                            explicit_id.unwrap_or_else(|| utils::slugify(&text)),
                        )
                    }
                } else {
                    (
                        text.clone(),
                        explicit_id.unwrap_or_else(|| utils::slugify(&text)),
                    )
                };
                if *level == 1 && found_title.is_none() {
                    found_title = Some(final_text.clone());
                }
                headers.push(Header {
                    text: final_text,
                    level: *level,
                    id,
                });
            }
        }

        (headers, found_title)
    }

    /// Convert markdown to HTML using comrak and configured options.
    fn convert_to_html(&self, content: &str) -> String {
        safely_process_markup(
            content,
            |content| {
                let arena = Arena::new();
                let options = self.comrak_options();
                let root = parse_document(&arena, content, &options);

                // Apply AST transformations
                let prompt_transformer = PromptTransformer;
                prompt_transformer.transform(root);

                let mut html_output = Vec::new();
                comrak::format_html(root, &options, &mut html_output).unwrap_or_default();
                let html = String::from_utf8(html_output).unwrap_or_default();

                // Post-process HTML to handle header anchors
                self.process_header_anchors_html(&html)
            },
            "<div class=\"error\">Error processing markdown content</div>",
        )
    }

    /// Process header anchors in HTML by finding {#id} syntax and converting to proper id attributes
    fn process_header_anchors_html(&self, html: &str) -> String {
        use std::sync::LazyLock;

        use regex::Regex;

        static HEADER_ANCHOR_RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"<h([1-6])>(.*?)\s*\{#([a-zA-Z0-9_-]+)\}(.*?)</h[1-6]>").unwrap_or_else(
                |e| {
                    log::error!("Failed to compile HEADER_ANCHOR_RE regex: {e}");
                    utils::never_matching_regex()
                },
            )
        });

        HEADER_ANCHOR_RE
            .replace_all(html, |caps: &regex::Captures| {
                let level = &caps[1];
                let prefix = &caps[2];
                let id = &caps[3];
                let suffix = &caps[4];
                format!("<h{level} id=\"{id}\">{prefix}{suffix}</h{level}>")
            })
            .to_string()
    }

    /// Build comrak options from `MarkdownOptions` and feature flags.
    fn comrak_options(&self) -> ComrakOptions<'_> {
        let mut options = ComrakOptions::default();
        if self.options.gfm {
            options.extension.table = true;
            options.extension.footnotes = true;
            options.extension.strikethrough = true;
            options.extension.tasklist = true;
            options.extension.superscript = true;
            options.extension.autolink = true;
        }
        options.render.unsafe_ = true;
        // Disable automatic header ID generation - we handle anchors manually
        options.extension.header_ids = None;
        options
    }

    /// Get the manpage URLs mapping for use with standalone functions.
    #[must_use]
    pub fn manpage_urls(&self) -> Option<&HashMap<String, String>> {
        self.manpage_urls.as_ref()
    }

    /// Post-process HTML to enhance manpage references with URL links.
    /// This finds <span class="manpage-reference"> elements and converts them to links when URLs are available.
    #[cfg(feature = "nixpkgs")]
    fn process_manpage_references_html(&self, html: &str) -> String {
        super::extensions::process_manpage_references(html, self.manpage_urls.as_ref())
    }

    /// HTML post-processing using kuchiki DOM manipulation.
    fn kuchiki_postprocess(&self, html: &str) -> String {
        safely_process_markup(
            html,
            |html| {
                use tendril::TendrilSink;

                let document = kuchikikiki::parse_html().one(html);

                // Process list item ID markers: <li><!-- nixos-anchor-id:ID -->
                self.process_list_item_id_markers(&document);

                // Process header anchors with comments: <h1>text<!-- anchor: id --></h1>
                self.process_header_anchor_comments(&document);

                // Process remaining inline anchors in list items: <li>[]{#id}content</li>
                self.process_list_item_inline_anchors(&document);

                // Process inline anchors in paragraphs: <p>[]{#id}content</p>
                self.process_paragraph_inline_anchors(&document);

                // Process remaining standalone inline anchors
                self.process_remaining_inline_anchors(&document);

                // Process empty auto-links: [](#anchor)
                self.process_empty_auto_links(&document);

                // Process empty HTML links: <a href="#anchor"></a>
                self.process_empty_html_links(&document);

                let mut out = Vec::new();
                document.serialize(&mut out).ok();
                String::from_utf8(out).unwrap_or_default()
            },
            // Return original HTML on error
            "",
        )
    }

    /// Process list item ID markers: <li><!-- nixos-anchor-id:ID -->
    fn process_list_item_id_markers(&self, document: &kuchikikiki::NodeRef) {
        let mut to_modify = Vec::new();

        for comment in document.inclusive_descendants() {
            if let Some(comment_node) = comment.as_comment() {
                let comment_text = comment_node.borrow();
                if let Some(id_start) = comment_text.find("nixos-anchor-id:") {
                    let id = comment_text[id_start + 16..].trim();
                    if !id.is_empty()
                        && id
                            .chars()
                            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
                    {
                        // Check if this comment is inside an <li> element
                        if let Some(parent) = comment.parent() {
                            if let Some(element) = parent.as_element() {
                                if element.name.local.as_ref() == "li" {
                                    to_modify.push((comment.clone(), id.to_string()));
                                }
                            }
                        }
                    }
                }
            }
        }

        for (comment_node, id) in to_modify {
            let span = kuchikikiki::NodeRef::new_element(
                markup5ever::QualName::new(None, ns!(html), local_name!("span")),
                vec![
                    (
                        kuchikikiki::ExpandedName::new("", "id"),
                        kuchikikiki::Attribute {
                            prefix: None,
                            value: id,
                        },
                    ),
                    (
                        kuchikikiki::ExpandedName::new("", "class"),
                        kuchikikiki::Attribute {
                            prefix: None,
                            value: "nixos-anchor".into(),
                        },
                    ),
                ],
            );
            comment_node.insert_after(span);
            comment_node.detach();
        }
    }

    /// Process header anchors with comments: <h1>text<!-- anchor: id --></h1>
    fn process_header_anchor_comments(&self, document: &kuchikikiki::NodeRef) {
        let mut to_modify = Vec::new();

        for comment in document.inclusive_descendants() {
            if let Some(comment_node) = comment.as_comment() {
                let comment_text = comment_node.borrow();
                if let Some(anchor_start) = comment_text.find("anchor:") {
                    let id = comment_text[anchor_start + 7..].trim();
                    if !id.is_empty()
                        && id
                            .chars()
                            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
                    {
                        // Check if this comment is inside a header element
                        if let Some(parent) = comment.parent() {
                            if let Some(element) = parent.as_element() {
                                let tag_name = element.name.local.as_ref();
                                if matches!(tag_name, "h1" | "h2" | "h3" | "h4" | "h5" | "h6") {
                                    to_modify.push((
                                        parent.clone(),
                                        comment.clone(),
                                        id.to_string(),
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }

        for (header_element, comment_node, id) in to_modify {
            if let Some(element) = header_element.as_element() {
                element
                    .attributes
                    .borrow_mut()
                    .insert(local_name!("id"), id);
                comment_node.detach();
            }
        }
    }

    /// Process remaining inline anchors in list items: <li>[]{#id}content</li>
    fn process_list_item_inline_anchors(&self, document: &kuchikikiki::NodeRef) {
        for li_node in document.select("li").unwrap() {
            let li_element = li_node.as_node();

            // Check if this list item contains code elements
            let has_code = li_element.select("code, pre").is_ok()
                && li_element.select("code, pre").unwrap().next().is_some();
            if has_code {
                continue; // Skip list items with code blocks
            }

            let text_content = li_element.text_contents();

            if let Some(anchor_start) = text_content.find("[]{#") {
                if let Some(anchor_end) = text_content[anchor_start..].find('}') {
                    let id = &text_content[anchor_start + 4..anchor_start + anchor_end];
                    if !id.is_empty()
                        && id
                            .chars()
                            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
                    {
                        let remaining_content = &text_content[anchor_start + anchor_end + 1..];

                        // Clear current content and rebuild
                        for child in li_element.children() {
                            child.detach();
                        }

                        let span = kuchikikiki::NodeRef::new_element(
                            markup5ever::QualName::new(None, ns!(html), local_name!("span")),
                            vec![
                                (
                                    kuchikikiki::ExpandedName::new("", "id"),
                                    kuchikikiki::Attribute {
                                        prefix: None,
                                        value: id.into(),
                                    },
                                ),
                                (
                                    kuchikikiki::ExpandedName::new("", "class"),
                                    kuchikikiki::Attribute {
                                        prefix: None,
                                        value: "nixos-anchor".into(),
                                    },
                                ),
                            ],
                        );
                        li_element.append(span);
                        if !remaining_content.is_empty() {
                            li_element.append(kuchikikiki::NodeRef::new_text(remaining_content));
                        }
                    }
                }
            }
        }
    }

    /// Process inline anchors in paragraphs: <p>[]{#id}content</p>
    fn process_paragraph_inline_anchors(&self, document: &kuchikikiki::NodeRef) {
        for p_node in document.select("p").unwrap() {
            let p_element = p_node.as_node();

            // Check if this paragraph contains code elements
            let has_code = p_element.select("code, pre").is_ok()
                && p_element.select("code, pre").unwrap().next().is_some();
            if has_code {
                continue; // Skip paragraphs with code blocks
            }

            let text_content = p_element.text_contents();

            if let Some(anchor_start) = text_content.find("[]{#") {
                if let Some(anchor_end) = text_content[anchor_start..].find('}') {
                    let id = &text_content[anchor_start + 4..anchor_start + anchor_end];
                    if !id.is_empty()
                        && id
                            .chars()
                            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
                    {
                        let remaining_content = &text_content[anchor_start + anchor_end + 1..];

                        // Clear current content and rebuild
                        for child in p_element.children() {
                            child.detach();
                        }

                        let span = kuchikikiki::NodeRef::new_element(
                            markup5ever::QualName::new(None, ns!(html), local_name!("span")),
                            vec![
                                (
                                    kuchikikiki::ExpandedName::new("", "id"),
                                    kuchikikiki::Attribute {
                                        prefix: None,
                                        value: id.into(),
                                    },
                                ),
                                (
                                    kuchikikiki::ExpandedName::new("", "class"),
                                    kuchikikiki::Attribute {
                                        prefix: None,
                                        value: "nixos-anchor".into(),
                                    },
                                ),
                            ],
                        );
                        p_element.append(span);
                        if !remaining_content.is_empty() {
                            p_element.append(kuchikikiki::NodeRef::new_text(remaining_content));
                        }
                    }
                }
            }
        }
    }

    /// Process remaining standalone inline anchors throughout the document
    fn process_remaining_inline_anchors(&self, document: &kuchikikiki::NodeRef) {
        let mut text_nodes_to_process = Vec::new();

        for node in document.inclusive_descendants() {
            if let Some(text_node) = node.as_text() {
                // Check if this text node is inside a code block
                let mut parent = node.parent();
                let mut in_code = false;
                while let Some(p) = parent {
                    if let Some(element) = p.as_element() {
                        if element.name.local == local_name!("code")
                            || element.name.local == local_name!("pre")
                        {
                            in_code = true;
                            break;
                        }
                    }
                    parent = p.parent();
                }

                // Only process if not in code
                if !in_code {
                    let text_content = text_node.borrow().clone();
                    if text_content.contains("[]{#") {
                        text_nodes_to_process.push((node.clone(), text_content));
                    }
                }
            }
        }

        for (text_node, text_content) in text_nodes_to_process {
            let mut last_end = 0;
            let mut new_children = Vec::new();

            // Simple pattern matching for []{#id}
            let chars = text_content.chars().collect::<Vec<_>>();
            let mut i = 0;
            while i < chars.len() {
                if i + 4 < chars.len()
                    && chars[i] == '['
                    && chars[i + 1] == ']'
                    && chars[i + 2] == '{'
                    && chars[i + 3] == '#'
                {
                    // Found start of anchor pattern
                    let anchor_start = i;
                    i += 4; // skip "[]{#"

                    let mut id = String::new();
                    while i < chars.len() && chars[i] != '}' {
                        if chars[i].is_alphanumeric() || chars[i] == '-' || chars[i] == '_' {
                            id.push(chars[i]);
                            i += 1;
                        } else {
                            break;
                        }
                    }

                    if i < chars.len() && chars[i] == '}' && !id.is_empty() {
                        // Valid anchor found
                        let anchor_end = i + 1;

                        // Add text before anchor
                        if anchor_start > last_end {
                            let before_text: String =
                                chars[last_end..anchor_start].iter().collect();
                            if !before_text.is_empty() {
                                new_children.push(kuchikikiki::NodeRef::new_text(before_text));
                            }
                        }

                        // Add span element
                        let span = kuchikikiki::NodeRef::new_element(
                            markup5ever::QualName::new(None, ns!(html), local_name!("span")),
                            vec![
                                (
                                    kuchikikiki::ExpandedName::new("", "id"),
                                    kuchikikiki::Attribute {
                                        prefix: None,
                                        value: id,
                                    },
                                ),
                                (
                                    kuchikikiki::ExpandedName::new("", "class"),
                                    kuchikikiki::Attribute {
                                        prefix: None,
                                        value: "nixos-anchor".into(),
                                    },
                                ),
                            ],
                        );
                        new_children.push(span);

                        last_end = anchor_end;
                        i = anchor_end;
                    } else {
                        i += 1;
                    }
                } else {
                    i += 1;
                }
            }

            // Add remaining text
            if last_end < chars.len() {
                let after_text: String = chars[last_end..].iter().collect();
                if !after_text.is_empty() {
                    new_children.push(kuchikikiki::NodeRef::new_text(after_text));
                }
            }

            // Replace text node if we found anchors
            if !new_children.is_empty() {
                for child in new_children {
                    text_node.insert_before(child);
                }
                text_node.detach();
            }
        }
    }

    /// Process empty auto-links: [](#anchor) -> <a href="#anchor">Anchor</a>
    fn process_empty_auto_links(&self, document: &kuchikikiki::NodeRef) {
        for link_node in document.select("a").unwrap() {
            let link_element = link_node.as_node();
            if let Some(element) = link_element.as_element() {
                let href = element
                    .attributes
                    .borrow()
                    .get(local_name!("href"))
                    .map(std::string::ToString::to_string);
                let text_content = link_element.text_contents();

                if let Some(href_value) = href {
                    if href_value.starts_with('#') && text_content.trim().is_empty() {
                        // Empty link with anchor - add humanized text
                        let display_text = self.humanize_anchor_id(&href_value);
                        link_element.append(kuchikikiki::NodeRef::new_text(display_text));
                    }
                }
            }
        }
    }

    /// Process empty HTML links that have no content
    fn process_empty_html_links(&self, document: &kuchikikiki::NodeRef) {
        for link_node in document.select("a[href^='#']").unwrap() {
            let link_element = link_node.as_node();
            let text_content = link_element.text_contents();

            if text_content.trim().is_empty() {
                if let Some(element) = link_element.as_element() {
                    if let Some(href) = element.attributes.borrow().get(local_name!("href")) {
                        let display_text = self.humanize_anchor_id(href);
                        link_element.append(kuchikikiki::NodeRef::new_text(display_text));
                    }
                }
            }
        }
    }

    /// Convert an anchor ID to human-readable text
    fn humanize_anchor_id(&self, anchor: &str) -> String {
        // Strip the leading #
        let cleaned = anchor.trim_start_matches('#');

        // Remove common prefixes
        let without_prefix = cleaned
            .trim_start_matches("sec-")
            .trim_start_matches("ssec-")
            .trim_start_matches("opt-");

        // Replace separators with spaces
        let spaced = without_prefix.replace(['-', '_'], " ");

        // Capitalize each word
        spaced
            .split_whitespace()
            .map(|word| {
                let mut chars = word.chars();
                chars.next().map_or_else(String::new, |c| {
                    c.to_uppercase().collect::<String>() + chars.as_str()
                })
            })
            .collect::<Vec<String>>()
            .join(" ")
    }
}

/// Extract all inline text from a heading node.
pub fn extract_inline_text<'a>(node: &'a AstNode<'a>) -> String {
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

/// Collect all markdown files from the input directory
pub fn collect_markdown_files(input_dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::with_capacity(100);

    for entry in WalkDir::new(input_dir)
        .follow_links(true)
        .into_iter()
        .filter_map(Result::ok)
    {
        let path = entry.path();
        if path.is_file() && path.extension().is_some_and(|ext| ext == "md") {
            files.push(path.to_owned());
        }
    }

    trace!("Found {} markdown files to process", files.len());
    files
}
