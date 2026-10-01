//! Portable Unity package operations. Payloads are never stored in the index.
pub mod archive;
pub mod extract;
pub mod mutate;
pub mod pack;
pub mod paths;
pub mod query;
pub mod stats;
pub mod write;

use std::fmt;

#[derive(Debug)]
pub struct NoMatch(pub String);
impl fmt::Display for NoMatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
impl std::error::Error for NoMatch {}
