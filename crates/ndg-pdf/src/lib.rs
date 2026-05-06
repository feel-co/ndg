use std::{
  collections::{BTreeMap, HashMap},
  fmt::Write as _,
  fs,
  path::{Path, PathBuf},
  sync::OnceLock,
};

use color_eyre::eyre::{Context, Result};
use ndg_commonmark::{MarkdownProcessor, ProcessorPreset, create_processor};
use ndg_utils::json::extract_value;
use printpdf::{GeneratePdfOptions, PdfDocument, PdfSaveOptions};
use serde_json::Value;

/// Generate a PDF document from Nix module options JSON.
///
/// # Errors
///
/// Returns an error if the options file cannot be read, parsed, rendered, or
/// written.
pub fn generate_pdf(
  options_path: &Path,
  output_path: Option<&Path>,
  title: Option<&str>,
  header: Option<&str>,
  footer: Option<&str>,
) -> Result<()> {
  let json_content = fs::read_to_string(options_path).wrap_err_with(|| {
    format!("Failed to read options file: {}", options_path.display())
  })?;
  let options_data: Value = serde_json::from_str(&json_content)
    .wrap_err("Failed to parse options JSON")?;

  let document_title = title.unwrap_or("Module Options");
  let output_file = output_path.map_or_else(
    || {
      let safe_title = document_title
        .to_lowercase()
        .replace(|ch: char| !ch.is_alphanumeric(), "-");
      PathBuf::from(format!("{safe_title}.pdf"))
    },
    Path::to_path_buf,
  );

  let entries: Vec<(String, Value)> = match options_data {
    Value::Object(map) => {
      let mut v: Vec<_> = map.into_iter().collect();
      v.sort_unstable_by(|(a, _), (b, _)| a.cmp(b));
      v
    },
    _ => Vec::new(),
  };

  let mut blocks = vec![
    Block::Heading {
      level:   1,
      content: vec![Inline::Text(document_title.to_string())],
    },
    blank_paragraph(),
  ];

  if let Some(header_text) = header {
    blocks.extend(parse_markdown_blocks(header_text));
    blocks.push(blank_paragraph());
  }

  for (name, value) in entries {
    let Some(option_data) = value.as_object() else {
      continue;
    };
    if option_data
      .get("internal")
      .and_then(Value::as_bool)
      .unwrap_or(false)
      || option_data
        .get("visible")
        .and_then(Value::as_bool)
        .is_some_and(|visible| !visible)
    {
      continue;
    }

    blocks.push(Block::Heading {
      level:   2,
      content: vec![Inline::Text(name)],
    });

    if let Some(type_name) = option_data.get("type").and_then(Value::as_str) {
      push_labeled_blocks(
        &mut blocks,
        "Type",
        parse_markdown_blocks(type_name),
      );
    }

    if let Some(description) =
      extract_text_field(option_data.get("description"))
    {
      push_labeled_blocks(
        &mut blocks,
        "Description",
        parse_markdown_blocks(&description),
      );
    }

    if let Some(default_text) =
      extract_option_value(option_data, "defaultText", "default")
    {
      push_labeled_blocks(
        &mut blocks,
        "Default",
        parse_markdown_blocks(&default_text),
      );
    }

    if let Some(example_text) =
      extract_option_value(option_data, "exampleText", "example")
    {
      push_labeled_blocks(
        &mut blocks,
        "Example",
        parse_markdown_blocks(&example_text),
      );
    }

    blocks.push(blank_paragraph());
  }

  if let Some(footer_text) = footer {
    blocks.push(blank_paragraph());
    blocks.extend(parse_markdown_blocks(footer_text));
  }

  let pdf_bytes = render_pdf(&blocks).wrap_err("Failed to render PDF")?;
  fs::write(&output_file, pdf_bytes).wrap_err_with(|| {
    format!("Failed to write PDF to {}", output_file.display())
  })?;
  Ok(())
}

