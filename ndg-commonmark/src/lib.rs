mod extensions;
mod processor;
mod types;
mod utils;

pub use crate::{
    processor::{AstTransformer, MarkdownOptions, MarkdownProcessor},
    types::{Header, MarkdownResult},
};
