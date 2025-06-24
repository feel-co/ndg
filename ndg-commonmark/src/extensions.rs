#![allow(dead_code)]
//! Extension logic for ndg-commonmark: GFM and Nixpkgs/NixOS flavored markdown.
//!
//! This module is intended to house all logic for feature-flagged extensions
//! to the base CommonMark processor, including GitHub Flavored Markdown (GFM)
//! and Nixpkgs/NixOS documentation-specific syntax.

/// Apply GitHub Flavored Markdown (GFM) extensions to the input markdown.
///
/// This is a placeholder for future GFM-specific preprocessing or AST transformations.
/// In practice, most GFM features are enabled via comrak options, but additional
/// logic (such as custom tables, task lists, etc.) can be added here.
#[cfg(feature = "gfm")]
pub fn apply_gfm_extensions(markdown: &str) -> String {
    // XXX: Comrak already supports GFM, but if there is any feature in the spec
    // that is not implemented as we'd like for it to be, we can add it here.
    markdown.to_owned()
}

/// Apply Nixpkgs/NixOS documentation extensions to the input markdown.
///
/// Placeholder for now.
#[cfg(feature = "nixpkgs")]
pub fn apply_nixpkgs_extensions(markdown: &str) -> String {
    // TODO: Implement Nixpkgs-specific preprocessing if needed.
    markdown.to_owned()
}
