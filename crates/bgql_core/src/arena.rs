//! Arena allocation for Better GraphQL.

use bumpalo::Bump;

/// An arena allocator for AST nodes.
///
/// Uses bumpalo for fast bump allocation.
#[derive(Debug)]
pub struct Arena {
    bump: Bump,
}

impl Default for Arena {
    fn default() -> Self {
        Self::new()
    }
}

impl Arena {
    /// Creates a new arena.
    #[must_use]
    pub fn new() -> Self {
        Self { bump: Bump::new() }
    }

    /// Creates a new arena with pre-allocated capacity.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            bump: Bump::with_capacity(capacity),
        }
    }

    /// Allocates a value in the arena.
    #[inline]
    pub fn alloc<T>(&self, value: T) -> &mut T {
        self.bump.alloc(value)
    }

    /// Allocates a slice in the arena.
    #[inline]
    pub fn alloc_slice<T: Copy>(&self, slice: &[T]) -> &mut [T] {
        self.bump.alloc_slice_copy(slice)
    }

    /// Allocates a string in the arena.
    #[inline]
    pub fn alloc_str(&self, s: &str) -> &str {
        self.bump.alloc_str(s)
    }

    /// Returns the number of bytes allocated.
    #[must_use]
    pub fn allocated_bytes(&self) -> usize {
        self.bump.allocated_bytes()
    }

    /// Resets the arena, deallocating all allocations.
    pub fn reset(&mut self) {
        self.bump.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alloc() {
        let arena = Arena::new();
        let value = arena.alloc(42);
        assert_eq!(*value, 42);
    }

    #[test]
    fn test_alloc_str() {
        let arena = Arena::new();
        let s = arena.alloc_str("hello");
        assert_eq!(s, "hello");
    }

    #[test]
    fn test_alloc_slice() {
        let arena = Arena::new();
        let slice = arena.alloc_slice(&[1, 2, 3]);
        assert_eq!(slice, &[1, 2, 3]);
    }
}
