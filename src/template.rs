use anyhow::{Context, Result};
use std::{collections::HashMap, fs, path::Path};

use crate::config::Config;
use crate::markdown::Header;
use crate::options::NixOption;

// Template constants
const DEFAULT_TEMPLATE: &str = include_str!("../templates/default.html");
const OPTIONS_TEMPLATE: &str = include_str!("../templates/options.html");
const SEARCH_TEMPLATE: &str = include_str!("../templates/search.html");

/// Render a documentation page
pub fn render(
    config: &Config,
    content: &str,
    title: &str,
    headers: &[Header],
    _rel_path: &Path,
) -> Result<String> {
    // Get template
    let template_content = get_template_content(config)?;

    // Generate table of contents from headers
    let toc = generate_toc(headers);

    // Generate document navigation
    let doc_nav = generate_doc_nav(config)?;

    // Check if options are available
    let has_options = if config.module_options.is_some() {
        ""
    } else {
        "style=\"display:none;\""
    };

    // Generate custom scripts HTML
    let custom_scripts = generate_custom_scripts(config)?;

    // Replace template variables
    let html = template_content
        .replace("{{content}}", content)
        .replace("{{title}}", title)
        .replace("{{site_title}}", &config.title)
        .replace("{{footer_text}}", &config.footer_text)
        .replace("{{toc}}", &toc)
        .replace("{{doc_nav}}", &doc_nav)
        .replace("{{has_options}}", has_options)
        .replace("{{custom_scripts}}", &custom_scripts);

    Ok(html)
}

/// Render NixOS module options page
pub fn render_options(config: &Config, options: &HashMap<String, NixOption>) -> Result<String> {
    // Create options HTML
    let options_html = generate_options_html(options);

    // Generate document navigation
    let doc_nav = generate_doc_nav(config)?;

    // Generate custom scripts HTML
    let custom_scripts = generate_custom_scripts(config)?;

    // Replace template variables
    let html = OPTIONS_TEMPLATE
        .replace("{{title}}", &format!("{} - Options", config.title))
        .replace("{{site_title}}", &config.title)
        .replace("{{heading}}", &format!("{} Options", config.title))
        .replace("{{options}}", &options_html)
        .replace("{{footer_text}}", &config.footer_text)
        .replace("{{custom_scripts}}", &custom_scripts)
        .replace("{{doc_nav}}", &doc_nav)
        .replace("{{has_options}}", "class=\"active\"")
        .replace("{{toc}}", ""); // No TOC for options page

    Ok(html)
}

/// Render search page
pub fn render_search(config: &Config, context: &HashMap<&str, String>) -> Result<String> {
    let title_str = context
        .get("title")
        .cloned()
        .unwrap_or_else(|| format!("{} - Search", config.title));

    // Generate document navigation
    let doc_nav = generate_doc_nav(config)?;

    // Generate custom scripts HTML
    let custom_scripts = generate_custom_scripts(config)?;

    // Check if options are available
    let has_options = if config.module_options.is_some() {
        ""
    } else {
        "style=\"display:none;\""
    };

    // Replace template variables
    let html = SEARCH_TEMPLATE
        .replace("{{title}}", &title_str)
        .replace("{{site_title}}", &config.title)
        .replace("{{heading}}", "Search")
        .replace("{{footer_text}}", &config.footer_text)
        .replace("{{custom_scripts}}", &custom_scripts)
        .replace("{{doc_nav}}", &doc_nav)
        .replace("{{has_options}}", has_options)
        .replace("{{toc}}", ""); // No TOC for search page

    Ok(html)
}

/// Get the template content from file or use default
fn get_template_content(config: &Config) -> Result<String> {
    if let Some(template_path) = &config.template_path {
        if template_path.exists() {
            fs::read_to_string(template_path).context(format!(
                "Failed to read template file: {}",
                template_path.display()
            ))
        } else {
            Ok(DEFAULT_TEMPLATE.to_string())
        }
    } else {
        Ok(DEFAULT_TEMPLATE.to_string())
    }
}

/// Generate the document navigation HTML
fn generate_doc_nav(config: &Config) -> Result<String> {
    let mut doc_nav = String::new();
    let entries: Vec<_> = walkdir::WalkDir::new(&config.input_dir)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file() && e.path().extension().is_some_and(|ext| ext == "md"))
        .collect();

    if !entries.is_empty() {
        for entry in entries {
            let path = entry.path();
            if let Ok(rel_doc_path) = path.strip_prefix(&config.input_dir) {
                let mut html_path = rel_doc_path.to_path_buf();
                html_path.set_extension("html");

                let page_title = if let Ok(content) = fs::read_to_string(path) {
                    if let Some(first_line) = content.lines().next() {
                        if let Some(title) = first_line.strip_prefix("# ") {
                            title.trim().to_string()
                        } else {
                            html_path
                                .file_stem()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .to_string()
                        }
                    } else {
                        html_path
                            .file_stem()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string()
                    }
                } else {
                    html_path
                        .file_stem()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string()
                };

                doc_nav.push_str(&format!(
                    "<li><a href=\"{}\">{}</a></li>\n",
                    html_path.to_string_lossy(),
                    page_title
                ));
            }
        }
    }

    Ok(doc_nav)
}

