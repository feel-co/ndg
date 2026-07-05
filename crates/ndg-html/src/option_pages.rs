use indexmap::IndexMap;
use ndg_config::Config;
use ndg_manpage::types::NixOption;

/// A generated option group page.
#[derive(Debug, Clone)]
pub struct OptionPage {
  /// Option attrset prefix owned by this page.
  pub prefix: String,

  /// Display title for the page/group.
  pub title: String,

  /// Relative output path for the generated page.
  pub path: String,

  /// Custom sort position for the options index.
  pub position: Option<usize>,

  /// Options rendered on this page.
  pub options: IndexMap<String, NixOption>,
}

/// Complete split options output model.
#[derive(Debug, Clone, Default)]
pub struct OptionPageSet {
  /// Generated option group pages.
  pub pages: Vec<OptionPage>,

  /// Options that stay on `options.html`.
  pub root_options: IndexMap<String, NixOption>,
}

/// Whether multi-page options output is enabled.
#[must_use]
pub fn pages_enabled(config: &Config) -> bool {
  config
    .options
    .as_ref()
    .and_then(|options| options.pages.as_ref())
    .is_some_and(|pages| pages.enabled)
}

/// Return the owning page path for an option, or `options.html` in index mode.
#[must_use]
pub fn option_page_path(config: &Config, option_name: &str) -> String {
  option_route(config, option_name)
    .map_or_else(|| "options.html".to_string(), |route| route.path)
}

/// Group options by their configured owning page.
#[must_use]
pub fn build_option_pages(
  config: &Config,
  options: &IndexMap<String, NixOption>,
) -> OptionPageSet {
  let mut pages: IndexMap<String, OptionPage> = IndexMap::new();
  let mut root_options = IndexMap::new();

  for option in options.values() {
    let Some(route) = option_route(config, &option.name) else {
      root_options.insert(option.name.clone(), option.clone());
      continue;
    };

    let page = pages.entry(route.path.clone()).or_insert_with(|| {
      OptionPage {
        prefix:   route.prefix,
        title:    route.title,
        path:     route.path,
        position: route.position,
        options:  IndexMap::new(),
      }
    });

    page.options.insert(option.name.clone(), option.clone());
  }

  let mut pages: Vec<_> = pages.into_values().collect();
  pages.sort_by(|a, b| {
    match (a.position, b.position) {
      (Some(a_pos), Some(b_pos)) => a_pos.cmp(&b_pos),
      (Some(_), None) => std::cmp::Ordering::Less,
      (None, Some(_)) => std::cmp::Ordering::Greater,
      (None, None) => a.prefix.cmp(&b.prefix),
    }
  });
  OptionPageSet {
    pages,
    root_options,
  }
}

#[derive(Debug, Clone)]
struct OptionRoute {
  prefix:   String,
  title:    String,
  path:     String,
  position: Option<usize>,
}

fn option_route(config: &Config, option_name: &str) -> Option<OptionRoute> {
  let pages = config
    .options
    .as_ref()
    .and_then(|options| options.pages.as_ref())?;

  if !pages.enabled {
    return None;
  }

  let matched = pages.find_match(option_name);
  if matched.is_none() && option_name.split('.').count() == 1 {
    return None;
  }

  let depth = matched.and_then(|m| m.depth).unwrap_or(pages.depth).max(1);
  let prefix = option_prefix(option_name, depth);
  let title = matched
    .and_then(|m| m.title.as_deref())
    .unwrap_or(&prefix)
    .to_string();
  let position = matched.and_then(|m| m.position);
  let root = clean_root(&pages.root);
  let path = format!("{root}/{}.html", sanitize_page_slug(&prefix));

  Some(OptionRoute {
    prefix,
    title,
    path,
    position,
  })
}

fn option_prefix(option_name: &str, depth: usize) -> String {
  option_name
    .split('.')
    .take(depth)
    .collect::<Vec<_>>()
    .join(".")
}

fn clean_root(root: &str) -> String {
  let root = root.trim_matches('/');
  if root.is_empty() {
    "options".to_string()
  } else {
    root.to_string()
  }
}

fn sanitize_page_slug(prefix: &str) -> String {
  prefix
    .chars()
    .map(|c| {
      match c {
        '/' | '\\' | '<' | '>' | '[' | ']' | ':' | '"' | ' ' | '*' => '_',
        c => c,
      }
    })
    .collect()
}

#[cfg(test)]
mod tests {
  #![allow(clippy::expect_used, reason = "Fine in tests")]

  use ndg_config::{
    matchers::OptionNameMatch,
    options::{OptionsConfig, OptionsPageMatch, OptionsPagesConfig},
  };

  use super::*;

  fn config(depth: usize, matches: Vec<OptionsPageMatch>) -> Config {
    Config {
      options: Some(OptionsConfig {
        pages: Some(OptionsPagesConfig {
          enabled: true,
          depth,
          matches,
          ..Default::default()
        }),
        ..Default::default()
      }),
      ..Default::default()
    }
  }

  #[test]
  fn routes_by_default_depth() {
    let config = config(1, Vec::new());
    assert_eq!(
      option_page_path(&config, "foo.bar.enable"),
      "options/foo.html"
    );
  }

  #[test]
  fn root_options_stay_on_index_without_match() {
    let config = config(1, Vec::new());
    assert_eq!(option_page_path(&config, "enable"), "options.html");
  }

  #[test]
  fn match_can_route_deep_prefix() {
    let mut config = config(1, vec![OptionsPageMatch {
      name: Some(OptionNameMatch {
        exact:          None,
        regex:          Some(r"^foo\.bar\.baz(\.|$)".to_string()),
        compiled_regex: None,
      }),
      depth: Some(3),
      ..Default::default()
    }]);
    config
      .options
      .as_mut()
      .expect("options config")
      .validate()
      .expect("valid page match");

    assert_eq!(
      option_page_path(&config, "foo.bar.baz.quz.enable"),
      "options/foo.bar.baz.html"
    );
  }

  #[test]
  fn split_result_keeps_root_options_without_rerouting() {
    let config = config(1, Vec::new());
    let mut options = IndexMap::new();
    for name in ["enable", "foo.bar.enable"] {
      options.insert(name.to_string(), NixOption {
        name: name.to_string(),
        ..Default::default()
      });
    }

    let page_set = build_option_pages(&config, &options);
    assert!(page_set.root_options.contains_key("enable"));
    assert_eq!(page_set.pages.len(), 1);
    assert_eq!(page_set.pages[0].path, "options/foo.html");
  }
}
