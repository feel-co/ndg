pub mod assets;
pub mod html;
pub mod json;
pub mod markdown;
pub mod output;
pub mod postprocess;

// Re-export commonly used utilities
pub use assets::copy_assets;
pub use markdown::{
  collect_included_files,
  create_processor,
  process_markdown_files,
};
pub use output::create_fallback_index;
