//! Feature-specific Markdown processing extensions.

use super::process::process_safe;
use crate::utils;

/// Apply GitHub Flavored Markdown (GFM) extensions to the input markdown.
///
/// This is a placeholder for future GFM-specific preprocessing or AST
/// transformations. In practice, most GFM features are enabled via comrak
/// options, but additional logic (such as custom tables, task lists, etc.) can
/// be added here.
///
/// # Arguments
/// * `markdown` - The input markdown text
///
/// # Returns
/// The processed markdown text with GFM extensions applied
#[cfg(feature = "gfm")]
#[must_use]
pub fn apply_gfm_extensions(markdown: &str) -> String {
  // XXX: Comrak already supports GFM, but if there is any feature in the spec
  // that is not implemented as we'd like for it to be, we can add it here.
  markdown.to_owned()
}

/// Process file includes in Nixpkgs/NixOS documentation.
///
/// This function processes file include syntax:
///
/// ````markdown
/// ```{=include=}
/// path/to/file1.md
/// path/to/file2.md
/// ```
/// ````
///
/// # Arguments
///
/// * `markdown` - The input markdown text
/// * `base_dir` - The base directory for resolving relative file paths
///
/// # Returns
///
/// The processed markdown text with included files expanded
///
/// # Safety
///
/// Only relative paths without ".." are allowed for security.
#[cfg(feature = "nixpkgs")]
#[must_use]
pub fn process_file_includes(
  markdown: &str,
  base_dir: &std::path::Path,
) -> String {
  use std::{fs, path::Path};

  // Check if a path is safe (no absolute, no ..)
  fn is_safe_path(path: &str) -> bool {
    let p = Path::new(path);
    !p.is_absolute() && !path.contains("..") && !path.contains('\\')
  }

  // Read included files, return concatenated content
  fn read_includes(listing: &str, base_dir: &Path) -> String {
    let mut result = String::new();
    for line in listing.lines() {
      let trimmed = line.trim();
      if trimmed.is_empty() || !is_safe_path(trimmed) {
        continue;
      }
      let full_path = base_dir.join(trimmed);
      log::info!("Including file: {}", full_path.display());
      match fs::read_to_string(&full_path) {
        Ok(content) => {
          result.push_str(&content);
          if !content.ends_with('\n') {
            result.push('\n');
          }
        },
        Err(_) => {
          // Insert a warning comment for missing files
          result.push_str(&format!(
            "<!-- ndg: could not include file: {} -->\n",
            full_path.display()
          ));
        },
      }
    }
    result
  }

  // Replace {=include=} code blocks with included file contents
  let mut output = String::new();
  let mut lines = markdown.lines().peekable();
  let mut in_code_block = false;
  let mut code_fence_char = None;
  let mut code_fence_count = 0;

  while let Some(line) = lines.next() {
    let trimmed = line.trim_start();

    // Check for includes BEFORE checking for code fences
    if !in_code_block && trimmed.starts_with("```{=include=}") {
      // Start of an include block
      let mut include_listing = String::new();
      for next_line in lines.by_ref() {
        if next_line.trim_start().starts_with("```") {
          break;
        }
        include_listing.push_str(next_line);
        include_listing.push('\n');
      }

      let included = read_includes(&include_listing, base_dir);
      output.push_str(&included);
      continue;
    }

    // Check for code fences
    if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
      let fence_char = trimmed.chars().next().unwrap();
      let fence_count =
        trimmed.chars().take_while(|&c| c == fence_char).count();

      if fence_count >= 3 {
        if !in_code_block {
          // Starting a code block
          in_code_block = true;
          code_fence_char = Some(fence_char);
          code_fence_count = fence_count;
        } else if code_fence_char == Some(fence_char)
          && fence_count >= code_fence_count
        {
          // Ending a code block
          in_code_block = false;
          code_fence_char = None;
          code_fence_count = 0;
        }
      }
    }

    // Regular line, keep as-is
    output.push_str(line);
    output.push('\n');
  }

  output
}

