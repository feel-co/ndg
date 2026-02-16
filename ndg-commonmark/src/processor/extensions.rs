//! Feature-specific Markdown processing extensions.
use std::{fmt::Write, fs, path::Path};

use html_escape::encode_text;

use super::process::process_safe;

/// Safely select DOM elements with graceful error handling.
fn safe_select(
  document: &kuchikikiki::NodeRef,
  selector: &str,
) -> Vec<kuchikikiki::NodeRef> {
  match document.select(selector) {
    Ok(selections) => selections.map(|sel| sel.as_node().clone()).collect(),
    Err(e) => {
      log::warn!("DOM selector '{selector}' failed: {e:?}");
      Vec::new()
    },
  }
}

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

/// Maximum recursion depth for file includes to prevent infinite recursion.
const MAX_INCLUDE_DEPTH: usize = 8;

/// Check if a path is safe for file inclusion (no absolute paths, no parent
/// directory traversal).
#[cfg(feature = "nixpkgs")]
fn is_safe_path(path: &str, _base_dir: &Path) -> bool {
  let p = Path::new(path);
  if p.is_absolute() || path.contains('\\') {
    return false;
  }

  // Reject any path containing parent directory components
  for component in p.components() {
    if matches!(component, std::path::Component::ParentDir) {
      return false;
    }
  }

  true
}

/// Parse the custom output directive from an include block.
#[cfg(feature = "nixpkgs")]
#[allow(
  clippy::option_if_let_else,
  reason = "Nested options are clearer with if-let"
)]
fn parse_include_directive(line: &str) -> Option<String> {
  if let Some(start) = line.find("html:into-file=") {
    let start = start + "html:into-file=".len();
    if let Some(end) = line[start..].find(' ') {
      Some(line[start..start + end].to_string())
    } else {
      Some(line[start..].trim().to_string())
    }
  } else {
    None
  }
}

