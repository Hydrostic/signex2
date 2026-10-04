//! Checked arithmetic shared by binary asset decoders.

use crate::DecodeError;
use crate::reader::CheckedReader;

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

pub(crate) fn validate_range(
    bytes: &[u8],
    start: usize,
    len: usize,
    field: &'static str,
) -> Result<(), DecodeError> {
    if start.checked_add(len).is_none_or(|end| end > bytes.len()) {
        return Err(DecodeError::InvalidRange {
            field,
            offset: 0,
            start: start as u64,
            length: len as u64,
            input_len: bytes.len() as u64,
        });
    }
    Ok(())
}

pub(crate) fn slice_at<'a>(
    bytes: &'a [u8],
    start: usize,
    len: usize,
    field: &'static str,
) -> Result<CheckedReader<'a>, DecodeError> {
    validate_range(bytes, start, len, field)?;
    Ok(CheckedReader::from_slice(
        &bytes[start..start + len],
        start as u64,
    ))
}

pub(crate) fn xor_words(bytes: &mut [u8], key: u32) {
    for word in bytes.chunks_exact_mut(4) {
        let value = u32::from_le_bytes(word.try_into().expect("four bytes")) ^ key;
        word.copy_from_slice(&value.to_le_bytes());
    }
}

pub(crate) fn xor_cycle(bytes: &mut [u8], key: &[u8]) {
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte ^= key[index % key.len()];
    }
}
