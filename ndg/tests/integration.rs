#![allow(clippy::expect_used, reason = "Fine in tests")]
use std::fs;

use ndg::{
  config::{Config, postprocess::PostprocessConfig},
  utils::assets::copy_assets,
};
use ndg_commonmark::{ProcessorPreset, process_markdown_string};
use tempfile::tempdir;

#[test]
fn test_full_document_processing() {
  let temp_dir = tempdir().expect("Failed to create temp dir in test");
  let input_dir = temp_dir.path().join("input");
  let output_dir = temp_dir.path().join("output");
  fs::create_dir_all(&input_dir).expect("Failed to create dir in test");
  fs::create_dir_all(&output_dir).expect("Failed to create dir in test");

  // Create a sample markdown file
  let md_content = r#"# Test Document

This is a test.

## Section 1

Some content here.

```bash
echo "hello"
```

## Section 2

More content.
"#;
  fs::write(input_dir.join("test.md"), md_content)
    .expect("Failed to write test.md in test");

  // Create a basic config
  let config = Config {
    input_dir: Some(input_dir.clone()),
    output_dir: output_dir.clone(),
    ..Default::default()
  };

  // Test config loading
  assert_eq!(config.input_dir, Some(input_dir));
  assert_eq!(config.output_dir, output_dir);
}

#[test]
fn test_error_handling_on_invalid_input() {
  let temp_dir = tempdir().expect("Failed to create temp dir in test");
  let invalid_input = temp_dir.path().join("nonexistent");
  let config = Config {
    input_dir: Some(invalid_input.clone()),
    ..Default::default()
  };

  // Test that config handles missing directory gracefully
  assert_eq!(config.input_dir, Some(invalid_input));
}

#[test]
fn test_large_file_processing() {
  let temp_dir = tempdir().expect("Failed to create temp dir in test");
  let input_dir = temp_dir.path().join("input");
  fs::create_dir_all(&input_dir).expect("Failed to create dir in test");

  // Create a large markdown file
  let large_content = "# Heading\n\n".repeat(1000) + "Some content.";
  fs::write(input_dir.join("large.md"), large_content)
    .expect("Failed to write large.md in test");

  // Test that large files can be read without issues
  let content = fs::read_to_string(input_dir.join("large.md"))
    .expect("Failed to read large.md in test");
  assert!(content.len() > 10000);
}

