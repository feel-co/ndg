use ndg_commonmark::{MarkdownProcessor, MarkdownOptions, Header};

/// Check if HTML output contains all expected substrings.
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

fn processor() -> MarkdownProcessor {
    MarkdownProcessor::new(MarkdownOptions::default())
}

#[test]
fn test_admonition_note() {
    let md = "::: {.note}\nThis is a note.\n:::";
    let html = processor().render(md).html;
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
    let md = "{command}`ls -l`";
    let html = processor().render(md).html;
    assert_html_contains(&html, &[r#"<code class="command">ls -l</code>"#]);
}

#[test]
fn test_role_option() {
    let md = "{option}`services.nginx.enable`";
    let html = processor().render(md).html;
    assert_html_contains(
        &html,
        &[r#"<code class="nixos-option">services.nginx.enable</code>"#],
    );
}

#[test]
fn test_command_prompt() {
    let md = "`$ echo hi`";
    let html = processor().render(md).html;
    assert_html_contains(
        &html,
        &[r#"<code class="terminal"><span class="prompt">$</span> echo hi</code>"#],
    );
}

#[test]
fn test_repl_prompt() {
    let md = "`nix-repl> 1 + 1`";
    let html = processor().render(md).html;
    assert_html_contains(
        &html,
        &[r#"<code class="nix-repl"><span class="prompt">nix-repl&gt;</span> 1 + 1</code>"#],
    );
}

#[test]
fn test_inline_anchor() {
    let md = "Go here []{#target}.";
    let html = processor().render(md).html;
    assert_html_contains(
        &html,
        &[r#"<span id="target" class="nixos-anchor"></span>"#],
    );
}

#[test]
fn test_list_item_with_anchor() {
    let md = "- []{#item1} Item 1";
    let html = processor().render(md).html;
    assert_html_contains(
        &html,
        &[r#"<span id="item1" class="nixos-anchor"></span> Item 1"#],
    );
}

#[test]
fn test_explicit_header_anchor() {
    let md = "## Section {#sec}";
    let html = processor().render(md).html;
    assert!(
        html.contains(r#"<h2 id="sec">"#) && html.contains("Section</h2>"),
        "Expected HTML to contain <h2 id=\"sec\">...Section</h2>, got:\n{}",
        html
    );
}

#[test]
fn test_figure_block() {
    let md = "::: {.figure #fig1}\n# Figure Title\nFigure content\n:::";
    let html = processor().render(md).html;
    // Accept admonition-style figure rendering
    assert!(
        html.contains(r#"<div class="admonition figure" id="fig1">"#)
            && html.contains(r#"<p class="admonition-title">Figure</p>"#)
            && html.contains("Figure Title")
            && html.contains("Figure content"),
        "Expected HTML to contain admonition-style figure, got:\n{}",
        html
    );
}

#[test]
fn test_definition_list() {
    let md = "Term\n:   Definition";
    let html = processor().render(md).html;
    assert_html_contains(
        &html,
        &["<dl>", "<dt>Term</dt>", "<dd>Definition</dd>", "</dl>"],
    );
}

#[test]
fn test_option_reference() {
    let md = "`foo.bar.baz`";
    let html = processor().render(md).html;
    // Option references may be rendered as <code> or as a link depending on context
    assert!(
        html.contains(r#"<code>foo.bar.baz</code>"#) || html.contains(r#"option-foo-bar-baz"#),
        "Expected option reference in HTML, got:\n{}",
        html
    );
}

#[test]
fn test_myst_role_markup() {
    let md = r#"<span class="command-markup">foo</span>"#;
    let html = processor().render(md).html;
    assert_html_contains(&html, &[r#"<code class="command">foo</code>"#]);
}

#[test]
fn test_autolink() {
    let md = "Visit https://example.com for info.";
    let html = processor().render(md).html;
    assert_html_contains(
        &html,
        &[r#"<a href="https://example.com">https://example.com</a>"#],
    );
}

#[test]
fn test_header_extraction() {
    let md = "# Title\n\n## Section {#sec}\n### Subsection";
    let result = processor().render(md);
    let headers = result.headers;
    let title = result.title;
    assert_eq!(title.as_deref(), Some("Title"));
    assert_eq!(headers[0].text, "Title");
    assert_eq!(headers[0].level, 1);
    assert_eq!(headers[1].id, "sec");
    assert_eq!(headers[2].level, 3);
}

#[test]
fn test_raw_inline_anchor() {
    let md = "[]{#anchor}";
    let html = processor().render(md).html;
    assert_html_contains(
        &html,
        &[r#"<span id="anchor" class="nixos-anchor"></span>"#],
    );
}

#[test]
fn test_block_and_inline_code() {
    let md = "Here is `inline code`.\n\n```\nblock code\n```";
    let html = processor().render(md).html;
    assert_html_contains(
        &html,
        &["<code>inline code</code>", "<pre><code>block code"],
    );
}

#[test]
fn test_tables_footnotes_strikethrough_tasklists() {
    let md = "\
| A | B |\n|---|---|\n| 1 | 2 |\n\n\
Here is a footnote.[^1]\n\n[^1]: Footnote text.\n\n\
~~strikethrough~~\n\n\
- [x] Task done\n- [ ] Task not done";
    let html = processor().render(md).html;
    assert!(
        html.contains("<table>")
            && html.contains("<del>strikethrough</del>")
            && html.contains("Footnote text")
            && html.contains(r#"<input type="checkbox" checked="" disabled="" />"#)
            && html.contains(r#"<input type="checkbox" disabled="" />"#),
        "Expected HTML to contain table, strikethrough, tasklist checkboxes, and footnote text. Got:\n{}",
        html
    );
}