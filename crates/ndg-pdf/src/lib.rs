use std::{
  fs,
  path::{Path, PathBuf},
  sync::OnceLock,
};

use color_eyre::eyre::{Context, Result};
use html_escape::decode_html_entities;
use ndg_commonmark::{MarkdownProcessor, ProcessorPreset, create_processor};
use ndg_utils::json::extract_value;
use printpdf::{
  BuiltinFont,
  Color,
  Line,
  LinePoint,
  Mm,
  Op,
  PaintMode,
  PdfDocument,
  PdfFontHandle,
  PdfPage,
  PdfSaveOptions,
  Point,
  Pt,
  Rect,
  Rgb,
  TextItem,
  WindingOrder,
};
use rustc_hash::FxHashMap;
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

  let pdf_bytes = render_pdf(&blocks);
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

  let value = option_data.get(fallback_key)?;

  // literalExpression values are Nix code and must be shown as code blocks,
  // matching the block-level treatment in the HTML renderer.
  if let Value::Object(obj) = value {
    match obj.get("_type").and_then(Value::as_str) {
      Some("literalExpression") => {
        let text = obj.get("text").and_then(Value::as_str).unwrap_or("");
        return Some(format!("```\n{text}\n```"));
      },
      Some("literalMD") => {
        return obj
          .get("text")
          .and_then(Value::as_str)
          .map(ToOwned::to_owned);
      },
      _ => {},
    }
  }

  extract_value(value, false)
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
  attrs:    FxHashMap<String, String>,
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
      let text = decode_html_entities(raw).into_owned();
      if !text.is_empty() {
        let node = HtmlNode {
          tag: None,
          attrs: FxHashMap::default(),
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
      let text = decode_html_entities(&html[i..]).into_owned();
      if !text.is_empty() {
        let node = HtmlNode {
          tag: None,
          attrs: FxHashMap::default(),
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

fn parse_tag_open(tag: &str) -> (String, FxHashMap<String, String>) {
  let mut parts = tag.splitn(2, char::is_whitespace);
  let name = parts.next().unwrap_or_default().to_ascii_lowercase();
  let attrs = parts.next().map_or_else(FxHashMap::default, parse_attrs);
  (name, attrs)
}

fn parse_attrs(input: &str) -> FxHashMap<String, String> {
  let mut attrs = FxHashMap::default();
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
        let value = decode_html_entities(&input[value_start..i]).into_owned();
        attrs.insert(key, value);
        if i < bytes.len() {
          i += 1;
        }
      } else {
        let value_start = i;
        while i < bytes.len() && !bytes[i].is_ascii_whitespace() {
          i += 1;
        }
        attrs.insert(
          key,
          decode_html_entities(&input[value_start..i]).into_owned(),
        );
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

/// Transliterate a string to the `WinAnsiEncoding` (Windows-1252) subset that
/// PDF built-in fonts support. Characters outside Latin-1 that appear in
/// `WinAnsiEncoding`'s 0x80–0x9F range are mapped to ASCII equivalents;
/// anything else that cannot be represented falls back to `?`.
fn sanitize_for_pdf(s: &str) -> String {
  let mut out = String::with_capacity(s.len());
  for c in s.chars() {
    match c as u32 {
      0x00A0 => out.push(' '),
      0x0000..=0x007F | 0x00A1..=0x00FF => out.push(c),
      0x0152 => out.push_str("OE"),
      0x0153 => out.push_str("oe"),
      0x0160 => out.push('S'),
      0x0161 => out.push('s'),
      0x0178 => out.push('Y'),
      0x017D => out.push('Z'),
      0x017E => out.push('z'),
      0x0192 => out.push('f'),
      0x02C6 => out.push('^'),
      0x02DC => out.push('~'),
      0x2013 => out.push('-'),
      0x2014 => out.push_str("--"),
      0x2018 | 0x2019 => out.push('\''),
      0x201A => out.push(','),
      0x201C..=0x201E => out.push('"'),
      0x2020 => out.push('+'),
      0x2021 => out.push_str("++"),
      0x2022 => out.push('*'),
      0x2026 => out.push_str("..."),
      0x2039 => out.push('<'),
      0x203A => out.push('>'),
      0x20AC => out.push_str("EUR"),
      0x2122 => out.push_str("(tm)"),
      _ => out.push('?'),
    }
  }
  out
}

fn render_pdf(blocks: &[Block]) -> Vec<u8> {
  let mut builder = PdfBuilder::new();
  for block in blocks {
    builder.render_block(block);
  }
  builder.finish()
}

#[derive(Clone, Copy)]
enum FontKind {
  Regular,
  Bold,
  Italic,
  Code,
}

const fn font_handle(kind: FontKind) -> PdfFontHandle {
  PdfFontHandle::Builtin(match kind {
    FontKind::Regular => BuiltinFont::Helvetica,
    FontKind::Bold => BuiltinFont::HelveticaBold,
    FontKind::Italic => BuiltinFont::HelveticaOblique,
    FontKind::Code => BuiltinFont::Courier,
  })
}

const fn char_width_factor(kind: FontKind) -> f32 {
  match kind {
    FontKind::Regular | FontKind::Italic => 0.50,
    FontKind::Bold => 0.56,
    FontKind::Code => 0.60,
  }
}

#[expect(
  clippy::cast_precision_loss,
  reason = "char count to f32 for text width estimation is approximate by \
            design"
)]
fn text_width_pt(text: &str, kind: FontKind, size: f32) -> f32 {
  text.chars().count() as f32 * size * char_width_factor(kind)
}

#[derive(Clone)]
struct Word {
  text: String,
  font: FontKind,
}

fn inlines_to_words(inlines: &[Inline]) -> Vec<Word> {
  let mut words = Vec::new();
  for inline in inlines {
    match inline {
      Inline::Text(t) => {
        for w in t.split_whitespace() {
          words.push(Word {
            text: sanitize_for_pdf(w),
            font: FontKind::Regular,
          });
        }
      },
      Inline::Code(t) if !t.trim().is_empty() => {
        words.push(Word {
          text: sanitize_for_pdf(t),
          font: FontKind::Code,
        });
      },
      Inline::Emphasis(t) => {
        for w in t.split_whitespace() {
          words.push(Word {
            text: sanitize_for_pdf(w),
            font: FontKind::Italic,
          });
        }
      },
      Inline::Strong(t) => {
        for w in t.split_whitespace() {
          words.push(Word {
            text: sanitize_for_pdf(w),
            font: FontKind::Bold,
          });
        }
      },
      Inline::Link {
        label,
        url,
        render_as_url,
      } => {
        for w in label.split_whitespace() {
          words.push(Word {
            text: sanitize_for_pdf(w),
            font: FontKind::Regular,
          });
        }
        if *render_as_url && !url.is_empty() {
          words.push(Word {
            text: format!("({url})"),
            font: FontKind::Regular,
          });
        }
      },
      Inline::Code(_) => {},
    }
  }
  words
}

/// Returns each line as a vec of indices into `words`.
fn wrap_words(words: &[Word], max_w: f32, size: f32) -> Vec<Vec<usize>> {
  let space = size * 0.28;
  let mut lines: Vec<Vec<usize>> = Vec::new();
  let mut line: Vec<usize> = Vec::new();
  let mut line_w = 0.0f32;

  for (i, word) in words.iter().enumerate() {
    let w = text_width_pt(&word.text, word.font, size);
    let gap = if line.is_empty() { 0.0 } else { space };
    if !line.is_empty() && line_w + gap + w > max_w {
      lines.push(std::mem::take(&mut line));
      line_w = 0.0;
    }
    line_w += (if line.is_empty() { 0.0 } else { space }) + w;
    line.push(i);
  }
  if !line.is_empty() {
    lines.push(line);
  }
  lines
}

struct PdfBuilder {
  doc:      PdfDocument,
  page_ops: Vec<Op>,
  y:        f32,
  page_num: usize,
}

impl PdfBuilder {
  const PAGE_W: f32 = 595.28; // 210 mm
  const PAGE_H: f32 = 841.89; // 297 mm
  const ML: f32 = 51.02; // 18 mm left margin
  const MT: f32 = 45.35; // 16 mm top margin
  const MB: f32 = 45.35; // 16 mm bottom margin
  const CW: f32 = Self::PAGE_W - Self::ML - 51.02; // content width (18 mm right margin)
  const BS: f32 = 10.8; // body font size (pt)
  const BH: f32 = 15.3; // body line height (BS * 1.42)

  fn new() -> Self {
    Self {
      doc:      PdfDocument::new(""),
      page_ops: Vec::new(),
      y:        Self::PAGE_H - Self::MT,
      page_num: 0,
    }
  }

  fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::Rgb(Rgb {
      r:           f32::from(r) / 255.0,
      g:           f32::from(g) / 255.0,
      b:           f32::from(b) / 255.0,
      icc_profile: None,
    })
  }

  fn flush_page(&mut self) {
    if !self.page_ops.is_empty() {
      let ops = std::mem::take(&mut self.page_ops);
      self
        .doc
        .with_pages(vec![PdfPage::new(Mm(210.0), Mm(297.0), ops)]);
    }
  }

  fn new_page(&mut self) {
    self.flush_page();
    self.y = Self::PAGE_H - Self::MT;
    self.page_num += 1;
  }

  fn check_space(&mut self, needed: f32) {
    if self.y - needed < Self::MB {
      self.new_page();
    }
  }

  fn emit_words(
    &mut self,
    words: &[Word],
    indices: &[usize],
    x: f32,
    y: f32,
    size: f32,
    col: Color,
  ) {
    if indices.is_empty() {
      return;
    }
    self.page_ops.push(Op::StartTextSection);
    self.page_ops.push(Op::SetFillColor { col });
    self.page_ops.push(Op::SetTextCursor {
      pos: Point { x: Pt(x), y: Pt(y) },
    });
    let mut first = true;
    for &i in indices {
      let word = &words[i];
      let text = if first {
        word.text.clone()
      } else {
        format!(" {}", word.text)
      };
      first = false;
      self.page_ops.push(Op::SetFont {
        font: font_handle(word.font),
        size: Pt(size),
      });
      self.page_ops.push(Op::ShowText {
        items: vec![TextItem::Text(text)],
      });
    }
    self.page_ops.push(Op::EndTextSection);
  }

  fn render_text_block(
    &mut self,
    words: &[Word],
    x: f32,
    max_w: f32,
    size: f32,
    line_h: f32,
  ) {
    for indices in wrap_words(words, max_w, size) {
      self.check_space(line_h);
      self.emit_words(words, &indices, x, self.y, size, Self::rgb(17, 24, 39));
      self.y -= line_h;
    }
  }

  fn render_block(&mut self, block: &Block) {
    match block {
      Block::Heading { level, content } => self.render_heading(*level, content),
      Block::Paragraph(inlines) => {
        let words = inlines_to_words(inlines);
        if words.is_empty() {
          return;
        }
        self.render_text_block(&words, Self::ML, Self::CW, Self::BS, Self::BH);
        self.y -= 4.0;
      },
      Block::List { ordered, items } => {
        self.render_list(*ordered, items, Self::ML);
      },
      Block::Quote(children) => self.render_quote(children),
      Block::FencedCode(code) => self.render_code(code),
      Block::Admonition { title, body } => self.render_admonition(title, body),
      Block::ThematicBreak => self.render_hr(),
    }
  }

  fn render_heading(&mut self, level: u8, content: &[Inline]) {
    let (size, above, below) = match level {
      1 => (22.0_f32, 0.0_f32, 8.0_f32),
      2 => (14.0, 14.0, 6.0),
      _ => (11.0, 8.0, 4.0),
    };
    let line_h = size * 1.3;

    self.y -= above;

    let words: Vec<Word> = content
      .iter()
      .flat_map(|inline| {
        let text = match inline {
          Inline::Text(t)
          | Inline::Code(t)
          | Inline::Emphasis(t)
          | Inline::Strong(t) => t.as_str(),
          Inline::Link { label, .. } => label.as_str(),
        };
        text
          .split_whitespace()
          .filter(|w| !w.is_empty())
          .map(|w| {
            Word {
              text: sanitize_for_pdf(w),
              font: FontKind::Bold,
            }
          })
          .collect::<Vec<_>>()
      })
      .collect();

    for indices in wrap_words(&words, Self::CW, size) {
      self.check_space(line_h);
      self.emit_words(
        &words,
        &indices,
        Self::ML,
        self.y,
        size,
        Self::rgb(17, 24, 39),
      );
      self.y -= line_h;
    }

    if level == 1 {
      let rule_y = self.y + line_h * 0.5;
      self.draw_line(
        Self::ML,
        rule_y,
        Self::ML + Self::CW,
        rule_y,
        1.2,
        Self::rgb(17, 24, 39),
      );
      self.y -= 4.0;
    }

    self.y -= below;
  }

  fn render_list(&mut self, ordered: bool, items: &[ListItem], x: f32) {
    let indent = 14.0_f32;
    let text_x = x + indent;
    let max_w = Self::ML + Self::CW - text_x;

    for (i, item) in items.iter().enumerate() {
      let marker = if ordered {
        format!("{}.", i + 1)
      } else {
        "-".to_string()
      };

      let content_words = inlines_to_words(&item.content);
      let content_lines = wrap_words(&content_words, max_w, Self::BS);
      let lines_count = content_lines.len().max(1);

      #[expect(
        clippy::cast_precision_loss,
        reason = "line count to f32 for layout spacing is approximate by \
                  design"
      )]
      self.check_space(Self::BH * lines_count as f32);

      let mword = vec![Word {
        text: marker,
        font: FontKind::Regular,
      }];
      self.emit_words(&mword, &[0], x, self.y, Self::BS, Self::rgb(17, 24, 39));

      if !content_words.is_empty() {
        for (li, indices) in content_lines.iter().enumerate() {
          if li > 0 {
            self.y -= Self::BH;
          }
          self.emit_words(
            &content_words,
            indices,
            text_x,
            self.y,
            Self::BS,
            Self::rgb(17, 24, 39),
          );
        }
      }
      self.y -= Self::BH;

      for child in &item.children {
        match child {
          Block::List { ordered, items } => {
            self.render_list(*ordered, items, text_x);
          },
          _ => self.render_block(child),
        }
      }
    }
    self.y -= 4.0;
  }

  fn render_quote(&mut self, children: &[Block]) {
    let start_y = self.y;
    for child in children {
      self.render_block(child);
    }
    let end_y = self.y;
    if start_y > end_y {
      self.draw_line(
        Self::ML + 2.0,
        start_y,
        Self::ML + 2.0,
        end_y,
        2.2,
        Self::rgb(148, 163, 184),
      );
    }
  }

  #[expect(
    clippy::suboptimal_flops,
    clippy::cast_precision_loss,
    reason = "layout arithmetic using f32 approximations; precision not \
              critical for PDF rendering"
  )]
  fn render_code(&mut self, code: &str) {
    let size = 9.4_f32;
    let line_h = size * 1.32;
    let pad = 7.0;
    let bar_w = 3.2;
    let lines: Vec<&str> = code.lines().collect();
    let total_h = lines.len() as f32 * line_h + 2.0 * pad;

    if self.y - total_h < Self::MB {
      self.new_page();
    }

    let box_y = self.y - total_h;

    self.page_ops.push(Op::SaveGraphicsState);
    self.page_ops.push(Op::SetFillColor {
      col: Self::rgb(0xF3, 0xF6, 0xFA),
    });
    self.page_ops.push(Op::DrawRectangle {
      rectangle: Rect {
        x:             Pt(Self::ML - pad),
        y:             Pt(box_y),
        width:         Pt(Self::CW + 2.0 * pad),
        height:        Pt(total_h),
        mode:          Some(PaintMode::Fill),
        winding_order: Some(WindingOrder::NonZero),
      },
    });
    self.page_ops.push(Op::SetFillColor {
      col: Self::rgb(0x33, 0x41, 0x55),
    });
    self.page_ops.push(Op::DrawRectangle {
      rectangle: Rect {
        x:             Pt(Self::ML - pad),
        y:             Pt(box_y),
        width:         Pt(bar_w),
        height:        Pt(total_h),
        mode:          Some(PaintMode::Fill),
        winding_order: Some(WindingOrder::NonZero),
      },
    });
    self.page_ops.push(Op::RestoreGraphicsState);

    self.y -= pad;
    for line in &lines {
      self.page_ops.push(Op::StartTextSection);
      self.page_ops.push(Op::SetFillColor {
        col: Self::rgb(17, 24, 39),
      });
      self.page_ops.push(Op::SetFont {
        font: PdfFontHandle::Builtin(BuiltinFont::Courier),
        size: Pt(size),
      });
      self.page_ops.push(Op::SetTextCursor {
        pos: Point {
          x: Pt(Self::ML),
          y: Pt(self.y),
        },
      });
      self.page_ops.push(Op::ShowText {
        items: vec![TextItem::Text(sanitize_for_pdf(line))],
      });
      self.page_ops.push(Op::EndTextSection);
      self.y -= line_h;
    }
    self.y -= pad + 8.0;
  }

  fn render_admonition(&mut self, title: &str, body: &[Block]) {
    let bar_w = 4.0_f32;
    let pad_l = 10.0_f32;
    let pad_t = 6.0_f32;
    let pad_b = 4.0_f32;
    let t_size = 10.4_f32;
    let t_line_h = t_size * 1.4;

    let start_y = self.y;
    let start_page = self.page_num;
    let ops_start = self.page_ops.len();

    self.y -= pad_t;

    let tw = vec![Word {
      text: sanitize_for_pdf(&title.to_uppercase()),
      font: FontKind::Bold,
    }];
    self.check_space(t_line_h);
    self.emit_words(
      &tw,
      &[0],
      Self::ML + bar_w + pad_l,
      self.y,
      t_size,
      Self::rgb(0x5F, 0x31, 0x06),
    );
    self.y -= t_line_h + 3.0;

    for child in body {
      self.render_block(child);
    }
    self.y -= pad_b;

    let end_y = self.y;
    let height = start_y - end_y;

    if height > 0.0 && self.page_num == start_page {
      let content_ops: Vec<Op> = self.page_ops.drain(ops_start..).collect();

      self.page_ops.push(Op::SaveGraphicsState);
      self.page_ops.push(Op::SetFillColor {
        col: Self::rgb(0xFF, 0xF7, 0xDF),
      });
      self.page_ops.push(Op::DrawRectangle {
        rectangle: Rect {
          x:             Pt(Self::ML),
          y:             Pt(end_y),
          width:         Pt(Self::CW),
          height:        Pt(height),
          mode:          Some(PaintMode::Fill),
          winding_order: Some(WindingOrder::NonZero),
        },
      });
      self.page_ops.push(Op::SetFillColor {
        col: Self::rgb(0xCF, 0x6F, 0x12),
      });
      self.page_ops.push(Op::DrawRectangle {
        rectangle: Rect {
          x:             Pt(Self::ML),
          y:             Pt(end_y),
          width:         Pt(bar_w),
          height:        Pt(height),
          mode:          Some(PaintMode::Fill),
          winding_order: Some(WindingOrder::NonZero),
        },
      });
      self.page_ops.push(Op::RestoreGraphicsState);
      self.page_ops.extend(content_ops);
    }

    self.y -= 8.0;
  }

  fn render_hr(&mut self) {
    self.y -= 6.0;
    self.check_space(2.0);
    self.draw_line(
      Self::ML,
      self.y,
      Self::ML + Self::CW,
      self.y,
      0.8,
      Self::rgb(0xCB, 0xD5, 0xE1),
    );
    self.y -= 8.0;
  }

  fn draw_line(
    &mut self,
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
    width: f32,
    col: Color,
  ) {
    self.page_ops.push(Op::SaveGraphicsState);
    self
      .page_ops
      .push(Op::SetOutlineThickness { pt: Pt(width) });
    self.page_ops.push(Op::SetOutlineColor { col });
    self.page_ops.push(Op::DrawLine {
      line: Line {
        points:    vec![
          LinePoint {
            p:      Point {
              x: Pt(x1),
              y: Pt(y1),
            },
            bezier: false,
          },
          LinePoint {
            p:      Point {
              x: Pt(x2),
              y: Pt(y2),
            },
            bezier: false,
          },
        ],
        is_closed: false,
      },
    });
    self.page_ops.push(Op::RestoreGraphicsState);
  }

  fn finish(mut self) -> Vec<u8> {
    self.flush_page();
    let mut warnings = Vec::new();
    self.doc.save(&PdfSaveOptions::default(), &mut warnings)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_render_produces_valid_pdf() {
    let blocks = parse_markdown_blocks(
      "# Title\n\nUse `null` or `pkgs.something`.\n\n::: {.note}\nImportant \
       note.\n:::\n\n```nix\nhjem.linker = null;\n```",
    );
    let bytes = render_pdf(&blocks);
    assert!(bytes.starts_with(b"%PDF-"), "missing PDF header");
  }

  #[test]
  fn test_inline_code_and_links_parsed() {
    let blocks = parse_markdown_blocks(
      "Use `null`, {option}`hjem.<user>.files`, and [docs](https://example.com).",
    );
    // Blocks must parse without panic and contain at least one Paragraph.
    assert!(blocks.iter().any(|b| matches!(b, Block::Paragraph(_))));
    assert!(!blocks.iter().any(|b| matches!(b, Block::FencedCode(_))));
  }
}
