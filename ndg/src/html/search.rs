use std::{
  collections::{HashMap, HashSet},
  fs,
  path::PathBuf,
  sync::OnceLock,
};

use color_eyre::eyre::{Context, Result};
use log::info;
use rayon::prelude::*;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{config::Config, html};

/// Optimized search document with tokenized content
#[derive(Debug, Serialize, Deserialize)]
pub struct SearchDocument {
  id:           String,
  title:        String,
  content:      String,
  path:         String,
  tokens:       Vec<String>,
  title_tokens: Vec<String>,
}

/// Search index structure for efficient lookup
#[derive(Debug, Serialize, Deserialize)]
pub struct SearchIndex {
  documents: Vec<SearchDocument>,
  token_map: HashMap<String, Vec<usize>>,
}

impl SearchIndex {
  fn new() -> Self {
    Self {
      documents: Vec::new(),
      token_map: HashMap::new(),
    }
  }

  fn add_document(&mut self, doc: SearchDocument) {
    let doc_id = self.documents.len();
    for token in &doc.tokens {
      self
        .token_map
        .entry(token.clone())
        .or_default()
        .push(doc_id);
    }
    for token in &doc.title_tokens {
      self
        .token_map
        .entry(token.clone())
        .or_default()
        .push(doc_id);
    }

    self.documents.push(doc);
  }
}

/// Tokenize text into searchable terms
fn tokenize(text: &str) -> Vec<String> {
  static WORD_REGEX: OnceLock<Regex> = OnceLock::new();
  let word_regex =
    WORD_REGEX.get_or_init(|| Regex::new(r"\b[a-zA-Z0-9_-]+\b").unwrap());

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

/// Generate search index from markdown files
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
  markdown_files: &[PathBuf],
) -> Result<()> {
  if !config.generate_search {
    return Ok(());
  }

  info!("Generating search index...");

  // Create search directory
  let search_dir = config.output_dir.join("assets");
  fs::create_dir_all(&search_dir)?;

  let mut search_index = SearchIndex::new();
  let mut doc_id = 0;

  // Process markdown files in parallel if available and input_dir is provided
  if !markdown_files.is_empty() && config.input_dir.is_some() {
    let input_dir = config.input_dir.as_ref().unwrap();

    let documents: Result<Vec<_>> = markdown_files
      .par_iter()
      .map(|file_path| {
        let content = fs::read_to_string(file_path).wrap_err_with(|| {
          format!(
            "Failed to read file for search indexing: {}",
            file_path.display()
          )
        })?;

        let title = extract_title(&content).unwrap_or_else(|| {
          file_path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string()
        });

        let plain_text = crate::utils::html::content_to_plaintext(&content);

        let rel_path =
          file_path.strip_prefix(input_dir).wrap_err_with(|| {
            format!(
              "Failed to determine relative path for {}",
              file_path.display()
            )
          })?;

        let mut output_path = rel_path.to_owned();
        output_path.set_extension("html");

        let tokens = tokenize(&plain_text);
        let title_tokens = tokenize(&title);

        Ok(SearchDocument {
          id: doc_id.to_string(),
          title,
          content: plain_text,
          path: output_path.to_string_lossy().to_string(),
          tokens,
          title_tokens,
        })
      })
      .collect();

    let documents = documents?;
    for doc in documents {
      search_index.add_document(doc);
      doc_id += 1;
    }
  }

  // Process options if available
  if let Some(options_path) = &config.module_options {
    if let Ok(options_content) = fs::read_to_string(options_path) {
      if let Ok(options_data) = serde_json::from_str::<Value>(&options_content)
      {
        if let Some(options_obj) = options_data.as_object() {
          for (key, option_value) in options_obj {
            let raw_description =
              option_value["description"].as_str().unwrap_or("");
            let plain_description =
              crate::utils::html::content_to_plaintext(raw_description);

            let title = format!("Option: {key}");
            let tokens = tokenize(&plain_description);
            let title_tokens = tokenize(&title);

            search_index.add_document(SearchDocument {
              id: doc_id.to_string(),
              title,
              content: plain_description,
              path: format!("options.html#option-{}", key.replace('.', "-")),
              tokens,
              title_tokens,
            });

            doc_id += 1;
          }
        }
      }
    }
  }

  // Write search index as JSON
  let search_data_path = search_dir.join("search-data.json");
  fs::write(
    &search_data_path,
    serde_json::to_string(&search_index.documents)?,
  )
  .wrap_err_with(|| {
    format!(
      "Failed to write search data to {}",
      search_data_path.display()
    )
  })?;

  // Create search page
  create_search_page(config)?;

  info!(
    "Search index generated successfully: {} documents indexed",
    search_index.documents.len()
  );

  Ok(())
}

/// Extract title from markdown content (first H1)
fn extract_title(content: &str) -> Option<String> {
  ndg_commonmark::utils::extract_title_from_markdown(content)
}

/// Create the search page HTML and write it to the output directory.
///
/// # Errors
///
/// Returns an error if the search page cannot be rendered or written.
pub fn create_search_page(config: &Config) -> Result<()> {
  if !config.generate_search {
    return Ok(());
  }

  info!("Creating search page...");

  let mut context = HashMap::new();
  context.insert("title", format!("{} - Search", config.title));

  let html = html::template::render_search(config, &context)?;

  let search_page_path = config.output_dir.join("search.html");
  fs::write(&search_page_path, &html).wrap_err_with(|| {
    format!(
      "Failed to write search page to {}",
      search_page_path.display()
    )
  })?;

  Ok(())
}