fn push_labeled_blocks(
  blocks: &mut Vec<Block>,
  label: &str,
  mut body: Vec<Block>,
) {
  blocks.push(Block::Heading {
    level:   3,
    content: vec![Inline::Text(format!("{label}:"))],
  });
  if body.is_empty() {
    body.push(blank_paragraph());
  }
  blocks.extend(body);
}

fn extract_text_field(value: Option<&Value>) -> Option<String> {
  match value {
    Some(Value::String(text)) => Some(text.clone()),
    Some(Value::Object(obj))
      if obj.get("_type").and_then(Value::as_str) == Some("literalMD") =>
    {
      obj
        .get("text")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
    },
    _ => None,
  }
}

fn extract_option_value(
  option_data: &serde_json::Map<String, Value>,
  preferred_key: &str,
  fallback_key: &str,
) -> Option<String> {
  if let Some(Value::String(text)) = option_data.get(preferred_key) {
    return Some(text.clone());
  }

  option_data
    .get(fallback_key)
    .and_then(|value| extract_value(value, false))
}

fn blank_paragraph() -> Block {
  Block::Paragraph(vec![Inline::Text(String::new())])
}

fn markdown_processor() -> &'static MarkdownProcessor {
  static PROCESSOR: OnceLock<MarkdownProcessor> = OnceLock::new();
  PROCESSOR.get_or_init(|| create_processor(ProcessorPreset::Nixpkgs))
}

#[derive(Clone, Debug)]
enum Block {
  Heading {
    level:   u8,
    content: Vec<Inline>,
  },
  Paragraph(Vec<Inline>),
  List {
    ordered: bool,
    items:   Vec<ListItem>,
  },
  Quote(Vec<Self>),
  FencedCode(String),
  Admonition {
    title: String,
    body:  Vec<Self>,
  },
  ThematicBreak,
}

#[derive(Clone, Debug)]
struct ListItem {
  content:  Vec<Inline>,
  children: Vec<Block>,
}

#[derive(Clone, Debug)]
enum Inline {
  Text(String),
  Code(String),
  Emphasis(String),
  Strong(String),
  Link {
    label:         String,
    url:           String,
    render_as_url: bool,
  },
}

fn parse_markdown_blocks(text: &str) -> Vec<Block> {
  let normalized = normalize_admonition_block_markers(text);
  if let Some(raw_blocks) = parse_raw_structured_blocks(&normalized) {
    return raw_blocks;
  }
  parse_markdown_blocks_without_raw_admonitions(&normalized)
}

fn normalize_admonition_block_markers(text: &str) -> String {
  text.replace(" :::", "\n:::")
}

fn parse_markdown_blocks_without_raw_admonitions(text: &str) -> Vec<Block> {
  if text.trim().is_empty() {
    return Vec::new();
  }

  let html = markdown_processor().render(text).html;
  let nodes = parse_html_nodes(&html);
  let body = nodes
    .iter()
    .find(|node| node.tag.as_deref() == Some("body"))
    .map_or_else(|| nodes.as_slice(), |node| node.children.as_slice());

  body.iter().flat_map(node_to_blocks).collect()
}

fn flush_prose(prose: &mut String, out: &mut Vec<Block>) {
  if !prose.trim().is_empty() {
    out.extend(parse_markdown_blocks_without_raw_admonitions(prose));
    prose.clear();
  }
}

fn collect_body(
  lines: &[&str],
  i: &mut usize,
  is_end: impl Fn(&str) -> bool,
) -> String {
  let mut body = String::new();
  while *i < lines.len() && !is_end(lines[*i].trim()) {
    if !body.is_empty() {
      body.push('\n');
    }
    body.push_str(lines[*i]);
    *i += 1;
  }
  body
}

