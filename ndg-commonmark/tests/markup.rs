/// Check if HTML output contains all expected substrings or exact fragments.
/// If `exact` is true, requires the fragment to appear exactly as-is (including order).
/// If `exact` is false, checks for substring presence.
fn assert_html_contains(html: &str, expected: &[&str]) {
    for &needle in expected {
        assert!(
            html.contains(needle),
            "Expected HTML to contain '{needle}', but it did not.\nFull HTML:\n{html}"
        );
    }
}

/// Like `assert_html_contains`, but requires the fragment to appear exactly as-is (not just as a substring).
fn assert_html_exact(html: &str, expected: &[&str]) {
    for &fragment in expected {
        assert!(
            html.contains(fragment),
            "Expected HTML to contain exact fragment '{fragment}', but it did not.\nFull HTML:\n{html}"
        );
    }
}

fn ndg_html(md: &str) -> String {
    let processor =
        ndg_commonmark::MarkdownProcessor::new(ndg_commonmark::MarkdownOptions::default());
    processor.render(md).html
}

fn ndg_full_result(md: &str) -> (String, Vec<ndg_commonmark::Header>, Option<String>) {
    let processor =
        ndg_commonmark::MarkdownProcessor::new(ndg_commonmark::MarkdownOptions::default());
    let result = processor.render(md);
    (result.html, result.headers, result.title)
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
        &[r#"Go here <span id="target" class="nixos-anchor"></span>."#],
    );
}