/// Process role markup in markdown content.
///
/// This function processes role syntax like `{command}`ls -la``
///
/// # Arguments
///
/// * `content` - The markdown content to process
/// * `manpage_urls` - Optional mapping of manpage names to URLs
///
/// # Returns
///
/// The processed markdown with role markup converted to HTML
#[cfg(any(feature = "nixpkgs", feature = "ndg-flavored"))]
#[must_use]
pub fn process_role_markup(
  content: &str,
  manpage_urls: Option<&std::collections::HashMap<String, String>>,
) -> String {
  let mut result = String::new();
  let mut chars = content.chars().peekable();
  let mut in_code_block = false;
  let mut in_inline_code = false;
  let mut code_fence_char = None;
  let mut code_fence_count = 0;

  while let Some(ch) = chars.next() {
    // Handle code fences (```)
    if ch == '`' {
      let mut tick_count = 1;
      while chars.peek() == Some(&'`') {
        chars.next();
        tick_count += 1;
      }

      if tick_count >= 3 {
        // This is a code fence
        if !in_code_block {
          // Starting a code block
          in_code_block = true;
          code_fence_char = Some('`');
          code_fence_count = tick_count;
        } else if code_fence_char == Some('`') && tick_count >= code_fence_count
        {
          // Ending a code block
          in_code_block = false;
          code_fence_char = None;
          code_fence_count = 0;
        }
      } else if tick_count == 1 && !in_code_block {
        // Single backtick - inline code
        in_inline_code = !in_inline_code;
      }

      // Add all the backticks
      result.push_str(&"`".repeat(tick_count));
      continue;
    }

    // Handle tilde code fences (~~~)
    if ch == '~' && chars.peek() == Some(&'~') {
      let mut tilde_count = 1;
      while chars.peek() == Some(&'~') {
        chars.next();
        tilde_count += 1;
      }

      if tilde_count >= 3 {
        if !in_code_block {
          in_code_block = true;
          code_fence_char = Some('~');
          code_fence_count = tilde_count;
        } else if code_fence_char == Some('~')
          && tilde_count >= code_fence_count
        {
          in_code_block = false;
          code_fence_char = None;
          code_fence_count = 0;
        }
      }

      result.push_str(&"~".repeat(tilde_count));
      continue;
    }

    // Handle newlines
    // They can end inline code if not properly closed
    if ch == '\n' {
      in_inline_code = false;
      result.push(ch);
      continue;
    }

    // Process role markup only if we're not in any kind of code
    if ch == '{' && !in_code_block && !in_inline_code {
      // Collect remaining characters to test parsing
      let remaining: Vec<char> = chars.clone().collect();
      let remaining_str: String = remaining.iter().collect();
      let mut temp_chars = remaining_str.chars().peekable();

      if let Some(role_markup) =
        parse_role_markup(&mut temp_chars, manpage_urls)
      {
        // Valid role markup found, advance the main iterator
        let remaining_after_parse: String = temp_chars.collect();
        let consumed = remaining_str.len() - remaining_after_parse.len();
        for _ in 0..consumed {
          chars.next();
        }
        result.push_str(&role_markup);
      } else {
        // Not a valid role markup, keep the original character
        result.push(ch);
      }
    } else {
      result.push(ch);
    }
  }

  result
}

/// Parse a role markup from the character iterator.
///
/// # Returns
///
/// `Some(html)` if a valid role markup is found, `None` otherwise.
fn parse_role_markup(
  chars: &mut std::iter::Peekable<std::str::Chars>,
  manpage_urls: Option<&std::collections::HashMap<String, String>>,
) -> Option<String> {
  let mut role_name = String::new();

  // Parse role name (lowercase letters only)
  while let Some(&ch) = chars.peek() {
    if ch.is_ascii_lowercase() {
      role_name.push(ch);
      chars.next();
    } else {
      break;
    }
  }

  // Must have a non-empty role name
  if role_name.is_empty() {
    return None;
  }

  // Expect closing brace
  if chars.peek() != Some(&'}') {
    return None;
  }
  chars.next(); // consume '}'

  // Expect opening backtick
  if chars.peek() != Some(&'`') {
    return None;
  }
  chars.next(); // consume '`'

  // Parse content until closing backtick
  let mut content = String::new();
  for ch in chars.by_ref() {
    if ch == '`' {
      // Found closing backtick, validate content
      // Most role types should not have empty content
      if content.is_empty() && !matches!(role_name.as_str(), "manpage") {
        return None; // reject empty content for most roles
      }
      return Some(format_role_markup(&role_name, &content, manpage_urls));
    }
    content.push(ch);
  }

  // No closing backtick found
  None
}

