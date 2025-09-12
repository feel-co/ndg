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

mod extensions;
pub mod processor;
mod types;
pub mod utils;

pub use crate::{
    processor::{AstTransformer, MarkdownOptions, MarkdownProcessor},
    types::{Header, MarkdownResult},
};
