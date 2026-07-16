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
//!
//! - github (dark/light variants)
//! - gruvbox (dark/light)
//! - nord, dracula, catppuccin
//! - tokyo night, solarized, monokai
//! - And many more...

use std::{
  fs,
  path::{Path, PathBuf},
  sync::Mutex,
};

use rustc_hash::FxHashMap;
use syntastica::{
  Processor,
  language_set::{HighlightConfiguration, LanguageSet},
  render,
  renderer::HtmlRenderer,
};
use syntastica_core::theme::ResolvedTheme;
use syntastica_parsers::Lang;
use syntastica_query_preprocessor::{
  process_highlights,
  process_injections,
  process_locals,
};

use super::{
  error::{SyntaxError, SyntaxResult},
  types::{SyntaxConfig, SyntaxHighlighter, SyntaxManager},
};

/// Syntastica-based syntax highlighter.
pub struct SyntasticaHighlighter {
  themes:        FxHashMap<String, ResolvedTheme>,
  default_theme: ResolvedTheme,
  language_set:  &'static UserQueryLanguageSet,
  workers:       Mutex<Vec<HighlightWorker>>,
}

struct HighlightWorker {
  processor: Processor<'static, UserQueryLanguageSet>,
  renderer:  HtmlRenderer,
}

struct UserQueryLanguageSet {
  configs:            Mutex<FxHashMap<Lang, &'static HighlightConfiguration>>,
  syntax_queries_dir: Option<PathBuf>,
}

impl UserQueryLanguageSet {
  fn new(syntax_queries_dir: Option<&Path>) -> Self {
    Self {
      configs:            Mutex::new(FxHashMap::default()),
      syntax_queries_dir: syntax_queries_dir.map(Path::to_path_buf),
    }
  }

  fn config_for(
    &self,
    lang: Lang,
  ) -> syntastica::Result<&'static HighlightConfiguration> {
    {
      let configs = self.configs.lock().map_err(|e| {
        syntastica::Error::UnsupportedLanguage(format!(
          "syntax language-set lock poisoned: {e}"
        ))
      })?;

      if let Some(config) = configs.get(&lang).copied() {
        return Ok(config);
      }
    }

    let config =
      build_highlight_config(lang, self.syntax_queries_dir.as_deref())
        .map_err(|e| syntastica::Error::UnsupportedLanguage(e.to_string()))?;

    let mut configs = self.configs.lock().map_err(|e| {
      syntastica::Error::UnsupportedLanguage(format!(
        "syntax language-set lock poisoned: {e}"
      ))
    })?;

    let config = configs.get(&lang).copied().unwrap_or_else(|| {
      let config: &'static HighlightConfiguration = Box::leak(Box::new(config));
      configs.insert(lang, config);
      config
    });
    drop(configs);

    Ok(config)
  }
}

fn build_highlight_config(
  lang: Lang,
  syntax_queries_dir: Option<&Path>,
) -> SyntaxResult<HighlightConfiguration> {
  let mut highlights_query = lang.highlights_query().to_string();
  let mut injections_query = lang.injections_query().to_string();
  let mut locals_query = lang.locals_query().to_string();

  if let Some(base_dir) = syntax_queries_dir {
    if let Some(query) = read_user_query(base_dir, lang, "highlights.scm")? {
      let extends = is_extends_query(&query);
      let processed =
        process_highlights("", true, &rewrite_any_of_predicates(&query));
      if extends {
        highlights_query = format!("{highlights_query}\n{processed}");
      } else {
        highlights_query = processed;
      }
    }

    if let Some(query) = read_user_query(base_dir, lang, "injections.scm")? {
      let extends = is_extends_query(&query);
      let processed =
        process_injections("", true, &rewrite_any_of_predicates(&query));
      if extends {
        injections_query = format!("{injections_query}\n{processed}");
      } else {
        injections_query = processed;
      }
    }

    if let Some(query) = read_user_query(base_dir, lang, "locals.scm")? {
      let extends = is_extends_query(&query);
      let processed =
        process_locals("", true, &rewrite_any_of_predicates(&query));
      if extends {
        locals_query = format!("{locals_query}\n{processed}");
      } else {
        locals_query = processed;
      }
    }
  }

  let mut config = HighlightConfiguration::new(
    lang.get(),
    <&str>::from(lang),
    &highlights_query,
    &injections_query,
    &locals_query,
  )
  .map_err(|e| {
    SyntaxError::BackendError(format!(
      "failed to build highlight config for '{}': {e}",
      <&str>::from(lang)
    ))
  })?;
  config.configure(syntastica::theme::THEME_KEYS);
  Ok(config)
}

