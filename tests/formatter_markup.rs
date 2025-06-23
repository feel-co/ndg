use ndg::{
    config::Config,
    formatter::{markdown, markup},
};

#[test]
fn parses_basic_markdown_ast() {
    let config = Config::default();
    let md = "# Heading 1\n\nSome *italic* and **bold** text.";
    let html = markdown::process_markdown_string(md, &config);
    assert!(html.contains("<h1") && html.contains("Heading 1"));
    assert!(html.contains("<em>italic</em>"));
    assert!(html.contains("<strong>bold</strong>"));
}

#[test]
fn parses_list_with_inline_anchor() {
    let config = Config::default();
    let md = "- []{#item1} Item 1";
    let html = markdown::process_markdown_string(md, &config);
    assert!(html.contains(r#"<span id="item1" class="nixos-anchor"></span> Item 1"#));
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
    let caps = markdown::HEADING_ANCHOR
        .captures(s)
        .expect("Should match heading anchor");
    assert_eq!(&caps[2], "Section");
    assert_eq!(&caps[3], "sec");
}

#[test]
fn markdown_list_item_with_anchor_regex() {
    let s = "- []{#foo} Bar";
    let caps = markdown::LIST_ITEM_WITH_ANCHOR_RE
        .captures(s)
        .expect("Should match list item anchor");
    assert_eq!(&caps[2], "foo");
    assert_eq!(caps[3].trim(), "Bar");
}

#[test]
fn markdown_process_markdown_string_handles_links() {
    let config = Config::default();
    let html = markdown::process_markdown_string("[link](https://example.com)", &config);
    assert!(html.contains("<a href=\"https://example.com\""));
}
