use color_eyre::{Result, eyre::eyre};
use oxc_allocator::Allocator;
use oxc_codegen::{Codegen, CodegenOptions};
use oxc_minifier::{Minifier, MinifierOptions};
use oxc_parser::Parser;
use oxc_span::SourceType;

use crate::config::postprocess::PostprocessConfig;

/// Apply HTML minification if enabled
///
/// # Arguments
///
/// * `content` - HTML content to process
/// * `config` - Postprocessing configuration controlling minification behavior
///
/// # Returns
///
/// Returns the original content unchanged if `config.minify_html` is `false`,
/// otherwise returns minified HTML.
///
/// # Errors
///
/// This function does not currently return errors, but uses `Result` for
/// API consistency with other postprocessing functions.
#[allow(
  clippy::unnecessary_wraps,
  reason = "API consistency with other postprocessing functions"
)]
pub fn process_html(
  content: &str,
  config: &PostprocessConfig,
) -> Result<String> {
  if !config.minify_html {
    return Ok(content.to_string());
  }

  let html_opts = config.html_options();

  let cfg = minify_html::Cfg {
    keep_comments: !html_opts.remove_comments,
    ..minify_html::Cfg::default()
  };

  let minified = minify_html::minify(content.as_bytes(), &cfg);
  Ok(String::from_utf8_lossy(&minified).into_owned())
}

/// Apply CSS minification if enabled
///
/// # Arguments
///
/// * `content` - CSS content to process
/// * `config` - Postprocessing configuration controlling minification behavior
///
/// # Returns
///
/// Returns the original content unchanged if `config.minify_css` is `false`,
/// otherwise returns minified CSS.
///
/// # Errors
///
/// Returns an error if:
/// - The CSS cannot be parsed (syntax errors)
/// - The minification process fails
pub fn process_css(
  content: &str,
  config: &PostprocessConfig,
) -> Result<String> {
  if !config.minify_css {
    return Ok(content.to_string());
  }

  let stylesheet = lightningcss::stylesheet::StyleSheet::parse(
    content,
    lightningcss::stylesheet::ParserOptions::default(),
  )
  .map_err(|e| eyre!("Failed to parse CSS: {e}"))?;

  let result = stylesheet
    .to_css(lightningcss::stylesheet::PrinterOptions {
      minify: true,
      ..Default::default()
    })
    .map_err(|e| eyre!("Failed to minify CSS: {e}"))?;

  Ok(result.code)
}

