pub mod assets;
pub mod html;
pub mod json;
pub mod markdown;
pub mod output;
pub mod postprocess;
pub mod xref;

// Re-export commonly used utilities
pub use assets::copy_assets;
pub use markdown::{
  create_processor,
  process_markdown_files,
  process_markdown_files_with_cache,
};
pub use output::create_fallback_index;
pub use xref::{
  AnchorEntry,
  AnchorRegistry,
  apply_cross_page_link_rewrites,
  build_anchor_registry,
};
