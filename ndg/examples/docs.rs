#![allow(clippy::unwrap_used, reason = "Fine in examples")]
use ndg::{
  config::postprocess::{HtmlMinifyOptions, PostprocessConfig},
  utils::postprocess::{process_css, process_html, process_js},
};

// This examples matches the sample input in POSTPROCESSING.md to give the user
// a way to verify the minification for themselves. This is essentially what the
// postprocess example does, but with less "fluff". We simply demonstrate
// instead of showing off. For once.
fn main() {
  let html = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>Sample Page</title>
</head>
<body>
    <!-- This is a comment -->
    <div class="container">
        <h1>Hello    World</h1>
        <p>
            This   is   a   paragraph   with
            lots     of     whitespace.
        </p>
    </div>
    <!-- Another comment -->
</body>
</html>"#;

  let config_keep_comments = PostprocessConfig {
    minify_html: true,
    html: Some(HtmlMinifyOptions {
      remove_comments: false,
    }),
    ..Default::default()
  };

  let html_result = process_html(html, &config_keep_comments).unwrap();
  println!("HTML OUTPUT:");
  println!("{html_result}");
  println!();

  let css = r"body {
    margin: 0;
    padding: 0;
    font-family: Arial, sans-serif;
}

.container {
    max-width: 1200px;
    margin: 0 auto;
    padding: 20px;
}

h1 {
    color: #333333;
    font-size: 2em;
}

p {
    color: #666666;
    line-height: 1.6;
}";

  let config_css = PostprocessConfig {
    minify_css: true,
    ..Default::default()
  };

  let css_result = process_css(css, &config_css).unwrap();
  println!("CSS OUTPUT:");
  println!("{css_result}");
  println!();

  let js = r#"function greet(name) {
    console.log("Hello, " + name);
    return true;
}

function calculateSum(a, b) {
    // Add two numbers
    const result = a + b;
    return result;
}

// Initialize
greet("World");"#;

  let config_js = PostprocessConfig {
    minify_js: true,
    ..Default::default()
  };

  let js_result = process_js(js, &config_js).unwrap();
  println!("JS OUTPUT:");
  println!("{js_result}");
}
