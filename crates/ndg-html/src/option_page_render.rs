use std::{fmt::Write, path::Path};

use color_eyre::eyre::Result;
use html_escape::encode_text;
use ndg_config::Config;
use ndg_utils::html::calculate_root_relative_path;

use crate::{
  option_pages::{OptionPage, OptionPageSet},
  template::{self, OptionsSidebarHtml},
  toc_tree::{TocPathPart, TocTree},
};

/// Render one generated option group page.
///
/// # Errors
///
/// Returns an error if the options template or TOC template cannot be rendered.
pub fn render_option_page(
  config: &Config,
  page: &OptionPage,
  pages: &[OptionPage],
) -> Result<String> {
  let root_path = Path::new(&page.path);
  let mut options_html = generate_option_page_header(page, root_path);
  options_html
    .push_str(&template::generate_options_html(&page.options, config));
  let options_toc = template::render_options_toc(config, &page.options)?;
  let option_groups_toc =
    generate_option_pages_toc(pages, Some(&page.path), root_path);
  let option_page_nav =
    generate_option_page_sidebar_nav(pages, page, root_path);
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
) -> Result<String> {
  let mut options_html = generate_options_index_html(&page_set.pages);
  if !page_set.root_options.is_empty() {
    options_html.push_str(&template::generate_options_html(
      &page_set.root_options,
      config,
    ));
  }
  let toc_html = generate_options_index_toc(&page_set.pages);
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

fn generate_option_page_header(page: &OptionPage, root_path: &Path) -> String {
  let root_prefix = calculate_root_relative_path(root_path);
  let index_path = format!("{root_prefix}options.html");
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
  root_path: &Path,
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
    previous, root_path, "Previous", "prev",
  ));
  html.push_str(&generate_option_page_sidebar_link(
    next, root_path, "Next", "next",
  ));
  html.push_str("</nav>\n");
  html
}

fn generate_option_page_sidebar_link(
  page: Option<&OptionPage>,
  root_path: &Path,
  label: &str,
  rel: &str,
) -> String {
  let Some(page) = page else {
    return format!(
      "  <span class=\"option-page-sidebar-link option-page-sidebar-disabled \
       option-page-sidebar-{rel}\"></span>\n"
    );
  };

  let root_prefix = calculate_root_relative_path(root_path);
  let href_path = format!("{root_prefix}{}", page.path);
  let href = html_escape::encode_double_quoted_attribute(&href_path);
  let title = encode_text(&page.title);

  format!(
    "  <a class=\"option-page-sidebar-link option-page-sidebar-{rel}\" \
     href=\"{href}\" \
     rel=\"{rel}\"><span>{label}</span><strong>{title}</strong></a>\n"
  )
}

fn generate_options_index_html(pages: &[OptionPage]) -> String {
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
    let href = html_escape::encode_double_quoted_attribute(&page.path);
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

fn generate_options_index_toc(pages: &[OptionPage]) -> String {
  generate_option_pages_toc(pages, None, Path::new("options.html"))
}

fn generate_option_pages_toc(
  pages: &[OptionPage],
  current_path: Option<&str>,
  root_path: &Path,
) -> String {
  let root_prefix = calculate_root_relative_path(root_path);
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
      option_page_leaf_html(page, current_path, &root_prefix),
      current_path == Some(page.path.as_str()),
    );
  }
  tree.render(2)
}

fn option_page_leaf_html(
  page: &OptionPage,
  current_path: Option<&str>,
  root_prefix: &str,
) -> String {
  let href_path = format!("{root_prefix}{}", page.path);
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
