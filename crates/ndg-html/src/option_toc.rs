use std::fmt::Write;

use color_eyre::eyre::Result;
use html_escape::encode_text;
use indexmap::IndexMap;
use ndg_config::sidebar::{OptionsConfig, OptionsMatch, SidebarOrdering};
use ndg_manpage::types::NixOption;
use rustc_hash::FxHashMap;

use crate::{
  template::sanitize_option_id,
  toc_tree::{TocPathPart, TocTree},
};

pub(crate) struct OptionsTocModel<'a> {
  pub(crate) grouped_options:       IndexMap<String, Vec<&'a NixOption>>,
  pub(crate) direct_parent_options: FxHashMap<String, &'a NixOption>,
  pub(crate) custom_names:          FxHashMap<String, String>,
  pub(crate) option_positions:      FxHashMap<String, usize>,
  pub(crate) group_positions:       FxHashMap<String, usize>,
  pub(crate) input_order:           FxHashMap<String, usize>,
  pub(crate) group_input_order:     FxHashMap<String, usize>,
  pub(crate) nested_depth:          usize,
  pub(crate) ordering:              SidebarOrdering,
}

impl<'a> OptionsTocModel<'a> {
  pub(crate) fn new(
    options: &'a IndexMap<String, NixOption>,
    sidebar_options: Option<&OptionsConfig>,
    input_order: Option<&FxHashMap<String, usize>>,
  ) -> Self {
    let default_depth = sidebar_options.map_or(2, |options| options.depth);
    let nested_depth =
      sidebar_options.map_or(0, |options| options.nested_depth);
    let ordering = sidebar_options
      .map_or(SidebarOrdering::Alphabetical, |options| options.ordering);
    let input_order = input_order.cloned().unwrap_or_else(|| {
      options
        .keys()
        .enumerate()
        .map(|(index, name)| (name.clone(), index))
        .collect()
    });
    let mut grouped_options: IndexMap<String, Vec<&NixOption>> =
      IndexMap::default();
    let mut direct_parent_options = FxHashMap::default();
    let mut custom_names = FxHashMap::default();
    let mut option_positions = FxHashMap::default();

    for option in options.values() {
      let match_result =
        sidebar_options.and_then(|options| options.find_match(&option.name));
      if let Some(matched) = match_result {
        if matched.is_hidden() {
          continue;
        }
        if let Some(name) = matched.get_name() {
          custom_names.insert(option.name.clone(), name.to_string());
        }
        if let Some(position) = matched.get_position() {
          option_positions.insert(option.name.clone(), position);
        }
      }

      let depth = match_result
        .and_then(OptionsMatch::get_depth)
        .unwrap_or(default_depth);
      let parent = option_parent(&option.name, depth);
      if option.name == parent {
        direct_parent_options.insert(parent.clone(), option);
      }
      grouped_options.entry(parent).or_default().push(option);
    }

    let group_positions = group_ranks(&grouped_options, &option_positions);
    let group_input_order = group_ranks(&grouped_options, &input_order);
    Self {
      grouped_options,
      direct_parent_options,
      custom_names,
      option_positions,
      group_positions,
      input_order,
      group_input_order,
      nested_depth,
      ordering,
    }
  }

  pub(crate) fn categories(&self) -> Vec<(&String, &Vec<&'a NixOption>)> {
    let mut categories: Vec<_> = self.grouped_options.iter().collect();
    match self.ordering {
      SidebarOrdering::Alphabetical => {
        categories.sort_by_key(|(name, _)| *name);
      },
      SidebarOrdering::Custom => {
        categories.sort_by(|(left, _), (right, _)| {
          compare_positions(
            self.group_positions.get(*left),
            self.group_positions.get(*right),
            left,
            right,
          )
        });
      },
      SidebarOrdering::Filesystem => {
        categories.sort_by_key(|(name, _)| self.group_input_order.get(*name));
      },
    }
    categories
  }

