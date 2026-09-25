//! Core data types used by Gameexe.ini lexing and parsing.

use std::borrow::Cow;
use std::ops::Range;

/// A byte range in the original UTF-8 input, with an exclusive end offset.
///
/// Byte offsets can be used directly to slice the source string. They are not
/// character counts; a Japanese character occupies more than one byte.
pub type Span = Range<usize>;

/// One lexical unit in a Gameexe.ini file.
///
/// Unchanged words borrow their spelling from the input. Words joined across
/// block comments, and signed integers with spaces, own their normalized text.
/// In particular, `070` remains `070`, and string escapes are not decoded here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token<'a> {
    /// The `#` at the beginning of a configuration entry.
    Hash,
    /// A key or other unquoted name, including dots and embedded hyphens.
    Identifier(Cow<'a, str>),
    /// A signed or unsigned decimal integer in its original spelling.
    Integer(Cow<'a, str>),
    /// The content between double quotes, without the surrounding quotes.
    String(&'a str),
    /// The `=` between a key and its values.
    Equals,
    /// A comma separating values.
    Comma,
    /// An opening parenthesis in a grouped value.
    LeftParen,
    /// A closing parenthesis in a grouped value.
    RightParen,
    /// A line ending (`\n` or `\r\n`).
    Newline,
    /// A bare carriage return is retained in the current logical line.
    CarriageReturn,
}

/// A token together with its location in the original input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpannedToken<'a> {
    /// The recognized token.
    pub token: Token<'a>,
    /// Byte range covering the whole token, including string quotes.
    pub span: Span,
}

/// The reason lexing could not continue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LexErrorKind {
    /// A character is not part of the Gameexe.ini token alphabet.
    UnexpectedCharacter,
    /// A quoted string reached a line ending or the end of input.
    UnterminatedString,
    /// A block comment reached the end of input.
    UnterminatedComment,
}

/// A lexical error and the byte range that caused it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexError {
    /// The error category.
    pub kind: LexErrorKind,
    /// Byte range of the offending character or unfinished string.
    pub span: Span,
}

/// A count-bounded sequence addressed by Gameexe.ini numeric path segments.
pub struct GArray<T> {
    items: Vec<T>,
    inactive: std::collections::BTreeMap<usize, T>,
    make_item: fn(usize) -> T,
    min_count: usize,
    max_count: usize,
}

impl<T: std::fmt::Debug> std::fmt::Debug for GArray<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_slice().fmt(f)
    }
}

impl<T> GArray<T> {
    pub fn new(
        count: usize,
        min_count: usize,
        max_count: usize,
        make_item: fn(usize) -> T,
    ) -> Self {
        assert!(min_count <= count && count <= max_count);
        Self {
            items: (0..count).map(make_item).collect(),
            inactive: std::collections::BTreeMap::new(),
            make_item,
            min_count,
            max_count,
        }
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        self.as_slice().get(index)
    }

    pub fn as_slice(&self) -> &[T] {
        &self.items
    }

    pub fn as_mut_slice(&mut self) -> &mut [T] {
        &mut self.items
    }

    pub(crate) fn count_bounds(&self) -> (usize, usize) {
        (self.min_count, self.max_count)
    }

    pub(crate) fn set_active_count(&mut self, count: usize) {
        while self.items.len() > count {
            let index = self.items.len() - 1;
            self.inactive.insert(index, self.items.pop().unwrap());
        }
        while self.items.len() < count {
            let index = self.items.len();
            let item = self
                .inactive
                .remove(&index)
                .unwrap_or_else(|| (self.make_item)(index));
            self.items.push(item);
        }
    }

    pub(crate) fn storage_mut(&mut self, index: usize) -> Option<&mut T> {
        if index >= self.max_count {
            return None;
        }
        if index < self.items.len() {
            return self.items.get_mut(index);
        }
        Some(
            self.inactive
                .entry(index)
                .or_insert_with(|| (self.make_item)(index)),
        )
    }
}
