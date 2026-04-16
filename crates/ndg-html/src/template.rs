use std::{
  collections::HashMap,
  fmt::Write,
  fs,
  path::Path,
  sync::{LazyLock, RwLock},
};

use color_eyre::eyre::{Context, Result, bail};
use html_escape::encode_text;
use indexmap::IndexMap;
use ndg_commonmark::Header;
use ndg_config::{Config, sidebar::SidebarOrdering};
use ndg_manpage::types::NixOption;
use ndg_templates as templates;
use ndg_utils::{
  html::{calculate_root_relative_path, generate_asset_paths},
  markdown::PageFrontmatter,
};
use serde_json::Value;
use tera::Tera;

const DEFAULT_TEMPLATE: &str = templates::DEFAULT_TEMPLATE;
const OPTIONS_TEMPLATE: &str = templates::OPTIONS_TEMPLATE;
const SEARCH_TEMPLATE: &str = templates::SEARCH_TEMPLATE;
const OPTIONS_TOC_TEMPLATE: &str = templates::OPTIONS_TOC_TEMPLATE;
const NAVBAR_TEMPLATE: &str = templates::NAVBAR_TEMPLATE;
const FOOTER_TEMPLATE: &str = templates::FOOTER_TEMPLATE;
const LIB_TEMPLATE: &str = templates::LIB_TEMPLATE;

/// Per-page frontmatter data exposed to Tera templates as the `page` variable.
#[derive(serde::Serialize)]
struct PageContext {
  title:       Option<String>,
  description: Option<String>,
  author:      Option<String>,
  template:    Option<String>,
  toc:         Option<bool>,
  extra:       serde_json::Value,
}

impl PageContext {
  fn from_frontmatter(fm: Option<&PageFrontmatter>) -> Self {
    let Some(fm) = fm else {
      return Self {
        title:       None,
        description: None,
        author:      None,
        template:    None,
        toc:         None,
        extra:       serde_json::Value::Null,
      };
    };
    let extra = fm
      .extra
      .as_ref()
      .and_then(|t| serde_json::to_value(t).ok())
      .unwrap_or(serde_json::Value::Null);
    Self {
      title: fm.title.clone(),
      description: fm.description.clone(),
      author: fm.author.clone(),
      template: fm.template.clone(),
      toc: fm.toc,
      extra,
    }
  }
}

/// Cache for template content strings to avoid repeated disk I/O
static TEMPLATE_CONTENT_CACHE: LazyLock<RwLock<HashMap<String, String>>> =
  LazyLock::new(|| RwLock::new(HashMap::new()));

/// Cache for compiled Tera instances keyed by template configuration
/// This eliminates the overhead of re-parsing templates on every render
static TERA_CACHE: LazyLock<RwLock<HashMap<String, Tera>>> =
  LazyLock::new(|| RwLock::new(HashMap::new()));

/// Setup Tera templates for a specific page type, using caching for
/// performance.
///
/// # Arguments
///
/// * `config` - Site configuration
/// * `main_template_name` - Name of the main template (e.g., "default",
///   "options", "lib", "search")
/// * `main_template_content` - Content of the main template
///
/// # Returns
///
/// A cached or newly created Tera instance with all required templates loaded
///
/// # Panics
///
/// Panics if the internal template cache `RwLock` is poisoned.
#[allow(
  clippy::expect_used,
  reason = "RwLock poisoning is unrecoverable; panic is the correct response"
)]
fn setup_tera_templates(
  config: &Config,
  main_template_name: &str,
  main_template_content: &str,
) -> Result<Tera> {
  // Create a cache key based on template configuration and main template.
  // Use both template_dir and template_path to ensure uniqueness even when
  // only template_path (a file) is set - get_template_path() returns None
  // for file-only templates, causing cache collisions.
  let template_key = config
    .template_dir
    .as_ref()
    .or(config.template_path.as_ref())
    .map_or_else(|| "default".to_string(), |p| p.display().to_string());
  let cache_key = format!("{template_key}:{main_template_name}");

  // Check cache first
  {
    let cache = TERA_CACHE.read().expect("tera cache lock not poisoned");
    if let Some(tera) = cache.get(&cache_key) {
      return Ok(tera.clone());
    }
  }

  // Create new Tera instance and load templates
  let mut tera = Tera::default();
  tera.add_raw_template(main_template_name, main_template_content)?;

  // Load shared templates
  let navbar_content =
    get_template_content(config, "navbar.html", NAVBAR_TEMPLATE)?;
  tera.add_raw_template("navbar", &navbar_content)?;

  let footer_content =
    get_template_content(config, "footer.html", FOOTER_TEMPLATE)?;
  tera.add_raw_template("footer", &footer_content)?;

  // Cache the compiled Tera instance
  {
    let mut cache = TERA_CACHE.write().expect("tera cache lock not poisoned");
    cache.insert(cache_key, tera.clone());
  }

  Ok(tera)
}

