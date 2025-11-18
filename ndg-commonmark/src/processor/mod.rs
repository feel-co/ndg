//! Markdown processing module with modular organization.
//!
//! This module provides a comprehensive, trait-based architecture for
//! processing Markdown content with support for various extensions and output
//! formats.
//!
//! # Architecture
//!
//! The processor module is organized into focused submodules:
//!
//! - [`core`]: Main processor implementation and processing pipeline
//! - [`process`]: High-level processing functions with error recovery
//! - [`extensions`]: Feature-gated processing functions for different Markdown
//!   flavors
//! - [`types`]: Core type definitions and configuration structures
pub mod core;
pub mod extensions;
pub mod process;
pub mod types;

// Re-export commonly used types from submodules
pub use core::{ProcessorFeature, collect_markdown_files, extract_inline_text};

// Re-export extension functions for third-party use
#[cfg(feature = "gfm")]
pub use extensions::apply_gfm_extensions;
#[cfg(feature = "nixpkgs")]
pub use extensions::process_manpage_references;
pub use extensions::process_myst_autolinks;
#[cfg(feature = "ndg-flavored")]
pub use extensions::process_option_references;
#[cfg(any(feature = "nixpkgs", feature = "ndg-flavored"))]
pub use extensions::process_role_markup;
#[cfg(feature = "nixpkgs")]
pub use extensions::{
  process_block_elements,
  process_file_includes,
  process_inline_anchors,
};
pub use process::{
  ProcessorPreset,
  create_processor,
  process_batch,
  process_markdown_file,
  process_markdown_file_with_basedir,
  process_markdown_string,
  process_safe,
  process_with_recovery,
};
pub use types::{
  AstTransformer,
  MarkdownOptions,
  MarkdownOptionsBuilder,
  MarkdownProcessor,
  PromptTransformer,
};

#[cfg(test)]
mod tests {
  use html_escape;

  use super::{MarkdownOptions, MarkdownProcessor, types::TabStyle};

  #[test]
  fn test_html_escaped_roles() {
    // Test that HTML characters in role content are properly escaped
    #[cfg(any(feature = "nixpkgs", feature = "ndg-flavored"))]
    {
      let result = super::extensions::format_role_markup(
        "option",
        "hjem.users.<name>.enable",
        None,
        true,
        None,
      );

      // Should escape < and > characters in content
      assert!(result.contains("&lt;name&gt;"));
      // Should not contain unescaped HTML in code content
      assert!(!result.contains("<code>hjem.users.<name>.enable</code>"));
      // Should contain escaped content in code with proper class
      assert!(result.contains(
        "<code class=\"nixos-option\">hjem.users.&lt;name&gt;.enable</code>"
      ));
      // Should have properly formatted option ID in href with preserved special
      // chars
      assert!(result.contains("option-hjem-users-<name>-enable"));
    }
  }

  #[test]
  fn test_html_escape_util() {
    let input = "test<>&\"'";
    let escaped = html_escape::encode_text(input);

    // html-escape crate doesn't escape single quotes by default
    assert_eq!(escaped, "test&lt;&gt;&amp;\"'");
  }

  #[test]
  fn test_various_role_types_with_html_characters() {
    #[cfg(any(feature = "nixpkgs", feature = "ndg-flavored"))]
    {
      let content = "<script>alert('xss')</script>";

      let command_result = super::extensions::format_role_markup(
        "command", content, None, true, None,
      );
      assert!(command_result.contains("&lt;script&gt;"));
      assert!(!command_result.contains("<script>alert"));

      let env_result =
        super::extensions::format_role_markup("env", content, None, true, None);
      assert!(env_result.contains("&lt;script&gt;"));
      assert!(!env_result.contains("<script>alert"));

      let file_result = super::extensions::format_role_markup(
        "file", content, None, true, None,
      );
      assert!(file_result.contains("&lt;script&gt;"));
      assert!(!file_result.contains("<script>alert"));
    }
  }

