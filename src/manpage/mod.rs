use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::Path;

use anyhow::{Context, Result};
use lazy_static::lazy_static;
use log::{debug, info};
use pulldown_cmark::{Alignment, Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use regex::Regex;

use crate::formatter::markdown::{collect_markdown_files, extract_headers};

/// Represents a list with nesting information
#[derive(Clone, Debug)]
struct List {
    width: usize,
    next_idx: Option<usize>,
    compact: bool,
    first_item_seen: bool,
    level: usize,
}

impl List {
    const fn new(width: usize, next_idx: Option<usize>, compact: bool, level: usize) -> Self {
        Self {
            width,
            next_idx,
            compact,
            first_item_seen: false,
            level,
        }
    }
}

// Define regex patterns for preprocessing
lazy_static! {
    static ref ROLE_PATTERN: Regex = Regex::new(r"\{([a-z]+)\}`([^`]+)`").unwrap();
    static ref ADMONITION_START_RE: Regex =
        Regex::new(r"^:::\s*\{\.([a-zA-Z]+)(?:\s+#([a-zA-Z0-9_-]+))?\}(.*)$").unwrap();
    static ref ADMONITION_END_RE: Regex = Regex::new(r"^(.*?):::$").unwrap();
    static ref COMMAND_PROMPT: Regex = Regex::new(r"`\s*\$\s+([^`]+)`").unwrap();
    static ref REPL_PROMPT: Regex = Regex::new(r"`nix-repl>\s*([^`]+)`").unwrap();
    static ref HEADING_ANCHOR: Regex =
        Regex::new(r"^(?:(#+)?\s*)?(.+?)(?:\s+\{#([a-zA-Z0-9_-]+)\})\s*$").unwrap();
    static ref INLINE_ANCHOR: Regex = Regex::new(r"\[\]\{#([a-zA-Z0-9_-]+)\}").unwrap();
    static ref LIST_ITEM_WITH_ANCHOR_RE: Regex =
        Regex::new(r"^(\s*[-*+]|\s*\d+\.)\s+\[\]\{#([a-zA-Z0-9_-]+)\}(.*)$").unwrap();
    static ref AUTO_EMPTY_LINK_RE: Regex = Regex::new(r"\[\]\((#[a-zA-Z0-9_-]+)\)").unwrap();
    static ref AUTO_SECTION_LINK_RE: Regex =
        Regex::new(r"\[([^\]]+)\]\((#[a-zA-Z0-9_-]+)\)").unwrap();
    static ref BACKSLASH_ESCAPE: Regex = Regex::new(r"\\ef([A-Z])").unwrap();
}

/// Map of characters that need to be escaped in manpages
fn get_roff_escapes() -> HashMap<char, &'static str> {
    let mut map = HashMap::new();
    map.insert('"', "\\(dq");
    map.insert('\'', "\\(aq");
    map.insert('-', "\\-");
    map.insert('.', "\\&.");
    map.insert('\\', "\\\\");
    map.insert('^', "\\(ha");
    map.insert('`', "\\(ga");
    map.insert('~', "\\(ti");
    map
}

/// Escapes a string for use in manpages
fn man_escape(s: &str) -> String {
    let escapes = get_roff_escapes();
    let mut result = String::with_capacity(s.len() * 2);

    for c in s.chars() {
        if let Some(escape) = escapes.get(&c) {
            result.push_str(escape);
        } else {
            // Don't try to encode ASCII as unicode escapes
            result.push(c);
        }
    }

    result
}

/// Escape a leading dot to prevent it from being interpreted as a troff command
fn escape_leading_dot(text: &str) -> String {
    if text.starts_with('.') || text.starts_with('\'') {
        format!("\\&{text}")
    } else {
        text.to_string()
    }
}

/// Generate man pages from markdown files in a directory
pub fn generate_manpages_from_directory(
    input_dir: &Path,
    output_dir: &Path,
    section: u8,
    manual: &str,
    _jobs: Option<usize>,
) -> Result<()> {
    fs::create_dir_all(output_dir)?;

    info!("Collecting markdown files from {}", input_dir.display());
    let files = collect_markdown_files(input_dir)?;
    info!(
        "Found {} markdown files to convert to manpages",
        files.len()
    );

    for file_path in &files {
        process_markdown_to_manpage(file_path, input_dir, output_dir, section, manual)?;
    }

    Ok(())
}