fn parse_raw_structured_blocks(text: &str) -> Option<Vec<Block>> {
  let lines: Vec<&str> = text.lines().collect();
  if !lines.iter().any(|line| {
    line.trim_start().starts_with(":::") || line.trim_start().starts_with("```")
  }) {
    return None;
  }

  let mut out = Vec::new();
  let mut i = 0usize;
  let mut prose = String::new();

  while i < lines.len() {
    let line = lines[i];
    let trimmed = line.trim();
    if let Some(kind) = parse_admonition_start(trimmed) {
      flush_prose(&mut prose, &mut out);
      i += 1;
      let body = collect_body(&lines, &mut i, |line| line == ":::");
      let title = kind_to_title(&kind);
      out.push(Block::Admonition {
        title,
        body: parse_markdown_blocks_without_raw_admonitions(&body),
      });
    } else if trimmed.starts_with("```") {
      flush_prose(&mut prose, &mut out);
      i += 1;
      let code = collect_body(&lines, &mut i, |line| line.starts_with("```"));
      out.push(Block::FencedCode(code));
    } else {
      if !prose.is_empty() {
        prose.push('\n');
      }
      prose.push_str(line);
    }
    i += 1;
  }

  flush_prose(&mut prose, &mut out);

  Some(out)
}

fn parse_admonition_start(line: &str) -> Option<String> {
  if !line.starts_with(":::") {
    return None;
  }
  let rest = line.trim_start_matches(':').trim();
  if rest.is_empty() {
    return Some("note".to_string());
  }
  let normalized = rest
    .trim_start_matches('{')
    .trim_end_matches('}')
    .trim()
    .trim_start_matches('.');
  if normalized.is_empty() {
    Some("note".to_string())
  } else {
    Some(normalized.to_string())
  }
}

#[derive(Clone, Debug, Default)]
struct HtmlNode {
  tag:      Option<String>,
  attrs:    HashMap<String, String>,
  children: Vec<Self>,
  text:     String,
}

fn parse_html_nodes(html: &str) -> Vec<HtmlNode> {
  let mut roots = Vec::new();
  let mut stack: Vec<HtmlNode> = Vec::new();
  let bytes = html.as_bytes();
  let mut i = 0usize;

  while i < bytes.len() {
    if bytes[i] == b'<' {
      if let Some(j) = html[i..].find('>') {
        let tag_text = &html[i + 1..i + j];
        i += j + 1;

        let tag_text = tag_text.trim();
        if tag_text.starts_with('!') {
          continue;
        }
        if tag_text.starts_with('/') {
          if let Some(node) = stack.pop() {
            if let Some(parent) = stack.last_mut() {
              parent.children.push(node);
            } else {
              roots.push(node);
            }
          }
          continue;
        }

        let self_closing = tag_text.ends_with('/');
        let tag_body = tag_text.trim_end_matches('/').trim();
        let (tag, attrs) = parse_tag_open(tag_body);

        let node = HtmlNode {
          tag: Some(tag),
          attrs,
          children: Vec::new(),
          text: String::new(),
        };

        if self_closing {
          if let Some(parent) = stack.last_mut() {
            parent.children.push(node);
          } else {
            roots.push(node);
          }
        } else {
          stack.push(node);
        }
      } else {
        break;
      }
      continue;
    }

    if let Some(j) = html[i..].find('<') {
      let raw = &html[i..i + j];
      let text = decode_html_entities(raw);
      if !text.is_empty() {
        let node = HtmlNode {
          tag: None,
          attrs: HashMap::new(),
          children: Vec::new(),
          text,
        };
        if let Some(parent) = stack.last_mut() {
          parent.children.push(node);
        } else {
          roots.push(node);
        }
      }
      i += j;
    } else {
      let text = decode_html_entities(&html[i..]);
      if !text.is_empty() {
        let node = HtmlNode {
          tag: None,
          attrs: HashMap::new(),
          children: Vec::new(),
          text,
        };
        if let Some(parent) = stack.last_mut() {
          parent.children.push(node);
        } else {
          roots.push(node);
        }
      }
      break;
    }
  }

  while let Some(node) = stack.pop() {
    if let Some(parent) = stack.last_mut() {
      parent.children.push(node);
    } else {
      roots.push(node);
    }
  }

  roots
}

