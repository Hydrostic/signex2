//! OVK voice-pack and NWA audio readers.

pub mod nwa;
pub mod ovk;

pub use nwa::{NwaAudio, NwaHeader, decode_nwa, parse_nwa_header};
pub use ovk::{OvkEntry, OvkIndex, parse_ovk};

use crate::reader::ReadError;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AudioDecodeError {
    #[error(
        "unexpected EOF while reading `{field}` at byte {offset}: needed {needed}, {remaining} remaining"
    )]
    UnexpectedEof {
        field: &'static str,
        offset: u64,
        needed: usize,
        remaining: usize,
    },
    #[error("invalid value {value} for `{field}` at byte {offset}")]
    InvalidValue {
        field: &'static str,
        offset: u64,
        value: i64,
    },
    #[error(
        "invalid `{field}` range at byte {offset}: start {start}, length {length}, input length {input_len}"
    )]
    InvalidRange {
        field: &'static str,
        offset: u64,
        start: u64,
        length: u64,
        input_len: u64,
    },
    #[error("arithmetic overflow in `{field}` at byte {offset}")]
    ArithmeticOverflow { field: &'static str, offset: u64 },
    #[error("unsupported `{feature}` at byte {offset}")]
    Unsupported { feature: &'static str, offset: u64 },
    #[error("cannot allocate `{field}`")]
    AllocationFailed { field: &'static str },
}

impl From<ReadError> for AudioDecodeError {
    fn from(error: ReadError) -> Self {
        match error {
            ReadError::UnexpectedEof {
                field,
                offset,
                needed,
                remaining,
            } => Self::UnexpectedEof {
                field,
                offset,
                needed,
                remaining,
            },
            ReadError::ArithmeticOverflow { field, offset } => {
                Self::ArithmeticOverflow { field, offset }
            }
        }
    }
}

pub(crate) fn checked_size(
    count: usize,
    item_size: usize,
    field: &'static str,
    offset: u64,
) -> Result<usize, AudioDecodeError> {
    count
        .checked_mul(item_size)
        .ok_or(AudioDecodeError::ArithmeticOverflow { field, offset })
}