/// Apply JavaScript minification if enabled
///
/// # Arguments
///
/// * `content` - JavaScript content to process
/// * `config` - Postprocessing configuration controlling minification behavior
///
/// # Returns
///
/// Returns the original content unchanged if `config.minify_js` is `false`,
/// otherwise returns minified JavaScript.
///
/// # Errors
///
/// Returns an error if:
/// - The JavaScript cannot be parsed (syntax errors)
/// - The minification process fails
pub fn process_js(content: &str, config: &PostprocessConfig) -> Result<String> {
  if !config.minify_js {
    return Ok(content.to_string());
  }

  let allocator = Allocator::default();
  let source_type = SourceType::mjs();

  let ret = Parser::new(&allocator, content, source_type).parse();

  if !ret.errors.is_empty() {
    return Err(eyre!(
      "Failed to parse JavaScript: {}",
      ret
        .errors
        .iter()
        .map(std::string::ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ")
    ));
  }

  let mut program = ret.program;

  let minifier_options = MinifierOptions::default();
  let minifier = Minifier::new(minifier_options);
  minifier.minify(&allocator, &mut program);

  let codegen_options = CodegenOptions::minify();
  let printed = Codegen::new().with_options(codegen_options).build(&program);

  Ok(printed.code)
}

#[cfg(test)]
mod tests {
  #![allow(clippy::unwrap_used, reason = "Tests can unwrap")]

  use super::*;

  #[test]
  fn test_html_minification_disabled() {
    let config = PostprocessConfig {
      minify_html: false,
      ..Default::default()
    };
    let html = "<html>  <body>  Test  </body>  </html>";
    let result = process_html(html, &config).unwrap();
    assert_eq!(
      result, html,
      "Content should be unchanged when minification is disabled"
    );
  }

  #[test]
  fn test_html_minification_enabled() {
    let config = PostprocessConfig {
      minify_html: true,
      ..Default::default()
    };
    let html = "<html>\n  <body>\n    <p>Test</p>\n  </body>\n</html>";
    let result = process_html(html, &config).unwrap();

    assert!(
      result.len() <= html.len(),
      "Minified HTML should be shorter or equal"
    );
    assert!(result.contains("Test"), "Content should be preserved");
  }

  #[test]
  fn test_html_preserves_whitespace_sensitive_elements() {
    let config = PostprocessConfig {
      minify_html: true,
      ..Default::default()
    };
    let html = "<pre>code    with    spaces</pre>";
    let result = process_html(html, &config).unwrap();

    assert!(
      result.contains("code    with    spaces"),
      "Pre tags should preserve whitespace"
    );
  }

  #[test]
  fn test_html_removes_comments_by_default() {
    let config = PostprocessConfig {
      minify_html: true,
      ..Default::default()
    };
    let html = "<div><!-- This is a comment -->Content</div>";
    let result = process_html(html, &config).unwrap();

    assert!(
      !result.contains("<!-- This is a comment -->"),
      "Comments should be removed by default"
    );
    assert!(result.contains("Content"), "Content should be preserved");
  }

  #[test]
  fn test_html_keeps_comments_when_configured() {
    let config = PostprocessConfig {
      minify_html: true,
      html: Some(crate::config::postprocess::HtmlMinifyOptions {
        remove_comments: false,
      }),
      ..Default::default()
    };
    let html = "<div><!-- Keep this -->Content</div>";
    let result = process_html(html, &config).unwrap();

    assert!(
      result.contains("<!-- Keep this -->"),
      "Comments should be kept when configured"
    );
  }

  #[test]
  fn test_css_minification_disabled() {
    let config = PostprocessConfig {
      minify_css: false,
      ..Default::default()
    };
    let css = "body { color: red; }";
    let result = process_css(css, &config).unwrap();
    assert_eq!(
      result, css,
      "CSS should be unchanged when minification is disabled"
    );
  }

  #[test]
  fn test_css_minification_enabled() {
    let config = PostprocessConfig {
      minify_css: true,
      ..Default::default()
    };
    let css = "body {\n  color: red;\n  margin: 0;\n}";
    let result = process_css(css, &config).unwrap();

    assert!(result.len() < css.len(), "Minified CSS should be shorter");
    assert!(result.contains("red"), "Color value should be preserved");
    assert!(!result.contains('\n'), "Newlines should be removed");
  }

  #[test]
  fn test_css_minification_optimizes() {
    let config = PostprocessConfig {
      minify_css: true,
      ..Default::default()
    };
    // lightningcss can optimize color values
    let css = ".test { color: #ff0000; }";
    let result = process_css(css, &config).unwrap();

    // Should either keep #ff0000 or optimize to red
    assert!(
      result.contains("red") || result.contains("#f00"),
      "Color should be optimized"
    );
  }

  #[test]
  fn test_css_minification_handles_invalid_syntax() {
    let config = PostprocessConfig {
      minify_css: true,
      ..Default::default()
    };
    // This is actually valid CSS in some contexts (declarations can be omitted)
    // We need to use truly invalid CSS
    let css = "body { @@@invalid: syntax; }";
    let result = process_css(css, &config);

    assert!(result.is_err(), "Invalid CSS should return an error");
    assert!(
      result.unwrap_err().to_string().contains("parse"),
      "Error should mention parsing"
    );
  }

  #[test]
  fn test_js_minification_disabled() {
    let config = PostprocessConfig {
      minify_js: false,
      ..Default::default()
    };
    let js = "function test() { return true; }";
    let result = process_js(js, &config).unwrap();
    assert_eq!(
      result, js,
      "JS should be unchanged when minification is disabled"
    );
  }

  #[test]
  fn test_js_minification_enabled() {
    let config = PostprocessConfig {
      minify_js: true,
      ..Default::default()
    };

    // Use code with side effects so it won't be eliminated by DCE
    let js =
      "console.log( 'test' ) ;\nconst x  =  1  +  2 ;\nconsole.log( x ) ;";
    let result = process_js(js, &config).unwrap();

    // Minification should compress, a.k.a, remove extra spaces, optimize
    // syntax, and apply constant folding
    assert!(result.len() < js.len(), "Minified JS should be shorter");

    // Should preserve console.log calls (they have side effects)
    assert!(
      result.contains("console.log"),
      "Should preserve console.log"
    );
  }

  #[test]
  fn test_js_minification_invalid_syntax() {
    let config = PostprocessConfig {
      minify_js: true,
      ..Default::default()
    };
    // Use truly invalid JavaScript that cannot be parsed
    let js = "function test() { @@@invalid }";
    let result = process_js(js, &config);

    assert!(result.is_err(), "Invalid JS should return an error");
    assert!(
      result.unwrap_err().to_string().contains("parse"),
      "Error should mention parsing"
    );
  }

  #[test]
  fn test_html_document_prod() {
    let config = PostprocessConfig {
      minify_html: true,
      ..Default::default()
    };
    let html = r#"<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="UTF-8">
    <title>Test Page</title>
  </head>
  <body>
    <nav>
      <a href="/">Home</a>
      <a href="/about">About</a>
    </nav>
    <main>
      <h1>Welcome</h1>
      <p>This is a test paragraph with some content.</p>
    </main>
  </body>
</html>"#;
    let result = process_html(html, &config).unwrap();

    assert!(result.len() < html.len(), "Should compress realistic HTML");
    assert!(result.contains("Welcome"), "Should preserve content");
    assert!(result.contains("charset"), "Should preserve meta tags");
    assert!(!result.contains("  "), "Should remove extra spaces");
  }

  #[test]
  fn test_html_empty_content() {
    let config = PostprocessConfig {
      minify_html: true,
      ..Default::default()
    };
    let html = "";
    let result = process_html(html, &config).unwrap();
    assert_eq!(result, "", "Empty HTML should remain empty");
  }

  #[test]
  fn test_css_realistic_stylesheet() {
    let config = PostprocessConfig {
      minify_css: true,
      ..Default::default()
    };
    let css = r"
// Header Styles
.header {
  background-color: #ffffff;
  padding: 20px;
  margin: 0;
}

.nav-item {
  display: inline-block;
  color: #000000;
  text-decoration: none;
}

// Footer
.footer {
  background: #333333;
  color: white;
}
";
    let result = process_css(css, &config).unwrap();

    assert!(result.len() < css.len(), "Should compress realistic CSS");
    assert!(!result.contains("/*"), "Should remove comments");
    assert!(!result.contains('\n'), "Should remove newlines");
    assert!(result.contains(".header"), "Should preserve class names");
    assert!(
      result.contains("inline-block"),
      "Should preserve property values"
    );
  }

  #[test]
  fn test_css_empty_content() {
    let config = PostprocessConfig {
      minify_css: true,
      ..Default::default()
    };
    let css = "";
    let result = process_css(css, &config).unwrap();
    assert_eq!(result, "", "Empty CSS should remain empty");
  }

  #[test]
  fn test_css_with_media_queries() {
    let config = PostprocessConfig {
      minify_css: true,
      ..Default::default()
    };
    let css = r"
@media (max-width: 768px) {
  .container {
    width: 100%;
  }
}
";
    let result = process_css(css, &config).unwrap();

    assert!(result.contains("@media"), "Should preserve media queries");
    // lightningcss optimizes "max-width: 768px" to modern "width<=768px" syntax
    assert!(
      result.contains("width") && result.contains("768"),
      "Should preserve width conditions"
    );
    assert!(result.len() < css.len(), "Should still minify");
  }

  #[test]
  fn test_js_realistic_code() {
    let config = PostprocessConfig {
      minify_js: true,
      ..Default::default()
    };
    let js = r"
// Initialize application
function init() {
  const container = document.getElementById('app');
  if (container) {
    container.innerHTML = 'Hello World';
  }
}

// Run on load
window.addEventListener('load', function() {
  init();
});
";
    let result = process_js(js, &config).unwrap();

    assert!(result.len() < js.len(), "Should compress realistic JS");
    // NOTE: oxc may preserve some comments depending on configuration
    // The important thing is that code is minified and functional
    assert!(
      result.contains("getElementById"),
      "Should preserve method names"
    );
    assert!(
      result.contains("addEventListener"),
      "Should preserve API calls"
    );
  }

  #[test]
  fn test_js_empty_content() {
    let config = PostprocessConfig {
      minify_js: true,
      ..Default::default()
    };
    let js = "";
    let result = process_js(js, &config).unwrap();
    assert_eq!(result, "", "Empty JS should remain empty");
  }

  #[test]
  fn test_js_constant_folding() {
    let config = PostprocessConfig {
      minify_js: true,
      ..Default::default()
    };
    let js = "const result = 5 + 10 * 2; console.log(result);";
    let result = process_js(js, &config).unwrap();

    // oxc should fold the constant expression
    assert!(
      result.len() < js.len(),
      "Should optimize constant expressions"
    );
    assert!(result.contains("25"), "Should fold 5 + 10 * 2 to 25");
  }

  #[test]
  fn test_js_preserves_string_literals() {
    let config = PostprocessConfig {
      minify_js: true,
      ..Default::default()
    };
    let js = r#"const message = "Hello, World!"; console.log(message);"#;
    let result = process_js(js, &config).unwrap();

    assert!(
      result.contains("Hello, World!"),
      "Should preserve string content"
    );
  }

  #[test]
  fn test_all_minification_disabled() {
    let config = PostprocessConfig::default();
    let html = "<div>  test  </div>";
    let css = "body { color: red; }";
    let js = "const x = 1 + 2;";

    assert_eq!(process_html(html, &config).unwrap(), html);
    assert_eq!(process_css(css, &config).unwrap(), css);
    assert_eq!(process_js(js, &config).unwrap(), js);
  }

  #[test]
  fn test_html_with_inline_styles_and_scripts() {
    let config = PostprocessConfig {
      minify_html: true,
      ..Default::default()
    };
    let html = r#"<div style="color: red;">
      <script>console.log('test');</script>
      Content
    </div>"#;
    let result = process_html(html, &config).unwrap();

    assert!(result.contains("style="), "Should preserve inline styles");
    assert!(result.contains("<script>"), "Should preserve script tags");
    assert!(result.contains("Content"), "Should preserve content");
  }
}
