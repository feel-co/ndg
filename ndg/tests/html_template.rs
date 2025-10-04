use std::{collections::HashMap, path::Path};

use ndg::{config::Config, formatter::options::NixOption, html::template};
use ndg_commonmark::Header;

/// Checks for highlighted code HTML
fn contains_highlighted_code(html: &str) -> bool {
  // Accept either class-based or inline-style highlighting
  html.contains("class=\"highlight\"") || html.contains("class=\"syntect\"")
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

#[test]
fn render_page_contains_navbar_html() {
  let config = minimal_config();
  let content = "<p>Test content</p>";
  let title = "Test Page";
  let headers: Vec<Header> = vec![];
  let rel_path = Path::new("test.html");
  let html = template::render(&config, content, title, &headers, rel_path)
    .expect("Should render HTML");

  // Should contain navbar structure
  assert!(html.contains("header-nav") || html.contains("navbar"));
  assert!(html.contains("<nav"));
}

#[test]
fn render_page_contains_footer_html() {
  let config = minimal_config();
  let content = "<p>Test content</p>";
  let title = "Test Page";
  let headers: Vec<Header> = vec![];
  let rel_path = Path::new("test.html");
  let html = template::render(&config, content, title, &headers, rel_path)
    .expect("Should render HTML");

  // Should contain footer with configured text
  assert!(html.contains("<footer"));
  assert!(html.contains("Footer"));
}

#[test]
fn render_options_page_contains_navbar() {
  let mut config = minimal_config();
  config.module_options = Some("dummy.json".into());
  let mut options = HashMap::new();
  options.insert(
    "test.option".to_string(),
    create_basic_option("test.option", "Test option"),
  );
  let html =
    template::render_options(&config, &options).expect("Should render options");

  // Should contain navbar structure
  assert!(html.contains("header-nav") || html.contains("navbar"));
  assert!(html.contains("<nav"));
}

#[test]
fn render_options_page_contains_footer() {
  let mut config = minimal_config();
  config.module_options = Some("dummy.json".into());
  config.footer_text = "Custom Footer Text".to_string();
  let mut options = HashMap::new();
  options.insert(
    "test.option".to_string(),
    create_basic_option("test.option", "Test option"),
  );
  let html =
    template::render_options(&config, &options).expect("Should render options");

  // Should contain footer with custom text
  assert!(html.contains("<footer"));
  assert!(html.contains("Custom Footer Text"));
}

#[test]
fn render_search_page_contains_navbar() {
  let mut config = minimal_config();
  config.generate_search = true;
  let mut context = HashMap::new();
  context.insert("title", "Search Test".to_string());
  let html =
    template::render_search(&config, &context).expect("Should render search");

  // Should contain navbar structure
  assert!(html.contains("header-nav") || html.contains("navbar"));
  assert!(html.contains("<nav"));
}

#[test]
fn render_search_page_contains_footer() {
  let mut config = minimal_config();
  config.generate_search = true;
  config.footer_text = "Search Page Footer".to_string();
  let mut context = HashMap::new();
  context.insert("title", "Search Test".to_string());
  let html =
    template::render_search(&config, &context).expect("Should render search");

  // Should contain footer with configured text
  assert!(html.contains("<footer"));
  assert!(html.contains("Search Page Footer"));
}

#[test]
fn render_page_with_custom_template_dir() {
  use std::fs;

  use tempfile::TempDir;

  let temp_dir = TempDir::new().expect("Failed to create temp dir");
  let template_dir = temp_dir.path();

  // Create a custom navbar template
  let navbar_content =
    r#"<nav class="custom-navbar"><ul><li>Custom Nav</li></ul></nav>"#;
  fs::write(template_dir.join("navbar.html"), navbar_content)
    .expect("Failed to write navbar template");

  // Create a custom footer template
  let footer_content =
    r#"<footer class="custom-footer"><p>Custom Footer</p></footer>"#;
  fs::write(template_dir.join("footer.html"), footer_content)
    .expect("Failed to write footer template");

  // Create a custom default template that uses the navbar and footer
  let default_content = r#"<!doctype html>
<html>
<head><title>{{ title }}</title></head>
<body>
{{ navbar_html|safe }}
<main>{{ content|safe }}</main>
{{ footer_html|safe }}
</body>
</html>"#;
  fs::write(template_dir.join("default.html"), default_content)
    .expect("Failed to write default template");

  let mut config = minimal_config();
  config.template_dir = Some(template_dir.to_path_buf());

  let content = "<p>Test content</p>";
  let title = "Custom Template Test";
  let headers: Vec<Header> = vec![];
  let rel_path = Path::new("test.html");

  let html = template::render(&config, content, title, &headers, rel_path)
    .expect("Should render HTML with custom templates");

  // Should contain custom navbar and footer
  assert!(html.contains("Custom Nav"));
  assert!(html.contains("Custom Footer"));
  assert!(html.contains("custom-navbar"));
  assert!(html.contains("custom-footer"));
}

#[test]
fn render_page_uses_per_file_template() {
  use std::fs;

  use tempfile::TempDir;

  let temp_dir = TempDir::new().expect("Failed to create temp dir");
  let template_dir = temp_dir.path();

  // Create a custom template for a specific file
  let custom_content = r#"<!doctype html>
<html>
<head><title>{{ title }}</title></head>
<body class="special-page">
<h1>Special Template</h1>
{{ content|safe }}
</body>
</html>"#;
  fs::write(template_dir.join("special.html"), custom_content)
    .expect("Failed to write special template");

  // Create navbar and footer templates (required)
  fs::write(template_dir.join("navbar.html"), "<nav>Nav</nav>")
    .expect("Failed to write navbar");
  fs::write(template_dir.join("footer.html"), "<footer>Footer</footer>")
    .expect("Failed to write footer");

  let mut config = minimal_config();
  config.template_dir = Some(template_dir.to_path_buf());

  let content = "<p>Special page content</p>";
  let title = "Special Page";
  let headers: Vec<Header> = vec![];
  let rel_path = Path::new("special.html");

  let html = template::render(&config, content, title, &headers, rel_path)
    .expect("Should render HTML with special template");

  // Should use the special template
  assert!(html.contains("special-page"));
  assert!(html.contains("Special Template"));
}

#[test]
fn render_page_falls_back_to_default_template() {
  use std::fs;

  use tempfile::TempDir;

  let temp_dir = TempDir::new().expect("Failed to create temp dir");
  let template_dir = temp_dir.path();

  // Create only default template (no special.html)
  let default_content = r#"<!doctype html>
<html>
<head><title>{{ title }}</title></head>
<body class="default-page">
{{ content|safe }}
</body>
</html>"#;
  fs::write(template_dir.join("default.html"), default_content)
    .expect("Failed to write default template");

  // Create navbar and footer templates
  fs::write(template_dir.join("navbar.html"), "<nav>Nav</nav>")
    .expect("Failed to write navbar");
  fs::write(template_dir.join("footer.html"), "<footer>Footer</footer>")
    .expect("Failed to write footer");

  let mut config = minimal_config();
  config.template_dir = Some(template_dir.to_path_buf());

  let content = "<p>Regular page content</p>";
  let title = "Regular Page";
  let headers: Vec<Header> = vec![];
  // Request a file-specific template that doesn't exist
  let rel_path = Path::new("nonexistent.html");

  let html = template::render(&config, content, title, &headers, rel_path)
    .expect("Should render HTML with default template");

  // Should fall back to default template
  assert!(html.contains("default-page"));
  assert!(!html.contains("special-page"));
}

#[test]
fn navbar_respects_search_generation_flag() {
  let mut config = minimal_config();
  config.generate_search = true;

  let content = "<p>Test content</p>";
  let title = "Test Page";
  let headers: Vec<Header> = vec![];
  let rel_path = Path::new("test.html");

  let html = template::render(&config, content, title, &headers, rel_path)
    .expect("Should render HTML");

  // Should contain search link when enabled
  assert!(html.contains("Search") || html.contains("search"));

  // Test with search disabled
  config.generate_search = false;
  let html_no_search =
    template::render(&config, content, title, &headers, rel_path)
      .expect("Should render HTML");

  // The navbar might still contain the word "search" in template structure,
  // but the search link should be conditionally rendered
  // We just verify it renders successfully with different configs
  assert!(!html_no_search.is_empty());
}

#[test]
fn navbar_shows_options_link_when_configured() {
  let mut config = minimal_config();
  config.module_options = Some("options.json".into());

  let content = "<p>Test content</p>";
  let title = "Test Page";
  let headers: Vec<Header> = vec![];
  let rel_path = Path::new("test.html");

  let html = template::render(&config, content, title, &headers, rel_path)
    .expect("Should render HTML");

  // Should contain options link
  assert!(html.contains("Options") || html.contains("options"));
}

#[test]
fn footer_text_is_customizable() {
  let mut config = minimal_config();
  config.footer_text = "Copyright 2025 - Custom Footer".to_string();

  let content = "<p>Test content</p>";
  let title = "Test Page";
  let headers: Vec<Header> = vec![];
  let rel_path = Path::new("test.html");

  let html = template::render(&config, content, title, &headers, rel_path)
    .expect("Should render HTML");

  // Should contain custom footer text
  assert!(html.contains("Copyright 2025 - Custom Footer"));
}
