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

pub mod processor;
pub mod syntax;
mod types;
pub mod utils;

// Re-export extension functions for third-party use
#[cfg(feature = "gfm")]
pub use crate::processor::apply_gfm_extensions;
#[cfg(feature = "ndg-flavored")]
pub use crate::processor::process_option_references;
#[cfg(any(feature = "nixpkgs", feature = "ndg-flavored"))]
pub use crate::processor::process_role_markup;
#[cfg(feature = "nixpkgs")]
pub use crate::processor::{process_block_elements, process_file_includes, process_inline_anchors};
pub use crate::{
    processor::{AstTransformer, MarkdownOptions, MarkdownProcessor},
    syntax::{SyntaxConfig, SyntaxError, SyntaxHighlighter, SyntaxManager, create_default_manager},
    types::{Header, MarkdownResult},
};