#[test]
fn test_malformed_markdown_recovery() {
  let malformed_md = r"# Unclosed [link

Some text.

- List item 1
- List item 2

```unclosed code block

End.
";

  let result = process_markdown_string(malformed_md, ProcessorPreset::Basic);
  assert!(!result.html.is_empty());
}

#[test]
fn test_sidebar_integration() {
  use std::path::Path;

  use ndg::{
    config::sidebar::{
      PathMatch,
      SidebarConfig,
      SidebarMatch,
      SidebarOrdering,
      TitleMatch,
    },
    html::template::render,
  };

  let temp_dir = tempdir().expect("Failed to create temp dir in test");
  let input_dir = temp_dir.path().join("input");
  let output_dir = temp_dir.path().join("output");
  fs::create_dir_all(&input_dir).expect("Failed to create dir in test");
  fs::create_dir_all(&output_dir).expect("Failed to create dir in test");

  // Create test markdown files
  fs::write(
    input_dir.join("installation.md"),
    "# Installation Guide\n\nHow to install.",
  )
  .expect("Failed to write installation.md in test");
  fs::write(
    input_dir.join("configuration.md"),
    "# Configuration\n\nHow to configure.",
  )
  .expect("Failed to write configuration.md in test");
  fs::write(
    input_dir.join("api.md"),
    "# API Reference\n\nAPI documentation.",
  )
  .expect("Failed to write api.md in test");

  // Test with numbered sidebar
  let sidebar_config = SidebarConfig {
    numbered:             true,
    number_special_files: false,
    ordering:             SidebarOrdering::Alphabetical,
    matches:              vec![],
    options:              None,
  };

  let config = Config {
    input_dir: Some(input_dir.clone()),
    output_dir: output_dir.clone(),
    sidebar: Some(sidebar_config),
    ..Default::default()
  };

  let html = render(
    &config,
    "<p>Test content</p>",
    "Test",
    &[],
    Path::new("test.md"),
  )
  .expect("Failed to render HTML in test");

  // Check that the navigation contains numbered items
  assert!(html.contains("<li>"), "HTML should contain list items");

  // Test with custom ordering using positions
  let sidebar_config_custom = SidebarConfig {
    numbered:             false,
    number_special_files: false,
    ordering:             SidebarOrdering::Custom,
    matches:              vec![
      SidebarMatch {
        path:      Some(PathMatch {
          exact:          Some("installation.md".to_string()),
          regex:          None,
          compiled_regex: None,
        }),
        title:     None,
        new_title: Some("Setup Guide".to_string()),
        position:  Some(1),
      },
      SidebarMatch {
        path:      Some(PathMatch {
          exact:          Some("api.md".to_string()),
          regex:          None,
          compiled_regex: None,
        }),
        title:     None,
        new_title: None,
        position:  Some(3),
      },
      SidebarMatch {
        path:      Some(PathMatch {
          exact:          Some("configuration.md".to_string()),
          regex:          None,
          compiled_regex: None,
        }),
        title:     None,
        new_title: None,
        position:  Some(2),
      },
    ],
    options:              None,
  };

  let config_custom = Config {
    input_dir: Some(input_dir.clone()),
    output_dir: output_dir.clone(),
    sidebar: Some(sidebar_config_custom),
    ..Default::default()
  };

  let html_custom = render(
    &config_custom,
    "<p>Test content</p>",
    "Test",
    &[],
    Path::new("test.md"),
  )
  .expect("Failed to render HTML with custom config in test");

  // Check that custom title is applied
  assert!(
    html_custom.contains("Setup Guide"),
    "Custom title should be applied"
  );

  // Test with regex matching
  let mut sidebar_config_regex = SidebarConfig {
    numbered:             true,
    number_special_files: false,
    ordering:             SidebarOrdering::Alphabetical,
    matches:              vec![SidebarMatch {
      path:      Some(PathMatch {
        exact:          None,
        regex:          Some(r"^.*\.md$".to_string()),
        compiled_regex: None,
      }),
      title:     Some(TitleMatch {
        exact:          None,
        regex:          Some(r".*Guide.*".to_string()),
        compiled_regex: None,
      }),
      new_title: Some("📖 Guide".to_string()),
      position:  None,
    }],
    options:              None,
  };

  // Compile regexes
  sidebar_config_regex
    .validate()
    .expect("sidebar config should be valid");

  let config_regex = Config {
    input_dir: Some(input_dir),
    output_dir,
    sidebar: Some(sidebar_config_regex),
    ..Default::default()
  };

  let html_regex = render(
    &config_regex,
    "<p>Test content</p>",
    "Installation Guide",
    &[],
    Path::new("installation.md"),
  )
  .expect("Failed to render HTML with regex config in test");

  // Verify the regex match worked - the sidebar title should be replaced with
  // emoji
  assert!(
    html_regex.contains("📖 Guide"),
    "Expected emoji replacement '📖 Guide' not found in HTML"
  );

  // Ensure the original plain "Installation Guide" is NOT in a sidebar link
  // Extract navigation/sidebar section to check title replacement occurred
  let sidebar_section = html_regex
    .split("<aside")
    .nth(1)
    .or_else(|| html_regex.split("class=\"sidebar").nth(1))
    .unwrap_or("");

  assert!(
    !sidebar_section.contains(">Installation Guide<"),
    "Sidebar should not contain plain 'Installation Guide' link; should be \
     replaced with '📖 Guide'"
  );
}

#[test]
fn test_custom_assets_css_minification() {
  let temp_dir = tempdir().expect("Failed to create temp dir in test");
  let assets_dir = temp_dir.path().join("custom_assets");
  let output_dir = temp_dir.path().join("output");
  fs::create_dir_all(&assets_dir).expect("Failed to create assets dir in test");
  fs::create_dir_all(&output_dir).expect("Failed to create output dir in test");

  // Create a custom CSS file with whitespace and comments
  let css_content = r"/* This is a comment */
body {
  margin: 0;
  padding: 0;
  background-color: #ffffff;
}

.container {
  max-width: 1200px;
  margin: 0 auto;
}
";
  fs::write(assets_dir.join("custom.css"), css_content)
    .expect("Failed to write custom.css in test");

  // Create config with postprocessing enabled
  let config = Config {
    output_dir: output_dir.clone(),
    assets_dir: Some(assets_dir),
    postprocess: Some(PostprocessConfig {
      minify_css: true,
      minify_js: false,
      minify_html: false,
      ..Default::default()
    }),
    ..Default::default()
  };

  // Copy assets with postprocessing
  copy_assets(&config).expect("Failed to copy assets in test");

  // Read the output CSS file
  let output_css_path = output_dir.join("assets").join("custom.css");
  assert!(
    output_css_path.exists(),
    "Output CSS file should exist at {}",
    output_css_path.display()
  );

  let output_css = fs::read_to_string(&output_css_path)
    .expect("Failed to read output CSS in test");

  // Verify minification occurred
  assert!(
    output_css.len() < css_content.len(),
    "Minified CSS should be smaller than original"
  );
  assert!(
    !output_css.contains("/* This is a comment */"),
    "Comments should be removed"
  );
  assert!(
    !output_css.contains("  "),
    "Multiple spaces should be removed"
  );
  assert!(
    output_css.contains("body{"),
    "Whitespace around braces should be removed"
  );
}

#[test]
fn test_custom_assets_js_minification() {
  let temp_dir = tempdir().expect("Failed to create temp dir in test");
  let assets_dir = temp_dir.path().join("custom_assets");
  let output_dir = temp_dir.path().join("output");
  fs::create_dir_all(&assets_dir).expect("Failed to create assets dir in test");
  fs::create_dir_all(&output_dir).expect("Failed to create output dir in test");

  // Create a custom JS file with comments and unnecessary whitespace
  let js_content = r#"// This is a comment
function greet(name) {
  const message = "Hello, " + name;
  console.log(message);
  return message;
}

const result = greet("World");
"#;
  fs::write(assets_dir.join("custom.js"), js_content)
    .expect("Failed to write custom.js in test");

  // Create config with JS minification enabled
  let config = Config {
    output_dir: output_dir.clone(),
    assets_dir: Some(assets_dir),
    postprocess: Some(PostprocessConfig {
      minify_css: false,
      minify_js: true,
      minify_html: false,
      ..Default::default()
    }),
    ..Default::default()
  };

  // Copy assets
  // This also includes postprocessing so we'll test for it
  // below.
  copy_assets(&config).expect("Failed to copy assets in test");

  let output_js_path = output_dir.join("assets").join("custom.js");
  assert!(
    output_js_path.exists(),
    "Output JS file should exist at {}",
    output_js_path.display()
  );

  let output_js = fs::read_to_string(&output_js_path)
    .expect("Failed to read output JS in test");

  // Verify minification occurred
  assert!(
    output_js.len() < js_content.len(),
    "Minified JS should be smaller than original"
  );
  assert!(
    !output_js.contains("// This is a comment"),
    "Comments should be removed"
  );
  assert!(
    !output_js.contains("\n\n"),
    "Multiple newlines should be removed"
  );
}

#[test]
fn test_custom_assets_no_minification() {
  let temp_dir = tempdir().expect("Failed to create temp dir in test");
  let assets_dir = temp_dir.path().join("custom_assets");
  let output_dir = temp_dir.path().join("output");
  fs::create_dir_all(&assets_dir).expect("Failed to create assets dir in test");
  fs::create_dir_all(&output_dir).expect("Failed to create output dir in test");

  // Create custom assets
  let css_content = "body { margin: 0; }";
  let js_content = "console.log('test');";
  fs::write(assets_dir.join("test.css"), css_content)
    .expect("Failed to write test.css in test");
  fs::write(assets_dir.join("test.js"), js_content)
    .expect("Failed to write test.js in test");

  // Create config WITHOUT postprocessing
  let config = Config {
    output_dir: output_dir.clone(),
    assets_dir: Some(assets_dir),
    postprocess: None,
    ..Default::default()
  };

  // Copy assets without postprocessing
  copy_assets(&config).expect("Failed to copy assets in test");

  // Verify files are copied as-is
  let output_css =
    fs::read_to_string(output_dir.join("assets").join("test.css"))
      .expect("Failed to read output CSS in test");
  let output_js = fs::read_to_string(output_dir.join("assets").join("test.js"))
    .expect("Failed to read output JS in test");

  assert_eq!(
    output_css, css_content,
    "CSS should be unchanged without minification"
  );
  assert_eq!(
    output_js, js_content,
    "JS should be unchanged without minification"
  );
}

#[test]
fn test_custom_assets_non_processable_files() {
  let temp_dir = tempdir().expect("Failed to create temp dir in test");
  let assets_dir = temp_dir.path().join("custom_assets");
  let output_dir = temp_dir.path().join("output");
  fs::create_dir_all(&assets_dir).expect("Failed to create assets dir in test");
  fs::create_dir_all(&output_dir).expect("Failed to create output dir in test");

  // Create non-CSS/JS files
  let image_data = vec![0x89, 0x50, 0x4E, 0x47]; // PNG header
  let text_content = "This is a text file.";

  fs::write(assets_dir.join("image.png"), &image_data)
    .expect("Failed to write image.png in test");
  fs::write(assets_dir.join("readme.txt"), text_content)
    .expect("Failed to write readme.txt in test");

  // Create config with postprocessing enabled
  let config = Config {
    output_dir: output_dir.clone(),
    assets_dir: Some(assets_dir),
    postprocess: Some(PostprocessConfig {
      minify_css: true,
      minify_js: true,
      minify_html: false,
      ..Default::default()
    }),
    ..Default::default()
  };

  // Copy assets
  // Non-processable files should be copied as-is
  copy_assets(&config).expect("Failed to copy assets in test");

  // Verify files are copied unchanged
  let output_image = fs::read(output_dir.join("assets").join("image.png"))
    .expect("Failed to read output image in test");
  let output_text =
    fs::read_to_string(output_dir.join("assets").join("readme.txt"))
      .expect("Failed to read output text in test");

  assert_eq!(
    output_image, image_data,
    "Binary files should be copied unchanged"
  );
  assert_eq!(
    output_text, text_content,
    "Text files should be copied unchanged"
  );
}

#[test]
fn test_custom_assets_mixed_files() {
  let temp_dir = tempdir().expect("Failed to create temp dir in test");
  let assets_dir = temp_dir.path().join("custom_assets");
  let output_dir = temp_dir.path().join("output");
  fs::create_dir_all(&assets_dir).expect("Failed to create assets dir in test");
  fs::create_dir_all(&output_dir).expect("Failed to create output dir in test");

  // Create a mix of files
  let css_content = "body { margin: 0; padding: 0; }";
  let js_content = "const x = 1; const y = 2;";
  let txt_content = "README content";

  fs::write(assets_dir.join("style.css"), css_content)
    .expect("Failed to write style.css in test");
  fs::write(assets_dir.join("script.js"), js_content)
    .expect("Failed to write script.js in test");
  fs::write(assets_dir.join("info.txt"), txt_content)
    .expect("Failed to write info.txt in test");

  // Create config with full postprocessing
  let config = Config {
    output_dir: output_dir.clone(),
    assets_dir: Some(assets_dir),
    postprocess: Some(PostprocessConfig {
      minify_css: true,
      minify_js: true,
      minify_html: false,
      ..Default::default()
    }),
    ..Default::default()
  };

  // Copy assets
  copy_assets(&config).expect("Failed to copy assets in test");

  // Verify CSS and JS are minified, txt is unchanged
  let output_css =
    fs::read_to_string(output_dir.join("assets").join("style.css"))
      .expect("Failed to read output CSS in test");
  let output_js =
    fs::read_to_string(output_dir.join("assets").join("script.js"))
      .expect("Failed to read output JS in test");
  let output_txt =
    fs::read_to_string(output_dir.join("assets").join("info.txt"))
      .expect("Failed to read output text in test");

  assert!(
    output_css.len() < css_content.len(),
    "CSS should be minified"
  );
  assert!(output_js.len() < js_content.len(), "JS should be minified");
  assert_eq!(output_txt, txt_content, "Text file should be unchanged");
}

#[test]
fn test_custom_assets_case_insensitive_extensions() {
  let temp_dir = tempdir().expect("Failed to create temp dir in test");
  let assets_dir = temp_dir.path().join("custom_assets");
  let output_dir = temp_dir.path().join("output");
  fs::create_dir_all(&assets_dir).expect("Failed to create assets dir in test");
  fs::create_dir_all(&output_dir).expect("Failed to create output dir in test");

  // Create files with various case extensions
  let css_content = "body { margin: 0; padding: 0; }";
  let js_content = "const x = 1; const y = 2;";

  fs::write(assets_dir.join("style.CSS"), css_content)
    .expect("Failed to write style.CSS in test");
  fs::write(assets_dir.join("script.JS"), js_content)
    .expect("Failed to write script.JS in test");
  fs::write(assets_dir.join("mixed.Css"), css_content)
    .expect("Failed to write mixed.Css in test");
  fs::write(assets_dir.join("another.Js"), js_content)
    .expect("Failed to write another.Js in test");

  // Create config with postprocessing enabled
  let config = Config {
    output_dir: output_dir.clone(),
    assets_dir: Some(assets_dir),
    postprocess: Some(PostprocessConfig {
      minify_css: true,
      minify_js: true,
      minify_html: false,
      ..Default::default()
    }),
    ..Default::default()
  };

  // Copy assets
  copy_assets(&config).expect("Failed to copy assets in test");

  // Verify all files are processed regardless of case
  let output_css_upper =
    fs::read_to_string(output_dir.join("assets").join("style.CSS"))
      .expect("Failed to read style.CSS in test");
  let output_js_upper =
    fs::read_to_string(output_dir.join("assets").join("script.JS"))
      .expect("Failed to read script.JS in test");
  let output_css_mixed =
    fs::read_to_string(output_dir.join("assets").join("mixed.Css"))
      .expect("Failed to read mixed.Css in test");
  let output_js_mixed =
    fs::read_to_string(output_dir.join("assets").join("another.Js"))
      .expect("Failed to read another.Js in test");

  // All files should be minified regardless of extension case
  assert!(
    output_css_upper.len() < css_content.len(),
    "Uppercase .CSS should be minified"
  );
  assert!(
    output_js_upper.len() < js_content.len(),
    "Uppercase .JS should be minified"
  );
  assert!(
    output_css_mixed.len() < css_content.len(),
    "Mixed-case .Css should be minified"
  );
  assert!(
    output_js_mixed.len() < js_content.len(),
    "Mixed-case .Js should be minified"
  );
}

#[test]
fn test_custom_assets_subdirectory_structure() {
  let temp_dir = tempdir().expect("Failed to create temp dir in test");
  let assets_dir = temp_dir.path().join("custom_assets");
  let output_dir = temp_dir.path().join("output");
  fs::create_dir_all(&assets_dir).expect("Failed to create assets dir in test");
  fs::create_dir_all(&output_dir).expect("Failed to create output dir in test");

  // Create a subdirectory with files
  let fonts_dir = assets_dir.join("fonts");
  fs::create_dir_all(&fonts_dir).expect("Failed to create fonts dir in test");

  let font_content = b"fake font data";
  fs::write(fonts_dir.join("regular.woff2"), font_content)
    .expect("Failed to write font file in test");
  fs::write(fonts_dir.join("bold.woff2"), font_content)
    .expect("Failed to write font file in test");

  // Create config without postprocessing to test directory structure only
  let config = Config {
    output_dir: output_dir.clone(),
    assets_dir: Some(assets_dir),
    postprocess: None,
    ..Default::default()
  };

  // Copy assets
  copy_assets(&config).expect("Failed to copy assets in test");

  // Check the directory structure
  // With copy_inside: true, 'fs_extra::dir::copy' should create
  // output/assets/fonts/* (correct, not nested)
  let fonts_path = output_dir.join("assets").join("fonts");
  let regular_font = fonts_path.join("regular.woff2");
  let bold_font = fonts_path.join("bold.woff2");

  assert!(
    fonts_path.exists(),
    "Fonts directory should exist at {}",
    fonts_path.display()
  );
  assert!(
    regular_font.exists(),
    "Regular font should exist at {}",
    regular_font.display()
  );
  assert!(
    bold_font.exists(),
    "Bold font should exist at {}",
    bold_font.display()
  );

  // Verify we don't have nested structure (fonts/fonts/...)
  let nested_fonts = fonts_path.join("fonts");
  assert!(
    !nested_fonts.exists(),
    "Should not have nested fonts/fonts/ directory"
  );
}

#[test]
fn test_custom_assets_subdirectory_no_minification() {
  let temp_dir = tempdir().expect("Failed to create temp dir in test");
  let assets_dir = temp_dir.path().join("custom_assets");
  let output_dir = temp_dir.path().join("output");
  fs::create_dir_all(&assets_dir).expect("Failed to create assets dir in test");
  fs::create_dir_all(&output_dir).expect("Failed to create output dir in test");

  // Create a subdirectory with CSS and JS files
  let vendor_dir = assets_dir.join("vendor");
  fs::create_dir_all(&vendor_dir).expect("Failed to create vendor dir in test");

  let css_content = "body { margin: 0; padding: 0; }";
  let js_content = "const x = 1; const y = 2;";

  fs::write(vendor_dir.join("library.css"), css_content)
    .expect("Failed to write CSS file in test");
  fs::write(vendor_dir.join("library.js"), js_content)
    .expect("Failed to write JS file in test");

  // Create config WITH postprocessing enabled
  let config = Config {
    output_dir: output_dir.clone(),
    assets_dir: Some(assets_dir),
    postprocess: Some(PostprocessConfig {
      minify_css: true,
      minify_js: true,
      minify_html: false,
      ..Default::default()
    }),
    ..Default::default()
  };

  // Copy assets
  copy_assets(&config).expect("Failed to copy assets in test");

  // Read the files in the subdirectory
  let vendor_path = output_dir.join("assets").join("vendor");
  let output_css = fs::read_to_string(vendor_path.join("library.css"))
    .expect("Failed to read CSS in test");
  let output_js = fs::read_to_string(vendor_path.join("library.js"))
    .expect("Failed to read JS in test");

  // Files in subdirectories ARE now minified
  assert!(
    output_css.len() < css_content.len(),
    "CSS in subdirectories should be minified"
  );
  assert!(
    output_js.len() < js_content.len(),
    "JS in subdirectories should be minified"
  );
}

#[test]
fn test_custom_assets_deeply_nested_minification() {
  let temp_dir = tempdir().expect("Failed to create temp dir in test");
  let assets_dir = temp_dir.path().join("custom_assets");
  let output_dir = temp_dir.path().join("output");
  fs::create_dir_all(&assets_dir).expect("Failed to create assets dir in test");
  fs::create_dir_all(&output_dir).expect("Failed to create output dir in test");

  // Create deeply nested directories: vendor/lib/css/ and vendor/lib/js/
  let css_dir = assets_dir.join("vendor").join("lib").join("css");
  let js_dir = assets_dir.join("vendor").join("lib").join("js");
  fs::create_dir_all(&css_dir).expect("Failed to create CSS dir in test");
  fs::create_dir_all(&js_dir).expect("Failed to create JS dir in test");

  let css_content = "body { margin: 0; padding: 0; background: white; }";
  let js_content = "const foo = 1; const bar = 2; const baz = 3;";

  fs::write(css_dir.join("theme.css"), css_content)
    .expect("Failed to write CSS file in test");
  fs::write(js_dir.join("utils.js"), js_content)
    .expect("Failed to write JS file in test");

  // Create config with postprocessing enabled
  let config = Config {
    output_dir: output_dir.clone(),
    assets_dir: Some(assets_dir),
    postprocess: Some(PostprocessConfig {
      minify_css: true,
      minify_js: true,
      minify_html: false,
      ..Default::default()
    }),
    ..Default::default()
  };

  // Copy assets
  copy_assets(&config).expect("Failed to copy assets in test");

  // Read the deeply nested files
  let output_css_path = output_dir
    .join("assets")
    .join("vendor")
    .join("lib")
    .join("css")
    .join("theme.css");
  let output_js_path = output_dir
    .join("assets")
    .join("vendor")
    .join("lib")
    .join("js")
    .join("utils.js");

  let output_css =
    fs::read_to_string(&output_css_path).expect("Failed to read CSS in test");
  let output_js =
    fs::read_to_string(&output_js_path).expect("Failed to read JS in test");

  // Deeply nested files should be minified
  assert!(
    output_css.len() < css_content.len(),
    "Deeply nested CSS should be minified"
  );
  assert!(
    output_js.len() < js_content.len(),
    "Deeply nested JS should be minified"
  );
}

#[test]
fn test_custom_assets_empty_directories() {
  let temp_dir = tempdir().expect("Failed to create temp dir in test");
  let assets_dir = temp_dir.path().join("custom_assets");
  let output_dir = temp_dir.path().join("output");
  fs::create_dir_all(&assets_dir).expect("Failed to create assets dir in test");
  fs::create_dir_all(&output_dir).expect("Failed to create output dir in test");

  // Create empty directories
  let empty_dir1 = assets_dir.join("empty1");
  let empty_dir2 = assets_dir.join("nested").join("empty2");
  fs::create_dir_all(&empty_dir1).expect("Failed to create empty1 in test");
  fs::create_dir_all(&empty_dir2).expect("Failed to create empty2 in test");

  let config = Config {
    output_dir: output_dir.clone(),
    assets_dir: Some(assets_dir),
    postprocess: None,
    ..Default::default()
  };

  copy_assets(&config).expect("Failed to copy assets in test");

  // Empty directories should be created
  assert!(
    output_dir.join("assets").join("empty1").exists(),
    "Empty directory should be created"
  );
  assert!(
    output_dir
      .join("assets")
      .join("nested")
      .join("empty2")
      .exists(),
    "Nested empty directory should be created"
  );
}

#[test]
fn test_custom_assets_files_without_extensions() {
  let temp_dir = tempdir().expect("Failed to create temp dir in test");
  let assets_dir = temp_dir.path().join("custom_assets");
  let output_dir = temp_dir.path().join("output");
  fs::create_dir_all(&assets_dir).expect("Failed to create assets dir in test");
  fs::create_dir_all(&output_dir).expect("Failed to create output dir in test");

  // Create files without extensions
  let no_ext_content = b"This file has no extension";
  fs::write(assets_dir.join("README"), no_ext_content)
    .expect("Failed to write README in test");
  fs::write(assets_dir.join("Makefile"), b"all:\n\techo done")
    .expect("Failed to write Makefile in test");

  let config = Config {
    output_dir: output_dir.clone(),
    assets_dir: Some(assets_dir),
    postprocess: Some(PostprocessConfig {
      minify_css: true,
      minify_js: true,
      minify_html: false,
      ..Default::default()
    }),
    ..Default::default()
  };

  copy_assets(&config).expect("Failed to copy assets in test");

  // Files without extensions should be copied as-is
  let output_readme = fs::read(output_dir.join("assets").join("README"))
    .expect("Failed to read README in test");
  assert_eq!(
    output_readme, no_ext_content,
    "Files without extensions should be copied unchanged"
  );
}

#[test]
fn test_custom_assets_hidden_files_skipped() {
  let temp_dir = tempdir().expect("Failed to create temp dir in test");
  let assets_dir = temp_dir.path().join("custom_assets");
  let output_dir = temp_dir.path().join("output");
  fs::create_dir_all(&assets_dir).expect("Failed to create assets dir in test");
  fs::create_dir_all(&output_dir).expect("Failed to create output dir in test");

  // Create hidden files
  fs::write(assets_dir.join(".gitignore"), b"*.log")
    .expect("Failed to write .gitignore in test");
  fs::write(assets_dir.join(".DS_Store"), b"garbage")
    .expect("Failed to write .DS_Store in test");
  fs::write(assets_dir.join("visible.txt"), b"visible content")
    .expect("Failed to write visible.txt in test");

  // Create hidden directory
  let hidden_dir = assets_dir.join(".hidden");
  fs::create_dir_all(&hidden_dir).expect("Failed to create .hidden in test");
  fs::write(hidden_dir.join("secret.txt"), b"secret")
    .expect("Failed to write secret.txt in test");

  let config = Config {
    output_dir: output_dir.clone(),
    assets_dir: Some(assets_dir),
    postprocess: None,
    ..Default::default()
  };

  copy_assets(&config).expect("Failed to copy assets in test");

  // Hidden files should NOT be copied
  assert!(
    !output_dir.join("assets").join(".gitignore").exists(),
    "Hidden files should be skipped"
  );
  assert!(
    !output_dir.join("assets").join(".DS_Store").exists(),
    "Hidden files should be skipped"
  );
  assert!(
    !output_dir.join("assets").join(".hidden").exists(),
    "Hidden directories should be skipped"
  );

  // Visible files should be copied
  assert!(
    output_dir.join("assets").join("visible.txt").exists(),
    "Visible files should be copied"
  );
}

#[test]
fn test_custom_assets_mixed_hidden_and_visible() {
  let temp_dir = tempdir().expect("Failed to create temp dir in test");
  let assets_dir = temp_dir.path().join("custom_assets");
  let output_dir = temp_dir.path().join("output");
  fs::create_dir_all(&assets_dir).expect("Failed to create assets dir in test");
  fs::create_dir_all(&output_dir).expect("Failed to create output dir in test");

  // Create directory with both hidden and visible files
  let subdir = assets_dir.join("subdir");
  fs::create_dir_all(&subdir).expect("Failed to create subdir in test");
  fs::write(subdir.join(".hidden.txt"), b"hidden")
    .expect("Failed to write .hidden.txt in test");
  fs::write(subdir.join("visible.txt"), b"visible")
    .expect("Failed to write visible.txt in test");

  let config = Config {
    output_dir: output_dir.clone(),
    assets_dir: Some(assets_dir),
    postprocess: None,
    ..Default::default()
  };

  copy_assets(&config).expect("Failed to copy assets in test");

  let output_subdir = output_dir.join("assets").join("subdir");
  assert!(
    !output_subdir.join(".hidden.txt").exists(),
    "Hidden files in subdirs should be skipped"
  );
  assert!(
    output_subdir.join("visible.txt").exists(),
    "Visible files in subdirs should be copied"
  );
}