/// Build the common Tera context with asset paths and navigation data.
///
/// # Arguments
///
/// * `config` - Site configuration
/// * `asset_paths` - Map of asset paths for the current page
/// * `root_prefix` - Root-relative path prefix
/// * `has_options` - CSS class or style for options nav item
///
/// # Returns
///
/// A Tera context populated with common fields
fn build_common_context(
  config: &Config,
  asset_paths: &HashMap<&'static str, String>,
  root_prefix: &str,
  has_options: &str,
) -> tera::Context {
  let mut ctx = tera::Context::new();

  // Insert user-defined variables first so built-in variables always win.
  for (key, value) in &config.vars {
    ctx.insert(key.as_str(), value);
  }

  ctx.insert("site_title", &config.title);
  ctx.insert("footer_text", &config.footer_text);
  ctx.insert("has_options", has_options);
  ctx.insert("generate_search", &config.is_search_enabled());
  ctx.insert("root_prefix", root_prefix);

  // Insert asset paths with fallbacks
  ctx.insert(
    "stylesheet_path",
    asset_paths
      .get("stylesheet_path")
      .map_or("assets/style.css", String::as_str),
  );
  ctx.insert(
    "main_js_path",
    asset_paths
      .get("main_js_path")
      .map_or("assets/main.js", String::as_str),
  );
  ctx.insert(
    "search_js_path",
    asset_paths
      .get("search_js_path")
      .map_or("assets/search.js", String::as_str),
  );
  ctx.insert(
    "index_path",
    asset_paths
      .get("index_path")
      .map_or("index.html", String::as_str),
  );
  ctx.insert(
    "options_path",
    asset_paths
      .get("options_path")
      .map_or("options.html", String::as_str),
  );
  ctx.insert(
    "search_path",
    asset_paths
      .get("search_path")
      .map_or("search.html", String::as_str),
  );

  // Build favicon HTML for the template head.
  let favicon_links: Vec<String> = if let Some(ref meta) = config.meta {
    meta
      .favicon
      .iter()
      .filter_map(|entry| {
        let name = entry
          .dest
          .as_ref()
          .map(|p| p.as_os_str())
          .or(entry.href.file_name())
          .and_then(|n| n.to_str())?;
        let href = format!(
          "{root_prefix}{}",
          html_escape::encode_double_quoted_attribute(name)
        );
        let rel = html_escape::encode_double_quoted_attribute(&entry.rel);

        let mut tag = format!(r#"<link rel="{rel}" href="{href}""#);
        if let Some(ref mime_type) = entry.mime_type {
          tag.push_str(&format!(
            r#" type="{}""#,
            html_escape::encode_double_quoted_attribute(mime_type)
          ));
        }
        if let Some(ref sizes) = entry.sizes {
          tag.push_str(&format!(
            r#" sizes="{}""#,
            html_escape::encode_double_quoted_attribute(sizes)
          ));
        }
        tag.push_str(" />");
        Some(tag)
      })
      .collect()
  } else {
    Vec::new()
  };

  ctx.insert("favicon_links", &favicon_links.join("\n    "));

  ctx
}

/// Render navbar and footer templates with consistent context.
///
/// # Arguments
///
/// * `tera` - Tera template engine instance
/// * `config` - Site configuration
/// * `asset_paths` - Map of asset paths
/// * `has_options` - CSS class or style for options nav item
///
/// # Returns
///
/// Tuple of (`navbar_html`, `footer_html`)
fn render_navbar_footer(
  tera: &Tera,
  config: &Config,
  asset_paths: &HashMap<&'static str, String>,
  has_options: &str,
) -> Result<(String, String)> {
  // Render navbar
  let mut navbar_ctx = tera::Context::new();
  for (key, value) in &config.vars {
    navbar_ctx.insert(key.as_str(), value);
  }
  navbar_ctx.insert("has_options", has_options);
  navbar_ctx.insert("generate_search", &config.is_search_enabled());
  navbar_ctx.insert(
    "options_path",
    asset_paths
      .get("options_path")
      .map_or("options.html", String::as_str),
  );
  navbar_ctx.insert(
    "search_path",
    asset_paths
      .get("search_path")
      .map_or("search.html", String::as_str),
  );
  let navbar_html = tera.render("navbar", &navbar_ctx)?;

  // Render footer
  let mut footer_ctx = tera::Context::new();
  for (key, value) in &config.vars {
    footer_ctx.insert(key.as_str(), value);
  }
  footer_ctx.insert("footer_text", &config.footer_text);
  let footer_html = tera.render("footer", &footer_ctx)?;

  Ok((navbar_html, footer_html))
}

/// Generate HTML for meta tags, merging site-wide config with per-page
/// frontmatter.
///
/// Per-page `description` and `author` from frontmatter take precedence over
/// the corresponding keys in the global `meta.tags` config.
///
/// # Arguments
///
/// * `config` - Site configuration
/// * `frontmatter` - Optional per-page frontmatter
///
/// # Returns
///
/// HTML string of meta tags, or empty string if none are configured
fn generate_meta_tags_html(
  config: &Config,
  frontmatter: Option<&PageFrontmatter>,
) -> String {
  let mut merged: std::collections::HashMap<String, String> =
    get_meta_tags(config).cloned().unwrap_or_default();

  if let Some(fm) = frontmatter {
    if let Some(ref desc) = fm.description {
      merged.insert("description".to_owned(), desc.clone());
    }
    if let Some(ref author) = fm.author {
      merged.insert("author".to_owned(), author.clone());
    }
  }

  if merged.is_empty() {
    return String::new();
  }

  let mut pairs: Vec<_> = merged.iter().collect();
  pairs.sort_by_key(|(k, _)| k.as_str());
  pairs
    .into_iter()
    .map(|(k, v)| {
      format!(
        "<meta name=\"{}\" content=\"{}\" />",
        encode_text(k),
        encode_text(v)
      )
    })
    .collect::<Vec<_>>()
    .join("\n    ")
}

/// Render a documentation page
///
/// # Arguments
///
/// * `config` - Site configuration
/// * `content` - Rendered HTML content for the page body
/// * `title` - Page title (may be overridden by `frontmatter.title`)
/// * `headers` - Extracted headings used to build the table of contents
/// * `rel_path` - Output path relative to `output_dir` (e.g. `foo/bar.html`)
/// * `frontmatter` - Optional per-page frontmatter parsed from the source
///   markdown file
///
/// # Errors
///
/// Returns an error if the template cannot be rendered.
pub fn render(
  config: &Config,
  content: &str,
  title: &str,
  headers: &[Header],
  rel_path: &Path,
  frontmatter: Option<&PageFrontmatter>,
) -> Result<String> {
  // Effective title: frontmatter wins over the caller-supplied value.
  let effective_title = frontmatter
    .and_then(|fm| fm.title.as_deref())
    .unwrap_or(title);

  // Resolve which template to use and its Tera-internal name.
  // Priority: frontmatter.template > file-stem specific > default.html
  let (template_name, template_content) =
    resolve_doc_template(config, rel_path, frontmatter)?;

  // Setup Tera with caching, keyed on the effective template name.
  let tera = setup_tera_templates(config, &template_name, &template_content)?;

  // Table of contents .
  // Suppressed when frontmatter sets `toc = false`.
  let toc = if frontmatter.is_none_or(|fm| fm.toc != Some(false)) {
    generate_toc(headers)
  } else {
    String::new()
  };

  let doc_nav = generate_doc_nav(config, rel_path);

  // Check if options are available
  let has_options = if config.module_options.is_some() {
    ""
  } else {
    "style=\"display:none;\""
  };

  let custom_scripts = generate_custom_scripts(config, rel_path)?;
  let asset_paths = generate_asset_paths(rel_path);
  let root_prefix = calculate_root_relative_path(rel_path);

  // Generate meta tags (per-page frontmatter overrides global config).
  let meta_tags_html = generate_meta_tags_html(config, frontmatter);
  let opengraph_html = build_opengraph_html(config);

  // Render navbar and footer using helper
  let (navbar_html, footer_html) =
    render_navbar_footer(&tera, config, &asset_paths, has_options)?;

  // Build common context and add page-specific fields
  let mut tera_context =
    build_common_context(config, &asset_paths, &root_prefix, has_options);
  tera_context.insert("content", content);
  tera_context.insert("title", effective_title);
  tera_context.insert("navbar_html", &navbar_html);
  tera_context.insert("footer_html", &footer_html);
  tera_context.insert("toc", &toc);
  tera_context.insert("doc_nav", &doc_nav);
  tera_context.insert("custom_scripts", &custom_scripts);
  tera_context.insert("meta_tags_html", &meta_tags_html);
  tera_context.insert("opengraph_html", &opengraph_html);
  tera_context.insert("page", &PageContext::from_frontmatter(frontmatter));

  // Render the template
  let html = tera.render(&template_name, &tera_context)?;
  Ok(html)
}

/// Resolve the template name and content for a documentation page.
///
/// Priority: `frontmatter.template` → file-stem specific template →
/// `default.html`.
///
/// # Returns
///
/// A `(tera_name, content)` pair where `tera_name` is both the key
/// used in the Tera instance and in the template cache.
fn resolve_doc_template(
  config: &Config,
  rel_path: &Path,
  frontmatter: Option<&PageFrontmatter>,
) -> Result<(String, String)> {
  // Frontmatter-specified template takes highest priority.
  if let Some(fm) = frontmatter
    && let Some(ref tmpl) = fm.template
  {
    let lc = tmpl.to_ascii_lowercase();
    let (file_name, tera_name) = if lc.ends_with(".html") {
      (tmpl.clone(), tmpl[..tmpl.len() - 5].to_owned())
    } else {
      (format!("{tmpl}.html"), tmpl.clone())
    };
    let content = get_template_content(config, &file_name, "")?;
    if !content.is_empty() {
      return Ok((tera_name, content));
    }
    log::warn!(
      "Frontmatter template '{file_name}' not found; falling back to default \
       template resolution"
    );
  }

  // File-stem specific template (e.g. foo.html for foo.md).
  let file_stem = rel_path
    .file_stem()
    .and_then(|s| s.to_str())
    .unwrap_or("default");
  let specific_name = format!("{file_stem}.html");
  let specific_content = get_template_content(config, &specific_name, "")?;
  if !specific_content.is_empty() {
    return Ok((file_stem.to_owned(), specific_content));
  }

  // Default template.
  let content = get_template_content(config, "default.html", DEFAULT_TEMPLATE)?;
  Ok(("default".to_owned(), content))
}

/// Render `NixOS` module options page
///
/// # Errors
///
/// Returns an error if the options template or any required template cannot be
/// rendered or written.
///
/// # Panics
///
/// Panics if an internal template or content cache `RwLock` is poisoned.
#[allow(
  clippy::implicit_hasher,
  reason = "Standard HashMap sufficient for this use case"
)]
#[allow(
  clippy::expect_used,
  reason = "RwLock poisoning is unrecoverable; panic is the correct response"
)]
pub fn render_options(
  config: &Config,
  options: &IndexMap<String, NixOption>,
) -> Result<String> {
  // Load templates with caching
  let options_template =
    get_template_content(config, "options.html", OPTIONS_TEMPLATE)?;
  let mut tera = setup_tera_templates(config, "options", &options_template)?;

  // Load additional template for options TOC
  let options_toc_template =
    get_template_content(config, "options_toc.html", OPTIONS_TOC_TEMPLATE)?;
  tera.add_raw_template("options_toc", &options_toc_template)?;

  // Update the cache with the modified Tera instance
  let cache_key = format!(
    "{}:options",
    config
      .get_template_path()
      .map_or_else(|| "default".to_string(), |p| p.display().to_string())
  );
  {
    let mut cache = TERA_CACHE.write().expect("tera cache lock not poisoned");
    cache.insert(cache_key, tera.clone());
  }

  // Create options HTML
  let options_html = generate_options_html(options);

  // Generate options TOC using Tera templating
  let options_toc = generate_options_toc(options, config, &tera)?;

  // Generate document navigation and paths
  let root_path = Path::new("options.html");
  let doc_nav = generate_doc_nav(config, root_path);
  let custom_scripts = generate_custom_scripts(config, root_path)?;
  let asset_paths = generate_asset_paths(root_path);
  let root_prefix = calculate_root_relative_path(root_path);

  // Render navbar and footer using helper
  let (navbar_html, footer_html) =
    render_navbar_footer(&tera, config, &asset_paths, "class=\"active\"")?;

  // Build common context and add page-specific fields
  let mut tera_context = build_common_context(
    config,
    &asset_paths,
    &root_prefix,
    "class=\"active\"",
  );
  tera_context.insert("title", &format!("{} - Options", config.title));
  tera_context.insert("heading", &format!("{} Options", config.title));
  tera_context.insert("options", &options_html);
  tera_context.insert("navbar_html", &navbar_html);
  tera_context.insert("footer_html", &footer_html);
  tera_context.insert("custom_scripts", &custom_scripts);
  tera_context.insert("doc_nav", &doc_nav);
  tera_context.insert("toc", &options_toc);

  // Add meta and opengraph tags
  let meta_tags_html = generate_meta_tags_html(config, None);
  let opengraph_html = build_opengraph_html(config);
  tera_context.insert("meta_tags_html", &meta_tags_html);
  tera_context.insert("opengraph_html", &opengraph_html);

  // Render the template
  let html = tera.render("options", &tera_context)?;
  Ok(html)
}

