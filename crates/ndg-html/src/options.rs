use std::{fs, path::Path};

use color_eyre::eyre::{Context, Result};
use indexmap::IndexMap;
use log::debug;
use ndg_commonmark::MarkdownProcessor;
use ndg_config::Config;
use ndg_manpage::types::NixOption;
use ndg_utils::{
  markdown::create_processor,
  options::{OptionLocation, compare_option_locs, parse_options_json},
  postprocess,
};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::{option_page_render, option_pages, template};

/// Process options from a JSON file and generate the options documentation
/// page.
///
/// Reads the options JSON file, parses all options, sorts and formats them,
/// and writes the rendered HTML to the output directory specified in the
/// config.
///
/// # Arguments
///
/// * `config` - The loaded configuration for documentation generation.
/// * `options_path` - Path to the options.json file.
///
/// # Errors
///
/// Returns an error if the file cannot be read, parsed, or written.
pub fn process_options(config: &Config, options_path: &Path) -> Result<()> {
  // Read options JSON
  let json_content = fs::read_to_string(options_path).wrap_err_with(|| {
    format!("Failed to read options file: {}", options_path.display())
  })?;

  let options_data = parse_options_json(&json_content).wrap_err_with(|| {
    format!(
      "Failed to validate options JSON at {}",
      options_path.display()
    )
  })?;

  // First pass: collect all option names for validation
  let mut valid_options = FxHashSet::default();
  for key in options_data.keys() {
    valid_options.insert(key.clone());
  }

  // Create processor once with validation enabled
  let processor = create_processor(config, Some(valid_options))?;

  // Extract options
  let mut options: IndexMap<String, NixOption> = IndexMap::default();

  for (key, option_data) in options_data {
    let description_text = option_data
      .description
      .as_ref()
      .map_or_else(String::new, |text| text.text().to_string());
    let description = option_data.description.as_ref().map_or_else(
      String::new,
      |description| {
        let markdown = if description.is_markdown_documentation() {
          description.text().to_string()
        } else {
          escape_html_in_markdown(description.text())
        };
        processor.render(&markdown).html
      },
    );
    let related_packages = option_data
      .related_packages
      .as_ref()
      .map(|text| processor.render(text).html);

    let internal = option_data.is_hidden();
    let read_only = option_data.read_only;
    let mut option = NixOption {
      name: key.clone(),
      type_name: option_data.type_name,
      description,
      default: option_data.default,
      example: option_data.example,
      loc: option_data.loc.clone(),
      declared_in_url: option_data.declaration_url,
      related_packages,
      internal,
      read_only,
      ..Default::default()
    };

    if let Some(declaration) = option_data.declarations.first() {
      let (display, url) = format_location(declaration, &config.revision);
      option.declared_in = display;
      if url.is_some() {
        option.declared_in_url = url;
      }
    }

    for definition in &option_data.definitions {
      let (display, url) = format_location(definition, &config.revision);
      if let Some(display) = display {
        option.defined_in.push((display, url));
      }
    }

    let has_default = option.default.is_some();
    let has_description = !description_text.trim().is_empty();
    if let Some(filter) = config
      .options
      .as_ref()
      .and_then(|options| options.filter.as_ref())
      && !filter.matches(
        &option.name,
        &option.type_name,
        &description_text,
        has_default,
        has_description,
        option.internal,
      )
    {
      continue;
    }

    if option.declared_in.is_none() && !option_data.loc.is_empty() {
      let location = option_data.loc.join(".");
      debug!("Set declared_in from loc: {location}");
      option.declared_in = Some(location);
    }

    if option.declared_in.is_none() {
      option.declared_in = Some("configuration.nix".to_string());
      debug!("Using fallback declared_in for {key}");
    }

    options.insert(key, option);
  }

  let input_order = options
    .keys()
    .enumerate()
    .map(|(index, name)| (name.clone(), index))
    .collect();

  // Match nixos-render-docs' component-wise option ordering.
  let mut sorted: Vec<_> = options.into_iter().collect();
  sorted.sort_by(|(name_a, option_a), (name_b, option_b)| {
    compare_option_locs(&option_a.loc, &option_b.loc)
      .then_with(|| name_a.cmp(name_b))
  });
  let customized_options = sorted.into_iter().collect();

  if option_pages::pages_enabled(config) {
    write_split_options(config, &customized_options, &input_order, &processor)?;
  } else {
    let html = template::render_options_with_order(
      config,
      &customized_options,
      Some(&input_order),
      &processor,
    )?;
    write_options_html(config, "options.html", html)?;
  }

  Ok(())
}

