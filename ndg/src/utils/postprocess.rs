use color_eyre::{Result, eyre::eyre};
use oxc_allocator::Allocator;
use oxc_codegen::{Codegen, CodegenOptions};
use oxc_minifier::{CompressOptions, MangleOptions, Minifier, MinifierOptions};
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

  let css_opts = config.css_options();

  let stylesheet = lightningcss::stylesheet::StyleSheet::parse(
    content,
    lightningcss::stylesheet::ParserOptions::default(),
  )
  .map_err(|e| eyre!("Failed to parse CSS: {e}"))?;

  let result = stylesheet
    .to_css(lightningcss::stylesheet::PrinterOptions {
      minify: css_opts.minify,
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

  let js_opts = config.js_options();

  let minifier_options = MinifierOptions {
    compress: if js_opts.compress {
      Some(CompressOptions::default())
    } else {
      None
    },
    mangle:   if js_opts.mangle {
      Some(MangleOptions::default())
    } else {
      None
    },
  };

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
    assert_eq!(result, html);
  }

  #[test]
  fn test_html_minification_enabled() {
    let config = PostprocessConfig {
      minify_html: true,
      ..Default::default()
    };
    let html = r#"<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="UTF-8">
    <title>Test</title>
  </head>
  <body>
    <pre>code    with    spaces</pre>
    <div><!-- Comment -->Content</div>
  </body>
</html>"#;
    let result = process_html(html, &config).unwrap();

    assert!(result.len() < html.len());
    assert!(result.contains("charset"));
    assert!(result.contains("code    with    spaces")); // <pre> preserves whitespace
    assert!(!result.contains("<!-- Comment -->")); // Comments removed by default
    assert!(result.contains("Content"));
  }

  #[test]
  fn test_html_remove_comments_option() {
    let config = PostprocessConfig {
      minify_html: true,
      html: Some(crate::config::postprocess::HtmlMinifyOptions {
        remove_comments: false,
      }),
      ..Default::default()
    };
    let html = "<div><!-- Keep -->Content</div>";
    let result = process_html(html, &config).unwrap();

    assert!(result.contains("<!-- Keep -->"));
  }

  #[test]
  fn test_css_minification_disabled() {
    let config = PostprocessConfig {
      minify_css: false,
      ..Default::default()
    };
    let css = "body { color: red; }";
    let result = process_css(css, &config).unwrap();
    assert_eq!(result, css);
  }

  #[test]
  fn test_css_minification_enabled() {
    let config = PostprocessConfig {
      minify_css: true,
      ..Default::default()
    };
    let css = r"
/* Comment */
body {
  color: #ff0000;
  margin: 0;
}

@media (max-width: 768px) {
  .container { width: 100%; }
}
";
    let result = process_css(css, &config).unwrap();

    assert!(result.len() < css.len());
    assert!(!result.contains("/*"));
    assert!(!result.contains('\n'));
    assert!(result.contains("@media"));
    // lightningcss optimizes #ff0000 to red
    assert!(result.contains("red") || result.contains("#f00"));
  }

  #[test]
  fn test_css_minify_option() {
    let css = "body { color: red; }";

    // With minify: true
    let config_minify = PostprocessConfig {
      minify_css: true,
      css: Some(crate::config::postprocess::CssMinifyOptions { minify: true }),
      ..Default::default()
    };
    let result_minify = process_css(css, &config_minify).unwrap();

    // With minify: false (should still parse but not minify)
    let config_no_minify = PostprocessConfig {
      minify_css: true,
      css: Some(crate::config::postprocess::CssMinifyOptions { minify: false }),
      ..Default::default()
    };
    let result_no_minify = process_css(css, &config_no_minify).unwrap();

    // minify: false should produce longer output
    assert!(result_no_minify.len() >= result_minify.len());
  }

  #[test]
  fn test_css_invalid_syntax() {
    let config = PostprocessConfig {
      minify_css: true,
      ..Default::default()
    };
    let css = "body { @@@invalid: syntax; }";
    let result = process_css(css, &config);

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("parse"));
  }

  #[test]
  fn test_js_minification_disabled() {
    let config = PostprocessConfig {
      minify_js: false,
      ..Default::default()
    };
    let js = "function test() { return true; }";
    let result = process_js(js, &config).unwrap();
    assert_eq!(result, js);
  }

  #[test]
  fn test_js_minification_enabled() {
    let config = PostprocessConfig {
      minify_js: true,
      ..Default::default()
    };

    let js = r"
// Initialize app
function init() {
  const x = 5 + 10 * 2;
  console.log('test');
  console.log(x);
}
window.addEventListener('load', function() {
  init();
});
";
    let result = process_js(js, &config).unwrap();

    assert!(result.len() < js.len());
    assert!(result.contains("25")); // Constant folding: 5 + 10 * 2
    assert!(result.contains("console.log"));
    assert!(result.contains("addEventListener"));
  }

  #[test]
  fn test_js_compress_option() {
    let js = "const unused = 1; const x = 5 + 10; console.log(x);";

    // With compress: true (should do DCE and constant folding)
    let config_compress = PostprocessConfig {
      minify_js: true,
      js: Some(crate::config::postprocess::JsMinifyOptions {
        compress: true,
        mangle:   false,
      }),
      ..Default::default()
    };
    let result_compress = process_js(js, &config_compress).unwrap();

    // With compress: false (no optimizations)
    let config_no_compress = PostprocessConfig {
      minify_js: true,
      js: Some(crate::config::postprocess::JsMinifyOptions {
        compress: false,
        mangle:   false,
      }),
      ..Default::default()
    };
    let result_no_compress = process_js(js, &config_no_compress).unwrap();

    // Compress should be more aggressive
    assert!(result_compress.len() < result_no_compress.len());
    // With compress, constant folding should produce 15
    assert!(result_compress.contains("15"));
  }

  #[test]
  fn test_js_mangle_option() {
    let js = "function test() { const localVar = 1; return localVar + 1; }";

    // With mangle: true
    let config_mangle = PostprocessConfig {
      minify_js: true,
      js: Some(crate::config::postprocess::JsMinifyOptions {
        compress: false,
        mangle:   true,
      }),
      ..Default::default()
    };
    let result_mangle = process_js(js, &config_mangle).unwrap();

    // With mangle: false
    let config_no_mangle = PostprocessConfig {
      minify_js: true,
      js: Some(crate::config::postprocess::JsMinifyOptions {
        compress: false,
        mangle:   false,
      }),
      ..Default::default()
    };
    let result_no_mangle = process_js(js, &config_no_mangle).unwrap();

    // Without mangle, original names should be preserved
    assert!(result_no_mangle.contains("localVar"));
    // With mangle, names may be shortened (oxc's mangler behavior)
    // At minimum, both should produce valid code
    assert!(!result_mangle.is_empty());
    assert!(!result_no_mangle.is_empty());
  }

  #[test]
  fn test_js_invalid_syntax() {
    let config = PostprocessConfig {
      minify_js: true,
      ..Default::default()
    };
    let js = "function test() { @@@invalid }";
    let result = process_js(js, &config);

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("parse"));
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
}
