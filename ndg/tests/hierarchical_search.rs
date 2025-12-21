#![allow(
  clippy::expect_used,
  clippy::panic,
  clippy::unwrap_used,
  reason = "Fine in tests"
)]

use std::fs;

use ndg::{
  config::{Config, search::SearchConfig},
  html::search::generate_search_index,
};
use ndg_commonmark::collect_markdown_files;
use serde_json::Value;
use tempfile::TempDir;

/// Test that `SearchAnchor` extraction works for markdown with multiple
/// heading levels
#[test]
fn test_search_anchor_extraction_from_markdown() {
  let temp_dir = TempDir::new().expect("Failed to create temp dir");
  let input_dir = temp_dir.path().join("input");
  let output_dir = temp_dir.path().join("output");
  fs::create_dir_all(&input_dir).expect("Failed to create input dir");
  fs::create_dir_all(&output_dir).expect("Failed to create output dir");

  // Create markdown file with multiple heading levels
  let md_content = r#"# Installation Guide

This is the main installation guide.

## Prerequisites

You need these things first.

### System Requirements

Hardware requirements here.

#### CPU Requirements

At least 2 cores.

## Installation Steps

Follow these steps.

### Download

Get the package.

### Configure

Set up your config.
"#;

  let md_file = input_dir.join("install.md");
  fs::write(&md_file, md_content).expect("Failed to write markdown file");

  let config = Config {
    input_dir: Some(input_dir.clone()),
    output_dir: output_dir.clone(),
    title: "Test Docs".to_string(),
    search: Some(SearchConfig {
      enable:            true,
      max_heading_level: 3, // only index H1, H2, H3
    }),
    ..Default::default()
  };

  let markdown_files = collect_markdown_files(&input_dir);

  generate_search_index(&config, &markdown_files)
    .expect("Failed to generate search index");

  // Read and parse the generated search data
  let search_data_path = output_dir.join("assets").join("search-data.json");
  let search_data = fs::read_to_string(&search_data_path)
    .expect("Failed to read search-data.json");
  let search_docs: Vec<Value> = serde_json::from_str(&search_data)
    .expect("Failed to parse search-data.json");

  // Should have exactly 1 document
  assert_eq!(search_docs.len(), 1, "Should have 1 search document");

  let doc = &search_docs[0];
  let anchors = doc
    .get("anchors")
    .expect("Document should have anchors field")
    .as_array()
    .expect("Anchors should be an array");

  // Should have 6 anchors (H1, H2, H3 only - H4 is excluded by
  // max_heading_level) H1: Installation Guide
  // H2: Prerequisites, Installation Steps
  // H3: System Requirements, Download, Configure
  assert_eq!(
    anchors.len(),
    6,
    "Should have 6 anchors (excluding H4 'CPU Requirements')"
  );

  // Verify the H1 anchor
  let h1_anchor = anchors
    .iter()
    .find(|a| a["level"] == 1)
    .expect("Should have H1 anchor");
  assert_eq!(h1_anchor["text"].as_str().unwrap(), "Installation Guide");
  assert_eq!(h1_anchor["id"].as_str().unwrap(), "installation-guide");
  assert_eq!(h1_anchor["level"].as_u64().unwrap(), 1);

  // Verify H2 anchors
  let h2_anchors: Vec<&Value> =
    anchors.iter().filter(|a| a["level"] == 2).collect();
  assert_eq!(h2_anchors.len(), 2, "Should have 2 H2 anchors");

  let h2_texts: Vec<&str> = h2_anchors
    .iter()
    .map(|a| a["text"].as_str().unwrap())
    .collect();
  assert!(h2_texts.contains(&"Prerequisites"));
  assert!(h2_texts.contains(&"Installation Steps"));

  // Verify H3 anchors
  let h3_anchors: Vec<&Value> =
    anchors.iter().filter(|a| a["level"] == 3).collect();
  assert_eq!(h3_anchors.len(), 3, "Should have 3 H3 anchors");

  let h3_texts: Vec<&str> = h3_anchors
    .iter()
    .map(|a| a["text"].as_str().unwrap())
    .collect();
  assert!(h3_texts.contains(&"System Requirements"));
  assert!(h3_texts.contains(&"Download"));
  assert!(h3_texts.contains(&"Configure"));

  // Verify no H4 anchors (should be filtered out by max_heading_level=3)
  let h4_anchors: Vec<&Value> =
    anchors.iter().filter(|a| a["level"] == 4).collect();
  assert_eq!(
    h4_anchors.len(),
    0,
    "Should have no H4 anchors due to max_heading_level=3"
  );
}

