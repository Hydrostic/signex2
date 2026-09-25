//! Tools for reading Siglus Gameexe.ini configuration text.

mod lexer;
pub mod model;
mod parser;
pub mod types;

pub use lexer::Lexer;
pub use parser::{GameexeError, GameexeNode, GameexeValue, parse};
pub use signex_gameexe_macro::gameexe;
pub use types::{GArray, LexError, LexErrorKind, Span, SpannedToken, Token};
