use anyhow::{anyhow, Context as AnyhowContext, Result};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::config::Config;
use crate::markdown::Header;

/// Render markdown content with template
pub fn render(
    config: &Config,
    content: &str,
    title: &str,
    headers: &[Header],
    rel_path: &Path,
) -> Result<String> {
    // Use custom template if provided, otherwise use default
    let template_content = if let Some(template_path) = config.get_template_path() {
        if template_path.exists() {
            fs::read_to_string(&template_path).context(format!(
                "Failed to read template file: {}",
                template_path.display()
            ))?
        } else {
            return Err(anyhow!(
                "Template file not found: {}",
                template_path.display()
            ));
        }
    } else {
        include_str!("../templates/default.html").to_string()
    };

    // Replace template variables
    let mut html = template_content;

    // Replace basic variables
    html = html.replace("{{title}}", title);
    html = html.replace("{{site_title}}", &config.title);
    html = html.replace("{{content}}", content);
    html = html.replace("{{footer_text}}", &config.footer_text);

    // Add conditional flag for options page
    let has_options = config.options_json.is_some();
    html = html.replace(
        "{{has_options}}",
        if has_options {
            ""
        } else {
            "style=\"display:none\""
        },
    );

    // Generate document navigation
    let mut doc_nav = String::new();

    let entries: Vec<_> = walkdir::WalkDir::new(&config.input_dir)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file() && e.path().extension().map_or(false, |ext| ext == "md"))
        .collect();

    // Get list of all markdown files
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
                        if first_line.starts_with("# ") {
                            page_title = first_line[2..].trim().to_string();
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

    // Replace navigation with proper HTML links
    let mut toc = String::new();
    if !headers.is_empty() {
        for header in headers {
            toc.push_str(&format!(
                "<li class=\"toc-level-{}\"><a href=\"#{}\">{}</a></li>\n",
                header.level, header.id, header.text
            ));
        }
    } else {
        toc = "<li>No headings found</li>".to_string();
    }
    html = html.replace("{{toc}}", &toc);

    // Replace relative path
    html = html.replace("{{relPath}}", &rel_path.to_string_lossy().to_string());

    // Add current date and time
    use chrono::prelude::*;
    let now = Utc::now();
    html = html.replace(
        "{{current_date}}",
        &now.format("%Y-%m-%d %H:%M:%S").to_string(),
    );

    Ok(html)
}

/// Render options page
pub fn render_options(
    config: &Config,
    options: &HashMap<String, crate::options::NixOption>,
) -> Result<String> {
    // Create options HTML
    let mut options_html = String::new();

    // Sort options by name
    let mut option_keys: Vec<_> = options.keys().collect();
    option_keys.sort();

    for key in option_keys {
        let option = &options[key];

        options_html.push_str(&format!(
            r#"<div class="option" id="option-{id}">
                <h3 class="option-name">{name}</h3>
                <div class="option-type">Type: <code>{type}</code></div>
                <div class="option-description">{description}</div>
                {default_html}
                {example_html}
                {declared_html}
            </div>
            "#,
            id = option.name.replace(".", "-"),
            name = option.name,
            type = option.type_name,
            description = option.description,
            default_html = if let Some(default_text) = &option.default_text {
                format!("<div class=\"option-default\">Default: <code>{}</code></div>", default_text)
            } else if let Some(default_val) = &option.default {
                format!("<div class=\"option-default\">Default: <code>{}</code></div>", default_val)
            } else {
                String::new()
            },
            example_html = if let Some(example_text) = &option.example_text {
                format!("<div class=\"option-example\">Example: <code>{}</code></div>", example_text)
            } else if let Some(example_val) = &option.example {
                format!("<div class=\"option-example\">Example: <code>{}</code></div>", example_val)
            } else {
                String::new()
            },
            declared_html = if let Some(declared_in) = &option.declared_in {
                format!("<div class=\"option-declared\">Declared in: <code>{}</code></div>", declared_in)
            } else {
                String::new()
            }
        ));
    }

    let content = format!(
        r#"<h1>NixOS Options</h1>
        <p>This page lists all available configuration options.</p>
        <div class="search-form">
            <input type="text" id="options-filter" placeholder="Filter options...">
        </div>
        <div class="options-container">{}</div>
        <script>
            // Options filter functionality
            document.getElementById('options-filter').addEventListener('input', function(e) {{
                const searchTerm = e.target.value.toLowerCase();
                document.querySelectorAll('.option').forEach(function(option) {{
                    const name = option.querySelector('.option-name').textContent.toLowerCase();
                    const description = option.querySelector('.option-description').textContent.toLowerCase();

                    if (name.includes(searchTerm) || description.includes(searchTerm)) {{
                        option.style.display = '';
                    }} else {{
                        option.style.display = 'none';
                    }}
                }});
            }});
        </script>
        "#,
        options_html
    );

    render(
        config,
        &content,
        &format!("{} - Options", config.title),
        &[],
        &PathBuf::from("options.html"),
    )
}

/// Render search page
pub fn render_search(config: &Config, _context: &HashMap<&str, String>) -> Result<String> {
    let search_content = r#"
        <h1>Search Documentation</h1>
        <p>Use the search box below to find content throughout the documentation.</p>

        <div class="search-form">
            <input type="text" id="full-search" placeholder="Enter search terms...">
        </div>

        <div id="search-results-container">
            <h2>Results</h2>
            <div id="full-search-results"></div>
        </div>

        <script>
            // Search functionality
            document.addEventListener('DOMContentLoaded', function() {
                const searchInput = document.getElementById('full-search');
                const resultsContainer = document.getElementById('full-search-results');

                // Load search data
                fetch('assets/search-data.json')
                    .then(response => response.json())
                    .then(data => {
                        searchInput.addEventListener('input', function() {
                            const searchTerm = this.value.toLowerCase();
                            if (searchTerm.length < 2) {
                                resultsContainer.innerHTML = '<p>Please enter at least 2 characters</p>';
                                return;
                            }

                            const results = data.filter(doc =>
                                doc.title.toLowerCase().includes(searchTerm) ||
                                doc.content.toLowerCase().includes(searchTerm)
                            );

                            if (results.length === 0) {
                                resultsContainer.innerHTML = '<p>No results found</p>';
                            } else {
                                resultsContainer.innerHTML = results
                                    .slice(0, 20)
                                    .map(doc => `
                                        <div class="search-result">
                                            <h3><a href="${doc.path}">${doc.title}</a></h3>
                                            <p>${doc.content.substring(0, 200)}...</p>
                                        </div>
                                    `)
                                    .join('');
                            }
                        });
                    })
                    .catch(error => {
                        console.error('Error loading search data:', error);
                        resultsContainer.innerHTML = '<p>Error loading search data</p>';
                    });
            });
        </script>
    "#;

    render(
        config,
        search_content,
        &format!("{} - Search", config.title),
        &[],
        &PathBuf::from("search.html"),
    )
}