fn parse_tag_open(tag: &str) -> (String, HashMap<String, String>) {
  let mut parts = tag.splitn(2, char::is_whitespace);
  let name = parts.next().unwrap_or_default().to_ascii_lowercase();
  let attrs = parts.next().map_or_else(HashMap::new, parse_attrs);
  (name, attrs)
}

fn parse_attrs(input: &str) -> HashMap<String, String> {
  let mut attrs = HashMap::new();
  let mut i = 0usize;
  let bytes = input.as_bytes();

  while i < bytes.len() {
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
      i += 1;
    }
    if i >= bytes.len() {
      break;
    }

    let start = i;
    while i < bytes.len() && !bytes[i].is_ascii_whitespace() && bytes[i] != b'='
    {
      i += 1;
    }
    let key = input[start..i].to_ascii_lowercase();

    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
      i += 1;
    }

    if i < bytes.len() && bytes[i] == b'=' {
      i += 1;
      while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
      }
      if i < bytes.len() && (bytes[i] == b'"' || bytes[i] == b'\'') {
        let quote = bytes[i];
        i += 1;
        let value_start = i;
        while i < bytes.len() && bytes[i] != quote {
          i += 1;
        }
        let value = decode_html_entities(&input[value_start..i]);
        attrs.insert(key, value);
        if i < bytes.len() {
          i += 1;
        }
      } else {
        let value_start = i;
        while i < bytes.len() && !bytes[i].is_ascii_whitespace() {
          i += 1;
        }
        attrs.insert(key, decode_html_entities(&input[value_start..i]));
      }
    } else {
      attrs.insert(key, String::new());
    }
  }

  attrs
}

fn node_to_blocks(node: &HtmlNode) -> Vec<Block> {
  match node.tag.as_deref() {
    Some("h1") => {
      vec![Block::Heading {
        level:   1,
        content: node_to_inlines(node),
      }]
    },
    Some("h2") => {
      vec![Block::Heading {
        level:   2,
        content: node_to_inlines(node),
      }]
    },
    Some("h3") => {
      vec![Block::Heading {
        level:   3,
        content: node_to_inlines(node),
      }]
    },
    Some("h4" | "h5" | "h6") => {
      vec![Block::Heading {
        level:   4,
        content: node_to_inlines(node),
      }]
    },
    Some("p") => vec![Block::Paragraph(node_to_inlines(node))],
    Some("ul") => vec![node_to_list(node, false)],
    Some("ol") => vec![node_to_list(node, true)],
    Some("blockquote") => {
      let children = node.children.iter().flat_map(node_to_blocks).collect();
      vec![Block::Quote(children)]
    },
    Some("pre") => vec![Block::FencedCode(extract_code_block(node))],
    Some("hr") => vec![Block::ThematicBreak],
    Some("div") if is_admonition(node) => vec![node_to_admonition(node)],
    Some("html" | "body") => {
      node.children.iter().flat_map(node_to_blocks).collect()
    },
    Some(_) => {
      let text = extract_text(node).trim().to_string();
      if text.is_empty() {
        Vec::new()
      } else {
        vec![Block::Paragraph(vec![Inline::Text(text)])]
      }
    },
    None => {
      let text = node.text.trim();
      if text.is_empty() {
        Vec::new()
      } else {
        vec![Block::Paragraph(vec![Inline::Text(text.to_string())])]
      }
    },
  }
}

