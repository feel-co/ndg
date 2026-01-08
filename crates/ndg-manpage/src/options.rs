use std::{fs, io::Write, path::Path, sync::LazyLock};

use color_eyre::eyre::{Context, Result};
use log::{error, info};
use ndg_commonmark::{
  MarkdownOptions,
  MarkdownProcessor,
  process_role_markup,
  utils::never_matching_regex,
};
use ndg_utils::json::extract_value;
use rayon::prelude::*;
use regex::Regex;
use serde_json::{self, Value};

use crate::{
  escape::{
    ROFF_ESCAPES,
    TROFF_ESCAPE,
    TROFF_FORMATTING,
    escape_leading_dot,
    escape_non_macro_lines,
    man_escape,
  },
  types::NixOption,
};

// Shared processor instance for manpage generation
thread_local! {
  static MANPAGE_PROCESSOR: MarkdownProcessor = {
    let options = MarkdownOptions::default();
    MarkdownProcessor::new(options)
  };
}

// HTML tags
static HTML_TAGS: LazyLock<Regex> = LazyLock::new(|| {
  Regex::new(r"</?[a-zA-Z][^>]*>").unwrap_or_else(|e| {
    error!("Failed to compile HTML_TAGS regex: {e}");
    never_matching_regex().unwrap_or_else(|_| {
      #[allow(
        clippy::expect_used,
        reason = "This pattern is guaranteed to be valid"
      )]
      Regex::new(r"[^\s\S]")
        .expect("regex pattern [^\\s\\S] should always compile")
    })
  })
});

// Admonition patterns for pre-processed content
static ADMONITION_START: LazyLock<Regex> = LazyLock::new(|| {
  Regex::new(r"\.ADMONITION_START\s+(\w+)(.*)").unwrap_or_else(|e| {
    error!("Failed to compile ADMONITION_START regex: {e}");
    never_matching_regex().unwrap_or_else(|_| {
      #[allow(
        clippy::expect_used,
        reason = "This pattern is guaranteed to be valid"
      )]
      Regex::new(r"[^\s\S]")
        .expect("regex pattern [^\\s\\S] should always compile")
    })
  })
});
// Don't use regex for simple string matching with ADMONITION_END

// Markdown list items
static LIST_ITEM: LazyLock<Regex> = LazyLock::new(|| {
  Regex::new(r"^\s*[-*+]\s+(.+)$").unwrap_or_else(|e| {
    error!("Failed to compile LIST_ITEM regex: {e}");
    never_matching_regex().unwrap_or_else(|_| {
      #[allow(
        clippy::expect_used,
        reason = "This pattern is guaranteed to be valid"
      )]
      Regex::new(r"[^\s\S]")
        .expect("regex pattern [^\\s\\S] should always compile")
    })
  })
});

static NUMBERED_LIST_ITEM: LazyLock<Regex> = LazyLock::new(|| {
  Regex::new(r"^\s*(\d+)\.\s+(.+)$").unwrap_or_else(|e| {
    error!("Failed to compile NUMBERED_LIST_ITEM regex: {e}");
    never_matching_regex().unwrap_or_else(|_| {
      #[allow(
        clippy::expect_used,
        reason = "This pattern is guaranteed to be valid"
      )]
      Regex::new(r"[^\s\S]")
        .expect("regex pattern [^\\s\\S] should always compile")
    })
  })
});

// Markdown links
static MARKDOWN_LINK: LazyLock<Regex> = LazyLock::new(|| {
  Regex::new(r"\[([^\]]+)\]\(([^)]+)\)").unwrap_or_else(|e| {
    error!("Failed to compile MARKDOWN_LINK regex: {e}");
    never_matching_regex().unwrap_or_else(|_| {
      #[allow(
        clippy::expect_used,
        reason = "This pattern is guaranteed to be valid"
      )]
      Regex::new(r"[^\s\S]")
        .expect("regex pattern [^\\s\\S] should always compile")
    })
  })
});

