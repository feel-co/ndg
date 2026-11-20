use std::path::{Path, PathBuf};

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

/// Prettify a relative URL, or return `None` for an absolute one.
#[must_use]
pub fn pretty_content_url(
  url: &str,
  is_link: bool,
  logical: &Path,
) -> Option<String> {
  let url_trimmed = url.trim();
  if url_trimmed.is_empty()
    || url_trimmed.starts_with(['#', '?'])
    || is_external_url(url_trimmed)
  {
    return None;
  }
  let href = if is_link {
    page_href(url_trimmed, true)
  } else {
    url_trimmed.to_string()
  };
  let href = if is_index_page(logical) {
    href
  } else {
    format!("../{href}")
  };
  let href = join_href("", &href);
  (href != url).then_some(href)
}

/// Prettify URLs to drop `.html`.
#[must_use]
pub fn rewrite_pretty_content_urls(html: &str, logical: &Path) -> String {
  ndg_commonmark::rewrite_urls(html, |url, is_link| {
    pretty_content_url(url, is_link, logical)
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
  let plain_text = strip_markdown(content);

  let mut result = String::with_capacity(plain_text.len());
  let mut line = String::new();
  let mut prev_empty = false;
  let mut has_output = false;

  for raw_line in plain_text.lines() {
    line.clear();

    // Remove anchor markers {#id} and collapse whitespace.
    for word in raw_line
      .split_whitespace()
      .filter(|word| !(word.starts_with("{#") && word.ends_with('}')))
    {
      if !line.is_empty() {
        line.push(' ');
      }
      line.push_str(word);
    }

    if line.is_empty() {
      if !prev_empty && has_output {
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
  use super::*;

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
    let url = |u, link, p| pretty_content_url(u, link, p);
    assert_eq!(url("other.html", true, page).as_deref(), Some("../other/"));
    assert_eq!(
      url("other.html#x", true, page).as_deref(),
      Some("../other/#x")
    );
    assert_eq!(url("img.png", false, page).as_deref(), Some("../img.png"));
    assert_eq!(url("other.html", true, index).as_deref(), Some("other/"));
    assert_eq!(url("index.html", true, index).as_deref(), Some("./"));
    assert_eq!(url("img.png", false, index), None);
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
      r#"<p><a href="b.html#x">b</a> <img src="i.png" srcset="i.png 1x, j.png 2x"> <a href="https://e.org">e</a></p>"#,
      Path::new("a.html"),
    );
    assert!(html.contains(r#"href="../b/#x""#), "{html}");
    assert!(html.contains(r#"src="../i.png""#), "{html}");
    assert!(
      html.contains(r#"srcset="../i.png 1x, ../j.png 2x""#),
      "{html}"
    );
    assert!(html.contains(r#"href="https://e.org""#), "{html}");
  }
}