fn node_to_list(node: &HtmlNode, ordered: bool) -> Block {
  let items = node
    .children
    .iter()
    .filter(|child| child.tag.as_deref() == Some("li"))
    .map(|li| {
      let mut content_nodes = Vec::new();
      let mut child_blocks = Vec::new();

      for child in &li.children {
        match child.tag.as_deref() {
          Some("ul") => child_blocks.push(node_to_list(child, false)),
          Some("ol") => child_blocks.push(node_to_list(child, true)),
          Some("p") => {
            if content_nodes.is_empty() {
              content_nodes.extend(child.children.clone());
            } else {
              child_blocks.push(Block::Paragraph(node_to_inlines(child)));
            }
          },
          _ => content_nodes.push(child.clone()),
        }
      }

      let content = if content_nodes.is_empty() {
        vec![Inline::Text(String::new())]
      } else {
        nodes_to_inlines(&content_nodes)
      };

      ListItem {
        content,
        children: child_blocks,
      }
    })
    .collect();

  Block::List { ordered, items }
}

fn node_to_admonition(node: &HtmlNode) -> Block {
  let class_name = node.attrs.get("class").cloned().unwrap_or_default();
  let mut title = class_name
    .split_whitespace()
    .find(|part| *part != "admonition")
    .map_or_else(|| "Note".to_string(), kind_to_title);
  let mut body = Vec::new();

  for child in &node.children {
    if child.tag.as_deref() == Some("p")
      && child.attrs.get("class").is_some_and(|class| {
        class.split_whitespace().any(|c| c == "admonition-title")
      })
    {
      let extracted = extract_text(child).trim().to_string();
      if !extracted.is_empty() {
        title = extracted;
      }
      continue;
    }
    body.extend(node_to_blocks(child));
  }

  Block::Admonition { title, body }
}

fn is_admonition(node: &HtmlNode) -> bool {
  node
    .attrs
    .get("class")
    .is_some_and(|class| class.split_whitespace().any(|c| c == "admonition"))
}

fn kind_to_title(kind: &str) -> String {
  let mut chars = kind.chars();
  chars.next().map_or_else(
    || "Note".to_string(),
    |first| format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
  )
}

fn node_to_inlines(node: &HtmlNode) -> Vec<Inline> {
  nodes_to_inlines(&node.children)
}

fn nodes_to_inlines(nodes: &[HtmlNode]) -> Vec<Inline> {
  let mut out = Vec::new();
  for child in nodes {
    match child.tag.as_deref() {
      None => {
        let normalized = normalize_ws(&child.text);
        if !normalized.is_empty() {
          out.push(Inline::Text(normalized));
        }
      },
      Some("code") => {
        let text = normalize_ws(&extract_text(child));
        if !text.is_empty() {
          out.push(Inline::Code(text));
        }
      },
      Some("em") => {
        let text = normalize_ws(&extract_text(child));
        if !text.is_empty() {
          out.push(Inline::Emphasis(text));
        }
      },
      Some("strong") => {
        let text = normalize_ws(&extract_text(child));
        if !text.is_empty() {
          out.push(Inline::Strong(text));
        }
      },
      Some("a") => {
        let label = normalize_ws(&extract_text(child));
        let url = child.attrs.get("href").cloned().unwrap_or_default();
        if !label.is_empty() {
          let class = child.attrs.get("class").cloned().unwrap_or_default();
          let is_option_reference = class
            .split_whitespace()
            .any(|value| value == "option-reference")
            || url.starts_with("options.html#option-");
          if is_option_reference {
            out.push(Inline::Code(label));
          } else {
            out.push(Inline::Link {
              label,
              url,
              render_as_url: true,
            });
          }
        }
      },
      Some("br") => out.push(Inline::Text("\n".to_string())),
      Some(_) => {
        let nested = nodes_to_inlines(&child.children);
        out.extend(nested);
      },
    }
  }
  collapse_adjacent_text(out)
}