/// Generate a manpage from options JSON
/// Generate a manpage from options JSON
///
/// # Errors
///
/// Returns an error if the options file cannot be read, parsed, or written.
pub fn generate_manpage(
  options_path: &Path,
  output_path: Option<&Path>,
  title: Option<&str>,
  header: Option<&str>,
  footer: Option<&str>,
  section: u8,
) -> Result<()> {
  // Read options JSON
  let json_content = fs::read_to_string(options_path).wrap_err_with(|| {
    format!("Failed to read options file: {}", options_path.display())
  })?;

  let options_data: Value = serde_json::from_str(&json_content)
    .wrap_err("Failed to parse options JSON")?;

  // Extract options
  let mut options = Vec::new();

  if let Value::Object(map) = options_data {
    // Process options in parallel for large option sets
    let options_vec: Vec<_> = map.into_iter().collect();

    let parsed_options: Vec<_> = options_vec
      .par_iter()
      .filter_map(|(key, value)| {
        if let Value::Object(option_data) = value {
          let option = parse_option(key, option_data);
          Some(option)
        } else {
          None
        }
      })
      .collect();

    options.extend(parsed_options);
  }

  // Sort options by name
  options.sort_by(|a, b| a.name.cmp(&b.name));

  // Generate the manpage
  let manpage_title = title.unwrap_or("Module Options");

  // Determine output file path
  let output_file = output_path.map_or_else(
    || {
      let safe_title = manpage_title
        .to_lowercase()
        .replace(|c: char| !c.is_alphanumeric(), "-");
      Path::new(&format!("{safe_title}.{section}")).to_path_buf()
    },
    std::path::Path::to_path_buf,
  );

  // Create output file
  let mut file = fs::File::create(&output_file).wrap_err_with(|| {
    format!("Failed to create output file: {}", output_file.display())
  })?;

  // Write manpage header
  let today = jiff::Zoned::now().strftime("%Y-%m-%d").to_string();
  writeln!(file, ".\\\" Generated by ndg")?;
  writeln!(
    file,
    ".TH \"{}\" \"{}\" \"{}\" \"\" \"{}\"",
    man_escape(manpage_title),
    section,
    today,
    man_escape(manpage_title)
  )?;

  // Write header information
  writeln!(file, ".SH NAME")?;
  writeln!(file, "{}", man_escape(manpage_title))?;
  writeln!(file, ".SH DESCRIPTION")?;

  if let Some(header_text) = header {
    writeln!(file, "{}", process_description(header_text))?;
  } else {
    writeln!(file, "Available configuration options")?;
  }

  // Write options section
  writeln!(file, ".SH OPTIONS")?;

  for option in options {
    // Skip internal options
    if option.internal {
      continue;
    }

    // Option name with bold formatting
    writeln!(file, ".PP")?;
    writeln!(file, "\\fB{}\\fR", man_escape(&option.name))?;
    writeln!(file, ".RS 4")?;

    // Description
    writeln!(file, "{}", process_description(&option.description))?;

    // Type
    writeln!(file, ".sp")?;
    writeln!(
      file,
      "\\fIType:\\fR {}",
      process_raw_type(&option.type_name)
    )?;

    // Default value if present
    if let Some(default_text) = &option.default_text {
      writeln!(file, ".sp")?;
      writeln!(file, "\\fIDefault:\\fR {}", process_value(default_text))?;
    } else if let Some(default_val) = &option.default {
      writeln!(file, ".sp")?;
      writeln!(
        file,
        "\\fIDefault:\\fR {}",
        process_value(&default_val.to_string())
      )?;
    }

    // Example if present
    if let Some(example_text) = &option.example_text {
      writeln!(file, ".sp")?;
      writeln!(file, "\\fIExample:\\fR")?;
      writeln!(file, ".sp")?;
      writeln!(file, ".RS 4")?;
      writeln!(file, ".nf")?;
      writeln!(file, "{}", process_example(example_text))?;
      writeln!(file, ".fi")?;
      writeln!(file, ".RE")?;
    } else if let Some(example_val) = &option.example {
      writeln!(file, ".sp")?;
      writeln!(
        file,
        "\\fIExample:\\fR {}",
        process_value(&example_val.to_string())
      )?;
    }

    // Declaration source if available
    if let Some(declared_in) = &option.declared_in {
      writeln!(file, ".sp")?;
      writeln!(file, "\\fIDeclared by:\\fP")?;
      writeln!(file, ".RS 4")?;
      if let Some(url) = &option.declared_in_url {
        writeln!(
          file,
          "\\fB<{}>\\fP (\\fI{}\\fP)",
          man_escape(declared_in),
          man_escape(url)
        )?;
      } else {
        writeln!(file, "\\fB<{}>\\fP", man_escape(declared_in))?;
      }
      writeln!(file, ".RE")?;
    }

    // Read-only status
    if option.read_only {
      writeln!(file, ".sp")?;
      writeln!(file, "\\fINote: This option is read-only.\\fP")?;
    }

    // Close option section
    writeln!(file, ".RE")?;
  }

  // Write footer if provided
  if let Some(footer_text) = footer {
    writeln!(file, ".SH NOTES")?;
    writeln!(file, "{}", process_description(footer_text))?;
  }

  // Add SEE ALSO section
  // Extract base name without extension to use in see also
  let _file_base_name = options_path
    .file_stem()
    .and_then(|s| s.to_str())
    .unwrap_or("configuration");

  writeln!(file, ".SH SEE ALSO")?;

  info!("Generated manpage: {}", output_file.display());

  Ok(())
}

