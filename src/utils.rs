use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use log::{debug, info};
use rayon::iter::IntoParallelRefIterator;
use rayon::prelude::*;

use crate::completion;
use crate::config::Config;
use crate::formatter::{markdown, options};
use crate::html;
use crate::manpage;

// Template constants
const DEFAULT_CSS: &str = include_str!("../templates/default.css");
const SEARCH_JS: &str = include_str!("../templates/search.js");

/// Copy assets to output directory
pub fn copy_assets(config: &Config) -> Result<()> {
    // Create assets directory
    let assets_dir = config.output_dir.join("assets");
    fs::create_dir_all(&assets_dir)?;

    // Generate and write CSS
    let css = generate_css(config)?;
    fs::write(assets_dir.join("style.css"), css).context("Failed to write CSS file")?;

    // Copy custom assets if they exist
    copy_custom_assets(config, &assets_dir)?;

    // Copy script files to assets directory
    copy_script_files(config, &assets_dir)?;

    // Create search.js for search functionality
    if config.generate_search {
        // Use template from config if available, otherwise use embedded template
        let search_js = if let Some(path) = config.get_template_file("search.js") {
            if path.exists() {
                fs::read_to_string(&path)
                    .with_context(|| format!("Failed to read search.js from: {}", path.display()))?
            } else {
                SEARCH_JS.to_string()
            }
        } else {
            SEARCH_JS.to_string()
        };

        fs::write(assets_dir.join("search.js"), search_js).context("Failed to write search.js")?;
    }

    Ok(())
}

/// Copy custom assets from the configured directory
fn copy_custom_assets(config: &Config, assets_dir: &Path) -> Result<()> {
    if let Some(custom_assets_dir) = &config.assets_dir {
        if custom_assets_dir.exists() && custom_assets_dir.is_dir() {
            debug!("Copying custom assets from {}", custom_assets_dir.display());

            let options = fs_extra::dir::CopyOptions::new().overwrite(true);
            fs_extra::dir::copy(custom_assets_dir, assets_dir, &options)
                .context("Failed to copy custom assets")?;
        }
    }
    Ok(())
}

/// Copy script files to the assets directory
fn copy_script_files(config: &Config, assets_dir: &Path) -> Result<()> {
    for script_path in &config.script_paths {
        if script_path.exists() {
            let file_name = script_path
                .file_name()
                .ok_or_else(|| anyhow::anyhow!("Invalid script filename"))?;
            let dest_path = assets_dir.join(file_name);

            fs::read(script_path)
                .with_context(|| format!("Failed to read script file {}", script_path.display()))
                .and_then(|content| {
                    fs::write(&dest_path, content).with_context(|| {
                        format!("Failed to write script file to {}", dest_path.display())
                    })
                })?;
        }
    }
    Ok(())
}

/// Generate CSS from stylesheet
fn generate_css(config: &Config) -> Result<String> {
    // Check for custom stylesheet
    if let Some(stylesheet_path) = &config.stylesheet_path {
        if stylesheet_path.exists() {
            let content = fs::read_to_string(stylesheet_path).with_context(|| {
                format!("Failed to read stylesheet: {}", stylesheet_path.display())
            })?;

            // Process SCSS if needed
            if stylesheet_path.extension().is_some_and(|ext| ext == "scss") {
                return grass::from_string(content, &grass::Options::default())
                    .context("Failed to compile SCSS to CSS");
            }
            return Ok(content);
        }
    }

    // Fall back to template CSS if available
    if let Some(template_path) = config.get_template_path() {
        let template_css_path = template_path.join("default.css");
        if template_css_path.exists() {
            return fs::read_to_string(&template_css_path).with_context(|| {
                format!(
                    "Failed to read template CSS: {}",
                    template_css_path.display()
                )
            });
        }
    }

    // Fall back to embedded default
    Ok(DEFAULT_CSS.to_string())
}

/// Handle the generate command
pub fn handle_generate_command(
    output_dir: &std::path::Path,
    completions_only: bool,
    manpage_only: bool,
) -> Result<()> {
    if completions_only {
        completion::generate_comp(output_dir)?;
        println!("Shell completions generated in {}", output_dir.display());
    } else if manpage_only {
        completion::generate_manpage(output_dir)?;
        println!("Manpage generated in {}", output_dir.display());
    } else {
        completion::generate_all(output_dir)?;
    }
    Ok(())
}

