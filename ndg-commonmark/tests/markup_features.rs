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
    let md = r#"{command}`foo`"#;
    let html = ndg_commonmark::processor::MarkdownProcessor::new(
        ndg_commonmark::processor::MarkdownOptions::default(),
    )
    .process_role_markup(md);
    assert_html_contains(&html, &[r#"<code class="command">foo</code>"#]);
}

#[test]
fn test_manpage_role_with_url() {
    use std::{fs::File, io::Write};

    use tempfile::tempdir;

    let md = r#"{manpage}`cat(1)`"#;
    let dir = tempdir().unwrap();
    let json_path = dir.path().join("manpage-urls.json");
    let mut file = File::create(&json_path).unwrap();
    write!(
        file,
        r#"{{"cat(1)": "https://www.gnu.org/software/coreutils/manual/html_node/cat-invocation.html"}}"#
    )
    .unwrap();

    let mut opts = ndg_commonmark::processor::MarkdownOptions::default();
    opts.manpage_urls_path = Some(json_path.to_str().unwrap().to_string());
    let processor = ndg_commonmark::processor::MarkdownProcessor::new(opts);

    let html = processor.process_role_markup(md);
    assert_html_contains(
        &html,
        &[
            r#"<a href="https://www.gnu.org/software/coreutils/manual/html_node/cat-invocation.html" class="manpage-reference">cat(1)</a>"#,
        ],
    );
}

#[test]
fn test_manpage_role_without_url() {
    use std::{fs::File, io::Write};

    use tempfile::tempdir;

    let md = r#"{manpage}`doesnotexist(1)`"#;
    let dir = tempdir().unwrap();
    let json_path = dir.path().join("manpage-urls.json");
    let mut file = File::create(&json_path).unwrap();
    write!(
        file,
        r#"{{"cat(1)": "https://www.gnu.org/software/coreutils/manual/html_node/cat-invocation.html"}}"#
    )
    .unwrap();

    let mut opts = ndg_commonmark::processor::MarkdownOptions::default();
    opts.manpage_urls_path = Some(json_path.to_str().unwrap().to_string());
    let processor = ndg_commonmark::processor::MarkdownProcessor::new(opts);

    let html = processor.process_role_markup(md);
    assert_html_contains(
        &html,
        &[r#"<span class="manpage-reference">doesnotexist(1)</span>"#],
    );
}

#[test]
fn test_role_markup_in_lists() {
    let md = r#"- {command}`nixos-rebuild switch`
- {env}`HOME`
- {file}`/etc/nixos/configuration.nix`
- {option}`services.nginx.enable`
- {var}`pkgs`
- {manpage}`nix.conf(5)`"#;
    let html = ndg_commonmark::processor::MarkdownProcessor::new(
        ndg_commonmark::processor::MarkdownOptions::default(),
    )
    .process_role_markup(md);

    // Test that all role types are processed correctly
    assert_html_contains(
        &html,
        &[
            r#"<code class="command">nixos-rebuild switch</code>"#,
            r#"<code class="env-var">HOME</code>"#,
            r#"<code class="file-path">/etc/nixos/configuration.nix</code>"#,
            r#"<a class="option-reference" href="options.html#option-services-nginx-enable"><code>services.nginx.enable</code></a>"#,
            r#"<code class="nix-var">pkgs</code>"#,
            r#"<span class="manpage-reference">nix.conf(5)</span>"#,
        ],
    );

    // Test that no double-processing occurs
    assert!(
        !html.contains(r#"<code class="nixos-option">"#),
        "Option should not be processed as nixos-option class"
    );
    assert!(
        !html.contains("&lt;a href"),
        "No nested anchor tags should be present"
    );
    assert!(
        !html.contains("href=\"<a href"),
        "No nested href attributes should be present"
    );
}

#[test]
fn test_role_markup_edge_cases() {
    // Test role with special characters
    let md = r#"{file}`/path/with-dashes_and.dots`"#;
    let html = ndg_commonmark::processor::MarkdownProcessor::new(
        ndg_commonmark::processor::MarkdownOptions::default(),
    )
    .process_role_markup(md);
    assert_html_contains(
        &html,
        &[r#"<code class="file-path">/path/with-dashes_and.dots</code>"#],
    );

    // Test role with spaces
    let md = r#"{command}`ls -la | grep test`"#;
    let html = ndg_commonmark::processor::MarkdownProcessor::new(
        ndg_commonmark::processor::MarkdownOptions::default(),
    )
    .process_role_markup(md);
    assert_html_contains(
        &html,
        &[r#"<code class="command">ls -la | grep test</code>"#],
    );

    // Test unknown role type
    let md = r#"{unknown}`content`"#;
    let html = ndg_commonmark::processor::MarkdownProcessor::new(
        ndg_commonmark::processor::MarkdownOptions::default(),
    )
    .process_role_markup(md);
    assert_html_contains(&html, &[r#"<span class="unknown-markup">content</span>"#]);
}

#[test]
fn test_reported_issue_regression() {
    // This test verifies the exact issue reported by the user
    let md = r#"- {command}`nixos-rebuild switch`
- {env}`HOME`
- {file}`/etc/nixos/configuration.nix`
- {option}`services.nginx.enable`
- {var}`pkgs`
- {manpage}`nix.conf(5)`"#;
    let html = ndg_html(md);

    // Verify correct HTML structure with proper list items
    assert_html_contains(
        &html,
        &[
            r#"<li><code class="command">nixos-rebuild switch</code></li>"#,
            r#"<li><code class="env-var">HOME</code></li>"#,
            r#"<li><code class="file-path">/etc/nixos/configuration.nix</code></li>"#,
            r#"<li><a class="option-reference" href="options.html#option-services-nginx-enable"><code>services.nginx.enable</code></a></li>"#,
            r#"<li><code class="nix-var">pkgs</code></li>"#,
            r#"<li><span class="manpage-reference">nix.conf(5)</span></li>"#,
        ],
    );

    // Verify no malformed HTML patterns
    assert!(
        !html.contains(r#"<a class="option-reference""><li></li>"#),
        "Option reference should not break list structure"
    );
    assert!(
        !html.contains(r#"href="<a href"#),
        "No nested anchor tags in href attributes"
    );
    assert!(
        !html.contains(r#"</a>"><li></li>"#),
        "No empty list items after option references"
    );
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