impl LanguageSet<'_> for UserQueryLanguageSet {
  type Language = Lang;

  fn get_language(
    &self,
    language: Self::Language,
  ) -> syntastica::Result<&HighlightConfiguration> {
    self.config_for(language)
  }
}

fn is_extends_query(content: &str) -> bool {
  content
    .lines()
    .next()
    .is_some_and(|l| matches!(l.trim(), ";; extends" | ";;extends"))
}

/// Rewrites `(#any-of? @cap "a" "b" ...)` into `(#match? @cap "^(a|b|...)$")`.
///
/// nvim-treesitter's `#any-of?` is a Lua-backed predicate with no standard
/// tree-sitter equivalent. The rewrite preserves the same semantics using the
/// `#match?` predicate that tree-sitter-highlight natively supports.
fn rewrite_any_of_predicates(query: &str) -> String {
  const NEEDLE: &str = "#any-of?";
  let mut result = String::with_capacity(query.len());
  let mut remaining = query;

  loop {
    match remaining.find(NEEDLE) {
      None => {
        result.push_str(remaining);
        break;
      },
      Some(pos) => {
        result.push_str(&remaining[..pos]);
        let from = &remaining[pos..];
        if let Some((replacement, consumed)) = parse_any_of_predicate(from) {
          result.push_str(&replacement);
          remaining = &from[consumed..];
        } else {
          result.push_str(NEEDLE);
          remaining = &from[NEEDLE.len()..];
        }
      },
    }
  }

  result
}

fn parse_any_of_predicate(s: &str) -> Option<(String, usize)> {
  const NEEDLE: &str = "#any-of?";
  let mut pos = NEEDLE.len();

  let skip_ws = |p: usize| p + s[p..].len() - s[p..].trim_start().len();

  pos = skip_ws(pos);

  if !s[pos..].starts_with('@') {
    return None;
  }

  let cap_start = pos;
  pos += 1;
  while pos < s.len() {
    let b = s.as_bytes()[pos];
    if b.is_ascii_whitespace() || b == b')' {
      break;
    }
    pos += 1;
  }
  let capture_name = &s[cap_start..pos];

  pos = skip_ws(pos);

  let mut values: Vec<&str> = Vec::new();
  while pos < s.len() && s.as_bytes()[pos] == b'"' {
    pos += 1;
    let val_start = pos;
    while pos < s.len() && s.as_bytes()[pos] != b'"' {
      if s.as_bytes()[pos] == b'\\' {
        pos += 1;
      }
      pos += 1;
    }
    if pos >= s.len() {
      return None;
    }
    values.push(&s[val_start..pos]);
    pos += 1;
    pos = skip_ws(pos);
  }

  if values.is_empty() {
    return None;
  }

  let pattern = format!(
    "^({})$",
    values
      .iter()
      .map(|v| ts_regex_escape(v))
      .collect::<Vec<_>>()
      .join("|")
  );
  Some((format!("#match? {capture_name} \"{pattern}\""), pos))
}

fn ts_regex_escape(s: &str) -> String {
  let mut out = String::with_capacity(s.len());
  for c in s.chars() {
    if matches!(
      c,
      '.'
        | '*'
        | '+'
        | '?'
        | '^'
        | '$'
        | '{'
        | '}'
        | '['
        | ']'
        | '|'
        | '('
        | ')'
        | '\\'
    ) {
      out.push('\\');
    }
    out.push(c);
  }
  out
}

fn read_user_query(
  base_dir: &Path,
  lang: Lang,
  file_name: &str,
) -> SyntaxResult<Option<String>> {
  let query_path = query_path_for_lang(base_dir, lang, file_name);
  if !query_path.exists() {
    return Ok(None);
  }

  fs::read_to_string(&query_path).map(Some).map_err(|e| {
    SyntaxError::BackendError(format!(
      "failed to read query override '{}': {e}",
      query_path.display()
    ))
  })
}

fn query_path_for_lang(
  base_dir: &Path,
  lang: Lang,
  file_name: &str,
) -> PathBuf {
  base_dir.join(<&str>::from(lang)).join(file_name)
}