/// Render the library reference page from pre-built HTML strings.
///
/// # Arguments
///
/// * `config` - Site configuration
/// * `entries_html` - Pre-rendered HTML for all library entries
/// * `toc_html` - Pre-rendered TOC HTML for the library page
///
/// # Errors
///
/// Returns an error if template rendering fails.
pub fn render_lib(
  config: &Config,
  entries_html: &str,
  toc_html: &str,
) -> Result<String> {
  // Setup Tera with caching
  let lib_template = get_template_content(config, "lib.html", LIB_TEMPLATE)?;
  let tera = setup_tera_templates(config, "lib", &lib_template)?;

  // Generate navigation and paths
  let root_path = Path::new("lib.html");
  let doc_nav = generate_doc_nav(config, root_path);
  let custom_scripts = generate_custom_scripts(config, root_path)?;
  let asset_paths = generate_asset_paths(root_path);
  let root_prefix = calculate_root_relative_path(root_path);

  // Determine has_options value
  let has_options = if config.module_options.is_some() {
    ""
  } else {
    "style=\"display:none;\""
  };

  // Render navbar and footer using helper
  let (navbar_html, footer_html) =
    render_navbar_footer(&tera, config, &asset_paths, has_options)?;

  // Build common context and add page-specific fields
  let mut tera_context =
    build_common_context(config, &asset_paths, &root_prefix, has_options);
  tera_context
    .insert("title", &format!("{} - Library Reference", config.title));
  tera_context
    .insert("heading", &format!("{} Library Reference", config.title));
  tera_context.insert("entries", entries_html);
  tera_context.insert("toc", toc_html);
  tera_context.insert("navbar_html", &navbar_html);
  tera_context.insert("footer_html", &footer_html);
  tera_context.insert("custom_scripts", &custom_scripts);
  tera_context.insert("doc_nav", &doc_nav);

  // Add meta and opengraph tags
  let meta_tags_html = generate_meta_tags_html(config, None);
  let opengraph_html = build_opengraph_html(config);
  tera_context.insert("meta_tags_html", &meta_tags_html);
  tera_context.insert("opengraph_html", &opengraph_html);

  let html = tera.render("lib", &tera_context)?;
  Ok(html)
}