/// Read and process files listed in an include block.
#[cfg(feature = "nixpkgs")]
#[allow(
  clippy::needless_pass_by_value,
  reason = "Owned value needed for cloning in loop"
)]
fn read_includes(
  listing: &str,
  base_dir: &Path,
  custom_output: Option<String>,
  included_files: &mut Vec<crate::types::IncludedFile>,
  depth: usize,
) -> Result<String, String> {
  let mut result = String::new();

  for line in listing.lines() {
    let trimmed = line.trim();
    if trimmed.is_empty() || !is_safe_path(trimmed, base_dir) {
      continue;
    }
    let full_path = base_dir.join(trimmed);
    log::info!("Including file: {}", full_path.display());

    match fs::read_to_string(&full_path) {
      Ok(content) => {
        let file_dir = full_path.parent().unwrap_or(base_dir);
        let (processed_content, nested_includes) =
          process_file_includes(&content, file_dir, depth + 1)?;

        result.push_str(&processed_content);
        if !processed_content.ends_with('\n') {
          result.push('\n');
        }

        included_files.push(crate::types::IncludedFile {
          path:          trimmed.to_string(),
          custom_output: custom_output.clone(),
        });

        // Normalize nested include paths relative to original base_dir
        for nested in nested_includes {
          let nested_full_path = file_dir.join(&nested.path);
          if let Ok(normalized_path) = nested_full_path.strip_prefix(base_dir) {
            included_files.push(crate::types::IncludedFile {
              path:          normalized_path.to_string_lossy().to_string(),
              custom_output: nested.custom_output,
            });
          }
        }
      },
      Err(_) => {
        let _ = writeln!(
          result,
          "<!-- ndg: could not include file: {} -->",
          full_path.display()
        );
      },
    }
  }
  Ok(result)
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
/// * `depth` - Current recursion depth (use 0 for initial call)
///
/// # Returns
///
/// Returns `Ok((processed_markdown, included_files))` where `included_files` is
/// a list of all successfully included files.
///
/// # Errors
///
/// Returns `Err(message)` if recursion depth exceeds [`MAX_INCLUDE_DEPTH`],
/// which likely indicates a circular include cycle.
///
/// # Safety
///
/// Only relative paths without ".." are allowed for security.
#[cfg(feature = "nixpkgs")]
pub fn process_file_includes(
  markdown: &str,
  base_dir: &std::path::Path,
  depth: usize,
) -> Result<(String, Vec<crate::types::IncludedFile>), String> {
  // Check recursion depth limit
  if depth >= MAX_INCLUDE_DEPTH {
    return Err(format!(
      "Maximum include recursion depth ({MAX_INCLUDE_DEPTH}) exceeded. This \
       likely indicates a cycle in file includes."
    ));
  }

  let mut output = String::new();
  let mut lines = markdown.lines();
  let mut fence_tracker = crate::utils::codeblock::FenceTracker::new();
  let mut all_included_files: Vec<crate::types::IncludedFile> = Vec::new();

  while let Some(line) = lines.next() {
    let trimmed = line.trim_start();

    if !fence_tracker.in_code_block() && trimmed.starts_with("```{=include=}") {
      let custom_output = parse_include_directive(trimmed);

      let mut include_listing = String::new();
      for next_line in lines.by_ref() {
        if next_line.trim_start().starts_with("```") {
          break;
        }
        include_listing.push_str(next_line);
        include_listing.push('\n');
      }

      let included = read_includes(
        &include_listing,
        base_dir,
        custom_output,
        &mut all_included_files,
        depth,
      )?;
      output.push_str(&included);
      continue;
    }

    // Update fence tracking state
    fence_tracker = fence_tracker.process_line(line);

    output.push_str(line);
    output.push('\n');
  }

  Ok((output, all_included_files))
}

/// Process role markup in markdown content.
///
/// This function processes role syntax like `{command}ls -la`
///
/// # Arguments
///
/// * `content` - The markdown content to process
/// * `manpage_urls` - Optional mapping of manpage names to URLs
/// * `auto_link_options` - Whether to convert {option} roles to links
/// * `valid_options` - Optional set of valid option names for validation
///
/// # Returns
///
/// The processed markdown with role markup converted to HTML
#[cfg(any(feature = "nixpkgs", feature = "ndg-flavored"))]
#[must_use]
#[allow(
  clippy::implicit_hasher,
  reason = "Standard HashMap/HashSet sufficient for this use case"
)]
pub fn process_role_markup(
  content: &str,
  manpage_urls: Option<&std::collections::HashMap<String, String>>,
  auto_link_options: bool,
  valid_options: Option<&std::collections::HashSet<String>>,
) -> String {
  let mut result = String::new();
  let mut chars = content.chars().peekable();
  let mut tracker = crate::utils::codeblock::InlineTracker::new();

  while let Some(ch) = chars.next() {
    // Handle backticks (code fences and inline code)
    if ch == '`' {
      let (new_tracker, tick_count) = tracker.process_backticks(&mut chars);
      tracker = new_tracker;

      // Add all the backticks
      result.push_str(&"`".repeat(tick_count));
      continue;
    }

    // Handle tilde code fences (~~~)
    if ch == '~' && chars.peek() == Some(&'~') {
      let (new_tracker, tilde_count) = tracker.process_tildes(&mut chars);
      tracker = new_tracker;

      result.push_str(&"~".repeat(tilde_count));
      continue;
    }

    // Handle newlines
    if ch == '\n' {
      tracker = tracker.process_newline();
      result.push(ch);
      continue;
    }

    // Process role markup only if we're not in any kind of code
    if ch == '{' && !tracker.in_any_code() {
      // Collect remaining characters to test parsing
      let remaining: Vec<char> = chars.clone().collect();
      let remaining_str: String = remaining.iter().collect();
      let mut temp_chars = remaining_str.chars().peekable();

      if let Some(role_markup) = parse_role_markup(
        &mut temp_chars,
        manpage_urls,
        auto_link_options,
        valid_options,
      ) {
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
  auto_link_options: bool,
  valid_options: Option<&std::collections::HashSet<String>>,
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
      return Some(format_role_markup(
        &role_name,
        &content,
        manpage_urls,
        auto_link_options,
        valid_options,
      ));
    }
    content.push(ch);
  }

  // No closing backtick found
  None
}

/// Format the role markup as HTML based on the role type and content.
#[must_use]
#[allow(
  clippy::option_if_let_else,
  reason = "Nested options clearer with if-let"
)]
#[allow(
  clippy::implicit_hasher,
  reason = "Standard HashMap/HashSet sufficient for this use case"
)]
pub fn format_role_markup(
  role_type: &str,
  content: &str,
  manpage_urls: Option<&std::collections::HashMap<String, String>>,
  auto_link_options: bool,
  valid_options: Option<&std::collections::HashSet<String>>,
) -> String {
  let escaped_content = encode_text(content);
  match role_type {
    "manpage" => {
      if let Some(urls) = manpage_urls {
        if let Some(url) = urls.get(content) {
          format!(
            "<a href=\"{url}\" \
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
      if cfg!(feature = "ndg-flavored") && auto_link_options {
        // Check if validation is enabled and option is valid
        let should_link =
          valid_options.is_none_or(|opts| opts.contains(content)); // If no validation set, link all options

        if should_link {
          let option_id = format!("option-{}", content.replace('.', "-"));
          format!(
            "<a class=\"option-reference\" \
             href=\"options.html#{option_id}\"><code \
             class=\"nixos-option\">{escaped_content}</code></a>"
          )
        } else {
          format!("<code class=\"nixos-option\">{escaped_content}</code>")
        }
      } else {
        format!("<code class=\"nixos-option\">{escaped_content}</code>")
      }
    },
    "var" => format!("<code class=\"nix-var\">{escaped_content}</code>"),
    _ => format!("<span class=\"{role_type}-markup\">{escaped_content}</span>"),
  }
}

/// Process MyST-style autolinks in markdown content.
///
/// Converts MyST-like autolinks supported by Nixpkgs-flavored commonmark:
/// - `[](#anchor)` -> `[](#anchor) -> {{ANCHOR}}` (placeholder for comrak)
/// - `[](https://url)` -> `<https://url>` (converted to standard autolink)
///
/// # Arguments
///
/// * `content` - The markdown content to process
///
/// # Returns
///
/// The processed markdown with `MyST` autolinks converted as a [`String`]
#[must_use]
pub fn process_myst_autolinks(content: &str) -> String {
  let mut result = String::with_capacity(content.len());
  let mut fence_tracker = crate::utils::codeblock::FenceTracker::new();

  for line in content.lines() {
    // Update fence tracking state
    fence_tracker = fence_tracker.process_line(line);

    // Only process MyST autolinks if we're not in a code block
    if fence_tracker.in_code_block() {
      result.push_str(line);
    } else {
      result.push_str(&process_line_myst_autolinks(line));
    }
    result.push('\n');
  }

  result
}

/// Process `MyST` autolinks in a single line.
fn process_line_myst_autolinks(line: &str) -> String {
  let mut result = String::with_capacity(line.len());
  let mut chars = line.chars().peekable();

  while let Some(ch) = chars.next() {
    if ch == '[' && chars.peek() == Some(&']') {
      chars.next(); // consume ']'

      // Check if this is []{#...} syntax (inline anchor, not autolink)
      // Nice pit, would be a shame if someone was to... fall into it.
      if chars.peek() == Some(&'{') {
        // This is inline anchor syntax, not autolink, keep as-is
        result.push_str("[]");
        continue;
      }

      if chars.peek() == Some(&'(') {
        chars.next(); // consume '('

        // Collect URL until ')'
        let mut url = String::new();
        let mut found_closing = false;
        while let Some(&next_ch) = chars.peek() {
          if next_ch == ')' {
            chars.next(); // consume ')'
            found_closing = true;
            break;
          }
          url.push(next_ch);
          chars.next();
        }

        if found_closing && !url.is_empty() {
          // Check if it's an anchor link (starts with #) or a URL
          if url.starts_with('#') {
            // Add placeholder text for comrak to parse it as a link
            let _ = write!(result, "[{{{{ANCHOR}}}}]({url})");
          } else if url.starts_with("http://") || url.starts_with("https://") {
            // Convert URL autolinks to standard <url> format
            let _ = write!(result, "<{url}>");
          } else {
            // Keep other patterns as-is
            let _ = write!(result, "[]({url})");
          }
        } else {
          // Malformed, put back what we consumed
          result.push_str("](");
          result.push_str(&url);
        }
      } else {
        // Not a link, put back consumed character
        result.push(']');
      }
    } else {
      result.push(ch);
    }
  }

  result
}

/// Process inline anchors in markdown content.
///
/// This function processes inline anchor syntax like `[]{#my-anchor}` while
/// being code-block aware to avoid processing inside code fences.
///
/// # Arguments
///
/// * `content` - The markdown content to process
///
/// # Returns
///
/// The processed markdown with inline anchors converted to HTML spans
///
/// # Panics
///
/// Panics if a code fence marker line is empty (which should not occur in valid
/// markdown).
#[cfg(feature = "nixpkgs")]
#[must_use]
pub fn process_inline_anchors(content: &str) -> String {
  let mut result = String::with_capacity(content.len() + 100);
  let mut fence_tracker = crate::utils::codeblock::FenceTracker::new();

  for line in content.lines() {
    let trimmed = line.trim_start();

    // Update fence tracking state
    fence_tracker = fence_tracker.process_line(line);

    // Only process inline anchors if we're not in a code block
    if fence_tracker.in_code_block() {
      // In code block, keep line as-is
      result.push_str(line);
    } else {
      // Check for list items with anchors:
      // "- []{#id} content" or "1. []{#id} content"
      if let Some(anchor_start) = find_list_item_anchor(trimmed)
        && let Some(processed_line) =
          process_list_item_anchor(line, anchor_start)
      {
        result.push_str(&processed_line);
        result.push('\n');
        continue;
      }

      // Process regular inline anchors in the line
      result.push_str(&process_line_anchors(line));
    }
    result.push('\n');
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
                let _ = write!(
                  result,
                  "<span id=\"{id}\" class=\"nixos-anchor\"></span>"
                );
              } else {
                // Invalid ID, put back original text
                let _ = write!(result, "[]{{{{#{id}}}}}");
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
              let _ = write!(result, "[]{{{{#{id}");
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
///
/// # Panics
///
/// Panics if a code fence marker line is empty (which should not occur in valid
/// markdown).
#[cfg(feature = "nixpkgs")]
#[must_use]
pub fn process_block_elements(content: &str) -> String {
  let mut result = Vec::new();
  let mut lines = content.lines().peekable();
  let mut fence_tracker = crate::utils::codeblock::FenceTracker::new();

  while let Some(line) = lines.next() {
    // Update fence tracking state
    fence_tracker = fence_tracker.process_line(line);

    // Only process block elements if we're not in a code block
    if !fence_tracker.in_code_block() {
      // Check for GitHub-style callouts: > [!TYPE]
      if let Some((callout_type, initial_content)) = parse_github_callout(line)
      {
        let content =
          collect_github_callout_content(&mut lines, &initial_content);
        let admonition = render_admonition(&callout_type, None, &content);
        result.push(admonition);
        continue;
      }

      // Check for fenced admonitions: ::: {.type}
      if let Some((adm_type, id)) = parse_fenced_admonition_start(line) {
        let (content, trailing) = collect_fenced_content(&mut lines);
        let admonition = render_admonition(&adm_type, id.as_deref(), &content);
        result.push(admonition);
        // If there's trailing content after the closing :::, add it as a new
        // line
        if let Some(trailing_content) = trailing {
          result.push(trailing_content);
        }
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
  if let Some(close_bracket) = trimmed.find(']')
    && close_bracket > 4
  {
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

  None
}

/// Check if a line starts with a valid ATX header (1-6 '#' followed by
/// whitespace or EOL).
///
/// Per `CommonMark` spec, an ATX header requires 1-6 '#' characters followed by
/// either:
/// - A whitespace character (space, tab, etc.)
/// - End of line (the string ends)
///
/// # Arguments
/// * `line` - The line to check
///
/// # Returns
/// `true` if the line starts with a valid ATX header marker
fn is_atx_header(line: &str) -> bool {
  let mut chars = line.chars();
  let mut hash_count = 0;

  // count leading '#' characters (max 6)
  while let Some(c) = chars.next() {
    if c == '#' {
      hash_count += 1;
      if hash_count > 6 {
        return false;
      }
    } else {
      // found a non-'#' character, check if it's whitespace or we're at EOL
      return (1..=6).contains(&hash_count)
        && (c.is_whitespace() || chars.as_str().is_empty());
    }
  }

  // reached end of string, check if we have 1-6 hashes
  (1..=6).contains(&hash_count)
}

/// Collect content for GitHub-style callouts
fn collect_github_callout_content(
  lines: &mut std::iter::Peekable<std::str::Lines>,
  initial_content: &str,
) -> String {
  let mut content = String::new();

  if !initial_content.is_empty() {
    content.push_str(initial_content);
    content.push('\n');
  }

  while let Some(line) = lines.peek() {
    let trimmed = line.trim_start();

    // Empty line ends the blockquote
    if trimmed.is_empty() {
      break;
    }

    // Check if this is a continuation line with `>`
    let content_part = if trimmed.starts_with('>') {
      trimmed.strip_prefix('>').unwrap_or("").trim_start()
    } else {
      // Check if this line starts a new block element that cannot be
      // lazy-continued ATX headers, setext header underlines, code
      // fences, and thematic breaks
      let starts_new_block = is_atx_header(trimmed)
        || trimmed.starts_with("```")
        || trimmed.starts_with("~~~")
        || (trimmed.starts_with("---")
          && trimmed.chars().all(|c| c == '-' || c.is_whitespace()))
        || (trimmed.starts_with("===")
          && trimmed.chars().all(|c| c == '=' || c.is_whitespace()))
        || (trimmed.starts_with("***")
          && trimmed.chars().all(|c| c == '*' || c.is_whitespace()));

      if starts_new_block {
        break;
      }

      // Lazy continuation
      // Mind yu, "lazy" doesn't refer to me being lazy but the GFM feature for
      // a line without `>` that continues the blockquote
      // paragraph
      trimmed
    };

    content.push_str(content_part);
    content.push('\n');
    lines.next(); // consume the line
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
///
/// Returns a tuple of (admonition_content, trailing_content).
/// If there's content after the closing `:::` on the same line,
/// it's returned as trailing_content.
fn collect_fenced_content(
  lines: &mut std::iter::Peekable<std::str::Lines>,
) -> (String, Option<String>) {
  let mut content = String::new();

  for line in lines.by_ref() {
    let trimmed = line.trim();
    if trimmed.starts_with(":::") {
      // check if there's content after the closing :::
      let after_colons = trimmed.strip_prefix(":::").unwrap_or("");
      if !after_colons.is_empty() {
        // there's trailing content on the same line as the closing delimiter
        return (content.trim().to_string(), Some(after_colons.to_string()));
      }
      break;
    }
    content.push_str(line);
    content.push('\n');
  }

  (content.trim().to_string(), None)
}

/// Parse figure block: ::: {.figure #id}
#[allow(
  clippy::option_if_let_else,
  reason = "Nested options clearer with if-let"
)]
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

  let opening = format!(
    "<div class=\"admonition {adm_type}\"{id_attr}>\n<p \
     class=\"admonition-title\">{capitalized_type}</p>"
  );
  format!("{opening}\n\n{content}\n\n</div>\n")
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
#[allow(
  clippy::implicit_hasher,
  reason = "Standard HashMap sufficient for this use case"
)]
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
      for span_node in safe_select(&document, "span.manpage-reference") {
        let span_el = span_node;
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
      let _ = document.serialize(&mut out);
      String::from_utf8(out).unwrap_or_default()
    },
    // Return original HTML on error
    "",
  )
}

/// Process option references
/// Converts {option} role markup into links to the options page.
///
/// This processes `<code>` elements that have the `nixos-option` class, i.e.,
/// {option} role markup and convert them into links to the options page.
///
/// # Arguments
///
/// * `html` - The HTML string to process.
/// * `valid_options` - Optional set of valid option names for validation.
///
/// # Returns
///
/// The HTML string with option references rewritten as links.
#[cfg(feature = "ndg-flavored")]
#[must_use]
#[allow(
  clippy::implicit_hasher,
  reason = "Standard HashSet sufficient for this use case"
)]
pub fn process_option_references(
  html: &str,
  valid_options: Option<&std::collections::HashSet<String>>,
) -> String {
  use kuchikikiki::{Attribute, ExpandedName, NodeRef};
  use markup5ever::{QualName, local_name, ns};
  use tendril::TendrilSink;

  process_safe(
    html,
    |html| {
      let document = kuchikikiki::parse_html().one(html);

      let mut to_replace = vec![];

      // Only process code elements that already have the nixos-option class
      // from {option} role syntax
      for code_node in safe_select(&document, "code.nixos-option") {
        let code_el = code_node;
        let code_text = code_el.text_contents();

        // Skip if already wrapped in an option-reference link
        let mut is_already_option_ref = false;
        let mut current = code_el.parent();
        while let Some(parent) = current {
          if let Some(element) = parent.as_element()
            && element.name.local == local_name!("a")
            && let Some(class_attr) =
              element.attributes.borrow().get(local_name!("class"))
            && class_attr.contains("option-reference")
          {
            is_already_option_ref = true;
            break;
          }
          current = parent.parent();
        }

        if !is_already_option_ref {
          // Check if validation is enabled and option is valid
          let should_link =
            valid_options.is_none_or(|opts| opts.contains(code_text.as_str())); // If no validation set, link all options

          if should_link {
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
          // If should_link is false, leave the code element as-is (no wrapping)
        }
      }

      for (old, new) in to_replace {
        old.insert_before(new);
        old.detach();
      }

      let mut out = Vec::new();
      let _ = document.serialize(&mut out);
      String::from_utf8(out).unwrap_or_default()
    },
    // Return original HTML on error
    "",
  )
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

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_is_atx_header_valid_headers() {
    // valid ATX headers with 1-6 hashes followed by space
    assert!(is_atx_header("# Header"));
    assert!(is_atx_header("## Header"));
    assert!(is_atx_header("### Header"));
    assert!(is_atx_header("#### Header"));
    assert!(is_atx_header("##### Header"));
    assert!(is_atx_header("###### Header"));

    // valid ATX headers with tab after hashes
    assert!(is_atx_header("#\tHeader"));
    assert!(is_atx_header("##\tHeader"));

    // valid ATX headers with just hashes (no content after)
    assert!(is_atx_header("#"));
    assert!(is_atx_header("##"));
    assert!(is_atx_header("###"));
    assert!(is_atx_header("####"));
    assert!(is_atx_header("#####"));
    assert!(is_atx_header("######"));

    // valid ATX headers with multiple spaces
    assert!(is_atx_header("#  Header with multiple spaces"));
    assert!(is_atx_header("##   Header"));
  }

  #[test]
  fn test_is_atx_header_invalid_headers() {
    // more than 6 hashes
    assert!(!is_atx_header("####### Too many hashes"));
    assert!(!is_atx_header("######## Even more"));

    // no space after hash
    assert!(!is_atx_header("#NoSpace"));
    assert!(!is_atx_header("##NoSpace"));

    // hash in the middle
    assert!(!is_atx_header("Not # a header"));

    // empty string
    assert!(!is_atx_header(""));

    // no hash at all
    assert!(!is_atx_header("Regular text"));

    // hash with non-whitespace immediately after
    assert!(!is_atx_header("#hashtag"));
    assert!(!is_atx_header("##hashtag"));
    assert!(!is_atx_header("#123"));
    assert!(!is_atx_header("##abc"));

    // special characters immediately after hash
    assert!(!is_atx_header("#!important"));
    assert!(!is_atx_header("#@mention"));
    assert!(!is_atx_header("#$variable"));
  }

  #[test]
  fn test_is_atx_header_edge_cases() {
    // whitespace before hash is handled by caller (trimmed)
    // but testing it here to ensure robustness
    assert!(!is_atx_header(" # Header"));
    assert!(!is_atx_header("  ## Header"));

    // only spaces after hash (should be valid)
    assert!(is_atx_header("#     "));
    assert!(is_atx_header("##    "));

    // newline handling (string ends after valid header marker)
    assert!(is_atx_header("# Header\n"));
    assert!(is_atx_header("## Header\n"));

    // mixed whitespace after hash
    assert!(is_atx_header("# \t  Header"));
    assert!(is_atx_header("##  \tHeader"));
  }

  #[test]
  fn test_is_atx_header_blockquote_context() {
    // these are the types of strings that would be passed from
    // collect_github_callout_content after trim_start()
    assert!(is_atx_header("# New Section"));
    assert!(is_atx_header("## Subsection"));

    // non-headers that should not break blockquote
    assert!(!is_atx_header("#tag"));
    assert!(!is_atx_header("##issue-123"));
    assert!(!is_atx_header("###no-space"));

    // edge case: exactly 6 hashes (valid)
    assert!(is_atx_header("###### Level 6"));

    // edge case: 7 hashes (invalid)
    assert!(!is_atx_header("####### Not valid"));
  }
}
