use ndg_commonmark::{MarkdownOptions, MarkdownProcessor};

#[test]
fn test_new_processor_role_markup_in_lists() {
    let processor = MarkdownProcessor::new(MarkdownOptions::default());

    let md = r#"- {command}`nixos-rebuild switch`
- {env}`HOME`
- {file}`/etc/nixos/configuration.nix`
- {option}`services.nginx.enable`
- {var}`pkgs`
- {manpage}`nix.conf(5)`"#;

    let result = processor.render(md);
    let html = result.html;

    // Verify correct HTML structure with proper list items
    assert!(html.contains(r#"<li><code class="command">nixos-rebuild switch</code></li>"#));
    assert!(html.contains(r#"<li><code class="env-var">HOME</code></li>"#));
    assert!(
        html.contains(r#"<li><code class="file-path">/etc/nixos/configuration.nix</code></li>"#)
    );
    assert!(html.contains(r#"<li><a class="option-reference" href="options.html#option-services-nginx-enable"><code>services.nginx.enable</code></a></li>"#));
    assert!(html.contains(r#"<li><code class="nix-var">pkgs</code></li>"#));
    assert!(html.contains(r#"<li><span class="manpage-reference">nix.conf(5)</span></li>"#));

    // Verify no malformed HTML patterns
    assert!(!html.contains(r#"<a class="option-reference""><li></li>"#));
    assert!(!html.contains(r#"href="<a href"#));
    assert!(!html.contains(r#"</a>"><li></li>"#));
}

#[test]
fn test_new_processor_option_reference_edge_case() {
    let processor = MarkdownProcessor::new(MarkdownOptions::default());

    let md = r#"- `hjem.<user>.home` is where user home resides!"#;

    let result = processor.render(md);
    let html = result.html;

    // This should NOT be processed as an option reference because it contains <user>
    assert!(!html.contains(r#"<a class="option-reference""#));
    assert!(
        html.contains(
            r#"<li><code>hjem.&lt;user&gt;.home</code> is where user home resides!</li>"#
        )
    );
}

#[test]
fn test_new_processor_valid_vs_invalid_options() {
    let processor = MarkdownProcessor::new(MarkdownOptions::default());

    let md = r#"Valid: `services.nginx.enable`
Invalid: `some/path.conf`
Invalid: `$HOME.config`
Invalid: `file.name.ext`
Valid: `boot.loader.systemd-boot.enable`"#;

    let result = processor.render(md);
    let html = result.html;

    // Only the valid option paths should be converted to links
    assert!(html.contains(r#"<a class="option-reference" href="options.html#option-services-nginx-enable"><code>services.nginx.enable</code></a>"#));
    assert!(html.contains(r#"<a class="option-reference" href="options.html#option-boot-loader-systemd-boot-enable"><code>boot.loader.systemd-boot.enable</code></a>"#));

    // These should remain as plain code
    assert!(html.contains(r#"<code>some/path.conf</code>"#));
    assert!(html.contains(r#"<code>$HOME.config</code>"#));
    assert!(html.contains(r#"<code>file.name.ext</code>"#));
}

#[test]
fn test_new_processor_headers_and_title() {
    let processor = MarkdownProcessor::new(MarkdownOptions::default());

    let md = r#"# Main Title

## Section One

### Subsection

## Section Two"#;

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