fn collapse_adjacent_text(items: Vec<Inline>) -> Vec<Inline> {
  let mut out = Vec::new();
  for item in items {
    match (&mut out.last_mut(), item) {
      (Some(Inline::Text(prev)), Inline::Text(next)) => {
        if !prev.ends_with(' ')
          && !next.starts_with('\n')
          && !next.starts_with(' ')
        {
          prev.push(' ');
        }
        prev.push_str(&next);
      },
      (_, other) => out.push(other),
    }
  }
  out
}

fn normalize_ws(text: &str) -> String {
  let mut out = String::new();
  let mut pending_space = false;
  for ch in text.chars() {
    if ch.is_whitespace() {
      if !out.is_empty() {
        pending_space = true;
      }
    } else {
      if pending_space {
        out.push(' ');
        pending_space = false;
      }
      out.push(ch);
    }
  }
  out
}

fn extract_text(node: &HtmlNode) -> String {
  if node.tag.is_none() {
    return node.text.clone();
  }

  let mut text = String::new();
  for child in &node.children {
    if child.tag.as_deref() == Some("br") {
      text.push('\n');
      continue;
    }
    text.push_str(&extract_text(child));
  }
  text
}

fn extract_code_block(node: &HtmlNode) -> String {
  let Some(code_node) = node
    .children
    .iter()
    .find(|child| child.tag.as_deref() == Some("code"))
  else {
    return extract_text(node).trim_end().to_string();
  };

  let has_line_spans = code_node.children.iter().any(|child| {
    child.attrs.get("class").is_some_and(|class| {
      class.split_whitespace().any(|value| value == "line")
    })
  });

  if !has_line_spans {
    return extract_text(code_node).trim_end().to_string();
  }

  let mut lines = Vec::new();
  for child in &code_node.children {
    if child.attrs.get("class").is_some_and(|class| {
      class.split_whitespace().any(|value| value == "line")
    }) {
      lines.push(extract_text(child));
    }
  }

  lines.join("\n").trim_end().to_string()
}

fn decode_html_entities(input: &str) -> String {
  let mut out = String::with_capacity(input.len());
  let mut rest = input;
  while let Some(amp) = rest.find('&') {
    out.push_str(&rest[..amp]);
    let tail = &rest[amp..];
    let (entity, len) = if tail.starts_with("&lt;") {
      ("<", 4)
    } else if tail.starts_with("&gt;") {
      (">", 4)
    } else if tail.starts_with("&amp;") {
      ("&", 5)
    } else if tail.starts_with("&quot;") {
      ("\"", 6)
    } else if tail.starts_with("&apos;") {
      ("'", 6)
    } else if tail.starts_with("&#39;") {
      ("'", 5)
    } else if tail.starts_with("&nbsp;") {
      (" ", 6)
    } else {
      ("&", 1)
    };
    out.push_str(entity);
    rest = &rest[amp + len..];
  }
  out.push_str(rest);
  out
}

fn render_pdf(blocks: &[Block]) -> Result<Vec<u8>> {
  let html = render_document_html(blocks);
  let mut warnings = Vec::new();
  let document = PdfDocument::from_html(
    &html,
    &BTreeMap::new(),
    &BTreeMap::new(),
    &GeneratePdfOptions {
      page_width: Some(210.0),
      page_height: Some(297.0),
      margin_top: Some(16.0),
      margin_right: Some(18.0),
      margin_bottom: Some(18.0),
      margin_left: Some(18.0),
      ..GeneratePdfOptions::default()
    },
    &mut warnings,
  )
  .map_err(|e| color_eyre::eyre::eyre!(e))?;

  Ok(document.save(
    &PdfSaveOptions {
      optimize: false,
      ..PdfSaveOptions::default()
    },
    &mut warnings,
  ))
}

