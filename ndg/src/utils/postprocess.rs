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
  let source_type = SourceType::unambiguous();

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
    assert_eq!(process_html(html, &config).unwrap(), html);
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
    let expected = "<!doctype html><html lang=en><meta charset=UTF-8><title>Test</title><body><pre>code    with    spaces</pre><div>Content</div>";
    assert_eq!(result, expected);
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
    assert_eq!(process_html(html, &config).unwrap(), "<div><!-- Keep -->Content</div>");
  }

  #[test]
  fn test_css_minification_disabled() {
    let config = PostprocessConfig {
      minify_css: false,
      ..Default::default()
    };
    let css = "body { color: red; }";
    assert_eq!(process_css(css, &config).unwrap(), css);
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
    let expected = "body{color:red;margin:0}@media (width<=768px){.container{width:100%}}";
    assert_eq!(result, expected);
  }

  #[test]
  fn test_css_minify_option() {
    let css = "body {\n  color: red;\n}";
    let config_minify = PostprocessConfig {
      minify_css: true,
      css: Some(crate::config::postprocess::CssMinifyOptions { minify: true }),
      ..Default::default()
    };
    assert_eq!(process_css(css, &config_minify).unwrap(), "body{color:red}");

    let config_no_minify = PostprocessConfig {
      minify_css: true,
      css: Some(crate::config::postprocess::CssMinifyOptions { minify: false }),
      ..Default::default()
    };
    assert_eq!(process_css(css, &config_no_minify).unwrap(), "body {\n  color: red;\n}\n");
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
    assert!(result.unwrap_err().to_string().contains("Failed to parse CSS"));
  }

  #[test]
  fn test_js_minification_disabled() {
    let config = PostprocessConfig {
      minify_js: false,
      ..Default::default()
    };
    let js = "function test() { return true; }";
    assert_eq!(process_js(js, &config).unwrap(), js);
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
    let expected = "function init(){console.log(`test`),console.log(25)}window.addEventListener(`load`,function(){init()});";
    assert_eq!(result, expected);
  }

  #[test]
  fn test_js_compress_option() {
    let js = "const unused = 1; const x = 5 + 10; console.log(x);";
    let config_compress = PostprocessConfig {
      minify_js: true,
      js: Some(crate::config::postprocess::JsMinifyOptions {
        compress: true,
        mangle:   false,
      }),
      ..Default::default()
    };
    assert_eq!(process_js(js, &config_compress).unwrap(), "const unused=1,x=15;console.log(15);");

    let config_no_compress = PostprocessConfig {
      minify_js: true,
      js: Some(crate::config::postprocess::JsMinifyOptions {
        compress: false,
        mangle:   false,
      }),
      ..Default::default()
    };
    assert_eq!(process_js(js, &config_no_compress).unwrap(), "const unused=1;const x=5+10;console.log(x);");
  }

  #[test]
  fn test_js_mangle_option() {
    let js = "function test() { const localVar = 1; return localVar + 1; }";
    let config = PostprocessConfig {
      minify_js: true,
      js: Some(crate::config::postprocess::JsMinifyOptions {
        compress: false,
        mangle:   true,
      }),
      ..Default::default()
    };
    // oxc mangler doesn't rename const in this case
    let expected = "function test(){const localVar=1;return localVar+1}";
    assert_eq!(process_js(js, &config).unwrap(), expected);
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
    assert!(result.unwrap_err().to_string().contains("Failed to parse JavaScript"));
  }

  #[test]
  fn test_all_minification_disabled() {
    let config = PostprocessConfig::default();
    assert_eq!(process_html("<div>  test  </div>", &config).unwrap(), "<div>  test  </div>");
    assert_eq!(process_css("body { color: red; }", &config).unwrap(), "body { color: red; }");
    assert_eq!(process_js("const x = 1 + 2;", &config).unwrap(), "const x = 1 + 2;");
  }

  #[test]
  fn test_js_commonjs_syntax() {
    let config = PostprocessConfig {
      minify_js: true,
      js: Some(crate::config::postprocess::JsMinifyOptions {
        compress: false,
        mangle:   false,
      }),
      ..Default::default()
    };
    let js = r"
const fs = require('fs');
function processFile(filename) {
  return fs.readFileSync(filename, 'utf8').trim();
}
module.exports = { processFile };
";
    let result = process_js(js, &config).unwrap();
    let expected = "const fs=require(`fs`);function processFile(filename){return fs.readFileSync(filename,`utf8`).trim()}module.exports={processFile};";
    assert_eq!(result, expected);
  }

  #[test]
  fn test_js_es_module_syntax() {
    let config = PostprocessConfig {
      minify_js: true,
      js: Some(crate::config::postprocess::JsMinifyOptions {
        compress: false,
        mangle:   false,
      }),
      ..Default::default()
    };
    let js = r"
import { readFile } from 'fs';
export function processFile(filename) {
  return readFile(filename).trim();
}
export default { processFile };
";
    let result = process_js(js, &config).unwrap();
    let expected = "import{readFile}from\"fs\";export function processFile(filename){return readFile(filename).trim()}export default {processFile};";
    assert_eq!(result, expected);
  }

  #[test]
  fn test_js_browser_apis() {
    let config = PostprocessConfig {
      minify_js: true,
      js: Some(crate::config::postprocess::JsMinifyOptions {
        compress: false,
        mangle:   false,
      }),
      ..Default::default()
    };
    let js = r"
document.addEventListener('click', function(e) {
  fetch('/api').then(r => r.json());
});
";
    let result = process_js(js, &config).unwrap();
    let expected = "document.addEventListener(`click`,function(e){fetch(`/api`).then(r=>r.json())});";
    assert_eq!(result, expected);
  }

  #[test]
  fn test_js_modern_syntax() {
    let config = PostprocessConfig {
      minify_js: true,
      js: Some(crate::config::postprocess::JsMinifyOptions {
        compress: false,
        mangle:   false,
      }),
      ..Default::default()
    };
    let js = r"
const fn = (x) => x * 2;
const obj = { a: 1, ...rest };
async function getData() {
  return await fetch(url);
}
";
    let result = process_js(js, &config).unwrap();
    let expected = "const fn=x=>x*2;const obj={a:1,...rest};async function getData(){return await fetch(url)}";
    assert_eq!(result, expected);
  }

  #[test]
  fn test_js_template_literals() {
    let config = PostprocessConfig {
      minify_js: true,
      js: Some(crate::config::postprocess::JsMinifyOptions {
        compress: false,
        mangle:   false,
      }),
      ..Default::default()
    };
    let js = r#"
const name = "World";
const greeting = `Hello, ${name}!`;
"#;
    let result = process_js(js, &config).unwrap();
    let expected = "const name=`World`;const greeting=`Hello, ${name}!`;";
    assert_eq!(result, expected);
  }
}
