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

use std::{str::FromStr, sync::LazyLock};

use comrak::nodes::AstNode;
use regex::Regex;
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};

static COMMAND_PROMPT_RE: LazyLock<Regex> = LazyLock::new(|| {
  Regex::new(r"^\s*\$\s+(.+)$").unwrap_or_else(|e| {
    log::error!(
      "Failed to compile COMMAND_PROMPT_RE regex: {e}\n Falling back to never \
       matching regex."
    );
    crate::utils::never_matching_regex().unwrap_or_else(|_| {
      #[expect(
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
      "Failed to compile REPL_PROMPT_RE regex: {e}\n Falling back to never \
       matching regex."
    );
    crate::utils::never_matching_regex().unwrap_or_else(|_| {
      #[expect(
        clippy::expect_used,
        reason = "This pattern is guaranteed to be valid"
      )]
      Regex::new(r"[^\s\S]")
        .expect("regex pattern [^\\s\\S] should always compile")
    })
  })
});

/// Options for configuring the Markdown processor.
#[derive(Debug, Clone)]
#[expect(
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

  /// Additional Comrak Markdown extensions to enable.
  pub extensions: Vec<MarkdownExtension>,

  /// Optional: Custom syntax highlighting theme name.
  pub highlight_theme: Option<String>,

  /// Optional: Path to manpage URL mappings (for {manpage} roles).
  pub manpage_urls_path: Option<String>,

  /// Optional: Path to user-provided Tree-sitter query overrides.
  ///
  /// Expected layout:
  /// `queries/<language>/{highlights,injections,locals}.scm`
  pub syntax_queries_path: Option<String>,

  /// Enable automatic linking for option role markup.
  /// When `true`, `{option}` roles will be converted to links to options.html.
  /// When `false`, they will be rendered as plain `<code>` elements.
  pub auto_link_options: bool,

  /// Optional: Set of valid option names for validation.
  /// When provided, only options in this set will be auto-linked.
  /// When `None`, all options will be linked (no validation).
  pub valid_options: Option<FxHashSet<String>>,

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
      gfm:                 cfg!(feature = "gfm"),
      nixpkgs:             cfg!(feature = "nixpkgs"),
      highlight_code:      cfg!(any(
        feature = "syntastica",
        feature = "syntect"
      )),
      extensions:          Vec::new(),
      highlight_theme:     None,
      manpage_urls_path:   None,
      syntax_queries_path: None,
      auto_link_options:   true,
      valid_options:       None,
      tab_style:           TabStyle::None,
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
      extensions: Vec::new(),
      highlight_theme: None,
      manpage_urls_path: None,
      syntax_queries_path: None,
      auto_link_options: true,
      valid_options: None,
      tab_style: TabStyle::None,
    }
  }
}

impl Default for MarkdownOptions {
  fn default() -> Self {
    Self {
      gfm:                 cfg!(feature = "gfm"),
      nixpkgs:             cfg!(feature = "nixpkgs"),
      highlight_code:      cfg!(feature = "syntastica"),
      extensions:          Vec::new(),
      manpage_urls_path:   None,
      syntax_queries_path: None,
      highlight_theme:     None,
      auto_link_options:   true,
      valid_options:       None,
      tab_style:           TabStyle::None,
    }
  }
}

/// Optional Markdown syntax extensions provided by Comrak.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MarkdownExtension {
  /// GitHub-style alerts.
  Alerts,
  /// Automatic links.
  Autolink,
  /// Container block directives using `:::`.
  BlockDirective,
  /// CJK-friendly emphasis parsing.
  CjkFriendlyEmphasis,
  /// Greentext parsing.
  Greentext,
  /// Description lists.
  DescriptionLists,
  /// Footnotes.
  Footnotes,
  /// Highlighted text using `==`.
  Highlight,
  /// Inserted text using `++`.
  Insert,
  /// Inline footnotes using `^[...]`.
  InlineFootnotes,
  /// Math using code syntax.
  MathCode,
  /// Math using dollar syntax.
  MathDollars,
  /// Math using LaTeX delimiters.
  MathLatex,
  /// Multiline block quotes.
  MultilineBlockQuotes,
  /// Spoiler text using `||`.
  Spoiler,
  /// Strikethrough text.
  Strikethrough,
  /// Subscript text using `~`.
  Subscript,
  /// Block-scoped subtext.
  Subtext,
  /// Superscript text.
  Superscript,
  /// Tables.
  Table,
  /// GFM tag filtering.
  Tagfilter,
  /// Task lists.
  Tasklist,
  /// Underlined text using `__`.
  Underline,
  /// Wikilinks with the title after the pipe.
  WikilinksTitleAfterPipe,
  /// Wikilinks with the title before the pipe.
  WikilinksTitleBeforePipe,
}

