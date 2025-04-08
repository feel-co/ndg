use std::borrow::Cow;

use lazy_static::lazy_static;
use regex::Regex;

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum FormatType {
    #[default]
    Html,
    Manpage,
}

pub fn preprocess_markdown(markdown: &str, format_type: FormatType) -> String {
    let mut result = preprocess_special_markup(markdown, format_type);

    lazy_static! {
        static ref HEADING_ANCHOR_RE: Regex =
            Regex::new(r"^(#+\s+.+?)(?:\s+\{#([a-zA-Z0-9_-]+)\})\s*$").unwrap();
        static ref INLINE_ANCHOR_RE: Regex = Regex::new(r"\[\]\{#([a-zA-Z0-9_-]+)\}").unwrap();
        static ref AUTO_LINK_RE: Regex = Regex::new(r"\[\]\((#[a-zA-Z0-9_-]+)\)").unwrap();
        static ref ADMONITION_START_RE: Regex =
            Regex::new(r"^:::\s*\{\.([a-zA-Z]+)(?:\s+#([a-zA-Z0-9_-]+))?\}(.*)$").unwrap();
        static ref ADMONITION_END_RE: Regex = Regex::new(r"^(.*?):::$").unwrap();
        static ref FIGURE_RE: Regex = Regex::new(
            r":::\s*\{\.figure(?:\s+#([a-zA-Z0-9_-]+))?\}\s*\n#\s+(.+)\n(!\[.*?\]\(.+?\))\s*:::"
        )
        .unwrap();
    }

    match format_type {
        FormatType::Html => {
            result = FIGURE_RE
                .replace_all(&result, |caps: &regex::Captures| {
                    let id_attr = caps
                        .get(1)
                        .map_or(String::new(), |id| format!(" id=\"{}\"", id.as_str()));
                    let title = &caps[2];
                    let image = &caps[3];
                    format!(
                        "<figure{id_attr}>\n<figcaption>{title}</figcaption>\n{image}\n</figure>"
                    )
                })
                .into_owned();

            let mut processed_lines = Vec::new();
            let mut lines = result.lines().peekable();
            let mut in_admonition = false;

            while let Some(line) = lines.next() {
                if !line.is_empty() && !line.starts_with(':') {
                    if let Some(next_line) = lines.peek() {
                        if next_line.starts_with(":   ") {
                            let term = line;
                            let def_line = lines.next().unwrap();
                            let definition = &def_line[4..];

                            processed_lines.push(format!(
                                "<dl>\n<dt>{term}</dt>\n<dd>{definition}</dd>\n</dl>"
                            ));
                            continue;
                        }
                    }
                }

                if let Some(start_caps) = ADMONITION_START_RE.captures(line) {
                    let admonition_type = &start_caps[1];
                    let id_attr = start_caps
                        .get(2)
                        .map_or(String::new(), |id| format!(" id=\"{}\"", id.as_str()));
                    let content_part = start_caps.get(3).map_or("", |m| m.as_str());

                    let title = admonition_type.chars().next().map_or_else(
                        || admonition_type.to_string(),
                        |c| c.to_uppercase().collect::<String>() + &admonition_type[1..],
                    );

                    if let Some(end_cap) = ADMONITION_END_RE.captures(content_part) {
                        let content = end_cap.get(1).map_or("", |m| m.as_str()).trim();

                        let mut admon = format!(
                            "<div class=\"admonition {admonition_type}\"{id_attr}>\n<p class=\"admonition-title\">{title}</p>"
                        );

                        if !content.is_empty() {
                            admon.push_str(&format!("\n<p>{content}</p>"));
                        }

                        admon.push_str("\n</div>");
                        processed_lines.push(admon);
                    } else {
                        let mut admon = format!(
                            "<div class=\"admonition {admonition_type}\"{id_attr}>\n<p class=\"admonition-title\">{title}</p>"
                        );

                        if !content_part.trim().is_empty() {
                            admon.push_str(&format!("\n<p>{}</p>", content_part.trim()));
                        }

                        processed_lines.push(admon);
                        in_admonition = true;
                    }
                    continue;
                }

                if in_admonition && ADMONITION_END_RE.is_match(line) {
                    let content_before_end = ADMONITION_END_RE
                        .captures(line)
                        .and_then(|c| c.get(1))
                        .map_or("", |m| m.as_str())
                        .trim();

                    if !content_before_end.is_empty() {
                        processed_lines.push(format!("<p>{content_before_end}</p>"));
                    }

                    processed_lines.push("</div>".to_string());
                    in_admonition = false;
                    continue;
                }

                let processed_line = process_line_markup(line, in_admonition, format_type);
                processed_lines.push(processed_line);
            }

            if in_admonition {
                processed_lines.push("</div>".to_string());
            }

            processed_lines.join("\n")
        }
        FormatType::Manpage => {
            let mut processed_lines = Vec::new();
            let mut lines = result.lines().peekable();

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

                // Extract admonition blocks
                if let Some(start_caps) = ADMONITION_START_RE.captures(line) {
                    let adm_type = &start_caps[1];
                    let content_part = start_caps.get(3).map_or("", |m| m.as_str());

                    // Get the capitalized title
                    let title = adm_type.chars().next().map_or_else(
                        || adm_type.to_string(),
                        |c| c.to_uppercase().collect::<String>() + &adm_type[1..],
                    );

                    // Handle single-line admonition - ensure .PP is on its own line
                    if let Some(end_cap) = ADMONITION_END_RE.captures(content_part) {
                        let content = end_cap.get(1).map_or("", |m| m.as_str()).trim();
                        processed_lines.push(".PP".to_string());
                        processed_lines.push(format!("\\fB{title}:\\fR {content}"));
                        continue;
                    }

                    // Start multi-line admonition
                    let mut adm_content = Vec::new();

                    // Add initial content if present
                    if !content_part.trim().is_empty() {
                        adm_content.push(content_part.trim().to_string());
                    }

                    // Collect all lines until the end marker
                    let mut found_end = false;
                    for next_line in lines.by_ref() {
                        if let Some(end_caps) = ADMONITION_END_RE.captures(next_line) {
                            let content_before_end =
                                end_caps.get(1).map_or("", |m| m.as_str()).trim();
                            if !content_before_end.is_empty() {
                                adm_content.push(content_before_end.to_string());
                            }
                            found_end = true;
                            break;
                        } else {
                            adm_content.push(next_line.to_string());
                        }
                    }

                    // Process the admonition content into paragraphs
                    let mut paragraphs = Vec::new();
                    let mut current_paragraph = Vec::new();

                    for content_line in adm_content {
                        if content_line.trim().is_empty() {
                            // Empty line means paragraph break
                            if !current_paragraph.is_empty() {
                                // Join the current paragraph and store it
                                paragraphs.push(current_paragraph.join(" "));
                                current_paragraph = Vec::new();
                            }
                        } else {
                            // Clean up header-like content in example admonitions
                            let cleaned_line =
                                if adm_type == "example" && content_line.trim().starts_with('#') {
                                    content_line.trim_start_matches('#').trim().to_string()
                                } else {
                                    content_line.trim().to_string()
                                };
                            current_paragraph.push(cleaned_line);
                        }
                    }

                    // Don't forget the last paragraph if there is one
                    if !current_paragraph.is_empty() {
                        paragraphs.push(current_paragraph.join(" "));
                    }

                    // First paragraph with admonition title - ensure .PP is on its own line
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

                processed_lines.push(line.to_string());
            }

            processed_lines.join("\n")
        }
    }
}

