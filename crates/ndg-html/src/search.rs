use std::{
  clone::Clone,
  collections::{HashMap, HashSet},
  fs,
  path::PathBuf,
  sync::OnceLock,
};

use color_eyre::eyre::{Context, Result};
use log::info;
use ndg_config::Config;
use ndg_utils::{html, postprocess};
use rayon::prelude::*;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::template;

/// Represents a searchable anchor/heading within a document
#[derive(Debug, Serialize, Deserialize)]
pub struct SearchAnchor {
  /// Anchor/heading text
  pub text: String,

  /// Anchor ID (e.g., "chapter-nixos-install")
  pub id: String,

  /// Heading level (1-6)
  pub level: u8,

  /// Tokenized anchor text for searching
  tokens: Vec<String>,
}

/// Search document with tokenized content
#[derive(Debug, Serialize, Deserialize)]
pub struct SearchDocument {
  id:           String,
  pub title:    String,
  content:      String,
  pub path:     String,
  tokens:       Vec<String>,
  title_tokens: Vec<String>,

  /// Searchable anchors/headings in this document
  pub anchors: Vec<SearchAnchor>,
}

/// Search index
#[derive(Debug, Serialize, Deserialize)]
pub struct SearchIndex {
  documents: Vec<SearchDocument>,
}

/// Search data including documents and configuration
#[derive(Debug, Serialize, Deserialize)]
pub struct SearchData {
  pub documents:       Vec<SearchDocument>,
  pub min_word_length: usize,
  pub stopwords:       Vec<String>,
  pub boost_title:     f32,
  pub boost_content:   f32,
  pub boost_anchor:    f32,
}

impl SearchIndex {
  const fn new() -> Self {
    Self {
      documents: Vec::new(),
    }
  }

  fn add_document(&mut self, doc: SearchDocument) {
    self.documents.push(doc);
  }
}

/// Tokenize text into searchable terms
fn tokenize(text: &str) -> Vec<String> {
  static WORD_REGEX: OnceLock<Regex> = OnceLock::new();
  let word_regex = WORD_REGEX.get_or_init(|| {
    #[allow(
      clippy::unwrap_used,
      reason = "regex pattern is statically known to be valid"
    )]
    Regex::new(r"\b[a-zA-Z0-9_-]+\b").unwrap()
  });

  let mut tokens = HashSet::new();

  for capture in word_regex.find_iter(text) {
    let token = capture.as_str().to_lowercase();
    if token.len() > 2 {
      // Skip very short tokens
      tokens.insert(token);
    }
  }

  tokens.into_iter().collect()
}

/// Represents a processed markdown document for search indexing.
///
/// This contains the rendered HTML content (which includes expanded includes)
/// that will be converted to plain text for indexing.
pub struct ProcessedDocument {
  /// Source file path
  pub source_path:  PathBuf,
  /// Rendered HTML content (after include expansion and rendering)
  pub html_content: String,
  /// Headers extracted from the document
  pub headers:      Vec<ndg_commonmark::Header>,
  /// Title of the document
  pub title:        Option<String>,
  /// HTML output path for the document
  pub html_path:    String,
}

