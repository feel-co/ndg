#[expect(
  clippy::disallowed_types,
  reason = "Borrow caller-owned sets without constraining their hasher"
)]
use std::collections::HashSet;
use std::{
  hash::BuildHasher,
  path::{Component, Path, PathBuf},
};

use kuchikikiki::{NodeRef, traits::TendrilSink};
use ndg_commonmark::utils::strip_markdown;
use rustc_hash::FxHashMap;

/// Whether the logical page path names `index.html`.
fn is_index_page(logical: &Path) -> bool {
  logical.file_name().is_some_and(|name| name == "index.html")
}

/// Get the output file for a logical page path.
#[must_use]
pub fn page_file(logical: &Path, pretty: bool) -> PathBuf {
  if !pretty
    || is_index_page(logical)
    || logical.extension().is_none_or(|ext| ext != "html")
  {
    return logical.to_path_buf();
  }
  logical.with_extension("").join("index.html")
}

/// Get the relative link for a logical page path.
#[must_use]
pub fn page_href(target: &str, pretty: bool) -> String {
  if !pretty || is_external_url(target) || target.starts_with('#') {
    return target.to_string();
  }
  let split = target.find(['#', '?']).unwrap_or(target.len());
  let (path, suffix) = target.split_at(split);
  let path = if path == "index.html" {
    ""
  } else if let Some(dir) = path.strip_suffix("/index.html") {
    return format!("{dir}/{suffix}");
  } else if let Some(stem) = path.strip_suffix(".html") {
    return format!("{stem}/{suffix}");
  } else {
    path
  };
  format!("{path}{suffix}")
}

/// Join a root prefix with a link target.
#[must_use]
pub fn join_href(root_prefix: &str, href: &str) -> String {
  let joined = format!("{root_prefix}{href}");
  if joined.is_empty() || joined.starts_with(['#', '?']) {
    format!("./{joined}")
  } else {
    joined
  }
}

/// Whether a URL is absolute (has a scheme or starts with `/`).
#[must_use]
pub fn is_external_url(url: &str) -> bool {
  if url.starts_with('/') {
    return true;
  }
  let end = url.find(['/', '?', '#']).unwrap_or(url.len());
  url[..end].contains(':')
}

/// Get a redirect page from the legacy `.html` path to the pretty URL.
#[must_use]
pub fn legacy_redirect_html(logical: &Path, pretty: bool) -> Option<String> {
  if page_file(logical, pretty) == logical {
    return None;
  }
  let stem = logical.file_stem()?.to_string_lossy();
  let href = html_escape::encode_double_quoted_attribute(&format!(
    "{}/",
    stem
      .replace('%', "%25")
      .replace('#', "%23")
      .replace('?', "%3F")
  ))
  .into_owned();
  Some(format!(
    concat!(
      "<!DOCTYPE html>\n",
      "<html lang=\"en\">\n",
      "<head>\n",
      "<meta charset=\"utf-8\">\n",
      "<title>Redirecting</title>\n",
      "<meta name=\"robots\" content=\"noindex\">\n",
      "<link rel=\"canonical\" href=\"{href}\">\n",
      "<script>location.replace(document.querySelector('link[rel=canonical]').\
       href + location.search + location.hash);</script>\n",
      "<meta http-equiv=\"refresh\" content=\"0; url={href}\">\n",
      "</head>\n",
      "<body><a href=\"{href}\">{href}</a></body>\n",
      "</html>\n",
    ),
    href = href
  ))
}

