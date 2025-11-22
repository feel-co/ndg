//! Type definitions for the Markdown processor.
//!
//! Contains all the core types used by the processor, including:
//! - Configuration options (`MarkdownOptions`)
//! - The main processor struct (`MarkdownProcessor`)
//! - AST transformation traits and implementations
//!
//! # Examples
//!
//! ```
//! use ndg_commonmark::{MarkdownOptions, MarkdownProcessor};
//!
//! let options = MarkdownOptions {
//!   gfm: true,
//!   nixpkgs: true,
//!   highlight_code: true,
//!   ..Default::default()
//! };
//!
//! let processor = MarkdownProcessor::new(options);
//! ```

use std::collections::{HashMap, HashSet};

use comrak::nodes::AstNode;

/// Options for configuring the Markdown processor.
#[derive(Debug, Clone)]
#[allow(
  clippy::struct_excessive_bools,
  reason = "Config struct with related boolean flags"
)]
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

  /// Enable automatic linking for option role markup.
  /// When `true`, `{option}` roles will be converted to links to options.html.
  /// When `false`, they will be rendered as plain `<code>` elements.
  pub auto_link_options: bool,

  /// Optional: Set of valid option names for validation.
  /// When provided, only options in this set will be auto-linked.
  /// When `None`, all options will be linked (no validation).
  pub valid_options: Option<HashSet<String>>,

  /// How to handle hard tabs in code blocks.
  pub tab_style: TabStyle,
}

/// Configuration for handling hard tabs in code blocks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabStyle {
  /// Leave hard tabs unchanged
  None,
  /// Issue a warning when hard tabs are detected
  Warn,
  /// Automatically convert hard tabs to spaces (using 2 spaces per tab)
  Normalize,
}

impl MarkdownOptions {
  /// Enable all available features based on compile-time feature flags.
  #[must_use]
  pub const fn with_all_features() -> Self {
    Self {
      gfm:               cfg!(feature = "gfm"),
      nixpkgs:           cfg!(feature = "nixpkgs"),
      highlight_code:    cfg!(any(feature = "syntastica", feature = "syntect")),
      highlight_theme:   None,
      manpage_urls_path: None,
      auto_link_options: true,
      valid_options:     None,
      tab_style:         TabStyle::None,
    }
  }

  /// Create options with runtime feature overrides.
  #[must_use]
  pub const fn with_features(
    gfm: bool,
    nixpkgs: bool,
    highlight_code: bool,
  ) -> Self {
    Self {
      gfm,
      nixpkgs,
      highlight_code,
      highlight_theme: None,
      manpage_urls_path: None,
      auto_link_options: true,
      valid_options: None,
      tab_style: TabStyle::None,
    }
  }
}

impl Default for MarkdownOptions {
  fn default() -> Self {
    Self {
      gfm:               cfg!(feature = "gfm"),
      nixpkgs:           cfg!(feature = "nixpkgs"),
      highlight_code:    cfg!(feature = "syntastica"),
      manpage_urls_path: None,
      highlight_theme:   None,
      auto_link_options: true,
      valid_options:     None,
      tab_style:         TabStyle::None,
    }
  }
}

/// Main Markdown processor.
///
/// Can be cheaply cloned since it uses `Arc` internally for the syntax manager.
#[derive(Clone)]
pub struct MarkdownProcessor {
  pub(crate) options:        MarkdownOptions,
  pub(crate) manpage_urls:   Option<HashMap<String, String>>,
  pub(crate) syntax_manager: Option<crate::syntax::SyntaxManager>,
  pub(crate) base_dir:       std::path::PathBuf,
}

/// Trait for AST transformations (e.g., prompt highlighting).
pub trait AstTransformer {
  fn transform<'a>(&self, node: &'a AstNode<'a>);
}

/// AST transformer for processing command and REPL prompts in inline code
/// blocks.
pub struct PromptTransformer;

impl AstTransformer for PromptTransformer {
  fn transform<'a>(&self, node: &'a AstNode<'a>) {
    use std::sync::LazyLock;

    use comrak::nodes::NodeValue;
    use regex::Regex;

    static COMMAND_PROMPT_RE: LazyLock<Regex> = LazyLock::new(|| {
      Regex::new(r"^\s*\$\s+(.+)$").unwrap_or_else(|e| {
        log::error!(
          "Failed to compile COMMAND_PROMPT_RE regex: {e}\n Falling back to \
           never matching regex."
        );
        crate::utils::never_matching_regex().unwrap_or_else(|_| {
          // As a last resort, create a regex that matches nothing
          #[allow(
            clippy::expect_used,
            reason = "This pattern is guaranteed to be valid"
          )]
          Regex::new(r"[^\s\S]")
            .expect("regex pattern [^\\s\\S] should always compile")
        })
      })
    });
    static REPL_PROMPT_RE: LazyLock<Regex> = LazyLock::new(|| {
      Regex::new(r"^nix-repl>\s*(.*)$").unwrap_or_else(|e| {
        log::error!(
          "Failed to compile REPL_PROMPT_RE regex: {e}\n Falling back to \
           never matching regex."
        );
        crate::utils::never_matching_regex().unwrap_or_else(|_| {
          // As a last resort, create a regex that matches nothing
          #[allow(
            clippy::expect_used,
            reason = "This pattern is guaranteed to be valid"
          )]
          Regex::new(r"[^\s\S]")
            .expect("regex pattern [^\\s\\S] should always compile")
        })
      })
    });

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
                "<code class=\"terminal\"><span class=\"prompt\">$</span> \
                 {command}</code>"
              );
              data.value = NodeValue::HtmlInline(html);
            }
          } else if let Some(caps) = REPL_PROMPT_RE.captures(literal) {
            // Skip double prompts
            if !literal.starts_with("nix-repl>>") {
              let expression = caps[1].trim();
              let html = format!(
                "<code class=\"nix-repl\"><span \
                 class=\"prompt\">nix-repl&gt;</span> {expression}</code>"
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
  pub const fn gfm(mut self, enabled: bool) -> Self {
    self.options.gfm = enabled;
    self
  }

  /// Enable or disable Nixpkgs extensions.
  #[must_use]
  pub const fn nixpkgs(mut self, enabled: bool) -> Self {
    self.options.nixpkgs = enabled;
    self
  }

  /// Enable or disable syntax highlighting.
  #[must_use]
  pub const fn highlight_code(mut self, enabled: bool) -> Self {
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

  /// Enable or disable automatic linking for {option} role markup.
  #[must_use]
  pub const fn auto_link_options(mut self, enabled: bool) -> Self {
    self.options.auto_link_options = enabled;
    self
  }

  /// Set the valid options for validation.
  #[must_use]
  pub fn valid_options(mut self, options: Option<HashSet<String>>) -> Self {
    self.options.valid_options = options;
    self
  }

  /// Set how to handle hard tabs in code blocks.
  #[must_use]
  pub const fn tab_style(mut self, style: TabStyle) -> Self {
    self.options.tab_style = style;
    self
  }

  /// Build the final `MarkdownOptions`.
  #[must_use]
  pub fn build(self) -> MarkdownOptions {
    self.options
  }

  /// Create options from external configuration with fluent interface.
  #[must_use]
  pub fn from_external_config<T>(_config: &T) -> Self {
    Self::new()
  }
}

impl Default for MarkdownOptionsBuilder {
  fn default() -> Self {
    Self::new()
  }
}
