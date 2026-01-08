//! # NDG: Not a Docs Generator
//!
//! `ndg` is a fast, customizable documentation generator for Nix, `NixOS`, and
//! Nixpkgs module systems. It converts Markdown and Nix module options into
//! HTML and manpages, supporting Nixpkgs-flavored `CommonMark`, automatic table
//! of contents, search, multi-threading, and fully customizable templates.
//!
//! ## Features
//! - Markdown to HTML and Manpage conversion with Nixpkgs-flavored `CommonMark`
//!   support
//! - Automatic table of contents and heading anchors
//! - Search functionality across documents
//! - Nix module options support for generating documentation from
//!   `options.json`
//! - Fully customizable templates and stylesheets
//! - Multi-threading for fast generation of large documentation sets
//! - Seamless integration with Nix workflows
//!
//! ## Usage
//!
//! Be advised that NDG's internal API is exposed ONLY in order to be used in
//! our unit tests. While this API could be useful to end users, e.g., by
//! exposing some wrapper functions, [ndg-commonmark](../ndg_commonmark)'s own
//! API should be preferred for library use. This interface is for
//! testing purposes ONLY and will not make any guarantees of stability between
//! versions.
//!
//! ### CLI Use
//!
//! [Github Repository]: https://github.com/feel-co/ndg
//! NDG is primarily designed as a CLI utility, and its documentation is located
//! in the [Github repository]. Please refer to the project README for more
//! information about the project, installing NDG, and CLI usage.

pub mod cli;
pub mod error;

// Re-export internal crates for backward compatibility
pub use ndg_config as config;
pub use ndg_html as html;
pub use ndg_manpage as manpage;
pub use ndg_utils as utils;