impl SyntasticaHighlighter {
  /// Create a new Syntastica highlighter with all available themes.
  ///
  /// # Errors
  ///
  /// Currently never returns an error, but returns a Result for API
  /// consistency.
  pub fn new(syntax_queries_dir: Option<&Path>) -> SyntaxResult<Self> {
    let mut themes = FxHashMap::default();

    // Load all available themes
    for theme_name in syntastica_themes::THEMES {
      if let Some(theme) = syntastica_themes::from_str(theme_name) {
        themes.insert((*theme_name).to_string(), theme);
      }
    }

    let default_theme = syntastica_themes::one::dark();

    // Leak the language set into a `'static` reference so the `Processor` can
    // hold it for the remainder of the process lifetime. This is sound for a
    // CLI: the process exits when documentation generation completes and the OS
    // reclaims the memory. It avoids the unsound lifetime fabrication that a
    // raw-pointer cast would require.
    let language_set_static: &'static UserQueryLanguageSet =
      Box::leak(Box::new(UserQueryLanguageSet::new(syntax_queries_dir)));
    Ok(Self {
      themes,
      default_theme,
      language_set: language_set_static,
      workers: Mutex::new(Vec::new()),
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
      #[expect(clippy::match_same_arms, reason = "Explicit for documentation")]
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

    let mut worker = self
      .workers
      .lock()
      .map_err(|e| {
        SyntaxError::HighlightingFailed(format!(
          "Worker pool lock poisoned: {e}"
        ))
      })?
      .pop()
      .unwrap_or_else(|| {
        HighlightWorker {
          processor: Processor::new(self.language_set),
          renderer:  HtmlRenderer::new(),
        }
      });

    let result = worker
      .processor
      .process(code, lang)
      .map(|highlights| render(&highlights, &mut worker.renderer, theme))
      .map_err(|e| SyntaxError::HighlightingFailed(e.to_string()));

    self
      .workers
      .lock()
      .map_err(|e| {
        SyntaxError::HighlightingFailed(format!(
          "Worker pool lock poisoned: {e}"
        ))
      })?
      .push(worker);

    result
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
pub fn create_syntastica_manager(
  syntax_queries_dir: Option<&Path>,
) -> SyntaxResult<SyntaxManager> {
  let highlighter = Box::new(SyntasticaHighlighter::new(syntax_queries_dir)?);
  let config = SyntaxConfig {
    default_theme: Some("one-dark".to_string()),
    ..Default::default()
  };
  Ok(SyntaxManager::new(highlighter, config))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_is_extends_query() {
    assert!(is_extends_query(";; extends\n(foo) @bar"));
    assert!(is_extends_query(";;extends\n(foo) @bar"));
    assert!(!is_extends_query("(foo) @bar"));
    assert!(!is_extends_query(""));
    assert!(!is_extends_query("; extends")); // single semicolon is a comment, not the directive
  }

  #[test]
  fn test_rewrite_any_of_basic() {
    let input = r#"((identifier) @_name (#any-of? @_name "foo" "bar"))"#;
    let output = rewrite_any_of_predicates(input);
    assert!(output.contains("#match?"));
    assert!(output.contains("@_name"));
    assert!(output.contains("^(foo|bar)$"));
    assert!(!output.contains("#any-of?"));
  }

  #[test]
  fn test_rewrite_any_of_multiple() {
    let input = r#"
      ((identifier) @a (#any-of? @a "x" "y"))
      ((identifier) @b (#any-of? @b "p" "q" "r"))
    "#;
    let output = rewrite_any_of_predicates(input);
    assert_eq!(output.matches("#match?").count(), 2);
    assert!(!output.contains("#any-of?"));
    assert!(output.contains("^(x|y)$"));
    assert!(output.contains("^(p|q|r)$"));
  }

  #[test]
  fn test_rewrite_any_of_regex_escaping() {
    let input = r#"((identifier) @a (#any-of? @a "foo.bar" "baz"))"#;
    let output = rewrite_any_of_predicates(input);
    assert!(output.contains("foo\\.bar"));
  }

  #[test]
  fn test_rewrite_any_of_no_match_passthrough() {
    let input = "(foo) @bar (#eq? @bar \"baz\")";
    let output = rewrite_any_of_predicates(input);
    assert_eq!(input, output);
  }

  #[test]
  fn test_rewrite_any_of_nvf_nix_query() {
    // Matches the actual query from nvf's nix.nix
    let input = r#"
;; extends

((apply_expression
  function: (variable_expression
    name: (identifier) @_func
    (#any-of? @_func "mkLuaInline" "entryAnywhere"))
  argument: (indented_string_expression
    (string_fragment) @injection.content))
(#set! injection.language "lua")
(#set! injection.combined))
"#;
    let output = rewrite_any_of_predicates(input);
    assert!(!output.contains("#any-of?"));
    assert!(
      output.contains("#match? @_func \"^(mkLuaInline|entryAnywhere)$\"")
    );
    // Non-any-of predicates must be preserved
    assert!(output.contains("#set! injection.language"));
    assert!(output.contains("#set! injection.combined"));
    assert!(output.contains(";; extends"));
  }
}