/// Generate specialized TOC for options page
fn generate_options_toc(
  options: &IndexMap<String, NixOption>,
  config: &Config,
  tera: &Tera,
) -> Result<String> {
  // Get depth from sidebar.options config or fallback to legacy
  // options_toc_depth
  let default_depth = config
    .sidebar
    .as_ref()
    .and_then(|s| s.options.as_ref())
    .map_or(config.options_toc_depth, |o| o.depth);

  let mut grouped_options: HashMap<String, Vec<&NixOption>> = HashMap::new();
  let mut direct_parent_options: HashMap<String, &NixOption> = HashMap::new();
  let mut option_custom_names: HashMap<String, String> = HashMap::new();
  let mut option_positions: HashMap<String, usize> = HashMap::new();

  for option in options.values() {
    // Check if this option has a matching rule in sidebar.options config
    let match_result = config
      .sidebar
      .as_ref()
      .and_then(|s| s.options.as_ref())
      .and_then(|o| o.find_match(&option.name));

    // Skip if option is marked as hidden
    if let Some(matched) = &match_result {
      if matched.is_hidden() {
        continue;
      }

      // Store custom name if provided
      if let Some(name) = matched.get_name() {
        option_custom_names.insert(option.name.clone(), name.to_string());
      }

      // Store custom position if provided
      if let Some(position) = matched.get_position() {
        option_positions.insert(option.name.clone(), position);
      }
    }

    // Use custom depth if specified, otherwise use default
    let depth = match_result
      .as_ref()
      .and_then(|m| m.get_depth())
      .unwrap_or(default_depth);

    let parent = get_option_parent(&option.name, depth);

    // Check if this option exactly matches its parent category
    if option.name == parent {
      direct_parent_options.insert(parent.clone(), option);
    }

    // Add to grouped options
    grouped_options.entry(parent).or_default().push(option);
  }

  // Separate categories into single options and dropdown categories
  let mut single_options: Vec<tera::Value> = Vec::new();
  let mut dropdown_categories: Vec<tera::Value> = Vec::new();

  for (parent, opts) in &grouped_options {
    let has_multiple_options = opts.len() > 1;
    let has_child_options =
      opts.len() > usize::from(direct_parent_options.contains_key(parent));

    if !has_multiple_options && !has_child_options {
      // Single option with no children
      let option = opts[0];

      // Use custom name if available, otherwise use option name
      #[allow(clippy::map_unwrap_or)]
      let display_name = option_custom_names
        .get(&option.name)
        .map(String::as_str)
        .unwrap_or(&option.name);

      let option_value = tera::to_value({
        let mut map = tera::Map::new();
        // HTML-escape display strings: Tera::default() has no auto-escaping.
        map.insert(
          "name".to_string(),
          tera::to_value(encode_text(&option.name).as_ref())?,
        );
        map.insert(
          "id".to_string(),
          tera::to_value(sanitize_option_id(&option.name))?,
        );
        map.insert(
          "display_name".to_string(),
          tera::to_value(encode_text(display_name).as_ref())?,
        );
        map.insert("internal".to_string(), tera::to_value(option.internal)?);
        map.insert("read_only".to_string(), tera::to_value(option.read_only)?);

        // Add position if custom position is set
        if let Some(position) = option_positions.get(&option.name) {
          map.insert("position".to_string(), tera::to_value(position)?);
        }

        map
      })?;
      single_options.push(option_value);
    } else {
      // Category with multiple options or child options
      let mut category = tera::Map::new();

      // Use custom name for category if the parent option has one
      #[allow(clippy::map_unwrap_or)]
      let category_display_name = option_custom_names
        .get(parent)
        .map(String::as_str)
        .unwrap_or(parent);

      category.insert(
        "name".to_string(),
        tera::to_value(encode_text(parent).as_ref())?,
      );
      category.insert(
        "display_name".to_string(),
        tera::to_value(encode_text(category_display_name).as_ref())?,
      );
      category.insert("count".to_string(), tera::to_value(opts.len())?);

      // Add parent option if it exists
      if let Some(parent_option) = direct_parent_options.get(parent) {
        let parent_option_value = tera::to_value({
          let mut map = tera::Map::new();
          map.insert(
            "name".to_string(),
            tera::to_value(encode_text(&parent_option.name).as_ref())?,
          );
          map.insert(
            "id".to_string(),
            tera::to_value(sanitize_option_id(&parent_option.name))?,
          );
          map.insert(
            "internal".to_string(),
            tera::to_value(parent_option.internal)?,
          );
          map.insert(
            "read_only".to_string(),
            tera::to_value(parent_option.read_only)?,
          );
          map
        })?;
        category.insert("parent_option".to_string(), parent_option_value);
      }

      // Add child options
      let mut children = Vec::new();
      let mut child_options: Vec<&NixOption> = opts
        .iter()
        .filter(|opt| opt.name != *parent)
        .copied()
        .collect();

      // Sort by suffix
      child_options.sort_by(|a, b| {
        let a_suffix = a
          .name
          .strip_prefix(&format!("{parent}."))
          .unwrap_or(&a.name);
        let b_suffix = b
          .name
          .strip_prefix(&format!("{parent}."))
          .unwrap_or(&b.name);
        a_suffix.cmp(b_suffix)
      });

      for option in child_options {
        let display_name = option
          .name
          .strip_prefix(&format!("{parent}."))
          .unwrap_or(&option.name);

        let child_value = tera::to_value({
          let mut map = tera::Map::new();
          map.insert(
            "name".to_string(),
            tera::to_value(encode_text(&option.name).as_ref())?,
          );
          map.insert(
            "id".to_string(),
            tera::to_value(sanitize_option_id(&option.name))?,
          );
          map.insert(
            "display_name".to_string(),
            tera::to_value(encode_text(display_name).as_ref())?,
          );
          map.insert("internal".to_string(), tera::to_value(option.internal)?);
          map
            .insert("read_only".to_string(), tera::to_value(option.read_only)?);
          map
        })?;
        children.push(child_value);
      }

      category.insert("children".to_string(), tera::to_value(children)?);

      // Add position if custom position is set for parent
      if let Some(position) = option_positions.get(parent) {
        category.insert("position".to_string(), tera::to_value(position)?);
      }

      dropdown_categories.push(tera::to_value(category)?);
    }
  }

  // Sort single options - by position first if available, then alphabetically
  single_options.sort_by(|a, b| {
    let a_name = a.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let b_name = b.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let a_position = a.get("position").and_then(Value::as_u64);
    let b_position = b.get("position").and_then(Value::as_u64);

    match (a_position, b_position) {
      (Some(a_pos), Some(b_pos)) => a_pos.cmp(&b_pos),
      (Some(_), None) => std::cmp::Ordering::Less,
      (None, Some(_)) => std::cmp::Ordering::Greater,
      (None, None) => a_name.cmp(b_name),
    }
  });

  // Sort dropdown categories - by position first if available, then by
  // component count and alphabetically
  dropdown_categories.sort_by(|a, b| {
    let a_name = a.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let b_name = b.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let a_position = a.get("position").and_then(Value::as_u64);
    let b_position = b.get("position").and_then(Value::as_u64);

    match (a_position, b_position) {
      (Some(a_pos), Some(b_pos)) => a_pos.cmp(&b_pos),
      (Some(_), None) => std::cmp::Ordering::Less,
      (None, Some(_)) => std::cmp::Ordering::Greater,
      (None, None) => {
        let a_components = a_name.split('.').count();
        let b_components = b_name.split('.').count();

        // Sort by component count first
        match a_components.cmp(&b_components) {
          std::cmp::Ordering::Equal => a_name.cmp(b_name), // then alphabetically
          other => other,
        }
      }
    }
  });

  let mut tera_context = tera::Context::new();
  tera_context.insert("single_options", &single_options);
  tera_context.insert("dropdown_categories", &dropdown_categories);

  // Render the template
  let rendered = tera.render("options_toc", &tera_context)?;

  Ok(rendered)
}

