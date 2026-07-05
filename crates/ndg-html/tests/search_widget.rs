#![expect(clippy::expect_used, clippy::panic, reason = "Fine in tests")]
use std::fs;

mod common;

use common::test_config;
use ndg_config::{
  matchers::OptionNameMatch,
  options::{
    FilterConfig,
    OptionsConfig,
    OptionsPageMatch,
    OptionsPagesConfig,
  },
};
use ndg_html::{
  options::process_options,
  search::{SearchData, generate_search_index},
};
use serde_json::json;
use tempfile::TempDir;

// Mock an options.json
// This doesn't have to be accurate, but it has to demonstrate a good enough
// reference for testing. <name>, for example, is important for knowing
// whether we've regressed the stupid HTML escaper again...
fn option_data() -> serde_json::Value {
  json!({
      "hjem.users.<name>.clobberFiles": {
          "type": "boolean",
          "description": "Files to clobber when creating home directory",
          "default": false
      },
      "system.<config>.path": {
          "type": "string",
          "description": "System configuration path",
          "default": "/etc/config"
      }
  })
}

fn split_option_data() -> serde_json::Value {
  json!({
      "enable": {
          "type": "boolean",
          "description": "Root option",
          "default": false
      },
      "foo.bar.enable": {
          "type": "boolean",
          "description": "Enable foo bar",
          "default": false
      },
      "foo.bar.package": {
          "type": "package",
          "description": "Foo bar package",
          "default": null
      },
      "foo.bar.baz.quz.enable": {
          "type": "boolean",
          "description": "Enable deep quz",
          "default": true
      }
  })
}

#[test]
fn test_options_dot_filter_applies_to_html_and_search() {
  let temp_dir =
    TempDir::new().expect("Failed to create temp dir in options filter test");
  let output_dir = temp_dir.path();

  let options_file = output_dir.join("options.json");
  fs::write(&options_file, option_data().to_string())
    .expect("Failed to write options.json in options filter test");

  let mut config = ndg_config::Config {
    module_options: Some(options_file.clone()),
    options: Some(OptionsConfig {
      filter: Some(FilterConfig {
        prefix: Some("hjem.users".to_string()),
        ..Default::default()
      }),
      ..Default::default()
    }),
    ..test_config(output_dir)
  };
  config
    .options
    .as_mut()
    .expect("options config should be present")
    .validate()
    .expect("options pages config should validate");

  process_options(&config, &options_file)
    .expect("Failed to process filtered options");
  generate_search_index(&config, &[])
    .expect("Failed to generate filtered search index");

  let options_html = fs::read_to_string(output_dir.join("options.html"))
    .expect("Failed to read filtered options.html");
  assert!(options_html.contains("hjem.users.&lt;name&gt;.clobberFiles"));
  assert!(!options_html.contains("system.&lt;config&gt;.path"));

  let search_data =
    fs::read_to_string(output_dir.join("assets").join("search-data.json"))
      .expect("Failed to read filtered search-data.json");
  let search_data: SearchData = serde_json::from_str(&search_data)
    .expect("Failed to parse filtered search-data.json");
  let titles: Vec<_> = search_data
    .documents
    .iter()
    .map(|doc| doc.title.as_str())
    .collect();

  assert!(titles.iter().any(|title| title.contains("hjem.users")));
  assert!(!titles.iter().any(|title| title.contains("system.")));
}

#[test]
fn test_search_html_escape() {
  let temp_dir =
    TempDir::new().expect("Failed to create temp dir in search_widget test");
  let output_dir = temp_dir.path();

  let options_file = output_dir.join("options.json");
  fs::write(&options_file, option_data().to_string())
    .expect("Failed to write options.json in search_widget test");

  let config = ndg_config::Config {
    module_options: Some(options_file.clone()),
    ..test_config(output_dir)
  };

  process_options(&config, &options_file)
    .expect("Failed to process options in search_widget test");
  generate_search_index(&config, &[])
    .expect("Failed to generate search index in search_widget test");

  let search_data =
    fs::read_to_string(output_dir.join("assets").join("search-data.json"))
      .expect("Failed to read search-data.json in search_widget test");
  let search_data_parsed: SearchData = serde_json::from_str(&search_data)
    .expect("Failed to parse search-data.json in search_widget test");
  let search_docs: Vec<serde_json::Value> =
    serde_json::to_value(&search_data_parsed.documents)
      .expect("Failed to convert documents to Value")
      .as_array()
      .expect("Documents should be array")
      .clone();

  let options_html = fs::read_to_string(output_dir.join("options.html"))
    .expect("Failed to read options.html in search_widget test");

  for option_key in ["hjem.users.<name>.clobberFiles", "system.<config>.path"] {
    let search_entry = search_docs
      .iter()
      .find(|doc| {
        doc
          .get("title")
          .and_then(|v| v.as_str())
          .is_some_and(|title| {
            title.contains(
              option_key
                .split('.')
                .next()
                .expect("option_key should have at least one segment"),
            )
          })
      })
      .unwrap_or_else(|| panic!("Search entry not found for {option_key}"));

    let title = search_entry
      .get("title")
      .expect("Search entry missing title field")
      .as_str()
      .expect("Search entry title is not a string");
    assert!(
      title.contains(option_key),
      "Search title should contain the raw option key. Expected: \
       {option_key}, Found: {title}"
    );

    let escaped_key = option_key.replace('<', "&lt;").replace('>', "&gt;");
    assert!(
      options_html.contains(&escaped_key),
      "Options HTML should contain HTML-escaped key: {escaped_key}"
    );
  }
}

