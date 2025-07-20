mod extensions;
pub mod processor;
mod types;
pub mod utils;

pub mod legacy_markdown;
pub mod legacy_markup;

pub use crate::{
    processor::{AstTransformer, MarkdownOptions, MarkdownProcessor},
    types::{Header, MarkdownResult},
};
