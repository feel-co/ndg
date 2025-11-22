use ndg_commonmark::{MarkdownOptions, MarkdownProcessor};

#[test]
fn test_codeblock_no_extra_newlines() {
  let processor = MarkdownProcessor::new(MarkdownOptions::default());

  let md = r#"# Test

```rust
fn main() {
    println!("line1");
    println!("line2");
    println!("line3");
}
```
"#;

  let result = processor.render(md);
  let html = result.html;

  // Count the number of <br> tags in the codeblock
  // Should be 4 lines of code = 4 <br> tags (one after each line including
  // last) NOT 8 <br> tags (which would indicate double newlines)
  let br_count = html.matches("<br>").count();

  // There should be exactly 4 <br> tags in the code (one per line)
  // If there were double newlines, we'd see 8
  assert!(
    br_count <= 5,
    "Found {} <br> tags, expected around 4-5. Double newlines may be present!",
    br_count
  );

  // Also verify no double <br><br> patterns
  assert!(
    !html.contains("<br><br>"),
    "Found double <br><br> which indicates double newlines in codeblock"
  );
}

#[test]
fn test_new_processor_role_markup_in_lists() {
  let processor = MarkdownProcessor::new(MarkdownOptions::default());

  let md = r"- {command}`nixos-rebuild switch`
- {env}`HOME`
- {file}`/etc/nixos/configuration.nix`
- {option}`services.nginx.enable`
- {var}`pkgs`
- {manpage}`nix.conf(5)`";

  let result = processor.render(md);
  let html = result.html;

  // Verify correct HTML structure with proper list items
  assert!(
    html.contains(
      r#"<li><code class="command">nixos-rebuild switch</code></li>"#
    )
  );
  assert!(html.contains(r#"<li><code class="env-var">HOME</code></li>"#));
  assert!(html.contains(
    r#"<li><code class="file-path">/etc/nixos/configuration.nix</code></li>"#
  ));
  assert!(html.contains(r#"<li><a class="option-reference" href="options.html#option-services-nginx-enable"><code class="nixos-option">services.nginx.enable</code></a></li>"#));
  assert!(html.contains(r#"<li><code class="nix-var">pkgs</code></li>"#));
  assert!(html.contains(
    r#"<li><span class="manpage-reference">nix.conf(5)</span></li>"#
  ));

  // Verify no malformed HTML patterns
  assert!(!html.contains(r#"<a class="option-reference""><li></li>"#));
  assert!(!html.contains(r#"href="<a href"#));
  assert!(!html.contains(r#"</a>"><li></li>"#));
}

#[test]
fn test_new_processor_option_reference_edge_case() {
  let processor = MarkdownProcessor::new(MarkdownOptions::default());

  let md = r"- `hjem.<user>.home` is where user home resides!";

  let result = processor.render(md);
  let html = result.html;

  // This should NOT be processed as an option reference because it contains
  // <user>
  assert!(!html.contains(r#"<a class="option-reference""#));
  assert!(html.contains(
    r"<li><code>hjem.&lt;user&gt;.home</code> is where user home resides!</li>"
  ));
}

#[test]
fn test_new_processor_valid_vs_invalid_options() {
  let processor = MarkdownProcessor::new(MarkdownOptions::default());

  let md = r"Valid: {option}`services.nginx.enable`
Invalid: `some/path.conf`
Invalid: `$HOME.config`
Invalid: `file.name.ext`
Valid: {option}`boot.loader.systemd-boot.enable`";

  let result = processor.render(md);
  let html = result.html;

  // Only the valid option paths marked with {option} should be converted to
  // links
  assert!(html.contains(r#"<a class="option-reference" href="options.html#option-services-nginx-enable"><code class="nixos-option">services.nginx.enable</code></a>"#));
  assert!(html.contains(r#"<a class="option-reference" href="options.html#option-boot-loader-systemd-boot-enable"><code class="nixos-option">boot.loader.systemd-boot.enable</code></a>"#));

  // These should remain as plain code
  assert!(html.contains(r"<code>some/path.conf</code>"));
  assert!(html.contains(r"<code>$HOME.config</code>"));
  assert!(html.contains(r"<code>file.name.ext</code>"));
}

#[test]
fn test_new_processor_headers_and_title() {
  let processor = MarkdownProcessor::new(MarkdownOptions::default());

  let md = r"# Main Title

## Section One

### Subsection

## Section Two";

  let result = processor.render(md);

  assert_eq!(result.title, Some("Main Title".to_string()));
  assert_eq!(result.headers.len(), 4);
  assert_eq!(result.headers[0].text, "Main Title");
  assert_eq!(result.headers[0].level, 1);
  assert_eq!(result.headers[1].text, "Section One");
  assert_eq!(result.headers[1].level, 2);
  assert_eq!(result.headers[2].text, "Subsection");
  assert_eq!(result.headers[2].level, 3);
  assert_eq!(result.headers[3].text, "Section Two");
  assert_eq!(result.headers[3].level, 2);
}

#[test]
fn test_new_processor_autolink() {
  let processor = MarkdownProcessor::new(MarkdownOptions::default());

  let md = "Visit https://example.com for info.";

  let result = processor.render(md);
  let html = result.html;

  // Should convert plain URLs to clickable links
  assert!(
    html.contains(r#"<a href="https://example.com">https://example.com</a>"#)
  );
}

#[test]
fn test_new_processor_autolink_multiple_urls() {
  let processor = MarkdownProcessor::new(MarkdownOptions::default());

  let md = "Check out https://github.com and also https://rust-lang.org for \
            more info.";

  let result = processor.render(md);
  let html = result.html;

  // Should convert both URLs to clickable links
  assert!(
    html.contains(r#"<a href="https://github.com">https://github.com</a>"#)
  );
  assert!(
    html
      .contains(r#"<a href="https://rust-lang.org">https://rust-lang.org</a>"#)
  );
}

#[test]
fn test_new_processor_autolink_in_existing_link() {
  let processor = MarkdownProcessor::new(MarkdownOptions::default());

  let md = r"Already a link: [Visit https://example.com](https://example.com)";

  let result = processor.render(md);
  let html = result.html;

  // Should not double-process URLs that are already inside links
  // The URL in the link text should remain as text, not become a nested link
  assert!(!html.contains(
        r#"<a href="https://example.com"><a href="https://example.com">https://example.com</a></a>"#
    ));
  assert!(html.contains(r#"<a href="https://example.com">"#));
}

#[test]
fn test_new_processor_autolink_with_punctuation() {
  let processor = MarkdownProcessor::new(MarkdownOptions::default());

  let md = "Visit https://example.com. Also try https://github.com/user/repo!";

  let result = processor.render(md);
  let html = result.html;

  // Should not include trailing punctuation in the links
  assert!(
    html.contains(r#"<a href="https://example.com">https://example.com</a>."#)
  );
  assert!(
        html.contains(
            r#"<a href="https://github.com/user/repo">https://github.com/user/repo</a>!"#
        )
    );
}

#[test]
fn test_new_processor_autolink_http_and_https() {
  let processor = MarkdownProcessor::new(MarkdownOptions::default());

  let md = "HTTP: http://example.com and HTTPS: https://example.com";

  let result = processor.render(md);
  let html = result.html;

  // Should handle both HTTP and HTTPS
  assert!(
    html.contains(r#"<a href="http://example.com">http://example.com</a>"#)
  );
  assert!(
    html.contains(r#"<a href="https://example.com">https://example.com</a>"#)
  );
}

#[test]
fn test_new_processor_error_handling() {
  let processor = MarkdownProcessor::new(MarkdownOptions::default());

  // Test with extremely malformed content that could cause issues
  let md = r"# Header with \x00 null bytes and \x{FFFF} invalid unicode

[]{#invalid-anchor-with-special-chars-\x00\x01\x02}

{malformed-role}`unclosed backtick and \x00 chars

> [!INVALID_TYPE_THAT_DOESNT_EXIST]
> This should still work even with invalid callout type

::: {.invalid-admonition-type-\x00}
Content with null bytes \x00
:::";

  let result = processor.render(md);
  let html = result.html;

  // Should not crash and should produce some HTML output
  assert!(!html.is_empty());
  assert!(html.contains("<html>"));

  // Should handle headers gracefully
  assert!(html.contains("Header"));

  // Should not crash the entire processor
  assert!(!html.contains("Error processing markdown content"));
}

#[test]
fn test_error_handling_utilities() {
  use ndg_commonmark::{process_safe, utils::never_matching_regex};

  // Test safely_process_markup with a function that panics
  let result = process_safe(
    "test input",
    |_| panic!("This function panics!"),
    "fallback output",
  );
  assert_eq!(result, "fallback output");

  // Test safely_process_markup with a normal function
  let result = process_safe(
    "test input",
    |input| format!("processed: {input}"),
    "fallback",
  );
  assert_eq!(result, "processed: test input");

  // Test never_matching_regex
  let regex =
    never_matching_regex().expect("Failed to create never matching regex");
  assert!(!regex.is_match("anything"));
  assert!(!regex.is_match(""));
  assert!(!regex.is_match("lots of text with various characters 123 !@#"));
}

#[test]
fn test_new_processor_html_post_processing() {
  let processor = MarkdownProcessor::new(MarkdownOptions::default());

  // Test comprehensive HTML post-processing with various patterns
  let md = "# Header with anchor {#my-header}

[]{#standalone-anchor}Some text with standalone anchor.

- []{#list-anchor} List item with anchor
- Regular list item

This paragraph has []{#para-anchor} an inline anchor.

[Empty link](#empty-link-target)

Text with <a href=\"#another-empty\"></a> empty HTML link.";

  let result = processor.render(md);
  let html = result.html;

  // Should process header with anchor ID
  assert!(html.contains("id=\"my-header\""));
  assert!(html.contains("Header with anchor"));

  // Should process standalone inline anchors
  assert!(
    html.contains(
      "<span id=\"standalone-anchor\" class=\"nixos-anchor\"></span>"
    )
  );

  // Should process paragraph inline anchors
  assert!(
    html.contains("<span id=\"para-anchor\" class=\"nixos-anchor\"></span>")
  );

  // Should humanize empty link text
  assert!(html.contains("<a href=\"#empty-link-target\">"));
  assert!(html.contains("<a href=\"#another-empty\">Another Empty</a>"));

  // Should not contain raw anchor syntax
  assert!(!html.contains("[]{#"));
}

#[test]
fn test_html_post_processing_edge_cases() {
  let processor = MarkdownProcessor::new(MarkdownOptions::default());

  // Test edge cases in HTML post-processing
  let md = "Text with multiple []{#anchor1} and []{#anchor2} anchors.

[]{#sec-complex-name} []{#opt-some-option} []{#ssec-subsection}

Empty links: <a href=\"#sec-introduction\"></a> and <a \
            href=\"#opt-services-nginx-enable\"></a>";

  let result = processor.render(md);
  let html = result.html;

  // Should process multiple anchors in same paragraph
  assert!(html.contains("<span id=\"anchor1\" class=\"nixos-anchor\"></span>"));
  assert!(html.contains("<span id=\"anchor2\" class=\"nixos-anchor\"></span>"));

  // Should process anchors with common prefixes and humanize them
  assert!(
    html
      .contains("<span id=\"sec-complex-name\" class=\"nixos-anchor\"></span>")
  );
  assert!(
    html
      .contains("<span id=\"opt-some-option\" class=\"nixos-anchor\"></span>")
  );
  assert!(
    html
      .contains("<span id=\"ssec-subsection\" class=\"nixos-anchor\"></span>")
  );

  // Should humanize anchor text by removing prefixes and formatting
  assert!(html.contains("<a href=\"#sec-introduction\">Introduction</a>"));
  // opt- anchors should be converted to options.html links with proper option
  // names
  assert!(html.contains(
    "<a href=\"options.html#opt-services-nginx-enable\">services.nginx.\
     enable</a>"
  ));
}