/// Format the role markup as HTML based on the role type and content.
#[must_use]
pub fn format_role_markup(
  role_type: &str,
  content: &str,
  manpage_urls: Option<&std::collections::HashMap<String, String>>,
) -> String {
  let escaped_content = utils::html_escape(content);
  match role_type {
    "manpage" => {
      if let Some(urls) = manpage_urls {
        if let Some(url) = urls.get(content) {
          let clean_url = extract_url_from_html(url);
          format!(
            "<a href=\"{clean_url}\" \
             class=\"manpage-reference\">{escaped_content}</a>"
          )
        } else {
          format!("<span class=\"manpage-reference\">{escaped_content}</span>")
        }
      } else {
        format!("<span class=\"manpage-reference\">{escaped_content}</span>")
      }
    },
    "command" => format!("<code class=\"command\">{escaped_content}</code>"),
    "env" => format!("<code class=\"env-var\">{escaped_content}</code>"),
    "file" => format!("<code class=\"file-path\">{escaped_content}</code>"),
    "option" => {
      if cfg!(feature = "ndg-flavored") {
        let option_id = format!("option-{}", content.replace('.', "-"));
        format!(
          "<a class=\"option-reference\" \
           href=\"options.html#{option_id}\"><code>{escaped_content}</code></\
           a>"
        )
      } else {
        format!("<code>{escaped_content}</code>")
      }
    },
    "var" => format!("<code class=\"nix-var\">{escaped_content}</code>"),
    _ => format!("<span class=\"{role_type}-markup\">{escaped_content}</span>"),
  }
}

/// Process inline anchors in markdown content.
///
/// This function processes inline anchor syntax like `[]{#my-anchor}` while
/// being code-block aware to avoid processing inside code fences.
///
/// # Arguments
/// * `content` - The markdown content to process
///
/// # Returns
/// The processed markdown with inline anchors converted to HTML spans
#[cfg(feature = "nixpkgs")]
#[must_use]
pub fn process_inline_anchors(content: &str) -> String {
  let mut result = String::with_capacity(content.len() + 100);
  let mut in_code_block = false;
  let mut code_fence_char = None;
  let mut code_fence_count = 0;

  for line in content.lines() {
    let trimmed = line.trim_start();

    // Check for code fences
    if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
      let fence_char = trimmed.chars().next().unwrap();
      let fence_count =
        trimmed.chars().take_while(|&c| c == fence_char).count();

      if fence_count >= 3 {
        if !in_code_block {
          // Starting a code block
          in_code_block = true;
          code_fence_char = Some(fence_char);
          code_fence_count = fence_count;
        } else if code_fence_char == Some(fence_char)
          && fence_count >= code_fence_count
        {
          // Ending a code block
          in_code_block = false;
          code_fence_char = None;
          code_fence_count = 0;
        }
      }
    }

    // Only process inline anchors if we're not in a code block
    if in_code_block {
      // In code block, keep line as-is
      result.push_str(line);
      result.push('\n');
    } else {
      // Check for list items with anchors:
      // "- []{#id} content" or "1. []{#id} content"
      if let Some(anchor_start) = find_list_item_anchor(trimmed) {
        if let Some(processed_line) =
          process_list_item_anchor(line, anchor_start)
        {
          result.push_str(&processed_line);
          result.push('\n');
          continue;
        }
      }

      // Process regular inline anchors in the line
      result.push_str(&process_line_anchors(line));
      result.push('\n');
    }
  }

  result
}