/// Process raw type string preserving escape sequences but formatting quotes
/// and newlines properly
fn process_raw_type(s: &str) -> String {
  // Replace common escape sequences with their troff equivalents
  let s = s
        .replace('"', "\\[u201C]") // opening double quote
        .replace("\\n", "\\en") // newline
        .replace('\'', "\\[u2019]") // single quote
        .replace('-', "\\-") // hyphen
        .replace('.', "\\&."); // period

  // For closing quote after \n

  s.replace("\\en\"", "\\en\\[u201D]")
}

/// Parse a single option from JSON data
fn parse_option(
  key: &str,
  option_data: &serde_json::Map<String, Value>,
) -> NixOption {
  let mut option = NixOption {
    name:            key.to_string(),
    type_name:       String::new(),
    description:     String::new(),
    default:         None,
    default_text:    None,
    example:         None,
    example_text:    None,
    declared_in:     None,
    declared_in_url: None,
    internal:        false,
    read_only:       false,
  };

  // Process fields from the option data
  if let Some(Value::String(type_name)) = option_data.get("type") {
    option.type_name.clone_from(type_name);
  }

  if let Some(Value::String(desc)) = option_data.get("description") {
    option.description.clone_from(desc);
  }

  // Handle default values
  if let Some(default_val) = option_data.get("default") {
    if let Some(extracted_value) = extract_value(default_val, false) {
      option.default_text = Some(extracted_value);
    } else {
      option.default = Some(default_val.clone());
    }
  }

  if let Some(Value::String(text)) = option_data.get("defaultText") {
    option.default_text = Some(text.clone());
  }

  // Handle example values
  if let Some(example_val) = option_data.get("example") {
    if let Some(extracted_value) = extract_value(example_val, false) {
      option.example_text = Some(extracted_value);
    } else {
      option.example = Some(example_val.clone());
    }
  }

  if let Some(Value::String(text)) = option_data.get("exampleText") {
    option.example_text = Some(text.clone());
  }

  // Read-only status
  if let Some(Value::Bool(read_only)) = option_data.get("readOnly") {
    option.read_only = *read_only;
  }

  // Internal status
  if let Some(Value::Bool(internal)) = option_data.get("internal") {
    option.internal = *internal;
  }

  if let Some(Value::Bool(visible)) = option_data.get("visible")
    && !visible
  {
    option.internal = true;
  }

  // File where option is declared
  if let Some(Value::String(file)) = option_data.get("declarations") {
    option.declared_in = Some(file.clone());
  }

  // URL for the declaration
  if let Some(Value::String(url)) = option_data.get("declarationURL") {
    option.declared_in_url = Some(url.clone());
  }

  option
}

