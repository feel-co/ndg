use std::fs;

use ndg_commonmark::{MarkdownOptions, MarkdownProcessor};

fn main() -> Result<(), Box<dyn std::error::Error>> {
  println!("Testing NDG CommonMark Processor");
  println!("================================\n");

  let demo_content = fs::read_to_string("demo/test-all-features.md").expect(
    "Failed to read demo file. Make sure you're running from the ndg root \
     directory.",
  );

  println!("Processing demo markdown file...\n");

  // Create processor with all features enabled
  let mut options: MarkdownOptions = MarkdownOptions::default();
  options.gfm = true;
  options.nixpkgs = true;

  let processor = MarkdownProcessor::new(options);

  // Process the markdown
  let result = processor.render(&demo_content);

  println!("Processing completed successfully!");
  println!("Results:");
  println!("  - Title: {:?}", result.title);
  println!("  - Headers found: {}", result.headers.len());
  println!("  - HTML output length: {} characters", result.html.len());

  // Display headers
  if !result.headers.is_empty() {
    println!("\nHeaders extracted:");
    for (i, header) in result.headers.iter().enumerate() {
      println!(
        "  {}. {} (level {}) -> #{}",
        i + 1,
        header.text,
        header.level,
        header.id
      );
    }
  }

  // Write output to file
  fs::create_dir_all("demo/output")?;
  fs::write("demo/output/test-all-features.html", &result.html)?;
  println!("\nHTML output saved to: demo/output/test-all-features.html");

  // Test role markup
  let role_test = "Use {command}`systemctl status` to check service status.";
  let role_result = processor.render(role_test);
  println!(
    "Role markup: {}",
    if role_result.html.contains("class=\"command\"") {
      "WORKING"
    } else {
      "FAILED"
    }
  );

  // Test command prompts
  let prompt_test = "Run `$ sudo systemctl restart nginx` to restart nginx.";
  let prompt_result = processor.render(prompt_test);
  println!(
    "  ✓ Command prompts: {}",
    if prompt_result.html.contains("class=\"terminal\"") {
      "WORKING"
    } else {
      "FAILED"
    }
  );

  // Test REPL prompts
  let repl_test = "Try `nix-repl> builtins.toString 42` in the REPL.";
  let repl_result = processor.render(repl_test);
  println!(
    "REPL prompts: {}",
    if repl_result.html.contains("class=\"nix-repl\"") {
      "WORKING"
    } else {
      "FAILED"
    }
  );

  // Test header anchors
  let anchor_test = "## Test Header {#custom-id}";
  let anchor_result = processor.render(anchor_test);
  println!(
    "  ✓ Header anchors: {}",
    if anchor_result.html.contains("id=\"custom-id\"") {
      "WORKING"
    } else {
      "FAILED"
    }
  );

  // Test inline anchors
  let inline_anchor_test = "Text with []{#inline-id} inline anchor.";
  let inline_anchor_result = processor.render(inline_anchor_test);
  println!(
    "  ✓ Inline anchors: {}",
    if inline_anchor_result.html.contains("id=\"inline-id\"") {
      "WORKING"
    } else {
      "FAILED"
    }
  );

  // Test admonitions
  let admonition_test = "> [!NOTE]\n> This is a note.";
  let admonition_result = processor.render(admonition_test);
  println!(
    "  ✓ Admonitions: {}",
    if admonition_result.html.contains("class=\"admonition note\"") {
      "WORKING"
    } else {
      "FAILED"
    }
  );

  // Test autolinks
  let autolink_test = "Visit https://nixos.org for more information.";
  let autolink_result = processor.render(autolink_test);
  println!(
    "  ✓ Autolinks: {}",
    if autolink_result
      .html
      .contains("<a href=\"https://nixos.org\"")
    {
      "WORKING"
    } else {
      "FAILED"
    }
  );

  // Test tables
  let table_test =
    "| Header 1 | Header 2 |\n|----------|----------|\n| Cell 1   | Cell 2   |";
  let table_result = processor.render(table_test);
  println!(
    "  ✓ Tables: {}",
    if table_result.html.contains("<table>") {
      "WORKING"
    } else {
      "FAILED"
    }
  );

  // Test footnotes
  let footnote_test = "Text with footnote[^1].\n\n[^1]: This is a footnote.";
  let footnote_result = processor.render(footnote_test);
  println!(
    "  ✓ Footnotes: {}",
    if footnote_result.html.contains("class=\"footnote-ref\"") {
      "WORKING"
    } else {
      "FAILED"
    }
  );

  // Test task lists
  let tasklist_test = "- [x] Completed task\n- [ ] Incomplete task";
  let tasklist_result = processor.render(tasklist_test);
  println!(
    "  ✓ Task lists: {}",
    if tasklist_result.html.contains("type=\"checkbox\"") {
      "WORKING"
    } else {
      "FAILED"
    }
  );

  println!("\nAll tests completed!");
  println!("Demo output available at: demo/output/test-all-features.html");

  // Create a simple HTML wrapper for better viewing
  let html_wrapper = format!(
    r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>NDG Demo</title>
    <style>
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
            line-height: 1.6;
            max-width: 800px;
            margin: 2rem auto;
            padding: 0 1rem;
            color: #333;
        }}
        .role {{ padding: 0.2em 0.4em; border-radius: 3px; font-family: monospace; }}
        .role.command {{ background: #e3f2fd; color: #1565c0; }}
        .role.option {{ background: #f3e5f5; color: #7b1fa2; }}
        .role.file {{ background: #e8f5e8; color: #2e7d32; }}
        .role.env {{ background: #fff3e0; color: #f57c00; }}
        .role.var {{ background: #fce4ec; color: #c2185b; }}
        .terminal {{ background: #2d3748; color: #e2e8f0; padding: 0.5em; border-radius: 4px; }}
        .terminal .prompt {{ color: #68d391; font-weight: bold; }}
        .nix-repl {{ background: #1a202c; color: #e2e8f0; padding: 0.5em; border-radius: 4px; }}
        .nix-repl .prompt {{ color: #63b3ed; font-weight: bold; }}
        .admonition {{ margin: 1em 0; padding: 1em; border-left: 4px solid; border-radius: 4px; }}
        .admonition.note {{ background: #e3f2fd; border-color: #2196f3; }}
        .admonition.warning {{ background: #fff3e0; border-color: #ff9800; }}
        .admonition.tip {{ background: #e8f5e8; border-color: #4caf50; }}
        .admonition.important {{ background: #f3e5f5; border-color: #9c27b0; }}
        .admonition.caution {{ background: #fff8e1; border-color: #ffc107; }}
        .admonition.danger {{ background: #ffebee; border-color: #f44336; }}
        .nixos-anchor {{ color: #666; text-decoration: none; }}
        table {{ border-collapse: collapse; width: 100%; margin: 1em 0; }}
        th, td {{ border: 1px solid #ddd; padding: 0.5em; text-align: left; }}
        th {{ background: #f5f5f5; font-weight: bold; }}
        code {{ background: #f5f5f5; padding: 0.2em 0.4em; border-radius: 3px; font-family: monospace; }}
        pre {{ background: #f5f5f5; padding: 1em; border-radius: 4px; overflow-x: auto; }}
        blockquote {{ margin: 1em 0; padding: 0 1em; border-left: 4px solid #ddd; color: #666; }}
    </style>
</head>
<body>
{}
</body>
</html>"#,
    result.html
  );

  fs::write("demo/output/demo-with-styles.html", html_wrapper)?;
  println!(
    "Styled demo output available at: demo/output/demo-with-styles.html"
  );

  Ok(())
}