  pub(crate) fn child_options(
    &self,
    parent: &str,
    options: &[&'a NixOption],
  ) -> Vec<&'a NixOption> {
    let mut children: Vec<_> = options
      .iter()
      .filter(|option| option.name != parent)
      .copied()
      .collect();
    let parent_prefix = format!("{parent}.");
    children.sort_by(|left, right| {
      match self.ordering {
        SidebarOrdering::Filesystem => {
          self
            .input_order
            .get(&left.name)
            .cmp(&self.input_order.get(&right.name))
        },
        SidebarOrdering::Custom => {
          compare_positions(
            self.option_positions.get(&left.name),
            self.option_positions.get(&right.name),
            left.name.strip_prefix(&parent_prefix).unwrap_or(&left.name),
            right
              .name
              .strip_prefix(&parent_prefix)
              .unwrap_or(&right.name),
          )
        },
        SidebarOrdering::Alphabetical => {
          left
            .name
            .strip_prefix(&parent_prefix)
            .unwrap_or(&left.name)
            .cmp(
              right
                .name
                .strip_prefix(&parent_prefix)
                .unwrap_or(&right.name),
            )
        },
      }
    });
    children
  }
}

pub(crate) trait OptionsTocGenerator {
  fn generate(&self, model: &OptionsTocModel<'_>) -> Result<String>;
}

pub(crate) struct NestedOptionsTocGenerator;

impl OptionsTocGenerator for NestedOptionsTocGenerator {
  fn generate(&self, model: &OptionsTocModel<'_>) -> Result<String> {
    let (single_options, dropdown_categories): (Vec<_>, Vec<_>) = model
      .categories()
      .into_iter()
      .partition(|(parent, options)| {
        let child_count = options
          .iter()
          .filter(|option| option.name != **parent)
          .count();
        child_count == 0
          || (!model.direct_parent_options.contains_key(*parent)
            && child_count == 1)
      });
    let mut html = String::new();
    html.push_str("<ul class=\"toc-list\">\n");
    for (parent, options) in
      single_options.into_iter().chain(dropdown_categories)
    {
      let parent_option = model.direct_parent_options.get(parent).copied();
      let child_options = model.child_options(parent, options);
      if parent_option.is_none() && child_options.len() == 1 {
        let option = child_options[0];
        render_option_toc_link(
          &mut html,
          option,
          model
            .custom_names
            .get(&option.name)
            .map_or(&option.name, String::as_str),
          1,
        );
        continue;
      }
      let mut tree = TocTree::default();
      if let Some(option) = parent_option {
        tree.insert(
          &[parent_path_part(parent, &model.custom_names)],
          option_toc_leaf_html(option, &option.name),
          false,
        );
      }
      for option in child_options {
        tree.insert(
          &option_toc_path(
            parent,
            option,
            &model.custom_names,
            model.nested_depth,
          ),
          option_toc_leaf_html(
            option,
            model
              .custom_names
              .get(&option.name)
              .map_or_else(
                || option_leaf_label(parent, option, model.nested_depth),
                Clone::clone,
              )
              .as_str(),
          ),
          false,
        );
      }
      append_tree_contents(&mut html, &tree.render(0));
    }
    html.push_str("</ul>\n");
    Ok(html)
  }
}

fn compare_positions(
  left_position: Option<&usize>,
  right_position: Option<&usize>,
  left_name: &str,
  right_name: &str,
) -> std::cmp::Ordering {
  match (left_position, right_position) {
    (Some(left), Some(right)) => left.cmp(right),
    (Some(_), None) => std::cmp::Ordering::Less,
    (None, Some(_)) => std::cmp::Ordering::Greater,
    (None, None) => left_name.cmp(right_name),
  }
}

fn group_ranks(
  grouped_options: &IndexMap<String, Vec<&NixOption>>,
  positions: &FxHashMap<String, usize>,
) -> FxHashMap<String, usize> {
  grouped_options
    .iter()
    .filter_map(|(parent, options)| {
      options
        .iter()
        .filter_map(|option| positions.get(&option.name))
        .min()
        .map(|position| (parent.clone(), *position))
    })
    .collect()
}

fn option_parent(option_name: &str, depth: usize) -> String {
  let parts: Vec<_> = option_name.split('.').collect();
  if parts.len() <= depth {
    option_name.to_string()
  } else {
    parts[..depth].join(".")
  }
}

fn parent_path_part(
  parent: &str,
  option_custom_names: &FxHashMap<String, String>,
) -> TocPathPart {
  TocPathPart {
    key:   parent.to_string(),
    label: option_custom_names
      .get(parent)
      .map_or(parent, String::as_str)
      .to_string(),
    title: parent.to_string(),
  }
}

fn option_toc_path(
  parent: &str,
  option: &NixOption,
  option_custom_names: &FxHashMap<String, String>,
  nested_depth: usize,
) -> Vec<TocPathPart> {
  let mut path = vec![parent_path_part(parent, option_custom_names)];
  let parent_prefix = format!("{parent}.");
  let suffix = option
    .name
    .strip_prefix(&parent_prefix)
    .unwrap_or(&option.name);
  let parts: Vec<_> = suffix.split('.').collect();
  let branch_depth = branch_depth(parts.len(), nested_depth);

  let mut full_name = parent.to_string();
  for part in &parts[..branch_depth] {
    full_name.push('.');
    full_name.push_str(part);
    path.push(TocPathPart {
      key:   full_name.clone(),
      label: option_custom_names
        .get(&full_name)
        .map_or(*part, String::as_str)
        .to_string(),
      title: full_name.clone(),
    });
  }

  let leaf_label = parts[branch_depth..].join(".");
  let leaf_title = format!("{full_name}.{leaf_label}");
  path.push(TocPathPart {
    key:   option.name.clone(),
    label: option_custom_names
      .get(&option.name)
      .map_or(leaf_label.as_str(), String::as_str)
      .to_string(),
    title: leaf_title,
  });
  path
}

fn branch_depth(part_count: usize, nested_depth: usize) -> usize {
  let max_branch_depth = part_count.saturating_sub(1);
  if nested_depth == 0 {
    max_branch_depth
  } else {
    nested_depth.min(max_branch_depth)
  }
}

fn option_leaf_label(
  parent: &str,
  option: &NixOption,
  nested_depth: usize,
) -> String {
  let parent_prefix = format!("{parent}.");
  let suffix = option
    .name
    .strip_prefix(&parent_prefix)
    .unwrap_or(&option.name);
  let parts: Vec<_> = suffix.split('.').collect();
  parts[branch_depth(parts.len(), nested_depth)..].join(".")
}

fn render_option_toc_link(
  html: &mut String,
  option: &NixOption,
  display_name: &str,
  indent: usize,
) {
  let spaces = " ".repeat(indent);
  let _ = writeln!(
    html,
    "{spaces}<li>{}</li>",
    option_toc_leaf_html(option, display_name)
  );
}

fn option_toc_leaf_html(option: &NixOption, display_name: &str) -> String {
  let id = sanitize_option_id(&option.name);
  let safe_name = encode_text(&option.name);
  let safe_display = encode_text(display_name);
  let internal = if option.internal {
    "<span class=\"toc-internal\">internal</span>"
  } else {
    ""
  };
  let read_only = if option.read_only {
    "<span class=\"toc-readonly\">read-only</span>"
  } else {
    ""
  };
  format!(
    "<a href='#{id}' \
     title=\"{safe_name}\">{safe_display}{internal}{read_only}</a>"
  )
}

fn append_tree_contents(html: &mut String, tree_html: &str) {
  let content = tree_html
    .trim_start_matches("<ul class=\"toc-list\">\n")
    .trim_end_matches("</ul>\n");
  html.push_str(content);
  if !content.ends_with('\n') {
    html.push('\n');
  }
}