#[test]
fn test_split_options_pages_update_html_and_search_paths() {
  let temp_dir =
    TempDir::new().expect("Failed to create temp dir in split options test");
  let output_dir = temp_dir.path();

  let options_file = output_dir.join("options.json");
  fs::write(&options_file, split_option_data().to_string())
    .expect("Failed to write split options.json");

  let mut config = ndg_config::Config {
    module_options: Some(options_file.clone()),
    options: Some(OptionsConfig {
      pages: Some(OptionsPagesConfig {
        enabled: true,
        depth: 1,
        matches: vec![OptionsPageMatch {
          name: Some(OptionNameMatch {
            exact:          None,
            regex:          Some(r"^foo\.bar\.baz(\.|$)".to_string()),
            compiled_regex: None,
          }),
          depth: Some(3),
          ..Default::default()
        }],
        ..Default::default()
      }),
      ..Default::default()
    }),
    ..test_config(output_dir)
  };
  config
    .options
    .as_mut()
    .expect("options config should be present")
    .validate()
    .expect("options pages config should validate");

  process_options(&config, &options_file)
    .expect("Failed to process split options");
  generate_search_index(&config, &[])
    .expect("Failed to generate split search index");

  let index_html = fs::read_to_string(output_dir.join("options.html"))
    .expect("Failed to read split options index");
  assert!(index_html.contains("options-index-summary"));
  assert!(index_html.contains("options-index-list"));
  assert!(index_html.contains("option-page-row"));
  assert!(index_html.contains("<summary title=\"foo\""));
  assert!(index_html.contains("options/foo.html"));
  assert!(index_html.contains("options/foo.bar.baz.html"));
  assert!(!index_html.contains("data-section=\"option-groups\""));
  assert!(index_html.contains("Root option"));
  assert!(!index_html.contains("Enable foo bar"));

  let foo_html = fs::read_to_string(output_dir.join("options/foo.html"))
    .expect("Failed to read foo options page");
  assert!(foo_html.contains("option-page-breadcrumb"));
  assert!(foo_html.contains("../options.html"));
  assert!(foo_html.contains("option-page-meta"));
  assert!(foo_html.contains("data-section=\"option-groups\""));
  assert!(foo_html.contains("Option Groups"));
  assert!(foo_html.contains("option-page-sidebar-nav"));
  assert!(foo_html.contains("option-page-sidebar-next"));
  assert!(foo_html.contains("class=\"active\" href=\"../options/foo.html\""));
  assert!(foo_html.contains("../options/foo.bar.baz.html"));
  assert!(!foo_html.contains("option-page-adjacent"));
  assert!(!foo_html.contains("option-page-pagination"));
  assert!(foo_html.contains("foo.bar.enable"));
  assert!(foo_html.contains("foo.bar.package"));
  assert!(!foo_html.contains("foo.bar.baz.quz.enable"));

  let deep_html =
    fs::read_to_string(output_dir.join("options/foo.bar.baz.html"))
      .expect("Failed to read deep options page");
  assert!(deep_html.contains("option-page-breadcrumb"));
  assert!(deep_html.contains("data-section=\"option-groups\""));
  assert!(deep_html.contains("option-page-sidebar-nav"));
  assert!(deep_html.contains("option-page-sidebar-prev"));
  assert!(
    deep_html
      .contains("class=\"active\" href=\"../options/foo.bar.baz.html\"",)
  );
  assert!(!deep_html.contains("option-page-adjacent"));
  assert!(deep_html.contains("foo.bar.baz.quz.enable"));

  let search_data =
    fs::read_to_string(output_dir.join("assets").join("search-data.json"))
      .expect("Failed to read split search-data.json");
  let search_data: SearchData = serde_json::from_str(&search_data)
    .expect("Failed to parse split search-data.json");

  let path_for = |option_name: &str| {
    search_data
      .documents
      .iter()
      .find(|doc| doc.title == format!("Option: {option_name}"))
      .unwrap_or_else(|| panic!("Search entry not found for {option_name}"))
      .path
      .as_str()
  };

  assert!(
    path_for("foo.bar.enable").starts_with("options/foo.html#"),
    "foo.bar.enable should route to foo page"
  );
  assert!(
    path_for("foo.bar.baz.quz.enable").starts_with("options/foo.bar.baz.html#"),
    "deep option should route to overridden deep page"
  );
  assert!(
    path_for("enable").starts_with("options.html#"),
    "root option should stay on options.html"
  );
}
