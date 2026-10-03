use super::error::DecodeError;

pub struct CheckedReader<'a> {
    bytes: &'a [u8],
    cursor: usize,
    base: u64,
}

impl<'a> CheckedReader<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            cursor: 0,
            base: 0,
        }
    }
    pub fn from_slice(bytes: &'a [u8], base: u64) -> Self {
        Self {
            bytes,
            cursor: 0,
            base,
        }
    }
    pub fn offset(&self) -> u64 {
        self.base + self.cursor as u64
    }
    pub fn read_i32_le(&mut self) -> Result<i32, DecodeError> {
        Ok(i32::from_le_bytes(
            self.read_exact_named(4, "i32")?.try_into().unwrap(),
        ))
    }
    fn read_exact_named(
        &mut self,
        len: usize,
        field: &'static str,
    ) -> Result<&'a [u8], DecodeError> {
        let start = self.cursor;
        let end = start
            .checked_add(len)
            .ok_or(DecodeError::ArithmeticOverflow {
                field,
                offset: self.offset(),
            })?;
        let bytes = self
            .bytes
            .get(start..end)
            .ok_or(DecodeError::UnexpectedEof {
                field,
                offset: self.offset(),
                needed: len,
                remaining: self.bytes.len().saturating_sub(start),
            })?;
        self.cursor = end;
        Ok(bytes)
    }
}

pub fn checked_count(value: i32, field: &'static str, offset: u64) -> Result<usize, DecodeError> {
    usize::try_from(value).map_err(|_| DecodeError::InvalidValue {
        field,
        offset,
        value: i64::from(value),
    })
}

pub fn decode_utf16(bytes: &[u8], field: &'static str, offset: u64) -> Result<String, DecodeError> {
    if !bytes.len().is_multiple_of(2) {
        return Err(DecodeError::InvalidValue {
            field,
            offset,
            value: bytes.len() as i64,
        });
    }
    let units = bytes
        .chunks_exact(2)
        .map(|p| u16::from_le_bytes([p[0], p[1]]))
        .collect::<Vec<_>>();
    String::from_utf16(&units).map_err(|_| DecodeError::InvalidUtf16 { field, offset })
}
