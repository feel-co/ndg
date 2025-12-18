#![allow(
  clippy::unwrap_used,
  clippy::cast_precision_loss,
  reason = "Fine in examples"
)]

use ndg::{
  config::postprocess::{HtmlMinifyOptions, PostprocessConfig},
  utils::postprocess::{process_css, process_html, process_js},
};

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

  // Demo 1: HTML without minification
  println!("--- HTML (Normal) ---");
  println!("Size: {} bytes\n", html.len());
  let config_no_minify = PostprocessConfig {
    minify_html: false,
    ..Default::default()
  };
  let html_normal = process_html(html, &config_no_minify).unwrap();
  println!("{html_normal}\n");

  // Demo 2: HTML with minification (collapse whitespace only)
  println!("--- HTML (Minified - Collapse Whitespace) ---");
  let config_minify_ws = PostprocessConfig {
    minify_html: true,
    html: Some(HtmlMinifyOptions {
      collapse_whitespace: true,
      remove_comments:     false,
    }),
    ..Default::default()
  };
  let html_minified_ws = process_html(html, &config_minify_ws).unwrap();
  println!(
    "Size: {} bytes (saved {} bytes, {:.1}% reduction)",
    html_minified_ws.len(),
    html.len() - html_minified_ws.len(),
    ((html.len() - html_minified_ws.len()) as f64 / html.len() as f64) * 100.0
  );
  println!("{html_minified_ws}\n");

  // Demo 3: HTML with full minification (collapse whitespace + remove comments)
  println!("--- HTML (Minified - Full) ---");
  let config_minify_full = PostprocessConfig {
    minify_html: true,
    html: Some(HtmlMinifyOptions {
      collapse_whitespace: true,
      remove_comments:     true,
    }),
    ..Default::default()
  };
  let html_minified_full = process_html(html, &config_minify_full).unwrap();
  println!(
    "Size: {} bytes (saved {} bytes, {:.1}% reduction)",
    html_minified_full.len(),
    html.len() - html_minified_full.len(),
    ((html.len() - html_minified_full.len()) as f64 / html.len() as f64)
      * 100.0
  );
  println!("{html_minified_full}\n");

  // Demo 4: CSS without minification
  println!("--- CSS (Normal) ---");
  println!("Size: {} bytes\n", css.len());
  let config_no_minify_css = PostprocessConfig {
    minify_css: false,
    ..Default::default()
  };
  let css_normal = process_css(css, &config_no_minify_css).unwrap();
  println!("{css_normal}\n");

  // Demo 5: CSS with minification
  println!("--- CSS (Minified) ---");
  let config_minify_css = PostprocessConfig {
    minify_css: true,
    ..Default::default()
  };
  let css_minified = process_css(css, &config_minify_css).unwrap();
  println!(
    "Size: {} bytes (saved {} bytes, {:.1}% reduction)",
    css_minified.len(),
    css.len() - css_minified.len(),
    ((css.len() - css_minified.len()) as f64 / css.len() as f64) * 100.0
  );
  println!("{css_minified}\n");

  // Demo 6: JavaScript without minification
  println!("--- JavaScript (Normal) ---");
  println!("Size: {} bytes\n", js.len());
  let config_no_minify_js = PostprocessConfig {
    minify_js: false,
    ..Default::default()
  };
  let js_normal = process_js(js, &config_no_minify_js).unwrap();
  println!("{js_normal}\n");

  // Demo 7: JavaScript with minification
  println!("--- JavaScript (Minified) ---");
  let config_minify_js = PostprocessConfig {
    minify_js: true,
    ..Default::default()
  };
  let js_minified = process_js(js, &config_minify_js).unwrap();
  println!(
    "Size: {} bytes (saved {} bytes, {:.1}% reduction)",
    js_minified.len(),
    js.len() - js_minified.len(),
    ((js.len() - js_minified.len()) as f64 / js.len() as f64) * 100.0
  );
  println!("{js_minified}\n");

  // Summary
  println!("=== Summary ===");
  println!(
    "HTML: {} -> {} bytes ({:.1}% reduction)",
    html.len(),
    html_minified_full.len(),
    ((html.len() - html_minified_full.len()) as f64 / html.len() as f64)
      * 100.0
  );
  println!(
    "CSS:  {} -> {} bytes ({:.1}% reduction)",
    css.len(),
    css_minified.len(),
    ((css.len() - css_minified.len()) as f64 / css.len() as f64) * 100.0
  );
  println!(
    "JS:   {} -> {} bytes ({:.1}% reduction)",
    js.len(),
    js_minified.len(),
    ((js.len() - js_minified.len()) as f64 / js.len() as f64) * 100.0
  );

  let total_original = html.len() + css.len() + js.len();
  let total_minified =
    html_minified_full.len() + css_minified.len() + js_minified.len();
  println!(
    "\nTotal: {} -> {} bytes ({:.1}% reduction)",
    total_original,
    total_minified,
    ((total_original - total_minified) as f64 / total_original as f64) * 100.0
  );
}
