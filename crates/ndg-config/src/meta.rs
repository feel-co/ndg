use std::collections::HashMap;

use ndg_macros::Configurable;
use serde::{Deserialize, Serialize};

/// Meta tag configuration for HTML head injection.
///
/// Contains `OpenGraph` tags and additional meta tags to be injected into
/// the HTML `<head>` element.
#[derive(Debug, Clone, Serialize, Deserialize, Default, Configurable)]
#[serde(default)]
pub struct MetaConfig {
  /// `OpenGraph` tags (e.g., `{"og:title": "...", "og:image": "..."}`)
  pub opengraph: Option<HashMap<String, String>>,

  /// Additional meta tags (e.g., `{"description": "...", "keywords": "..."}`)
  pub tags: Option<HashMap<String, String>>,

  /// Path to a favicon file (e.g., favicon.ico, favicon.png).
  /// The file is copied to the root of the output directory.
  #[config(key = "favicon", allow_empty)]
  pub favicon: Option<std::path::PathBuf>,
}
