use anyhow::{Context, Result};
use log::info;
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use crate::config::Config;

/// Search document data structure
#[derive(Debug, Serialize)]
pub struct SearchDocument {
    id: String,
    title: String,
    content: String,
    path: String,
}

/// Generate search index from markdown files
pub fn generate_search_index(config: &Config, markdown_files: &[PathBuf]) -> Result<()> {
    if !config.generate_search {
        return Ok(());
    }

    info!("Generating search index...");

    // Create search directory
    let search_dir = config.output_dir.join("assets");
    fs::create_dir_all(&search_dir)?;

    // Create search index data
    let mut documents = Vec::new();

    for (i, file_path) in markdown_files.iter().enumerate() {
        let content = fs::read_to_string(file_path).context(format!(
            "Failed to read file for search indexing: {}",
            file_path.display()
        ))?;

        let title = extract_title(&content).unwrap_or_else(|| {
            file_path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string()
        });

        let plain_text = strip_markdown(&content);

        let rel_path = file_path.strip_prefix(&config.input_dir).context(format!(
            "Failed to determine relative path for {}",
            file_path.display()
        ))?;

        let mut output_path = rel_path.to_owned();
        output_path.set_extension("html");

        documents.push(SearchDocument {
            id: i.to_string(),
            title,
            content: plain_text,
            path: output_path.to_string_lossy().to_string(),
        });
    }

    // Write search index data
    let search_data_path = search_dir.join("search-data.json");
    fs::write(&search_data_path, serde_json::to_string(&documents)?).context(format!(
        "Failed to write search data to {}",
        search_data_path.display()
    ))?;

    // Create search page
    let search_page = create_search_page(config)?;
    let search_page_path = config.output_dir.join("search.html");
    fs::write(&search_page_path, search_page).context(format!(
        "Failed to write search page to {}",
        search_page_path.display()
    ))?;

    info!("Search index generated successfully");

    Ok(())
}

/// Extract title from markdown content
fn extract_title(content: &str) -> Option<String> {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);

    let parser = Parser::new_ext(content, options);

    let mut title = None;
    let mut in_h1 = false;

    for event in parser {
        match event {
            Event::Start(Tag::Heading { level, .. })
                if level == pulldown_cmark::HeadingLevel::H1 =>
            {
                in_h1 = true;
            }
            Event::Text(text) if in_h1 => {
                title = Some(text.to_string());
                break;
            }
            Event::End(TagEnd::Heading(_)) if in_h1 => {
                in_h1 = false;
            }
            _ => {}
        }
    }

    title
}

/// Strip markdown to get plain text
fn strip_markdown(content: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);

    let parser = Parser::new_ext(content, options);

    let mut plain_text = String::new();
    let mut in_code_block = false;

    for event in parser {
        match event {
            Event::Text(text) => {
                if !in_code_block {
                    plain_text.push_str(&text);
                    plain_text.push(' ');
                }
            }
            Event::Start(Tag::CodeBlock(_)) => {
                in_code_block = true;
            }
            Event::End(TagEnd::CodeBlock) => {
                in_code_block = false;
            }
            Event::SoftBreak => {
                plain_text.push(' ');
            }
            Event::HardBreak => {
                plain_text.push('\n');
            }
            _ => {}
        }
    }

    plain_text
}

/// Create search page
fn create_search_page(config: &Config) -> Result<String> {
    let mut context = HashMap::new();
    context.insert("title", format!("{} - Search", config.title));

    let html = crate::template::render_search(config, &context)?;

    Ok(html)
}
