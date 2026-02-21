use std::{fmt::Write, path::Path};

use ndg_nixdoc::NixDocEntry;

pub fn generate_lib_entries_html(
  entries: &[NixDocEntry],
  revision: &str,
) -> String {
  let mut html = String::new();
  for entry in entries {
    let attr_path = entry.attr_path.join(".");
    let id_raw = attr_path.replace('.', "-");
    let id = html_escape::encode_safe(&id_raw);
    let attr_path_escaped = html_escape::encode_text(&attr_path);

    let _ = write!(html, "<section class=\"lib-entry\" id=\"{id}\">");
    let _ = write!(
      html,
      "<h3 class=\"lib-entry-name\"><a class=\"lib-entry-anchor\" \
       href=\"#{id}\">{attr_path_escaped}</a></h3>"
    );

    if let Some(doc) = &entry.doc {
      if let Some(type_sig) = &doc.type_sig {
        let escaped = html_escape::encode_text(type_sig);
        let _ = write!(
          html,
          "<div class=\"lib-entry-type\"><code>{escaped}</code></div>"
        );
      }

      if !doc.description.is_empty() {
        let description = html_escape::encode_text(&doc.description);
        let _ = write!(
          html,
          "<div class=\"lib-entry-description\">{description}</div>"
        );
      }

      if !doc.arguments.is_empty() {
        let _ = write!(
          html,
          "<div class=\"lib-entry-arguments\"><h3>Arguments</h3><dl>"
        );
        for arg in &doc.arguments {
          let name = html_escape::encode_text(&arg.name);
          let arg_desc = html_escape::encode_text(&arg.description);
          let _ =
            write!(html, "<dt><code>{name}</code></dt><dd>{arg_desc}</dd>",);
        }
        let _ = write!(html, "</dl></div>");
      }

      if !doc.examples.is_empty() {
        let _ =
          write!(html, "<div class=\"lib-entry-examples\"><h3>Examples</h3>");
        for ex in &doc.examples {
          let lang =
            html_escape::encode_safe(ex.language.as_deref().unwrap_or("nix"));
          let code = html_escape::encode_text(&ex.code);
          let _ = write!(
            html,
            "<pre><code class=\"language-{lang}\">{code}</code></pre>"
          );
        }
        let _ = write!(html, "</div>");
      }

      if doc.is_deprecated {
        let _ = write!(html, "<div class=\"lib-entry-deprecated\">");
        if let Some(notice) = &doc.deprecation_notice {
          let notice_escaped = html_escape::encode_text(notice);
          let _ = write!(
            html,
            "<p><strong>Deprecated:</strong> {notice_escaped}</p>"
          );
        } else {
          let _ = write!(html, "<p><strong>Deprecated</strong></p>");
        }
        let _ = write!(html, "</div>");
      }

      if !doc.notes.is_empty() {
        let _ =
          write!(html, "<div class=\"lib-entry-notes\"><h3>Notes</h3><ul>");
        for note in &doc.notes {
          let note_escaped = html_escape::encode_text(note);
          let _ = write!(html, "<li>{note_escaped}</li>");
        }
        let _ = write!(html, "</ul></div>");
      }

      if !doc.warnings.is_empty() {
        let _ = write!(
          html,
          "<div class=\"lib-entry-warnings\"><h3>Warnings</h3><ul>"
        );
        for warning in &doc.warnings {
          let warning_escaped = html_escape::encode_text(warning);
          let _ = write!(html, "<li>{warning_escaped}</li>");
        }
        let _ = write!(html, "</ul></div>");
      }
    }

    let file_path = &entry.location.file;
    let file_str = file_path.display().to_string();
    let file_display = html_escape::encode_text(&file_str);

    let (display, url) = format_location(file_path, revision);

    if let Some(link) = url {
      let link_escaped = html_escape::encode_safe(&link);
      let display_escaped = html_escape::encode_text(&display);
      let _ = write!(
        html,
        "  <div class=\"lib-entry-declared\">Declared in: <code><a \
         href=\"{link_escaped}\" \
         target=\"_blank\">{display_escaped}</a></code></div>"
      );
    } else {
      let _ = write!(
        html,
        "  <div class=\"lib-entry-declared\">Declared in: \
         <code>{file_display}</code></div>"
      );
    }
    let _ = write!(html, "</section>");
  }
  html
}

fn format_location(
  file_path: &Path,
  revision: &str,
) -> (String, Option<String>) {
  let path_str = file_path.to_string_lossy().to_string();

  if path_str.starts_with('/') {
    (path_str, None)
  } else {
    let url = if revision == "local" {
      format!("https://github.com/NixOS/nixpkgs/blob/master/{path_str}")
    } else {
      format!("https://github.com/NixOS/nixpkgs/blob/{revision}/{path_str}")
    };
    let display = format!("<nixpkgs/{path_str}>");
    (display, Some(url))
  }
}

pub fn generate_lib_toc_html(entries: &[NixDocEntry]) -> String {
  let mut html = String::new();
  for entry in entries {
    let attr_path = entry.attr_path.join(".");
    let id_raw = attr_path.replace('.', "-");
    let id = html_escape::encode_safe(&id_raw);
    let attr_path_escaped = html_escape::encode_text(&attr_path);
    let _ =
      writeln!(html, "<li><a href=\"#{id}\">{attr_path_escaped}</a></li>");
  }
  html
}
