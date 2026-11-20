use std::{fmt::Write, path::Path};

use html_escape::encode_text;
use misstep::Result;
use ndg_commonmark::MarkdownProcessor;
use ndg_config::Config;
use ndg_utils::html::{calculate_root_relative_path, join_href, page_href};
use rustc_hash::FxHashMap;

use crate::{
  option_pages::{OptionPage, OptionPageSet},
  template::{self, OptionsSidebarHtml},
  toc_tree::{TocPathPart, TocTree},
};

/// Link builder for a given prefix
struct PageLinks {
  root_prefix: String,
  pretty:      bool,
}

impl PageLinks {
  fn new(config: &Config, root_path: &Path) -> Self {
    Self {
      root_prefix: calculate_root_relative_path(root_path, config.pretty_urls),
      pretty:      config.pretty_urls,
    }
  }

  /// Get a page's link
  fn href(&self, logical: &str) -> String {
    join_href(&self.root_prefix, &page_href(logical, self.pretty))
  }
}

/// Render one generated option group page.
///
/// # Errors
///
/// Returns an error if the options template or TOC template cannot be rendered.
#[expect(
  clippy::implicit_hasher,
  reason = "the source-order map is internal and deliberately uses FxHashMap"
)]
pub fn render_option_page(
  config: &Config,
  page: &OptionPage,
  pages: &[OptionPage],
  input_order: &FxHashMap<String, usize>,
  processor: &MarkdownProcessor,
) -> Result<String> {
  let root_path = Path::new(&page.path);
  let links = PageLinks::new(config, root_path);
  let mut options_html = generate_option_page_header(page, &links);
  options_html.push_str(&template::generate_options_html(
    &page.options,
    config,
    processor,
  ));
  let options_toc = template::render_options_toc_with_order(
    config,
    &page.options,
    Some(input_order),
  )?;
  let option_groups_toc =
    generate_option_pages_toc(pages, Some(&page.path), &links);
  let option_page_nav = generate_option_page_sidebar_nav(pages, page, &links);
  template::render_options_body(
    config,
    root_path,
    &format!("{} Options: {}", config.title, page.title),
    &format!("Options: {}", page.title),
    &options_html,
    &options_toc,
    OptionsSidebarHtml {
      option_groups_toc: Some(&option_groups_toc),
      option_page_nav:   Some(&option_page_nav),
    },
  )
}

/// Render the split options index page.
///
/// # Errors
///
/// Returns an error if the options template cannot be rendered.
pub fn render_options_index(
  config: &Config,
  page_set: &OptionPageSet,
  processor: &MarkdownProcessor,
) -> Result<String> {
  let links = PageLinks::new(config, Path::new("options.html"));
  let mut options_html = generate_options_index_html(&page_set.pages, &links);
  if !page_set.root_options.is_empty() {
    options_html.push_str(&template::generate_options_html(
      &page_set.root_options,
      config,
      processor,
    ));
  }
  let toc_html = generate_option_pages_toc(&page_set.pages, None, &links);
  template::render_options_body(
    config,
    Path::new("options.html"),
    &format!("{} Options", config.title),
    "Options",
    &options_html,
    &toc_html,
    OptionsSidebarHtml::default(),
  )
}

fn generate_option_page_header(page: &OptionPage, links: &PageLinks) -> String {
  let index_path = links.href("options.html");
  let index_href = html_escape::encode_double_quoted_attribute(&index_path);
  let prefix = encode_text(&page.prefix);
  let count = page.options.len();
  let noun = if count == 1 { "option" } else { "options" };

  format!(
    "<nav class=\"option-page-breadcrumb\" aria-label=\"Option \
     navigation\">\n  <a href=\"{index_href}\">Option groups</a>\n  \
     <span>{prefix}</span>\n</nav>\n<div \
     class=\"option-page-meta\"><code>{prefix}</code><span>{count} \
     {noun}</span></div>\n"
  )
}