/// Find if a line starts with a list marker followed by an anchor.
fn find_list_item_anchor(trimmed: &str) -> Option<usize> {
  // Check for unordered list: "- []{#id}" or "* []{#id}" or "+ []{#id}"
  if (trimmed.starts_with("- ")
    || trimmed.starts_with("* ")
    || trimmed.starts_with("+ "))
    && trimmed.len() > 2
  {
    let after_marker = &trimmed[2..];
    if after_marker.starts_with("[]{#") {
      return Some(2);
    }
  }

  // Check for ordered list: "1. []{#id}" or "123. []{#id}"
  let mut i = 0;
  while i < trimmed.len()
    && trimmed.chars().nth(i).unwrap_or(' ').is_ascii_digit()
  {
    i += 1;
  }
  if i > 0 && i < trimmed.len() - 1 && trimmed.chars().nth(i) == Some('.') {
    let after_marker = &trimmed[i + 1..];
    if after_marker.starts_with(" []{#") {
      return Some(i + 2);
    }
  }

  None
}

/// Process a list item line that contains an anchor.
fn process_list_item_anchor(line: &str, anchor_start: usize) -> Option<String> {
  let before_anchor = &line[..anchor_start];
  let after_marker = &line[anchor_start..];

  if !after_marker.starts_with("[]{#") {
    return None;
  }

  // Find the end of the anchor: []{#id}
  if let Some(anchor_end) = after_marker.find('}') {
    let id = &after_marker[4..anchor_end]; // skip "[]{#" and take until '}'
    let remaining_content = &after_marker[anchor_end + 1..]; // skip '}'

    // Validate ID contains only allowed characters
    if id
      .chars()
      .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
      && !id.is_empty()
    {
      return Some(format!(
        "{before_anchor}<span id=\"{id}\" \
         class=\"nixos-anchor\"></span>{remaining_content}"
      ));
    }
  }

  None
}

/// Process inline anchors in a single line.
fn process_line_anchors(line: &str) -> String {
  let mut result = String::with_capacity(line.len());
  let mut chars = line.chars().peekable();

  while let Some(ch) = chars.next() {
    if ch == '[' && chars.peek() == Some(&']') {
      chars.next(); // consume ']'

      // Check for {#id} pattern
      if chars.peek() == Some(&'{') {
        chars.next(); // consume '{'
        if chars.peek() == Some(&'#') {
          chars.next(); // consume '#'

          // Collect the ID
          let mut id = String::new();
          while let Some(&next_ch) = chars.peek() {
            if next_ch == '}' {
              chars.next(); // consume '}'

              // Validate ID and create span
              if !id.is_empty()
                && id
                  .chars()
                  .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
              {
                result.push_str(&format!(
                  "<span id=\"{id}\" class=\"nixos-anchor\"></span>"
                ));
              } else {
                // Invalid ID, put back original text
                result.push_str(&format!("[]{{{{#{id}}}}}"));
              }
              break;
            } else if next_ch.is_ascii_alphanumeric()
              || next_ch == '-'
              || next_ch == '_'
            {
              id.push(next_ch);
              chars.next();
            } else {
              // Invalid character, put back original text
              result.push_str(&format!("[]{{{{#{id}"));
              break;
            }
          }
        } else {
          // Not an anchor, put back consumed characters
          result.push_str("]{");
        }
      } else {
        // Not an anchor, put back consumed character
        result.push(']');
      }
    } else {
      result.push(ch);
    }
  }

  result
}

/// Process block elements in markdown content.
///
/// This function processes block elements including admonitions, figures, and
/// definition lists while being code-block aware to avoid processing inside
/// code fences.
///
/// # Arguments
/// * `content` - The markdown content to process
///
/// # Returns
/// The processed markdown with block elements converted to HTML
#[cfg(feature = "nixpkgs")]
#[must_use]
pub fn process_block_elements(content: &str) -> String {
  let mut result = Vec::new();
  let mut lines = content.lines().peekable();
  let mut in_code_block = false;
  let mut code_fence_char = None;
  let mut code_fence_count = 0;

  while let Some(line) = lines.next() {
    // Check for code fences
    let trimmed = line.trim_start();
    if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
      let fence_char = trimmed.chars().next().unwrap();
      let fence_count =
        trimmed.chars().take_while(|&c| c == fence_char).count();

      if fence_count >= 3 {
        if !in_code_block {
          // Starting a code block
          in_code_block = true;
          code_fence_char = Some(fence_char);
          code_fence_count = fence_count;
        } else if code_fence_char == Some(fence_char)
          && fence_count >= code_fence_count
        {
          // Ending a code block
          in_code_block = false;
          code_fence_char = None;
          code_fence_count = 0;
        }
      }
    }

    // Only process block elements if we're not in a code block
    if !in_code_block {
      // Check for GitHub-style callouts: > [!TYPE]
      if let Some((callout_type, initial_content)) = parse_github_callout(line)
      {
        let content =
          collect_github_callout_content(&mut lines, initial_content);
        let admonition = render_admonition(&callout_type, None, &content);
        result.push(admonition);
        continue;
      }

      // Check for fenced admonitions: ::: {.type}
      if let Some((adm_type, id)) = parse_fenced_admonition_start(line) {
        let content = collect_fenced_content(&mut lines);
        let admonition = render_admonition(&adm_type, id.as_deref(), &content);
        result.push(admonition);
        continue;
      }

      // Check for figures: ::: {.figure #id}
      if let Some((id, title, content)) = parse_figure_block(line, &mut lines) {
        let figure = render_figure(id.as_deref(), &title, &content);
        result.push(figure);
        continue;
      }
    }

    // Regular line, keep as-is
    result.push(line.to_string());
  }

  result.join("\n")
}