/// Generate search index from standalone markdown files.
///
/// This function only processes markdown files that will be rendered as
/// standalone HTML pages. Files that are included in other documents via
/// `{=include=}` directives should not be passed to this function - their
/// content is already indexed as part of the parent document.
///
/// # Arguments
///
/// * `config` - The loaded configuration for documentation generation.
/// * `processed_docs` - Processed markdown documents with expanded includes.
///
/// # Errors
///
/// Returns an error if the search directory cannot be created or if any file
/// cannot be read or written.
///
/// # Panics
///
/// Panics if `config.input_dir` is `None` when processing markdown files.
pub fn generate_search_index(
  config: &Config,
  processed_docs: &[ProcessedDocument],
) -> Result<()> {
  if !config.is_search_enabled() {
    return Ok(());
  }

  info!("Generating search index...");

  // Create search directory
  let search_dir = config.output_dir.join("assets");
  fs::create_dir_all(&search_dir)?;

  let mut search_index = SearchIndex::new();
  let mut doc_id = 0;
  let mut markdown_count = 0;

  // Get max heading level for anchor indexing
  let max_heading_level = config.search_max_heading_level();

  // Process standalone markdown documents in parallel
  if !processed_docs.is_empty() {
    let documents: Result<Vec<_>> = processed_docs
      .par_iter()
      .map(|doc| {
        // Convert HTML to plain text for indexing
        // This includes all content from expanded includes
        let plain_text = html::content_to_plaintext(&doc.html_content);

        let title = doc.title.clone().unwrap_or_else(|| {
          doc
            .source_path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string()
        });

        // Use the provided HTML path
        let path = doc.html_path.clone();

        // Extract anchors from headers, filtering by max level
        let anchors: Vec<SearchAnchor> = doc
          .headers
          .iter()
          .filter(|h| h.level <= max_heading_level)
          .map(|h| {
            SearchAnchor {
              text:   h.text.clone(),
              id:     h.id.clone(),
              level:  h.level,
              tokens: tokenize(&h.text),
            }
          })
          .collect();

        let tokens = tokenize(&plain_text);
        let title_tokens = tokenize(&title);

        Ok((title, plain_text, path, tokens, title_tokens, anchors))
      })
      .collect();

    let documents = documents?;
    let documents_count = documents.len();
    markdown_count = documents_count;
    for (index, (title, content, path, tokens, title_tokens, anchors)) in
      documents.into_iter().enumerate()
    {
      let current_doc_id = doc_id + index + 1;
      let doc = SearchDocument {
        id: current_doc_id.to_string(),
        title,
        content,
        path,
        tokens,
        title_tokens,
        anchors,
      };
      search_index.add_document(doc);
    }
    doc_id += documents_count + 1;
  }

  // Process options if available
  let mut options_count = 0;
  if let Some(options_path) = &config.module_options
    && let Ok(options_content) = fs::read_to_string(options_path)
    && let Ok(options_data) = serde_json::from_str::<Value>(&options_content)
    && let Some(options_obj) = options_data.as_object()
  {
    for (key, option_value) in options_obj {
      let raw_description = option_value["description"].as_str().unwrap_or("");
      let plain_description = html::content_to_plaintext(raw_description);

      let title = format!("Option: {}", html_escape::encode_text(key));
      let tokens = tokenize(&plain_description);
      let title_tokens = tokenize(&title);

      search_index.add_document(SearchDocument {
        id: doc_id.to_string(),
        title,
        content: plain_description,
        // XXX: this leads to breakage sometimes, I don't regret it
        path: format!("options.html#option-{}", key.replace('.', "-")),
        tokens,
        title_tokens,
        // options don't have sub-anchors (or at least we hope they don't)
        anchors: Vec::new(),
      });

      doc_id += 1;
      options_count += 1;
    }
  }

  // Write search index as JSON with config
  let search_config = config
    .search
    .as_ref()
    .map_or_else(ndg_config::search::SearchConfig::default, Clone::clone);

  let search_data = SearchData {
    documents:       search_index.documents,
    min_word_length: search_config.min_word_length,
    stopwords:       search_config.stopwords.clone(),
    boost_title:     search_config.get_title_boost(),
    boost_content:   search_config.get_content_boost(),
    boost_anchor:    search_config.get_anchor_boost(),
  };

  let search_data_path = search_dir.join("search-data.json");
  fs::write(&search_data_path, serde_json::to_string(&search_data)?)
    .wrap_err_with(|| {
      format!(
        "Failed to write search data to {}",
        search_data_path.display()
      )
    })?;

  // Create search page
  create_search_page(config)?;

  let total_count = search_data.documents.len();
  info!(
    "Search index generated successfully: {markdown_count} markdown \
     documents, {options_count} options indexed ({total_count} total)"
  );

  Ok(())
}

/// Create the search page HTML and write it to the output directory.
///
/// # Errors
///
/// Returns an error if the search page cannot be rendered or written.
pub fn create_search_page(config: &Config) -> Result<()> {
  if !config.is_search_enabled() {
    return Ok(());
  }

  info!("Creating search page...");

  let mut context = HashMap::new();
  context.insert("title", format!("{} - Search", config.title));

  let html = template::render_search(config, &context)?;

  // Apply postprocessing if configured
  let processed_html = if let Some(ref postprocess) = config.postprocess {
    postprocess::process_html(&html, postprocess)?
  } else {
    html
  };

  let search_page_path = config.output_dir.join("search.html");
  fs::write(&search_page_path, &processed_html).wrap_err_with(|| {
    format!(
      "Failed to write search page to {}",
      search_page_path.display()
    )
  })?;

  Ok(())
}
