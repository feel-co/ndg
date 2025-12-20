//! Syntastica-based syntax highlighting backend.
//!
//! This module provides a modern tree-sitter based syntax highlighter using the
//! Syntastica library, which offers excellent language support including native
//! Nix highlighting.
//!
//! ## Theme Support
//!
//! We programmatically load all available themes from `syntastica-themes`
//! Some of the popular themes included are:
//! - github (dark/light variants)
//! - gruvbox (dark/light)
//! - nord, dracula, catppuccin
//! - tokyo night, solarized, monokai
//! - And many more...

use std::{collections::HashMap, sync::{Arc, Mutex}};

use syntastica::{Processor, render, renderer::HtmlRenderer};
use syntastica_core::theme::ResolvedTheme;
use syntastica_parsers::{Lang, LanguageSetImpl};

use super::{
  error::{SyntaxError, SyntaxResult},
  types::{SyntaxConfig, SyntaxHighlighter, SyntaxManager},
};

/// Syntastica-based syntax highlighter.
pub struct SyntasticaHighlighter {
  #[allow(dead_code, reason = "Must be kept alive as processor holds reference to it")]
  language_set:  Arc<LanguageSetImpl>,
  themes:        HashMap<String, ResolvedTheme>,
  default_theme: ResolvedTheme,
  processor:     Mutex<Processor<'static, LanguageSetImpl>>,
  renderer:      Mutex<HtmlRenderer>,
}

impl SyntasticaHighlighter {
  /// Create a new Syntastica highlighter with all available themes.
  ///
  /// # Errors
  ///
  /// Currently never returns an error, but returns a Result for API
  /// consistency.
  pub fn new() -> SyntaxResult<Self> {
    let language_set = Arc::new(LanguageSetImpl::new());

    let mut themes = HashMap::new();

    // Load all available themes
    for theme_name in syntastica_themes::THEMES {
      if let Some(theme) = syntastica_themes::from_str(theme_name) {
        themes.insert((*theme_name).to_string(), theme);
      }
    }

    let default_theme = syntastica_themes::one::dark();

    // Create processor with a static reference to the language set
    // Safety: The Arc ensures the language set outlives the processor
    let processor = unsafe {
      let language_set_ref: &'static LanguageSetImpl =
        &*std::ptr::from_ref::<LanguageSetImpl>(language_set.as_ref());
      Processor::new(language_set_ref)
    };

    Ok(Self {
      language_set,
      themes,
      default_theme,
      processor: Mutex::new(processor),
      renderer: Mutex::new(HtmlRenderer::new()),
    })
  }

  /// Add a custom theme
  pub fn add_theme(&mut self, name: String, theme: ResolvedTheme) {
    self.themes.insert(name, theme);
  }

  /// Set the default theme
  pub fn set_default_theme(&mut self, theme: ResolvedTheme) {
    self.default_theme = theme;
  }

  /// Convert a language string to a Lang enum
  fn parse_language(language: &str) -> Option<Lang> {
    match language.to_lowercase().as_str() {
      "rust" | "rs" => Some(Lang::Rust),
      "python" | "py" => Some(Lang::Python),
      "javascript" | "js" => Some(Lang::Javascript),
      "typescript" | "ts" => Some(Lang::Typescript),
      "tsx" => Some(Lang::Tsx),
      "nix" => Some(Lang::Nix),
      "bash" | "sh" | "shell" => Some(Lang::Bash),
      "c" => Some(Lang::C),
      "cpp" | "c++" | "cxx" => Some(Lang::Cpp),
      "c_sharp" | "csharp" | "cs" => Some(Lang::CSharp),
      "go" => Some(Lang::Go),
      "java" => Some(Lang::Java),
      "json" => Some(Lang::Json),
      "yaml" | "yml" => Some(Lang::Yaml),
      "html" => Some(Lang::Html),
      "css" => Some(Lang::Css),
      "markdown" | "md" => Some(Lang::Markdown),
      "markdown_inline" => Some(Lang::MarkdownInline),
      "sql" => Some(Lang::Sql),
      "lua" => Some(Lang::Lua),
      "ruby" | "rb" => Some(Lang::Ruby),
      "php" => Some(Lang::Php),
      "php_only" => Some(Lang::PhpOnly),
      "haskell" | "hs" => Some(Lang::Haskell),
      "scala" => Some(Lang::Scala),
      "swift" => Some(Lang::Swift),
      "makefile" | "make" => Some(Lang::Make),
      "cmake" => Some(Lang::Cmake),
      "asm" | "assembly" => Some(Lang::Asm),
      "diff" | "patch" => Some(Lang::Diff),
      "elixir" | "ex" | "exs" => Some(Lang::Elixir),
      "jsdoc" => Some(Lang::Jsdoc),
      "printf" => Some(Lang::Printf),
      "regex" | "regexp" => Some(Lang::Regex),
      "zig" => Some(Lang::Zig),
      #[allow(clippy::match_same_arms, reason = "Explicit for documentation")]
      "text" | "txt" | "plain" => None, // use fallback for plain text
      _ => None,
    }
  }