#[expect(
  clippy::disallowed_types,
  reason = "Borrows the caller's set with its existing hasher"
)]
fn pretty_content_url(
  url: &str,
  is_link: bool,
  parent: &Path,
  moved: bool,
  pages: &HashSet<PathBuf, impl BuildHasher>,
) -> Option<String> {
  let url_trimmed = url.trim();
  if url_trimmed.is_empty()
    || url_trimmed.starts_with(['#', '?'])
    || is_external_url(url_trimmed)
  {
    return None;
  }
  let path = url_trimmed.split(['?', '#']).next().unwrap_or_default();
  let href = if is_link
    && Path::new(path).extension().is_some_and(|ext| ext == "html")
    && logical_link_target(parent, path)
      .is_some_and(|path| pages.contains(&path))
  {
    page_href(url_trimmed, true)
  } else {
    url_trimmed.to_string()
  };
  let href = if moved { format!("../{href}") } else { href };
  let href = join_href("", &href);
  (href != url).then_some(href)
}

fn logical_link_target(parent: &Path, url_path: &str) -> Option<PathBuf> {
  let decoded = percent_encoding::percent_decode_str(url_path)
    .decode_utf8()
    .ok()?;
  let mut target = parent.to_path_buf();
  for component in Path::new(decoded.as_ref()).components() {
    match component {
      Component::Normal(name) => target.push(name),
      Component::CurDir => {},
      Component::ParentDir if target.pop() => {},
      _ => return None,
    }
  }
  Some(target)
}

/// Rebase relative content URLs and prettify links to generated pages.
#[must_use]
#[expect(
  clippy::disallowed_types,
  reason = "Borrows the caller's set with its existing hasher"
)]
pub fn rewrite_pretty_content_urls(
  html: &str,
  logical: &Path,
  pages: &HashSet<PathBuf, impl BuildHasher>,
) -> String {
  let parent = logical.parent().unwrap_or_else(|| Path::new(""));
  let moved = page_file(logical, true) != logical;
  ndg_commonmark::rewrite_urls(html, |url, is_link| {
    pretty_content_url(url, is_link, parent, moved, pages)
  })
}

/// Get the relative path from a page to the root.
#[must_use]
pub fn calculate_root_relative_path(
  file_rel_path: &Path,
  pretty: bool,
) -> String {
  let file = page_file(file_rel_path, pretty);
  let depth = file.components().count();
  if depth <= 1 {
    String::new() // file is at root level
  } else {
    "../".repeat(depth - 1)
  }
}

/// Generate proper asset paths for templates based on file location
#[must_use]
pub fn generate_asset_paths(
  file_rel_path: &Path,
  pretty: bool,
) -> FxHashMap<&'static str, String> {
  let root_prefix = calculate_root_relative_path(file_rel_path, pretty);
  let page =
    |logical: &str| join_href(&root_prefix, &page_href(logical, pretty));

  let mut paths = FxHashMap::default();
  paths.reserve(6);
  paths.insert("stylesheet_path", format!("{root_prefix}assets/style.css"));
  paths.insert("main_js_path", format!("{root_prefix}assets/main.js"));
  paths.insert("search_js_path", format!("{root_prefix}assets/search.js"));

  // Navigation paths
  paths.insert("index_path", page("index.html"));
  paths.insert("options_path", page("options.html"));
  paths.insert("search_path", page("search.html"));

  paths
}

/// Process content through the markdown pipeline and extract plain text.
///
/// Converts markdown to plain text while preserving document structure:
///
/// - Headings and paragraphs are separated by newlines
/// - Multiple consecutive blank lines are collapsed to single blank lines
/// - Whitespace within lines is normalized to single spaces
/// - Anchor markers like {#id} are removed
#[must_use]
pub fn content_to_plaintext(content: &str) -> String {
  normalize_plaintext(&strip_markdown(content), true, true)
}

