//! Decoder and encoder for Siglus CGTABLE and CGTABLE2 files.

use encoding_rs::SHIFT_JIS;

use crate::DecodeError;
use crate::reader::CheckedReader;
use crate::util::{checked_count, checked_size, xor_cycle};

const HEADER_SIZE: usize = 32;
const NAME_SIZE: usize = 32;
const OLD_RECORD_SIZE: usize = 36;
const NEW_RECORD_SIZE: usize = 60;
const TPC_KEY: [u8; 256] = [
    0x8b, 0xe5, 0x5d, 0xc3, 0xa1, 0xe0, 0x30, 0x44, 0x00, 0x85, 0xc0, 0x74, 0x09, 0x5f, 0x5e, 0x33,
    0xc0, 0x5b, 0x8b, 0xe5, 0x5d, 0xc3, 0x8b, 0x45, 0x0c, 0x85, 0xc0, 0x75, 0x14, 0x8b, 0x55, 0xec,
    0x83, 0xc2, 0x20, 0x52, 0x6a, 0x00, 0xe8, 0xf5, 0x28, 0x01, 0x00, 0x83, 0xc4, 0x08, 0x89, 0x45,
    0x0c, 0x8b, 0x45, 0xe4, 0x6a, 0x00, 0x6a, 0x00, 0x50, 0x53, 0xff, 0x15, 0x34, 0xb1, 0x43, 0x00,
    0x8b, 0x45, 0x10, 0x85, 0xc0, 0x74, 0x05, 0x8b, 0x4d, 0xec, 0x89, 0x08, 0x8a, 0x45, 0xf0, 0x84,
    0xc0, 0x75, 0x78, 0xa1, 0xe0, 0x30, 0x44, 0x00, 0x8b, 0x7d, 0xe8, 0x8b, 0x75, 0x0c, 0x85, 0xc0,
    0x75, 0x44, 0x8b, 0x1d, 0xd0, 0xb0, 0x43, 0x00, 0x85, 0xff, 0x76, 0x37, 0x81, 0xff, 0x00, 0x00,
    0x04, 0x00, 0x6a, 0x00, 0x76, 0x43, 0x8b, 0x45, 0xf8, 0x8d, 0x55, 0xfc, 0x52, 0x68, 0x00, 0x00,
    0x04, 0x00, 0x56, 0x50, 0xff, 0x15, 0x2c, 0xb1, 0x43, 0x00, 0x6a, 0x05, 0xff, 0xd3, 0xa1, 0xe0,
    0x30, 0x44, 0x00, 0x81, 0xef, 0x00, 0x00, 0x04, 0x00, 0x81, 0xc6, 0x00, 0x00, 0x04, 0x00, 0x85,
    0xc0, 0x74, 0xc5, 0x8b, 0x5d, 0xf8, 0x53, 0xe8, 0xf4, 0xfb, 0xff, 0xff, 0x8b, 0x45, 0x0c, 0x83,
    0xc4, 0x04, 0x5f, 0x5e, 0x5b, 0x8b, 0xe5, 0x5d, 0xc3, 0x8b, 0x55, 0xf8, 0x8d, 0x4d, 0xfc, 0x51,
    0x57, 0x56, 0x52, 0xff, 0x15, 0x2c, 0xb1, 0x43, 0x00, 0xeb, 0xd8, 0x8b, 0x45, 0xe8, 0x83, 0xc0,
    0x20, 0x50, 0x6a, 0x00, 0xe8, 0x47, 0x28, 0x01, 0x00, 0x8b, 0x7d, 0xe8, 0x89, 0x45, 0xf4, 0x8b,
    0xf0, 0xa1, 0xe0, 0x30, 0x44, 0x00, 0x83, 0xc4, 0x08, 0x85, 0xc0, 0x75, 0x56, 0x8b, 0x1d, 0xd0,
    0xb0, 0x43, 0x00, 0x85, 0xff, 0x76, 0x49, 0x81, 0xff, 0x00, 0x00, 0x04, 0x00, 0x6a, 0x00, 0x76,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CgTableFormat {
    CgTable,
    CgTable2,
}

impl CgTableFormat {
    fn magic(self) -> &'static [u8] {
        match self {
            Self::CgTable => b"CGTABLE",
            Self::CgTable2 => b"CGTABLE2",
        }
    }

    fn record_size(self) -> usize {
        match self {
            Self::CgTable => OLD_RECORD_SIZE,
            Self::CgTable2 => NEW_RECORD_SIZE,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CgTableRecord {
    pub name: String,
    pub flag_no: i32,
    pub codes: [i32; 5],
    pub code_exist_cnt: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CgTable {
    pub format: CgTableFormat,
    pub auto_flag: i32,
    pub reserved: [i32; 2],
    pub records: Vec<CgTableRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CgTableEncodeError {
    #[error("CG table record count {value} exceeds i32")]
    TooManyRecords { value: usize },
    #[error("CG table name `{name}` contains NUL")]
    NameContainsNul { name: String },
    #[error("CG table name `{name}` cannot be encoded as CP932")]
    NameEncoding { name: String },
    #[error("CG table name `{name}` is {length} CP932 bytes; maximum is 31")]
    NameTooLong { name: String, length: usize },
    #[error("CG table {field} is too large")]
    SizeOverflow { field: &'static str },
}

pub fn decode_cg_table(bytes: &[u8]) -> Result<CgTable, DecodeError> {
    let mut reader = CheckedReader::new(bytes);
    let magic = reader.read_exact_named(16, "CG table magic")?;
    let magic_end = magic
        .iter()
        .position(|byte| *byte == 0)
        .ok_or(DecodeError::InvalidValue {
            field: "CG table magic",
            offset: 0,
            value: 0,
        })?;
    let format = match &magic[..magic_end] {
        b"CGTABLE" => CgTableFormat::CgTable,
        b"CGTABLE2" => CgTableFormat::CgTable2,
        _ => {
            return Err(DecodeError::InvalidValue {
                field: "CG table magic",
                offset: 0,
                value: 0,
            });
        }
    };
    let count_offset = reader.offset();
    let count = checked_count(reader.read_i32_le()?, "CG table record count", count_offset)?;
    let auto_flag = reader.read_i32_le()?;
    let reserved = [reader.read_i32_le()?, reader.read_i32_le()?];
    debug_assert_eq!(reader.offset(), HEADER_SIZE as u64);

    let mut compressed = reader.rest().to_vec();
    xor_cycle(&mut compressed, &TPC_KEY);
    let archive_size = compressed.get(..4).ok_or(DecodeError::UnexpectedEof {
        field: "LZSS archive size",
        offset: HEADER_SIZE as u64,
        needed: 4,
        remaining: compressed.len(),
    })?;
    let archive_size = u32::from_le_bytes(archive_size.try_into().unwrap()) as usize;
    if archive_size != compressed.len() {
        return Err(DecodeError::InvalidRange {
            field: "LZSS archive",
            offset: HEADER_SIZE as u64,
            start: 0,
            length: archive_size as u64,
            input_len: compressed.len() as u64,
        });
    }
    let payload = crate::lzss::decode(&compressed)?;
    let expected = checked_size(
        count,
        format.record_size(),
        "CG table record bytes",
        count_offset,
    )?;
    if payload.len() != expected {
        return Err(DecodeError::InvalidRange {
            field: "CG table record bytes",
            offset: HEADER_SIZE as u64,
            start: 0,
            length: u64::try_from(expected).unwrap_or(u64::MAX),
            input_len: u64::try_from(payload.len()).unwrap_or(u64::MAX),
        });
    }

    let mut payload = CheckedReader::from_slice(&payload, HEADER_SIZE as u64);
    let mut records = Vec::with_capacity(count);
    for _ in 0..count {
        let name_offset = payload.offset();
        let name = decode_name(
            payload.read_exact_named(NAME_SIZE, "CG table name")?,
            name_offset,
        )?;
        let flag_no = payload.read_i32_le()?;
        let (codes, code_exist_cnt) = match format {
            CgTableFormat::CgTable => ([0; 5], 0),
            CgTableFormat::CgTable2 => (
                [
                    payload.read_i32_le()?,
                    payload.read_i32_le()?,
                    payload.read_i32_le()?,
                    payload.read_i32_le()?,
                    payload.read_i32_le()?,
                ],
                payload.read_i32_le()?,
            ),
        };
        records.push(CgTableRecord {
            name,
            flag_no,
            codes,
            code_exist_cnt,
        });
    }
    Ok(CgTable {
        format,
        auto_flag,
        reserved,
        records,
    })
}

pub fn encode_cg_table(table: &CgTable) -> Result<Vec<u8>, CgTableEncodeError> {
    let count =
        i32::try_from(table.records.len()).map_err(|_| CgTableEncodeError::TooManyRecords {
            value: table.records.len(),
        })?;
    let record_size = table.format.record_size();
    let payload_size =
        table
            .records
            .len()
            .checked_mul(record_size)
            .ok_or(CgTableEncodeError::SizeOverflow {
                field: "record bytes",
            })?;
    let mut payload = Vec::with_capacity(payload_size);
    for record in &table.records {
        encode_name(&mut payload, &record.name)?;
        payload.extend_from_slice(&record.flag_no.to_le_bytes());
        if table.format == CgTableFormat::CgTable2 {
            for code in record.codes {
                payload.extend_from_slice(&code.to_le_bytes());
            }
            payload.extend_from_slice(&record.code_exist_cnt.to_le_bytes());
        }
    }
    let mut compressed =
        crate::lzss::encode(&payload).map_err(|_| CgTableEncodeError::SizeOverflow {
            field: "compressed bytes",
        })?;
    xor_cycle(&mut compressed, &TPC_KEY);
    let capacity =
        HEADER_SIZE
            .checked_add(compressed.len())
            .ok_or(CgTableEncodeError::SizeOverflow {
                field: "file bytes",
            })?;
    let mut output = Vec::with_capacity(capacity);
    let mut magic = [0; 16];
    magic[..table.format.magic().len()].copy_from_slice(table.format.magic());
    output.extend_from_slice(&magic);
    output.extend_from_slice(&count.to_le_bytes());
    output.extend_from_slice(&table.auto_flag.to_le_bytes());
    output.extend_from_slice(&table.reserved[0].to_le_bytes());
    output.extend_from_slice(&table.reserved[1].to_le_bytes());
    output.extend_from_slice(&compressed);
    Ok(output)
}

fn decode_name(bytes: &[u8], offset: u64) -> Result<String, DecodeError> {
    let Some(end) = bytes.iter().position(|byte| *byte == 0) else {
        return Err(DecodeError::InvalidValue {
            field: "CG table name",
            offset,
            value: NAME_SIZE as i64,
        });
    };
    let (name, _, had_errors) = SHIFT_JIS.decode(&bytes[..end]);
    if had_errors {
        return Err(DecodeError::InvalidValue {
            field: "CG table CP932 name",
            offset,
            value: 0,
        });
    }
    Ok(name.into_owned())
}

fn encode_name(output: &mut Vec<u8>, name: &str) -> Result<(), CgTableEncodeError> {
    if name.contains('\0') {
        return Err(CgTableEncodeError::NameContainsNul {
            name: name.to_owned(),
        });
    }
    let (encoded, _, had_errors) = SHIFT_JIS.encode(name);
    if had_errors {
        return Err(CgTableEncodeError::NameEncoding {
            name: name.to_owned(),
        });
    }
    if encoded.len() >= NAME_SIZE {
        return Err(CgTableEncodeError::NameTooLong {
            name: name.to_owned(),
            length: encoded.len(),
        });
    }
    output.extend_from_slice(&encoded);
    output.resize(output.len() + NAME_SIZE - encoded.len(), 0);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(format: CgTableFormat) -> CgTable {
        CgTable {
            format,
            auto_flag: 1,
            reserved: [2, 3],
            records: vec![CgTableRecord {
                name: "夏".into(),
                flag_no: 4,
                codes: [5, 6, 7, 8, 9],
                code_exist_cnt: 5,
            }],
        }
    }

    #[test]
    fn round_trips_both_formats() {
        for format in [CgTableFormat::CgTable, CgTableFormat::CgTable2] {
            let value = table(format);
            let decoded = decode_cg_table(&encode_cg_table(&value).unwrap()).unwrap();
            let mut expected = value;
            if format == CgTableFormat::CgTable {
                expected.records[0].codes = [0; 5];
                expected.records[0].code_exist_cnt = 0;
            }
            assert_eq!(decoded, expected);
        }
    }

    #[test]
    fn rejects_unterminated_name_and_invalid_count() {
        let mut value = table(CgTableFormat::CgTable);
        value.records[0].name = "a".repeat(32);
        assert!(matches!(
            encode_cg_table(&value),
            Err(CgTableEncodeError::NameTooLong { .. })
        ));

        let mut bytes = encode_cg_table(&table(CgTableFormat::CgTable)).unwrap();
        bytes[16..20].copy_from_slice(&(-1_i32).to_le_bytes());
        assert!(matches!(
            decode_cg_table(&bytes),
            Err(DecodeError::InvalidValue { .. })
        ));

        bytes[16..20].copy_from_slice(&1_000_001_i32.to_le_bytes());
        assert!(matches!(
            decode_cg_table(&bytes),
            Err(DecodeError::InvalidRange {
                field: "CG table record bytes",
                ..
            })
        ));
    }
}
