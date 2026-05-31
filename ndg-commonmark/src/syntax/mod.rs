//! Provides a trait-based architecture for syntax highlighting that allows
//! multiple backends to be plugged in.
//! Currently supported backends:
//! - **Syntastica** - Modern tree-sitter based highlighting with 60+ themes
//! - **Syntect** - Uses Sublime Text syntax definitions, with two-face added
//!   for extended syntax definitions

pub mod error;
pub mod types;

// Re-export commonly used types
pub use error::{SyntaxError, SyntaxResult};
pub use types::{SyntaxConfig, SyntaxHighlighter, SyntaxManager};

// Compile-time check for mutually exclusive backends
#[cfg(all(feature = "syntastica", feature = "syntect"))]
compile_error!(
  "Cannot enable both 'syntastica' and 'syntect' features simultaneously. \
   They are mutually exclusive."
);

// Syntastica backend implementation
#[cfg(feature = "syntastica")] mod syntastica;
#[cfg(feature = "syntastica")] pub use syntastica::*;

// Syntect backend implementation
#[cfg(feature = "syntect")] mod syntect;
#[cfg(feature = "syntect")] pub use syntect::*;

/// Create the default syntax manager based on available features.
///
/// This function will:
/// - Use **Syntastica** if the 'syntastica' feature is enabled
/// - Use **Syntect** if the 'syntect' feature is enabled
/// - Return an error if both are enabled (mutual exclusivity check)
/// - Return an error if neither is enabled
///
/// **Note**: While the `Syntect` feature is enabled, the two-face crate
/// will also be pulled to provide additional Syntax highlighting.
///
/// # Errors
///
/// Returns an error if both syntastica and syntect features are enabled,
/// or if neither feature is enabled, or if backend initialization fails.
#[allow(
  clippy::missing_const_for_fn,
  reason = "backend-enabled builds call non-const syntax manager constructors"
)]
pub fn create_default_manager(
  syntax_queries_dir: Option<&std::path::Path>,
) -> SyntaxResult<SyntaxManager> {
  #[cfg(not(feature = "syntastica"))]
  let _ = syntax_queries_dir;

  // Runtime check for mutual exclusivity (backup to compile-time check)
  #[cfg(all(feature = "syntastica", feature = "syntect"))]
  {
    return Err(SyntaxError::MutuallyExclusiveBackends);
  }

  #[cfg(feature = "syntastica")]
  {
    create_syntastica_manager(syntax_queries_dir)
  }

  #[cfg(feature = "syntect")]
  {
    return create_syntect_manager();
  }

  #[cfg(not(any(feature = "syntastica", feature = "syntect")))]
  {
    Err(SyntaxError::NoBackendAvailable)
  }
}

#[cfg(test)]
mod tests {
  use super::{types::*, *};

  #[test]
  fn test_syntax_config_default() {
    let config = SyntaxConfig::default();
    assert!(config.fallback_to_plain);
    assert!(config.language_aliases.contains_key("js"));
    assert_eq!(config.language_aliases["js"], "javascript");
  }

  #[test]
  fn test_language_resolution() {
    let manager =
      SyntaxManager::new(Box::new(NoopHighlighter), SyntaxConfig::default());

    assert_eq!(manager.resolve_language("js"), "javascript");
    assert_eq!(manager.resolve_language("py"), "python");
    assert_eq!(manager.resolve_language("ts"), "typescript");
    assert_eq!(manager.resolve_language("rust"), "rust");
    assert_eq!(manager.resolve_language("nix"), "nix");
  }

  #[test]
  fn test_modular_access_to_syntax_types() {
    // Test that we can access types from submodules
    use super::{
      error::{SyntaxError, SyntaxResult},
      types::SyntaxConfig,
    };

    // Test error type creation
    let error = SyntaxError::UnsupportedLanguage("test".to_string());
    assert!(matches!(error, SyntaxError::UnsupportedLanguage(_)));

    // Test result type
    let result: SyntaxResult<String> = Err(SyntaxError::NoBackendAvailable);
    assert!(result.is_err());

    // Test config creation
    let config = SyntaxConfig::default();
    assert!(config.fallback_to_plain);
    assert!(config.language_aliases.contains_key("js"));

    // Test that re-exports work at the module level
    let _config2: SyntaxConfig = SyntaxConfig::default();
    let _error2: SyntaxError = SyntaxError::BackendError("test".to_string());
  }

  #[cfg(feature = "syntect")]
  #[test]
  fn test_nix_language_support() {
    let manager = create_default_manager(None)
      .expect("Failed to create default syntax manager");
    let languages = manager.highlighter().supported_languages();

    // Verify that Nix is supported via two-face
    assert!(
      languages.contains(&"nix".to_string()),
      "Expected Nix language support via two-face"
    );

    // Test highlighting Nix code
    // Without two-face Nix is not highlighted, so what we are *really* trying
    // to test here is whether two-face integration worked.
    let nix_code = r#"
{ pkgs ? import <nixpkgs> {} }:

pkgs.stdenv.mkDerivation rec {
  pname = "hello";
  version = "2.12";

  src = pkgs.fetchurl {
    url = "mirror://gnu/hello/${pname}-${version}.tar.gz";
    sha256 = "1ayhp9v4m4rdhjmnl2bq3cibrbqqkgjbl3s7yk2nhlh8vj3ay16g";
  };
}
"#;

    let result = manager.highlight_code(nix_code, "nix", Some("Nord"));
    assert!(
      result.is_ok(),
      "Failed to highlight Nix code: {:?}",
      result.err()
    );
  }

  struct NoopHighlighter;

  impl SyntaxHighlighter for NoopHighlighter {
    fn name(&self) -> &'static str {
      "noop"
    }

    fn supported_languages(&self) -> Vec<String> {
      Vec::new()
    }

    fn available_themes(&self) -> Vec<String> {
      Vec::new()
    }

    fn highlight(
      &self,
      _code: &str,
      _language: &str,
      _theme: Option<&str>,
    ) -> SyntaxResult<String> {
      Ok(String::new())
    }

    fn language_from_extension(&self, _extension: &str) -> Option<String> {
      None
    }
  }
}