fn process_line_markup(line: &str, in_admonition: bool, format_type: FormatType) -> String {
    lazy_static! {
        static ref HEADING_ANCHOR_RE: Regex =
            Regex::new(r"^(#+\s+.+?)(?:\s+\{#([a-zA-Z0-9_-]+)\})\s*$").unwrap();
        static ref INLINE_ANCHOR_RE: Regex = Regex::new(r"\[\]\{#([a-zA-Z0-9_-]+)\}").unwrap();
        static ref AUTO_LINK_RE: Regex = Regex::new(r"\[\]\((#[a-zA-Z0-9_-]+)\)").unwrap();
    }

    let mut processed = Cow::from(line);

    match format_type {
        FormatType::Html => {
            if HEADING_ANCHOR_RE.is_match(&processed) {
                processed = Cow::Owned(
                    HEADING_ANCHOR_RE
                        .replace_all(&processed, |caps: &regex::Captures| {
                            let heading = &caps[1];
                            let id = &caps[2];
                            format!("{heading} <!-- anchor: {id} -->")
                        })
                        .into_owned(),
                );
            }

            if INLINE_ANCHOR_RE.is_match(&processed) {
                processed = Cow::Owned(
                    INLINE_ANCHOR_RE
                        .replace_all(&processed, |caps: &regex::Captures| {
                            let id = &caps[1];
                            format!("<span id=\"{id}\"></span>")
                        })
                        .into_owned(),
                );
            }

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

            let processed = processed.into_owned();

            if in_admonition
                && !processed.trim().is_empty()
                && !processed.trim_start().starts_with('<')
                && !processed.trim_start().starts_with('#')
            {
                format!("<p>{}</p>", processed.trim())
            } else {
                processed
            }
        }
        FormatType::Manpage => {
            if HEADING_ANCHOR_RE.is_match(&processed) {
                processed = Cow::Owned(
                    HEADING_ANCHOR_RE
                        .replace_all(&processed, |caps: &regex::Captures| {
                            let heading = &caps[1];
                            heading.to_string()
                        })
                        .into_owned(),
                );
            }

            if INLINE_ANCHOR_RE.is_match(&processed) {
                processed = Cow::Owned(
                    INLINE_ANCHOR_RE
                        .replace_all(&processed, |caps: &regex::Captures| {
                            let id = &caps[1];
                            format!("[{id}]")
                        })
                        .into_owned(),
                );
            }

            if AUTO_LINK_RE.is_match(&processed) {
                processed = Cow::Owned(
                    AUTO_LINK_RE
                        .replace_all(&processed, |caps: &regex::Captures| {
                            let anchor = &caps[1];
                            anchor.trim_start_matches('#').to_string()
                        })
                        .into_owned(),
                );
            }

            processed.into_owned()
        }
    }
}

