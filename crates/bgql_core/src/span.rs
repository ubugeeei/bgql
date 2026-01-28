//! Source span representation.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// A span in source code, represented as byte offsets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Span {
    /// Start byte offset (inclusive).
    pub start: u32,
    /// End byte offset (exclusive).
    pub end: u32,
}

impl Span {
    /// Creates a new span.
    #[must_use]
    #[inline]
    pub const fn new(start: u32, end: u32) -> Self {
        Self { start, end }
    }

    /// Creates an empty span at a position.
    #[must_use]
    #[inline]
    pub const fn empty(pos: u32) -> Self {
        Self {
            start: pos,
            end: pos,
        }
    }

    /// Returns the length of this span in bytes.
    #[must_use]
    #[inline]
    pub const fn len(&self) -> u32 {
        self.end - self.start
    }

    /// Returns true if this span is empty.
    #[must_use]
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.start == self.end
    }

    /// Returns a span that covers both spans.
    #[must_use]
    #[inline]
    pub fn merge(self, other: Self) -> Self {
        Self {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }

    /// Returns true if this span contains the given position.
    #[must_use]
    #[inline]
    pub const fn contains(&self, pos: u32) -> bool {
        self.start <= pos && pos < self.end
    }

    /// Returns true if this span overlaps with another.
    #[must_use]
    #[inline]
    pub const fn overlaps(&self, other: &Self) -> bool {
        self.start < other.end && other.start < self.end
    }
}

impl From<std::ops::Range<u32>> for Span {
    fn from(range: std::ops::Range<u32>) -> Self {
        Self::new(range.start, range.end)
    }
}

impl From<Span> for std::ops::Range<usize> {
    fn from(span: Span) -> Self {
        span.start as usize..span.end as usize
    }
}

impl From<Span> for miette::SourceSpan {
    fn from(span: Span) -> Self {
        miette::SourceSpan::new(
            miette::SourceOffset::from(span.start as usize),
            (span.end - span.start) as usize,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_span_new() {
        let span = Span::new(10, 20);
        assert_eq!(span.start, 10);
        assert_eq!(span.end, 20);
        assert_eq!(span.len(), 10);
    }

    #[test]
    fn test_span_merge() {
        let a = Span::new(10, 20);
        let b = Span::new(15, 30);
        let merged = a.merge(b);
        assert_eq!(merged.start, 10);
        assert_eq!(merged.end, 30);
    }

    #[test]
    fn test_span_contains() {
        let span = Span::new(10, 20);
        assert!(span.contains(10));
        assert!(span.contains(15));
        assert!(!span.contains(20));
        assert!(!span.contains(5));
    }
}
