//! Expose NDG's internal API for use in unit testing. While it *could* be useful, we do not
//! recommend using this API in production code. It is primarily intended for testing purposes.
pub mod cli;
pub mod completion;
pub mod config;
pub mod formatter;
pub mod html;
pub mod manpage;
pub mod utils;