/// Extract plain text from rendered HTML.
///
/// Uses the same output format as [`content_to_plaintext`], but without blank
/// lines. Block elements start a new line. The text of `<script>`, `<style>`
/// and `<template>` elements is not included. The text of code blocks is
/// included, so that a search can find commands and file names.
#[must_use]
pub fn html_to_plaintext(html: &str) -> String {
  fn walk(node: &NodeRef, out: &mut String) {
    if let Some(text) = node.as_text() {
      out.push_str(&text.borrow());
      return;
    }

    let name = node.as_element().map(|element| &*element.name.local);
    if matches!(name, Some("script" | "style" | "template")) {
      return;
    }
    let is_block = name.is_some_and(is_block_element);
    if name == Some("br") || is_block {
      out.push('\n');
    }
    for child in node.children() {
      walk(&child, out);
    }
    if is_block {
      out.push('\n');
    }
  }

  let mut text = String::with_capacity(html.len() / 2);
  walk(&kuchikikiki::parse_html().one(html), &mut text);
  normalize_plaintext(&text, false, false)
}

const fn is_block_element(name: &str) -> bool {
  matches!(
    name.as_bytes(),
    b"address"
      | b"article"
      | b"aside"
      | b"blockquote"
      | b"dd"
      | b"details"
      | b"div"
      | b"dl"
      | b"dt"
      | b"figcaption"
      | b"figure"
      | b"footer"
      | b"h1"
      | b"h2"
      | b"h3"
      | b"h4"
      | b"h5"
      | b"h6"
      | b"header"
      | b"hr"
      | b"li"
      | b"main"
      | b"nav"
      | b"ol"
      | b"p"
      | b"pre"
      | b"section"
      | b"summary"
      | b"table"
      | b"td"
      | b"th"
      | b"tr"
      | b"ul"
  )
}

/// Normalize extracted text:
///
/// - Multiple consecutive blank lines are collapsed to single blank lines, or
///   removed if `keep_blank_lines` is false
/// - Whitespace within lines is normalized to single spaces
/// - Anchor markers like {#id} are removed iff `strip_anchors`
fn normalize_plaintext(
  plain_text: &str,
  keep_blank_lines: bool,
  strip_anchors: bool,
) -> String {
  let mut result = String::with_capacity(plain_text.len());
  let mut line = String::new();
  let mut prev_empty = false;
  let mut has_output = false;

  for raw_line in plain_text.lines() {
    line.clear();

    // Remove anchor markers {#id} and collapse whitespace.
    for word in raw_line.split_whitespace().filter(|word| {
      !(strip_anchors && word.starts_with("{#") && word.ends_with('}'))
    }) {
      if !line.is_empty() {
        line.push(' ');
      }
      line.push_str(word);
    }

    if line.is_empty() {
      if keep_blank_lines && !prev_empty && has_output {
        result.push('\n');
      }
      prev_empty = true;
    } else {
      if has_output {
        result.push('\n');
      }
      result.push_str(&line);
      has_output = true;
      prev_empty = false;
    }
  }

  result.trim().to_string()
}

#[cfg(test)]
mod tests {
  use rustc_hash::FxHashSet;

  use super::*;

  #[test]
  fn content_to_plaintext_removes_anchor_markers() {
    assert_eq!(
      content_to_plaintext("# Title {#title}\n\nSome text."),
      "Title\nSome text."
    );
  }

  #[test]
  fn html_to_plaintext_keeps_anchor_like_code() {
    let html = "<p>Text</p><pre><code>{#literal-code-token}</code></pre>";
    assert_eq!(html_to_plaintext(html), "Text\n{#literal-code-token}");
  }

  #[test]
  fn html_to_plaintext_separates_blocks() {
    let html = "<h1 id=\"a\">Title</h1><p>Some <em>text</em> and \
                <code>code</code>.</p><ul><li>One</li><li>Two</li></ul>";
    assert_eq!(
      html_to_plaintext(html),
      "Title\nSome text and code.\nOne\nTwo"
    );
  }

  #[test]
  fn html_to_plaintext_keeps_code_blocks_and_skips_scripts() {
    let html = "<p>Before<br>after</p><pre><code>ls -la\ncd \
                /</code></pre><script>alert(1)</script><style>p \
                {}</style><p>End</p>";
    assert_eq!(html_to_plaintext(html), "Before\nafter\nls -la\ncd /\nEnd");
  }