/// Generate custom scripts HTML
fn generate_custom_scripts(config: &Config) -> Result<String> {
    let mut custom_scripts =
        String::with_capacity(config.script_paths.iter().fold(0, |acc, path| {
            if path.exists() {
                acc + 1000 // Rough estimate for script size
            } else {
                acc
            }
        }));

    for script_path in &config.script_paths {
        if script_path.exists() {
            let script_content = fs::read_to_string(script_path).context(format!(
                "Failed to read script file: {}",
                script_path.display()
            ))?;
            custom_scripts.push_str(&format!("<script>{}</script>\n", script_content));
        }
    }

    Ok(custom_scripts)
}

/// Generate table of contents from headers
fn generate_toc(headers: &[Header]) -> String {
    let mut toc = String::new();
    let mut current_level = 0;

    for header in headers {
        // Only include h1, h2, h3 in TOC
        if header.level <= 3 {
            // Adjust TOC nesting
            while current_level < header.level - 1 {
                toc.push_str("<ul>");
                current_level += 1;
            }
            while current_level > header.level - 1 {
                toc.push_str("</ul>");
                current_level -= 1;
            }

            // Add TOC item
            if current_level == header.level - 1 {
                toc.push_str("<li>");
            } else {
                toc.push_str("</li><li>");
            }

            toc.push_str(&format!("<a href=\"#{}\">{}</a>", header.id, header.text));
        }
    }

    // Close open tags
    while current_level > 0 {
        toc.push_str("</li></ul>");
        current_level -= 1;
    }
    if !headers.is_empty() && !toc.is_empty() && !toc.ends_with("</li>") {
        toc.push_str("</li>");
    }

    toc
}

/// Generate the options HTML content
fn generate_options_html(options: &HashMap<String, NixOption>) -> String {
    let mut options_html = String::with_capacity(options.len() * 500); // Rough estimate for capacity

    // Sort options by name
    let mut option_keys: Vec<_> = options.keys().collect();
    option_keys.sort();

    for key in option_keys {
        let option = &options[key];
        let option_id = format!("option-{}", option.name.replace(".", "-"));

        // Open option container with ID for direct linking
        options_html.push_str(&format!("<div class=\"option\" id=\"{}\">\n", option_id));

        // Option name with anchor link and copy button
        options_html.push_str(&format!(
            "  <h3 class=\"option-name\">\n    <a href=\"#{}\" class=\"option-anchor\">{}</a>\n    <span class=\"copy-link\" title=\"Copy link to this option\"></span>\n    <span class=\"copy-feedback\">Link copied!</span>\n  </h3>\n",
            option_id, option.name
        ));

        // Option metadata (internal/readOnly)
        let mut metadata = Vec::new();
        if option.internal {
            metadata.push("internal");
        }
        if option.read_only {
            metadata.push("read-only");
        }

        if !metadata.is_empty() {
            options_html.push_str(&format!(
                "  <div class=\"option-metadata\">{}</div>\n",
                metadata.join(", ")
            ));
        }

        // Option type
        options_html.push_str(&format!(
            "  <div class=\"option-type\">Type: <code>{}</code></div>\n",
            option.type_name
        ));

        // Option description
        options_html.push_str(&format!(
            "  <div class=\"option-description\">{}</div>\n",
            option.description
        ));

        // Add default value if available
        add_default_value(&mut options_html, option);

        // Add example if available
        add_example_value(&mut options_html, option);

        // Option declared in
        if let Some(declared_in) = &option.declared_in {
            options_html.push_str(&format!(
                "  <div class=\"option-declared\">Declared in: <code>{}</code></div>\n",
                declared_in
            ));
        }

        // Close option div
        options_html.push_str("</div>\n");
    }

    options_html
}

/// Add default value to options HTML
fn add_default_value(html: &mut String, option: &NixOption) {
    if let Some(default_text) = &option.default_text {
        html.push_str(&format!(
            "  <div class=\"option-default\">Default: <code>{}</code></div>\n",
            default_text
        ));
    } else if let Some(default_val) = &option.default {
        html.push_str(&format!(
            "  <div class=\"option-default\">Default: <code>{}</code></div>\n",
            default_val
        ));
    }
}

/// Add example value to options HTML
fn add_example_value(html: &mut String, option: &NixOption) {
    if let Some(example_text) = &option.example_text {
        if example_text.contains('\n') {
            html.push_str(&format!(
                "  <div class=\"option-example\">Example: <pre><code>{}</code></pre></div>\n",
                example_text
            ));
        } else {
            html.push_str(&format!(
                "  <div class=\"option-example\">Example: <code>{}</code></div>\n",
                example_text
            ));
        }
    } else if let Some(example_val) = &option.example {
        let example_str = example_val.to_string();
        if example_str.contains('\n') {
            html.push_str(&format!(
                "  <div class=\"option-example\">Example: <pre><code>{}</code></pre></div>\n",
                example_str
            ));
        } else {
            html.push_str(&format!(
                "  <div class=\"option-example\">Example: <code>{}</code></div>\n",
                example_str
            ));
        }
    }
}
