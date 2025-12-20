#![allow(clippy::expect_used, clippy::panic, reason = "Fine in tests")]
use ndg_commonmark::{MarkdownOptions, MarkdownProcessor};

/// Integration test to verify complete migration from legacy to new processor
/// Mainly I just want to make sure this went as smoothly as it could, and that
/// the new parser is as robust as I expect.
#[test]
fn test_migration_integration() {
  let processor = MarkdownProcessor::new(MarkdownOptions::default());

  let complex_markdown = r#"# Main Documentation

This document demonstrates all features working together.

## Basic Formatting

Here's some **bold** and *italic* text with `inline code`.

## Code Blocks

```rust
fn main() {
    println!("Hello, world!");
}
```

## Role Markup

- {command}`nixos-rebuild switch`
- {env}`HOME`
- {file}`/etc/nixos/configuration.nix`
- {option}`services.nginx.enable`
- {var}`pkgs`
- {manpage}`nix.conf(5)`

## Autolinks

Visit https://nixos.org for more information.

## Headers with Anchors

### Configuration {#config}

[]{#config-section}This section talks about configuration.

## Lists with Anchors

- []{#first-item} First item
- Regular item
- []{#third-item} Third item

## Admonitions

::: {.note}
This is an important note about the system.
:::

::: {.warning}
Be careful with this configuration!
:::

## Tables

| Feature | Status |
|---------|--------|
| Autolinks | ✅ |
| Role Markup | ✅ |
| Anchors | ✅ |

## Footnotes

Here's a footnote reference[^1].

[^1]: This is the footnote text.

## Task Lists

- [x] Migrate autolink processing
- [x] Migrate role markup
- [x] Migrate HTML post-processing
- [ ] Complete Phase 2 testing

## Definition Lists

Term 1
:   Definition for term 1

Term 2
:   Definition for term 2

## Empty Links

Check out the [introduction](#intro) section.
Visit the [](#getting-started) guide.
"#;

  let result = processor.render(complex_markdown);

  // Verify basic structure
  assert!(!result.html.is_empty());
  assert!(result.html.contains("<html>"));
  assert!(result.html.contains("</html>"));

  // Verify title extraction
  assert_eq!(result.title, Some("Main Documentation".to_string()));

  // Verify header extraction
  assert!(result.headers.len() >= 4);
  assert_eq!(result.headers[0].text, "Main Documentation");
  assert_eq!(result.headers[0].level, 1);

  // Verify role markup processing
  assert!(
    result
      .html
      .contains(r#"<code class="command">nixos-rebuild switch</code>"#)
  );
  assert!(result.html.contains(r#"<code class="env-var">HOME</code>"#));
  assert!(result.html.contains(
    r#"<code class="file-path">/etc/nixos/configuration.nix</code>"#
  ));
  assert!(result.html.contains(r#"<a class="option-reference" href="options.html#option-services-nginx-enable"><code class="nixos-option">services.nginx.enable</code></a>"#));
  assert!(result.html.contains(r#"<code class="nix-var">pkgs</code>"#));
  assert!(
    result
      .html
      .contains(r#"<span class="manpage-reference">nix.conf(5)</span>"#)
  );

  // Verify autolink processing
  assert!(
    result
      .html
      .contains(r#"<a href="https://nixos.org">https://nixos.org</a>"#)
  );

  // Verify anchor processing
  assert!(result.html.contains(r#"id="config""#));
  assert!(
    result
      .html
      .contains(r#"<span id="config-section" class="nixos-anchor"></span>"#)
  );
  assert!(
    result
      .html
      .contains(r#"<span id="first-item" class="nixos-anchor"></span>"#)
  );
  assert!(
    result
      .html
      .contains(r#"<span id="third-item" class="nixos-anchor"></span>"#)
  );

  // Verify admonitions
  assert!(result.html.contains(r#"<div class="admonition note""#));
  assert!(result.html.contains(r#"<div class="admonition warning""#));

  // Verify GFM features (tables, task lists, footnotes)
  assert!(result.html.contains("<table>"));
  assert!(result.html.contains("<thead>"));
  assert!(result.html.contains("<tbody>"));
  assert!(
    result
      .html
      .contains(r#"<input type="checkbox" checked="" disabled="">"#)
  );
  assert!(
    result
      .html
      .contains(r#"<input type="checkbox" disabled="">"#)
  );
  assert!(result.html.contains("footnote"));

  // Verify definition lists
  assert!(result.html.contains("<dl>"));
  assert!(result.html.contains("<dt>"));
  assert!(result.html.contains("<dd>"));

  // Verify empty link humanization
  assert!(result.html.contains("<a href=\"#intro\">introduction</a>"));

  // Verify empty anchor links are processed correctly
  assert!(
    result
      .html
      .contains("<a href=\"#getting-started\">Getting Started</a>")
  );

  // Verify no malformed HTML
  assert!(!result.html.contains("href=\"<a href"));
  assert!(!result.html.contains("</a>\"></a>"));
  assert!(
    !result
      .html
      .contains("<a class=\"option-reference\"\"><li></li>")
  );

  // Verify proper HTML structure
  assert!(result.html.contains("<body>"));
  assert!(result.html.contains("</body>"));
}

/// Test that the new processor handles edge cases gracefully
#[test]
fn test_edge_cases_integration() {
  let processor = MarkdownProcessor::new(MarkdownOptions::default());

  let edge_case_markdown = r#"
# Header with special chars & symbols {#special-id}

[]{#anchor-with-unicode-🦀}Text with unicode anchor.

{unknown-role}`some content`

Malformed role: {incomplete

Empty option: {option}``

Multiple anchors: []{#a1}[]{#a2}[]{#a3}

URL in link: [Visit https://example.com](https://example.com)

Nested emphasis: ***very important***

Code with prompt: `$ echo "test"`

Option-like but invalid: `$HOME/config` and `some.file.ext`

Valid option: {option}`boot.loader.grub.enable`
"#;

  let result = processor.render(edge_case_markdown);

  // Should not crash and produce valid HTML
  assert!(!result.html.is_empty());
  assert!(result.html.contains("<html>"));

  // Should handle unicode in anchors
  assert!(result.html.contains("anchor-with-unicode-🦀"));

  // Should handle unknown roles gracefully
  // Incomplete role markup should be preserved as literal text
  assert!(result.html.contains("{unknown-role}"));

  // Should preserve empty option role markup as literal text (including
  // backticks)
  assert!(result.html.contains("{option}``"));

  // Should handle multiple anchors
  assert!(result.html.contains(r#"id="a1""#));
  assert!(result.html.contains(r#"id="a2""#));
  assert!(result.html.contains(r#"id="a3""#));

  // Should not double-process URLs in existing links
  assert!(!result.html.contains(
    r#"<a href="https://example.com"><a href="https://example.com">"#
  ));

  // Should process valid options only when explicitly marked
  assert!(result.html.contains(
        r#"<a class="option-reference" href="options.html#option-boot-loader-grub-enable""#
    ));
  assert!(result.html.contains("boot-loader-grub-enable"));

  // Should not auto-link things that look like filenames
  assert!(result.html.contains(r"<code>$HOME/config</code>"));
  assert!(result.html.contains(r"<code>some.file.ext</code>"));
  assert!(!result.html.contains(
    r#"<a class="option-reference" href="options.html#option-some-file-ext""#
  ));

  // Should handle prompt transformation
  assert!(result.html.contains(r#"<span class="prompt">$</span>"#));
}

/// Test that options processing works correctly
#[test]
fn test_options_integration() {
  let mut options = MarkdownOptions::default();
  options.gfm = true;
  options.nixpkgs = true;

  let processor = MarkdownProcessor::new(options);

  let markdown = r"
# Test Document

Standard option: {option}`services.nginx.enable`

Explicit option in code: {option}`boot.initrd.luks.devices`

Not an option: `file.name.txt`

Table with options:

| Option | Description |
|--------|-------------|
| {option}`services.openssh.enable` | SSH service |
| {option}`networking.hostName` | System hostname |
";

  let result = processor.render(markdown);

  // Should process role-based options
  assert!(result.html.contains(
        r#"<a class="option-reference" href="options.html#option-services-nginx-enable">"#
    ));

  // Should process explicitly marked options
  assert!(result.html.contains(
        r#"<a class="option-reference" href="options.html#option-boot-initrd-luks-devices">"#
    ));

  // Should not process plain code that looks like a filename
  assert!(!result.html.contains(
    r#"<a class="option-reference" href="options.html#option-file-name-txt""#
  ));
  // Instead, file.name.txt should remain as plain code
  assert!(result.html.contains(r"<code>file.name.txt</code>"));

  // Should process options in tables when explicitly marked with {option} role
  assert!(result.html.contains("option-services-openssh-enable"));
  assert!(result.html.contains("option-networking-hostName"));
}
