//! Checked arithmetic shared by binary asset decoders.

use crate::DecodeError;

pub(crate) fn checked_count(
    value: i32,
    field: &'static str,
    offset: u64,
) -> Result<usize, DecodeError> {
    usize::try_from(value).map_err(|_| DecodeError::InvalidValue {
        field,
        offset,
        value: i64::from(value),
    })
}

pub(crate) fn checked_size(
    count: usize,
    item_size: usize,
    field: &'static str,
    offset: u64,
) -> Result<usize, DecodeError> {
    count
        .checked_mul(item_size)
        .ok_or(DecodeError::ArithmeticOverflow { field, offset })
}

pub(crate) fn checked_target(
    value: usize,
    max: usize,
    field: &'static str,
    offset: u64,
) -> Result<u32, DecodeError> {
    if value > max {
        return Err(DecodeError::InvalidRange {
            field,
            offset,
            start: u64::try_from(value).unwrap_or(u64::MAX),
            length: 1,
            input_len: u64::try_from(max).unwrap_or(u64::MAX),
        });
    }
    u32::try_from(value).map_err(|_| DecodeError::ArithmeticOverflow { field, offset })
}