#[test]
fn test_list_item_with_anchor() {
    let md = "- []{#item1} Item 1";
    let html = ndg_html(md);
    assert_html_exact(
        &html,
        &[r#"<span id="item1" class="nixos-anchor"></span> Item 1"#],
    );
}

#[test]
fn test_explicit_header_anchor() {
    let md = "## Section {#sec}";
    let html = ndg_html(md);
    assert!(
        html.contains(r#"<h2 id="sec">"#) && html.contains("Section</h2>"),
        "Expected HTML to contain <h2 id=\"sec\">...Section</h2>, got:\n{html}"
    );
}

// Edge case: header with anchor and trailing whitespace
#[test]
fn test_explicit_header_anchor_trailing_whitespace() {
    let md = "###   Weird Header   {#weird-anchor}   ";
    let html = ndg_html(md);
    assert!(
        html.contains(r#"<h3 id="weird-anchor">"#) && html.contains("Weird Header"),
        "Expected HTML to contain <h3 id=\"weird-anchor\">...Weird Header..., got:\n{html}"
    );
}

// Edge case: header with anchor and special characters
#[test]
fn test_explicit_header_anchor_special_chars() {
    let md = "## Header! With @Special #Chars {#special_123}";
    let html = ndg_html(md);
    assert!(
        html.contains(r#"<h2 id="special_123">"#) && html.contains("Header! With @Special #Chars"),
        "Expected HTML to contain <h2 id=\"special_123\">...Header! With @Special #Chars..., got:\n{html}"
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
            r#"<span id="start-anchor" class="nixos-anchor"></span>This line starts with an anchor."#,
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
        &[r#"This line ends with an anchor.<span id="end-anchor" class="nixos-anchor"></span>"#],
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
        "Expected HTML to contain admonition-style figure, got:\n{html}"
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
        html.contains(r"<code>foo.bar.baz</code>") || html.contains(r"option-foo-bar-baz"),
        "Expected option reference in HTML, got:\n{html}"
    );
}

#[test]
fn test_myst_role_markup() {
    let md = r"{command}`foo`";
    let html = ndg_commonmark::process_role_markup(md, None);
    assert_html_contains(&html, &[r#"<code class="command">foo</code>"#]);
}

#[test]
fn test_manpage_role_with_url() {
    use std::{fs::File, io::Write};

    use tempfile::tempdir;

    let md = r"{manpage}`cat(1)`";
    let dir = tempdir().unwrap();
    let json_path = dir.path().join("manpage-urls.json");
    let mut file = File::create(&json_path).unwrap();
    write!(
        file,
        r#"{{"cat(1)": "https://www.gnu.org/software/coreutils/manual/html_node/cat-invocation.html"}}"#
    )
    .unwrap();

    let mut opts = ndg_commonmark::MarkdownOptions::default();
    opts.manpage_urls_path = Some(json_path.to_str().unwrap().to_string());
    let processor = ndg_commonmark::MarkdownProcessor::new(opts);

    let html = ndg_commonmark::process_role_markup(md, processor.manpage_urls());
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

    let md = r"{manpage}`doesnotexist(1)`";
    let dir = tempdir().unwrap();
    let json_path = dir.path().join("manpage-urls.json");
    let mut file = File::create(&json_path).unwrap();
    write!(
        file,
        r#"{{"cat(1)": "https://www.gnu.org/software/coreutils/manual/html_node/cat-invocation.html"}}"#
    )
    .unwrap();

    let mut opts = ndg_commonmark::MarkdownOptions::default();
    opts.manpage_urls_path = Some(json_path.to_str().unwrap().to_string());
    let processor = ndg_commonmark::MarkdownProcessor::new(opts);

    let html = ndg_commonmark::process_role_markup(md, processor.manpage_urls());
    assert_html_contains(
        &html,
        &[r#"<span class="manpage-reference">doesnotexist(1)</span>"#],
    );
}

#[test]
fn test_role_markup_in_lists() {
    let md = r"- {command}`nixos-rebuild switch`
- {env}`HOME`
- {file}`/etc/nixos/configuration.nix`
- {option}`services.nginx.enable`
- {var}`pkgs`
- {manpage}`nix.conf(5)`";
    let html = ndg_commonmark::process_role_markup(md, None);

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
    let md = r"{file}`/path/with-dashes_and.dots`";
    let html = ndg_commonmark::process_role_markup(md, None);
    assert_html_contains(
        &html,
        &[r#"<code class="file-path">/path/with-dashes_and.dots</code>"#],
    );

    // Test role with spaces
    let md = r"{command}`ls -la | grep test`";
    let html = ndg_commonmark::process_role_markup(md, None);
    assert_html_contains(
        &html,
        &[r#"<code class="command">ls -la | grep test</code>"#],
    );

    // Test unknown role type
    let md = r"{unknown}`content`";
    let html = ndg_commonmark::process_role_markup(md, None);
    assert_html_contains(&html, &[r#"<span class="unknown-markup">content</span>"#]);
}

#[test]
fn test_reported_issue_regression() {
    // This test verifies the exact issue reported by the user
    let md = r"- {command}`nixos-rebuild switch`
- {env}`HOME`
- {file}`/etc/nixos/configuration.nix`
- {option}`services.nginx.enable`
- {var}`pkgs`
- {manpage}`nix.conf(5)`";
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
    let (_html, headers, title) = ndg_full_result(md);
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
        html.contains(r#"<span id="anchor" class="nixos-anchor"></span>"#),
        "Expected HTML to contain raw inline anchor, got:\n{html}"
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
            r#"<input type="checkbox" checked="" disabled="">"#,
            r#"<input type="checkbox" disabled="">"#,
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
        "Expected HTML to contain all footnote texts and footnote references. Got:\n{html}"
    );

    // Test missing footnote definition (should render a link or marker)
    let md_missing = "Reference to missing footnote.[^missing]";
    let html_missing = ndg_html(md_missing);
    assert!(
        html_missing.contains("missing"),
        "Expected HTML to mention missing footnote reference. Got:\n{html_missing}"
    );
}

// "Sütten ağızı yanan yoğurdu üfleyerek yermiş", or, transated:
// "Once bitten, twice shy"
// Test that role markup is NOT processed inside code blocks. I got bitten by this bug.
// Never again.

#[test]
fn test_role_markup_not_processed_in_code_blocks() {
    // Test that role markup is NOT processed inside fenced code blocks
    let md = r"Here is a code block with role markup:

```
{command}`ls -la`                    # Terminal command
{file}`/etc/nixos/configuration.nix` # File path
{option}`services.nginx.enable`      # NixOS option
```

Normal text after.";

    let html = ndg_html(md);

    // Role markup should NOT be processed inside code blocks
    assert!(
        !html.contains(r#"<code class="command">"#),
        "Role markup should NOT be processed inside fenced code blocks. Got:\n{html}"
    );
    assert!(
        !html.contains(r#"<code class="file-path">"#),
        "Role markup should NOT be processed inside fenced code blocks. Got:\n{html}"
    );
    assert!(
        !html.contains(r#"<a class="option-reference""#),
        "Role markup should NOT be processed inside fenced code blocks. Got:\n{html}"
    );

    // The literal text should still be present
    assert!(
        html.contains("{command}`ls -la`") && html.contains("{file}`/etc/nixos/configuration.nix`"),
        "Literal role markup text should be preserved in code blocks. Got:\n{html}"
    );
}

#[test]
fn test_role_markup_not_processed_in_inline_code() {
    // Test that role markup is NOT processed inside inline code
    let md = r"Here is `{command}`inline`` code with role markup.";

    let html = ndg_html(md);

    // Role markup should NOT be processed inside inline code
    assert!(
        !html.contains(r#"<code class="command">"#),
        "Role markup should NOT be processed inside inline code. Got:\n{html}"
    );

    // The literal text should still be present
    assert!(
        html.contains("{command}"),
        "Literal role markup text should be preserved in inline code. Got:\n{html}"
    );
}

#[test]
fn test_admonitions_not_processed_in_code_blocks() {
    // Test that admonitions are NOT processed inside code blocks
    let md = r"```
::: {.note}
This should not be processed as an admonition
:::
```";

    let html = ndg_html(md);

    // Admonitions should NOT be processed inside code blocks
    assert!(
        !html.contains(r#"<div class="admonition">"#),
        "Admonitions should NOT be processed inside code blocks. Got:\n{html}"
    );

    // The literal text should still be present
    assert!(
        html.contains("::: {.note}"),
        "Literal admonition text should be preserved in code blocks. Got:\n{html}"
    );
}

#[test]
fn test_github_callouts_not_processed_in_code_blocks() {
    // Test that GitHub callouts are NOT processed inside code blocks
    let md = r"```
> [!NOTE]
> This should not be processed as a callout
```";

    let html = ndg_html(md);

    // GitHub callouts should NOT be processed inside code blocks
    assert!(
        !html.contains(r#"<div class="admonition">"#),
        "GitHub callouts should NOT be processed inside code blocks. Got:\n{html}"
    );

    // The literal text should still be present (HTML-escaped in code blocks)
    assert!(
        html.contains("&gt; [!NOTE]"),
        "Literal GitHub callout text should be preserved in code blocks. Got:\n{html}"
    );
}

#[test]
fn test_inline_anchors_not_processed_in_code_blocks() {
    // Test that inline anchors are NOT processed inside code blocks
    let md = r"```
    []{#anchor1} Some content
    More []{#anchor2} content
```";

    let html = ndg_html(md);

    // Inline anchors should NOT be processed inside code blocks
    assert!(
        !html.contains(r#"<span class="nixos-anchor""#),
        "Inline anchors should NOT be processed inside code blocks. Got:\n{html}"
    );

    // The literal text should still be present (HTML-escaped in code blocks)
    assert!(
        html.contains("[]{#anchor1}") && html.contains("[]{#anchor2}"),
        "Literal inline anchor text should be preserved in code blocks. Got:\n{html}"
    );
}

#[test]
fn test_comprehensive_code_block_preservation() {
    // Test that ALL types of NDG-specific syntax are NOT processed inside code blocks
    let md = r#"````
{command}`ls -la`                    # Role markup
{file}`/etc/nixos/configuration.nix`
{option}`services.nginx.enable`
{env}`HOME`
{var}`myVariable`
{manpage}`man(1)`
{incomplete-role}                    # Incomplete role markup

::: {.note}                          # Admonitions
This should not be an admonition
:::

> [!WARNING]                         # GitHub callouts
> This should not be a callout

[]{#anchor1} Content                 # Inline anchors
More []{#anchor2} content

`$ echo "command prompt"`            # Command prompts
`nix-repl> 1 + 1`                   # REPL prompts

Term                                 # Definition lists
:   Definition

https://example.com                  # Autolinks
https://nixos.org/downloads

```{=include=}                       # File includes
path/to/file1.md
path/to/file2.md
```
````"#;

    let html = ndg_html(md);

    // Role markup should NOT be processed
    assert!(
        !html.contains(r#"<code class="command">"#)
            && !html.contains(r#"<code class="file-path">"#)
            && !html.contains(r#"<a class="option-reference""#)
            && !html.contains(r#"<code class="env-var">"#)
            && !html.contains(r#"<code class="nix-var">"#)
            && !html.contains(r#"<span class="manpage-reference">"#),
        "Role markup should NOT be processed inside code blocks. Got:\n{html}"
    );

    // Admonitions should NOT be processed
    assert!(
        !html.contains(r#"<div class="admonition">"#),
        "Admonitions should NOT be processed inside code blocks. Got:\n{html}"
    );

    // GitHub callouts should NOT be processed
    assert!(
        !html.contains(r#"<div class="admonition">"#),
        "GitHub callouts should NOT be processed inside code blocks. Got:\n{html}"
    );

    // Inline anchors should NOT be processed
    assert!(
        !html.contains(r#"<span class="nixos-anchor""#),
        "Inline anchors should NOT be processed inside code blocks. Got:\n{html}"
    );

    // Command/REPL prompts should NOT be processed
    assert!(
        !html.contains(r#"<span class="prompt">"#),
        "Command/REPL prompts should NOT be processed inside code blocks. Got:\n{html}"
    );

    // Definition lists should NOT be processed
    assert!(
        !html.contains("<dl>") && !html.contains("<dt>") && !html.contains("<dd>"),
        "Definition lists should NOT be processed inside code blocks. Got:\n{html}"
    );

    // Autolinks should NOT be processed
    assert!(
        !html.contains(r#"<a href="https://example.com""#)
            && !html.contains(r#"<a href="https://nixos.org""#),
        "Autolinks should NOT be processed inside code blocks. Got:\n{html}"
    );

    // File includes should NOT be processed
    assert!(
        !html.contains("<!-- ndg: could not include file:")
            && html.contains("```{=include=}")
            && html.contains("path/to/file1.md"),
        "File includes should NOT be processed inside code blocks. Got:\n{html}"
    );

    // All literal text should be preserved
    assert!(
        html.contains("{command}`ls -la`")
            && html.contains("{incomplete-role}")
            && html.contains("::: {.note}")
            && html.contains("&gt; [!WARNING]")
            && html.contains("[]{#anchor1}")
            && html.contains("`$ echo \"command prompt\"`")
            && html.contains("Term")
            && html.contains(":   Definition")
            && html.contains("https://example.com")
            && html.contains("https://nixos.org"),
        "Literal text should be preserved in code blocks. Got:\n{html}"
    );
}

#[test]
fn test_command_prompts_not_processed_in_code_blocks() {
    // Test that command and REPL prompts are NOT processed inside code blocks
    let md = r#"```
`$ echo "this should not be processed"`
`nix-repl> 1 + 1`
```"#;

    let html = ndg_html(md);

    // Command/REPL prompts should NOT be processed inside code blocks
    assert!(
        !html.contains(r#"<span class="prompt">"#),
        "Command/REPL prompts should NOT be processed inside code blocks. Got:\n{html}"
    );

    // The literal text should still be present (HTML-escaped in code blocks)
    assert!(
        html.contains("`$ echo \"this should not be processed\"`")
            && html.contains("`nix-repl&gt; 1 + 1`"),
        "Literal prompt text should be preserved in code blocks. Got:\n{html}"
    );
}

#[test]
fn test_incomplete_role_markup_bug() {
    // Test incomplete role markup like {var} without content
    let md = r"Here is incomplete role markup: {var} and complete: {var}`content`";
    let html = ndg_html(md);

    // Both should be processed correctly - incomplete should be left as-is
    assert!(
        html.contains("{var}") && html.contains(r#"<code class="nix-var">content</code>"#),
        "Incomplete role markup should be preserved, complete should be processed. Got:\n{html}"
    );
}

#[test]
fn test_incomplete_role_markup_with_empty_content() {
    // Test that incomplete role markup with empty content is preserved as literal text
    let md = r"Empty option: {option}``";
    let html = ndg_html(md);

    // Should preserve the entire incomplete markup as literal text
    assert!(
        html.contains("{option}``"),
        "Incomplete role markup with empty content should be preserved as literal text. Got:\n{html}"
    );

    // Should not create empty code elements
    assert!(
        !html.contains("<code></code>"),
        "Empty option with double backticks should not generate empty code tags. Got:\n{html}"
    );

    // Test standalone incomplete roles
    let test_cases = vec!["{var}", "{command}", "{file}", "{unknown}"];

    for case in test_cases {
        let html = ndg_html(case);
        assert!(
            !html.contains("<code>") && !html.contains('`'),
            "Incomplete role markup {case} should not generate code tags or backticks. Got:\n{html}"
        );
        assert!(
            html.contains(case),
            "Should preserve literal {case} text. Got:\n{html}"
        );
    }
}

#[test]
fn test_markdown_parsing_inside_admonitions() {
    // Test that Markdown features are correctly parsed inside admonitions
    let md = r"::: {.note}
This is **bold** text and *italic* text.

Here is `inline code` and {var}`myVariable`.

- List item 1
- List item 2

## Header inside admonition

[Link text](https://example.com)
:::";

    let html = ndg_html(md);

    // Debug output can be enabled if needed
    // println!("DEBUG: Admonition HTML output:");
    // println!("{}", html);

    // Should contain properly parsed Markdown elements
    assert!(
        html.contains("<strong>bold</strong>") && html.contains("<em>italic</em>"),
        "Bold and italic text should be parsed inside admonitions. Got:\n{html}"
    );

    assert!(
        html.contains(r"<code>inline code</code>"),
        "Inline code should be parsed inside admonitions. Got:\n{html}"
    );

    assert!(
        html.contains(r#"<code class="nix-var">myVariable</code>"#),
        "Role markup should be parsed inside admonitions. Got:\n{html}"
    );

    assert!(
        html.contains("<ul>") && html.contains("<li>List item 1</li>"),
        "Lists should be parsed inside admonitions. Got:\n{html}"
    );

    assert!(
        html.contains("<h2>Header inside admonition</h2>"),
        "Headers should be parsed inside admonitions. Got:\n{html}"
    );

    assert!(
        html.contains(r#"<a href="https://example.com">Link text</a>"#),
        "Links should be parsed inside admonitions. Got:\n{html}"
    );
}

#[test]
fn test_markdown_parsing_inside_github_callouts() {
    // Test that Markdown features are correctly parsed inside GitHub callouts
    let md = r"> [!NOTE]
> This is **bold** text and *italic* text.
>
> Here is `inline code` and {var}`myVariable`.
>
> - List item 1
> - List item 2";

    let html = ndg_html(md);

    // Should contain properly parsed Markdown elements
    assert!(
        html.contains("<strong>bold</strong>") && html.contains("<em>italic</em>"),
        "Bold and italic text should be parsed inside GitHub callouts. Got:\n{html}"
    );

    assert!(
        html.contains(r"<code>inline code</code>"),
        "Inline code should be parsed inside GitHub callouts. Got:\n{html}"
    );

    assert!(
        html.contains(r#"<code class="nix-var">myVariable</code>"#),
        "Role markup should be parsed inside GitHub callouts. Got:\n{html}"
    );

    assert!(
        html.contains("<ul>") && html.contains("<li>List item 1</li>"),
        "Lists should be parsed inside GitHub callouts. Got:\n{html}"
    );
}

#[test]
fn test_markdown_parsing_inside_figures() {
    // Test that Markdown features are correctly parsed inside figures
    let md = r"::: {.figure #sample-figure}

# Figure Caption with **bold** text

This is *italic* text and `inline code`.

Here is {var}`myVariable` role markup.

![Alt text](image.png)
:::";

    let html = ndg_html(md);

    // Should contain properly parsed Markdown elements
    assert!(
        html.contains("<strong>bold</strong>") && html.contains("<em>italic</em>"),
        "Bold and italic text should be parsed inside figures. Got:\n{html}"
    );

    assert!(
        html.contains(r"<code>inline code</code>"),
        "Inline code should be parsed inside figures. Got:\n{html}"
    );

    assert!(
        html.contains(r#"<code class="nix-var">myVariable</code>"#),
        "Role markup should be parsed inside figures. Got:\n{html}"
    );

    assert!(
        html.contains(r#"<img src="image.png" alt="Alt text""#),
        "Images should be parsed inside figures. Got:\n{html}"
    );
}

#[test]
fn test_public_extension_api() {
    // Test that the public extension functions work correctly for third-party use

    // Test GFM extensions (currently a placeholder)
    #[cfg(feature = "gfm")]
    {
        let md = "# Test\n\nSome **bold** text.";
        let result = ndg_commonmark::apply_gfm_extensions(md);
        // Currently a no-op, should return unchanged
        assert_eq!(result, md);
    }

    // Test Nixpkgs extensions with file includes
    #[cfg(feature = "nixpkgs")]
    {
        use std::fs;

        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        let file1_path = dir.path().join("test1.md");
        let file2_path = dir.path().join("test2.md");

        // Create test files
        fs::write(&file1_path, "# Included File 1\nContent from file 1.").unwrap();
        fs::write(&file2_path, "## Included File 2\nContent from file 2.").unwrap();

        // Test file inclusion
        let md = format!(
            r"# Main Document

```{{=include=}}
{}
{}
```

End of document.",
            file1_path.file_name().unwrap().to_str().unwrap(),
            file2_path.file_name().unwrap().to_str().unwrap()
        );

        let result = ndg_commonmark::process_file_includes(&md, dir.path());

        // Should include both files
        assert!(result.contains("# Included File 1"));
        assert!(result.contains("Content from file 1."));
        assert!(result.contains("## Included File 2"));
        assert!(result.contains("Content from file 2."));
        assert!(result.contains("End of document."));
        assert!(!result.contains("```{=include=}"));
    }

    // Test that file includes respect code block boundaries
    #[cfg(feature = "nixpkgs")]
    {
        let md = r"````
```{=include=}
some/file.md
```
````";

        let result = ndg_commonmark::process_file_includes(md, std::path::Path::new("."));

        // Should NOT process includes inside code blocks
        assert!(result.contains("```{=include=}"));
        assert!(result.contains("some/file.md"));
        assert!(!result.contains("<!-- ndg: could not include file:"));
    }

    // Test with the main processor to verify integration
    #[cfg(feature = "nixpkgs")]
    {
        let mut options = ndg_commonmark::MarkdownOptions::default();
        options.nixpkgs = true;
        let processor = ndg_commonmark::MarkdownProcessor::new(options);

        let simple_md = r"```{=include=}
test1.md
```";
        let result = processor.render(simple_md);

        // Should show include processing (file not found)
        assert!(result.html.contains("<!-- ndg: could not include file:"));
    }
}

#[test]
fn test_file_includes_not_processed_in_code_blocks() {
    // Test that file includes are NOT processed inside code blocks
    let md = r"````
```{=include=}
path/to/file1.md
path/to/file2.md
```
````";

    let html = ndg_html(md);

    // File includes should NOT be processed inside code blocks
    // Content should be preserved as plain text without syntax highlighting
    assert!(
        html.contains("```{=include=}")
            && html.contains("path/to/file1.md")
            && html.contains("<pre><code>"),
        "File include syntax should be preserved in code blocks as plain text. Got:\n{html}"
    );
}

#[test]
fn test_simple_nested_file_includes() {
    // Test simple case with file includes inside code blocks
    let md = r"````
```{=include=}
path/to/file1.md
```
````";

    let html = ndg_html(md);

    // File includes should NOT be processed inside code blocks
    assert!(
        !html.contains("<!-- ndg: could not include file:")
            && html.contains("```{=include=}")
            && html.contains("path/to/file1.md"),
        "File include syntax should be preserved in nested code blocks. Got:\n{html}"
    );
}

#[test]
fn test_autolinks_not_processed_in_code_blocks() {
    // Test that autolinks are NOT processed inside code blocks
    let md = r"```markdown
Visit https://nixos.org for more information.
Also check https://example.com/test
```";

    let html = ndg_html(md);

    // Autolinks should NOT be processed inside code blocks
    assert!(
        !html.contains(r#"<a href="https://nixos.org""#)
            && !html.contains(r#"<a href="https://example.com""#),
        "Autolinks should NOT be processed inside code blocks. Got:\n{html}"
    );

    // The literal URLs should still be present
    assert!(
        html.contains("https://nixos.org") && html.contains("https://example.com/test"),
        "Literal URLs should be preserved in code blocks. Got:\n{html}"
    );
}