/// Extract the parent category from an option name with configurable depth
fn get_option_parent(option_name: &str, depth: usize) -> String {
  let parts: Vec<&str> = option_name.split('.').collect();
  if parts.len() <= depth {
    option_name.to_string()
  } else {
    parts[..depth].join(".")
  }
}

/// Render search page
///
/// # Errors
///
/// Returns an error if the search template or any required template cannot be
/// rendered or written.
#[allow(
  clippy::implicit_hasher,
  reason = "Standard HashMap sufficient for this use case"
)]
pub fn render_search(
  config: &Config,
  context: &HashMap<&str, String>,
) -> Result<String> {
  // Skip rendering if search is disabled
  if !config.is_search_enabled() {
    bail!("Search functionality is disabled");
  }

  // Setup Tera with caching
  let search_template =
    get_template_content(config, "search.html", SEARCH_TEMPLATE)?;
  let tera = setup_tera_templates(config, "search", &search_template)?;

  let title_str = context
    .get("title")
    .cloned()
    .unwrap_or_else(|| format!("{} - Search", config.title));

  // Generate navigation and paths (search page is always at root)
  let root_path = Path::new("search.html");
  let doc_nav = generate_doc_nav(config, root_path);
  let custom_scripts = generate_custom_scripts(config, root_path)?;

  // Check if options are available
  let has_options = if config.module_options.is_some() {
    ""
  } else {
    "style=\"display:none;\""
  };

  // Generate asset and navigation paths
  let asset_paths = ndg_utils::html::generate_asset_paths(root_path);
  let root_prefix = ndg_utils::html::calculate_root_relative_path(root_path);

  // Render navbar and footer using helper
  let (navbar_html, footer_html) =
    render_navbar_footer(&tera, config, &asset_paths, has_options)?;

  // Build common context and add page-specific fields
  let mut tera_context =
    build_common_context(config, &asset_paths, &root_prefix, has_options);
  tera_context.insert("title", &title_str);
  tera_context.insert("heading", "Search");
  tera_context.insert("navbar_html", &navbar_html);
  tera_context.insert("footer_html", &footer_html);
  tera_context.insert("custom_scripts", &custom_scripts);
  tera_context.insert("doc_nav", &doc_nav);
  tera_context.insert("toc", ""); // no TOC for search page
  tera_context.insert("generate_search", &true); // always true for search page

  // Add meta and opengraph tags
  let meta_tags_html = generate_meta_tags_html(config, None);
  let opengraph_html = build_opengraph_html(config);
  tera_context.insert("meta_tags_html", &meta_tags_html);
  tera_context.insert("opengraph_html", &opengraph_html);

  // Render the template
  let html = tera.render("search", &tera_context)?;
  Ok(html)
}

/// Get the template content from file in template directory or use default.
/// Results are cached to avoid repeated disk I/O on every render.
#[allow(
  clippy::expect_used,
  reason = "RwLock poisoning is unrecoverable; panic is the correct response"
)]
fn get_template_content(
  config: &Config,
  template_name: &str,
  fallback: &str,
) -> Result<String> {
  // Create cache key that includes template path to handle different configs.
  // Use both template_dir and template_path to ensure uniqueness even when
  // only template_path (a file) is set - get_template_path() returns None
  // for file-only templates, causing cache collisions.
  let template_path_key = config
    .template_dir
    .as_ref()
    .or(config.template_path.as_ref())
    .map_or_else(|| "default".to_string(), |p| p.display().to_string());
  let cache_key = format!("{template_path_key}:{template_name}");

  // Check cache first (read lock)
  {
    let cache = TEMPLATE_CONTENT_CACHE
      .read()
      .expect("template content cache lock not poisoned");
    if let Some(cached) = cache.get(&cache_key) {
      return Ok(cached.clone());
    }
  }

  // Load template content (not cached, this is the first call)
  let content = load_template_content(config, template_name, fallback)?;

  // Insert into cache (write lock)
  {
    let mut cache = TEMPLATE_CONTENT_CACHE
      .write()
      .expect("template content cache lock not poisoned");
    cache.entry(cache_key).or_insert_with(|| content.clone());
  }

  Ok(content)
}

/// Actually load the template content from disk or embedded defaults
fn load_template_content(
  config: &Config,
  template_name: &str,
  fallback: &str,
) -> Result<String> {
  // Try to get the template from the configured template directory
  if let Some(template_dir) = config.get_template_path() {
    let template_path = template_dir.join(template_name);
    if template_path.exists() {
      log::debug!("Loading custom template: {}", template_path.display());
      return fs::read_to_string(&template_path).wrap_err({
        format!("Failed to read template file: {}", template_path.display())
      });
    }
    // Custom template directory is set but this specific template is missing
    log::warn!(
      "Template not found in custom directory: {}. Using embedded default.",
      template_path.display()
    );
  }
  // If template_path is specified but doesn't point to a directory with our
  // template
  if let Some(template_path) = &config.template_path
    && template_path.exists()
    && template_name == "default.html"
  {
    // XXX: For backward compatibility
    // If template_path is a file, use it for default.html
    log::debug!(
      "Loading custom template (backward compat): {}",
      template_path.display()
    );
    return fs::read_to_string(template_path).wrap_err({
      format!(
        "Failed to read custom template file: {}. Check file permissions and \
         ensure the file is valid UTF-8",
        template_path.display()
      )
    });
  }
  // Use fallback embedded template if no custom template found
  // If fallback is empty, return empty string (used for optional templates)
  log::debug!("Using embedded default template: {template_name}");
  Ok(fallback.to_string())
}

/// Represents a navigation item with its metadata for rendering
struct NavItem {
  path:     String,
  title:    String,
  /// Parent directory relative to `input_dir` ().
  rel_dir:  String, // empty string for root items
  position: Option<usize>,
  number:   Option<usize>,
}

/// Render a single navigation item as a `<li>` element, with optional
/// numbering.
fn render_nav_item(output: &mut String, item: &NavItem) {
  if let Some(num) = item.number {
    let _ = writeln!(
      output,
      "<li><a href=\"{}\">{num}. {}</a></li>",
      item.path, item.title
    );
  } else {
    let _ = writeln!(
      output,
      "<li><a href=\"{}\">{}</a></li>",
      item.path, item.title
    );
  }
}

