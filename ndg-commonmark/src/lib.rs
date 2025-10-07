//! # NDG-Commonmark
//!
//! This is a high-performance Markdown processor designed for Nix, `NixOS` and
//! Nixpkgs documentation featuring AST-based processing with role markup,
//! autolinks, GFM support and more.
//!
//! ## Processing
//!
//! ```rust
//! use ndg_commonmark::{ProcessorPreset, process_markdown_string};
//!
//! let result = process_markdown_string(
//!   "# Hello World\n\nThis is **bold** text.",
//!   ProcessorPreset::Basic,
//! );
//! println!("HTML: {}", result.html);
//! println!("Title: {:?}", result.title);
//! ```
//!
//! ## API
//!
//! ```rust
//! use ndg_commonmark::{MarkdownOptions, MarkdownProcessor};
//!
//! let processor = MarkdownProcessor::new(MarkdownOptions::default());
//! let result = processor.render("# Hello World\n\nThis is **bold** text.");
//!
//! println!("HTML: {}", result.html);
//! println!("Title: {:?}", result.title);
//! println!("Headers: {:?}", result.headers);
//! ```
//!
//! ## Builder Pattern
//!
//! ```rust
//! use ndg_commonmark::{MarkdownOptionsBuilder, MarkdownProcessor};
//!
//! let options = MarkdownOptionsBuilder::new()
//!   .gfm(true)
//!   .nixpkgs(true)
//!   .highlight_code(true)
//!   .highlight_theme(Some("github"))
//!   .build();
//!
//! let processor = MarkdownProcessor::new(options);
//! ```

pub mod processor;
pub mod syntax;
mod types;
pub mod utils;

// Re-export main API
#[cfg(feature = "gfm")]
pub use crate::processor::apply_gfm_extensions;
#[cfg(feature = "nixpkgs")]
pub use crate::processor::{
  process_block_elements,
  process_file_includes,
  process_inline_anchors,
  process_myst_autolinks,
  process_option_references,
  process_role_markup,
};
// Those don't require any feature gates, unlike above APIs.
pub use crate::{
  processor::{
    AstTransformer,
    MarkdownOptions,
    MarkdownOptionsBuilder,
    MarkdownProcessor,
    ProcessorFeature,
    ProcessorPreset,
    PromptTransformer,
    collect_markdown_files,
    create_processor,
    extract_inline_text,
    process_batch,
    process_markdown_file,
    process_markdown_file_with_basedir,
    process_markdown_string,
    process_safe,
    process_with_recovery,
  },
  syntax::{
    SyntaxConfig,
    SyntaxError,
    SyntaxHighlighter,
    SyntaxManager,
    create_default_manager,
  },
  types::{Header, IncludedFile, MarkdownResult},
};