/// Process a single markdown file to a man page
fn process_markdown_to_manpage(
    file_path: &Path,
    input_dir: &Path,
    output_dir: &Path,
    section: u8,
    manual: &str,
) -> Result<()> {
    debug!("Converting to manpage: {}", file_path.display());

    let content = fs::read_to_string(file_path)
        .with_context(|| format!("Failed to read markdown file: {}", file_path.display()))?;

    let (_, title_opt) = extract_headers(&content);
    let filename = file_path.file_stem().unwrap_or_default().to_string_lossy();
    let title = title_opt.unwrap_or_else(|| filename.to_string());

    let rel_path = file_path.strip_prefix(input_dir).with_context(|| {
        format!(
            "Failed to determine relative path for {}",
            file_path.display()
        )
    })?;

    let mut output_path = output_dir.join(rel_path);
    output_path.set_extension(section.to_string());

    if let Some(parent) = output_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }

    let manpage_content = markdown_to_manpage(&content, &title, section, manual)?;

    fs::write(&output_path, manpage_content)
        .with_context(|| format!("Failed to write manpage file: {}", output_path.display()))?;

    info!("Generated manpage: {}", output_path.display());

    Ok(())
}

/// Convert markdown to manpage format
pub fn markdown_to_manpage(
    markdown: &str,
    title: &str,
    section: u8,
    manual: &str,
) -> Result<String> {
    // Preprocess markdown to handle role-based markup and other special syntax
    let preprocessed = preprocess_content(markdown);

    // Initialize state
    let manpage_urls = HashMap::new();
    let href_targets = HashMap::new();

    let mut state = ManpageRendererState {
        do_parbreak_stack: vec![false],
        font_stack: vec!["R".to_string()],
        list_stack: Vec::new(),
        link_stack: Vec::new(),
        href_targets,
        manpage_urls,
        link_footnotes: None,
        inline_code_is_quoted: true,
        table_alignments: Vec::new(),
        in_table_header: false,
        table_row_count: 0,
        current_list_level: 0,
        in_admonition: false,
        admonition_content: String::new(),
    };

    let mut output = Vec::new();

    // Write manpage header
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    writeln!(output, ".\\\" Generated by ndg")?;
    writeln!(
        output,
        ".TH \"{}\" \"{}\" \"{}\" \"\" \"{}\"",
        man_escape(title),
        section,
        today,
        man_escape(manual)
    )?;

    // Set up parser
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);

    let parser = Parser::new_ext(&preprocessed, options);

    // Render the manpage content
    for event in parser {
        process_event(&event, &mut output, &mut state)?;
    }

    let result = String::from_utf8(output)?;
    Ok(fix_formatting(&result))
}

fn fix_formatting(content: &str) -> String {
    // Fix problematic escape sequences and formatting issues
    let result = BACKSLASH_ESCAPE.replace_all(content, "\\f$1").to_string();

    result
        .replace("\\fR", "\\fP")
        .replace("\\fP\\fP", "\\fP")
        .replace(".sp\n.sp", ".sp")
        .replace("\\&\\&", "\\&")
        .replace(" \\&.", "\\&.")
        .replace("\\e", "\\\\")
        .replace(".TI 0\n", ".TI 0\n")
}

struct ManpageRendererState {
    do_parbreak_stack: Vec<bool>,
    font_stack: Vec<String>,
    list_stack: Vec<List>,
    link_stack: Vec<String>,
    href_targets: HashMap<String, String>,
    manpage_urls: HashMap<String, String>,
    link_footnotes: Option<Vec<String>>,
    inline_code_is_quoted: bool,
    table_alignments: Vec<String>,
    in_table_header: bool,
    table_row_count: usize,
    current_list_level: usize,
    in_admonition: bool,
    admonition_content: String,
}

impl ManpageRendererState {
    fn enter_block(&mut self) {
        self.do_parbreak_stack.push(false);
    }

    fn leave_block(&mut self) {
        self.do_parbreak_stack.pop();
        if let Some(last) = self.do_parbreak_stack.last_mut() {
            *last = true;
        }
    }

    fn maybe_parbreak(&mut self, suffix: &str) -> String {
        let result = if self.do_parbreak_stack.last().copied().unwrap_or(false) {
            format!(".sp{suffix}")
        } else {
            String::new()
        };

        if let Some(last) = self.do_parbreak_stack.last_mut() {
            *last = true;
        }

        result
    }

    fn push_font(&mut self, font: &str) -> String {
        self.font_stack.push(font.to_string());
        format!("\\f{font}")
    }

    fn pop_font(&mut self) -> String {
        self.font_stack.pop();
        let default = "P".to_string();
        let prev = match self.font_stack.last() {
            Some(font) => font,
            None => &default,
        };
        format!("\\f{prev}")
    }

