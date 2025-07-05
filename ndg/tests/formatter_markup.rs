use ndg::{config::Config, formatter::markup};
use ndg_commonmark::legacy_markdown;

#[test]
fn parses_basic_markdown_ast() {
    let config = Config::default();
    let md = "# Heading 1\n\nSome *italic* and **bold** text.";
    let (html, ..) =
        legacy_markdown::process_markdown(md, None, Some(&config.title), std::path::Path::new("."));
    assert!(html.contains("<h1") && html.contains("Heading 1"));
    assert!(html.contains("<em>italic</em>"));
    assert!(html.contains("<strong>bold</strong>"));
}

#[test]
fn parses_list_with_inline_anchor() {
    let config = Config::default();
    let md = "- []{#item1} Item 1";
    let (html, ..) =
        legacy_markdown::process_markdown(md, None, Some(&config.title), std::path::Path::new("."));

    assert!(html.contains(r#"<span class="nixos-anchor" id="item1"></span> Item 1"#));
}

#[test]
fn markup_role_pattern_matches() {
    let s = "{command}`ls -l`";
    let caps = markup::ROLE_PATTERN
        .captures(s)
        .expect("Should match role pattern");
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
    let caps = legacy_markdown::HEADING_ANCHOR
        .captures(s)
        .expect("Should match heading anchor");
    assert_eq!(&caps[2], "Section");
    assert_eq!(&caps[3], "sec");
}

#[test]
fn markdown_list_item_with_anchor_regex() {
    let s = "- []{#foo} Bar";
    let caps = legacy_markdown::LIST_ITEM_WITH_ANCHOR_RE
        .captures(s)
        .expect("Should match list item anchor");
    assert_eq!(&caps[2], "foo");
    assert_eq!(caps[3].trim(), "Bar");
}

#[test]
fn markdown_process_markdown_string_handles_links() {
    let config = Config::default();
    let (html, ..) = legacy_markdown::process_markdown(
        "[link](https://example.com)",
        None,
        Some(&config.title),
        std::path::Path::new("."),
    );
    assert!(html.contains("<a href=\"https://example.com\""));
}