/// Process description text for troff format
fn process_description(text: &str) -> String {
  // Pre-check for troff formatting codes to preserve them
  let text = preserve_existing_formatting(text);

  // Convert any HTML first
  let without_html = HTML_TAGS.replace_all(&text, "");

  // Process individual lines to handle lists and other block-level elements
  let processed_lines = without_html
    .lines()
    .map(|line| {
      // Process list items
      LIST_ITEM.captures(line).map_or_else(
        || {
          NUMBERED_LIST_ITEM.captures(line).map_or_else(
            || {
              // Process inline markdown for regular lines
              process_inline_markdown(line)
            },
            |captures| {
              let number = &captures[1];
              let content = &captures[2];
              format!(
                ".IP \"{number}.\" 4\n{}",
                process_inline_markdown(content)
              )
            },
          )
        },
        |captures| {
          let content = &captures[1];
          format!(".IP \"\\[u2022]\" 4\n{}", process_inline_markdown(content))
        },
      )
    })
    .collect::<Vec<_>>()
    .join("\n");

  // Process special admonitions and other block elements
  let with_admonitions = process_admonitions(&processed_lines);

  // Escape only non-macro-leading lines so list markers stay active
  let escaped = escape_non_macro_lines(&with_admonitions);

  // Preserve explicit paragraph breaks so list items do not merge
  escaped.replace("\n\n", "\n.br\n")
}

/// Preserve existing troff formatting codes so they don't get double-escaped
fn preserve_existing_formatting(text: &str) -> String {
  // Replace troff formatting temporarily
  let with_placeholders =
    TROFF_FORMATTING.replace_all(text, |caps: &regex::Captures| {
      format!("__TROFF_FORMAT_{}__", caps[0].replace('\\', ""))
    });

  // Replace troff escapes temporarily
  let with_all_placeholders =
    TROFF_ESCAPE.replace_all(&with_placeholders, |caps: &regex::Captures| {
      format!("__TROFF_ESCAPE_{}__", caps[0].replace('\\', ""))
    });

  with_all_placeholders.to_string()
}

/// Restore troff formatting codes after processing
fn restore_formatting(text: &str) -> String {
  let with_formats = text
    .replace("__TROFF_FORMAT_fB__", "\\fB")
    .replace("__TROFF_FORMAT_fI__", "\\fI")
    .replace("__TROFF_FORMAT_fP__", "\\fP")
    .replace("__TROFF_FORMAT_fR__", "\\fR");

  with_formats
    .replace("__TROFF_ESCAPE_(__", "\\(")
    .replace("__TROFF_ESCAPE_\\__", "\\\\")
}

/// Process inline markdown elements like roles, code, etc.
fn process_inline_markdown(text: &str) -> String {
  // Process markdown links first
  let with_links = process_markdown_links(text);

  // Process MyST-like roles
  let with_roles = process_roles(&with_links);

  // Process command prompts
  let with_prompts = process_command_prompts(&with_roles);

  // Process repl prompts
  let with_repl = process_repl_prompts(&with_prompts);

  // Handle inline code
  let with_code = process_inline_code(&with_repl);

  // Strip outer HTML wrappers produced by renderer for plain inline content
  let without_wrappers = with_code
    .replace("<html><head></head><body>", "")
    .replace("</body></html>", "")
    .replace("<p>", "")
    .replace("</p>", "");

  // Also strip wrappers in defaults/examples
  let without_block_wrappers = without_wrappers
    .replace("<html>", "")
    .replace("</html>", "")
    .replace("<body>", "")
    .replace("</body>", "");

  // Restore any preserved troff formatting
  let with_formatting_restored = restore_formatting(&without_block_wrappers);

  // Escape any troff special characters and commands, but not existing troff
  // formatting
  let escaped = selective_man_escape(&with_formatting_restored);

  // Ensure no leading dots that would be interpreted as commands
  escape_leading_dot(&escaped)
}

