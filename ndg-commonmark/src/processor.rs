pub use std::collections::HashMap;
use std::path::{Path, PathBuf};

use comrak::{
    Arena, ComrakOptions,
    nodes::{AstNode, NodeHeading, NodeValue},
    parse_document,
};
use log::trace;
use markup5ever::{local_name, namespace_url, ns};
use walkdir::WalkDir;

use crate::{
    types::{Header, MarkdownResult},
    utils::{self, safely_process_markup},
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
    /// Create a new `MarkdownProcessor` with the given options.
    #[must_use]
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
    #[must_use]
    pub fn render(&self, markdown: &str) -> MarkdownResult {
        // 1. Preprocess (includes, block elements, headers, inline anchors, roles)
        let preprocessed = self.preprocess(markdown);

        // 2. Extract headers and title
        let (headers, title) = self.extract_headers(&preprocessed);

        // 3. Convert to HTML
        let html = self.convert_to_html(&preprocessed);

        // 4. Process option references
        let html = process_option_references(&html);

        // 5. Process autolinks
        let html = self.process_autolinks(&html);

        // 6. Post-process manpage references
        let html = self.process_manpage_references_html(&html);

        // 7. Complete HTML post-processing
        let html = self.kuchiki_postprocess(&html);

        MarkdownResult {
            html,
            headers,
            title,
        }
    }

    /// Preprocess the markdown content (includes, block elements, headers, roles, etc).
    fn preprocess(&self, content: &str) -> String {
        // 1. Process file includes using new extensions implementation
        let with_includes = if self.options.nixpkgs {
            #[cfg(feature = "nixpkgs")]
            {
                crate::extensions::apply_nixpkgs_extensions(content, std::path::Path::new("."))
            }
            #[cfg(not(feature = "nixpkgs"))]
            {
                content.to_string()
            }
        } else {
            content.to_string()
        };

        // 2. Process block elements (admonitions, figures, definition lists)
        let preprocessed = self.process_block_elements(&with_includes);

        // 3. Process inline anchors
        let with_inline_anchors = self.process_inline_anchors(&preprocessed);

        // 4. Process role markup
        self.process_role_markup(&with_inline_anchors)
    }

    /// Process inline anchors by converting []{#id} syntax to HTML spans.
    /// Also handles list items with anchors at the beginning.
    fn process_inline_anchors(&self, content: &str) -> String {
        let mut result = String::with_capacity(content.len() + 100);

        for line in content.lines() {
            let trimmed = line.trim_start();

            // Check for list items with anchors:
            // "- []{#id} content" or "1. []{#id} content"
            if let Some(anchor_start) = Self::find_list_item_anchor(trimmed) {
                if let Some(processed_line) = Self::process_list_item_anchor(line, anchor_start) {
                    result.push_str(&processed_line);
                    result.push('\n');
                    continue;
                }
            }

            // Process regular inline anchors in the line
            result.push_str(&Self::process_line_anchors(line));
            result.push('\n');
        }

        result
    }

    /// Find if a line starts with a list marker followed by an anchor.
    fn find_list_item_anchor(trimmed: &str) -> Option<usize> {
        // Check for unordered list: "- []{#id}" or "* []{#id}" or "+ []{#id}"
        if (trimmed.starts_with("- ") || trimmed.starts_with("* ") || trimmed.starts_with("+ "))
            && trimmed.len() > 2
        {
            let after_marker = &trimmed[2..];
            if after_marker.starts_with("[]{#") {
                return Some(2);
            }
        }

        // Check for ordered list: "1. []{#id}" or "123. []{#id}"
        let mut i = 0;
        while i < trimmed.len() && trimmed.chars().nth(i).unwrap_or(' ').is_ascii_digit() {
            i += 1;
        }
        if i > 0 && i < trimmed.len() - 1 && trimmed.chars().nth(i) == Some('.') {
            let after_marker = &trimmed[i + 1..];
            if after_marker.starts_with(" []{#") {
                return Some(i + 2);
            }
        }

        None
    }

    /// Process a list item line that contains an anchor.
    fn process_list_item_anchor(line: &str, anchor_start: usize) -> Option<String> {
        let before_anchor = &line[..anchor_start];
        let after_marker = &line[anchor_start..];

        if !after_marker.starts_with("[]{#") {
            return None;
        }

        // Find the end of the anchor: []{#id}
        if let Some(anchor_end) = after_marker.find('}') {
            let id = &after_marker[4..anchor_end]; // skip "[]{#" and take until '}'
            let remaining_content = &after_marker[anchor_end + 1..]; // skip '}'

            // Validate ID contains only allowed characters
            if id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
                && !id.is_empty()
            {
                return Some(format!(
                    "{before_anchor}<span id=\"{id}\" class=\"nixos-anchor\"></span>{remaining_content}"
                ));
            }
        }

        None
    }

    /// Process inline anchors in a single line.
    fn process_line_anchors(line: &str) -> String {
        let mut result = String::with_capacity(line.len());
        let mut chars = line.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '[' && chars.peek() == Some(&']') {
                chars.next(); // consume ']'

                // Check for {#id} pattern
                if chars.peek() == Some(&'{') {
                    chars.next(); // consume '{'
                    if chars.peek() == Some(&'#') {
                        chars.next(); // consume '#'

                        // Collect the ID
                        let mut id = String::new();
                        while let Some(&next_ch) = chars.peek() {
                            if next_ch == '}' {
                                chars.next(); // consume '}'

                                // Validate ID and create span
                                if !id.is_empty()
                                    && id
                                        .chars()
                                        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
                                {
                                    result.push_str(&format!(
                                        "<span id=\"{id}\" class=\"nixos-anchor\"></span>"
                                    ));
                                } else {
                                    // Invalid ID, put back original text
                                    result.push_str(&format!("[]{{{{#{id}}}}}"));
                                }
                                break;
                            } else if next_ch.is_ascii_alphanumeric()
                                || next_ch == '-'
                                || next_ch == '_'
                            {
                                id.push(next_ch);
                                chars.next();
                            } else {
                                // Invalid character, put back original text
                                result.push_str(&format!("[]{{{{#{id}"));
                                break;
                            }
                        }
                    } else {
                        // Not an anchor, put back consumed characters
                        result.push_str("]{");
                    }
                } else {
                    // Not an anchor, put back consumed character
                    result.push(']');
                }
            } else {
                result.push(ch);
            }
        }

        result
    }

    /// Process block elements including admonitions, figures, and definition lists.
    fn process_block_elements(&self, content: &str) -> String {
        let mut result = Vec::new();
        let mut lines = content.lines().peekable();

        while let Some(line) = lines.next() {
            // Check for GitHub-style callouts: > [!TYPE]
            if let Some((callout_type, initial_content)) = Self::parse_github_callout(line) {
                let content = self.collect_github_callout_content(&mut lines, initial_content);
                let admonition = Self::render_admonition(&callout_type, None, &content);
                result.push(admonition);
                continue;
            }

            // Check for fenced admonitions: ::: {.type}
            if let Some((adm_type, id)) = Self::parse_fenced_admonition_start(line) {
                let content = self.collect_fenced_content(&mut lines);
                let admonition = Self::render_admonition(&adm_type, id.as_deref(), &content);
                result.push(admonition);
                continue;
            }

            // Check for figures: ::: {.figure #id}
            if let Some((id, title, content)) = Self::parse_figure_block(line, &mut lines) {
                let figure = Self::render_figure(id.as_deref(), &title, &content);
                result.push(figure);
                continue;
            }

            // Check for definition lists: Term\n:   Definition
            if !line.is_empty() && !line.starts_with(':') {
                if let Some(next_line) = lines.peek() {
                    if next_line.starts_with(":   ") {
                        let term = line;
                        let def_line = lines.next().unwrap();
                        let definition = &def_line[4..]; // Skip ":   "
                        let dl = format!("<dl>\n<dt>{term}</dt>\n<dd>{definition}</dd>\n</dl>");
                        result.push(dl);
                        continue;
                    }
                }
            }

            // Regular line, keep as-is
            result.push(line.to_string());
        }

        result.join("\n")
    }

    /// Parse GitHub-style callout syntax: > [!TYPE] content
    fn parse_github_callout(line: &str) -> Option<(String, String)> {
        let trimmed = line.trim_start();
        if !trimmed.starts_with("> [!") {
            return None;
        }

        // Find the closing bracket
        if let Some(close_bracket) = trimmed.find(']') {
            if close_bracket > 4 {
                let callout_type = &trimmed[4..close_bracket];

                // Validate callout type
                match callout_type {
                    "NOTE" | "TIP" | "IMPORTANT" | "WARNING" | "CAUTION" | "DANGER" => {
                        let content = trimmed[close_bracket + 1..].trim();
                        return Some((callout_type.to_lowercase(), content.to_string()));
                    }
                    _ => return None,
                }
            }
        }

        None
    }

    /// Collect content for GitHub-style callouts
    fn collect_github_callout_content(
        &self,
        lines: &mut std::iter::Peekable<std::str::Lines>,
        initial_content: String,
    ) -> String {
        let mut content = String::new();

        if !initial_content.is_empty() {
            content.push_str(&initial_content);
            content.push('\n');
        }

        while let Some(line) = lines.peek() {
            let trimmed = line.trim_start();
            if trimmed.starts_with('>') {
                let content_part = trimmed.strip_prefix('>').unwrap_or("").trim_start();
                content.push_str(content_part);
                content.push('\n');
                lines.next(); // consume the line
            } else {
                break;
            }
        }

        content.trim().to_string()
    }

    /// Parse fenced admonition start: ::: {.type #id}
    fn parse_fenced_admonition_start(line: &str) -> Option<(String, Option<String>)> {
        let trimmed = line.trim();
        if !trimmed.starts_with(":::") {
            return None;
        }

        let after_colons = trimmed[3..].trim_start();
        if !after_colons.starts_with("{.") {
            return None;
        }

        // Find the closing brace
        if let Some(close_brace) = after_colons.find('}') {
            let content = &after_colons[2..close_brace]; // Skip "{."

            // Parse type and optional ID
            let parts: Vec<&str> = content.split_whitespace().collect();
            if let Some(&adm_type) = parts.first() {
                let id = parts
                    .iter()
                    .find(|part| part.starts_with('#'))
                    .map(|id_part| id_part[1..].to_string()); // Remove '#'

                return Some((adm_type.to_string(), id));
            }
        }

        None
    }

    /// Collect content until closing :::
    fn collect_fenced_content(&self, lines: &mut std::iter::Peekable<std::str::Lines>) -> String {
        let mut content = String::new();

        for line in lines.by_ref() {
            if line.trim().starts_with(":::") {
                break;
            }
            content.push_str(line);
            content.push('\n');
        }

        content.trim().to_string()
    }

    /// Parse figure block: ::: {.figure #id}
    fn parse_figure_block(
        line: &str,
        lines: &mut std::iter::Peekable<std::str::Lines>,
    ) -> Option<(Option<String>, String, String)> {
        let trimmed = line.trim();
        if !trimmed.starts_with(":::") {
            return None;
        }

        let after_colons = trimmed[3..].trim_start();
        if !after_colons.starts_with("{.figure") {
            return None;
        }

        // Extract ID if present
        let id = if let Some(hash_pos) = after_colons.find('#') {
            if let Some(close_brace) = after_colons.find('}') {
                if hash_pos < close_brace {
                    Some(after_colons[hash_pos + 1..close_brace].trim().to_string())
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        // Get title from next line (should start with #)
        let title = if let Some(title_line) = lines.next() {
            let trimmed_title = title_line.trim();
            if let Some(this) = trimmed_title.strip_prefix('#') {
                { this.trim_matches(char::is_whitespace) }.to_string()
            } else {
                // Put the line back if it's not a title
                return None;
            }
        } else {
            return None;
        };

        // Collect figure content
        let mut content = String::new();
        for line in lines.by_ref() {
            if line.trim().starts_with(":::") {
                break;
            }
            content.push_str(line);
            content.push('\n');
        }

        Some((id, title, content.trim().to_string()))
    }

    /// Render an admonition as HTML
    fn render_admonition(adm_type: &str, id: Option<&str>, content: &str) -> String {
        let capitalized_type = crate::utils::capitalize_first(adm_type);
        let id_attr = id.map_or(String::new(), |id| format!(" id=\"{id}\""));

        format!(
            "<div class=\"admonition {adm_type}\"{id_attr}>\n<p class=\"admonition-title\">{capitalized_type}</p>\n{content}\n</div>"
        )
    }

    /// Render a figure as HTML
    fn render_figure(id: Option<&str>, title: &str, content: &str) -> String {
        let id_attr = id.map_or(String::new(), |id| format!(" id=\"{id}\""));

        format!("<figure{id_attr}>\n<figcaption>{title}</figcaption>\n{content}\n</figure>")
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
        // Disable automatic header ID generation - we handle anchors manually
        options.extension.header_ids = None;
        options
    }

    /// Process role markup
    /// Handles patterns like {command}`ls -l` and {option}`services.nginx.enable`.
    #[must_use]
    pub fn process_role_markup(&self, content: &str) -> String {
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
            }
            content.push(ch);
        }

        // No closing backtick found
        None
    }

    /// Format the role markup as HTML based on the role type and content.
    #[must_use]
    pub fn format_role_markup(&self, role_type: &str, content: &str) -> String {
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
            "option" => {
                if cfg!(feature = "ndg-flavored") {
                    let option_id = format!("option-{}", content.replace('.', "-"));
                    format!(
                        "<a class=\"option-reference\" href=\"options.html#{option_id}\"><code>{content}</code></a>"
                    )
                } else {
                    format!("<code>{content}</code>")
                }
            }
            "var" => format!("<code class=\"nix-var\">{content}</code>"),
            _ => format!("<span class=\"{role_type}-markup\">{content}</span>"),
        }
    }

    /// Process autolinks by converting plain URLs to HTML anchor tags.
    /// This searches for URLs in text nodes and converts them to clickable links.
    fn process_autolinks(&self, html: &str) -> String {
        safely_process_markup(
            html,
            |html| {
                use std::sync::LazyLock;

                use kuchiki::NodeRef;
                use regex::Regex;
                use tendril::TendrilSink;

                static AUTOLINK_RE: LazyLock<Regex> = LazyLock::new(|| {
                    Regex::new(r#"(https?://[^\s<>"')\}]+)"#).unwrap_or_else(|e| {
                        log::error!("Failed to compile AUTOLINK_RE regex: {e}");
                        utils::never_matching_regex()
                    })
                });

                let document = kuchiki::parse_html().one(html);

                // Find all text nodes that aren't already inside links
                let mut text_nodes_to_process = Vec::new();

                for node in document.inclusive_descendants() {
                    if let Some(text_node) = node.as_text() {
                        let text_content = text_node.borrow().clone();

                        // Skip if this text node is inside a link already
                        let mut is_inside_link = false;
                        let mut current = Some(node.clone());
                        while let Some(parent) = current.and_then(|n| n.parent()) {
                            if let Some(element) = parent.as_element() {
                                if element.name.local.as_ref() == "a" {
                                    is_inside_link = true;
                                    break;
                                }
                            }
                            current = parent.parent();
                        }

                        if !is_inside_link && AUTOLINK_RE.is_match(&text_content) {
                            text_nodes_to_process.push((node.clone(), text_content));
                        }
                    }
                }

                // Process each text node that contains URLs
                for (text_node, text_content) in text_nodes_to_process {
                    let mut last_end = 0;
                    let mut new_children = Vec::new();

                    for url_match in AUTOLINK_RE.find_iter(&text_content) {
                        // Add text before the URL
                        if url_match.start() > last_end {
                            let before_text = &text_content[last_end..url_match.start()];
                            if !before_text.is_empty() {
                                new_children.push(NodeRef::new_text(before_text));
                            }
                        }

                        // Create link for the URL, trimming trailing punctuation
                        let mut url = url_match.as_str();

                        // Trim common trailing punctuation
                        while let Some(last_char) = url.chars().last() {
                            if matches!(last_char, '.' | '!' | '?' | ';' | ',' | ')' | ']' | '}') {
                                url = &url[..url.len() - last_char.len_utf8()];
                            } else {
                                break;
                            }
                        }

                        let link = NodeRef::new_element(
                            markup5ever::QualName::new(None, ns!(html), local_name!("a")),
                            vec![(
                                kuchiki::ExpandedName::new("", "href"),
                                kuchiki::Attribute {
                                    prefix: None,
                                    value: url.into(),
                                },
                            )],
                        );
                        link.append(NodeRef::new_text(url));
                        new_children.push(link);

                        // Add any trimmed punctuation as separate text
                        let original_url = url_match.as_str();
                        if url.len() < original_url.len() {
                            let punctuation = &original_url[url.len()..];
                            new_children.push(NodeRef::new_text(punctuation));
                        }

                        last_end = url_match.end();
                    }

                    // Add remaining text after the last URL
                    if last_end < text_content.len() {
                        let after_text = &text_content[last_end..];
                        if !after_text.is_empty() {
                            new_children.push(NodeRef::new_text(after_text));
                        }
                    }

                    // Replace the text node with new children
                    if !new_children.is_empty() {
                        for child in new_children {
                            text_node.insert_before(child);
                        }
                        text_node.detach();
                    }
                }

                let mut out = Vec::new();
                document.serialize(&mut out).ok();
                String::from_utf8(out).unwrap_or_default()
            },
            // Return original HTML on error
            "",
        )
    }

    /// Post-process HTML to enhance manpage references with URL links.
    /// This finds <span class="manpage-reference"> elements and converts them to links when URLs are available.
    fn process_manpage_references_html(&self, html: &str) -> String {
        safely_process_markup(
            html,
            |html| {
                use kuchiki::NodeRef;
                use tendril::TendrilSink;

                let document = kuchiki::parse_html().one(html);
                let mut to_replace = Vec::new();

                // Find all spans with class "manpage-reference"
                for span_node in document.select("span.manpage-reference").unwrap() {
                    let span_el = span_node.as_node();
                    let span_text = span_el.text_contents();

                    if let Some(ref urls) = self.manpage_urls {
                        // Check for direct URL match
                        if let Some(url) = urls.get(&span_text) {
                            let clean_url = extract_url_from_html(url);
                            let link = NodeRef::new_element(
                                markup5ever::QualName::new(None, ns!(html), local_name!("a")),
                                vec![
                                    (
                                        kuchiki::ExpandedName::new("", "href"),
                                        kuchiki::Attribute {
                                            prefix: None,
                                            value: clean_url.into(),
                                        },
                                    ),
                                    (
                                        kuchiki::ExpandedName::new("", "class"),
                                        kuchiki::Attribute {
                                            prefix: None,
                                            value: "manpage-reference".into(),
                                        },
                                    ),
                                ],
                            );
                            link.append(NodeRef::new_text(span_text.clone()));
                            to_replace.push((span_el.clone(), link));
                        }
                    }
                }

                // Apply replacements
                for (old, new) in to_replace {
                    old.insert_before(new);
                    old.detach();
                }

                let mut out = Vec::new();
                document.serialize(&mut out).ok();
                String::from_utf8(out).unwrap_or_default()
            },
            // Return original HTML on error
            "",
        )
    }

    /// HTML post-processing using kuchiki DOM manipulation.
    fn kuchiki_postprocess(&self, html: &str) -> String {
        safely_process_markup(
            html,
            |html| {
                use tendril::TendrilSink;

                let document = kuchiki::parse_html().one(html);

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
    fn process_list_item_id_markers(&self, document: &kuchiki::NodeRef) {
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
            let span = kuchiki::NodeRef::new_element(
                markup5ever::QualName::new(None, ns!(html), local_name!("span")),
                vec![
                    (
                        kuchiki::ExpandedName::new("", "id"),
                        kuchiki::Attribute {
                            prefix: None,
                            value: id,
                        },
                    ),
                    (
                        kuchiki::ExpandedName::new("", "class"),
                        kuchiki::Attribute {
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
    fn process_header_anchor_comments(&self, document: &kuchiki::NodeRef) {
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
    fn process_list_item_inline_anchors(&self, document: &kuchiki::NodeRef) {
        for li_node in document.select("li").unwrap() {
            let li_element = li_node.as_node();
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

                        let span = kuchiki::NodeRef::new_element(
                            markup5ever::QualName::new(None, ns!(html), local_name!("span")),
                            vec![
                                (
                                    kuchiki::ExpandedName::new("", "id"),
                                    kuchiki::Attribute {
                                        prefix: None,
                                        value: id.into(),
                                    },
                                ),
                                (
                                    kuchiki::ExpandedName::new("", "class"),
                                    kuchiki::Attribute {
                                        prefix: None,
                                        value: "nixos-anchor".into(),
                                    },
                                ),
                            ],
                        );
                        li_element.append(span);
                        if !remaining_content.is_empty() {
                            li_element.append(kuchiki::NodeRef::new_text(remaining_content));
                        }
                    }
                }
            }
        }
    }

    /// Process inline anchors in paragraphs: <p>[]{#id}content</p>
    fn process_paragraph_inline_anchors(&self, document: &kuchiki::NodeRef) {
        for p_node in document.select("p").unwrap() {
            let p_element = p_node.as_node();
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

                        let span = kuchiki::NodeRef::new_element(
                            markup5ever::QualName::new(None, ns!(html), local_name!("span")),
                            vec![
                                (
                                    kuchiki::ExpandedName::new("", "id"),
                                    kuchiki::Attribute {
                                        prefix: None,
                                        value: id.into(),
                                    },
                                ),
                                (
                                    kuchiki::ExpandedName::new("", "class"),
                                    kuchiki::Attribute {
                                        prefix: None,
                                        value: "nixos-anchor".into(),
                                    },
                                ),
                            ],
                        );
                        p_element.append(span);
                        if !remaining_content.is_empty() {
                            p_element.append(kuchiki::NodeRef::new_text(remaining_content));
                        }
                    }
                }
            }
        }
    }

    /// Process remaining standalone inline anchors throughout the document
    fn process_remaining_inline_anchors(&self, document: &kuchiki::NodeRef) {
        let mut text_nodes_to_process = Vec::new();

        for node in document.inclusive_descendants() {
            if let Some(text_node) = node.as_text() {
                let text_content = text_node.borrow().clone();
                if text_content.contains("[]{#") {
                    text_nodes_to_process.push((node.clone(), text_content));
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
                                new_children.push(kuchiki::NodeRef::new_text(before_text));
                            }
                        }

                        // Add span element
                        let span = kuchiki::NodeRef::new_element(
                            markup5ever::QualName::new(None, ns!(html), local_name!("span")),
                            vec![
                                (
                                    kuchiki::ExpandedName::new("", "id"),
                                    kuchiki::Attribute {
                                        prefix: None,
                                        value: id,
                                    },
                                ),
                                (
                                    kuchiki::ExpandedName::new("", "class"),
                                    kuchiki::Attribute {
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
                    new_children.push(kuchiki::NodeRef::new_text(after_text));
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
    fn process_empty_auto_links(&self, document: &kuchiki::NodeRef) {
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
                        link_element.append(kuchiki::NodeRef::new_text(display_text));
                    }
                }
            }
        }
    }

    /// Process empty HTML links that have no content
    fn process_empty_html_links(&self, document: &kuchiki::NodeRef) {
        for link_node in document.select("a[href^='#']").unwrap() {
            let link_element = link_node.as_node();
            let text_content = link_element.text_contents();

            if text_content.trim().is_empty() {
                if let Some(element) = link_element.as_element() {
                    if let Some(href) = element.attributes.borrow().get(local_name!("href")) {
                        let display_text = self.humanize_anchor_id(href);
                        link_element.append(kuchiki::NodeRef::new_text(display_text));
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

/// Trait for AST transformations (e.g., prompt highlighting).
pub trait AstTransformer {
    fn transform<'a>(&self, node: &'a AstNode<'a>);
}

/// Extract all inline text from a heading node.
/// AST transformer for processing command and REPL prompts in inline code blocks.
pub struct PromptTransformer;

impl AstTransformer for PromptTransformer {
    fn transform<'a>(&self, node: &'a AstNode<'a>) {
        use comrak::nodes::NodeValue;
        use regex::Regex;

        let command_prompt_re = Regex::new(r"^\s*\$\s+(.+)$").unwrap();
        let repl_prompt_re = Regex::new(r"^nix-repl>\s*(.*)$").unwrap();

        for child in node.children() {
            {
                let mut data = child.data.borrow_mut();
                if let NodeValue::Code(ref code) = data.value {
                    let literal = code.literal.trim();

                    // Match command prompts with flexible whitespace
                    if let Some(caps) = command_prompt_re.captures(literal) {
                        // Skip escaped prompts
                        if !literal.starts_with("\\$") && !literal.starts_with("$$") {
                            let command = caps[1].trim();
                            let html = format!(
                                "<code class=\"terminal\"><span class=\"prompt\">$</span> {command}</code>"
                            );
                            data.value = NodeValue::HtmlInline(html);
                        }
                    } else if let Some(caps) = repl_prompt_re.captures(literal) {
                        // Skip double prompts
                        if !literal.starts_with("nix-repl>>") {
                            let expression = caps[1].trim();
                            let html = format!(
                                "<code class=\"nix-repl\"><span class=\"prompt\">nix-repl&gt;</span> {expression}</code>"
                            );
                            data.value = NodeValue::HtmlInline(html);
                        }
                    }
                }
            }
            self.transform(child);
        }
    }
}

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
#[must_use]
pub fn process_option_references(html: &str) -> String {
    use kuchiki::{Attribute, ExpandedName, NodeRef};
    use markup5ever::{QualName, local_name, namespace_url, ns};
    use tendril::TendrilSink;

    safely_process_markup(
        html,
        |html| {
            let document = kuchiki::parse_html().one(html);

            let mut to_replace = vec![];

            for code_node in document.select("code").unwrap() {
                let code_el = code_node.as_node();
                let code_text = code_el.text_contents();

                // Skip if this code element already has a role-specific class
                if let Some(element) = code_el.as_element() {
                    if let Some(class_attr) = element.attributes.borrow().get(local_name!("class"))
                    {
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
                            if let Some(class_attr) =
                                element.attributes.borrow().get(local_name!("class"))
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
                    let a = NodeRef::new_element(
                        QualName::new(None, ns!(html), local_name!("a")),
                        attrs,
                    );
                    let code = NodeRef::new_element(
                        QualName::new(None, ns!(html), local_name!("code")),
                        vec![],
                    );
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
        },
        // Return original HTML on error
        "",
    )
}

/// Check if a string looks like a `NixOS` option reference
fn is_nixos_option_reference(text: &str) -> bool {
    // Must have at least 2 dots and no whitespace
    let dot_count = text.chars().filter(|&c| c == '.').count();
    if dot_count < 2 || text.chars().any(char::is_whitespace) {
        return false;
    }

    // Must not contain special characters that indicate it's not an option
    if text.contains('<') || text.contains('>') || text.contains('$') || text.contains('/') {
        return false;
    }

    // Must start with a letter (options don't start with numbers or special chars)
    if !text.chars().next().is_some_and(char::is_alphabetic) {
        return false;
    }

    // Must look like a structured option path (letters, numbers, dots, dashes, underscores)
    text.chars()
        .all(|c| c.is_alphanumeric() || c == '.' || c == '-' || c == '_')
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
