use std::borrow::Cow;

use lazy_static::lazy_static;
use regex::Regex;

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum FormatType {
    #[default]
    Html,
    Manpage,
}

// Define regex patterns at module level
lazy_static! {
    // Heading and anchor patterns
    static ref HEADING_ANCHOR_RE: Regex =
        Regex::new(r"^(#+\s+.+?)(?:\s+\{#([a-zA-Z0-9_-]+)\})\s*$").unwrap();
    static ref INLINE_ANCHOR_RE: Regex = Regex::new(r"\[\]\{#([a-zA-Z0-9_-]+)\}").unwrap();
    static ref AUTO_LINK_RE: Regex = Regex::new(r"\[\]\((#[a-zA-Z0-9_-]+)\)").unwrap();

    // Admonition patterns
    static ref ADMONITION_START_RE: Regex =
        Regex::new(r"^:::\s*\{\.([a-zA-Z]+)(?:\s+#([a-zA-Z0-9_-]+))?\}(.*)$").unwrap();
    static ref ADMONITION_END_RE: Regex = Regex::new(r"^(.*?):::$").unwrap();

    // Figure pattern
    static ref FIGURE_RE: Regex = Regex::new(
        r":::\s*\{\.figure(?:\s+#([a-zA-Z0-9_-]+))?\}\s*\n#\s+(.+)\n(!\[.*?\]\(.+?\))\s*:::"
    ).unwrap();

    // Role patterns
    static ref MANPAGE_WITH_SECTION: Regex = Regex::new(r"\{manpage\}`([^`(]+)\(([0-9]+)\)`").unwrap();
    static ref MANPAGE_NO_SECTION: Regex = Regex::new(r"\{manpage\}`([^`(]+)`").unwrap();
    static ref COMMAND_ROLE: Regex = Regex::new(r"\{command\}`([^`]+)`").unwrap();
    static ref ENV_ROLE: Regex = Regex::new(r"\{env\}`([^`]+)`").unwrap();
    static ref FILE_ROLE: Regex = Regex::new(r"\{file\}`([^`]+)`").unwrap();
    static ref OPTION_ROLE: Regex = Regex::new(r"\{option\}`([^`]+)`").unwrap();
    static ref VAR_ROLE: Regex = Regex::new(r"\{var\}`([^`]+)`").unwrap();

    // Terminal prompts
    static ref COMMAND_PROMPT_RE: Regex = Regex::new(r"`\s*\$\s+([^`]+)`").unwrap();
    static ref REPL_PROMPT_RE: Regex = Regex::new(r"`nix-repl>\s*([^`]+)`").unwrap();
}

pub fn preprocess_markdown(markdown: &str, format_type: FormatType) -> String {
    let preprocessed = preprocess_special_markup(markdown, format_type);

    match format_type {
        FormatType::Html => process_html_format(&preprocessed),
        FormatType::Manpage => process_manpage_format(&preprocessed),
    }
}

fn process_html_format(markdown: &str) -> String {
    let with_figures = process_figures(markdown);
    process_html_admonitions_and_definitions(&with_figures)
}

fn process_figures(markdown: &str) -> String {
    FIGURE_RE
        .replace_all(markdown, |caps: &regex::Captures| {
            let id_attr = caps
                .get(1)
                .map_or(String::new(), |id| format!(" id=\"{}\"", id.as_str()));
            let title = &caps[2];
            let image = &caps[3];
            format!("<figure{id_attr}>\n<figcaption>{title}</figcaption>\n{image}\n</figure>")
        })
        .into_owned()
}

fn process_html_admonitions_and_definitions(markdown: &str) -> String {
    let mut processed_lines = Vec::new();
    let mut lines = markdown.lines().peekable();
    let mut in_admonition = false;

    while let Some(line) = lines.next() {
        if !line.is_empty()
            && !line.starts_with(':')
            && process_definition_list(line, &mut lines, &mut processed_lines)
        {
            continue;
        }

        if let Some(start_caps) = ADMONITION_START_RE.captures(line) {
            if process_admonition_start(
                &start_caps,
                &mut processed_lines,
                &mut in_admonition,
                FormatType::Html,
            ) {
                continue;
            }
        }

        if in_admonition && ADMONITION_END_RE.is_match(line) {
            process_admonition_end(line, &mut processed_lines, &mut in_admonition);
            continue;
        }

        processed_lines.push(process_line_markup(line, in_admonition, FormatType::Html));
    }

    if in_admonition {
        processed_lines.push("</div>".to_string());
    }

    processed_lines.join("\n")
}

