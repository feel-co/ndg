//! Syntastica-based syntax highlighting backend.
//!
//! This module provides a modern tree-sitter based syntax highlighter using the
//! Syntastica library, which offers excellent language support including native
//! Nix highlighting.
//!
//! ## Theme Support
//!
//! We programmaticall loads all available themes from `syntastica-themes`
//! Some of the popular themes included are:
//! - github (dark/light variants)
//! - gruvbox (dark/light)
//! - nord, dracula, catppuccin
//! - tokyo night, solarized, monokai
//! - And many more...

use std::{collections::HashMap, sync::Arc};

use syntastica::{Processor, render, renderer::HtmlRenderer};
use syntastica_core::theme::ResolvedTheme;
use syntastica_parsers::{Lang, LanguageSetImpl};

use super::{
  error::{SyntaxError, SyntaxResult},
  types::{SyntaxConfig, SyntaxHighlighter, SyntaxManager},
};

/// Syntastica-based syntax highlighter.
pub struct SyntasticaHighlighter {
  language_set:  Arc<LanguageSetImpl>,
  themes:        HashMap<String, ResolvedTheme>,
  default_theme: ResolvedTheme,
}

impl SyntasticaHighlighter {
  /// Create a new Syntastica highlighter with all available themes.
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

    Ok(Self {
      language_set,
      themes,
      default_theme,
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
  fn parse_language(&self, language: &str) -> Option<Lang> {
    match language.to_lowercase().as_str() {
      "rust" | "rs" => Some(Lang::Rust),
      "python" | "py" => Some(Lang::Python),
      "javascript" | "js" => Some(Lang::Javascript),
      "typescript" | "ts" => Some(Lang::Typescript),
      "nix" => Some(Lang::Nix),
      "bash" | "sh" | "shell" => Some(Lang::Bash),
      "c" => Some(Lang::C),
      "cpp" | "c++" | "cxx" => Some(Lang::Cpp),
      "go" => Some(Lang::Go),
      "java" => Some(Lang::Java),
      "json" => Some(Lang::Json),
      "yaml" | "yml" => Some(Lang::Yaml),
      "html" => Some(Lang::Html),
      "css" => Some(Lang::Css),
      "markdown" | "md" => Some(Lang::Markdown),
      "sql" => Some(Lang::Sql),
      "lua" => Some(Lang::Lua),
      "ruby" | "rb" => Some(Lang::Ruby),
      "php" => Some(Lang::Php),
      "haskell" | "hs" => Some(Lang::Haskell),
      "ocaml" | "ml" => Some(Lang::Ocaml),
      "scala" => Some(Lang::Scala),
      "swift" => Some(Lang::Swift),
      "makefile" | "make" => Some(Lang::Make),
      "cmake" => Some(Lang::Cmake),
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

impl Default for SyntasticaHighlighter {
  fn default() -> Self {
    Self::new().expect("Failed to create Syntastica highlighter")
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
      "nix",
      "bash",
      "sh",
      "shell",
      "c",
      "cpp",
      "c++",
      "cxx",
      "go",
      "java",
      "json",
      "yaml",
      "yml",
      "html",
      "css",
      "markdown",
      "md",
      "sql",
      "lua",
      "ruby",
      "rb",
      "php",
      "haskell",
      "hs",
      "ocaml",
      "ml",
      "scala",
      "swift",
      "makefile",
      "make",
      "cmake",
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
    let lang = self
      .parse_language(language)
      .ok_or_else(|| SyntaxError::UnsupportedLanguage(language.to_string()))?;

    let theme = self.get_theme(theme);

    // Create a processor for this highlighting operation
    let mut processor = Processor::new(self.language_set.as_ref());

    // Process the code to get highlights
    let highlights = processor
      .process(code, lang)
      .map_err(|e| SyntaxError::HighlightingFailed(e.to_string()))?;

    // Render to HTML
    let mut renderer = HtmlRenderer::new();
    let html = render(&highlights, &mut renderer, theme.clone());

    Ok(html)
  }

  fn language_from_extension(&self, extension: &str) -> Option<String> {
    match extension.to_lowercase().as_str() {
      "rs" => Some("rust".to_string()),
      "py" | "pyw" => Some("python".to_string()),
      "js" | "mjs" => Some("javascript".to_string()),
      "ts" => Some("typescript".to_string()),
      "nix" => Some("nix".to_string()),
      "sh" | "bash" | "zsh" | "fish" => Some("bash".to_string()),
      "c" | "h" => Some("c".to_string()),
      "cpp" | "cxx" | "cc" | "hpp" | "hxx" | "hh" => Some("cpp".to_string()),
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
      "txt" => Some("text".to_string()),
      _ => None,
    }
  }
}

/// Create a Syntastica-based syntax manager with default configuration.
///
/// Syntastica provides modern tree-sitter based syntax highlighting with
/// excellent language support including native Nix highlighting.
pub fn create_syntastica_manager() -> SyntaxResult<SyntaxManager> {
  let highlighter = Box::new(SyntasticaHighlighter::new()?);
  let config = SyntaxConfig {
    default_theme: Some("one-dark".to_string()),
    ..Default::default()
  };
  Ok(SyntaxManager::new(highlighter, config))
}
