# ndg-commonmark

High-performance, extensible Nixpkgs-flavored CommonMark processor designed
specifically for [ndg](../ndg) in order to document Nix, NixOS and Nixpkgs
adjacent projects.

Features AST-based processing, syntax highlighting, and generous documentation.

## Features

### Core Functionality

- **CommonMark Compliance**: Full CommonMark specification support
- **GitHub Flavored Markdown (GFM)**: Tables, strikethrough, task lists, and
  more via Comrak
- **Extensible Syntax Highlighting**: Modern tree-sitter based highlighting with
  Syntastica
- **Memory Safe**: Built in Rust with zero-cost abstractions
- **High Performance**: AST-based processing for optimal speed

### Nix Ecosystem Support

- **Nix Language Highlighting**: First-class support for Nix syntax via
  Syntastica [^1]
- **Role Markup**: MySt-like roles such as:
  - {command}`ls -la`,
  - {file}`/etc/nixos/configuration.nix`,
  - {var}`$XDG_CONFIG_HOME`
  - {man}`nix.conf`
- **Option References**: Automatic linking to NixOS option documentation
- **File Includes**: Process `{=include=}` blocks for modular documentation
- **Manpage Integration**: Automatic linking to system manuals using a `{man}`
  role and a manpage map.

[^1]: Nix syntax highlighting is available only if using Syntastica as the
    highlighting backend. Syntect requires an external parser collection, which
    we no longer support.

### Other Markup Features

- **Inline Anchors**: `[]{#custom-id}` syntax for precise linking
- **Admonitions**: GitHub-style callouts and custom admonition blocks
- **Figure Support**: Structured figure blocks with captions
- **Definition Lists**: Technical documentation support
- **Auto-linking**: Automatic URL detection and conversion

## Quick Start

### Basic Usage

```rust
use ndg_commonmark::{MarkdownProcessor, MarkdownOptions};

let processor = MarkdownProcessor::new(MarkdownOptions::default());
let result = processor.render("# Hello World\n\nThis is **bold** text.");

// The entire document
println!("HTML: {}", result.html);

// Fine-grained components
println!("Title: {:?}", result.title);
println!("Headers: {:?}", result.headers);
```

### With Syntax Highlighting

````rust
use ndg_commonmark::{MarkdownProcessor, MarkdownOptions};

let mut options = MarkdownOptions::default();
options.highlight_code = true;
options.highlight_theme = Some("one-dark".to_string());

let processor = MarkdownProcessor::new(options);

let markdown = r#"
# Code Example

```rust
fn main() {
    println!("Hello, world!");
}
```

```nix
{ pkgs, ... }: {
  environment.systemPackages = with pkgs; [ vim git ];
}
```

"#;

let result = processor.render(markdown);
````

### Feature Configuration

```rust
use ndg_commonmark::MarkdownOptions;

let mut options = MarkdownOptions::default();

// Enable GitHub Flavored Markdown (GFM)
options.gfm = true;

// Enable Nixpkgs-specific extensions
options.nixpkgs = true;

// Configure syntax highlighting
options.highlight_code = true;
options.highlight_theme = Some("gruvbox-dark".to_string());

// Set manpage URL mappings
// Note: this is required for {man} role to function correctly
options.manpage_urls_path = Some("manpage-urls.json".to_string());

let processor = MarkdownProcessor::new(options);
```

### Cargo Feature Flags

Some of ndg-commonmark's features are gated behind feature flags. This allows
compile-time control over what will be made available. Namely you can decide
which parts of the flavored CommonMark will be supported.

- `default`: Enables `ndg-flavored`
- `ndg-flavored`: All NDG-specific features (`gfm` + `nixpkgs` + `syntastica`)
- `gfm`: GitHub Flavored Markdown extensions
- `nixpkgs`: NixOS/Nixpkgs documentation features
- `syntastica`: Modern tree-sitter based syntax highlighting (recommended)
- `syntect`: Legacy, less robust and more lightweight syntax highlighting

> [!NOTE]
> The `syntastica` and `syntect` features are mutually exclusive. Enable only
> one at a time.

```toml
[dependencies]
# Recommended: Modern syntax highlighting
ndg-commonmark = { version = "1.0", features = ["gfm", "nixpkgs", "syntastica"] }

# Alternative: Legacy syntax highlighting
ndg-commonmark = { version = "1.0", features = ["gfm", "nixpkgs", "syntect"] }

# No syntax highlighting
ndg-commonmark = { version = "1.0", features = ["gfm", "nixpkgs"] }
```

## API Reference

### `MarkdownProcessor`

The main processor class:

```rust
impl MarkdownProcessor {
    pub fn new(options: MarkdownOptions) -> Self;
    pub fn render(&self, markdown: &str) -> MarkdownResult;
    pub fn extract_headers(&self, content: &str) -> (Vec<Header>, Option<String>);
}
```

### `MarkdownOptions`

Configuration options:

```rust
pub struct MarkdownOptions {
    pub gfm: bool,                              // GitHub Flavored Markdown
    pub nixpkgs: bool,                          // NixOS documentation features
    pub highlight_code: bool,                   // Syntax highlighting
    pub manpage_urls_path: Option<String>,      // Manpage URL mappings
    pub highlight_theme: Option<String>,        // Syntax highlighting theme
}
```

### `MarkdownResult`

Processing result:

```rust
pub struct MarkdownResult {
    pub html: String,                           // Generated HTML
    pub headers: Vec<Header>,                   // Extracted headers
    pub title: Option<String>,                  // Document title (first h1)
}
```

### Syntax Highlighting API

For advanced syntax highlighting control:

```rust
use ndg_commonmark::syntax::{create_default_manager, SyntaxManager};

let manager = create_default_manager()?;

// Highlight code directly
let html = manager.highlight_code("fn main() {}", "rust", Some("one-dark"))?;

// Language detection
if let Some(lang) = manager.highlighter().language_from_filename("script.py") {
    println!("Detected: {}", lang);
}

// Available languages and themes
println!("Languages: {:?}", manager.highlighter().supported_languages());
println!("Themes: {:?}", manager.highlighter().available_themes());
```

## Examples

### Basic Document Processing

````rust
use ndg_commonmark::{MarkdownProcessor, MarkdownOptions};

let processor = MarkdownProcessor::new(MarkdownOptions::default());

let markdown = r#"
# Documentation Example

This document demonstrates **ndg-commonmark** features.

## Code Highlighting

```rust
use std::collections::HashMap;

fn main() {
    let mut map = HashMap::new();
    map.insert("key", "value");
    println!("{:?}", map);
}
```

## NixOS Configuration

```nix
{ config, pkgs, ... }: {
  services.nginx = {
    enable = true;
    virtualHosts."example.com" = {
      enableACME = true;
      forceSSL = true;
      root = "/var/www/example.com";
    };
  };
}
```
````

### Advanced Configuration

```rust
use ndg_commonmark::{MarkdownProcessor, MarkdownOptions};
use std::collections::HashMap;

let mut options = MarkdownOptions::default();
options.gfm = true;
options.nixpkgs = true;
options.highlight_code = true;
options.highlight_theme = Some("gruvbox-dark".to_string());
options.manpage_urls_path = Some("manpages.json".to_string());

let processor = MarkdownProcessor::new(options);

// Process multiple files
let files = ["intro.md", "configuration.md", "examples.md"];
for file in files {
    let content = std::fs::read_to_string(file)?;
    let result = processor.render(&content);
    
    println!("Title: {:?}", result.title);
    println!("Headers: {:?}", result.headers);
    
    // Write HTML output
    let output_file = file.replace(".md", ".html");
    std::fs::write(output_file, result.html)?;
}
```

## Integration

### Static Site Generators

```rust
use ndg_commonmark::{MarkdownProcessor, MarkdownOptions};
use std::path::Path;
use walkdir::WalkDir;

fn build_site(input_dir: &Path, output_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut options = MarkdownOptions::default();
    options.gfm = true;
    options.nixpkgs = true;
    options.highlight_code = true;

    let processor = MarkdownProcessor::new(options);

    for entry in WalkDir::new(input_dir) {
        let entry = entry?;
        if entry.path().extension() == Some("md".as_ref()) {
            let content = std::fs::read_to_string(entry.path())?;
            let result = processor.render(&content);
            
            let output_path = output_dir.join(
                entry.path()
                    .strip_prefix(input_dir)?
                    .with_extension("html")
            );
            
            std::fs::create_dir_all(output_path.parent().unwrap())?;
            std::fs::write(output_path, result.html)?;
        }
    }
    
    Ok(())
}
```

## License

This project is licensed under the Mozilla Public License 2.0. See the
[LICENSE](../LICENSE) file for more details.

## Acknowledgments

- [CommonMark](https://commonmark.org/) specification
- [comrak](https://github.com/kivikakk/comrak) for CommonMark parsing
- [Syntastica](https://github.com/RubixDev/syntastica) for modern syntax
  highlighting
- [tree-sitter](https://tree-sitter.github.io/) for parsing technology
- The Nix community for inspiration and requirements