/// Parse GitHub-style callout syntax: > [!TYPE] content
fn parse_github_callout(line: &str) -> Option<(String, String)> {
  let trimmed = line.trim_start();
  if !trimmed.starts_with("> [!") {
    return None;
  }

  // Find the closing bracket
  if let Some(close_bracket) = trimmed.find(']') {
    if close_bracket > 4 {
      let callout_type = &trimmed[4..close_bracket];

      // Validate callout type
      match callout_type {
        "NOTE" | "TIP" | "IMPORTANT" | "WARNING" | "CAUTION" | "DANGER" => {
          let content = trimmed[close_bracket + 1..].trim();
          return Some((callout_type.to_lowercase(), content.to_string()));
        },
        _ => return None,
      }
    }
  }

  None
}

/// Collect content for GitHub-style callouts
fn collect_github_callout_content(
  lines: &mut std::iter::Peekable<std::str::Lines>,
  initial_content: String,
) -> String {
  let mut content = String::new();

  if !initial_content.is_empty() {
    content.push_str(&initial_content);
    content.push('\n');
  }

  while let Some(line) = lines.peek() {
    let trimmed = line.trim_start();
    if trimmed.starts_with('>') {
      let content_part = trimmed.strip_prefix('>').unwrap_or("").trim_start();
      content.push_str(content_part);
      content.push('\n');
      lines.next(); // consume the line
    } else {
      break;
    }
  }

  content.trim().to_string()
}

/// Parse fenced admonition start: ::: {.type #id}
fn parse_fenced_admonition_start(
  line: &str,
) -> Option<(String, Option<String>)> {
  let trimmed = line.trim();
  if !trimmed.starts_with(":::") {
    return None;
  }

  let after_colons = trimmed[3..].trim_start();
  if !after_colons.starts_with("{.") {
    return None;
  }

  // Find the closing brace
  if let Some(close_brace) = after_colons.find('}') {
    let content = &after_colons[2..close_brace]; // Skip "{."

    // Parse type and optional ID
    let parts: Vec<&str> = content.split_whitespace().collect();
    if let Some(&adm_type) = parts.first() {
      let id = parts
        .iter()
        .find(|part| part.starts_with('#'))
        .map(|id_part| id_part[1..].to_string()); // Remove '#'

      return Some((adm_type.to_string(), id));
    }
  }

  None
}

/// Collect content until closing :::
fn collect_fenced_content(
  lines: &mut std::iter::Peekable<std::str::Lines>,
) -> String {
  let mut content = String::new();

  for line in lines.by_ref() {
    if line.trim().starts_with(":::") {
      break;
    }
    content.push_str(line);
    content.push('\n');
  }

  content.trim().to_string()
}

