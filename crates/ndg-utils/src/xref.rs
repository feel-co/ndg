//! Cross-page anchor registry and link-rewrite utilities.
use std::collections::HashMap;

use ndg_commonmark::rewrite_cross_page_anchor_links;

use crate::markdown::ProcessedMarkdown;

/// A resolved anchor target, i.e., the output page that owns it and the heading
/// title text.
#[derive(Debug, Clone)]
pub struct AnchorEntry {
  pub page:  String,
  pub title: String,
}

/// Registry mapping anchor IDs to their owning output page and heading title.
pub type AnchorRegistry = HashMap<String, AnchorEntry>;

/// Build an anchor registry from all rendered pages.
///
/// Iterates every [`ProcessedMarkdown`] item and registers each `Header.id`
/// with its `output_path` and `Header.text` as the title. Duplicate IDs are
/// registered with the first occurrence winning.
#[must_use]
pub fn build_anchor_registry(pages: &[ProcessedMarkdown]) -> AnchorRegistry {
  let mut registry = AnchorRegistry::new();
  for page in pages {
    for header in &page.headers {
      if header.id.is_empty() {
        continue;
      }
      if registry.contains_key(&header.id) {
        log::debug!(
          "Duplicate anchor ID '{}' on '{}' (already registered from another \
           page)",
          header.id,
          page.output_path
        );
        continue;
      }
      registry.insert(header.id.clone(), AnchorEntry {
        page:  page.output_path.clone(),
        title: header.text.clone(),
      });
    }
  }
  registry
}

/// Apply cross-page link rewrites to all non-included pages in `pages`.
///
/// Converts the registry to the flat form expected by
/// [`rewrite_cross_page_anchor_links`] and mutates each page's `html_content`
/// in place. Included pages are skipped, and their content is embedded within
/// their parent page's output.
pub fn apply_cross_page_link_rewrites(
  pages: &mut [ProcessedMarkdown],
  registry: &AnchorRegistry,
) {
  if registry.is_empty() {
    return;
  }

  let flat: HashMap<String, (String, String)> = registry
    .iter()
    .map(|(id, entry)| (id.clone(), (entry.page.clone(), entry.title.clone())))
    .collect();

  for page in pages.iter_mut() {
    if page.is_included {
      continue;
    }
    page.html_content = rewrite_cross_page_anchor_links(
      &page.html_content,
      &page.output_path,
      &flat,
    );
  }
}
