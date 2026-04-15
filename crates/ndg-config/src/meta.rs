use std::{collections::HashMap, ffi::OsStr, path::PathBuf};

use ndg_macros::Configurable;
use serde::{Deserialize, Serialize};

fn default_rel() -> String {
  "icon".to_string()
}

/// A single favicon entry, corresponding to one `<link rel="...">` tag in the
/// HTML `<head>`.
///
/// # Examples
///
/// ```toml
/// [[meta.favicon]]
/// href  = "favicon-32.png"
/// type  = "image/png"
/// sizes = "32x32"
///
/// [[meta.favicon]]
/// href  = "/nix/store/...-apple-touch-icon.png"
/// dest  = "apple-touch-icon.png"
/// rel   = "apple-touch-icon"
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaviconEntry {
  /// Path to the favicon file on disk. The file is copied to the root of the
  /// output directory.
  pub href: PathBuf,

  /// Destination filename in the output root. Defaults to the filename portion
  /// of `href`. Use this when `href` is a path with unwanted components (e.g.
  /// a Nix store path).
  #[serde(default)]
  pub dest: Option<PathBuf>,

  /// The `rel` attribute value. Defaults to `"icon"` when omitted.
  #[serde(default = "default_rel")]
  pub rel: String,

  /// The `type` attribute (MIME type), e.g. `"image/png"`, `"image/x-icon"`.
  #[serde(rename = "type", default)]
  pub mime_type: Option<String>,

  /// The `sizes` attribute, e.g. `"16x16"`, `"32x32"`, `"any"`.
  #[serde(default)]
  pub sizes: Option<String>,
}

impl FaviconEntry {
  /// Returns the filename to use in the output directory.
  ///
  /// Uses `dest` if provided, otherwise the filename portion of `href`.
  ///
  /// # Errors
  ///
  /// Returns an error if neither `dest` nor `href` yields a valid filename.
  pub fn output_filename(&self) -> Option<&OsStr> {
    self
      .dest
      .as_ref()
      .and_then(|p| p.file_name())
      .or_else(|| self.href.file_name())
  }
}

/// Meta tag configuration for HTML head injection.
///
/// Contains `OpenGraph` tags, additional meta tags, and favicon entries to be
/// injected into the HTML `<head>` element.
#[derive(Debug, Clone, Serialize, Deserialize, Default, Configurable)]
#[serde(default)]
pub struct MetaConfig {
  /// `OpenGraph` tags (e.g., `{"og:title": "...", "og:image": "..."}`)
  pub opengraph: Option<HashMap<String, String>>,

  /// Additional meta tags (e.g., `{"description": "...", "keywords": "..."}`)
  pub tags: Option<HashMap<String, String>>,

  /// Favicon entries. Each entry produces one `<link>` tag in the HTML head.
  #[serde(default)]
  pub favicon: Vec<FaviconEntry>,
}
