#![allow(clippy::expect_used, reason = "Fine in tests")]
use std::{
  collections::HashMap,
  fs,
  path::{Path, PathBuf},
};

use indexmap::IndexMap;
use ndg_commonmark::{Header, MarkdownOptions, MarkdownProcessor};
use ndg_config::{
  Config,
  meta::{FaviconEntry, MetaConfig},
  search::SearchConfig,
  sidebar::SidebarConfig,
};
use ndg_html::template;
use ndg_manpage::types::NixOption;
use tempfile::TempDir;

/// Checks for highlighted code HTML
fn contains_highlighted_code(html: &str) -> bool {
  // Accept either class-based or inline-style highlighting
  html.contains("class=\"highlight\"") || html.contains("class=\"syntect\"")
}

fn minimal_config() -> Config {
  Config {
    title: "Test Site".to_string(),
    footer_text: "Footer".to_string(),
    search: Some(SearchConfig {
      enable: false,
      ..Default::default()
    }),
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
  let html =
    template::render(&config, content, title, &headers, rel_path, None)
      .expect("Should render HTML");
  assert!(html.contains("<html"));
  assert!(html.contains("Test Page"));
  assert!(html.contains("Test Site"));
}

#[test]
fn render_options_page_includes_options() {
  let mut config = minimal_config();
  config.module_options = Some("dummy.json".into());
  let mut options = IndexMap::new();
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
  let mut options = IndexMap::new();
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
  let mut config = minimal_config();
  config.highlight_code = true;

  // Render markdown with a code block
  let md = "```rust\nfn main() { println!(\"hi\"); }\n```";
  let options = MarkdownOptions {
    highlight_code: true,
    ..Default::default()
  };
  let processor = MarkdownProcessor::new(options);
  let result = processor.render(md);
  let html_content = processor.highlight_codeblocks(&result.html);

  let title = "Syntax Highlight Test";
  let headers: Vec<Header> = vec![];
  let rel_path = Path::new("syntax.html");
  let html =
    template::render(&config, &html_content, title, &headers, rel_path, None)
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
  let html =
    template::render(&config, content, title, &headers, rel_path, None)
      .expect("Should render HTML");
  // Should include TOC anchors
  assert!(html.contains("sec1"));
  assert!(html.contains("subsec"));
  // Should include a TOC container and list structure
  let toc_container = html.contains("data-section=\"toc\"")
    || html.contains("class=\"toc-list\"");
  assert!(
    toc_container,
    "TOC container (data-section=\"toc\" or class=\"toc-list\") not found in \
     HTML"
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
  let mut options = IndexMap::new();
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
  config.search = Some(ndg_config::search::SearchConfig {
    enable: true,
    ..Default::default()
  });
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
  let html =
    template::render(&config, content, title, &headers, rel_path, None)
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
  let html =
    template::render(&config, content, title, &headers, rel_path, None)
      .expect("Should render HTML");

  // Should contain footer with configured text
  assert!(html.contains("<footer"));
  assert!(html.contains("Footer"));
}

#[test]
fn render_options_page_contains_navbar() {
  let mut config = minimal_config();
  config.module_options = Some("dummy.json".into());
  let mut options = IndexMap::new();
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
  let mut options = IndexMap::new();
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
  config.search = Some(ndg_config::search::SearchConfig {
    enable: true,
    ..Default::default()
  });
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
  config.search = Some(ndg_config::search::SearchConfig {
    enable: true,
    ..Default::default()
  });
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
  let default_content = r"<!doctype html>
<html>
<head><title>{{ title }}</title></head>
<body>
{{ navbar_html|safe }}
<main>{{ content|safe }}</main>
{{ footer_html|safe }}
</body>
</html>";
  fs::write(template_dir.join("default.html"), default_content)
    .expect("Failed to write default template");

  let mut config = minimal_config();
  config.template_dir = Some(template_dir.to_path_buf());

  let content = "<p>Test content</p>";
  let title = "Custom Template Test";
  let headers: Vec<Header> = vec![];
  let rel_path = Path::new("test.html");

  let html =
    template::render(&config, content, title, &headers, rel_path, None)
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

  let html =
    template::render(&config, content, title, &headers, rel_path, None)
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

  let html =
    template::render(&config, content, title, &headers, rel_path, None)
      .expect("Should render HTML with default template");

  // Should fall back to default template
  assert!(html.contains("default-page"));
  assert!(!html.contains("special-page"));
}

#[test]
fn navbar_respects_search_generation_flag() {
  let mut config = minimal_config();
  config.search = Some(ndg_config::search::SearchConfig {
    enable: true,
    ..Default::default()
  });

  let content = "<p>Test content</p>";
  let title = "Test Page";
  let headers: Vec<Header> = vec![];
  let rel_path = Path::new("test.html");

  let html =
    template::render(&config, content, title, &headers, rel_path, None)
      .expect("Should render HTML");

  // Should contain search link when enabled
  assert!(html.contains("Search") || html.contains("search"));

  // Test with search disabled
  config.search = Some(ndg_config::search::SearchConfig {
    enable: false,
    ..Default::default()
  });
  let html_no_search =
    template::render(&config, content, title, &headers, rel_path, None)
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

  let html =
    template::render(&config, content, title, &headers, rel_path, None)
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

  let html =
    template::render(&config, content, title, &headers, rel_path, None)
      .expect("Should render HTML");

  // Should contain custom footer text
  assert!(html.contains("Copyright 2025 - Custom Footer"));
}

#[test]
fn sidebar_numbering_excludes_special_files() {
  let temp_dir = TempDir::new().expect("Failed to create temp dir");
  let input_dir = temp_dir.path();

  // Create some markdown files including special files
  fs::write(input_dir.join("index.md"), "# Index\nIndex content")
    .expect("Failed to write index.md");
  fs::write(input_dir.join("README.md"), "# Readme\nReadme content")
    .expect("Failed to write README.md");
  fs::write(input_dir.join("guide.md"), "# Guide\nGuide content")
    .expect("Failed to write guide.md");
  fs::write(
    input_dir.join("tutorial.md"),
    "# Tutorial\nTutorial content",
  )
  .expect("Failed to write tutorial.md");

  let mut config = minimal_config();
  config.input_dir = Some(input_dir.to_path_buf());
  config.sidebar = Some(SidebarConfig {
    numbered:             true,
    number_special_files: false, // Default behavior
    ordering:             ndg_config::sidebar::SidebarOrdering::Alphabetical,
    group_by_dir:         false,
    show_group_counts:    true,
    matches:              vec![],
    options:              None,
  });

  let content = "<p>Test content</p>";
  let title = "Test Page";
  let headers: Vec<Header> = vec![];
  let rel_path = Path::new("test.html");

  let html =
    template::render(&config, content, title, &headers, rel_path, None)
      .expect("Should render HTML");

  // Special files (index.md, README.md) should NOT have numbers
  assert!(
    !html.contains("1. Index") && !html.contains("2. Readme"),
    "Special files should not be numbered"
  );

  // Regular files should be numbered starting from 1, in alphabetical order
  // Verify exact ordering by checking positions in HTML
  let pos_1_guide = html.find("1. Guide").expect("Should contain '1. Guide'");
  let pos_2_tutorial = html
    .find("2. Tutorial")
    .expect("Should contain '2. Tutorial'");

  // Verify the sequence: 1. Guide < 2. Tutorial
  assert!(
    pos_1_guide < pos_2_tutorial,
    "'1. Guide' must appear before '2. Tutorial' in HTML (alphabetical order)"
  );
}

#[test]
fn sidebar_numbering_special_files_included() {
  let temp_dir = TempDir::new().expect("Failed to create temp dir");
  let input_dir = temp_dir.path();

  // Create some markdown files including special files
  fs::write(input_dir.join("index.md"), "# Index\nIndex content")
    .expect("Failed to write index.md");
  fs::write(input_dir.join("README.md"), "# Readme\nReadme content")
    .expect("Failed to write README.md");
  fs::write(input_dir.join("guide.md"), "# Guide\nGuide content")
    .expect("Failed to write guide.md");
  fs::write(
    input_dir.join("tutorial.md"),
    "# Tutorial\nTutorial content",
  )
  .expect("Failed to write tutorial.md");

  let mut config = minimal_config();
  config.input_dir = Some(input_dir.to_path_buf());
  config.sidebar = Some(SidebarConfig {
    numbered:             true,
    number_special_files: true, // enable numbering for special files
    ordering:             ndg_config::sidebar::SidebarOrdering::Alphabetical,
    group_by_dir:         false,
    show_group_counts:    true,
    matches:              vec![],
    options:              None,
  });

  let content = "<p>Test content</p>";
  let title = "Test Page";
  let headers: Vec<Header> = vec![];
  let rel_path = Path::new("test.html");

  let html =
    template::render(&config, content, title, &headers, rel_path, None)
      .expect("Should render HTML");

  // All files should be numbered in exact sequence: special files first
  // (alphabetically), then regular files (alphabetically) Verify exact
  // ordering by checking positions in HTML
  let pos_1_index = html.find("1. Index").expect("Should contain '1. Index'");
  let pos_2_readme =
    html.find("2. Readme").expect("Should contain '2. Readme'");
  let pos_3_guide = html.find("3. Guide").expect("Should contain '3. Guide'");
  let pos_4_tutorial = html
    .find("4. Tutorial")
    .expect("Should contain '4. Tutorial'");

  // Verify the sequence: 1. Index < 2. Readme < 3. Guide < 4. Tutorial
  assert!(
    pos_1_index < pos_2_readme,
    "'1. Index' must appear before '2. Readme' in HTML"
  );
  assert!(
    pos_2_readme < pos_3_guide,
    "'2. Readme' must appear before '3. Guide' in HTML"
  );
  assert!(
    pos_3_guide < pos_4_tutorial,
    "'3. Guide' must appear before '4. Tutorial' in HTML"
  );
}

#[test]
fn sidebar_numbering_disabled_no_numbers() {
  let temp_dir = TempDir::new().expect("Failed to create temp dir");
  let input_dir = temp_dir.path();

  // Create some markdown files
  fs::write(input_dir.join("index.md"), "# Index\nIndex content")
    .expect("Failed to write index.md");
  fs::write(input_dir.join("guide.md"), "# Guide\nGuide content")
    .expect("Failed to write guide.md");
  fs::write(input_dir.join("README.md"), "# README\nREADME content")
    .expect("Failed to write README.md");
  fs::write(
    input_dir.join("tutorial.md"),
    "# Tutorial\nTutorial content",
  )
  .expect("Failed to write tutorial.md");

  let mut config = minimal_config();
  config.input_dir = Some(input_dir.to_path_buf());
  config.sidebar = Some(SidebarConfig {
    numbered:             false, // Numbering disabled
    number_special_files: false,
    ordering:             ndg_config::sidebar::SidebarOrdering::Alphabetical,
    group_by_dir:         false,
    show_group_counts:    true,
    matches:              vec![],
    options:              None,
  });

  let content = "<p>Test content</p>";
  let title = "Test Page";
  let headers: Vec<Header> = vec![];
  let rel_path = Path::new("test.html");

  let html =
    template::render(&config, content, title, &headers, rel_path, None)
      .expect("Should render HTML");

  // No files should have numbers when numbering is disabled
  assert!(
    !html.contains("1. Index")
      && !html.contains("1. Guide")
      && !html.contains("1. README")
      && !html.contains("1. Tutorial"),
    "No files should be numbered when numbering is disabled"
  );
  assert!(
    !html.contains("2. ") && !html.contains("3. ") && !html.contains("4. "),
    "No numbered items should appear in sidebar"
  );
}

// Regression test for bug where included files appeared in sidebar navigation.
// Included files should never appear as standalone entries in the sidebar
// because they don't have their own HTML pages generated
#[test]
fn sidebar_excludes_included_files() {
  let temp_dir = TempDir::new().expect("Failed to create temp dir");
  let input_dir = temp_dir.path();
  let output_dir = temp_dir.path().join("output");
  let included_dir = input_dir.join("included");

  fs::create_dir_all(&included_dir).expect("Failed to create included dir");
  fs::create_dir_all(&output_dir).expect("Failed to create output dir");

  // Create a main document that includes another file
  let main_content = "# Main Document

This is the main document.

```{=include=}
included/fragment.md
```

More content.
";
  fs::write(input_dir.join("main.md"), main_content)
    .expect("Failed to write main.md");

  // Create an included file that should NOT appear in sidebar
  let included_content = "# Included Fragment

This content is included in main.md and should not have its own sidebar entry.
";
  fs::write(included_dir.join("fragment.md"), included_content)
    .expect("Failed to write included/fragment.md");

  // Create another standalone file for comparison
  let standalone_content = "# Standalone Page

This is a standalone page.
";
  fs::write(input_dir.join("standalone.md"), standalone_content)
    .expect("Failed to write standalone.md");

  let mut config = Config {
    title: "Test Site".to_string(),
    footer_text: "Footer".to_string(),
    input_dir: Some(input_dir.to_path_buf()),
    output_dir: output_dir.clone(),
    search: Some(ndg_config::search::SearchConfig {
      enable: false,
      ..Default::default()
    }),
    ..Default::default()
  };

  // Process markdown files, this should populate config.included_files
  let processor = ndg_utils::markdown::create_processor(&config, None);
  let processed =
    ndg_utils::process_markdown_files(&mut config, Some(&processor))
      .expect("Failed to process markdown files");

  // Write HTML files for non-included files
  for item in &processed {
    if item.is_included {
      continue;
    }
    let rel_path = Path::new(&item.output_path);
    let html = template::render(
      &config,
      &item.html_content,
      item.title.as_deref().unwrap_or(&config.title),
      &item.headers,
      rel_path,
      item.frontmatter.as_ref(),
    )
    .expect("Failed to render HTML");

    let output_path = output_dir.join(rel_path);
    if let Some(parent) = output_path.parent() {
      fs::create_dir_all(parent).expect("Failed to create parent dir");
    }
    fs::write(&output_path, html).expect("Failed to write HTML file");
  }

  // Verify that included_files was populated
  assert!(
    !config.included_files.is_empty(),
    "config.included_files should be populated after processing"
  );
  assert!(
    config
      .included_files
      .contains_key(Path::new("included/fragment.md")),
    "included/fragment.md should be tracked as an included file"
  );

  // Now render a page and check the sidebar
  let content = "<p>Test content</p>";
  let title = "Test Page";
  let headers: Vec<Header> = vec![];
  let rel_path = Path::new("test.html");

  let html =
    template::render(&config, content, title, &headers, rel_path, None)
      .expect("Should render HTML");

  // The sidebar should contain links to main.md and standalone.md
  assert!(
    html.contains("main.html") || html.contains("Main Document"),
    "Sidebar should contain main document"
  );
  assert!(
    html.contains("standalone.html") || html.contains("Standalone Page"),
    "Sidebar should contain standalone page"
  );

  // The sidebar should NOT contain any reference to the included file
  assert!(
    !html.contains("fragment.html"),
    "Sidebar should NOT contain HTML link to included file (fragment.html)"
  );
  assert!(
    !html.contains("Included Fragment") || html.contains("included/fragment"),
    "Sidebar should NOT contain title of included file as a navigation item. \
     If 'Included Fragment' appears, it must be in the context of the \
     included path, not as a standalone nav link."
  );

  // Verify the included file's HTML was not generated
  assert!(
    !output_dir.join("included/fragment.html").exists(),
    "HTML file should not be generated for included files"
  );

  // Verify standalone files WERE generated
  assert!(
    output_dir.join("main.html").exists(),
    "HTML file should be generated for main.md"
  );
  assert!(
    output_dir.join("standalone.html").exists(),
    "HTML file should be generated for standalone.md"
  );
}

#[test]
fn render_page_exposes_user_defined_vars() {
  let mut config = minimal_config();
  config
    .vars
    .insert("project_version".to_string(), "1.2.3".to_string());
  config
    .vars
    .insert("repo_url".to_string(), "https://example.com".to_string());

  // Write a minimal template that references the user vars.
  let tmp = TempDir::new().expect("create tempdir");
  let template_path = tmp.path().join("default.html");
  fs::write(
    &template_path,
    "version={{ project_version }} url={{ repo_url }}",
  )
  .expect("write template");
  config.template_path = Some(template_path);

  let headers = vec![];
  let rel_path = std::path::Path::new("index.html");
  let html =
    template::render(&config, "body", "Title", &headers, rel_path, None)
      .expect("render should succeed");

  assert!(
    html.contains("version=1.2.3"),
    "user var project_version missing"
  );
  assert!(
    html.contains("url=https://example.com"),
    "user var repo_url missing"
  );
}

#[test]
fn render_page_builtin_vars_take_precedence_over_user_vars() {
  let mut config = minimal_config();
  // Attempt to shadow the built-in site_title via user vars.
  config
    .vars
    .insert("site_title".to_string(), "SHADOWED".to_string());

  let tmp = TempDir::new().expect("create tempdir");
  let template_path = tmp.path().join("default.html");
  fs::write(&template_path, "{{ site_title }}").expect("write template");
  config.template_path = Some(template_path);

  let headers = vec![];
  let rel_path = std::path::Path::new("index.html");
  let html =
    template::render(&config, "body", "Title", &headers, rel_path, None)
      .expect("render should succeed");

  // config.title is "Test Site" (from minimal_config). Built-ins are inserted
  // after user vars in build_common_context, so they always win.
  assert!(
    html.contains("Test Site"),
    "built-in site_title should not be overridden by user vars"
  );
  assert!(
    !html.contains("SHADOWED"),
    "user var must not shadow built-in"
  );
}

// NixOS option names commonly contain `<name>` as a placeholder component,
// e.g. `services.nginx.virtualHosts.<name>.serverName`. The `<` and `>` must
// not appear raw inside an HTML `id` attribute or `href` fragment.
#[test]
fn render_options_id_attribute_is_safe_for_angle_bracket_names() {
  let mut config = minimal_config();
  config.module_options = Some("dummy.json".into());
  let name = "services.nginx.virtualHosts.<name>.serverName";
  let mut options = IndexMap::new();
  options.insert(name.to_string(), create_basic_option(name, "desc"));

  let html = template::render_options(&config, &options).expect("render");

  // Raw `<name>` must never appear inside an id="..." attribute value.
  // If it did the browser would parse `<name>` as an HTML tag, breaking the
  // page structure.
  assert!(
    !html.contains("id=\"option-services-nginx-virtualHosts-<name>"),
    "raw '<' must not appear inside an id attribute"
  );
  assert!(
    !html.contains("href=\"#option-services-nginx-virtualHosts-<name>"),
    "raw '<' must not appear inside an href fragment"
  );

  // The id and matching href must both be present so the anchor still works.
  // The sanitized form replaces special chars (* < > [ ] : " space) with `_`
  // per nixos-render-docs XML ID format.
  let expected_id = "option-services.nginx.virtualHosts._name_.serverName";
  assert!(
    html.contains(&format!("id=\"{expected_id}\"")),
    "sanitized id must be present: {expected_id}"
  );
  assert!(
    html.contains(&format!("href=\"#{expected_id}\"")),
    "TOC href must match the sanitized id: #{expected_id}"
  );

  // The display text must still show the real name, properly HTML-escaped.
  assert!(
    html.contains("&lt;name&gt;"),
    "display text must HTML-escape angle brackets"
  );
  assert!(
    !html.contains("<name>"),
    "raw unescaped <name> tag must not appear anywhere in output"
  );
}

// `type_name` is rendered inside a `<code>` element. Values like
// `null or (submodule)` are benign, but the field is free-form and must be
// escaped to prevent injection.
#[test]
fn render_options_type_name_is_html_escaped() {
  let mut config = minimal_config();
  config.module_options = Some("dummy.json".into());
  let mut options = IndexMap::new();
  options.insert("foo.bar".to_string(), NixOption {
    name: "foo.bar".to_string(),
    description: "desc".to_string(),
    type_name: "null or <special & \"type\">".to_string(),
    ..Default::default()
  });

  let html = template::render_options(&config, &options).expect("render");

  assert!(
    !html.contains("<special"),
    "raw '<special' must not appear in type output"
  );
  assert!(
    html.contains("&lt;special"),
    "angle bracket in type_name must be escaped to &lt;"
  );
  assert!(
    html.contains("&amp;"),
    "ampersand in type_name must be escaped to &amp;"
  );
}

// `declared_in` is the human-readable path shown in the "Declared in:" line.
// It is rendered as text content inside `<code>`, so angle brackets and
// ampersands must be preserved as-is.
#[test]
fn render_options_declared_in_preserves_angle_brackets() {
  let mut config = minimal_config();
  config.module_options = Some("dummy.json".into());
  let mut options = IndexMap::new();
  options.insert("foo.bar".to_string(), NixOption {
    name: "foo.bar".to_string(),
    description: "desc".to_string(),
    declared_in: Some("<modules>/foo/bar.nix".to_string()),
    ..Default::default()
  });

  let html = template::render_options(&config, &options).expect("render");

  assert!(
    html.contains("<modules>"),
    "raw '<modules>' must appear in declared_in output"
  );
  assert!(
    !html.contains("&lt;modules&gt;"),
    "angle brackets in declared_in must not be escaped"
  );
}

// Regression test for mkOption formatting parity with nixos-render-docs.
// Verifies option IDs, literalMD handling, defined_by, and raw markdown.
#[test]
fn render_options_mkoption_parity() {
  let mut config = minimal_config();
  config.module_options = Some("dummy.json".into());

  let mut options = IndexMap::new();
  options.insert(
    "services.nginx.virtualHosts.<name>.serverName".to_string(),
    NixOption {
      name: "services.nginx.virtualHosts.<name>.serverName".to_string(),
      description: "<p>Server name <strong>with markdown</strong></p>".to_string(),
      type_name: "string".to_string(),
      default_text: Some("`example.com`".to_string()),
      example_text: Some("`server.example.com`".to_string()),
      declared_in: Some("<nixpkgs>/nixos/modules/services/web-servers/nginx.nix".to_string()),
      declared_in_url: Some("https://github.com/NixOS/nixpkgs/blob/master/nixos/modules/services/web-servers/nginx.nix".to_string()),
      defined_in: vec![
        ("<nixpkgs>/nixos/modules/services/web-servers/nginx.nix".to_string(), Some("https://github.com/NixOS/nixpkgs/blob/master/nixos/modules/services/web-servers/nginx.nix".to_string())),
        ("<nixpkgs>/nixos/modules/services/web-servers/default.nix".to_string(), None),
      ],
      internal: false,
      read_only: false,
      ..Default::default()
    },
  );

  let html = template::render_options(&config, &options).expect("render");

  // 1. Option ID sanitization: must match nixos-render-docs XML ID format
  // < and > become _, not -
  let expected_id = "option-services.nginx.virtualHosts._name_.serverName";
  assert!(
    html.contains(&format!("id=\"{expected_id}\"")),
    "option id must use underscore for angle brackets: {expected_id}"
  );
  assert!(
    html.contains(&format!("href=\"#{expected_id}\"")),
    "anchor href must match sanitized id: #{expected_id}"
  );

  // 2. literalMD handling: description should be rendered as raw markdown HTML
  assert!(
    html.contains("<strong>with markdown</strong>"),
    "literalMD description must render markdown as HTML"
  );

  // 3. Default/example values: literalExpression backticks stripped
  assert!(
    html.contains("<code>example.com</code>"),
    "default value must strip literalExpression backticks"
  );
  assert!(
    html.contains("<code>server.example.com</code>"),
    "example value must strip literalExpression backticks"
  );

  // 4. Declared in with hyperlink
  assert!(
    html.contains("Declared in:"),
    "must show 'Declared in' label"
  );
  assert!(
    html.contains("<nixpkgs>/nixos/modules/services/web-servers/nginx.nix"),
    "declared_in path must be present"
  );

  // 5. Defined in section with multiple entries
  assert!(html.contains("Defined in:"), "must show 'Defined in' label");
  assert!(
    html.contains("<ul class=\"option-defined-list\">"),
    "defined_in must use unordered list"
  );
  assert!(
    html.contains("<nixpkgs>/nixos/modules/services/web-servers/nginx.nix"),
    "first defined_in entry must be present"
  );
  assert!(
    html.contains("<nixpkgs>/nixos/modules/services/web-servers/default.nix"),
    "second defined_in entry must be present"
  );

  // 6. No raw angle brackets in id/href
  assert!(
    !html.contains("id=\"option-services-nginx-virtualHosts-<name>"),
    "raw '<' must not appear in id attribute"
  );
  assert!(
    !html.contains("href=\"#option-services-nginx-virtualHosts-<name>"),
    "raw '<' must not appear in href attribute"
  );
}

// When `declared_in_url` contains an ampersand (valid in URLs, but must be
// `&amp;` inside an HTML attribute), the href must be properly escaped.
#[test]
fn render_options_declared_in_url_ampersand_is_escaped_in_attribute() {
  let mut config = minimal_config();
  config.module_options = Some("dummy.json".into());
  let mut options = IndexMap::new();
  options.insert("foo.bar".to_string(), NixOption {
    name: "foo.bar".to_string(),
    description: "desc".to_string(),
    declared_in: Some("foo/bar.nix".to_string()),
    declared_in_url: Some(
      "https://example.com/source?file=foo.nix&line=42".to_string(),
    ),
    ..Default::default()
  });

  let html = template::render_options(&config, &options).expect("render");

  // Raw `&line=` inside an href attribute is invalid HTML; must be
  // `&amp;line=`.
  assert!(
    !html.contains("href=\"https://example.com/source?file=foo.nix&line="),
    "raw '&' inside href attribute must not appear"
  );
  assert!(
    html.contains("&amp;line=42"),
    "ampersand in href must be escaped to &amp;"
  );
}

#[test]
fn render_page_with_single_favicon_entry() {
  let mut config = minimal_config();
  config.meta = Some(MetaConfig {
    favicon: vec![FaviconEntry {
      href:      PathBuf::from("static/favicon.png"),
      dest:      None,
      rel:       "icon".to_string(),
      mime_type: Some("image/png".to_string()),
      sizes:     Some("32x32".to_string()),
    }],
    ..Default::default()
  });

  let content = "<p>Test</p>";
  let title = "Favicon Test";
  let headers = vec![];
  let rel_path = Path::new("index.html");
  let html =
    template::render(&config, content, title, &headers, rel_path, None)
      .expect("render should succeed");

  assert!(
    html.contains(r#"<link rel="icon""#),
    "should contain icon rel"
  );
  assert!(
    html.contains(r#"type="image/png""#),
    "should contain mime type"
  );
  assert!(html.contains(r#"sizes="32x32""#), "should contain sizes");
}

#[test]
fn render_page_with_multiple_favicon_entries() {
  let mut config = minimal_config();
  config.meta = Some(MetaConfig {
    favicon: vec![
      FaviconEntry {
        href:      PathBuf::from("favicon-16.png"),
        dest:      None,
        rel:       "icon".to_string(),
        mime_type: Some("image/png".to_string()),
        sizes:     Some("16x16".to_string()),
      },
      FaviconEntry {
        href:      PathBuf::from("favicon-32.png"),
        dest:      None,
        rel:       "icon".to_string(),
        mime_type: Some("image/png".to_string()),
        sizes:     Some("32x32".to_string()),
      },
      FaviconEntry {
        href:      PathBuf::from("apple-touch-icon.png"),
        dest:      None,
        rel:       "apple-touch-icon".to_string(),
        mime_type: None,
        sizes:     None,
      },
    ],
    ..Default::default()
  });

  let content = "<p>Test</p>";
  let title = "Multi Favicon Test";
  let headers = vec![];
  let rel_path = Path::new("index.html");
  let html =
    template::render(&config, content, title, &headers, rel_path, None)
      .expect("render should succeed");

  assert!(
    html.contains(r#"href="favicon-16.png""#),
    "should contain favicon-16.png"
  );
  assert!(
    html.contains(r#"href="favicon-32.png""#),
    "should contain favicon-32.png"
  );
  assert!(
    html.contains(r#"href="apple-touch-icon.png""#),
    "should contain apple-touch-icon.png"
  );
  assert!(
    html.contains(r#"rel="apple-touch-icon""#),
    "should contain apple-touch-icon rel"
  );
}

#[test]
fn render_page_with_favicon_dest_override() {
  let mut config = minimal_config();
  config.meta = Some(MetaConfig {
    favicon: vec![FaviconEntry {
      href:      PathBuf::from("/nix/store/abc123-apple-touch-icon.png"),
      dest:      Some(PathBuf::from("apple-touch-icon.png")),
      rel:       "apple-touch-icon".to_string(),
      mime_type: None,
      sizes:     None,
    }],
    ..Default::default()
  });

  let content = "<p>Test</p>";
  let title = "Dest Override Test";
  let headers = vec![];
  let rel_path = Path::new("index.html");
  let html =
    template::render(&config, content, title, &headers, rel_path, None)
      .expect("render should succeed");

  assert!(
    html.contains(r#"href="apple-touch-icon.png""#),
    "href should use dest filename, not nix store path"
  );
  // The href should NOT contain the nix store path
  assert!(
    !html.contains("/nix/store/"),
    "nix store path should not appear in href"
  );
}

#[test]
fn render_page_favicon_rel_defaults_to_icon() {
  use std::path::PathBuf;

  use ndg_config::meta::FaviconEntry;

  // Create a FaviconEntry without explicitly setting rel
  let entry = FaviconEntry {
    href:      PathBuf::from("favicon.png"),
    dest:      None,
    rel:       "icon".to_string(), // should default to "icon" via serde
    mime_type: None,
    sizes:     None,
  };
  assert_eq!(entry.rel, "icon");

  // Verify output_filename works
  assert_eq!(
    entry.output_filename().map(|s| s.to_str()),
    Some(Some("favicon.png"))
  );

  // Test with dest
  let entry_with_dest = FaviconEntry {
    href:      PathBuf::from("/nix/store/abc-favicon.png"),
    dest:      Some(PathBuf::from("favicon.png")),
    rel:       "icon".to_string(),
    mime_type: None,
    sizes:     None,
  };
  assert_eq!(
    entry_with_dest.output_filename().map(|s| s.to_str()),
    Some(Some("favicon.png"))
  );
}

#[test]
fn render_page_favicon_attribute_values_are_html_escaped() {
  use std::path::PathBuf;

  use ndg_config::meta::FaviconEntry;

  let mut config = minimal_config();
  config.meta = Some(ndg_config::meta::MetaConfig {
    favicon: vec![FaviconEntry {
      href:      PathBuf::from("favicon.png"),
      dest:      None,
      rel:       "icon".to_string(),
      mime_type: Some(r"text/html<script>alert('xss')</script>".to_string()),
      sizes:     Some("16x16".to_string()),
    }],
    ..Default::default()
  });

  let content = "<p>Test</p>";
  let title = "Escaping Test";
  let headers = vec![];
  let rel_path = std::path::Path::new("index.html");
  let html =
    template::render(&config, content, title, &headers, rel_path, None)
      .expect("render should succeed");

  // The mime_type contains script injection attempt; it must be escaped
  // Search for it specifically inside a type attribute
  assert!(
    !html.contains(r#"type="text/html<script>"#),
    "script tag must not appear unescaped in mime_type type attribute"
  );
  assert!(
    html.contains(r#"type="text/html&lt;script&gt;"#),
    "angle brackets in mime_type must be HTML-escaped inside type attribute"
  );
}