/// Process markdown links to proper format for manpages
fn process_markdown_links(text: &str) -> String {
  MARKDOWN_LINK
    .replace_all(text, |caps: &regex::Captures| {
      let link_text = &caps[1];
      let url = &caps[2];

      // For manpages, we can't have clickable links, so we format as text + URL
      format!("\\fB[{link_text}]\\fP ({url})")
    })
    .to_string()
}

/// Selectively escape text for troff
fn selective_man_escape(text: &str) -> String {
  let mut result = String::with_capacity(text.len() * 2);

  let mut i = 0;
  let chars: Vec<char> = text.chars().collect();

  while i < chars.len() {
    // Check if we're at a troff formatting code
    if i + 2 < chars.len() && chars[i] == '\\' && chars[i + 1] == 'f' {
      // Don't escape troff formatting codes
      result.push(chars[i]);
      result.push(chars[i + 1]);
      result.push(chars[i + 2]);
      i += 3;
      continue;
    }

    // Check if we're at a troff escape sequence
    if i + 1 < chars.len()
      && chars[i] == '\\'
      && (chars[i + 1] == '(' || chars[i + 1] == '\\' || chars[i + 1] == '[')
    {
      // Don't escape troff escape sequences
      result.push(chars[i]);
      result.push(chars[i + 1]);
      i += 2;

      // If it's a special escape, process accordingly
      if chars[i - 1] == '(' && i + 2 <= chars.len() {
        result.push(chars[i]);
        result.push(chars[i + 1]);
        i += 2;
      } else if chars[i - 1] == '[' {
        // Handle \[uXXXX] type escapes
        while i < chars.len() && chars[i] != ']' {
          result.push(chars[i]);
          i += 1;
        }
        if i < chars.len() {
          result.push(chars[i]); // closing ]
          i += 1;
        }
      }
      continue;
    }

    // Otherwise escape normally
    if let Some(escape) = ROFF_ESCAPES.get(&chars[i]) {
      result.push_str(escape);
    } else {
      result.push(chars[i]);
    }

    i += 1;
  }

  result
}

/// Process all role-based formatting in text
fn process_roles(text: &str) -> String {
  MANPAGE_PROCESSOR.with(|processor| {
    process_role_markup(text, processor.manpage_urls(), true, None)
  })
}

/// Process command prompts ($ command)
fn process_command_prompts(text: &str) -> String {
  MANPAGE_PROCESSOR.with(|processor| {
    let result = processor.render(text);
    result
      .html
      .replace(
        "<code class=\"terminal\"><span class=\"prompt\">$</span> ",
        "\\fR\\fB$\\fP ",
      )
      .replace("</code>", "")
  })
}

/// Process REPL prompts (nix-repl> command)
fn process_repl_prompts(text: &str) -> String {
  MANPAGE_PROCESSOR.with(|processor| {
    let result = processor.render(text);
    result
      .html
      .replace(
        "<code class=\"nix-repl\"><span class=\"prompt\">nix-repl&gt;</span> ",
        "\\fR\\fBnix-repl>\\fP ",
      )
      .replace("</code>", "")
  })
}

