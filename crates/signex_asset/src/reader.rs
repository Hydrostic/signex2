//! Bounded little-endian reads shared by asset decoders.

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum ReadError {
    #[error(
        "unexpected EOF while reading `{field}` at byte {offset}: needed {needed}, {remaining} remaining"
    )]
    UnexpectedEof {
        field: &'static str,
        offset: u64,
        needed: usize,
        remaining: usize,
    },
    #[error("arithmetic overflow in `{field}` at byte {offset}")]
    ArithmeticOverflow { field: &'static str, offset: u64 },
}

pub(crate) struct CheckedReader<'a> {
    bytes: &'a [u8],
    cursor: usize,
    base: u64,
}

impl<'a> CheckedReader<'a> {
    pub(crate) fn new(bytes: &'a [u8]) -> Self {
        Self::from_slice(bytes, 0)
    }

    pub(crate) fn from_slice(bytes: &'a [u8], base: u64) -> Self {
        Self {
            bytes,
            cursor: 0,
            base,
        }
    }

    pub(crate) fn offset(&self) -> u64 {
        self.base + self.cursor as u64
    }

    pub(crate) fn remaining(&self) -> usize {
        self.bytes.len() - self.cursor
    }

    pub(crate) fn rest(&self) -> &'a [u8] {
        &self.bytes[self.cursor..]
    }

    pub(crate) fn read_exact_named(
        &mut self,
        len: usize,
        field: &'static str,
    ) -> Result<&'a [u8], ReadError> {
        let start = self.cursor;
        let end = start
            .checked_add(len)
            .ok_or(ReadError::ArithmeticOverflow {
                field,
                offset: self.offset(),
            })?;
        let bytes = self.bytes.get(start..end).ok_or(ReadError::UnexpectedEof {
            field,
            offset: self.offset(),
            needed: len,
            remaining: self.bytes.len() - start,
        })?;
        self.cursor = end;
        Ok(bytes)
    }

    pub(crate) fn read_u8(&mut self) -> Result<u8, ReadError> {
        Ok(self.read_exact_named(1, "u8")?[0])
    }

    pub(crate) fn read_u16_le(&mut self) -> Result<u16, ReadError> {
        Ok(u16::from_le_bytes(
            self.read_exact_named(2, "u16")?.try_into().unwrap(),
        ))
    }

    pub(crate) fn read_u32_le(&mut self) -> Result<u32, ReadError> {
        Ok(u32::from_le_bytes(
            self.read_exact_named(4, "u32")?.try_into().unwrap(),
        ))
    }

    pub(crate) fn read_i32_le(&mut self) -> Result<i32, ReadError> {
        Ok(i32::from_le_bytes(
            self.read_exact_named(4, "i32")?.try_into().unwrap(),
        ))
    }
}
