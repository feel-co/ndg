use ndg_commonmark::{MarkdownOptions, MarkdownProcessor, syntax::create_default_manager};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== NDG CommonMark Syntax Highlighting ===\n");

    // 1. Basic usage with MarkdownProcessor
    demo_markdown_processor()?;

    // 2. Direct syntax manager usage
    demo_syntax_manager()?;

    // 3: Multiple language support
    demo_multiple_languages()?;

    // 4: Theme showcase
    demo_themes()?;

    // 5. Language detection
    demo_language_detection()?;

    Ok(())
}

fn demo_markdown_processor() -> Result<(), Box<dyn std::error::Error>> {
    println!("1. Markdown Processor with Syntax Highlighting\n");

    let mut options = MarkdownOptions::default();
    options.highlight_code = true;
    options.highlight_theme = Some("one-dark".to_string());

    let processor = MarkdownProcessor::new(options);

    let markdown = r#"
# Code Examples

Here's some Rust code:

```rust
use std::collections::HashMap;

fn fibonacci(n: u32) -> u32 {
    match n {
        0 => 0,
        1 => 1,
        _ => fibonacci(n - 1) + fibonacci(n - 2),
    }
}

fn main() {
    let mut cache = HashMap::new();
    for i in 0..10 {
        cache.insert(i, fibonacci(i));
    }
    println!("Fibonacci cache: {:?}", cache);
}
```

And some Nix configuration:

```nix
{ config, lib, pkgs, ... }:

with lib;

{
  options.myService = {
    enable = mkEnableOption "my custom service";

    package = mkOption {
      type = types.package;
      default = pkgs.hello;
      description = "Package to use for the service";
    };
  };

  config = mkIf config.myService.enable {
    systemd.services.my-service = {
      description = "My Custom Service";
      wantedBy = [ "multi-user.target" ];
      serviceConfig = {
        ExecStart = "${config.myService.package}/bin/hello";
        Restart = "always";
      };
    };
  };
}
```
"#;

    let result = processor.render(markdown);

    println!("Generated HTML (excerpt):");
    let html = &result.html;
    if html.len() > 500 {
        println!("{}...\n", &html[..500]);
    } else {
        println!("{}\n", html);
    }

    println!("Title: {:?}", result.title);
    println!("Headers: {:?}\n", result.headers);

    Ok(())
}

fn demo_syntax_manager() -> Result<(), Box<dyn std::error::Error>> {
    println!("2. Direct Syntax Manager Usage\n");

    let manager = create_default_manager()?;

    println!("Backend: {}", manager.highlighter().name());
    println!(
        "Supported languages: {}",
        manager.highlighter().supported_languages().len()
    );
    println!(
        "Available themes: {}",
        manager.highlighter().available_themes().len()
    );

    // Highlight some code directly
    let rust_code = r#"
#[derive(Debug, Clone)]
pub struct Config {
    pub name: String,
    pub version: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            name: "ndg".to_string(),
            version: "1.0.0".to_string(),
        }
    }
}
"#;

    let highlighted = manager.highlight_code(rust_code, "rust", Some("one-dark"))?;
    println!("Highlighted Rust code (excerpt):");
    if highlighted.len() > 200 {
        println!("{}...\n", &highlighted[..200]);
    } else {
        println!("{}\n", highlighted);
    }

    Ok(())
}

fn demo_multiple_languages() -> Result<(), Box<dyn std::error::Error>> {
    println!("3. Multiple Language Support\n");

    let manager = create_default_manager()?;

    let examples = vec![
        (
            "JavaScript",
            "js",
            "const greeting = (name) => `Hello, ${name}!`;",
        ),
        (
            "Python",
            "python",
            "def fibonacci(n):\n    return n if n <= 1 else fibonacci(n-1) + fibonacci(n-2)",
        ),
        (
            "Nix",
            "nix",
            "{ pkgs }: pkgs.writeText \"hello\" \"Hello, world!\"",
        ),
        (
            "Bash",
            "bash",
            "#!/bin/bash\necho \"Processing files...\"\nfind . -name \"*.rs\" | wc -l",
        ),
    ];

    for (lang_name, lang_code, code) in examples {
        println!("--- {} ---", lang_name);

        match manager.highlight_code(code, lang_code, None) {
            Ok(highlighted) => {
                println!("✓ Successfully highlighted {} code", lang_name);
                if highlighted.len() > 100 {
                    println!("Preview: {}...", &highlighted[..100]);
                } else {
                    println!("Full output: {}", highlighted);
                }
            }
            Err(e) => {
                println!("✗ Failed to highlight {}: {}", lang_name, e);
            }
        }
        println!();
    }

    Ok(())
}

fn demo_themes() -> Result<(), Box<dyn std::error::Error>> {
    println!("4. Theme Showcase\n");

    let manager = create_default_manager()?;
    let themes = manager.highlighter().available_themes();

    println!("Available themes: {:?}", themes);

    let sample_code = "fn main() { println!(\"Hello, themes!\"); }";

    for theme in themes.iter().take(3) {
        // Show first 3 themes
        println!("--- Theme: {} ---", theme);
        match manager.highlight_code(sample_code, "rust", Some(theme)) {
            Ok(highlighted) => {
                if highlighted.len() > 150 {
                    println!("{}...", &highlighted[..150]);
                } else {
                    println!("{}", highlighted);
                }
            }
            Err(e) => {
                println!("Error with theme {}: {}", theme, e);
            }
        }
        println!();
    }

    Ok(())
}

fn demo_language_detection() -> Result<(), Box<dyn std::error::Error>> {
    println!("5. Language Detection from Filenames\n");

    let manager = create_default_manager()?;

    let test_files = vec![
        "main.rs",
        "script.py",
        "config.nix",
        "build.sh",
        "package.json",
        "README.md",
        "style.css",
        "index.html",
        "unknown.xyz",
    ];

    for filename in test_files {
        if let Some(detected_lang) = manager.highlighter().language_from_filename(filename) {
            println!("✓ {}: detected as '{}'", filename, detected_lang);
        } else {
            println!("✗ {}: no language detected", filename);
        }
    }

    println!();

    // Test alias resolution
    println!("Language alias resolution:");
    let aliases = vec!["js", "py", "ts", "sh", "yml"];
    for alias in aliases {
        let resolved = manager.resolve_language(alias);
        println!("  {} -> {}", alias, resolved);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_demo_functions() {
        // Test that demo functions don't panic
        let _ = demo_syntax_manager();
        let _ = demo_multiple_languages();
        let _ = demo_language_detection();
    }
}