/// Render navigation items grouped by their parent directory.
///
/// Items at the root (empty `rel_dir`) are rendered flat. Items in a
/// subdirectory are wrapped in a collapsible `<details>` element with a
/// `<summary>` label derived from the directory name.
fn render_grouped(output: &mut String, items: Vec<NavItem>, show_counts: bool) {
  use std::collections::BTreeMap;

  // Separate root items from directory-grouped items.
  let mut root_items: Vec<NavItem> = Vec::new();
  // BTreeMap preserves directory insertion order (sorted by dir name).
  let mut dir_groups: BTreeMap<String, Vec<NavItem>> = BTreeMap::new();

  for item in items {
    if item.rel_dir.is_empty() {
      root_items.push(item);
    } else {
      // Use only the first path component as the group label so that
      // `features/foo/bar.md` is still grouped under `features`.
      let top_dir = std::path::Path::new(&item.rel_dir)
        .components()
        .next()
        .and_then(|c| c.as_os_str().to_str())
        .unwrap_or(&item.rel_dir)
        .to_string();
      dir_groups.entry(top_dir).or_default().push(item);
    }
  }

  // Render root items flat.
  for item in root_items {
    render_nav_item(output, &item);
  }

  // Render each directory group as a collapsible block.
  for (dir_name, group_items) in dir_groups {
    // Capitalise the first letter of the directory name for the label.
    let label = {
      let mut chars = dir_name.chars();
      match chars.next() {
        None => dir_name.clone(),
        Some(first) => {
          first.to_uppercase().collect::<String>() + chars.as_str()
        },
      }
    };
    let count = group_items.len();
    let count_badge = if show_counts {
      format!("<span class=\"sidebar-dir-count\">{count}</span>")
    } else {
      String::new()
    };
    let _ = writeln!(
      output,
      "<li class=\"sidebar-dir-group\"><details open><summary><span \
       class=\"sidebar-dir-label\">{label}</span>{count_badge}</summary><ul>"
    );

    for item in group_items {
      render_nav_item(output, &item);
    }

    let _ = writeln!(output, "</ul></details></li>");
  }
}

/// Generate the document navigation HTML
fn generate_doc_nav(config: &Config, current_file_rel_path: &Path) -> String {
  let mut doc_nav = String::new();
  let root_prefix = calculate_root_relative_path(current_file_rel_path);

  // Only process markdown files if input_dir is provided
  if let Some(input_dir) = &config.input_dir {
    let entries: Vec<_> = walkdir::WalkDir::new(input_dir)
      .follow_links(true)
      .into_iter()
      .filter_map(std::result::Result::ok)
      .filter(|e| {
        if !e.path().is_file()
          || e.path().extension().is_none_or(|ext| ext != "md")
        {
          return false;
        }

        // Filter out excluded files (included files)
        e.path()
          .strip_prefix(input_dir)
          .is_ok_and(|rel_path| !config.included_files.contains_key(rel_path))
      })
      .collect();

    if !entries.is_empty() {
      // Partition entries into special and regular
      let mut special_entries = Vec::new();
      let mut regular_entries = Vec::new();

      for entry in &entries {
        let path = entry.path();
        if let Ok(rel_doc_path) = path.strip_prefix(input_dir) {
          let file_name = rel_doc_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
          if file_name == "index.md" || file_name == "readme.md" {
            special_entries.push(entry);
          } else {
            regular_entries.push(entry);
          }
        }
      }

      // Process regular entries with sidebar configuration
      let mut nav_items: Vec<NavItem> = regular_entries
        .iter()
        .filter_map(|entry| {
          let path = entry.path();
          let rel_doc_path = path.strip_prefix(input_dir).ok()?;
          let mut html_path = rel_doc_path.to_path_buf();
          html_path.set_extension("html");

          let target_path =
            format!("{}{}", root_prefix, html_path.to_string_lossy());

          // Extract page title
          let page_title =
            ndg_utils::markdown::extract_page_title(path, &html_path);

          // Apply sidebar configuration if available
          let (display_title, position) =
            if let Some(sidebar_config) = &config.sidebar {
              let path_str = rel_doc_path.to_string_lossy();
              if let Some(matched_rule) =
                sidebar_config.find_match(&path_str, &page_title)
              {
                let title = matched_rule
                  .get_title()
                  .map_or_else(|| page_title.clone(), String::from);
                let pos = matched_rule.get_position();
                (title, pos)
              } else {
                (page_title, None)
              }
            } else {
              (page_title, None)
            };

          Some(NavItem {
            path: target_path,
            title: display_title,
            rel_dir: rel_doc_path
              .parent()
              .and_then(|p| p.to_str())
              .unwrap_or("")
              .to_string(),
            position,
            number: None,
          })
        })
        .collect();

      // Sort items based on sidebar ordering configuration
      if let Some(sidebar_config) = &config.sidebar {
        use ndg_config::sidebar::SidebarOrdering;
        match sidebar_config.ordering {
          SidebarOrdering::Alphabetical => {
            nav_items.sort_by(|a, b| a.title.cmp(&b.title));
          },
          SidebarOrdering::Custom => {
            // Sort by position first, then alphabetically
            nav_items.sort_by(|a, b| {
              match (a.position, b.position) {
                (Some(pos_a), Some(pos_b)) => pos_a.cmp(&pos_b),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => a.title.cmp(&b.title),
              }
            });
          },
          SidebarOrdering::Filesystem => {
            // Keep filesystem order (already in order thanks to walkdir)
          },
        }

        // Add numbering if enabled
        if sidebar_config.numbered {
          for (idx, item) in nav_items.iter_mut().enumerate() {
            item.number = Some(idx + 1);
          }
        }
      } else {
        // Default: alphabetical sorting
        nav_items.sort_by(|a, b| a.title.cmp(&b.title));
      }

      // Process special entries
      let mut special_nav_items: Vec<NavItem> = special_entries
        .iter()
        .filter_map(|entry| {
          let path = entry.path();
          let rel_doc_path = path.strip_prefix(input_dir).ok()?;
          let mut html_path = rel_doc_path.to_path_buf();
          html_path.set_extension("html");

          let target_path =
            format!("{}{}", root_prefix, html_path.to_string_lossy());

          let page_title =
            ndg_utils::markdown::extract_page_title(path, &html_path);

          // Apply sidebar configuration to special files if available
          let display_title = if let Some(sidebar_config) = &config.sidebar {
            let path_str = rel_doc_path.to_string_lossy();
            if let Some(matched_rule) =
              sidebar_config.find_match(&path_str, &page_title)
            {
              matched_rule
                .get_title()
                .map_or_else(|| page_title.clone(), String::from)
            } else {
              page_title
            }
          } else {
            page_title
          };

          Some(NavItem {
            path:     target_path,
            title:    display_title,
            rel_dir:  rel_doc_path
              .parent()
              .and_then(|p| p.to_str())
              .unwrap_or("")
              .to_string(),
            position: None,
            number:   None,
          })
        })
        .collect();

      // Sort special entries according to sidebar ordering configuration
      if let Some(sidebar_config) = &config.sidebar {
        match sidebar_config.ordering {
          SidebarOrdering::Alphabetical => {
            special_nav_items.sort_by(|a, b| a.title.cmp(&b.title));
          },
          SidebarOrdering::Custom => {
            // Custom ordering uses position from matches
            special_nav_items.sort_by_key(|item| item.position);
          },
          SidebarOrdering::Filesystem => {
            // Filesystem ordering keeps the order from directory iteration
          },
        }
      } else {
        // Default: alphabetical sorting
        special_nav_items.sort_by(|a, b| a.title.cmp(&b.title));
      }

      // Determine if we should number special files
      let should_number_special = config
        .sidebar
        .as_ref()
        .is_some_and(|s| s.numbered && s.number_special_files);

      let group_by_dir =
        config.sidebar.as_ref().is_some_and(|s| s.group_by_dir);

      let show_group_counts =
        config.sidebar.as_ref().is_some_and(|s| s.show_group_counts);

      // Render navigation items
      if should_number_special {
        // Combine special and regular items with unified numbering
        let mut all_items = special_nav_items;
        all_items.extend(nav_items);

        // Apply numbering to all items
        for (idx, item) in all_items.iter_mut().enumerate() {
          item.number = Some(idx + 1);
        }

        if group_by_dir {
          render_grouped(&mut doc_nav, all_items, show_group_counts);
        } else {
          // Render all items flat
          for item in all_items {
            render_nav_item(&mut doc_nav, &item);
          }
        }
      } else {
        // Render special entries first without numbering
        for item in &special_nav_items {
          let _ = writeln!(
            doc_nav,
            "<li><a href=\"{}\">{}</a></li>",
            item.path, item.title
          );
        }

        if group_by_dir {
          render_grouped(&mut doc_nav, nav_items, show_group_counts);
        } else {
          // Render regular entries with optional numbering
          for item in nav_items {
            render_nav_item(&mut doc_nav, &item);
          }
        }
      }
    }
  }

  // Add link to options page if module_options is configured
  if doc_nav.is_empty() && config.module_options.is_some() {
    let _ = writeln!(
      doc_nav,
      "<li><a href=\"{root_prefix}options.html\">Module Options</a></li>"
    );
  }

  // Add link to lib page if nixdoc_inputs is configured
  if !config.nixdoc_inputs.is_empty() {
    let _ = writeln!(
      doc_nav,
      "<li><a href=\"{root_prefix}lib.html\">Library Reference</a></li>"
    );
  }

  // Add search link only if search is enabled
  if config.is_search_enabled() {
    let _ = writeln!(
      doc_nav,
      "<li><a href=\"{root_prefix}search.html\">Search</a></li>"
    );
  }

  doc_nav
}

