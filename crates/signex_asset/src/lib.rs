//! Asset format support for Siglus games.

extern crate self as signex_asset;

pub mod audio;
pub mod g00;
pub mod gameexe;
mod lzss;
mod reader;
pub mod scene;

pub use audio::{
    AudioDecodeError, NwaAudio, NwaHeader, OvkEntry, OvkIndex, decode_nwa, nwa, ovk,
    parse_nwa_header, parse_ovk,
};
pub use g00::{G00Chip, G00Cut, G00Error, G00Image, G00Kind, G00Rect, decode_g00};