fn write_split_options(
  config: &Config,
  options: &IndexMap<String, NixOption>,
  input_order: &FxHashMap<String, usize>,
  processor: &MarkdownProcessor,
) -> Result<()> {
  let page_set = option_pages::build_option_pages(config, options);
  let index_html =
    option_page_render::render_options_index(config, &page_set, processor)?;
  write_options_html(config, "options.html", index_html)?;

  for page in &page_set.pages {
    let html = option_page_render::render_option_page(
      config,
      page,
      &page_set.pages,
      input_order,
      processor,
    )?;
    write_options_html(config, &page.path, html)?;
  }

  Ok(())
}

fn write_options_html(
  config: &Config,
  relative_path: &str,
  html: String,
) -> Result<()> {
  let processed_html = if let Some(ref postprocess) = config.postprocess {
    postprocess::process_html(&html, postprocess)?
  } else {
    html
  };

  let output_path = config.output_dir.join(relative_path);
  if let Some(parent) = output_path.parent() {
    fs::create_dir_all(parent).wrap_err_with(|| {
      format!("Failed to create options directory: {}", parent.display())
    })?;
  }

  fs::write(&output_path, processed_html).wrap_err_with(|| {
    format!("Failed to write options file: {}", output_path.display())
  })?;

  Ok(())
}

/// Format location with URL handling based on nixos-render-docs.
///
/// Converts a location value from the options JSON into a display string and
/// an optional URL, supporting both local and GitHub-based references.
///
/// # Arguments
///
/// * `loc_value` - The JSON value representing the location.
/// * `revision` - The git revision for GitHub links.
///
/// # Returns
///
/// A tuple of (display string, URL).
fn format_location(
  location: &OptionLocation,
  revision: &str,
) -> (Option<String>, Option<String>) {
  match location {
    // Handle string path
    OptionLocation::Path(path) => {
      let path_str = path.as_str();

      if path_str.starts_with('/') {
        let url = format!("file://{path_str}");
        if path_str.contains("nixops") && path_str.contains("/nix/") {
          let suffix_index = path_str.find("/nix/").map_or(0, |i| i + 5);
          let suffix = &path_str[suffix_index..];
          (Some(format!("<nixops/{suffix}>")), Some(url))
        } else {
          (Some(path_str.to_string()), Some(url))
        }
      } else {
        // Path is relative to nixpkgs repo
        let url = if revision == "local" {
          format!("https://github.com/NixOS/nixpkgs/blob/master/{path_str}")
        } else {
          format!("https://github.com/NixOS/nixpkgs/blob/{revision}/{path_str}")
        };

        // Format display name
        let display = format!("<nixpkgs/{path_str}>");
        (Some(display), Some(url))
      }
    },

    // Handle object with name and url
    OptionLocation::Link { name, url } => (name.clone(), url.clone()),
  }
}

/// Escape HTML tags in markdown text before it's processed by the markdown
/// processor, but preserve code blocks.
///
/// This function ensures that angle brackets are escaped outside of code
/// blocks, preventing accidental HTML injection in documentation.
///
/// # Arguments
///
/// * `text` - The markdown text to escape.
///
/// # Returns
///
/// The escaped string, safe for markdown rendering.
fn escape_html_in_markdown(text: &str) -> String {
  // Split the text on backticks (`) to separate code blocks from regular text
  let mut result = String::with_capacity(text.len());

  // Track code block state
  let mut in_code_block = false;
  let mut in_inline_code = false;
  let mut backquote_counter = 0;

  for c in text.chars() {
    if c == '`' {
      // Manage backtick counting for code blocks
      backquote_counter += 1;

      if backquote_counter == 3 && !in_inline_code {
        // Toggle fenced code block state
        in_code_block = !in_code_block;
        backquote_counter = 0; // reset counter after handling triple backticks
      } else if backquote_counter == 1 && !in_code_block {
        // Toggle inline code state
        in_inline_code = !in_inline_code;
      }

      result.push(c);
    } else {
      // Reset backtick counter when not a backtick
      if backquote_counter > 0 && backquote_counter < 3 {
        // We've seen some backticks but not enough for a code block
        // These are either inline code delimiters or just literal backticks
        backquote_counter = 0;
      }

      // Handle character based on context
      if c == '<' && !in_code_block && !in_inline_code {
        result.push_str("&lt;");
      } else if c == '>' && !in_code_block && !in_inline_code {
        result.push_str("&gt;");
      } else {
        result.push(c);
      }
    }
  }

  result
}
