use pulldown_cmark::{Event, Options, Parser, Tag};

use super::{markdown, markup};
use crate::config::Config;

#[test]
fn parses_basic_markdown_ast() {
    let md = "# Heading 1\n\nSome *italic* and **bold** text.";
    let parser = Parser::new_ext(md, Options::all());
    let mut found_heading = false;
    let mut found_em = false;
    let mut found_strong = false;
    for event in parser {
        match event {
            Event::Start(Tag::Heading { .. }) => found_heading = true,
            Event::Start(Tag::Emphasis) => found_em = true,
            Event::Start(Tag::Strong) => found_strong = true,
            _ => {}
        }
    }
    assert!(found_heading);
    assert!(found_em);
    assert!(found_strong);
}

#[test]
fn parses_list_with_inline_anchor() {
    let md = "- []{#item1} Item 1";
    let parser = Parser::new_ext(md, Options::all());
    let mut found_item = false;
    for event in parser {
        if let Event::Text(text) = &event {
            if text.contains("Item 1") {
                found_item = true;
            }
        }
    }
    assert!(found_item);
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
