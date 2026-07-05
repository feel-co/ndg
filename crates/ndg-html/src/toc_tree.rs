use std::fmt::Write;

use html_escape::encode_text;

pub struct TocPathPart {
  pub key:   String,
  pub label: String,
  pub title: String,
}

#[derive(Default)]
pub struct TocTree {
  children: Vec<TocNode>,
}

#[derive(Default)]
struct TocNode {
  key:       String,
  label:     String,
  title:     String,
  leaf_html: Option<String>,
  children:  Vec<Self>,
  open:      bool,
}

impl TocTree {
  pub fn insert(
    &mut self,
    path: &[TocPathPart],
    leaf_html: String,
    open_path: bool,
  ) {
    insert_path(&mut self.children, path, leaf_html, open_path);
  }

  #[must_use]
  pub fn render(&self, default_open_depth: usize) -> String {
    let mut html = String::new();
    html.push_str("<ul class=\"toc-list\">\n");
    for node in &self.children {
      render_node(&mut html, node, 2, default_open_depth);
    }
    html.push_str("</ul>\n");
    html
  }
}

fn insert_path(
  children: &mut Vec<TocNode>,
  path: &[TocPathPart],
  leaf_html: String,
  open_path: bool,
) {
  let Some((part, rest)) = path.split_first() else {
    return;
  };
  let index = children
    .iter()
    .position(|node| node.key == part.key)
    .unwrap_or_else(|| {
      children.push(TocNode {
        key: part.key.clone(),
        label: part.label.clone(),
        title: part.title.clone(),
        ..Default::default()
      });
      children.len() - 1
    });
  let node = &mut children[index];
  node.label.clone_from(&part.label);
  node.title.clone_from(&part.title);
  node.open |= open_path;

  if rest.is_empty() {
    node.leaf_html = Some(leaf_html);
  } else {
    insert_path(&mut node.children, rest, leaf_html, open_path);
  }
}

fn render_node(
  html: &mut String,
  node: &TocNode,
  indent: usize,
  default_open_depth: usize,
) {
  let spaces = " ".repeat(indent);
  if node.children.is_empty() {
    if let Some(leaf_html) = &node.leaf_html {
      let _ = writeln!(html, "{spaces}<li>{leaf_html}</li>");
    }
    return;
  }

  let safe_label = encode_text(&node.label);
  let safe_title = encode_text(&node.title);
  let count = count_leaves(node);
  let open = if indent <= default_open_depth || node.open {
    " open"
  } else {
    ""
  };
  let _ = writeln!(
    html,
    "{spaces}<li>\n{spaces}  <details \
     class=\"toc-category\"{open}>\n{spaces}    <summary \
     title=\"{safe_title}\"><span>{safe_label}</span><span \
     class=\"toc-count\">{count}</span></summary>\n{spaces}    <ul>"
  );

  if let Some(leaf_html) = &node.leaf_html {
    let _ = writeln!(html, "{spaces}      <li>{leaf_html}</li>");
  }

  for child in &node.children {
    render_node(html, child, indent + 6, default_open_depth);
  }

  let _ = writeln!(
    html,
    "{spaces}    </ul>\n{spaces}  </details>\n{spaces}</li>"
  );
}

fn count_leaves(node: &TocNode) -> usize {
  usize::from(node.leaf_html.is_some())
    + node.children.iter().map(count_leaves).sum::<usize>()
}
