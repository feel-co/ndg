//! Type definitions for the Markdown processor.
//!
//! This module contains all the core types used by the processor, including:
//! - Configuration options (`MarkdownOptions`)
//! - The main processor struct (`MarkdownProcessor`)
//! - AST transformation traits and implementations
//!
//! # Examples
//!
//! ```
//! use ndg_commonmark::{MarkdownProcessor, MarkdownOptions};
//!
//! let options = MarkdownOptions {
//!     gfm: true,
//!     nixpkgs: true,
//!     highlight_code: true,
//!     ..Default::default()
//! };
//!
//! let processor = MarkdownProcessor::new(options);
//! ```

use std::collections::HashMap;

use comrak::nodes::AstNode;

/// Options for configuring the Markdown processor.
#[derive(Debug, Clone)]
pub struct MarkdownOptions {
    /// Enable GitHub Flavored Markdown (GFM) extensions.
    pub gfm: bool,

    /// Enable Nixpkgs/NixOS documentation extensions.
    pub nixpkgs: bool,

    /// Enable syntax highlighting for code blocks.
    pub highlight_code: bool,

    /// Optional: Custom syntax highlighting theme name.
    pub highlight_theme: Option<String>,

    /// Optional: Path to manpage URL mappings (for {manpage} roles).
    pub manpage_urls_path: Option<String>,
}

impl Default for MarkdownOptions {
    fn default() -> Self {
        Self {
            gfm: cfg!(feature = "gfm"),
            nixpkgs: cfg!(feature = "nixpkgs"),
            highlight_code: cfg!(feature = "syntastica"),
            manpage_urls_path: None,
            highlight_theme: None,
        }
    }
}

/// Main Markdown processor.
pub struct MarkdownProcessor {
    pub(crate) options: MarkdownOptions,
    pub(crate) manpage_urls: Option<HashMap<String, String>>,
    pub(crate) syntax_manager: Option<crate::syntax::SyntaxManager>,
}

/// Trait for AST transformations (e.g., prompt highlighting).
pub trait AstTransformer {
    fn transform<'a>(&self, node: &'a AstNode<'a>);
}

/// AST transformer for processing command and REPL prompts in inline code blocks.
pub struct PromptTransformer;

impl AstTransformer for PromptTransformer {
    fn transform<'a>(&self, node: &'a AstNode<'a>) {
        use std::sync::LazyLock;

        use comrak::nodes::NodeValue;
        use regex::Regex;

        static COMMAND_PROMPT_RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^\s*\$\s+(.+)$").unwrap());
        static REPL_PROMPT_RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^nix-repl>\s*(.*)$").unwrap());

        for child in node.children() {
            {
                let mut data = child.data.borrow_mut();
                if let NodeValue::Code(ref code) = data.value {
                    let literal = code.literal.trim();

                    // Match command prompts with flexible whitespace
                    if let Some(caps) = COMMAND_PROMPT_RE.captures(literal) {
                        // Skip escaped prompts
                        if !literal.starts_with("\\$") && !literal.starts_with("$$") {
                            let command = caps[1].trim();
                            let html = format!(
                                "<code class=\"terminal\"><span class=\"prompt\">$</span> {command}</code>"
                            );
                            data.value = NodeValue::HtmlInline(html);
                        }
                    } else if let Some(caps) = REPL_PROMPT_RE.captures(literal) {
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

/// Builder for constructing `MarkdownOptions` with method chaining.
#[derive(Debug, Clone)]
pub struct MarkdownOptionsBuilder {
    options: MarkdownOptions,
}

impl MarkdownOptionsBuilder {
    /// Create a new builder with default options.
    #[must_use]
    pub fn new() -> Self {
        Self {
            options: MarkdownOptions::default(),
        }
    }

    /// Enable or disable GitHub Flavored Markdown.
    #[must_use]
    pub fn gfm(mut self, enabled: bool) -> Self {
        self.options.gfm = enabled;
        self
    }

    /// Enable or disable Nixpkgs extensions.
    #[must_use]
    pub fn nixpkgs(mut self, enabled: bool) -> Self {
        self.options.nixpkgs = enabled;
        self
    }

    /// Enable or disable syntax highlighting.
    #[must_use]
    pub fn highlight_code(mut self, enabled: bool) -> Self {
        self.options.highlight_code = enabled;
        self
    }

    /// Set the syntax highlighting theme.
    #[must_use]
    pub fn highlight_theme<S: Into<String>>(mut self, theme: Option<S>) -> Self {
        self.options.highlight_theme = theme.map(Into::into);
        self
    }

    /// Set the manpage URLs path.
    #[must_use]
    pub fn manpage_urls_path<S: Into<String>>(mut self, path: Option<S>) -> Self {
        self.options.manpage_urls_path = path.map(Into::into);
        self
    }

    /// Build the final `MarkdownOptions`.
    #[must_use]
    pub fn build(self) -> MarkdownOptions {
        self.options
    }
}

impl Default for MarkdownOptionsBuilder {
    fn default() -> Self {
        Self::new()
    }
}
