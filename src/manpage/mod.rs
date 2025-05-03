mod options;

pub use crate::manpage::options::generate_manpage;

/// Map of characters that need to be escaped in manpages
pub fn get_roff_escapes() -> std::collections::HashMap<char, &'static str> {
   let mut map = std::collections::HashMap::new();
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
pub fn man_escape(s: &str) -> String {
   let escapes = get_roff_escapes();
   let mut result = String::with_capacity(s.len() * 2);

   for c in s.chars() {
      if let Some(escape) = escapes.get(&c) {
         result.push_str(escape);
      } else {
         result.push(c);
      }
   }

   result
}

/// Escape a leading dot to prevent it from being interpreted as a troff command
pub fn escape_leading_dot(text: &str) -> String {
   if text.starts_with('.') || text.starts_with('\'') {
      format!("\\&{text}")
   } else {
      text.to_string()
   }
}
