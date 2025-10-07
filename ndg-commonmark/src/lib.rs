//! # NDG-Commonmark
//!
//! NDG-Commonmark is a high-performance, extensible Markdown processor for Nix,
//! NixOS, and Nixpkgs projects. It is the AST-based Markdown parser and
//! converter component that is capable of processing Nixpkgs-flavored
//! Commonmark, optionally with additional flavors such as Github Flavored
//! Markdown (GFM).
//!
//! This crate is designed to be a robust, extensible and customizable
//! cornerstone in the Nix ecosystem designed for usage in documentation tools
//! and static site generators that would like to integrate Nix module systems
//! as a first-class citizen while keeping convenient Markdown features under
//! their belt for advanced use.
//!
//! As NDG-Commonmark aims to replace tools such as nixos-render-docs, its
//! syntax is a superset of the Nixpkgs-flavored Commonmark, with optional
//! feature flags for controlling what features are available. It is fully
//! possible to use NDG-Commonmark as a drop-in replacement for
//! nixos-render-docs.
//!
//! ## Overview
//!
//! - **AST-based processing**: Enables advanced transformations and custom
//!   syntax extensions.
//! - **Role markup and Nix-specific features**: Support for `{command}` blocks,
//!   option references, file includes, and more.
//! - **Syntax highlighting**: Modern, themeable code highlighting for many
//!   languages.
//! - **Extensible**: Use feature flags to enable only the extensions you need.
//!
//! ## Usage Examples
//!
//! The following examples are designed to show how to use this crate as a
//! library. They demonstrate how to process Markdown to HTML, extract document
//! structure, and configure the processor for different use cases.
//!
//! ### Basic Markdown Processing
//!
//! This example demonstrates how to convert a Markdown string to HTML using a
//! preset configuration. The [`process_markdown_string`] function is a
//! convenience wrapper for common use cases. The result contains the rendered
//! HTML, the document title, and extracted headers.
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
//! ### Custom Processor and Options
//!
//! For more control, you can create a [`MarkdownProcessor`] with custom
//! options. This allows you to enable or disable features such as GFM, Nixpkgs
//! extensions, and syntax highlighting. The processor exposes methods to render
//! Markdown, extract headers, and more.
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
//! ### Builder Pattern for Configuration
//!
//! The builder pattern allows you to construct [`MarkdownOptions`] with
//! fine-grained control over all features. This is useful for applications that
//! need to dynamically configure the Markdown processor.
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
pub mod types;
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
