//! The otter-lang compiler's span type.
//!
//! # Note
//!
//! This API is completely unstable and subject to change.

use core::ops::Range;

/// A range typically used to define a slice of source-text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    /// The end of the span.
    end: usize,
    /// The start of the span.
    start: usize,
}

// constructors

impl Span {
    /// Creates a new span, starting from the lowest start, and continuing to the highest end
    #[inline]
    #[must_use]
    pub fn merge(&self, other: &Self) -> Self {
        Self::new(self.start.min(other.start), self.end.max(other.end))
    }

    /// Creates a new instance of `Span`
    #[inline]
    #[must_use]
    pub const fn new(start: usize, end: usize) -> Self {
        Self { end, start }
    }
}

// methods

impl Span {
    /// Returns whether or not `pos` falls within `self`
    #[inline]
    #[must_use]
    pub const fn contains(&self, pos: usize) -> bool {
        pos >= self.start && pos < self.end
    }

    /// The end of the span
    #[inline]
    #[must_use]
    pub const fn end(&self) -> usize {
        self.end
    }

    /// Whether or not the start comes at - or after - the end
    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.start >= self.end
    }

    /// The distance between the start and the end
    #[inline]
    #[must_use]
    pub const fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }

    /// the start of the span
    #[inline]
    #[must_use]
    pub const fn start(&self) -> usize {
        self.start
    }
}

impl From<Span> for Range<usize> {
    #[inline]
    fn from(span: Span) -> Self {
        span.start..span.end
    }
}

impl From<Range<usize>> for Span {
    #[inline]
    fn from(range: Range<usize>) -> Self {
        Self::new(range.start, range.end)
    }
}