/// Test that max_heading_level filtering works correctly
#[test]
fn test_max_heading_level_filtering() {
  let temp_dir = TempDir::new().expect("Failed to create temp dir");
  let input_dir = temp_dir.path().join("input");
  let output_dir = temp_dir.path().join("output");
  fs::create_dir_all(&input_dir).expect("Failed to create input dir");
  fs::create_dir_all(&output_dir).expect("Failed to create output dir");

  let md_content = r#"# Title
## Section
### Subsection
#### Detail
##### Note
###### Fine Print
"#;

  let md_file = input_dir.join("test.md");
  fs::write(&md_file, md_content).expect("Failed to write markdown file");

  // Test with max_heading_level = 2 (only H1 and H2)
  let config = Config {
    input_dir: Some(input_dir.clone()),
    output_dir: output_dir.clone(),
    title: "Test".to_string(),
    search: Some(SearchConfig {
      enable:            true,
      max_heading_level: 2,
    }),
    ..Default::default()
  };

  let markdown_files = collect_markdown_files(&input_dir);

  generate_search_index(&config, &markdown_files)
    .expect("Failed to generate search index");

  let search_data_path = output_dir.join("assets").join("search-data.json");
  let search_data = fs::read_to_string(&search_data_path)
    .expect("Failed to read search-data.json");
  let search_docs: Vec<Value> = serde_json::from_str(&search_data)
    .expect("Failed to parse search-data.json");

  let anchors = search_docs[0]["anchors"]
    .as_array()
    .expect("Should have anchors");

  assert_eq!(
    anchors.len(),
    2,
    "Should only have 2 anchors with max_heading_level=2"
  );

  let levels: Vec<u64> = anchors
    .iter()
    .map(|a| a["level"].as_u64().unwrap())
    .collect();
  assert!(levels.contains(&1), "Should contain H1");
  assert!(levels.contains(&2), "Should contain H2");
  assert!(!levels.contains(&3), "Should not contain H3");
  assert!(!levels.contains(&4), "Should not contain H4");
}

/// Test that anchor tokens are properly generated for searching
#[test]
fn test_anchor_tokenization() {
  let temp_dir = TempDir::new().expect("Failed to create temp dir");
  let input_dir = temp_dir.path().join("input");
  let output_dir = temp_dir.path().join("output");
  fs::create_dir_all(&input_dir).expect("Failed to create input dir");
  fs::create_dir_all(&output_dir).expect("Failed to create output dir");

  let md_content = r"# NixOS Installation Guide

## System Requirements and Prerequisites
";

  let md_file = input_dir.join("guide.md");
  fs::write(&md_file, md_content).expect("Failed to write markdown file");

  let config = Config {
    input_dir: Some(input_dir.clone()),
    output_dir: output_dir.clone(),
    title: "Test".to_string(),
    search: Some(SearchConfig {
      enable:            true,
      max_heading_level: 6,
    }),
    ..Default::default()
  };

  let markdown_files = collect_markdown_files(&input_dir);

  generate_search_index(&config, &markdown_files)
    .expect("Failed to generate search index");

  let search_data_path = output_dir.join("assets").join("search-data.json");
  let search_data = fs::read_to_string(&search_data_path)
    .expect("Failed to read search-data.json");
  let search_docs: Vec<Value> = serde_json::from_str(&search_data)
    .expect("Failed to parse search-data.json");

  let anchors = search_docs[0]["anchors"]
    .as_array()
    .expect("Should have anchors");

  // Check H1 anchor tokens
  let h1_anchor = anchors
    .iter()
    .find(|a| a["level"] == 1)
    .expect("Should have H1");
  let h1_tokens = h1_anchor["tokens"].as_array().expect("Should have tokens");

  // Tokens should be lowercase and split by word boundaries
  let token_strings: Vec<String> = h1_tokens
    .iter()
    .map(|t| t.as_str().unwrap().to_string())
    .collect();

  assert!(
    token_strings.contains(&"nixos".to_string()),
    "Should contain 'nixos' token"
  );
  assert!(
    token_strings.contains(&"installation".to_string()),
    "Should contain 'installation' token"
  );
  assert!(
    token_strings.contains(&"guide".to_string()),
    "Should contain 'guide' token"
  );

  // Check H2 anchor tokens
  let h2_anchor = anchors
    .iter()
    .find(|a| a["level"] == 2)
    .expect("Should have H2");
  let h2_tokens = h2_anchor["tokens"].as_array().expect("Should have tokens");

  let token_strings: Vec<String> = h2_tokens
    .iter()
    .map(|t| t.as_str().unwrap().to_string())
    .collect();

  assert!(
    token_strings.contains(&"system".to_string()),
    "Should contain 'system' token"
  );
  assert!(
    token_strings.contains(&"requirements".to_string()),
    "Should contain 'requirements' token"
  );
  assert!(
    token_strings.contains(&"prerequisites".to_string()),
    "Should contain 'prerequisites' token"
  );
}

