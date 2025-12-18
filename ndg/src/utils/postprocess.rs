use color_eyre::eyre::Result;
use minifier::{css, html, js};

use crate::config::postprocess::PostprocessConfig;

/// Apply configured postprocessing to HTML content
///
/// # Errors
///
/// Returns an error if minification fails
#[allow(
  clippy::unnecessary_wraps,
  reason = "Maintains consistent API for postprocessing functions"
)]
pub fn process_html(
  content: &str,
  config: &PostprocessConfig,
) -> Result<String> {
  if !config.minify_html {
    return Ok(content.to_string());
  }

  let html_opts = config.html_options();

  let minified = html::minify(content)
    .replace('\n', "")
    .split_whitespace()
    .collect::<Vec<_>>()
    .join(" ");

  let mut result = if html_opts.collapse_whitespace {
    minified
  } else {
    content.to_string()
  };

  if html_opts.remove_comments {
    result = remove_html_comments(&result);
  }

  Ok(result)
}

/// Apply configured postprocessing to CSS content
///
/// # Errors
///
/// Returns an error if minification fails
pub fn process_css(
  content: &str,
  config: &PostprocessConfig,
) -> Result<String> {
  if !config.minify_css {
    return Ok(content.to_string());
  }

  css::minify(content)
    .map(|s| s.to_string())
    .map_err(|e| color_eyre::eyre::eyre!("Failed to minify CSS: {e}"))
}

/// Apply configured postprocessing to JavaScript content
///
/// # Errors
///
/// Returns an error if minification fails
#[allow(
  clippy::unnecessary_wraps,
  reason = "Maintains consistent API for postprocessing functions"
)]
pub fn process_js(content: &str, config: &PostprocessConfig) -> Result<String> {
  if !config.minify_js {
    return Ok(content.to_string());
  }

  Ok(js::minify(content).to_string())
}

/// Remove HTML comments from content
fn remove_html_comments(content: &str) -> String {
  let mut result = String::with_capacity(content.len());
  let mut in_comment = false;
  let mut chars = content.chars().peekable();

  while let Some(ch) = chars.next() {
    if in_comment {
      // Check for end of comment
      if ch == '-' {
        if let Some(&next_ch) = chars.peek() {
          if next_ch == '-' {
            chars.next(); // consume second dash
            if let Some(&next_next_ch) = chars.peek() {
              if next_next_ch == '>' {
                chars.next(); // consume >
                in_comment = false;
              }
            }
          }
        }
      }
    } else if ch == '<' {
      // Check for start of comment
      if let Some(&next_ch) = chars.peek() {
        if next_ch == '!' {
          chars.next(); // consume !
          if let Some(&dash1) = chars.peek() {
            if dash1 == '-' {
              chars.next(); // consume first dash
              if let Some(&dash2) = chars.peek() {
                if dash2 == '-' {
                  chars.next(); // consume second dash
                  in_comment = true;
                  continue;
                }
              }
            }
          }
          // Not a comment, restore the !
          result.push(ch);
          result.push('!');
          continue;
        }
      }
      result.push(ch);
    } else {
      result.push(ch);
    }
  }

  result
}

#[cfg(test)]
mod tests {
  #![allow(clippy::unwrap_used, reason = "Fine in tests")]

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
    let html = "<html>  <body>  Test  </body>  </html>";
    let result = process_html(html, &config).unwrap();
    assert!(result.len() < html.len());
    assert!(!result.contains("  ")); // no double spaces
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
    let css = "body {\n  color: red;\n}";
    let result = process_css(css, &config).unwrap();
    assert!(result.len() < css.len());
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
    let js = "function test() {\n  return true;\n}";
    let result = process_js(js, &config).unwrap();
    assert!(result.len() <= js.len());
  }

  #[test]
  fn test_remove_html_comments() {
    let html = "<div><!-- Comment -->Content</div>";
    let result = remove_html_comments(html);
    assert_eq!(result, "<div>Content</div>");
  }

  #[test]
  fn test_remove_multiple_html_comments() {
    let html = "<!-- First -->A<!-- Second -->B<!-- Third -->";
    let result = remove_html_comments(html);
    assert_eq!(result, "AB");
  }

  #[test]
  fn test_preserve_non_comment_content() {
    let html = "<div><!DOCTYPE html><p>Text</p></div>";
    let result = remove_html_comments(html);
    assert!(result.contains("<!DOCTYPE html>"));
    assert!(result.contains("<p>Text</p>"));
  }
}
