//! Asset format support for Siglus games.

extern crate self as signex_asset;

pub mod audio;
pub mod cgtable;
pub mod database;
mod error;
pub mod g00;
pub mod gameexe;
mod lzss;
pub mod omv;
mod reader;
pub mod scene;
mod util;

pub use audio::{
    AudioDecodeError, NwaAudio, NwaHeader, OvkEntry, OvkIndex, decode_nwa, nwa, ovk,
    parse_nwa_header, parse_ovk,
};
pub use cgtable::{
    CgTable, CgTableEncodeError, CgTableFormat, CgTableRecord, decode_cg_table, encode_cg_table,
};
pub use database::{
    Database, DatabaseCell, DatabaseColumn, DatabaseColumnType, DatabaseRow, decode_database,
};
pub use error::DecodeError;
pub use g00::{G00Chip, G00Cut, G00Error, G00Image, G00Kind, G00Rect, decode_g00};
pub use omv::{
    OMV_HEADER_SIZE, OMV_PACKET_SIZE, OMV_PAGE_SIZE, OmvContainer, OmvError, OmvHeader,
    OmvTheoraType, parse_omv,
};
