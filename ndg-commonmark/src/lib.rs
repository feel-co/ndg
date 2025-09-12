//! # NDG - A documentation generator for Nix/OS projects
//!
//! This is a high-performance Markdown processor designed for Nix, `NixOS` and Nixpkgs documentation
//! featuring AST-based processing with role markup, autolinks, GFM support and more.
//!
//! ## Quick Start
//!
//! ```rust
//! use ndg_commonmark::{MarkdownProcessor, MarkdownOptions};
//!
//! let processor = MarkdownProcessor::new(MarkdownOptions::default());
//! let result = processor.render("# Hello World\n\nThis is **bold** text.");
//!
//! println!("HTML: {}", result.html);
//! println!("Title: {:?}", result.title);
//! println!("Headers: {:?}", result.headers);
//! ```
//!
//! ## Features
//!
//! - **AST-based processing** using `comrak` for robust, maintainable code
//! - **Role markup** for semantic documentation (`{command}`, `{file}`, `{option}`, etc.)
//! - **Autolink processing** with intelligent punctuation handling
//! - **Header extraction** with automatic anchor generation
//! - **Error recovery** with graceful degradation for malformed input
//! - **`NixOS` extensions** including admonitions, anchors, and manpage references
//!
//! ## Migration from Legacy API
//!
//! **Before (deprecated):**
//! ```rust,ignore
//! use ndg_commonmark::legacy_markdown::process_markdown;
//! let (html, headers, title) = process_markdown(content, None, None, path);
//! ```
//!
//! **After (recommended):**
//! ```rust
//! use ndg_commonmark::{MarkdownProcessor, MarkdownOptions};
//! let processor = MarkdownProcessor::new(MarkdownOptions::default());
//! let result = processor.render("# Hello World");
//! let (html, headers, title) = (result.html, result.headers, result.title);
//! ```
//!
//! ## Configuration
//!
//! ```rust
//! use ndg_commonmark::{MarkdownProcessor, MarkdownOptions};
//!
//! let mut options = MarkdownOptions::default();
//! options.gfm = true;  // Enable GitHub Flavored Markdown
//! options.nixpkgs = true;  // Enable NixOS-specific extensions
//! options.manpage_urls_path = Some("manpage-urls.json".to_string());
//!
//! let processor = MarkdownProcessor::new(options);
//! ```

mod extensions;
pub mod processor;
mod types;
pub mod utils;

// Legacy modules
#[deprecated(
    since = "0.4.0",
    note = "This module is deprecated. Use `MarkdownProcessor` from the `processor` module instead. Will be removed in a future version."
)]
pub mod legacy_markdown;

#[deprecated(
    since = "0.4.0",
    note = "This module is deprecated. Use `MarkdownProcessor` from the `processor` module instead. Will be removed in a future version."
)]
pub mod legacy_markup;

// Legacy API exports
#[deprecated(
    since = "0.4.0",
    note = "Use `MarkdownProcessor::new().render()` instead. This legacy function will be removed in a future version."
)]
pub use crate::legacy_markdown::process_markdown;
#[deprecated(
    since = "0.4.0",
    note = "Use `MarkdownProcessor::new().render()` instead. This legacy function will be removed in a future version."
)]
pub use crate::legacy_markdown::process_markdown_file;
// New API
pub use crate::{
    processor::{AstTransformer, MarkdownOptions, MarkdownProcessor},
    types::{Header, MarkdownResult},
};
