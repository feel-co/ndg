//! NDG's internal API exposed in order to be used in unit testing. While this
//! could be useful for end users, end users should opt in to consume
//! ndg-commonmark directly. This interface is for testing purposes ONLY and
//! will not make any guarantees of stability.

pub mod cli;
pub mod completion;
pub mod config;
pub mod formatter;
pub mod html;
pub mod manpage;
pub mod utils;
