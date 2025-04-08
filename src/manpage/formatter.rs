use regex::Regex;

/// Escapes characters that need special handling in man pages
pub fn escape_manpage(text: &str) -> String {
    // These characters need to be escaped in troff: - \ '
    // Important: escape backslash FIRST, then other characters
    let mut result = text.replace('\\', "\\\\");
    result = result.replace('-', "\\-");
    result = result.replace('\'', "\\(aq");
    result
}

/// Handle lines starting with a dot (which have special meaning in troff/man)
pub fn escape_leading_dot(text: &str) -> String {
    if text.starts_with('.') {
        format!("\\&{text}")
    } else {
        text.to_string()
    }
}

/// Fix common issues in generated manpage content
pub fn postprocess_manpage(content: &str) -> String {
    lazy_static::lazy_static! {
        // Fix list item formatting issues
        static ref LIST_ITEM_CLEANUP: Regex = Regex::new(r"(\\f[BI][^\\]+\\fR)\.IP").unwrap();

        // Remove stray bullet points
        static ref STRAY_IP_BULLET: Regex = Regex::new(r"\.IP • 2").unwrap();

        // Fix trailing formatting in lists
        static ref TRAILING_RE: Regex = Regex::new(r"\\fR\.RE").unwrap();

        // Remove double newlines (troff is sensitive to these)
        static ref DOUBLE_NEWLINE_RE: Regex = Regex::new(r"\n\n").unwrap();

        // Deduplicate paragraph markers
        static ref DUPLICATE_PP: Regex = Regex::new(r"\.PP\n\.PP").unwrap();

        // Fix admonition formatting
        static ref EXAMPLE_HASH_CLEANUP: Regex = Regex::new(r"Example:\s*# (.+)").unwrap();

        // Fix inline paragraph directives
        static ref INLINE_PP_RE: Regex = Regex::new(r"([^\n])\s*\.PP\s+").unwrap();

        // Fix paragraph breaks within admonitions
        static ref ADMONITION_PARAGRAPH_FIX: Regex = Regex::new(r"(\.PP\n[A-Z][a-z]+: .+)(\.PP)").unwrap();
    }

    let mut result = content.to_string();

    // Fix duplicate paragraph markers
    result = DUPLICATE_PP.replace_all(&result, ".PP").to_string();

    // Clean up list item markup
    result = LIST_ITEM_CLEANUP
        .replace_all(&result, "$1\n.IP")
        .to_string();

    // Fix example admonitions with # in title
    result = EXAMPLE_HASH_CLEANUP
        .replace_all(&result, "Example: $1")
        .to_string();

    // Fix paragraph breaks within admonitions
    result = ADMONITION_PARAGRAPH_FIX
        .replace_all(&result, "$1\n.br\n")
        .to_string();

    // Fix trailing formatting issues
    result = TRAILING_RE.replace_all(&result, "\\fR\n.RE").to_string();
    result = STRAY_IP_BULLET.replace_all(&result, "").to_string();

    // Normalize multiple newlines to single newlines where appropriate
    result = DOUBLE_NEWLINE_RE.replace_all(&result, "\n").to_string();

    // Ensure paragraph markers are on their own lines
    result = INLINE_PP_RE.replace_all(&result, "$1\n.PP\n").to_string();

    // Fix escaping issues for formatting codes
    result = result.replace("\\\\fB", "\\fB");
    result = result.replace("\\\\fI", "\\fI");
    result = result.replace("\\\\fR", "\\fR");

    result
}
