#![expect(clippy::expect_used, reason = "Fine in tests")]

use std::fs;

use ndg_pdf::generate_pdf;
use printpdf::{PdfDocument, PdfParseOptions};
use tempfile::tempdir;

fn render_pdf_bytes(options_json: &str) -> Vec<u8> {
  let temp_dir = tempdir().expect("create tempdir");
  let options_path = temp_dir.path().join("options.json");
  let output_path = temp_dir.path().join("options.pdf");

  fs::write(&options_path, options_json).expect("write options.json");

  generate_pdf(
    &options_path,
    Some(output_path.as_path()),
    Some("Module Options"),
    Some("Header with {command}`ndg html` and [docs](https://example.org)."),
    Some("Footer with `inline` content."),
  )
  .expect("generate pdf");

  fs::read(&output_path).expect("read pdf")
}

fn parse_pdf_page_count(pdf_bytes: &[u8]) -> usize {
  let mut warnings = Vec::new();
  let document =
    PdfDocument::parse(pdf_bytes, &PdfParseOptions::default(), &mut warnings)
      .expect("parse generated pdf");
  document.page_count()
}

#[test]
fn test_pdf_semantic_markdown_rendering() {
  let options_json = r#"
{
  "hjem.linker": {
    "type": "null or package",
    "description": "Method to use to link files. `{null}` uses `systemd-tmpfiles`. Reference {option}`hjem.<user>.files`. :::{.note}\nThis linker is currently experimental.\n:::\n\n- First bullet\n- Second bullet\n  1. Nested number\n\n```nix\nhjem.linker = null;\n# keep line breaks\n```\n\nSee [upstream docs](https://example.com/docs).",
    "default": {
      "_type": "literalExpression",
      "text": "null"
    },
    "exampleText": "`hjem.linker = pkgs.my-linker;`"
  }
}
"#;

  let pdf_bytes = render_pdf_bytes(options_json);
  let pdf_text = String::from_utf8_lossy(&pdf_bytes);

  assert!(!pdf_text.contains("{option}`"), "role syntax leaked");
  assert!(!pdf_text.contains(":::{.note}"), "admonition syntax leaked");
  assert!(
    !pdf_text.contains("options.html#option-"),
    "option role links must not be rendered as raw internal URLs"
  );

  assert!(pdf_bytes.starts_with(b"%PDF-"), "PDF header missing");
  assert!(
    parse_pdf_page_count(&pdf_bytes) >= 1,
    "generated PDF has no pages"
  );
}

#[test]
fn test_pdf_stress_with_large_blocks() {
  let long_code = (0..160)
    .map(|n| format!("line-{n:03}: let value = {};", n * 7))
    .collect::<Vec<_>>()
    .join("\n");
  let long_code_json = long_code.replace('\n', "\\n");

  let options_json = format!(
    r#"{{
  "services.massive": {{
    "type": "attrs",
    "description": ":::{{.warning}}\nLong warning block start.\n{}\n:::\n\n```nix\n{}\n```"
  }}
}}"#,
    "A very long paragraph. ".repeat(120),
    long_code_json
  );

  let pdf_bytes = render_pdf_bytes(&options_json);
  let pdf_text = String::from_utf8_lossy(&pdf_bytes);

  assert!(!pdf_text.contains(":::{{.warning}}"));
  assert!(!pdf_text.contains("```nix"));
  assert!(
    parse_pdf_page_count(&pdf_bytes) >= 1,
    "generated PDF has no pages"
  );
}