/// Parse figure block: ::: {.figure #id}
fn parse_figure_block(
  line: &str,
  lines: &mut std::iter::Peekable<std::str::Lines>,
) -> Option<(Option<String>, String, String)> {
  let trimmed = line.trim();
  if !trimmed.starts_with(":::") {
    return None;
  }

  let after_colons = trimmed[3..].trim_start();
  if !after_colons.starts_with("{.figure") {
    return None;
  }

  // Extract ID if present
  let id = if let Some(hash_pos) = after_colons.find('#') {
    if let Some(close_brace) = after_colons.find('}') {
      if hash_pos < close_brace {
        Some(after_colons[hash_pos + 1..close_brace].trim().to_string())
      } else {
        None
      }
    } else {
      None
    }
  } else {
    None
  };

  // Get title from next line (should start with #)
  let title = if let Some(title_line) = lines.next() {
    let trimmed_title = title_line.trim();
    if let Some(this) = trimmed_title.strip_prefix('#') {
      { this.trim_matches(char::is_whitespace) }.to_string()
    } else {
      // Put the line back if it's not a title
      return None;
    }
  } else {
    return None;
  };

  // Collect figure content
  let mut content = String::new();
  for line in lines.by_ref() {
    if line.trim().starts_with(":::") {
      break;
    }
    content.push_str(line);
    content.push('\n');
  }

  Some((id, title, content.trim().to_string()))
}

/// Render an admonition as HTML
fn render_admonition(
  adm_type: &str,
  id: Option<&str>,
  content: &str,
) -> String {
  let capitalized_type = crate::utils::capitalize_first(adm_type);
  let id_attr = id.map_or(String::new(), |id| format!(" id=\"{id}\""));

  format!(
    "<div class=\"admonition {adm_type}\"{id_attr}>\n<p \
     class=\"admonition-title\">{capitalized_type}</p>\n\n{content}\n\n</div>"
  )
}

/// Render a figure as HTML
fn render_figure(id: Option<&str>, title: &str, content: &str) -> String {
  let id_attr = id.map_or(String::new(), |id| format!(" id=\"{id}\""));

  format!(
    "<figure{id_attr}>\n<figcaption>{title}</figcaption>\n{content}\n</figure>"
  )
}

/// Process manpage references in HTML content.
///
/// This function processes manpage references by finding span elements with
/// manpage-reference class and converting them to links when URLs are
/// available.
///
/// # Arguments
/// * `html` - The HTML content to process
/// * `manpage_urls` - Optional mapping of manpage names to URLs
///
/// # Returns
/// The processed HTML with manpage references converted to links
#[cfg(feature = "nixpkgs")]
#[must_use]
pub fn process_manpage_references(
  html: &str,
  manpage_urls: Option<&std::collections::HashMap<String, String>>,
) -> String {
  process_safe(
    html,
    |html| {
      use kuchikikiki::NodeRef;
      use tendril::TendrilSink;

      let document = kuchikikiki::parse_html().one(html);
      let mut to_replace = Vec::new();

      // Find all spans with class "manpage-reference"
      for span_node in document.select("span.manpage-reference").unwrap() {
        let span_el = span_node.as_node();
        let span_text = span_el.text_contents();

        if let Some(urls) = manpage_urls {
          // Check for direct URL match
          if let Some(url) = urls.get(&span_text) {
            let clean_url = extract_url_from_html(url);
            let link = NodeRef::new_element(
              markup5ever::QualName::new(
                None,
                markup5ever::ns!(html),
                markup5ever::local_name!("a"),
              ),
              vec![
                (
                  kuchikikiki::ExpandedName::new("", "href"),
                  kuchikikiki::Attribute {
                    prefix: None,
                    value:  clean_url.into(),
                  },
                ),
                (
                  kuchikikiki::ExpandedName::new("", "class"),
                  kuchikikiki::Attribute {
                    prefix: None,
                    value:  "manpage-reference".into(),
                  },
                ),
              ],
            );
            link.append(NodeRef::new_text(span_text.clone()));
            to_replace.push((span_el.clone(), link));
          }
        }
      }

      // Apply replacements
      for (old, new) in to_replace {
        old.insert_before(new);
        old.detach();
      }

      let mut out = Vec::new();
      document.serialize(&mut out).ok();
      String::from_utf8(out).unwrap_or_default()
    },
    // Return original HTML on error
    "",
  )
}