/// Test that documents without headings have empty anchors array
#[test]
fn test_document_without_headings() {
  let temp_dir = TempDir::new().expect("Failed to create temp dir");
  let input_dir = temp_dir.path().join("input");
  let output_dir = temp_dir.path().join("output");
  fs::create_dir_all(&input_dir).expect("Failed to create input dir");
  fs::create_dir_all(&output_dir).expect("Failed to create output dir");

  let md_content = "This is just some plain text without any headings.";

  let md_file = input_dir.join("plain.md");
  fs::write(&md_file, md_content).expect("Failed to write markdown file");

  let config = Config {
    input_dir: Some(input_dir.clone()),
    output_dir: output_dir.clone(),
    title: "Test".to_string(),
    search: Some(SearchConfig {
      enable:            true,
      max_heading_level: 3,
    }),
    ..Default::default()
  };

  let markdown_files = collect_markdown_files(&input_dir);

  generate_search_index(&config, &markdown_files)
    .expect("Failed to generate search index");

  let search_data_path = output_dir.join("assets").join("search-data.json");
  let search_data = fs::read_to_string(&search_data_path)
    .expect("Failed to read search-data.json");
  let search_docs: Vec<Value> = serde_json::from_str(&search_data)
    .expect("Failed to parse search-data.json");

  assert_eq!(search_docs.len(), 1);

  let anchors = search_docs[0]["anchors"]
    .as_array()
    .expect("Should have anchors field");

  assert_eq!(
    anchors.len(),
    0,
    "Document without headings should have empty anchors array"
  );
}

/// Test `SearchConfig` merge behavior
#[test]
fn test_search_config_merge() {
  let mut base = Config {
    search: Some(SearchConfig {
      enable:            true,
      max_heading_level: 3,
    }),
    ..Default::default()
  };

  let override_config = Config {
    search: Some(SearchConfig {
      enable:            false,
      max_heading_level: 5,
    }),
    ..Default::default()
  };

  base.merge(override_config);

  let search = base.search.as_ref().expect("Should have search config");
  assert!(!search.enable, "Enable should be overridden to false");
  assert_eq!(
    search.max_heading_level, 5,
    "Max heading level should be overridden to 5"
  );
}

/// Test `SearchConfig` partial merge (only enable field)
#[test]
fn test_search_config_partial_merge_enable() {
  let mut base = Config {
    search: Some(SearchConfig {
      enable:            true,
      max_heading_level: 3,
    }),
    ..Default::default()
  };

  let override_config = Config {
    search: Some(SearchConfig {
      enable:            false,
      max_heading_level: 3, // Same as base
    }),
    ..Default::default()
  };

  base.merge(override_config);

  let search = base.search.as_ref().expect("Should have search config");
  assert!(!search.enable, "Enable should be overridden");
  assert_eq!(
    search.max_heading_level, 3,
    "Max heading level should remain unchanged"
  );
}

/// Test backward compatibility with deprecated `generate_search` field
#[test]
fn test_deprecated_generate_search_compatibility() {
  #[allow(deprecated)]
  let config = Config {
    generate_search: true,
    search: None,
    ..Default::default()
  };

  assert!(
    config.is_search_enabled(),
    "Should respect deprecated generate_search field"
  );
  assert_eq!(
    config.search_max_heading_level(),
    3,
    "Should use default max_heading_level when search config is None"
  );
}

/// Test that new search.enable takes priority over deprecated `generate_search`
#[test]
fn test_search_config_priority_over_deprecated() {
  #[allow(deprecated)]
  let config = Config {
    generate_search: true, // deprecated, should be ignored
    search: Some(SearchConfig {
      enable:            false, // new config takes priority
      max_heading_level: 5,
    }),
    ..Default::default()
  };

  assert!(
    !config.is_search_enabled(),
    "New search.enable should override deprecated generate_search"
  );
  assert_eq!(config.search_max_heading_level(), 5);
}