    fn start_list_item(&mut self, ordered: bool) -> String {
        let mut output = String::new();

        let list = match self.list_stack.last_mut() {
            Some(list) => list,
            None => return output, // Should not happen
        };

        if list.first_item_seen && !list.compact {
            output.push_str(".sp\n");
        }
        list.first_item_seen = true;

        let marker = if ordered {
            if let Some(idx) = list.next_idx {
                let marker = format!("{idx}.");
                list.next_idx = Some(idx + 1);
                marker
            } else {
                "1.".to_string()
            }
        } else {
            "\\(bu".to_string()
        };

        output.push_str(&format!(".IP {} {}\n", marker, list.width));
        output
    }

    fn process_table_alignments(&mut self, alignments: &[Alignment]) {
        self.table_alignments = alignments
            .iter()
            .map(|a| match a {
                Alignment::Left => "l".to_string(),
                Alignment::Center => "c".to_string(),
                Alignment::Right => "r".to_string(),
                _ => "c".to_string(), // Default to center for unknown alignments
            })
            .collect();
    }

    fn get_table_format(&self) -> String {
        if self.table_alignments.is_empty() {
            "c c c c c c c c c".to_string() // Default alignment
        } else {
            self.table_alignments.join(" ")
        }
    }

    fn format_admonition(&self, type_name: &str, content: &str) -> String {
        let title = match type_name.to_lowercase().as_str() {
            "note" => "Note".to_string(),
            "warning" => "Warning".to_string(),
            "tip" => "Tip".to_string(),
            "info" => "Info".to_string(),
            "figure" => "Figure".to_string(),
            _ => type_name.to_string(),
        };

        let mut result = String::new();
        result.push_str(".sp\n");
        result.push_str(".RS 4\n");
        result.push_str(&format!("\\fB{title}:\\fP "));

        // Split content by lines and properly indent
        let lines: Vec<&str> = content.lines().collect();
        if !lines.is_empty() {
            result.push_str(lines[0]);
            result.push('\n');

            for line in &lines[1..] {
                result.push_str(&format!("{}\n", line.trim()));
            }
        }

        result.push_str(".RE\n");
        result
    }
}

