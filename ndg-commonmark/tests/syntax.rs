#![allow(clippy::expect_used, reason = "Fine in tests")]

use std::fs;

use ndg_commonmark::{
  MarkdownOptions,
  MarkdownProcessor,
  create_default_manager,
};

#[test]
fn test_syntax_highlighting_pipeline() {
  let processor = MarkdownProcessor::new(MarkdownOptions {
    highlight_code: true,
    ..Default::default()
  });

  let markdown = r#"
```rust
fn main() {
    println!("Hello, world!");
}
```

Inline `fn main()` should stay plain.

```nonexistent-language
some code here
```
"#;

  let html = processor.render(markdown).html;

  assert!(html.contains("println"));
  assert!(html.contains("color:rgb"));
  assert!(html.contains("<code>fn main()</code>"));
  assert!(html.contains("some code here"));
}

#[test]
fn test_syntax_highlighting_disabled() {
  let processor = MarkdownProcessor::new(MarkdownOptions {
    highlight_code: false,
    ..Default::default()
  });

  let html = processor
    .render("```rust\nfn main() { println!(\"Hello\"); }\n```")
    .html;

  assert!(html.contains("fn main"));
  assert!(!html.contains("color:rgb"));
}

#[cfg(feature = "syntastica")]
#[test]
fn test_syntastica_extends_appends_to_builtin_queries() {
  let temp_replace = tempfile::tempdir().expect("tempdir");
  let temp_extend = tempfile::tempdir().expect("tempdir");

  let nix_replace = temp_replace.path().join("nix");
  let nix_extend = temp_extend.path().join("nix");
  fs::create_dir_all(&nix_replace).expect("create replacement query dir");
  fs::create_dir_all(&nix_extend).expect("create extends query dir");

  let replacement_query = r#"(identifier) @function"#;
  let extending_query = format!(";; extends\n{replacement_query}");

  fs::write(nix_replace.join("highlights.scm"), replacement_query)
    .expect("write replacement query");
  fs::write(nix_extend.join("highlights.scm"), &extending_query)
    .expect("write extends query");

  let nix_code = "let x = 1; in x";

  let default_mgr = create_default_manager(None).expect("default manager");
  let replace_mgr = create_default_manager(Some(temp_replace.path()))
    .expect("replacement manager");
  let extend_mgr =
    create_default_manager(Some(temp_extend.path())).expect("extends manager");

  let default_html = default_mgr
    .highlight_code(nix_code, "nix", None)
    .expect("default highlight");
  let replace_html = replace_mgr
    .highlight_code(nix_code, "nix", None)
    .expect("replacement highlight");
  let extend_html = extend_mgr
    .highlight_code(nix_code, "nix", None)
    .expect("extends highlight");

  let count_spans = |s: &str| s.matches("<span").count();
  assert!(
    count_spans(&extend_html) >= count_spans(&replace_html),
    "extends query produced fewer spans than replacement query"
  );
  assert_ne!(
    replace_html, default_html,
    "replacement should differ from default"
  );
  assert_ne!(
    extend_html, default_html,
    "extends query should add custom highlighting to default"
  );
}

#[cfg(feature = "syntastica")]
#[test]
fn test_syntastica_custom_injection_queries_are_applied() {
  let temp_dir = tempfile::tempdir().expect("create temp dir");
  let markdown_dir = temp_dir.path().join("markdown");
  fs::create_dir_all(&markdown_dir).expect("create markdown query dir");

  let injection_query = r#"
((fenced_code_block
  (code_fence_content) @injection.content)
 (#set! injection.language "bash"))
"#;

  fs::write(markdown_dir.join("injections.scm"), injection_query)
    .expect("write custom injections query");

  let markdown = "```rust\nfn main() { println!(\"hello\"); }\n```";

  let default_manager = create_default_manager(None).expect("default manager");
  let overridden_manager = create_default_manager(Some(temp_dir.path()))
    .expect("manager with custom queries");

  let default_html = default_manager
    .highlight_code(markdown, "markdown", None)
    .expect("default markdown highlighting");
  let overridden_html = overridden_manager
    .highlight_code(markdown, "markdown", None)
    .expect("overridden markdown highlighting");

  assert!(default_html.contains("println"));
  assert!(overridden_html.contains("println"));
  assert_ne!(default_html, overridden_html);
}
