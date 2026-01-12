pub mod escape;
pub mod options;
pub mod types;

// Re-export commonly used items
pub use escape::{
  ROFF_ESCAPES,
  TROFF_ESCAPE,
  TROFF_FORMATTING,
  escape_leading_dot,
  escape_non_macro_lines,
  man_escape,
};
pub use options::generate_manpage;
pub use types::NixOption;