fn generate_option_page_sidebar_nav(
  pages: &[OptionPage],
  current: &OptionPage,
  links: &PageLinks,
) -> String {
  let Some(index) = pages.iter().position(|page| page.path == current.path)
  else {
    return String::new();
  };
  let previous = index.checked_sub(1).and_then(|idx| pages.get(idx));
  let next = pages.get(index + 1);
  if previous.is_none() && next.is_none() {
    return String::new();
  }

  let mut html = String::new();
  html.push_str(
    "<nav class=\"option-page-sidebar-nav\" aria-label=\"Adjacent option \
     groups\">\n",
  );
  html.push_str(&generate_option_page_sidebar_link(
    previous, links, "Previous", "prev",
  ));
  html.push_str(&generate_option_page_sidebar_link(
    next, links, "Next", "next",
  ));
  html.push_str("</nav>\n");
  html
}

fn generate_option_page_sidebar_link(
  page: Option<&OptionPage>,
  links: &PageLinks,
  label: &str,
  rel: &str,
) -> String {
  let Some(page) = page else {
    return format!(
      "  <span class=\"option-page-sidebar-link option-page-sidebar-disabled \
       option-page-sidebar-{rel}\"></span>\n"
    );
  };

  let href_path = links.href(&page.path);
  let href = html_escape::encode_double_quoted_attribute(&href_path);
  let title = encode_text(&page.title);

  format!(
    "  <a class=\"option-page-sidebar-link option-page-sidebar-{rel}\" \
     href=\"{href}\" \
     rel=\"{rel}\"><span>{label}</span><strong>{title}</strong></a>\n"
  )
}

fn generate_options_index_html(
  pages: &[OptionPage],
  links: &PageLinks,
) -> String {
  let mut html = String::new();
  html.push_str("<section class=\"options-index\">\n");

  if pages.is_empty() {
    html.push_str(
      "  <p class=\"options-index-empty\">No option groups were \
       generated.</p>\n",
    );
    html.push_str("</section>\n");
    return html;
  }

  let total_options: usize = pages.iter().map(|page| page.options.len()).sum();
  let noun = if total_options == 1 {
    "option"
  } else {
    "options"
  };
  let _ = writeln!(
    html,
    "  <div class=\"options-index-summary\"><strong>{}</strong> option groups \
     covering <strong>{}</strong> {noun}</div>",
    pages.len(),
    total_options
  );
  html.push_str(
    "  <nav class=\"options-index-list\" aria-label=\"Option groups\">\n",
  );

  for page in pages {
    let href_path = links.href(&page.path);
    let href = html_escape::encode_double_quoted_attribute(&href_path);
    let title = encode_text(&page.title);
    let prefix = encode_text(&page.prefix);
    let count = page.options.len();
    let noun = if count == 1 { "option" } else { "options" };
    let _ = writeln!(
      html,
      "    <a class=\"option-page-row\" href=\"{href}\">\n      <span \
       class=\"option-page-main\">\n        <span \
       class=\"option-page-title\">{title}</span>\n        \
       <code>{prefix}</code>\n      </span>\n      <span \
       class=\"option-page-count\">{count} {noun}</span>\n    </a>"
    );
  }
  html.push_str("  </nav>\n");
  html.push_str("</section>\n");
  html
}

fn generate_option_pages_toc(
  pages: &[OptionPage],
  current_path: Option<&str>,
  links: &PageLinks,
) -> String {
  let mut tree = TocTree::default();
  for page in pages {
    tree.insert(
      &page
        .prefix
        .split('.')
        .map(|part| {
          let label = part.to_string();
          TocPathPart {
            key:   label.clone(),
            label: label.clone(),
            title: label,
          }
        })
        .collect::<Vec<_>>(),
      option_page_leaf_html(page, current_path, links),
      current_path == Some(page.path.as_str()),
    );
  }
  tree.render(2)
}

fn option_page_leaf_html(
  page: &OptionPage,
  current_path: Option<&str>,
  links: &PageLinks,
) -> String {
  let href_path = links.href(&page.path);
  let href = html_escape::encode_double_quoted_attribute(&href_path);
  let title = encode_text(&page.title);
  let prefix = encode_text(&page.prefix);
  let active = if current_path == Some(page.path.as_str()) {
    " class=\"active\""
  } else {
    ""
  };
  format!("<a{active} href=\"{href}\" title=\"{prefix}\">{title}</a>")
}