/// Preprocess markdown to handle role-based markup and custom syntax
fn preprocess_content(content: &str) -> String {
    let mut result = String::new();
    let lines = content.lines().peekable();
    let mut in_admonition = false;
    let mut admonition_content = String::new();
    let mut admonition_type = String::new();

    for line in lines {
        // Process headings with explicit anchors
        if let Some(caps) = HEADING_ANCHOR.captures(line) {
            let level_signs = caps.get(1).map_or("", |m| m.as_str()); // Get level signs if present
            let text = caps.get(2).unwrap().as_str();
            let id = caps.get(3).map(|m| m.as_str());

            // If no level signs but has anchor, treat as h2
            let level_str = if level_signs.is_empty() {
                "## "
            } else {
                level_signs
            };

            let formatted_line = if let Some(id_str) = id {
                format!("{level_str} {text} <!-- anchor: {id_str} -->")
            } else {
                format!("{level_str} {text}")
            };

            result.push_str(&formatted_line);
            result.push('\n');
            continue;
        }

        // Process list items with anchors
        if let Some(caps) = LIST_ITEM_WITH_ANCHOR_RE.captures(line) {
            let list_marker = caps.get(1).unwrap().as_str();
            let anchor = caps.get(2).unwrap().as_str();
            let content = caps.get(3).unwrap().as_str();

            result.push_str(&format!(
                "{list_marker} <!-- nixos-anchor-id:{anchor} -->{content}\n"
            ));
            continue;
        }

        // Process admonitions
        if let Some(caps) = ADMONITION_START_RE.captures(line) {
            in_admonition = true;
            admonition_type = caps.get(1).unwrap().as_str().to_string();
            admonition_content.clear();

            // Handle single-line admonitions
            let content_part = caps.get(3).map_or("", |m| m.as_str());
            if let Some(end_caps) = ADMONITION_END_RE.captures(content_part) {
                let content = end_caps.get(1).map_or("", |m| m.as_str()).trim();

                // Format the admonition - this will be processed properly during event handling
                result.push_str(&format!(
                    ".ADMONITION_START {admonition_type} {content}\n"
                ));
                result.push_str(".ADMONITION_END\n");

                in_admonition = false;
                continue;
            }

            if !content_part.trim().is_empty() {
                admonition_content.push_str(content_part);
                admonition_content.push('\n');
            }

            continue;
        }

        if in_admonition {
            if let Some(end_caps) = ADMONITION_END_RE.captures(line) {
                let content_before_end = end_caps.get(1).map_or("", |m| m.as_str()).trim();
                if !content_before_end.is_empty() {
                    admonition_content.push_str(content_before_end);
                    admonition_content.push('\n');
                }

                // Format the admonition - this will be processed properly during event handling
                result.push_str(&format!(
                    ".ADMONITION_START {} {}\n",
                    admonition_type,
                    admonition_content.trim()
                ));
                result.push_str(".ADMONITION_END\n");

                in_admonition = false;
                continue;
            }

            admonition_content.push_str(line);
            admonition_content.push('\n');
            continue;
        }

        // Process role-based markup
        let processed = ROLE_PATTERN.replace_all(line, |caps: &regex::Captures| {
            let role_type = &caps[1];
            let content = &caps[2];

            match role_type {
                "command" => format!("\\fB{content}\\fP"),
                "env" => format!("\\fI{content}\\fP"),
                "file" => format!("\\fI{content}\\fP"),
                "option" => format!("\\fB{content}\\fP"),
                "var" => format!("\\fI{content}\\fP"),
                "manpage" => {
                    if let Some((page, section)) = content.rsplit_once('(') {
                        let page = page.trim();
                        let section = section.trim_end_matches(')');
                        format!("\\fB{page}\\fP({section})")
                    } else {
                        format!("\\fB{content}\\fP")
                    }
                }
                _ => format!("\\fI{content}\\fP"),
            }
        });

        // Process command prompts
        let processed = COMMAND_PROMPT.replace_all(&processed, |caps: &regex::Captures| {
            let command = &caps[1];
            format!("$ \\fB{command}\\fP")
        });

        // Process REPL prompts
        let processed = REPL_PROMPT.replace_all(&processed, |caps: &regex::Captures| {
            let expr = &caps[1];
            format!("nix-repl> \\fB{expr}\\fP")
        });

        // Process inline anchors (remove them)
        let processed = INLINE_ANCHOR.replace_all(&processed, "");

        // Process auto links
        let processed = AUTO_EMPTY_LINK_RE.replace_all(&processed, |caps: &regex::Captures| {
            let anchor = &caps[1];
            format!("[{anchor}]")
        });

        let processed = AUTO_SECTION_LINK_RE.replace_all(&processed, |caps: &regex::Captures| {
            let text = &caps[1];
            let anchor = &caps[2];
            format!("{text} [{anchor}]")
        });

        result.push_str(&processed);
        result.push('\n');
    }

    result
}

