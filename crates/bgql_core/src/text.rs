//! String interning for Better GraphQL.

use rustc_hash::FxHashMap;
use std::cell::RefCell;

/// An interned text identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Text(u32);

impl Text {
    /// Creates a new text from a raw index.
    #[must_use]
    pub const fn from_raw(index: u32) -> Self {
        Self(index)
    }

    /// Returns the raw index.
    #[must_use]
    pub const fn as_raw(self) -> u32 {
        self.0
    }
}

/// A string interner that deduplicates strings.
#[derive(Debug)]
pub struct Interner {
    /// Map from string to index.
    map: RefCell<FxHashMap<String, Text>>,
    /// Stored strings.
    strings: RefCell<Vec<String>>,
}

impl Default for Interner {
    fn default() -> Self {
        Self::new()
    }
}

impl Interner {
    /// Creates a new interner with built-in keywords pre-registered.
    #[must_use]
    pub fn new() -> Self {
        let interner = Self {
            map: RefCell::new(FxHashMap::default()),
            strings: RefCell::new(Vec::new()),
        };

        // Pre-register built-in scalars and keywords
        for keyword in [
            "Int",
            "Float",
            "String",
            "Boolean",
            "ID",
            "type",
            "interface",
            "union",
            "enum",
            "input",
            "scalar",
            "schema",
            "query",
            "mutation",
            "subscription",
            "fragment",
            "on",
            "directive",
            "extend",
            "implements",
            "opaque",
            "Option",
            "List",
            "alias",
            "true",
            "false",
            "null",
        ] {
            interner.intern(keyword);
        }

        interner
    }

    /// Interns a string, returning its identifier.
    pub fn intern(&self, s: &str) -> Text {
        let mut map = self.map.borrow_mut();
        if let Some(&id) = map.get(s) {
            return id;
        }

        let mut strings = self.strings.borrow_mut();
        let id = Text(strings.len() as u32);
        strings.push(s.to_string());
        map.insert(s.to_string(), id);
        id
    }

    /// Gets the string for an identifier.
    #[must_use]
    pub fn get(&self, id: Text) -> String {
        let strings = self.strings.borrow();
        strings.get(id.0 as usize).cloned().unwrap_or_default()
    }

    /// Returns the number of interned strings.
    #[must_use]
    pub fn len(&self) -> usize {
        self.strings.borrow().len()
    }

    /// Returns true if no strings are interned.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.strings.borrow().is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intern() {
        let interner = Interner::new();
        let id1 = interner.intern("hello");
        let id2 = interner.intern("hello");
        let id3 = interner.intern("world");

        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_get() {
        let interner = Interner::new();
        let id = interner.intern("test");
        assert_eq!(interner.get(id), "test");
    }

    #[test]
    fn test_builtin_keywords() {
        let interner = Interner::new();
        // Built-in scalars should already be interned
        let int_id = interner.intern("Int");
        assert_eq!(interner.get(int_id), "Int");
    }
}
