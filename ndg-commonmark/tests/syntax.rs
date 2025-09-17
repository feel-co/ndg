//! Integration tests for syntax highlighting functionality.

use ndg_commonmark::{
  MarkdownOptions,
  MarkdownProcessor,
  create_default_manager,
};

#[test]
fn test_basic_syntax_highlighting_integration() {
  let mut options = MarkdownOptions::default();
  options.highlight_code = true;

  let processor = MarkdownProcessor::new(options);

  let markdown = r#"
# Test Document

Here's some Rust code:

```rust
fn main() {
    println!("Hello, world!");
    let x = 42;
}
```

And some JavaScript:

```javascript
function greet(name) {
    console.log(`Hello, ${name}!`);
}
```

And some Nix:

```nix
{ pkgs, ... }:
{
  environment.systemPackages = with pkgs; [
    vim
    git
  ];
}
```
"#;

  let result = processor.render(markdown);

  // Check that HTML was generated
  assert!(!result.html.is_empty());

  // Check that code blocks are present and highlighted
  // Syntastica produces inline spans with color styling instead of <pre> tags
  assert!(result.html.contains("<span"));
  assert!(result.html.contains("main"));
  assert!(result.html.contains("println"));
  assert!(result.html.contains("greet"));
  assert!(result.html.contains("pkgs"));

  // Verify syntax highlighting is actually working by checking for color styles
  assert!(result.html.contains("color:rgb"));
}

#[test]
fn test_syntax_highlighting_with_unsupported_language() {
  let mut options = MarkdownOptions::default();
  options.highlight_code = true;

  let processor = MarkdownProcessor::new(options);

  let markdown = r#"
```nonexistent-language
some code here
that should still be wrapped
```
"#;

  let result = processor.render(markdown);

  // Should still generate HTML even for unsupported languages
  assert!(!result.html.is_empty());
  assert!(result.html.contains("some code here"));
}

#[test]
fn test_syntax_highlighting_disabled() {
  let mut options = MarkdownOptions::default();
  options.highlight_code = false;

  let processor = MarkdownProcessor::new(options);

  let markdown = r#"
```rust
fn main() {
    println!("Hello, world!");
}
```
"#;

  let result = processor.render(markdown);

  // Should still have code blocks but without syntax highlighting
  assert!(!result.html.is_empty());
  assert!(result.html.contains("fn main"));
  // When highlighting is disabled, should not contain color styling
  assert!(!result.html.contains("color:rgb"));
}

#[cfg(feature = "syntastica")]
#[test]
fn test_syntastica_backend_directly() {
  use ndg_commonmark::syntax::{SyntasticaHighlighter, SyntaxHighlighter};

  let highlighter = SyntasticaHighlighter::new()
    .expect("Failed to create Syntastica highlighter");

  // Test basic highlighting
  let result =
    highlighter.highlight("fn main() { println!(\"Hello\"); }", "rust", None);

  assert!(result.is_ok());
  let html = result.unwrap();
  assert!(html.contains("main"));
  assert!(html.contains("println"));

  // Test language support
  assert!(highlighter.supports_language("rust"));
  assert!(highlighter.supports_language("nix"));
  assert!(highlighter.supports_language("javascript"));
  assert!(!highlighter.supports_language("nonexistent"));

  // Test theme availability
  let themes = highlighter.available_themes();
  assert!(!themes.is_empty());
  assert!(themes.contains(&"one::dark".to_string()));
}

#[cfg(feature = "syntect")]
#[test]
fn test_syntect_backend_directly() {
  use ndg_commonmark::syntax::{SyntaxHighlighter, SyntectHighlighter};

  let highlighter = SyntectHighlighter::default();

  // Test basic highlighting
  let result =
    highlighter.highlight("fn main() { println!(\"Hello\"); }", "rust", None);

  assert!(result.is_ok());
  let html = result.unwrap();
  assert!(html.contains("main"));
  assert!(html.contains("println"));

  // Test language support
  assert!(highlighter.supports_language("rust"));
  assert!(!highlighter.supported_languages().is_empty());

  // Test theme availability
  let themes = highlighter.available_themes();
  assert!(!themes.is_empty());
}

#[cfg(any(feature = "syntastica", feature = "syntect"))]
#[test]
fn test_syntax_manager_language_aliases() {
  let manager =
    create_default_manager().expect("Failed to create syntax manager");

  // Test language resolution through aliases
  assert_eq!(manager.resolve_language("js"), "javascript");
  assert_eq!(manager.resolve_language("py"), "python");
  assert_eq!(manager.resolve_language("ts"), "typescript");
  assert_eq!(manager.resolve_language("nixos"), "nix");

  // Test non-alias languages pass through
  assert_eq!(manager.resolve_language("rust"), "rust");
  assert_eq!(manager.resolve_language("unknown"), "unknown");
}