/// Generate custom scripts HTML
fn generate_custom_scripts(
  config: &Config,
  current_file_rel_path: &Path,
) -> Result<String> {
  let mut custom_scripts = String::new();
  let root_prefix =
    ndg_utils::html::calculate_root_relative_path(current_file_rel_path);

  // Add any user scripts from script_paths. This is additive, not replacing. To
  // replace default content, the user should specify `--template-dir` or
  // `--template` instead.
  for script_path in &config.script_paths {
    // Relative path to script
    let script_relative_path =
      format!("{}{}", root_prefix, script_path.to_string_lossy());
    write!(
      custom_scripts,
      "<script defer src=\"{script_relative_path}\"></script>"
    )?;
  }

  Ok(custom_scripts)
}

/// Generate table of contents from headers
fn generate_toc(headers: &[Header]) -> String {
  let mut toc = String::new();
  let mut current_level = 0;

  for header in headers {
    // Only include h1, h2, h3 in TOC
    if header.level <= 3 {
      // Adjust TOC nesting
      while current_level < header.level - 1 {
        toc.push_str("<ul>");
        current_level += 1;
      }
      while current_level > header.level - 1 {
        toc.push_str("</ul>");
        current_level -= 1;
      }

      // Add TOC item
      if current_level == header.level - 1 {
        toc.push_str("<li>");
      } else {
        toc.push_str("</li><li>");
      }

      let mut visible_text = header.text.as_str();
      if let Some(idx) = visible_text.rfind("{#")
        && visible_text.ends_with('}')
      {
        visible_text = visible_text[..idx].trim_end();
      }
      // Writing to String is infallible
      let _ = writeln!(
        toc,
        "<a href=\"#{}\">{}</a>",
        header.id,
        encode_text(visible_text)
      );
    }
  }

  // Close open tags
  while current_level > 0 {
    toc.push_str("</li></ul>");
    current_level -= 1;
  }
  if !headers.is_empty() && !toc.is_empty() && !toc.ends_with("</li>") {
    toc.push_str("</li>");
  }

  toc
}

/// Produce a safe HTML `id` / URL fragment from an option name.
///
/// Dots are the canonical separator in NixOS option names, but names may also
/// contain `<name>` placeholders (angle brackets) and other characters that
/// are invalid or dangerous in HTML `id` attributes and URL fragments. Replace
/// every character that is not ASCII alphanumeric, `-`, or `_` with `-`.
fn sanitize_option_id(name: &str) -> String {
  let sanitized: String = name
    .chars()
    .map(|c| {
      if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
        c
      } else {
        '-'
      }
    })
    .collect();
  format!("option-{sanitized}")
}

/// Generate the options HTML content
fn generate_options_html(options: &IndexMap<String, NixOption>) -> String {
  let mut options_html = String::with_capacity(options.len() * 500); // rough capacity estimate, average ~500 bytes per option

  // Iterate in the order established by process_options, i.e., priority sort.
  for option in options.values() {
    let option_id = sanitize_option_id(&option.name);

    // Open option container with ID for direct linking.
    let _ = writeln!(options_html, "<div class=\"option\" id=\"{option_id}\">");

    // Option name with anchor link and copy button
    let _ = write!(
      options_html,
      "  <h3 class=\"option-name\">\n    <a href=\"#{}\" \
       class=\"option-anchor\">{}</a>\n    <span class=\"copy-link\" \
       title=\"Copy link to this option\"></span>\n    <span \
       class=\"copy-feedback\">Link copied!</span>\n  </h3>\n",
      option_id,
      encode_text(&option.name)
    );

    // Option metadata (internal/readOnly)
    let mut metadata = Vec::new();
    if option.internal {
      metadata.push("internal");
    }
    if option.read_only {
      metadata.push("read-only");
    }

    if !metadata.is_empty() {
      // Writing to String is infallible
      let _ = writeln!(
        options_html,
        "  <div class=\"option-metadata\">{}</div>",
        metadata.join(", ")
      );
    }

    // Option type
    let _ = writeln!(
      options_html,
      "  <div class=\"option-type\">Type: <code>{}</code></div>",
      encode_text(&option.type_name)
    );

    // Option description
    let _ = writeln!(
      options_html,
      "  <div class=\"option-description\">{}</div>",
      option.description
    );

    // Add default value if available
    add_default_value(&mut options_html, option);

    // Add example if available
    add_example_value(&mut options_html, option);

    // Option declared in - now with hyperlink support
    if let Some(declared_in) = &option.declared_in {
      // Writing to String is infallible
      if let Some(url) = &option.declared_in_url {
        let safe_url = html_escape::encode_double_quoted_attribute(url);
        let _ = writeln!(
          options_html,
          "  <div class=\"option-declared\">Declared in: <code><a \
           href=\"{safe_url}\" \
           target=\"_blank\">{declared_in}</a></code></div>"
        );
      } else {
        let _ = writeln!(
          options_html,
          "  <div class=\"option-declared\">Declared in: \
           <code>{declared_in}</code></div>"
        );
      }
    }

    // Close option div
    options_html.push_str("</div>\n");
  }

  options_html
}

