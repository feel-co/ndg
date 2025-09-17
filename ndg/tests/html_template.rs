use std::{collections::HashMap, path::Path};

use ndg::{config::Config, formatter::options::NixOption, html::template};
use ndg_commonmark::Header;

/// Helper to check for highlighted code HTML (syntect output)
fn contains_highlighted_code(html: &str) -> bool {
  // Accept either class-based or inline-style highlighting
  html.contains("class=\"highlight\"")
    || html.contains("class=\"syntect\"")
    || html.contains("style=\"")
}

fn minimal_config() -> Config {
  Config {
    title: "Test Site".to_string(),
    footer_text: "Footer".to_string(),
    generate_search: false,
    ..Default::default()
  }
}

fn create_basic_option(name: &str, description: &str) -> NixOption {
  NixOption {
    name: name.to_string(),
    description: description.to_string(),
    ..Default::default()
  }
}

fn create_detailed_option(
  name: &str,
  description: &str,
  type_name: &str,
  default_text: Option<&str>,
  example_text: Option<&str>,
) -> NixOption {
  NixOption {
    name: name.to_string(),
    description: description.to_string(),
    type_name: type_name.to_string(),
    default_text: default_text.map(std::string::ToString::to_string),
    example_text: example_text.map(std::string::ToString::to_string),
    ..Default::default()
  }
}

#[test]
fn render_basic_page_renders_html() {
  let config = minimal_config();
  let content = "<h1>Title</h1><p>Body</p>";
  let title = "Test Page";
  let headers: Vec<Header> = vec![];
  let rel_path = Path::new("index.html");
  let html = template::render(&config, content, title, &headers, rel_path)
    .expect("Should render HTML");
  assert!(html.contains("<html"));
  assert!(html.contains("Test Page"));
  assert!(html.contains("Test Site"));
}

#[test]
fn render_options_page_includes_options() {
  let mut config = minimal_config();
  config.module_options = Some("dummy.json".into());
  let mut options = HashMap::new();
  options.insert(
    "foo.bar".to_string(),
    create_basic_option("foo.bar", "desc"),
  );
  let html =
    template::render_options(&config, &options).expect("Should render options");
  assert!(html.contains("foo.bar"));
  assert!(html.contains("Options"));
}

#[test]
fn render_options_page_renders_description() {
  let mut config = minimal_config();
  config.module_options = Some("dummy.json".into());
  let mut options = HashMap::new();
  options.insert(
    "foo.bar".to_string(),
    create_detailed_option(
      "foo.bar",
      "desc for foo.bar",
      "string",
      Some("defaultval"),
      Some("exampleval"),
    ),
  );
  let html =
    template::render_options(&config, &options).expect("Should render options");
  assert!(html.contains("foo.bar"));
  assert!(html.contains("desc for foo.bar"));
  assert!(html.contains("defaultval"));
  assert!(html.contains("exampleval"));
  assert!(html.contains("string"));
}

#[test]
fn render_page_with_syntax_highlighting() {
  use ndg_commonmark::{MarkdownOptions, MarkdownProcessor};
  let mut config = minimal_config();
  config.highlight_code = true;

  // Render markdown with a code block
  let md = "```rust\nfn main() { println!(\"hi\"); }\n```";
  let mut options = MarkdownOptions::default();
  options.highlight_code = true;
  let processor = MarkdownProcessor::new(options);
  let result = processor.render(md);
  let html_content = processor.highlight_codeblocks(&result.html);

  let title = "Syntax Highlight Test";
  let headers: Vec<Header> = vec![];
  let rel_path = Path::new("syntax.html");
  let html =
    template::render(&config, &html_content, title, &headers, rel_path)
      .expect("Should render HTML with syntax highlighting");
  assert!(
    contains_highlighted_code(&html),
    "HTML output should contain syntax-highlighted code"
  );
}

#[test]
fn render_page_with_headers_toc() {
  let config = minimal_config();
  let content = "<h1>Title</h1><p>Body</p>";
  let title = "Test Page";
  let headers = vec![
    Header {
      level: 1,
      text:  "Section 1".to_string(),
      id:    "sec1".to_string(),
    },
    Header {
      level: 2,
      text:  "Subsection".to_string(),
      id:    "subsec".to_string(),
    },
  ];
  let rel_path = Path::new("index.html");
  let html = template::render(&config, content, title, &headers, rel_path)
    .expect("Should render HTML");
  // Should include TOC anchors
  assert!(html.contains("sec1"));
  assert!(html.contains("subsec"));
  // Should include a TOC container and list structure
  let toc_container =
    html.contains("id=\"toc\"") || html.contains("class=\"toc\"");
  assert!(
    toc_container,
    "TOC container (id or class) not found in HTML"
  );
  assert!(
    html.contains("<ul>") && html.contains("<li>"),
    "TOC list structure missing"
  );
}

#[test]
fn render_options_page_with_multiple_options() {
  let mut config = minimal_config();
  config.module_options = Some("dummy.json".into());
  let mut options = HashMap::new();
  options.insert(
    "foo.bar".to_string(),
    create_basic_option("foo.bar", "desc1"),
  );
  options.insert(
    "foo.baz".to_string(),
    create_basic_option("foo.baz", "desc2"),
  );
  let html =
    template::render_options(&config, &options).expect("Should render options");
  assert!(html.contains("foo.bar"));
  assert!(html.contains("foo.baz"));
  assert!(html.contains("desc1"));
  assert!(html.contains("desc2"));
}

#[test]
fn render_search_page_respects_flag() {
  let mut config = minimal_config();
  config.generate_search = true;
  let mut context = HashMap::new();
  context.insert("title", "Search Test".to_string());
  let html =
    template::render_search(&config, &context).expect("Should render search");
  assert!(html.contains("Search Test"));
  assert!(html.contains("Search"));
}

#[test]
fn render_search_page_disabled_returns_err() {
  let config = minimal_config();
  let context = HashMap::new();
  let result = template::render_search(&config, &context);
  assert!(result.is_err());
}
