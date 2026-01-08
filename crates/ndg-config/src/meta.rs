use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Meta tag configuration for HTML head injection.
///
/// Contains `OpenGraph` tags and additional meta tags to be injected into
/// the HTML `<head>` element.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct MetaConfig {
  /// `OpenGraph` tags (e.g., `{"og:title": "...", "og:image": "..."}`)
  pub opengraph: Option<HashMap<String, String>>,

  /// Additional meta tags (e.g., `{"description": "...", "keywords": "..."}`)
  pub tags: Option<HashMap<String, String>>,
}