fn process_definition_list<'a>(
    line: &str,
    lines: &mut std::iter::Peekable<impl Iterator<Item = &'a str>>,
    output: &mut Vec<String>,
) -> bool {
    if let Some(next_line) = lines.peek() {
        if next_line.starts_with(":   ") {
            let term = line;
            let def_line = lines.next().unwrap();
            let definition = &def_line[4..];

            output.push(format!(
                "<dl>\n<dt>{term}</dt>\n<dd>{definition}</dd>\n</dl>"
            ));
            return true;
        }
    }
    false
}

fn process_admonition_start(
    caps: &regex::Captures,
    output: &mut Vec<String>,
    in_admonition: &mut bool,
    format_type: FormatType,
) -> bool {
    let admonition_type = &caps[1];
    let content_part = caps.get(3).map_or("", |m| m.as_str());

    if format_type == FormatType::Html {
        let id_attr = caps
            .get(2)
            .map_or(String::new(), |id| format!(" id=\"{}\"", id.as_str()));
        let title = capitalize_first(admonition_type);

        // Check for single-line admonition
        if let Some(end_cap) = ADMONITION_END_RE.captures(content_part) {
            let content = end_cap.get(1).map_or("", |m| m.as_str()).trim();
            let mut admon = format!(
                "<div class=\"admonition {admonition_type}\"{id_attr}>\n<p class=\"admonition-title\">{title}</p>"
            );

            if !content.is_empty() {
                admon.push_str(&format!("\n<p>{content}</p>"));
            }

            admon.push_str("\n</div>");
            output.push(admon);
        } else {
            // Multi-line admonition start
            let mut admon = format!(
                "<div class=\"admonition {admonition_type}\"{id_attr}>\n<p class=\"admonition-title\">{title}</p>"
            );

            if !content_part.trim().is_empty() {
                admon.push_str(&format!("\n<p>{}</p>", content_part.trim()));
            }

            output.push(admon);
            *in_admonition = true;
        }
        return true;
    }
    false
}

fn process_admonition_end(line: &str, output: &mut Vec<String>, in_admonition: &mut bool) {
    let content_before_end = ADMONITION_END_RE
        .captures(line)
        .and_then(|c| c.get(1))
        .map_or("", |m| m.as_str())
        .trim();

    if !content_before_end.is_empty() {
        output.push(format!("<p>{content_before_end}</p>"));
    }

    output.push("</div>".to_string());
    *in_admonition = false;
}

fn process_manpage_format(markdown: &str) -> String {
    let mut processed_lines = Vec::new();
    let mut lines = markdown.lines().peekable();

    while let Some(line) = lines.next() {
        if !line.is_empty() && !line.starts_with(':') {
            if let Some(next_line) = lines.peek() {
                if next_line.starts_with(":   ") {
                    let term = line;
                    let def_line = lines.next().unwrap();
                    let definition = &def_line[4..];

                    processed_lines.push(format!(
                        ".PP\n.RS\n.TP\n{}\n{}\n.RE",
                        term.trim(),
                        definition.trim()
                    ));
                    continue;
                }
            }
        }

        // Process admonitions
        if let Some(start_caps) = ADMONITION_START_RE.captures(line) {
            let adm_type = &start_caps[1];
            let content_part = start_caps.get(3).map_or("", |m| m.as_str());
            let title = capitalize_first(adm_type);

            // Single-line admonition
            if let Some(end_cap) = ADMONITION_END_RE.captures(content_part) {
                let content = end_cap.get(1).map_or("", |m| m.as_str()).trim();
                processed_lines.push(".PP".to_string());
                processed_lines.push(format!("\\fB{title}:\\fR {content}"));
                continue;
            }

            // Multi-line admonition
            let paragraphs = collect_admonition_paragraphs(&mut lines, content_part, adm_type);

            // Output paragraphs with proper formatting
            processed_lines.push(".PP".to_string());
            if paragraphs.is_empty() {
                processed_lines.push(format!("\\fB{title}:\\fR"));
            } else {
                // First paragraph gets the title
                processed_lines.push(format!("\\fB{}:\\fR {}", title, paragraphs[0]));

                // Subsequent paragraphs get their own .PP directive
                for para in paragraphs.iter().skip(1) {
                    processed_lines.push(".PP".to_string());
                    processed_lines.push(para.clone());
                }
            }
            continue;
        }

        processed_lines.push(process_line_markup(line, false, FormatType::Manpage));
    }

    processed_lines.join("\n")
}

