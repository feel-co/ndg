use comrak::{Arena, ComrakOptions, parse_document};
use ndg_commonmark::legacy_markdown::{AstTransformer, PromptTransformer};

/// Render AST to HTML after transformation.
fn render_html_after_transform(markdown: &str, transformer: &dyn AstTransformer) -> String {
    let arena = Arena::new();
    let mut options = ComrakOptions::default();
    options.extension.table = true;
    options.extension.footnotes = true;
    options.extension.strikethrough = true;
    options.extension.tasklist = true;
    options.extension.header_ids = Some(String::new());
    options.render.unsafe_ = true;

    let root = parse_document(&arena, markdown, &options);
    transformer.transform(root);

    let mut html_output = Vec::new();
    comrak::format_html(root, &options, &mut html_output).unwrap();
    String::from_utf8(html_output).unwrap()
}

#[test]
fn test_shell_prompt_transformation() {
    let transformer = PromptTransformer;
    let md = r"Run `\$ echo hello` for output.";
    let html = render_html_after_transform(md, &transformer);
    println!("DEBUG test_shell_prompt_transformation HTML: {html}");
    // Escaped $ should not be transformed
    assert!(html.contains("<code>\\$ echo hello</code>"));
}

#[test]
fn test_repl_prompt_transformation() {
    let transformer = PromptTransformer;
    let md = "Try `nix-repl> 1 + 1` in the REPL.";
    let html = render_html_after_transform(md, &transformer);
    assert!(html.contains(
        "<code class=\"nix-repl\"><span class=\"prompt\">nix-repl&gt;</span> 1 + 1</code>"
    ));
}

#[test]
fn test_no_prompt_code_span() {
    let transformer = PromptTransformer;
    let md = "This is just `inline code`.";
    let html = render_html_after_transform(md, &transformer);
    assert!(html.contains("<code>inline code</code>"));
    assert!(!html.contains("class=\"terminal\""));
    assert!(!html.contains("class=\"nix-repl\""));
}

#[test]
fn test_multiple_prompts_and_normal_code() {
    let transformer = PromptTransformer;
    let md = "Shell: `$ ls -l` REPL: `nix-repl> help` Normal: `foo`";
    let html = render_html_after_transform(md, &transformer);
    assert!(html.contains("<code class=\"terminal\"><span class=\"prompt\">$</span> ls -l</code>"));
    assert!(html.contains(
        "<code class=\"nix-repl\"><span class=\"prompt\">nix-repl&gt;</span> help</code>"
    ));
    assert!(html.contains("<code>foo</code>"));
}

#[test]
fn test_prompt_with_leading_and_trailing_whitespace() {
    let transformer = PromptTransformer;
    let md = "Prompt: `  $   echo hi  `";
    let html = render_html_after_transform(md, &transformer);
    println!("DEBUG test_prompt_with_leading_and_trailing_whitespace HTML: {html}");
    // Should trim and still match as shell prompt
    assert!(
        html.contains("<code class=\"terminal\"><span class=\"prompt\">$</span> echo hi</code>")
    );
}

#[test]
fn test_prompt_inside_code_block_is_untouched() {
    let transformer = PromptTransformer;
    let md = "```\n$ echo not a prompt\n```\n";
    let html = render_html_after_transform(md, &transformer);
    // Code blocks should not be transformed
    assert!(html.contains("<pre><code>$ echo not a prompt\n</code></pre>"));
}

#[test]
fn test_edge_case_not_a_prompt() {
    let transformer = PromptTransformer;
    let md = "Not a prompt: `$$ foo` and `nix-repl>> bar`";
    let html = render_html_after_transform(md, &transformer);
    println!("DEBUG test_edge_case_not_a_prompt HTML: {html}");
    // Should not match or transform
    assert!(html.contains("<code>$$ foo</code>"));
    assert!(html.contains("<code>nix-repl&gt;&gt; bar</code>"));
}
