use anyhow::{Context, Result};
use std::{collections::HashMap, fs, path::Path};

use crate::config::Config;
use crate::markdown::Header;
use crate::options::NixOption;

pub fn render(
    config: &Config,
    content: &str,
    title: &str,
    headers: &[Header],
    _rel_path: &Path,
) -> Result<String> {
    // Get template
    let template_content = if let Some(template_path) = &config.template_path {
        if template_path.exists() {
            fs::read_to_string(template_path).context(format!(
                "Failed to read template file: {}",
                template_path.display()
            ))?
        } else {
            include_str!("../templates/default.html").to_string()
        }
    } else {
        include_str!("../templates/default.html").to_string()
    };

    // Generate document navigation
    let mut doc_nav = String::new();

    // Convert headers to TOC HTML
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

    // Get list of all markdown files
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

                let mut page_title = html_path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();

                if let Ok(content) = fs::read_to_string(path) {
                    if let Some(first_line) = content.lines().next() {
                        if let Some(title) = first_line.strip_prefix("# ") {
                            page_title = title.trim().to_string();
                        }
                    }
                }

                doc_nav.push_str(&format!(
                    "<li><a href=\"{}\">{}</a></li>\n",
                    html_path.to_string_lossy(),
                    page_title
                ));
            }
        }
    }

    // Check if active page
    let has_options = if config.options_json.is_some() {
        ""
    } else {
        "style=\"display:none;\""
    };

    // Replace template variables
    let mut html = template_content;
    html = html.replace("{{content}}", content);
    html = html.replace("{{title}}", title);
    html = html.replace("{{site_title}}", &config.title);
    html = html.replace("{{footer_text}}", &config.footer_text);
    html = html.replace("{{toc}}", &toc);
    html = html.replace("{{doc_nav}}", &doc_nav);
    html = html.replace("{{has_options}}", has_options);

    Ok(html)
}

pub fn render_options(config: &Config, options: &HashMap<String, NixOption>) -> Result<String> {
    // Get options template
    let template_content = include_str!("../templates/options.html").to_string();

    // Create options HTML
    let mut options_html = String::new();

    // Sort options by name
    let mut option_keys: Vec<_> = options.keys().collect();
    option_keys.sort();

    for key in option_keys {
        let option = &options[key];
        let option_id = format!("option-{}", option.name.replace(".", "-"));

        // Build the option HTML
        let mut option_html = String::new();

        // Add the option container with ID for direct linking
        option_html.push_str(&format!("<div class=\"option\" id=\"{}\">\n", option_id));

        // Option name with anchor link and copy button
        option_html.push_str(&format!(
            "  <h3 class=\"option-name\">\n    <a href=\"#{}\" class=\"option-anchor\">{}</a>\n    <span class=\"copy-link\" title=\"Copy link to this option\"></span>\n    <span class=\"copy-feedback\">Link copied!</span>\n  </h3>\n",
            option_id, option.name
        ));

        // Option type
        option_html.push_str(&format!(
            "  <div class=\"option-type\">Type: <code>{}</code></div>\n",
            option.type_name
        ));

        // Option description
        option_html.push_str(&format!(
            "  <div class=\"option-description\">{}</div>\n",
            option.description
        ));

        // Option default
        if let Some(default_text) = &option.default_text {
            option_html.push_str(&format!(
                "  <div class=\"option-default\">Default: <code>{}</code></div>\n",
                default_text
            ));
        } else if let Some(default_val) = &option.default {
            option_html.push_str(&format!(
                "  <div class=\"option-default\">Default: <code>{}</code></div>\n",
                default_val
            ));
        }

        // Option example
        if let Some(example_text) = &option.example_text {
            option_html.push_str(&format!(
                "  <div class=\"option-example\">Example: <code>{}</code></div>\n",
                example_text
            ));
        } else if let Some(example_val) = &option.example {
            option_html.push_str(&format!(
                "  <div class=\"option-example\">Example: <code>{}</code></div>\n",
                example_val
            ));
        }

        // Option declared in
        if let Some(declared_in) = &option.declared_in {
            option_html.push_str(&format!(
                "  <div class=\"option-declared\">Declared in: <code>{}</code></div>\n",
                declared_in
            ));
        }

        // Close option div
        option_html.push_str("</div>\n");

        options_html.push_str(&option_html);
    }

    // Replace template variables
    let mut html = template_content;
    html = html.replace("{{title}}", &format!("{} - Options", config.title));
    html = html.replace("{{site_title}}", &config.title);
    html = html.replace("{{heading}}", &format!("{} Options", config.title));
    html = html.replace("{{options}}", &options_html);
    html = html.replace("{{footer_text}}", &config.footer_text);

    // Generate document navigation
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

                let mut page_title = html_path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();

                if let Ok(content) = fs::read_to_string(path) {
                    if let Some(first_line) = content.lines().next() {
                        if let Some(title) = first_line.strip_prefix("# ") {
                            page_title = title.trim().to_string();
                        }
                    }
                }

                doc_nav.push_str(&format!(
                    "<li><a href=\"{}\">{}</a></li>\n",
                    html_path.to_string_lossy(),
                    page_title
                ));
            }
        }
    }

    html = html.replace("{{doc_nav}}", &doc_nav);
    html = html.replace("{{has_options}}", "class=\"active\"");
    html = html.replace("{{toc}}", ""); // No TOC for options page

    Ok(html)
}

pub fn render_search(config: &Config, context: &HashMap<&str, String>) -> Result<String> {
    // Get search template
    let template_content = include_str!("../templates/search.html").to_string();

    // Create a local variable to avoid the temporary value issue
    let title_str = context
        .get("title")
        .cloned()
        .unwrap_or_else(|| format!("{} - Search", config.title));

    // Replace template variables
    let mut html = template_content;
    html = html.replace("{{title}}", &title_str);
    html = html.replace("{{site_title}}", &config.title);
    html = html.replace("{{heading}}", "Search");
    html = html.replace("{{footer_text}}", &config.footer_text);

    // Generate document navigation
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

                let mut page_title = html_path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();

                if let Ok(content) = fs::read_to_string(path) {
                    if let Some(first_line) = content.lines().next() {
                        if let Some(title) = first_line.strip_prefix("# ") {
                            page_title = title.trim().to_string();
                        }
                    }
                }

                doc_nav.push_str(&format!(
                    "<li><a href=\"{}\">{}</a></li>\n",
                    html_path.to_string_lossy(),
                    page_title
                ));
            }
        }
    }

    html = html.replace("{{doc_nav}}", &doc_nav);

    // Check if active page
    let has_options = if config.options_json.is_some() {
        ""
    } else {
        "style=\"display:none;\""
    };
    html = html.replace("{{has_options}}", has_options);
    html = html.replace("{{toc}}", ""); // No TOC for search page

    Ok(html)
}