fn collect_admonition_paragraphs<'a, I>(
    lines: &mut I,
    initial_content: &str,
    adm_type: &str,
) -> Vec<String>
where
    I: Iterator<Item = &'a str>,
{
    let mut adm_content = Vec::new();

    // Add initial content if present
    if !initial_content.trim().is_empty() {
        adm_content.push(initial_content.trim().to_string());
    }

    // Collect all lines until the end marker
    for next_line in lines.by_ref() {
        if let Some(end_caps) = ADMONITION_END_RE.captures(next_line) {
            let content_before_end = end_caps.get(1).map_or("", |m| m.as_str()).trim();
            if !content_before_end.is_empty() {
                adm_content.push(content_before_end.to_string());
            }
            break;
        } else {
            adm_content.push(next_line.to_string());
        }
    }

    // Process content into paragraphs
    let mut paragraphs = Vec::new();
    let mut current_paragraph = Vec::new();

    for content_line in adm_content {
        if content_line.trim().is_empty() {
            // Empty line means paragraph break
            if !current_paragraph.is_empty() {
                paragraphs.push(current_paragraph.join(" "));
                current_paragraph = Vec::new();
            }
        } else {
            // Clean up header-like content in example admonitions
            let cleaned_line = if adm_type == "example" && content_line.trim().starts_with('#') {
                content_line.trim_start_matches('#').trim().to_string()
            } else {
                content_line.trim().to_string()
            };
            current_paragraph.push(cleaned_line);
        }
    }

    // Add the last paragraph if it exists
    if !current_paragraph.is_empty() {
        paragraphs.push(current_paragraph.join(" "));
    }

    paragraphs
}

fn process_line_markup(line: &str, in_admonition: bool, format_type: FormatType) -> String {
    let mut processed = Cow::from(line);

    match format_type {
        FormatType::Html => {
            // Process heading anchors
            if HEADING_ANCHOR_RE.is_match(&processed) {
                processed = Cow::Owned(
                    HEADING_ANCHOR_RE
                        .replace_all(&processed, |caps: &regex::Captures| {
                            format!("{} <!-- anchor: {} -->", &caps[1], &caps[2])
                        })
                        .into_owned(),
                );
            }

            // Process inline anchors
            if INLINE_ANCHOR_RE.is_match(&processed) {
                processed = Cow::Owned(
                    INLINE_ANCHOR_RE
                        .replace_all(&processed, |caps: &regex::Captures| {
                            format!("<span id=\"{}\"></span>", &caps[1])
                        })
                        .into_owned(),
                );
            }

            // Process auto links
            if AUTO_LINK_RE.is_match(&processed) {
                processed = Cow::Owned(
                    AUTO_LINK_RE
                        .replace_all(&processed, |caps: &regex::Captures| {
                            let anchor = &caps[1];
                            format!("[{}]({})", anchor.trim_start_matches('#'), anchor)
                        })
                        .into_owned(),
                );
            }

            // Add paragraph tags for plain text in admonitions
            if in_admonition
                && !processed.trim().is_empty()
                && !processed.trim_start().starts_with('<')
                && !processed.trim_start().starts_with('#')
            {
                format!("<p>{}</p>", processed.into_owned().trim())
            } else {
                processed.into_owned()
            }
        }
        FormatType::Manpage => {
            // Process heading anchors
            if HEADING_ANCHOR_RE.is_match(&processed) {
                processed = Cow::Owned(
                    HEADING_ANCHOR_RE
                        .replace_all(&processed, |caps: &regex::Captures| caps[1].to_string())
                        .into_owned(),
                );
            }

            // Process inline anchors
            if INLINE_ANCHOR_RE.is_match(&processed) {
                processed = Cow::Owned(
                    INLINE_ANCHOR_RE
                        .replace_all(&processed, |caps: &regex::Captures| {
                            format!("[{}]", &caps[1])
                        })
                        .into_owned(),
                );
            }

            // Process auto links
            if AUTO_LINK_RE.is_match(&processed) {
                processed = Cow::Owned(
                    AUTO_LINK_RE
                        .replace_all(&processed, |caps: &regex::Captures| {
                            caps[1].trim_start_matches('#').to_string()
                        })
                        .into_owned(),
                );
            }

            processed.into_owned()
        }
    }
}

fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().chain(chars).collect(),
    }
}

pub fn preprocess_special_markup(markdown: &str, format_type: FormatType) -> String {
    match format_type {
        FormatType::Html => process_html_special_markup(markdown),
        FormatType::Manpage => process_manpage_special_markup(markdown),
    }
}

fn process_html_special_markup(markdown: &str) -> String {
    let mut result = markdown.to_string();

    // Process role markups
    let markup_pattern = Regex::new(r"\{(command|env|file|option|var|manpage)\}`([^`]+)`").unwrap();
    result = markup_pattern
        .replace_all(&result, |caps: &regex::Captures| {
            format!("<span class=\"{}-markup\">{}</span>", &caps[1], &caps[2])
        })
        .into_owned();

    // Process command prompts
    result = COMMAND_PROMPT_RE
        .replace_all(&result, |caps: &regex::Captures| {
            format!(
                "<code class=\"terminal\"><span class=\"prompt\">$</span> {}</code>",
                &caps[1]
            )
        })
        .into_owned();

    // Process REPL prompts
    result = REPL_PROMPT_RE
        .replace_all(&result, |caps: &regex::Captures| {
            format!(
                "<code class=\"nix-repl\"><span class=\"prompt\">nix-repl&gt;</span> {}</code>",
                &caps[1]
            )
        })
        .into_owned();

    result
}

fn process_manpage_special_markup(markdown: &str) -> String {
    let mut result = markdown.to_string();

    // Process manpage references with section
    result = MANPAGE_WITH_SECTION
        .replace_all(&result, |caps: &regex::Captures| {
            format!("\\fB{}\\fR({})", &caps[1], &caps[2])
        })
        .into_owned();

    // Process manpage references without section
    result = MANPAGE_NO_SECTION
        .replace_all(&result, |caps: &regex::Captures| {
            format!("\\fB{}\\fR", &caps[1])
        })
        .into_owned();

    // Process role markups with a single structure
    let roles = [
        (&*COMMAND_ROLE, "\\fB", "\\fR"),
        (&*ENV_ROLE, "\\fI", "\\fR"),
        (&*FILE_ROLE, "\\fI", "\\fR"),
        (&*OPTION_ROLE, "\\fB", "\\fR"),
        (&*VAR_ROLE, "\\fI", "\\fR"),
    ];

    for (regex, prefix, suffix) in roles {
        result = regex
            .replace_all(&result, |caps: &regex::Captures| {
                format!("{}{}{}", prefix, &caps[1], suffix)
            })
            .into_owned();
    }

    // Process command prompts
    result = COMMAND_PROMPT_RE
        .replace_all(&result, |caps: &regex::Captures| {
            format!("\\fB$\\fR {}", &caps[1])
        })
        .into_owned();

    // Process REPL prompts
    result = REPL_PROMPT_RE
        .replace_all(&result, |caps: &regex::Captures| {
            format!("\\fBnix-repl>\\fR {}", &caps[1])
        })
        .into_owned();

    result
}
