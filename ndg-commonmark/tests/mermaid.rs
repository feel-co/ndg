#![allow(clippy::expect_used, clippy::unwrap_used, reason = "Fine in tests")]
use ndg_commonmark::{MarkdownOptions, MarkdownProcessor};

const DIAGRAM: &str = "```mermaid\nflowchart LR\n  a --> b\n```\n";

fn render(options: MarkdownOptions, md: &str) -> String {
  MarkdownProcessor::new(options).render(md).html
}

#[test]
fn mermaid_disabled_keeps_code_block() {
  let html = render(
    MarkdownOptions {
      highlight_code: false,
      ..Default::default()
    },
    DIAGRAM,
  );
  assert!(html.contains("language-mermaid"), "{html}");
  assert!(!html.contains(r#"<pre class="mermaid">"#), "{html}");
}

#[test]
fn mermaid_enabled_renders_diagram_element() {
  let html = render(
    MarkdownOptions {
      highlight_code: false,
      mermaid: true,
      ..Default::default()
    },
    DIAGRAM,
  );
  assert!(
    html.contains("<pre class=\"mermaid\">flowchart LR\n  a --&gt; b\n</pre>"),
    "{html}"
  );
  assert!(!html.contains("language-mermaid"), "{html}");
}

#[test]
fn mermaid_enabled_skips_highlighting() {
  let options = MarkdownOptions {
    mermaid: true,
    ..Default::default()
  };
  let highlight_code = options.highlight_code;
  let html =
    render(options, &format!("{DIAGRAM}\n```nix\n{{ a = 1; }}\n```\n"));
  assert!(
    html.contains("<pre class=\"mermaid\">flowchart LR\n  a --&gt; b\n</pre>"),
    "{html}"
  );
  if highlight_code {
    // Other code blocks are still highlighted.
    assert!(html.contains(r#"<pre class="highlight">"#), "{html}");
  }
}

#[test]
fn mermaid_enabled_leaves_other_languages() {
  let html = render(
    MarkdownOptions {
      highlight_code: false,
      mermaid: true,
      ..Default::default()
    },
    "```text\nflowchart LR\n```\n",
  );
  assert!(!html.contains(r#"<pre class="mermaid">"#), "{html}");
}
