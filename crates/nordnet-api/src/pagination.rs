//! Offset/limit pagination helpers.
//!
//! The Nordnet API uses query-string offset/limit for paged endpoints
//! (e.g. `?offset=0&limit=100`). The actual response shape varies by
//! endpoint — some return a bare JSON array, others wrap it in a typed
//! object. This module provides a generic [`Page`] holder that group
//! implementers can either compose into their own response types or use
//! directly when the wire shape matches.

use serde::{Deserialize, Serialize};

/// One page of results plus the offset/limit window it represents. The
/// API does not always return a total count, so it is omitted here.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub offset: u64,
    pub limit: u64,
}

impl<T> Page<T> {
    /// Build a page from a vector. `offset` and `limit` correspond to the
    /// query parameters that produced this page.
    pub fn new(items: Vec<T>, offset: u64, limit: u64) -> Self {
        Self {
            items,
            offset,
            limit,
        }
    }

    /// Number of items returned in this page (0..=limit).
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// True if this page is empty.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// True if this page is "probably last" — i.e. fewer items than the
    /// page size were returned. Heuristic; some endpoints return exactly
    /// `limit` on the last page.
    pub fn is_probably_last(&self) -> bool {
        (self.items.len() as u64) < self.limit
    }

    /// The offset to request for the next page.
    pub fn next_offset(&self) -> u64 {
        self.offset + self.items.len() as u64
    }
}

/// Iterator helper: converts a `Page<T>` into an owned iterator over its
/// items, discarding the window metadata.
impl<T> IntoIterator for Page<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.items.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_basics() {
        let p = Page::new(vec![1, 2, 3], 0, 10);
        assert_eq!(p.len(), 3);
        assert!(!p.is_empty());
        assert!(p.is_probably_last());
        assert_eq!(p.next_offset(), 3);
    }

    #[test]
    fn full_page_not_probably_last() {
        let p = Page::new(vec![1, 2, 3], 0, 3);
        assert!(!p.is_probably_last());
        assert_eq!(p.next_offset(), 3);
    }

    #[test]
    fn iter() {
        let p = Page::new(vec!["a", "b"], 5, 10);
        let v: Vec<_> = p.into_iter().collect();
        assert_eq!(v, vec!["a", "b"]);
    }
}