/// Process a single markdown event and write appropriate man page content
fn process_event(
    event: &Event,
    output: &mut Vec<u8>,
    state: &mut ManpageRendererState,
) -> Result<()> {
    match event {
        Event::Start(Tag::Heading { level, .. }) => {
            if !state.do_parbreak_stack.is_empty() {
                state.do_parbreak_stack[0] = false;
            }

            let section_macro = match level {
                HeadingLevel::H1 => ".SH",
                _ => ".SS",
            };
            write!(output, "{section_macro} \"")?;
        }

        Event::End(TagEnd::Heading(_)) => {
            writeln!(output, "\"")?;
        }

        Event::Start(Tag::Paragraph) => {
            if state.in_admonition {
                state.admonition_content.push('\n');
            } else {
                writeln!(output, "{}", state.maybe_parbreak(""))?;
            }
        }

        Event::End(TagEnd::Paragraph) => {
            if !state.in_admonition {
                writeln!(output)?;
            }
        }

        Event::Start(Tag::List(start_num)) => {
            let next_idx = start_num.as_ref().map(|&n| n as usize);
            // Use 4 for both numbered and bullet lists for consistency
            let width = 4;

            state.current_list_level += 1;
            state
                .list_stack
                .push(List::new(width, next_idx, false, state.current_list_level));

            writeln!(output, "{}", state.maybe_parbreak(""))?;
            writeln!(output, ".RS")?;
        }

        Event::End(TagEnd::List(_)) => {
            state.list_stack.pop();
            state.current_list_level = state.current_list_level.saturating_sub(1);
            writeln!(output, ".RE")?;
        }

        Event::Start(Tag::Item) => {
            state.enter_block();

            let is_ordered = state
                .list_stack
                .last()
                .is_some_and(|list| list.next_idx.is_some());

            let list_item_markup = state.start_list_item(is_ordered);
            write!(output, "{list_item_markup}")?;
        }

        Event::End(TagEnd::Item) => {
            state.leave_block();
            writeln!(output)?;
        }

        Event::Start(Tag::CodeBlock(_)) => {
            writeln!(output, ".sp")?;
            writeln!(output, ".RS 4")?;
            writeln!(output, ".nf")?;
        }

        Event::End(TagEnd::CodeBlock) => {
            writeln!(output, ".fi")?;
            writeln!(output, ".RE")?;
        }

        Event::Start(Tag::Emphasis) => {
            if state.in_admonition {
                state.admonition_content.push_str("\\fI");
            } else {
                write!(output, "{}", state.push_font("I"))?;
            }
        }

        Event::End(TagEnd::Emphasis) => {
            if state.in_admonition {
                state.admonition_content.push_str("\\fP");
            } else {
                write!(output, "{}", state.pop_font())?;
            }
        }

        Event::Start(Tag::Strong) => {
            if state.in_admonition {
                state.admonition_content.push_str("\\fB");
            } else {
                write!(output, "{}", state.push_font("B"))?;
            }
        }

        Event::End(TagEnd::Strong) => {
            if state.in_admonition {
                state.admonition_content.push_str("\\fP");
            } else {
                write!(output, "{}", state.pop_font())?;
            }
        }

        Event::Text(text) => {
            let text_str = text.as_ref();

            // Handle special admonition markers
            if text_str.starts_with(".ADMONITION_START ") {
                state.in_admonition = true;
                state.admonition_content.clear();

                let parts: Vec<&str> = text_str.splitn(3, ' ').collect();
                if parts.len() >= 3 {
                    let admonition_type = parts[1];
                    let content = parts[2];

                    let formatted = state.format_admonition(admonition_type, content);
                    write!(output, "{formatted}")?;
                }
                return Ok(());
            } else if text_str == ".ADMONITION_END" {
                state.in_admonition = false;
                return Ok(());
            }

            // Handle normal text
            if state.in_admonition {
                state.admonition_content.push_str(&man_escape(text_str));
            } else if text_str.starts_with('.') {
                write!(output, "{}", escape_leading_dot(text_str))?;
            } else {
                write!(output, "{}", man_escape(text_str))?;
            }
        }

        Event::Code(code) => {
            let code_str = code.as_ref();
            if state.in_admonition {
                state
                    .admonition_content
                    .push_str(&format!("\\fB{}\\fP", man_escape(code_str)));
            } else if state.inline_code_is_quoted {
                write!(output, "\\fB{}\\fP", man_escape(code_str))?;
            } else {
                write!(output, "{}", man_escape(code_str))?;
            }
        }

        Event::SoftBreak => {
            if state.in_admonition {
                state.admonition_content.push(' ');
            } else {
                write!(output, " ")?;
            }
        }

        Event::HardBreak => {
            if state.in_admonition {
                state.admonition_content.push('\n');
            } else {
                writeln!(output, ".br")?;
            }
        }

        Event::Start(Tag::Link { .. }) => {
            // For link start, just record that we're in a link
            state.link_stack.push("link".to_string());
        }

        Event::End(TagEnd::Link) => {
            // For link end, just clean up the stack
            state.link_stack.pop();
        }

        Event::Start(Tag::Table(alignments)) => {
            state.process_table_alignments(alignments);
            state.in_table_header = true;
            state.table_row_count = 0;

            writeln!(output, ".PP")?;
            writeln!(output, ".TS")?;
            writeln!(output, "allbox tab(|);")?;
            writeln!(output, "{}.", state.get_table_format())?;
        }

        Event::End(TagEnd::Table) => {
            writeln!(output, ".TE")?;
            state.in_table_header = false;
            state.table_row_count = 0;
            state.table_alignments.clear();
        }

        Event::Start(Tag::TableRow) => {
            state.table_row_count += 1;
            if state.table_row_count > 1 {
                state.in_table_header = false;
            }
        }

        Event::End(TagEnd::TableRow) => {
            writeln!(output)?;
        }

        Event::Start(Tag::TableCell) => {
            // If we're in the header, we might want to format differently
            if state.in_table_header {
                write!(output, "{}", state.push_font("B"))?;
            }
        }

        Event::End(TagEnd::TableCell) => {
            if state.in_table_header {
                write!(output, "{}", state.pop_font())?;
            }
            write!(output, "|")?;
        }

        // Handle any other events (images, comments, etc.)
        _ => {}
    }

    Ok(())
}
