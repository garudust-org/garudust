pub mod browser;
pub mod cron;
pub mod delegate;
pub mod files;
pub mod git;
pub mod image;
pub mod json_query;
pub mod mcp;
pub mod memory;
pub mod notes;
pub mod pdf;
pub mod rag;
pub mod script;
pub mod search;
pub mod skills;
pub mod terminal;
pub mod web;

/// Returns the largest byte index ≤ `index` that is a valid UTF-8 char boundary
/// in `s`. Equivalent to `str::floor_char_boundary` (stable since 1.91) but
/// implemented with `is_char_boundary` (stable since 1.0) for MSRV 1.87.
pub(crate) fn floor_char_boundary(s: &str, index: usize) -> usize {
    let mut i = index.min(s.len());
    while i > 0 && !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}
