# Syntax Highlighting in ndg-commonmark

The syntax highlighting system in ndg-commonmark is designed with extensibility
in mind, allowing different highlighting backends to be plugged in through a
trait-based architecture. This enables users to choose the best highlighting
engine for their needs while maintaining a consistent API.

> [!TIP]
> This document _highlights_[^1] using ndg-commonmark's syntax highlighting
> system for developers who intend to integrate ndg-commonmark into their
> project. If you are working with ndg, you may be more interested in its
> documentation and the [syntax reference](./SYNTAX.md).

[^1]: Ha, get it? Highlights. Ha.

## Quick Start

### Basic Usage with MarkdownProcessor

````rust
use ndg_commonmark::{MarkdownOptions, MarkdownProcessor};

let mut options = MarkdownOptions::default();
options.highlight_code = true;
options.highlight_theme = Some("one-dark".to_string());

let processor = MarkdownProcessor::new(options);

let markdown = r#"
# Example

```rust
fn main() {
    println!("Hello, world!");
}
```

"#;

let result = processor.render(markdown); println!("{}", result.html);
````

### Direct Syntax Manager Usage

```rust
use ndg_commonmark::syntax::create_default_manager;

let manager = create_default_manager()?;

let highlighted = manager.highlight_code(
    "fn main() { println!(\"Hello\"); }",
    "rust",
    Some("one-dark"),
)?;

println!("{}", highlighted);
```

## Configuration

### Feature Flags

The syntax highlighting system is controlled by feature flags:

- `syntastica`: Enables Syntastica backend (enabled by `ndg-flavored`)
- `ndg-flavored`: Enables all NDG-specific features including syntax
  highlighting
- `syntect`: Enables the legacy highlighting system. More lightweight, but lacks
  language support.

```toml
[dependencies]
ndg-commonmark = { version = "1.0", features = ["syntastica"] }
```

### MarkdownOptions

Configure syntax highlighting through `MarkdownOptions`:

```rust
let mut options = MarkdownOptions::default();

// Enable/disable syntax highlighting
options.highlight_code = true;

// Set default theme
options.highlight_theme = Some("gruvbox-dark".to_string());
```

### SyntaxConfig

For advanced configuration, use `SyntaxConfig`:

```rust
use ndg_commonmark::syntax::{SyntaxConfig, SyntaxManager};
use std::collections::HashMap;

let mut config = SyntaxConfig::default();

// Set default theme
config.default_theme = Some("one-dark".to_string());

// Add custom language aliases
config.language_aliases.insert("ts".to_string(), "typescript".to_string());

// Configure fallback behavior
config.fallback_to_plain = true;

let manager = SyntaxManager::new(highlighter, config);
```

## Supported Languages

The Syntastica crate supports a large variety of languages. We enable support
for _all_ languages by default.

### Language Aliases

Common aliases are automatically resolved:

| Alias   | Resolves To  |
| ------- | ------------ |
| `js`    | `javascript` |
| `ts`    | `typescript` |
| `py`    | `python`     |
| `rb`    | `ruby`       |
| `sh`    | `bash`       |
| `shell` | `bash`       |
| `yml`   | `yaml`       |
| `nixos` | `nix`        |
| `md`    | `markdown`   |

### File Extension Detection

Languages can be automatically detected from file extensions:

```rust
let manager = create_default_manager()?;

if let Some(lang) = manager.highlighter().language_from_filename("main.rs") {
    println!("Detected language: {}", lang); // "rust"
}
```

## Themes

The number and the names of supported themes depend on the backend crate used.

### Syntastica Backend

- `one-dark` (default)
- `one-light`
- `gruvbox-dark`
- `gruvbox-light`
- `github`

### Syntect Backend

- `InspiredGitHub` (default)
- Plus all themes from syntect's default theme set

### Using Themes

```rust
// With MarkdownProcessor
let mut options = MarkdownOptions::default();
options.highlight_theme = Some("gruvbox-dark".to_string());

// With SyntaxManager
let highlighted = manager.highlight_code(code, "rust", Some("one-dark"))?;
```

### Custom Themes

#### For Syntastica

```rust
use syntastica::theme;
use ndg_commonmark::syntax::SyntasticaHighlighter;

let theme = theme! {
    "purple": "#c678dd",
    "blue": "#61afef",
    "keyword": "$purple",
    "function": "$blue",
};

let mut highlighter = SyntasticaHighlighter::new()?;
highlighter.add_theme("my-theme".to_string(), theme);
```

## Backends

### Backend Selection

The backend is automatically selected based on feature flags:

1. If `syntastica` feature is enabled, then the Syntastica crate will be used.
2. If `syntect` feature is enabled, then Syntect will be used for syntax
   highlighting.
3. If neither are enabled, then ndg-commonmark will not do _any_ syntax
   highlighting at all.

Manual backend selection:

```rust
use ndg_commonmark::syntax::{SyntasticaHighlighter, SyntaxManager, SyntaxConfig};

// Explicitly use Syntastica
#[cfg(feature = "syntastica")]
let highlighter = Box::new(SyntasticaHighlighter::new()?);

// Explicitly use Syntect
let highlighter = Box::new(SyntectHighlighter::default());

let manager = SyntaxManager::with_highlighter(highlighter);
```

## Error Handling

Example:

```rust
use ndg_commonmark::syntax::SyntaxError;

match manager.highlight_code(code, "unknown-lang", None) {
    Ok(html) => println!("Success: {}", html),
    Err(SyntaxError::UnsupportedLanguage(lang)) => {
        eprintln!("Language '{}' not supported", lang);
    }
    Err(SyntaxError::ThemeNotFound(theme)) => {
        eprintln!("Theme '{}' not found", theme);
    }
    Err(SyntaxError::HighlightingFailed(msg)) => {
        eprintln!("Highlighting failed: {}", msg);
    }
    Err(e) => eprintln!("Other error: {}", e),
}
```

## Performance Considerations

ndg-commonmark is fast, very fast. Though you will still want to keep somethings
in mind to retain the performance benefits of ndg-commonmark. Here are some tips
that you might find useful.

1. Create syntax managers once and reuse them compatibility
2. Limit available themes as theme changes can be expensive
3. Avoid manual language specification when possible

## Advanced Usage

### Custom Highlighter Implementation

Implement the `SyntaxHighlighter` trait for custom backends:

```rust
use ndg_commonmark::syntax::{SyntaxHighlighter, SyntaxResult, SyntaxError};

struct CustomHighlighter {
    // Your implementation
}

impl SyntaxHighlighter for CustomHighlighter {
    fn name(&self) -> &'static str {
        "Custom"
    }

    fn supported_languages(&self) -> Vec<String> {
        vec!["custom".to_string()]
    }

    fn available_themes(&self) -> Vec<String> {
        vec!["default".to_string()]
    }

    fn highlight(&self, code: &str, language: &str, theme: Option<&str>) -> SyntaxResult<String> {
        // Your highlighting logic
        Ok(format!("<pre><code>{}</code></pre>", html_escape::encode_text(code)))
    }

    fn language_from_extension(&self, extension: &str) -> Option<String> {
        match extension {
            "custom" => Some("custom".to_string()),
            _ => None,
        }
    }
}
```
