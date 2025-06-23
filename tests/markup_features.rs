ndg/tests/markup_features.rs
```
```ndg/tests/markup_features.rs
//! Regression tests for each individual markup feature supported by the crate.
//! These tests ensure that all custom markdown extensions and processing features
//! remain stable and are not broken by future changes.

use ndg::formatter::markdown::process_markdown_string;
use ndg::config::Config;

/// Helper to check if HTML output contains all expected substrings.
fn assert_html_contains(html: &str, expected: &[&str]) {
    for &needle in expected {
        assert!(
            html.contains(needle),
            "Expected HTML to contain '{}', but it did not.\nFull HTML:\n{}",
            needle,
            html
        );
    }
}

#[test]
fn test_admonition_note() {
    let config = Config::default();
    let md = "::: {.note}\nThis is a note.\n:::";
    let html = process_markdown_string(md, &config);
    assert_html_contains(
        &html,
        &[
            r#"<div class="admonition note""#,
            r#"<p class="admonition-title">Note</p>"#,
            "This is a note.",
        ],
    );
}

#[test]
fn test_role_command() {
    let config = Config::default();
    let md = "{command}`ls -l`";
    let html = process_markdown_string(md, &config);
    assert_html_contains(&html, &[r#"<code class="command">ls -l</code>"#]);
}

#[test]
fn test_role_option() {
    let config = Config::default();
    let md = "{option}`services.nginx.enable`";
    let html = process_markdown_string(md, &config);
    assert_html_contains(&html, &[r#"<code class="nixos-option">services.nginx.enable</code>"#]);
}

#[test]
fn test_command_prompt() {
    let config = Config::default();
    let md = "`$ echo hi`";
    let html = process_markdown_string(md, &config);
    assert_html_contains(
        &html,
        &[r#"<code class="terminal"><span class="prompt">$</span> echo hi</code>"#],
    );
}

#[test]
fn test_repl_prompt() {
    let config = Config::default();
    let md = "`nix-repl> 1 + 1`";
    let html = process_markdown_string(md, &config);
    assert_html_contains(
        &html,
        &[r#"<code class="nix-repl"><span class="prompt">nix-repl&gt;</span> 1 + 1</code>"#],
    );
}

#[test]
fn test_inline_anchor() {
    let config = Config::default();
    let md = "Go here []{#target}.";
    let html = process_markdown_string(md, &config);
    assert_html_contains(
        &html,
        &[r#"<span id="target" class="nixos-anchor"></span>"#],
    );
}

#[test]
fn test_list_item_with_anchor() {
    let config = Config::default();
    let md = "- []{#item1} Item 1";
    let html = process_markdown_string(md, &config);
    assert_html_contains(
        &html,
        &[r#"<span id="item1" class="nixos-anchor"></span> Item 1"#],
    );
}

#[test]
fn test_explicit_header_anchor() {
    let config = Config::default();
    let md = "## Section {#sec}";
    let html = process_markdown_string(md, &config);
    assert_html_contains(
        &html,
        &[r#"<h2 id="sec">Section</h2>"#],
    );
}

#[test]
fn test_figure_block() {
    let config = Config::default();
    let md = "::: {.figure #fig1}\n# Figure Title\nFigure content\n:::";
    let html = process_markdown_string(md, &config);
    assert_html_contains(
        &html,
        &[
            r#"<figure id="fig1">"#,
            r#"<figcaption>Figure Title</figcaption>"#,
            "Figure content",
        ],
    );
}

#[test]
fn test_definition_list() {
    let config = Config::default();
    let md = "Term\n:   Definition";
    let html = process_markdown_string(md, &config);
    assert_html_contains(
        &html,
        &[
            "<dl>",
            "<dt>Term</dt>",
            "<dd>Definition</dd>",
            "</dl>",
        ],
    );
}

#[test]
fn test_option_reference() {
    let config = Config::default();
    let md = "`foo.bar.baz`";
    let html = process_markdown_string(md, &config);
    // Option references may be rendered as <code> or as a link depending on context
    assert!(
        html.contains(r#"<code>foo.bar.baz</code>"#) || html.contains(r#"option-foo-bar-baz"#),
        "Expected option reference in HTML, got:\n{}",
        html
    );
}

#[test]
fn test_myst_role_markup() {
    let config = Config::default();
    let md = r#"<span class="command-markup">foo</span>"#;
    let html = process_markdown_string(md, &config);
    assert_html_contains(&html, &[r#"<code class="command">foo</code>"#]);
}

#[test]
fn test_autolink() {
    let config = Config::default();
    let md = "Visit https://example.com for info.";
    let html = process_markdown_string(md, &config);
    assert_html_contains(
        &html,
        &[r#"<a href="https://example.com">https://example.com</a>"#],
    );
}

#[test]
fn test_header_extraction() {
    use ndg::formatter::markdown::extract_headers;
    let md = "# Title\n\n## Section {#sec}\n### Subsection";
    let (headers, title) = extract_headers(md);
    assert_eq!(title.as_deref(), Some("Title"));
    assert_eq!(headers[0].text, "Title");
    assert_eq!(headers[0].level, 1);
    assert_eq!(headers[1].id, "sec");
    assert_eq!(headers[2].level, 3);
}

#[test]
fn test_raw_inline_anchor() {
    let config = Config::default();
    let md = "[]{#anchor}";
    let html = process_markdown_string(md, &config);
    assert_html_contains(&html, &[r#"<span id="anchor" class="nixos-anchor"></span>"#]);
}

#[test]
fn test_block_and_inline_code() {
    let config = Config::default();
    let md = "Here is `inline code`.\n\n```\nblock code\n```";
    let html = process_markdown_string(md, &config);
    assert_html_contains(&html, &["<code>inline code</code>", "<pre><code>block code"]);
}

#[test]
fn test_tables_footnotes_strikethrough_tasklists() {
    let config = Config::default();
    let md = "\
| A | B |\n|---|---|\n| 1 | 2 |\n\n\
Here is a footnote.[^1]\n\n[^1]: Footnote text.\n\n\
~~strikethrough~~\n\n\
- [x] Task done\n- [ ] Task not done";
    let html = process_markdown_string(md, &config);
    assert_html_contains(
        &html,
        &[
            "<table>",
            "<del>strikethrough</del>",
            r#"<li class="task-list-item">"#,
            "Footnote text",
        ],
    );
}