/// Generate a manpage from options JSON
pub fn generate_options_manpage(
    module_options: &Path,
    output_file: Option<&Path>,
    title: Option<&str>,
    header: Option<&str>,
    footer: Option<&str>,
    section: u8,
) -> Result<()> {
    info!("Generating manpage from options JSON");
    manpage::generate_manpage(module_options, output_file, title, header, footer, section)?;
    Ok(())
}

/// Process markdown files
pub fn process_markdown_files(config: &Config) -> Result<Vec<std::path::PathBuf>> {
    if let Some(ref input_dir) = config.input_dir {
        info!("Input directory: {}", input_dir.display());
        let files = markdown::collect_markdown_files(input_dir);
        info!("Found {} markdown files", files.len());

        if !files.is_empty() {
            // Process files in parallel
            files
                .par_iter()
                .try_for_each(|file_path| markdown::process_markdown_file(config, file_path))?;
        }

        Ok(files)
    } else {
        info!("No input directory provided, skipping markdown processing");
        Ok(Vec::new())
    }
}

/// Process options file
pub fn process_options_file(config: &Config) -> Result<bool> {
    if let Some(options_path) = &config.module_options {
        info!("Processing options.json from {}", options_path.display());
        options::process_options(config, options_path)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Create a fallback index page
pub fn create_fallback_index(config: &Config, markdown_files: &[std::path::PathBuf]) -> String {
    let mut content = format!(
        "<h1>{}</h1>\n<p>This is a fallback page created by ndg.</p>",
        &config.title
    );

    // Add file listing if we have an input directory
    if let Some(input_dir) = &config.input_dir {
        if !markdown_files.is_empty() {
            let mut file_list = String::with_capacity(markdown_files.len() * 100); // Preallocate based on estimated size
            file_list.push_str("<h2>Available Documents</h2>\n<ul>\n");

            for file_path in markdown_files {
                if let Ok(rel_path) = file_path.strip_prefix(input_dir) {
                    let mut html_path = rel_path.to_path_buf();
                    html_path.set_extension("html");

                    // Get page title from first heading or filename
                    let page_title = extract_page_title(file_path, &html_path);

                    use std::fmt::Write;
                    writeln!(
                        file_list,
                        "  <li><a href=\"{}\">{}</a></li>",
                        html_path.to_string_lossy(),
                        page_title
                    )
                    .unwrap();
                }
            }

            file_list.push_str("</ul>");
            content.push_str(&file_list);
        }
    }

    content
}

/// Extract the page title from a markdown file
pub fn extract_page_title(file_path: &std::path::Path, html_path: &std::path::Path) -> String {
    let default_title = html_path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    // Try to read the file content
    match fs::read_to_string(file_path) {
        Ok(content) => {
            if let Some(first_line) = content.lines().next() {
                first_line
                    .strip_prefix("# ")
                    .map_or(default_title, |title| title.trim().to_string())
            } else {
                default_title
            }
        }
        Err(_) => default_title,
    }
}

/// Ensure that index.html exists in the output directory
pub fn ensure_index(
    config: &Config,
    options_processed: bool,
    markdown_files: &[std::path::PathBuf],
) -> Result<()> {
    let index_path = config.output_dir.join("index.html");

    // Check if index.html already exists (already generated from index.md)
    if index_path.exists() {
        return Ok(());
    }

    // If options were processed, try to use options.html as index
    if options_processed {
        let options_path = config.output_dir.join("options.html");
        if options_path.exists() {
            info!("Using options.html as index.html");

            // Copy the content of options.html to index.html
            let options_content = fs::read(&options_path)?;
            fs::write(&index_path, options_content)?;
            return Ok(());
        }
    }

    // Create a fallback index page if input directory was provided but no index.md existed
    if config.input_dir.is_some() {
        info!("No index.md found, creating fallback index.html");

        let content = create_fallback_index(config, markdown_files);

        // Create a simple HTML document using our template system
        let headers = vec![markdown::Header {
            text: config.title.clone(),
            level: 1,
            id: "welcome".to_string(),
        }];

        let html = html::template::render(
            config,
            &content,
            &config.title,
            &headers,
            &std::path::PathBuf::from("index.html"),
        )?;

        fs::write(&index_path, html)?;
    }

    Ok(())
}