/// Process option references
/// Rewrites NixOS/Nix option references in HTML output.
///
/// This scans the HTML for `<code>option.path</code>` elements that look like
/// NixOS/Nix option references and replaces them with option reference links.
/// Only processes plain `<code>` elements that don't already have specific role
/// classes.
///
/// # Arguments
///
/// * `html` - The HTML string to process.
///
/// # Returns
///
/// The HTML string with option references rewritten as links.
#[cfg(feature = "ndg-flavored")]
#[must_use]
pub fn process_option_references(html: &str) -> String {
  use kuchikikiki::{Attribute, ExpandedName, NodeRef};
  use markup5ever::{QualName, local_name, ns};
  use tendril::TendrilSink;

  process_safe(
    html,
    |html| {
      let document = kuchikikiki::parse_html().one(html);

      let mut to_replace = vec![];

      for code_node in document.select("code").unwrap() {
        let code_el = code_node.as_node();
        let code_text = code_el.text_contents();

        // Skip if this code element already has a role-specific class
        if let Some(element) = code_el.as_element() {
          if let Some(class_attr) =
            element.attributes.borrow().get(local_name!("class"))
          {
            if class_attr.contains("command")
              || class_attr.contains("env-var")
              || class_attr.contains("file-path")
              || class_attr.contains("nixos-option")
              || class_attr.contains("nix-var")
            {
              continue;
            }
          }
        }

        // Skip if this code element is already inside an option-reference link
        let mut is_already_option_ref = false;
        let mut current = code_el.parent();
        while let Some(parent) = current {
          if let Some(element) = parent.as_element() {
            if element.name.local == local_name!("a") {
              if let Some(class_attr) =
                element.attributes.borrow().get(local_name!("class"))
              {
                if class_attr.contains("option-reference") {
                  is_already_option_ref = true;
                  break;
                }
              }
            }
          }
          current = parent.parent();
        }

        if !is_already_option_ref && is_nixos_option_reference(&code_text) {
          let option_id = format!("option-{}", code_text.replace('.', "-"));
          let attrs = vec![
            (ExpandedName::new("", "href"), Attribute {
              prefix: None,
              value:  format!("options.html#{option_id}"),
            }),
            (ExpandedName::new("", "class"), Attribute {
              prefix: None,
              value:  "option-reference".into(),
            }),
          ];
          let a = NodeRef::new_element(
            QualName::new(None, ns!(html), local_name!("a")),
            attrs,
          );
          let code = NodeRef::new_element(
            QualName::new(None, ns!(html), local_name!("code")),
            vec![],
          );
          code.append(NodeRef::new_text(code_text.clone()));
          a.append(code);
          to_replace.push((code_el.clone(), a));
        }
      }

      for (old, new) in to_replace {
        old.insert_before(new);
        old.detach();
      }

      let mut out = Vec::new();
      document.serialize(&mut out).ok();
      String::from_utf8(out).unwrap_or_default()
    },
    // Return original HTML on error
    "",
  )
}

/// Check if a string looks like a `NixOS` option reference
fn is_nixos_option_reference(text: &str) -> bool {
  // Must have at least 2 dots and no whitespace
  let dot_count = text.chars().filter(|&c| c == '.').count();
  if dot_count < 2 || text.chars().any(char::is_whitespace) {
    return false;
  }

  // Must not contain special characters that indicate it's not an option
  if text.contains('<')
    || text.contains('>')
    || text.contains('$')
    || text.contains('/')
  {
    return false;
  }

  // Must start with a letter (options don't start with numbers or special
  // chars)
  if !text.chars().next().is_some_and(char::is_alphabetic) {
    return false;
  }

  // Must look like a structured option path (letters, numbers, dots, dashes,
  // underscores)
  text
    .chars()
    .all(|c| c.is_alphanumeric() || c == '.' || c == '-' || c == '_')
}

/// Extract URL from HTML anchor tag or return the string as-is if it's a plain
/// URL
fn extract_url_from_html(url_or_html: &str) -> &str {
  // Check if it looks like HTML (starts with <a href=")
  if url_or_html.starts_with("<a href=\"") {
    // Extract the URL from href attribute
    if let Some(start) = url_or_html.find("href=\"") {
      let start = start + 6; // Skip 'href="'
      if let Some(end) = url_or_html[start..].find('"') {
        return &url_or_html[start..start + end];
      }
    }
  }

  // Return as-is if not HTML or if extraction fails
  url_or_html
}