impl FromStr for MarkdownExtension {
  type Err = String;

  fn from_str(value: &str) -> Result<Self, Self::Err> {
    match value {
      "alerts" => Ok(Self::Alerts),
      "autolink" => Ok(Self::Autolink),
      "block-directive" => Ok(Self::BlockDirective),
      "cjk-friendly-emphasis" => Ok(Self::CjkFriendlyEmphasis),
      "greentext" => Ok(Self::Greentext),
      "description-lists" => Ok(Self::DescriptionLists),
      "footnotes" => Ok(Self::Footnotes),
      "highlight" => Ok(Self::Highlight),
      "insert" => Ok(Self::Insert),
      "inline-footnotes" => Ok(Self::InlineFootnotes),
      "math-code" => Ok(Self::MathCode),
      "math-dollars" => Ok(Self::MathDollars),
      "math-latex" => Ok(Self::MathLatex),
      "multiline-block-quotes" => Ok(Self::MultilineBlockQuotes),
      "spoiler" => Ok(Self::Spoiler),
      "strikethrough" => Ok(Self::Strikethrough),
      "subscript" => Ok(Self::Subscript),
      "subtext" => Ok(Self::Subtext),
      "superscript" => Ok(Self::Superscript),
      "table" => Ok(Self::Table),
      "tagfilter" => Ok(Self::Tagfilter),
      "tasklist" => Ok(Self::Tasklist),
      "underline" => Ok(Self::Underline),
      "wikilinks-title-after-pipe" => Ok(Self::WikilinksTitleAfterPipe),
      "wikilinks-title-before-pipe" => Ok(Self::WikilinksTitleBeforePipe),
      _ => Err(format!("unknown Markdown extension: {value}")),
    }
  }
}

/// Main Markdown processor.
///
/// Can be cheaply cloned since it uses `Arc` internally for the syntax manager.
#[derive(Clone)]
pub struct MarkdownProcessor {
  pub(crate) options:        MarkdownOptions,
  pub(crate) manpage_urls:   Option<FxHashMap<String, String>>,
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
    use comrak::nodes::NodeValue;

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

  /// Enable additional Comrak Markdown extensions.
  #[must_use]
  pub fn extensions(mut self, extensions: Vec<MarkdownExtension>) -> Self {
    self.options.extensions = extensions;
    self
  }

  /// Set the syntax highlighting theme.
  #[must_use]
  pub fn highlight_theme<S: Into<String>>(self, theme: Option<S>) -> Self {
    fn inner(
      mut this: MarkdownOptionsBuilder,
      theme: Option<String>,
    ) -> MarkdownOptionsBuilder {
      this.options.highlight_theme = theme;
      this
    }
    inner(self, theme.map(Into::into))
  }

  /// Set the manpage URLs path.
  #[must_use]
  pub fn manpage_urls_path<S: Into<String>>(self, path: Option<S>) -> Self {
    fn inner(
      mut this: MarkdownOptionsBuilder,
      path: Option<String>,
    ) -> MarkdownOptionsBuilder {
      this.options.manpage_urls_path = path;
      this
    }
    inner(self, path.map(Into::into))
  }

  /// Set the path to user-provided Tree-sitter query overrides.
  #[must_use]
  pub fn syntax_queries_path<S: Into<String>>(self, path: Option<S>) -> Self {
    fn inner(
      mut this: MarkdownOptionsBuilder,
      path: Option<String>,
    ) -> MarkdownOptionsBuilder {
      this.options.syntax_queries_path = path;
      this
    }
    inner(self, path.map(Into::into))
  }

  /// Enable or disable automatic linking for {option} role markup.
  #[must_use]
  pub const fn auto_link_options(mut self, enabled: bool) -> Self {
    self.options.auto_link_options = enabled;
    self
  }

  /// Set the valid options for validation.
  #[must_use]
  pub fn valid_options(mut self, options: Option<FxHashSet<String>>) -> Self {
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
}

impl Default for MarkdownOptionsBuilder {
  fn default() -> Self {
    Self::new()
  }
}