/// Test that anchor IDs are properly generated
#[test]
fn test_anchor_id_generation() {
  let temp_dir = TempDir::new().expect("Failed to create temp dir");
  let input_dir = temp_dir.path().join("input");
  let output_dir = temp_dir.path().join("output");
  fs::create_dir_all(&input_dir).expect("Failed to create input dir");
  fs::create_dir_all(&output_dir).expect("Failed to create output dir");

  let md_content = r"# Getting Started

## Installation & Setup

## User's Guide
";

  let md_file = input_dir.join("test.md");
  fs::write(&md_file, md_content).expect("Failed to write markdown file");

  let config = Config {
    input_dir: Some(input_dir.clone()),
    output_dir: output_dir.clone(),
    title: "Test".to_string(),
    search: Some(SearchConfig {
      enable:            true,
      max_heading_level: 6,
    }),
    ..Default::default()
  };

  let markdown_files = collect_markdown_files(&input_dir);

  generate_search_index(&config, &markdown_files)
    .expect("Failed to generate search index");

  let search_data_path = output_dir.join("assets").join("search-data.json");
  let search_data = fs::read_to_string(&search_data_path)
    .expect("Failed to read search-data.json");
  let search_docs: Vec<Value> = serde_json::from_str(&search_data)
    .expect("Failed to parse search-data.json");

  let anchors = search_docs[0]["anchors"]
    .as_array()
    .expect("Should have anchors");

  // Find "Getting Started" anchor
  let getting_started = anchors
    .iter()
    .find(|a| a["text"].as_str().unwrap() == "Getting Started")
    .expect("Should have 'Getting Started' anchor");
  assert_eq!(getting_started["id"].as_str().unwrap(), "getting-started");

  // Find "Installation & Setup" anchor
  let installation = anchors
    .iter()
    .find(|a| a["text"].as_str().unwrap() == "Installation & Setup")
    .expect("Should have 'Installation & Setup' anchor");
  // Ampersand and spaces are converted: " & " becomes "---"
  assert_eq!(installation["id"].as_str().unwrap(), "installation---setup");

  // Find "User's Guide" anchor
  let users_guide = anchors
    .iter()
    .find(|a| a["text"].as_str().unwrap() == "User's Guide")
    .expect("Should have \"User's Guide\" anchor");
  // Apostrophe is kept as-is in slug generation
  assert_eq!(users_guide["id"].as_str().unwrap(), "user-s-guide");
}

/// Test multiple documents with varying anchor counts
#[test]
fn test_multiple_documents_with_anchors() {
  let temp_dir = TempDir::new().expect("Failed to create temp dir");
  let input_dir = temp_dir.path().join("input");
  let output_dir = temp_dir.path().join("output");
  fs::create_dir_all(&input_dir).expect("Failed to create input dir");
  fs::create_dir_all(&output_dir).expect("Failed to create output dir");

  // Document 1: Multiple headings
  let doc1_content = r"# First Document
## Section A
## Section B
";
  let doc1_file = input_dir.join("doc1.md");
  fs::write(&doc1_file, doc1_content).expect("Failed to write doc1");

  // Document 2: No headings
  let doc2_content = "Just plain text.";
  let doc2_file = input_dir.join("doc2.md");
  fs::write(&doc2_file, doc2_content).expect("Failed to write doc2");

  // Document 3: One heading
  let doc3_content = "# Only Title";
  let doc3_file = input_dir.join("doc3.md");
  fs::write(&doc3_file, doc3_content).expect("Failed to write doc3");

  let config = Config {
    input_dir: Some(input_dir.clone()),
    output_dir: output_dir.clone(),
    title: "Test".to_string(),
    search: Some(SearchConfig {
      enable:            true,
      max_heading_level: 6,
    }),
    ..Default::default()
  };

  let markdown_files = collect_markdown_files(&input_dir);

  generate_search_index(&config, &markdown_files)
    .expect("Failed to generate search index");

  let search_data_path = output_dir.join("assets").join("search-data.json");
  let search_data = fs::read_to_string(&search_data_path)
    .expect("Failed to read search-data.json");
  let search_docs: Vec<Value> = serde_json::from_str(&search_data)
    .expect("Failed to parse search-data.json");

  assert_eq!(search_docs.len(), 3, "Should have 3 documents");

  // Find each document and check anchor counts
  let doc1 = search_docs
    .iter()
    .find(|d| d["path"].as_str().unwrap() == "doc1.html")
    .expect("Should find doc1");
  let doc1_anchors = doc1["anchors"].as_array().expect("Should have anchors");
  assert_eq!(doc1_anchors.len(), 3, "Doc1 should have 3 anchors");

  let doc2 = search_docs
    .iter()
    .find(|d| d["path"].as_str().unwrap() == "doc2.html")
    .expect("Should find doc2");
  let doc2_anchors = doc2["anchors"].as_array().expect("Should have anchors");
  assert_eq!(doc2_anchors.len(), 0, "Doc2 should have 0 anchors");

  let doc3 = search_docs
    .iter()
    .find(|d| d["path"].as_str().unwrap() == "doc3.html")
    .expect("Should find doc3");
  let doc3_anchors = doc3["anchors"].as_array().expect("Should have anchors");
  assert_eq!(doc3_anchors.len(), 1, "Doc3 should have 1 anchor");
}