pub fn preprocess_special_markup(markdown: &str, format_type: FormatType) -> String {
    lazy_static! {
        static ref MANPAGE_WITH_SECTION: Regex =
            Regex::new(r"\{manpage\}`([^`(]+)\(([0-9]+)\)`").unwrap();
        static ref MANPAGE_NO_SECTION: Regex = Regex::new(r"\{manpage\}`([^`(]+)`").unwrap();
        static ref COMMAND_ROLE: Regex = Regex::new(r"\{command\}`([^`]+)`").unwrap();
        static ref ENV_ROLE: Regex = Regex::new(r"\{env\}`([^`]+)`").unwrap();
        static ref FILE_ROLE: Regex = Regex::new(r"\{file\}`([^`]+)`").unwrap();
        static ref OPTION_ROLE: Regex = Regex::new(r"\{option\}`([^`]+)`").unwrap();
        static ref VAR_ROLE: Regex = Regex::new(r"\{var\}`([^`]+)`").unwrap();
        static ref COMMAND_PROMPT_RE: Regex = Regex::new(r"`\s*\$\s+([^`]+)`").unwrap();
        static ref REPL_PROMPT_RE: Regex = Regex::new(r"`nix-repl>\s*([^`]+)`").unwrap();
    }

    let mut result = markdown.to_string();

    match format_type {
        FormatType::Html => {
            let markup_pattern =
                Regex::new(r"\{(command|env|file|option|var|manpage)\}`([^`]+)`").unwrap();

            result = markup_pattern
                .replace_all(&result, |caps: &regex::Captures| {
                    let markup_type = &caps[1];
                    let content = &caps[2];
                    format!("<span class=\"{markup_type}-markup\">{content}</span>")
                })
                .into_owned();

            result = COMMAND_PROMPT_RE
                .replace_all(&result, |caps: &regex::Captures| {
                    format!(
                        "<code class=\"terminal\"><span class=\"prompt\">$</span> {}</code>",
                        &caps[1]
                    )
                })
                .into_owned();

            result = REPL_PROMPT_RE
                .replace_all(&result, |caps: &regex::Captures| {
                    format!(
                        "<code class=\"nix-repl\"><span class=\"prompt\">nix-repl&gt;</span> {}</code>",
                        &caps[1]
                    )
                })
                .into_owned();
        }

        FormatType::Manpage => {
            result = MANPAGE_WITH_SECTION
                .replace_all(&result, |caps: &regex::Captures| {
                    let page = &caps[1];
                    let section = &caps[2];
                    format!("\\fB{page}\\fR({section})")
                })
                .into_owned();

            result = MANPAGE_NO_SECTION
                .replace_all(&result, |caps: &regex::Captures| {
                    let page = &caps[1];
                    format!("\\fB{page}\\fR")
                })
                .into_owned();

            result = COMMAND_ROLE
                .replace_all(&result, |caps: &regex::Captures| {
                    format!("\\fB{}\\fR", &caps[1])
                })
                .into_owned();

            result = ENV_ROLE
                .replace_all(&result, |caps: &regex::Captures| {
                    format!("\\fI{}\\fR", &caps[1])
                })
                .into_owned();

            result = FILE_ROLE
                .replace_all(&result, |caps: &regex::Captures| {
                    format!("\\fI{}\\fR", &caps[1])
                })
                .into_owned();

            result = OPTION_ROLE
                .replace_all(&result, |caps: &regex::Captures| {
                    format!("\\fB{}\\fR", &caps[1])
                })
                .into_owned();

            result = VAR_ROLE
                .replace_all(&result, |caps: &regex::Captures| {
                    format!("\\fI{}\\fR", &caps[1])
                })
                .into_owned();

            result = COMMAND_PROMPT_RE
                .replace_all(&result, |caps: &regex::Captures| {
                    format!("\\fB$\\fR {}", &caps[1])
                })
                .into_owned();

            result = REPL_PROMPT_RE
                .replace_all(&result, |caps: &regex::Captures| {
                    format!("\\fBnix-repl>\\fR {}", &caps[1])
                })
                .into_owned();
        }
    }

    result
}
