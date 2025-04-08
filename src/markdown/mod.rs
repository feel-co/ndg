pub mod highlight;
pub mod parser;
pub mod postprocess;
pub mod preprocess;
mod utils;

// Re-export public items
pub use parser::{collect_markdown_files, process_markdown_file, process_markdown_string, Header};
pub use postprocess::extract_headers;