  #[test]
  fn test_option_role_escaping() {
    // Test the specific reported issue: {option}`hjem.users.<name>.enable`
    #[cfg(any(feature = "nixpkgs", feature = "ndg-flavored"))]
    {
      let result = super::extensions::format_role_markup(
        "option",
        "hjem.users.<name>.enable",
        None,
        true,
        None,
      );

      // Should not produce broken HTML like:
      // <code>hjem.users.<name>.enable</name></code>
      assert!(!result.contains("</name>"));

      // Should properly escape the angle brackets in display text
      assert!(result.contains("&lt;name&gt;"));

      // Should produce valid HTML structure with proper class
      assert!(result.contains(
        "<code class=\"nixos-option\">hjem.users.&lt;name&gt;.enable</code>"
      ));

      // Should preserve special characters in the option ID (the actual anchor)
      assert!(result.contains("options.html#option-hjem-users-<name>-enable"));
    }
  }

  #[test]
  fn test_option_role_special_chars_preserved() {
    // Test that special characters are preserved in option IDs
    #[cfg(any(feature = "nixpkgs", feature = "ndg-flavored"))]
    {
      let result = super::extensions::format_role_markup(
        "option",
        "services.foo.<bar>.enable",
        None,
        true,
        None,
      );

      // Option ID should preserve angle brackets
      assert!(result.contains("option-services-foo-<bar>-enable"));

      // Display text should be HTML escaped
      assert!(result.contains("&lt;bar&gt;"));
    }
  }

  #[test]
  fn test_hardtab_handling_none() {
    let options = MarkdownOptions {
      tab_style: TabStyle::None,
      highlight_code: false,
      ..Default::default()
    };
    let processor = MarkdownProcessor::new(options);

    let markdown = r#"
# Test Code

```rust
fn main() {
	println!("Hello, world!");
}
```
"#;

    let result = processor.render(markdown);
    assert!(result.html.contains("\tprintln"));
  }

  #[test]
  fn test_hardtab_handling_warn() {
    let options = MarkdownOptions {
      tab_style: TabStyle::Warn,
      highlight_code: false,
      ..Default::default()
    };
    let processor = MarkdownProcessor::new(options);

    let markdown = r#"
# Test Code

```rust
fn main() {
	println!("Hello, world!");
}
```
"#;

    let result = processor.render(markdown);
    // Should preserve hard tabs but issue warning
    assert!(result.html.contains("\tprintln"));
  }

  #[test]
  fn test_hardtab_handling_normalize() {
    let options = MarkdownOptions {
      tab_style: TabStyle::Normalize,
      highlight_code: false,
      ..Default::default()
    };
    let processor = MarkdownProcessor::new(options);

    let markdown = r#"
# Test Code

```rust
fn main() {
	println!("Hello, world!");
}
```
"#;

    let result = processor.render(markdown);
    // Should convert hard tabs to 2 spaces
    assert!(!result.html.contains("\tprintln"));
    assert!(result.html.contains("  println"));
  }

  #[test]
  fn test_hardtab_handling_no_tabs() {
    let options = MarkdownOptions {
      tab_style: TabStyle::Warn,
      highlight_code: false,
      ..Default::default()
    };
    let processor = MarkdownProcessor::new(options);

    let markdown = r#"
# Test Code

```rust
fn main() {
    println!("Hello, world!");
}
```
"#;

    let result = processor.render(markdown);
    // Should work fine when no tabs are present
    assert!(result.html.contains("    println"));
    assert!(!result.html.contains("\t"));
  }

  #[test]
  fn test_hardtab_handling_mixed_content() {
    let options = MarkdownOptions {
      tab_style: TabStyle::Normalize,
      highlight_code: false,
      ..Default::default()
    };
    let processor = MarkdownProcessor::new(options);

    let markdown = r#"
# Test Code

```rust
fn main() {
	println!("Hello");  // tab here
    println!("World");  // spaces here
}
```
"#;

    let result = processor.render(markdown);
    // Should convert only tabs, preserve spaces
    assert!(!result.html.contains("\tprintln"));
    assert!(result.html.contains("  println"));
    assert!(result.html.contains("    println"));
  }
}
