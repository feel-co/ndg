//! Provides a trait-based architecture for syntax highlighting that allows
//! multiple backends to be plugged in.
//! Currently supported backends:
//!
//! * Syntastica
//! * Syntect

// Module declarations
pub mod error;
pub mod types;

// Re-export commonly used types
pub use error::{SyntaxError, SyntaxResult};
pub use types::{SyntaxConfig, SyntaxHighlighter, SyntaxManager};

// Compile-time check for mutually exclusive backends
#[cfg(all(feature = "syntastica", feature = "syntect"))]
compile_error!(
    "Cannot enable both 'syntastica' and 'syntect' features simultaneously. They are mutually exclusive."
);

// Syntastica backend implementation
#[cfg(feature = "syntastica")]
mod syntastica;
#[cfg(feature = "syntastica")]
pub use syntastica::*;

// Syntect backend implementation
#[cfg(feature = "syntect")]
mod syntect;
#[cfg(feature = "syntect")]
pub use syntect::*;

/// Create the default syntax manager based on available features.
///
/// Returns a Syntastica-based manager if the `syntastica` feature is enabled,
/// otherwise falls back to a Syntect-based manager. Both provide syntax highlighting
/// capabilities, but Syntastica offers more modern tree-sitter based parsing.
/// Create a default syntax manager based on available features.
///
/// This function will:
/// - Use Syntastica if the 'syntastica' feature is enabled
/// - Use Syntect if the 'syntect' feature is enabled
/// - Return an error if both are enabled (mutual exclusivity check)
/// - Return an error if neither is enabled
pub fn create_default_manager() -> SyntaxResult<SyntaxManager> {
    // Runtime check for mutual exclusivity (backup to compile-time check)
    #[cfg(all(feature = "syntastica", feature = "syntect"))]
    {
        return Err(SyntaxError::MutuallyExclusiveBackends);
    }

    #[cfg(feature = "syntastica")]
    {
        return create_syntastica_manager();
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

    #[cfg(feature = "syntect")]
    #[test]
    fn test_syntect_highlighter() {
        let highlighter = SyntectHighlighter::default();
        assert_eq!(highlighter.name(), "Syntect");
        assert!(!highlighter.supported_languages().is_empty());
        assert!(!highlighter.available_themes().is_empty());
    }

    #[cfg(feature = "syntect")]
    #[test]
    fn test_syntect_highlight_simple() {
        let highlighter = SyntectHighlighter::default();
        let result = highlighter.highlight("fn main() {}", "rust", None);
        assert!(result.is_ok());
        let html = result.unwrap();
        assert!(html.contains("main"));
    }

    #[cfg(feature = "syntastica")]
    #[test]
    fn test_syntastica_highlighter() {
        let highlighter = SyntasticaHighlighter::new().unwrap();
        assert_eq!(highlighter.name(), "Syntastica");
        assert!(!highlighter.supported_languages().is_empty());
        assert!(!highlighter.available_themes().is_empty());
    }

    #[cfg(feature = "syntastica")]
    #[test]
    fn test_syntastica_highlight_simple() {
        let highlighter = SyntasticaHighlighter::new().unwrap();
        let result = highlighter.highlight("fn main() {}", "rust", None);
        assert!(result.is_ok());
        let html = result.unwrap();
        assert!(html.contains("main"));
    }

    #[cfg(any(feature = "syntastica", feature = "syntect"))]
    #[test]
    fn test_syntax_manager() {
        let manager = create_default_manager().unwrap();
        assert!(!manager.highlighter().supported_languages().is_empty());

        let resolved = manager.resolve_language("js");
        assert_eq!(resolved, "javascript");
    }

    #[cfg(any(feature = "syntastica", feature = "syntect"))]
    #[test]
    fn test_language_resolution() {
        let manager = create_default_manager().unwrap();

        // Test alias resolution
        assert_eq!(manager.resolve_language("js"), "javascript");
        assert_eq!(manager.resolve_language("py"), "python");
        assert_eq!(manager.resolve_language("ts"), "typescript");

        // Test non-alias languages
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
}
