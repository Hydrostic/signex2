//! Decoder for Siglus DBS tables.

use encoding_rs::SHIFT_JIS;

use crate::DecodeError;
use crate::reader::CheckedReader;
use crate::util::{checked_count, checked_size, slice_at, validate_range, xor_words};

const ROW_SIZE: usize = 4;
const COLUMN_SIZE: usize = 8;
const TILE_ROW_SIZE: usize = 64;
const OUTER_XOR: u32 = 0x89f4_622d;
const TILE_XOR_A: u32 = 0x7190_c70e;
const TILE_XOR_B: u32 = 0x499b_f135;
const TILE_MASK: [[bool; 5]; 5] = [
    [true, false, false, true, true],
    [false, false, true, true, false],
    [true, true, true, false, true],
    [false, false, true, false, false],
    [false, false, false, true, true],
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatabaseColumnType {
    Number,
    String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatabaseColumn {
    pub call_no: i32,
    pub kind: DatabaseColumnType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DatabaseCell {
    Number(i32),
    String(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatabaseRow {
    pub call_no: i32,
    pub cells: Vec<DatabaseCell>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Database {
    pub columns: Vec<DatabaseColumn>,
    pub rows: Vec<DatabaseRow>,
}

impl Database {
    pub fn column_index(&self, call_no: i32) -> Option<usize> {
        self.columns
            .iter()
            .position(|column| column.call_no == call_no)
    }

    pub fn row_index(&self, call_no: i32) -> Option<usize> {
        self.rows.iter().position(|row| row.call_no == call_no)
    }

    pub fn cell(&self, row_call_no: i32, column_call_no: i32) -> Option<&DatabaseCell> {
        self.rows
            .get(self.row_index(row_call_no)?)?
            .cells
            .get(self.column_index(column_call_no)?)
    }
}

pub fn decode_database(bytes: &[u8]) -> Result<Database, DecodeError> {
    let mut reader = CheckedReader::new(bytes);
    let string_type = reader.read_i32_le()?;
    let mut packed = reader.rest().to_vec();
    xor_words(&mut packed, OUTER_XOR);
    let archive_size = packed.get(..4).ok_or(DecodeError::UnexpectedEof {
        field: "LZSS archive size",
        offset: 4,
        needed: 4,
        remaining: packed.len(),
    })?;
    let archive_size = u32::from_le_bytes(archive_size.try_into().unwrap()) as usize;
    if archive_size != packed.len() {
        return Err(DecodeError::InvalidRange {
            field: "LZSS archive",
            offset: 4,
            start: 0,
            length: archive_size as u64,
            input_len: packed.len() as u64,
        });
    }
    let mut plaintext = crate::lzss::decode(&packed)?;
    if !plaintext.len().is_multiple_of(TILE_ROW_SIZE) {
        return Err(DecodeError::InvalidValue {
            field: "DBS tile bytes",
            offset: 4,
            value: i64::try_from(plaintext.len()).unwrap_or(i64::MAX),
        });
    }
    restore_tiles(&mut plaintext);
    parse_plaintext(&plaintext, string_type)
}

fn parse_plaintext(bytes: &[u8], string_type: i32) -> Result<Database, DecodeError> {
    let mut reader = CheckedReader::new(bytes);
    let data_size = reader.read_i32_le()?;
    if data_size < 0 || usize::try_from(data_size).ok() != Some(bytes.len()) {
        return Err(DecodeError::InvalidValue {
            field: "DBS data size",
            offset: 0,
            value: i64::from(data_size),
        });
    }
    let row_offset = reader.offset();
    let row_count = checked_count(reader.read_i32_le()?, "DBS row count", row_offset)?;
    let column_offset = reader.offset();
    let column_count = checked_count(reader.read_i32_le()?, "DBS column count", column_offset)?;
    let row_headers = checked_count(reader.read_i32_le()?, "DBS row headers", 12)?;
    let column_headers = checked_count(reader.read_i32_le()?, "DBS column headers", 16)?;
    let data_offset = checked_count(reader.read_i32_le()?, "DBS data", 20)?;
    let string_offset = checked_count(reader.read_i32_le()?, "DBS strings", 24)?;
    let row_bytes = checked_size(row_count, ROW_SIZE, "DBS row headers", 12)?;
    let column_bytes = checked_size(column_count, COLUMN_SIZE, "DBS column headers", 16)?;
    let cells = row_count
        .checked_mul(column_count)
        .ok_or(DecodeError::ArithmeticOverflow {
            field: "DBS cells",
            offset: 20,
        })?;
    let data_bytes = checked_size(cells, 4, "DBS data", 20)?;
    validate_range(bytes, row_headers, row_bytes, "DBS row headers")?;
    validate_range(bytes, column_headers, column_bytes, "DBS column headers")?;
    validate_range(bytes, data_offset, data_bytes, "DBS data")?;
    if string_offset > bytes.len() {
        return Err(DecodeError::InvalidRange {
            field: "DBS strings",
            offset: 24,
            start: u64::try_from(string_offset).unwrap_or(u64::MAX),
            length: 0,
            input_len: u64::try_from(bytes.len()).unwrap_or(u64::MAX),
        });
    }

    let mut row_reader = slice_at(bytes, row_headers, row_bytes, "DBS row headers")?;
    let mut rows = Vec::with_capacity(row_count);
    for _ in 0..row_count {
        rows.push(DatabaseRow {
            call_no: row_reader.read_i32_le()?,
            cells: Vec::with_capacity(column_count),
        });
    }
    let mut column_reader = slice_at(bytes, column_headers, column_bytes, "DBS column headers")?;
    let mut columns = Vec::with_capacity(column_count);
    for _ in 0..column_count {
        let call_no = column_reader.read_i32_le()?;
        let kind = match column_reader.read_i32_le()? {
            0x56 => DatabaseColumnType::Number,
            0x53 => DatabaseColumnType::String,
            value => {
                return Err(DecodeError::InvalidValue {
                    field: "DBS column type",
                    offset: column_reader.offset() - 4,
                    value: i64::from(value),
                });
            }
        };
        columns.push(DatabaseColumn { call_no, kind });
    }
    let mut data_reader = slice_at(bytes, data_offset, data_bytes, "DBS data")?;
    for row in &mut rows {
        for column in &columns {
            let value = data_reader.read_i32_le()?;
            row.cells.push(match column.kind {
                DatabaseColumnType::Number => DatabaseCell::Number(value),
                DatabaseColumnType::String => {
                    DatabaseCell::String(read_string(bytes, string_offset, value, string_type)?)
                }
            });
        }
    }
    Ok(Database { columns, rows })
}

fn read_string(
    bytes: &[u8],
    base: usize,
    value: i32,
    string_type: i32,
) -> Result<String, DecodeError> {
    let offset = usize::try_from(value).map_err(|_| DecodeError::InvalidValue {
        field: "DBS string offset",
        offset: base as u64,
        value: i64::from(value),
    })?;
    let start = base
        .checked_add(offset)
        .ok_or(DecodeError::ArithmeticOverflow {
            field: "DBS string offset",
            offset: base as u64,
        })?;
    let value = bytes.get(start..).ok_or(DecodeError::InvalidRange {
        field: "DBS string offset",
        offset: base as u64,
        start: offset as u64,
        length: 1,
        input_len: bytes.len() as u64,
    })?;
    if string_type == 0 {
        let end = value
            .iter()
            .position(|byte| *byte == 0)
            .ok_or(DecodeError::InvalidValue {
                field: "DBS ACP string",
                offset: start as u64,
                value: 0,
            })?;
        let (text, _, had_errors) = SHIFT_JIS.decode(&value[..end]);
        if had_errors {
            return Err(DecodeError::InvalidValue {
                field: "DBS ACP string",
                offset: start as u64,
                value: 0,
            });
        }
        Ok(text.into_owned())
    } else {
        let end = value
            .chunks_exact(2)
            .position(|unit| unit == [0, 0])
            .ok_or(DecodeError::InvalidValue {
                field: "DBS UTF-16 string",
                offset: start as u64,
                value: 0,
            })?;
        let units = value[..end * 2]
            .chunks_exact(2)
            .map(|unit| u16::from_le_bytes([unit[0], unit[1]]));
        String::from_utf16(&units.collect::<Vec<_>>()).map_err(|_| DecodeError::InvalidUtf16 {
            field: "DBS UTF-16 string",
            offset: start as u64,
        })
    }
}

fn restore_tiles(bytes: &mut [u8]) {
    for (index, word) in bytes.chunks_exact_mut(4).enumerate() {
        let x = index % 16;
        let y = index / 16;
        let key = if TILE_MASK[y % 5][x % 5] {
            TILE_XOR_A
        } else {
            TILE_XOR_B
        };
        let value = u32::from_le_bytes(word.try_into().expect("four bytes")) ^ key;
        word.copy_from_slice(&value.to_le_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encoded_database(string_type: i32) -> Vec<u8> {
        let mut plaintext = vec![0; TILE_ROW_SIZE];
        for (offset, value) in [
            (0, TILE_ROW_SIZE as i32),
            (4, 1),
            (8, 2),
            (12, 28),
            (16, 32),
            (20, 48),
            (24, 56),
        ] {
            plaintext[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        }
        plaintext[28..32].copy_from_slice(&7_i32.to_le_bytes());
        plaintext[32..36].copy_from_slice(&10_i32.to_le_bytes());
        plaintext[36..40].copy_from_slice(&0x56_i32.to_le_bytes());
        plaintext[40..44].copy_from_slice(&20_i32.to_le_bytes());
        plaintext[44..48].copy_from_slice(&0x53_i32.to_le_bytes());
        plaintext[48..52].copy_from_slice(&99_i32.to_le_bytes());
        if string_type == 2 {
            plaintext[52..56].copy_from_slice(&1_i32.to_le_bytes());
        }
        if string_type == 0 {
            plaintext[56..58].copy_from_slice(b"x\0");
        } else if string_type == 2 {
            plaintext[57..61].copy_from_slice(&[b'x', 0, 0, 0]);
        } else {
            plaintext[56..60].copy_from_slice(&[b'x', 0, 0, 0]);
        }
        restore_tiles(&mut plaintext);
        let mut packed = crate::lzss::encode(&plaintext).unwrap();
        xor_words(&mut packed, OUTER_XOR);
        let mut bytes = string_type.to_le_bytes().to_vec();
        bytes.extend_from_slice(&packed);
        bytes
    }

    #[test]
    fn decodes_cp932_and_utf16_tables() {
        for string_type in [0, 1, 2] {
            let database = decode_database(&encoded_database(string_type)).unwrap();
            assert_eq!(database.columns.len(), 2);
            assert_eq!(database.rows[0].call_no, 7);
            assert_eq!(database.cell(7, 10), Some(&DatabaseCell::Number(99)));
            assert_eq!(
                database.cell(7, 20),
                Some(&DatabaseCell::String("x".into()))
            );
        }
    }

    #[test]
    fn rejects_short_header_and_invalid_archive() {
        assert!(matches!(
            decode_database(&[0; 3]),
            Err(DecodeError::UnexpectedEof { .. })
        ));
        let mut bytes = encoded_database(0);
        bytes[4..8].copy_from_slice(&12_u32.to_le_bytes());
        assert!(decode_database(&bytes).is_err());
    }

    #[test]
    fn count_above_old_policy_limit_is_checked_against_file_bytes() {
        let mut plaintext = vec![0; TILE_ROW_SIZE];
        plaintext[..4].copy_from_slice(&(TILE_ROW_SIZE as i32).to_le_bytes());
        plaintext[4..8].copy_from_slice(&1_000_001_i32.to_le_bytes());
        plaintext[12..16].copy_from_slice(&28_i32.to_le_bytes());
        assert!(matches!(
            parse_plaintext(&plaintext, 0),
            Err(DecodeError::InvalidRange {
                field: "DBS row headers",
                ..
            })
        ));
    }
}
