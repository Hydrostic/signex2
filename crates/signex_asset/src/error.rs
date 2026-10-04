//! Shared errors for binary asset decoders.

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DecodeError {
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
    #[error("invalid UTF-16 in `{field}` at byte {offset}")]
    InvalidUtf16 { field: &'static str, offset: u64 },
    #[error("unsupported `{feature}` at byte {offset}")]
    Unsupported { feature: &'static str, offset: u64 },
    #[error("cannot allocate `{field}`")]
    AllocationFailed { field: &'static str },
    #[error("required decode key `{key}` is missing")]
    KeyMissing { key: &'static str },
    #[error("LZSS error: {0}")]
    Lzss(#[from] crate::lzss::LzssError),
}

impl From<crate::reader::ReadError> for DecodeError {
    fn from(error: crate::reader::ReadError) -> Self {
        match error {
            crate::reader::ReadError::UnexpectedEof {
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
            crate::reader::ReadError::ArithmeticOverflow { field, offset } => {
                Self::ArithmeticOverflow { field, offset }
            }
        }
    }
}