fn render_document_html(blocks: &[Block]) -> String {
  let mut html = String::from(
    r#"<!doctype html>
<html>
<head>
<meta charset="utf-8">
<style>
html {
  font-family: "DejaVu Sans", "Noto Sans", sans-serif;
  font-size: 10.8pt;
  color: #111827;
}
body {
  line-height: 1.42;
}
h1 {
  font-size: 22pt;
  margin: 0 0 18pt;
  padding-bottom: 6pt;
  border-bottom: 1.2pt solid #111827;
}
h2 {
  font-size: 14pt;
  margin: 18pt 0 8pt;
  page-break-after: avoid;
}
h3 {
  font-size: 11pt;
  margin: 12pt 0 5pt;
  page-break-after: avoid;
}
p {
  margin: 0 0 8pt;
  orphans: 3;
  widows: 3;
}
.inline-code {
  font-family: "DejaVu Sans", "Noto Sans", sans-serif;
  font-size: 0.9em;
  font-weight: 600;
  background-color: #eef2f7;
  color: #0f172a;
  border: 0.45pt solid #d5dde8;
  border-radius: 2pt;
  padding-left: 2pt;
  padding-right: 2pt;
}
pre {
  font-family: "DejaVu Sans", "Noto Sans", sans-serif;
  font-size: 9.4pt;
  line-height: 1.32;
  white-space: pre-wrap;
  background-color: #f3f6fa;
  color: #111827;
  border: 0.6pt solid #d6dee9;
  border-left: 3.2pt solid #334155;
  padding: 8pt 9pt;
  margin: 8pt 0 11pt;
  page-break-inside: avoid;
}
pre code {
  font-family: "DejaVu Sans", "Noto Sans", sans-serif;
  background: transparent;
  border: 0;
  padding: 0;
}
.admonition {
  margin: 10pt 0 12pt;
  padding: 8pt 10pt 9pt 12pt;
  background-color: #fff7df;
  border: 0.6pt solid #f0d28a;
  border-left: 4pt solid #cf6f12;
  page-break-inside: avoid;
  break-inside: avoid;
}
.admonition-title {
  font-size: 10.4pt;
  font-weight: 700;
  letter-spacing: 0.04em;
  margin: 0 0 5pt;
  color: #5f3106;
  text-transform: uppercase;
}
.admonition p:last-child {
  margin-bottom: 0;
}
blockquote {
  margin: 8pt 0 10pt;
  padding: 2pt 0 2pt 10pt;
  border-left: 2.2pt solid #94a3b8;
  color: #334155;
}
ul,
ol {
  margin: 4pt 0 9pt 18pt;
  padding: 0;
}
li {
  margin: 2pt 0;
}
a {
  color: #0f4c81;
}
hr {
  border: 0;
  border-top: 0.8pt solid #cbd5e1;
  margin: 12pt 0;
}
</style>
</head>
<body>
"#,
  );
  for block in blocks {
    render_block_html(block, &mut html);
  }
  html.push_str("</body></html>");
  html
}

