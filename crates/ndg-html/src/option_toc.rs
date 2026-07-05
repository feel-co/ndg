use std::fmt::Write;

use html_escape::encode_text;
use ndg_manpage::types::NixOption;
use rustc_hash::FxHashMap;

use crate::{
  template::sanitize_option_id,
  toc_tree::{TocPathPart, TocTree},
};

pub(crate) fn generate_nested_options_toc_html(
  grouped_options: &FxHashMap<String, Vec<&NixOption>>,
  direct_parent_options: &FxHashMap<String, &NixOption>,
  option_custom_names: &FxHashMap<String, String>,
  option_positions: &FxHashMap<String, usize>,
  nested_depth: usize,
) -> String {
  let mut categories: Vec<_> = grouped_options.iter().collect();
  categories.sort_by(|(a_name, _), (b_name, _)| {
    let a_position = option_positions.get(*a_name);
    let b_position = option_positions.get(*b_name);

    match (a_position, b_position) {
      (Some(a_pos), Some(b_pos)) => a_pos.cmp(b_pos),
      (Some(_), None) => std::cmp::Ordering::Less,
      (None, Some(_)) => std::cmp::Ordering::Greater,
      (None, None) => a_name.cmp(b_name),
    }
  });

  let mut html = String::new();
  html.push_str("<ul class=\"toc-list\">\n");

  for (parent, opts) in categories {
    let parent_option = direct_parent_options.get(parent).copied();
    let child_options: Vec<_> = opts
      .iter()
      .filter(|option| option.name != *parent)
      .copied()
      .collect();

    if parent_option.is_none() && child_options.len() == 1 {
      render_option_toc_link(
        &mut html,
        child_options[0],
        option_custom_names
          .get(&child_options[0].name)
          .map_or(&child_options[0].name, String::as_str),
        1,
      );
      continue;
    }

    let mut tree = TocTree::default();
    if let Some(option) = parent_option {
      tree.insert(
        &[parent_path_part(parent, option_custom_names)],
        option_toc_leaf_html(option, &option.name),
        false,
      );
    }

    for option in child_options {
      tree.insert(
        &option_toc_path(parent, option, option_custom_names, nested_depth),
        option_toc_leaf_html(
          option,
          option_custom_names
            .get(&option.name)
            .map_or_else(
              || option_leaf_label(parent, option, nested_depth),
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
  html
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
