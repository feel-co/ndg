use std::{collections::HashMap, path::Path};

use ndg::{
    config::Config,
    formatter::{markdown::Header, options::NixOption},
    html::template,
};

fn minimal_config() -> Config {
    Config {
        title: "Test Site".to_string(),
        footer_text: "Footer".to_string(),
        generate_search: false,
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
        template::render(&config, content, title, &headers, rel_path).expect("Should render HTML");
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
        NixOption {
            name: "foo.bar".to_string(),
            description: "desc".to_string(),
            ..Default::default()
        },
    );
    let html = template::render_options(&config, &options).expect("Should render options");
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
        NixOption {
            name: "foo.bar".to_string(),
            description: "desc for foo.bar".to_string(),
            type_name: "string".to_string(),
            default_text: Some("defaultval".to_string()),
            example_text: Some("exampleval".to_string()),
            ..Default::default()
        },
    );
    let html = template::render_options(&config, &options).expect("Should render options");
    assert!(html.contains("foo.bar"));
    assert!(html.contains("desc for foo.bar"));
    assert!(html.contains("defaultval"));
    assert!(html.contains("exampleval"));
    assert!(html.contains("string"));
}

#[test]
fn render_page_with_headers_toc() {
    let config = minimal_config();
    let content = "<h1>Title</h1><p>Body</p>";
    let title = "Test Page";
    let headers = vec![
        Header { level: 1, text: "Section 1".to_string(), id: "sec1".to_string() },
        Header { level: 2, text: "Subsection".to_string(), id: "subsec".to_string() },
    ];
    let rel_path = Path::new("index.html");
    let html = template::render(&config, content, title, &headers, rel_path).expect("Should render HTML");
    // Should include TOC anchors
    assert!(html.contains("sec1"));
    assert!(html.contains("subsec"));
    // Should include the TOC structure
    assert!(html.contains("toc") || html.contains("<ul>") || html.contains("<li>"));
}

#[test]
fn render_options_page_with_multiple_options() {
    let mut config = minimal_config();
    config.module_options = Some("dummy.json".into());
    let mut options = HashMap::new();
    options.insert(
        "foo.bar".to_string(),
        NixOption {
            name: "foo.bar".to_string(),
            description: "desc1".to_string(),
            ..Default::default()
        },
    );
    options.insert(
        "foo.baz".to_string(),
        NixOption {
            name: "foo.baz".to_string(),
            description: "desc2".to_string(),
            ..Default::default()
        },
    );
    let html = template::render_options(&config, &options).expect("Should render options");
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
    let html = template::render_search(&config, &context).expect("Should render search");
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