/// Process admonition blocks (:::)
fn process_admonitions(text: &str) -> String {
  let mut result = String::new();
  let mut _in_admonition = false;

  for line in text.lines() {
    if let Some(caps) = ADMONITION_START.captures(line) {
      _in_admonition = true;
      let adm_type = &caps[1];
      let title = match adm_type.to_lowercase().as_str() {
        "note" => "Note",
        "warning" => "Warning",
        "tip" => "Tip",
        "info" => "Info",
        "important" => "Important",
        "caution" => "Caution",
        "danger" => "Danger",
        _ => adm_type,
      };

      result.push_str(".sp\n.RS 4\n\\fB");
      result.push_str(title);
      result.push_str("\\fP\n.br");

      let content = caps.get(2).map_or("", |m| m.as_str()).trim();
      if !content.is_empty() {
        result.push('\n');
        result.push_str(content);
      }
      result.push('\n');
    } else if line.contains(".ADMONITION_END") {
      _in_admonition = false;
      result.push_str(".RE\n");
    } else {
      // Handle both in_admonition and normal lines the same way
      result.push_str(line);
      result.push('\n');
    }
  }

  // Basic replacement for any pre-processed admonitions
  result
    .replace(".ADMONITION_START note", ".sp\n.RS 4\n\\fBNote\\fP\n.br")
    .replace(
      ".ADMONITION_START warning",
      ".sp\n.RS 4\n\\fBWarning\\fP\n.br",
    )
    .replace(".ADMONITION_START tip", ".sp\n.RS 4\n\\fBTip\\fP\n.br")
    .replace(".ADMONITION_START info", ".sp\n.RS 4\n\\fBInfo\\fP\n.br")
    .replace(
      ".ADMONITION_START important",
      ".sp\n.RS 4\n\\fBImportant\\fP\n.br",
    )
    .replace(
      ".ADMONITION_START caution",
      ".sp\n.RS 4\n\\fBCaution\\fP\n.br",
    )
    .replace(
      ".ADMONITION_START danger",
      ".sp\n.RS 4\n\\fBDanger\\fP\n.br",
    )
    .replace(".ADMONITION_END", ".RE")
}

/// Process inline code blocks
fn process_inline_code(text: &str) -> String {
  MANPAGE_PROCESSOR.with(|processor| {
    let result = processor.render(text);
    result
      .html
      .replace("<code>", "\\fR\\(oq")
      .replace("</code>", "\\(cq\\fP")
  })
}

/// Process option values (for defaults and examples)
fn process_value(text: &str) -> String {
  // Pre-check for troff formatting codes to preserve them
  let text = preserve_existing_formatting(text);

  // Process roles and inline code in values
  let with_roles = process_roles(&text);
  let with_code = process_inline_code(&with_roles);

  // Strip any residual HTML wrappers from renderer
  let without_wrappers = with_code
    .replace("<html><head></head><body>", "")
    .replace("</body></html>", "")
    .replace("<html>", "")
    .replace("</html>", "")
    .replace("<body>", "")
    .replace("</body>", "")
    .replace("<p>", "")
    .replace("</p>", "");

  // Restore formatting codes
  let with_formatting_restored = restore_formatting(&without_wrappers);

  // Escape any troff special characters and ensure leading dots are safe
  escape_leading_dots(&selective_man_escape(&with_formatting_restored))
}

/// Process example text for troff format
fn process_example(text: &str) -> String {
  // Pre-check for troff formatting codes to preserve them
  let text = preserve_existing_formatting(text);

  // Process roles, prompts, and code in example text
  let with_roles = process_roles(&text);
  let with_prompts = process_command_prompts(&with_roles);
  let with_repl = process_repl_prompts(&with_prompts);

  // Strip any residual HTML wrappers
  let without_wrappers = with_repl
    .replace("<html><head></head><body>", "")
    .replace("</body></html>", "")
    .replace("<html>", "")
    .replace("</html>", "")
    .replace("<body>", "")
    .replace("</body>", "")
    .replace("<p>", "")
    .replace("</p>", "");

  // Restore formatting codes
  let with_formatting_restored = restore_formatting(&without_wrappers);

  // Return the processed example
  escape_leading_dots(&selective_man_escape(&with_formatting_restored))
}

/// Ensure no leading dots in any line of text
fn escape_leading_dots(text: &str) -> String {
  text
    .lines()
    .map(escape_leading_dot)
    .collect::<Vec<_>>()
    .join("\n")
}
