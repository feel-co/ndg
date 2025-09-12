use ndg::{config::Config, formatter::markup};
use ndg_commonmark::{MarkdownOptions, MarkdownProcessor};

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

    assert!(html.contains(r#"<span class="nixos-anchor" id="item1"></span> Item 1"#));
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
fn markup_command_prompt_matches() {
    let s = "`$ echo hi`";
    let caps = markup::COMMAND_PROMPT
        .captures(s)
        .expect("Should match command prompt");
    assert_eq!(&caps[1], "echo hi");
}

#[test]
fn markup_inline_code_matches() {
    let s = "`inline code`";
    let caps = markup::INLINE_CODE
        .captures(s)
        .expect("Should match inline code");
    assert_eq!(&caps[1], "inline code");
}

#[test]
fn safely_process_markup_handles_panic() {
    let result = markup::safely_process_markup("foo", |_| panic!("fail"), "fallback");
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