  #[test]
  fn legacy_redirect_points_to_pretty_url() {
    let redirect = |p: &str, pretty| legacy_redirect_html(Path::new(p), pretty);
    assert_eq!(redirect("a/b.html", false), None);
    assert_eq!(redirect("index.html", true), None);
    assert_eq!(redirect("a/index.html", true), None);
    let html = redirect("a/b.html", true).unwrap_or_default();
    assert!(
      html.contains(r#"<link rel="canonical" href="b/">"#),
      "{html}"
    );
    assert!(html.contains(r#"content="0; url=b/""#), "{html}");
    assert!(html.contains("location.hash"), "{html}");
  }

  #[test]
  fn page_file_maps_pages_to_directories() {
    let file = |p: &str, pretty| page_file(Path::new(p), pretty);
    assert_eq!(file("a/b.html", false), Path::new("a/b.html"));
    assert_eq!(file("a/b.html", true), Path::new("a/b/index.html"));
    assert_eq!(file("index.html", true), Path::new("index.html"));
    assert_eq!(file("a/index.html", true), Path::new("a/index.html"));
    assert_eq!(file("a/b.json", true), Path::new("a/b.json"));
  }

  #[test]
  fn page_href_gives_pretty_links() {
    assert_eq!(page_href("a/b.html", false), "a/b.html");
    assert_eq!(page_href("a/b.html", true), "a/b/");
    assert_eq!(page_href("a/b.html#x", true), "a/b/#x");
    assert_eq!(page_href("a/b.html?q=1", true), "a/b/?q=1");
    assert_eq!(page_href("index.html", true), "");
    assert_eq!(page_href("index.html#x", true), "#x");
    assert_eq!(page_href("a/index.html", true), "a/");
    assert_eq!(page_href("#x", true), "#x");
    assert_eq!(
      page_href("https://e.org/a.html", true),
      "https://e.org/a.html"
    );
    assert_eq!(page_href("/a.html", true), "/a.html");
  }

  #[test]
  fn join_href_never_gives_empty_link() {
    assert_eq!(join_href("../", "a/"), "../a/");
    assert_eq!(join_href("", ""), "./");
    assert_eq!(join_href("", "#x"), "./#x");
    assert_eq!(join_href("../", ""), "../");
  }

  #[test]
  fn external_urls() {
    assert!(is_external_url("https://e.org"));
    assert!(is_external_url("mailto:a@e.org"));
    assert!(is_external_url("/abs"));
    assert!(is_external_url("//e.org"));
    assert!(!is_external_url("a/b.html"));
    assert!(!is_external_url("a/b:c.html"));
    assert!(!is_external_url("a.html#x:y"));
  }

  #[test]
  fn pretty_content_urls() {
    let page = Path::new("reference/database.html");
    let index = Path::new("reference/index.html");
    let pages: FxHashSet<_> = ["reference/other.html", "reference/index.html"]
      .map(PathBuf::from)
      .into_iter()
      .collect();
    let url = |u, link, p: &Path| {
      pretty_content_url(
        u,
        link,
        p.parent().unwrap_or_else(|| Path::new("")),
        page_file(p, true) != p,
        &pages,
      )
    };
    assert_eq!(url("other.html", true, page).as_deref(), Some("../other/"));
    assert_eq!(
      url("other.html#x", true, page).as_deref(),
      Some("../other/#x")
    );
    assert_eq!(url("img.png", false, page).as_deref(), Some("../img.png"));
    assert_eq!(url("other.html", true, index).as_deref(), Some("other/"));
    assert_eq!(url("index.html", true, index).as_deref(), Some("./"));
    assert_eq!(url("img.png", false, index), None);
    let htm = Path::new("reference/page.htm");
    assert_eq!(url("other.html", true, htm).as_deref(), Some("other/"));
    assert_eq!(url("img.png", false, htm), None);
    assert_eq!(url("#x", true, page), None);
    assert_eq!(url("", true, page), None);
    assert_eq!(url("https://e.org/a.html", true, page), None);
  }

  #[test]
  fn root_relative_path_with_pretty_urls() {
    let root =
      |p: &str, pretty| calculate_root_relative_path(Path::new(p), pretty);
    assert_eq!(root("a.html", false), "");
    assert_eq!(root("a.html", true), "../");
    assert_eq!(root("index.html", true), "");
    assert_eq!(root("x/a.html", true), "../../");
    assert_eq!(root("x/index.html", true), "../");
  }

  #[test]
  fn asset_paths_with_pretty_urls() {
    let paths = generate_asset_paths(Path::new("a.html"), true);
    assert_eq!(paths["stylesheet_path"], "../assets/style.css");
    assert_eq!(paths["index_path"], "../");
    assert_eq!(paths["options_path"], "../options/");
    assert_eq!(paths["search_path"], "../search/");
    let paths = generate_asset_paths(Path::new("index.html"), true);
    assert_eq!(paths["index_path"], "./");
    assert_eq!(paths["options_path"], "options/");
  }

  #[test]
  fn rewrite_content_for_pretty_urls() {
    let html = rewrite_pretty_content_urls(
      concat!(
        r#"<p><a href="b.html#x">b</a> <img src="i.png" srcset="i.png 1x, j.png 2x">"#,
        r#" <input type="image" src="k.png"> <a href="https://e.org">e</a></p>"#,
      ),
      Path::new("a.html"),
      &FxHashSet::from_iter([PathBuf::from("b.html")]),
    );
    assert!(html.contains(r#"href="../b/#x""#), "{html}");
    assert!(html.contains(r#"src="../i.png""#), "{html}");
    assert!(
      html.contains(r#"srcset="../i.png 1x, ../j.png 2x""#),
      "{html}"
    );
    assert!(html.contains(r#"src="../k.png""#), "{html}");
    assert!(html.contains(r#"href="https://e.org""#), "{html}");
  }

  #[test]
  fn pretty_urls_keep_targets_relative_to_the_explicit_base() {
    let logical = Path::new("docs/intro.html");
    let pages: FxHashSet<_> = ["docs/static.html", "docs/guide.html"]
      .map(PathBuf::from)
      .into_iter()
      .collect();
    for (base, expected) in [
      ("static/", "../static/"),
      (
        "static.html?theme=dark#icons",
        "../static.html?theme=dark#icons",
      ),
      ("/docs/static/", "/docs/static/"),
      ("https://example.org/static/", "https://example.org/static/"),
    ] {
      let input = format!(
        r#"<base target="_blank"><base href="{base}"><base href="ignored/">
           <img src="logo.png" srcset="logo.png 1x, logo-2x.png 2x">
           <a href="guide.html">Guide</a>"#,
      );
      let html = rewrite_pretty_content_urls(&input, logical, &pages);
      assert!(
        html.contains(&format!(r#"<base href="{expected}">"#)),
        "{html}"
      );
      assert!(html.contains(r#"<base href="ignored/">"#), "{html}");
      assert!(html.contains(r#"src="logo.png""#), "{html}");
      assert!(
        html.contains(r#"srcset="logo.png 1x, logo-2x.png 2x""#),
        "{html}"
      );
      assert!(html.contains(r#"href="guide.html""#), "{html}");
    }
  }

  #[test]
  fn pretty_urls_rebase_image_with_query_only_base() {
    let html = rewrite_pretty_content_urls(
      r#"<base href="?theme=dark"><img src="logo.png">"#,
      Path::new("docs/intro.html"),
      &FxHashSet::default(),
    );
    assert!(html.contains(r#"<base href="?theme=dark">"#), "{html}");
    assert!(html.contains(r#"src="../logo.png""#), "{html}");
  }
}
