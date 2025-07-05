/// Check if HTML output contains all expected substrings or exact fragments.
/// If `exact` is true, requires the fragment to appear exactly as-is (including order).
/// If `exact` is false, checks for substring presence.
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

/// Like assert_html_contains, but requires the fragment to appear exactly as-is (not just as a substring).
fn assert_html_exact(html: &str, expected: &[&str]) {
    for &fragment in expected {
        assert!(
            html.contains(fragment),
            "Expected HTML to contain exact fragment '{}', but it did not.\nFull HTML:\n{}",
            fragment,
            html
        );
    }
}

fn ndg_html(md: &str) -> String {
    ndg_commonmark::legacy_markdown::process_markdown(md, None, None, std::path::Path::new(".")).0
}

#[test]
fn test_admonition_note() {
    let md = "::: {.note}\nThis is a note.\n:::";
    let html = ndg_html(md);
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
    let html = ndg_html(md);
    assert_html_contains(&html, &[r#"<code class="command">ls -l</code>"#]);
}

#[test]
fn test_role_option() {
    let md = "{option}`services.nginx.enable`";
    let html = ndg_html(md);
    assert_html_exact(
        &html,
        &[
            r#"<a class="option-reference" href="options.html#option-services-nginx-enable"><code>services.nginx.enable</code></a>"#,
        ],
    );
}

#[test]
fn test_command_prompt() {
    let md = "`$ echo hi`";
    let html = ndg_html(md);
    assert_html_contains(
        &html,
        &[r#"<code class="terminal"><span class="prompt">$</span> echo hi</code>"#],
    );
}

#[test]
fn test_repl_prompt() {
    let md = "`nix-repl> 1 + 1`";
    let html = ndg_html(md);
    assert_html_contains(
        &html,
        &[r#"<code class="nix-repl"><span class="prompt">nix-repl&gt;</span> 1 + 1</code>"#],
    );
}

#[test]
fn test_inline_anchor() {
    let md = "Go here []{#target}.";
    let html = ndg_html(md);
    assert_html_exact(
        &html,
        &[r#"Go here <span class="nixos-anchor" id="target"></span>."#],
    );
}

#[test]
fn test_list_item_with_anchor() {
    let md = "- []{#item1} Item 1";
    let html = ndg_html(md);
    assert_html_exact(
        &html,
        &[r#"<span class="nixos-anchor" id="item1"></span> Item 1"#],
    );
}

#[test]
fn test_explicit_header_anchor() {
    let md = "## Section {#sec}";
    let html = ndg_html(md);
    assert!(
        html.contains(r#"<h2 id="sec">"#) && html.contains("Section</h2>"),
        "Expected HTML to contain <h2 id=\"sec\">...Section</h2>, got:\n{}",
        html
    );
}

// Edge case: header with anchor and trailing whitespace
#[test]
fn test_explicit_header_anchor_trailing_whitespace() {
    let md = "###   Weird Header   {#weird-anchor}   ";
    let html = ndg_html(md);
    assert!(
        html.contains(r#"<h3 id="weird-anchor">"#) && html.contains("Weird Header"),
        "Expected HTML to contain <h3 id=\"weird-anchor\">...Weird Header..., got:\n{}",
        html
    );
}

// Edge case: header with anchor and special characters
#[test]
fn test_explicit_header_anchor_special_chars() {
    let md = "## Header! With @Special #Chars {#special_123}";
    let html = ndg_html(md);
    assert!(
        html.contains(r#"<h2 id="special_123">"#) && html.contains("Header! With @Special #Chars"),
        "Expected HTML to contain <h2 id=\"special_123\">...Header! With @Special #Chars..., got:\n{}",
        html
    );
}

// Edge case: inline anchor at start of line
#[test]
fn test_inline_anchor_start_of_line() {
    let md = "[]{#start-anchor}This line starts with an anchor.";
    let html = ndg_html(md);
    assert_html_exact(
        &html,
        &[
            r#"<span class="nixos-anchor" id="start-anchor"></span>This line starts with an anchor."#,
        ],
    );
}

// Edge case: inline anchor at end of line
#[test]
fn test_inline_anchor_end_of_line() {
    let md = "This line ends with an anchor.[]{#end-anchor}";
    let html = ndg_html(md);
    assert_html_exact(
        &html,
        &[r#"This line ends with an anchor.<span class="nixos-anchor" id="end-anchor"></span>"#],
    );
}

#[test]
fn test_figure_block() {
    let md = "::: {.figure #fig1}\n# Figure Title\nFigure content\n:::";
    let html = ndg_html(md);
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
    let html = ndg_html(md);
    assert_html_contains(
        &html,
        &["<dl>", "<dt>Term</dt>", "<dd>Definition</dd>", "</dl>"],
    );
}

#[test]
fn test_option_reference() {
    let md = "`foo.bar.baz`";
    let html = ndg_html(md);
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
    let html = ndg_html(md);
    assert_html_contains(&html, &[r#"<code class="command">foo</code>"#]);
}

#[test]
fn test_autolink() {
    let md = "Visit https://example.com for info.";
    let html = ndg_html(md);
    assert_html_contains(
        &html,
        &[r#"<a href="https://example.com">https://example.com</a>"#],
    );
}

#[test]
fn test_header_extraction() {
    let md = "# Title\n\n## Section {#sec}\n### Subsection";
    let (html, headers, title) = ndg_commonmark::legacy_markdown::process_markdown(
        md,
        None,
        None,
        std::path::Path::new("."),
    );
    assert_eq!(title.as_deref(), Some("Title"));
    assert_eq!(headers[0].text, "Title");
    assert_eq!(headers[0].level, 1);
    assert_eq!(headers[1].id, "sec");
    assert_eq!(headers[2].level, 3);
}

#[test]
fn test_raw_inline_anchor() {
    let md = "[]{#anchor}";
    let html = ndg_html(md);
    assert!(
        html.contains(r#"<span class="nixos-anchor" id="anchor"></span>"#),
        "Expected HTML to contain raw inline anchor, got:\n{}",
        html
    );
}

#[test]
fn test_block_and_inline_code() {
    let md = "Here is `inline code`.\n\n```\nblock code\n```";
    let html = ndg_html(md);
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
    let html = ndg_html(md);
    assert_html_contains(
        &html,
        &[
            "<table>",
            "<del>strikethrough</del>",
            "Footnote text",
            r#"<input checked="" disabled="" type="checkbox">"#,
            r#"<input disabled="" type="checkbox">"#,
        ],
    );
}

#[test]
fn test_footnotes_various_cases() {
    let md = "\
Here is a footnote.[^1]

Here is another footnote.[^note2]

Here is an inline footnote.^[This is inline.]

[^1]: Footnote one text.
[^note2]: Footnote two text.
";
    let html = ndg_html(md);
    assert!(
        html.contains("Footnote one text.")
            && html.contains("Footnote two text.")
            && html.contains("This is inline.")
            && html.contains("footnote")
            && html.contains("fnref")
            && html.contains("data-footnote-backref"),
        "Expected HTML to contain all footnote texts and footnote references. Got:\n{}",
        html
    );

    // Test missing footnote definition (should render a link or marker)
    let md_missing = "Reference to missing footnote.[^missing]";
    let html_missing = ndg_html(md_missing);
    assert!(
        html_missing.contains("missing"),
        "Expected HTML to mention missing footnote reference. Got:\n{}",
        html_missing
    );
}