  /// Get the theme by name, falling back to default
  fn get_theme(&self, theme_name: Option<&str>) -> &ResolvedTheme {
    theme_name
      .and_then(|name| self.themes.get(name))
      .unwrap_or(&self.default_theme)
  }
}

impl SyntaxHighlighter for SyntasticaHighlighter {
  fn name(&self) -> &'static str {
    "Syntastica"
  }

  fn supported_languages(&self) -> Vec<String> {
    vec![
      "rust",
      "rs",
      "python",
      "py",
      "javascript",
      "js",
      "typescript",
      "ts",
      "tsx",
      "nix",
      "bash",
      "sh",
      "shell",
      "c",
      "cpp",
      "c++",
      "cxx",
      "c_sharp",
      "csharp",
      "cs",
      "go",
      "java",
      "json",
      "yaml",
      "yml",
      "html",
      "css",
      "markdown",
      "md",
      "markdown_inline",
      "sql",
      "lua",
      "ruby",
      "rb",
      "php",
      "php_only",
      "haskell",
      "hs",
      "scala",
      "swift",
      "makefile",
      "make",
      "cmake",
      "asm",
      "assembly",
      "diff",
      "patch",
      "elixir",
      "ex",
      "exs",
      "jsdoc",
      "printf",
      "regex",
      "regexp",
      "zig",
      "text",
      "txt",
      "plain",
    ]
    .into_iter()
    .map(String::from)
    .collect()
  }

  fn available_themes(&self) -> Vec<String> {
    let mut themes: Vec<String> = self.themes.keys().cloned().collect();
    themes.sort();
    themes
  }

  fn highlight(
    &self,
    code: &str,
    language: &str,
    theme: Option<&str>,
  ) -> SyntaxResult<String> {
    let lang = Self::parse_language(language)
      .ok_or_else(|| SyntaxError::UnsupportedLanguage(language.to_string()))?;

    let theme = self.get_theme(theme);

    // Use the reusable processor via Mutex for thread-safe interior mutability
    let highlights = self
      .processor
      .lock()
      .map_err(|e| SyntaxError::HighlightingFailed(format!("Processor lock poisoned: {e}")))?
      .process(code, lang)
      .map_err(|e| SyntaxError::HighlightingFailed(e.to_string()))?;

    // Use the reusable renderer via Mutex for thread-safe interior mutability
    let html = {
      let mut renderer = self
        .renderer
        .lock()
        .map_err(|e| SyntaxError::HighlightingFailed(format!("Renderer lock poisoned: {e}")))?;
      render(&highlights, &mut *renderer, theme)
    };

    Ok(html)
  }

  fn language_from_extension(&self, extension: &str) -> Option<String> {
    match extension.to_lowercase().as_str() {
      "rs" => Some("rust".to_string()),
      "py" | "pyw" => Some("python".to_string()),
      "js" | "mjs" => Some("javascript".to_string()),
      "ts" => Some("typescript".to_string()),
      "tsx" => Some("tsx".to_string()),
      "nix" => Some("nix".to_string()),
      "sh" | "bash" | "zsh" | "fish" => Some("bash".to_string()),
      "c" | "h" => Some("c".to_string()),
      "cpp" | "cxx" | "cc" | "hpp" | "hxx" | "hh" => Some("cpp".to_string()),
      "cs" => Some("c_sharp".to_string()),
      "go" => Some("go".to_string()),
      "java" => Some("java".to_string()),
      "json" => Some("json".to_string()),
      "yaml" | "yml" => Some("yaml".to_string()),
      "html" | "htm" => Some("html".to_string()),
      "css" => Some("css".to_string()),
      "md" | "markdown" => Some("markdown".to_string()),
      "sql" => Some("sql".to_string()),
      "lua" => Some("lua".to_string()),
      "rb" => Some("ruby".to_string()),
      "php" => Some("php".to_string()),
      "hs" => Some("haskell".to_string()),
      "ml" | "mli" => Some("ocaml".to_string()),
      "scala" => Some("scala".to_string()),
      "swift" => Some("swift".to_string()),
      "s" | "asm" => Some("asm".to_string()),
      "diff" | "patch" => Some("diff".to_string()),
      "ex" | "exs" => Some("elixir".to_string()),
      "zig" => Some("zig".to_string()),
      "txt" => Some("text".to_string()),
      _ => None,
    }
  }
}

/// Create a Syntastica-based syntax manager with default configuration.
///
/// Syntastica provides modern tree-sitter based syntax highlighting with
/// excellent language support including native Nix highlighting.
///
/// # Errors
///
/// Returns an error if the Syntastica highlighter fails to initialize.
pub fn create_syntastica_manager() -> SyntaxResult<SyntaxManager> {
  let highlighter = Box::new(SyntasticaHighlighter::new()?);
  let config = SyntaxConfig {
    default_theme: Some("one-dark".to_string()),
    ..Default::default()
  };
  Ok(SyntaxManager::new(highlighter, config))
}