/// Add default value to options HTML
fn add_default_value(html: &mut String, option: &NixOption) {
  if let Some(default_text) = &option.default_text {
    // Remove surrounding backticks if present (from literalExpression)
    let clean_default = if default_text.starts_with('`')
      && default_text.ends_with('`')
      && default_text.len() > 2
    {
      &default_text[1..default_text.len() - 1]
    } else {
      default_text
    };

    // Writing to String is infallible
    let _ = writeln!(
      html,
      "  <div class=\"option-default\">Default: <code>{}</code></div>",
      html_escape::encode_text(clean_default)
    );
  } else if let Some(default_val) = &option.default {
    let _ = writeln!(
      html,
      "  <div class=\"option-default\">Default: <code>{}</code></div>",
      html_escape::encode_text(&default_val.to_string()),
    );
  }
}

/// Add example value to options HTML
fn add_example_value(html: &mut String, option: &NixOption) {
  if let Some(example_text) = &option.example_text {
    // Strip surrounding backticks (literalExpression wrapper) before escaping,
    // so the backtick positions are checked on the original text.
    let inner: &str = if example_text.starts_with('`')
      && example_text.ends_with('`')
      && example_text.len() > 2
    {
      &example_text[1..example_text.len() - 1]
    } else {
      example_text
    };

    let safe = encode_text(inner);

    if inner.contains('\n') {
      let _ = writeln!(
        html,
        "  <div class=\"option-example\">Example: \
         <pre><code>{safe}</code></pre></div>"
      );
    } else {
      let _ = writeln!(
        html,
        "  <div class=\"option-example\">Example: <code>{safe}</code></div>"
      );
    }
  } else if let Some(example_val) = &option.example {
    let example_str = example_val.to_string();
    let safe = encode_text(&example_str);
    if example_str.contains('\n') {
      let _ = writeln!(
        html,
        "  <div class=\"option-example\">Example: \
         <pre><code>{safe}</code></pre></div>"
      );
    } else {
      let _ = writeln!(
        html,
        "  <div class=\"option-example\">Example: <code>{safe}</code></div>"
      );
    }
  }
}

/// Build the `OpenGraph` HTML string, handling `og:image` local paths by
/// copying the file into `output_dir/assets/` and rewriting to a relative URL.
fn build_opengraph_html(config: &Config) -> String {
  get_opengraph(config).map_or_else(String::new, |opengraph| {
    opengraph
      .iter()
      .map(|(k, v)| {
        if k == "og:image"
          && !v.starts_with("http://")
          && !v.starts_with("https://")
        {
          // Local file path: copy to assets/ and use relative path
          let image_path = std::path::Path::new(v.as_str());
          let assets_dir = config.output_dir.join("assets");
          let file_name = image_path
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or(v.as_str());
          let dest_path = assets_dir.join(file_name);
          if let Err(e) = std::fs::create_dir_all(&assets_dir) {
            log::error!("Failed to create assets dir for og:image: {e}");
          }
          if let Err(e) = std::fs::copy(image_path, &dest_path) {
            log::error!(
              "Failed to copy og:image from {} to {}: {e}",
              v,
              dest_path.display()
            );
          }
          let rel_path = format!("assets/{file_name}");
          format!(
            "<meta property=\"{}\" content=\"{}\" />",
            encode_text(k),
            encode_text(&rel_path)
          )
        } else {
          format!(
            "<meta property=\"{}\" content=\"{}\" />",
            encode_text(k),
            encode_text(v)
          )
        }
      })
      .collect::<Vec<_>>()
      .join("\n    ")
  })
}

/// Get `OpenGraph` tags with backward compatibility
///
/// Checks the new `meta.opengraph` field first, then falls back to deprecated
/// `opengraph` field
#[allow(deprecated)]
const fn get_opengraph(config: &Config) -> Option<&HashMap<String, String>> {
  if let Some(ref meta) = config.meta
    && let Some(ref og) = meta.opengraph
  {
    return Some(og);
  }
  config.opengraph.as_ref()
}

/// Get meta tags with backward compatibility
///
/// Checks the new `meta.tags` field first, then falls back to deprecated
/// `meta_tags` field
#[allow(deprecated)]
const fn get_meta_tags(config: &Config) -> Option<&HashMap<String, String>> {
  if let Some(ref meta) = config.meta
    && let Some(ref tags) = meta.tags
  {
    return Some(tags);
  }
  config.meta_tags.as_ref()
}

/// Render a single processed markdown item to HTML and write to disk.
///
/// This function combines template rendering, optional postprocessing,
/// and file I/O for a single markdown document.
///
/// # Arguments
///
/// * `config` - Site configuration
/// * `item` - The processed markdown item containing HTML content and metadata
///
/// # Errors
///
/// Returns an error if template rendering, postprocessing, or file writing
/// fails.
pub fn render_and_write(
  config: &Config,
  item: &ndg_utils::markdown::ProcessedMarkdown,
) -> Result<()> {
  use ndg_utils::postprocess;

  let rel_path = std::path::Path::new(&item.output_path);
  if rel_path.is_absolute() {
    log::warn!(
      "Output path '{}' is absolute. Normalizing to relative.",
      item.output_path
    );
  }

  // Force rel_path to be relative
  let rel_path = rel_path
    .strip_prefix(std::path::MAIN_SEPARATOR_STR)
    .unwrap_or(rel_path);

  let html = render(
    config,
    &item.html_content,
    item.title.as_deref().unwrap_or(&config.title),
    &item.headers,
    rel_path,
    item.frontmatter.as_ref(),
  )?;

  // Apply postprocessing if requested
  let processed_html = if let Some(ref postprocess_config) = config.postprocess
  {
    postprocess::process_html(&html, postprocess_config)?
  } else {
    html
  };

  let output_path = config.output_dir.join(rel_path);
  fs::write(&output_path, processed_html).wrap_err_with(|| {
    format!("Failed to write output HTML: {}", output_path.display())
  })?;

  Ok(())
}