fn render_block_html(block: &Block, out: &mut String) {
  match block {
    Block::Heading { level, content } => {
      let tag = match level {
        1 => "h1",
        2 => "h2",
        _ => "h3",
      };
      let _ = write!(out, "<{tag}>");
      render_inlines_html(content, out);
      let _ = writeln!(out, "</{tag}>");
    },
    Block::Paragraph(content) if paragraph_is_blank(content) => {
      out.push_str("<p>&nbsp;</p>\n");
    },
    Block::Paragraph(content) => {
      out.push_str("<p>");
      render_inlines_html(content, out);
      out.push_str("</p>\n");
    },
    Block::List { ordered, items } => {
      let tag = if *ordered { "ol" } else { "ul" };
      let _ = writeln!(out, "<{tag}>");
      for item in items {
        out.push_str("<li>");
        render_inlines_html(&item.content, out);
        for child in &item.children {
          render_block_html(child, out);
        }
        out.push_str("</li>\n");
      }
      let _ = writeln!(out, "</{tag}>");
    },
    Block::Quote(children) => {
      out.push_str("<blockquote>\n");
      for child in children {
        render_block_html(child, out);
      }
      out.push_str("</blockquote>\n");
    },
    Block::FencedCode(code) => {
      out.push_str("<pre><code>");
      escape_html_into(code, out);
      out.push_str("</code></pre>\n");
    },
    Block::Admonition { title, body } => {
      out.push_str(r#"<section class="admonition">"#);
      out.push_str(r#"<p class="admonition-title">"#);
      escape_html_into(&title.to_uppercase(), out);
      out.push_str("</p>\n");
      for child in body {
        render_block_html(child, out);
      }
      out.push_str("</section>\n");
    },
    Block::ThematicBreak => out.push_str("<hr>\n"),
  }
}

fn paragraph_is_blank(content: &[Inline]) -> bool {
  content.iter().all(|inline| {
    match inline {
      Inline::Text(text) => text.trim().is_empty(),
      _ => false,
    }
  })
}

fn render_inlines_html(inlines: &[Inline], out: &mut String) {
  for inline in inlines {
    match inline {
      Inline::Text(text) => escape_html_into(text, out),
      Inline::Code(text) => {
        out.push_str(r#"<span class="inline-code">"#);
        escape_html_into(text, out);
        out.push_str("</span>");
      },
      Inline::Emphasis(text) => {
        out.push_str("<em>");
        escape_html_into(text, out);
        out.push_str("</em>");
      },
      Inline::Strong(text) => {
        out.push_str("<strong>");
        escape_html_into(text, out);
        out.push_str("</strong>");
      },
      Inline::Link {
        label,
        url,
        render_as_url,
      } => {
        if url.is_empty() || !render_as_url {
          escape_html_into(label, out);
        } else {
          out.push_str("<a href=\"");
          escape_html_into(url, out);
          out.push_str("\">");
          escape_html_into(label, out);
          out.push_str("</a> ");
          out.push('(');
          escape_html_into(url, out);
          out.push(')');
        }
      },
    }
  }
}

fn escape_html_into(input: &str, out: &mut String) {
  for ch in input.chars() {
    match ch {
      '&' => out.push_str("&amp;"),
      '<' => out.push_str("&lt;"),
      '>' => out.push_str("&gt;"),
      '"' => out.push_str("&quot;"),
      '\'' => out.push_str("&#39;"),
      _ => out.push(ch),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_pdf_html_renderer_preserves_semantic_markdown() {
    let blocks = parse_markdown_blocks(
      "Use `null`, {option}`hjem.<user>.files`, and \
       [docs](https://example.com).\n\n::: {.note}\nImportant body.\n:::\n\n\
       ```nix\nhjem.linker = null;\n# preserved\n```",
    );
    let html = render_document_html(&blocks);

    assert!(html.contains(r#"<span class="inline-code">null</span>"#));
    assert!(
      html.contains(
        r#"<span class="inline-code">hjem.&lt;user&gt;.files</span>"#
      )
    );
    assert!(html.contains(r#"<section class="admonition">"#));
    assert!(html.contains(r#"<p class="admonition-title">NOTE</p>"#));
    assert!(
      html.contains("<pre><code>hjem.linker = null;\n# preserved</code></pre>")
    );
    assert!(html.contains(
      r#"<a href="https://example.com">docs</a> (https://example.com)"#
    ));
    assert!(!html.contains("{option}`"));
    assert!(!html.contains(":::{.note}"));
  }

  #[test]
  fn test_pdf_html_renderer_includes_visual_styles() {
    let html = render_document_html(&[Block::Admonition {
      title: "Warning".to_string(),
      body:  vec![Block::Paragraph(vec![Inline::Text(
        "Styled admonition body.".to_string(),
      )])],
    }]);

    assert!(html.contains("border-left: 4pt solid #cf6f12"));
    assert!(html.contains("background-color: #fff7df"));
    assert!(html.contains("page-break-inside: avoid"));
    assert!(html.contains(r".inline-code"));
    assert!(html.contains("border: 0.45pt solid #d5dde8"));
  }
}
