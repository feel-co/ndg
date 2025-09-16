use ndg_commonmark::{MarkdownOptions, MarkdownProcessor, processor};

#[test]
fn parses_basic_markdown_ast() {
    let md = "# Heading 1\n\nSome *italic* and **bold** text.";
    let processor = MarkdownProcessor::new(MarkdownOptions::default());
    let result = processor.render(md);
    let html = result.html;
    assert!(html.contains("<h1") && html.contains("Heading 1"));
    assert!(html.contains("<em>italic</em>"));
    assert!(html.contains("<strong>bold</strong>"));
}

#[test]
fn parses_list_with_inline_anchor() {
    let md = "- []{#item1} Item 1";
    let processor = MarkdownProcessor::new(MarkdownOptions::default());
    let result = processor.render(md);
    let html = result.html;

    let expected = r#"<span id="item1" class="nixos-anchor"></span> Item 1"#;
    assert!(
        html.contains(expected),
        "HTML did not contain expected span with anchor: {html}"
    );
}

#[test]
fn markup_role_pattern_matches() {
    let s = "{command}`ls -l`";
    let role_re = regex::Regex::new(r"\{([a-z]+)\}`([^`]+)`").unwrap();
    let caps = role_re.captures(s).expect("Should match role pattern");
    assert_eq!(&caps[1], "command");
    assert_eq!(&caps[2], "ls -l");
}

#[test]
fn markdown_processor_handles_command_prompts() {
    let md = "`$ echo hi`";
    let processor = MarkdownProcessor::new(MarkdownOptions::default());
    let result = processor.render(md);

    // The processor should handle command prompts as code blocks
    assert!(result.html.contains("echo hi"));
}

#[test]
fn markdown_processor_handles_inline_code() {
    let md = "`inline code`";
    let processor = MarkdownProcessor::new(MarkdownOptions::default());
    let result = processor.render(md);
    assert!(result.html.contains("<code>inline code</code>"));
}

#[test]
fn safely_process_markup_handles_panic() {
    let result = processor::process_safe("foo", |_| panic!("fail"), "fallback");
    assert_eq!(result, "fallback");
}

#[test]
fn markdown_heading_anchor_regex() {
    let s = "## Section {#sec}";
    let processor = MarkdownProcessor::new(MarkdownOptions::default());
    let result = processor.render(s);
    assert!(result.html.contains("id=\"sec\""));
    assert!(result.html.contains("Section"));
}

#[test]
fn markdown_list_item_with_anchor_regex() {
    let s = "- []{#foo} Bar";
    let processor = MarkdownProcessor::new(MarkdownOptions::default());
    let result = processor.render(s);
    assert!(result.html.contains("id=\"foo\""));
    assert!(result.html.contains("Bar"));
}

#[test]
fn markdown_process_markdown_string_handles_links() {
    let processor = MarkdownProcessor::new(MarkdownOptions::default());
    let result = processor.render("[link](https://example.com)");
    assert!(result.html.contains("<a href=\"https://example.com\""));
}