#[cfg(any(feature = "syntastica", feature = "syntect"))]
#[test]
fn test_syntax_manager_highlighting_with_aliases() {
  let manager =
    create_default_manager().expect("Failed to create syntax manager");

  // Test highlighting with alias
  let result = manager.highlight_code(
    "console.log('Hello, world!');",
    "js", // alias for javascript
    None,
  );

  if manager.highlighter().supports_language("javascript") {
    assert!(result.is_ok());
    let html = result.unwrap();
    assert!(html.contains("console"));
    assert!(html.contains("log"));
  }
}

#[cfg(any(feature = "syntastica", feature = "syntect"))]
#[test]
fn test_syntax_manager_fallback_behavior() {
  let manager =
    create_default_manager().expect("Failed to create syntax manager");

  // Test fallback for unsupported language
  let result = manager.highlight_code(
    "some random code",
    "totally-unsupported-language",
    None,
  );

  // Should either succeed with fallback or fail gracefully
  match result {
    Ok(html) => {
      assert!(!html.is_empty());
      assert!(html.contains("some random code"));
    },
    Err(_) => {
      // This is acceptable if no fallback is configured
    },
  }
}

#[cfg(any(feature = "syntastica", feature = "syntect"))]
#[test]
fn test_language_detection_from_filename() {
  let manager =
    create_default_manager().expect("Failed to create syntax manager");

  // Test various file extensions
  if let Some(lang) = manager.highlighter().language_from_filename("test.rs") {
    assert_eq!(lang, "rust");
  }

  if let Some(lang) = manager.highlighter().language_from_filename("script.py")
  {
    assert_eq!(lang, "python");
  }

  if let Some(lang) = manager.highlighter().language_from_filename("config.nix")
  {
    assert_eq!(lang, "nix");
  }
}

#[cfg(any(feature = "syntastica", feature = "syntect"))]
#[test]
fn test_theme_handling() {
  let manager =
    create_default_manager().expect("Failed to create syntax manager");

  // Get available themes
  let themes = manager.highlighter().available_themes();
  assert!(!themes.is_empty());

  // Test highlighting with specific theme if available
  if !themes.is_empty() {
    let theme_name = &themes[0];
    let result =
      manager.highlight_code("fn test() {}", "rust", Some(theme_name));

    if manager.highlighter().supports_language("rust") {
      assert!(result.is_ok());
    }
  }
}

#[test]
fn test_complex_code_highlighting() {
  let mut options = MarkdownOptions::default();
  options.highlight_code = true;

  let processor = MarkdownProcessor::new(options);

  let markdown = r#"
# Complex Code Examples

## Rust with Generics

```rust
use std::collections::HashMap;

fn process_data<T: Clone + std::fmt::Debug>(
    data: &[T],
    transform: impl Fn(&T) -> String,
) -> HashMap<String, T> {
    let mut result = HashMap::new();
    for item in data {
        let key = transform(item);
        result.insert(key, item.clone());
    }
    result
}
```

## Nix with Complex Expressions

```nix
{ lib, stdenv, fetchFromGitHub, rustPlatform, pkg-config, openssl }:

rustPlatform.buildRustPackage rec {
  pname = "my-tool";
  version = "1.0.0";

  src = fetchFromGitHub {
    owner = "example";
    repo = pname;
    rev = "v${version}";
    sha256 = lib.fakeSha256;
  };

  cargoSha256 = lib.fakeSha256;

  nativeBuildInputs = [ pkg-config ];
  buildInputs = [ openssl ];

  meta = with lib; {
    description = "A useful tool";
    license = licenses.mit;
    maintainers = with maintainers; [ example ];
  };
}
```

## JavaScript with Modern Features

```javascript
class DataProcessor {
  constructor(options = {}) {
    this.options = { ...this.defaultOptions, ...options };
  }

  async processData(input) {
    try {
      const results = await Promise.all(
        input.map(async (item) => {
          const processed = await this.transformItem(item);
          return { ...item, processed };
        })
      );
      return results.filter(item => item.processed);
    } catch (error) {
      console.error('Processing failed:', error);
      throw error;
    }
  }
}
```
"#;

  let result = processor.render(markdown);

  // Check that the complex code was processed
  assert!(!result.html.is_empty());
  assert!(result.html.contains("process_data"));
  assert!(result.html.contains("rustPlatform"));
  assert!(result.html.contains("DataProcessor"));

  // Check that syntax highlighting is applied (multiple colored spans)
  let span_count = result.html.matches("<span").count();
  assert!(span_count >= 10); // Should have many highlighted spans
  assert!(result.html.contains("color:rgb"));
}

#[test]
fn test_inline_code_not_highlighted() {
  let mut options = MarkdownOptions::default();
  options.highlight_code = true;

  let processor = MarkdownProcessor::new(options);

  let markdown = r#"
Here's some inline `fn main()` code that should not be syntax highlighted.

But this should be:

```rust
fn main() {
    println!("Hello");
}
```
"#;

  let result = processor.render(markdown);

  // Inline code should be in <code> tags but not highlighted
  assert!(result.html.contains("<code>fn main()</code>"));

  // Block code should be highlighted with spans
  assert!(result.html.contains("<span"));
  assert!(result.html.contains("color:rgb"));
}
