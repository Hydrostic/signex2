//! OVK voice-pack reader.
//!
//! Wire contract (evidence: `SiglusEngine/Siglus/Source/elm_sound_player.cpp`
//! and `siglus-analysis-docs/resource_system.md` 4.2):
//!
//! `u32le count @0`, then `count` fixed 16-byte entries
//! `{u32 size, u32 offset, i32 no, i32 smp_cnt}`. Voice data is located by
//! linear search for `entry.no == line_no`, where
//! `line_no = koe_no % 100000`; the matching `(offset, size)` is an Ogg/Vorbis
//! substream of the same file (XOR 0).
//!
//! The legacy reader neither bounds the table nor the entry ranges; this reader
//! validates `count * 16` table bytes and every entry range before exposing any
//! substream. `smp_cnt` is informational only.

use super::AudioDecodeError as DecodeError;
use crate::reader::CheckedReader;
use crate::util::checked_size;

/// One 16-byte OVK voice entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OvkEntry {
    /// Byte size of the voice substream.
    pub size: u32,
    /// Byte offset of the voice substream, relative to the start of the OVK file.
    pub offset: u32,
    /// Voice line number (`koe_no % 100000`).
    pub no: i32,
    /// Encoder sample count (informational; not used for playback).
    pub sample_count: i32,
}

impl OvkEntry {
    /// Byte range of this entry within the OVK file, checked.
    pub fn range(&self) -> Option<core::ops::Range<usize>> {
        let start = usize::try_from(self.offset).ok()?;
        let size = usize::try_from(self.size).ok()?;
        Some(start..start.checked_add(size)?)
    }
}

/// Parsed OVK index. Owns no audio bytes; substreams are borrowed views.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OvkIndex {
    entries: Vec<OvkEntry>,
}

impl OvkIndex {
    /// The parsed entries.
    pub fn entries(&self) -> &[OvkEntry] {
        &self.entries
    }

    /// Find the first entry whose `no` equals `line_no`.
    pub fn find(&self, line_no: i32) -> Option<&OvkEntry> {
        self.entries.iter().find(|entry| entry.no == line_no)
    }

    /// Borrow the substream for `line_no`, bounded to the input bytes.
    ///
    /// Returns [`DecodeError::InvalidValue`] when no entry matches and
    /// [`DecodeError::InvalidRange`] when the entry range escapes the input.
    pub fn substream<'a>(&self, bytes: &'a [u8], line_no: i32) -> Result<&'a [u8], DecodeError> {
        let entry = self.find(line_no).ok_or(DecodeError::InvalidValue {
            field: "OVK line number",
            offset: 4,
            value: i64::from(line_no),
        })?;
        let start = usize::try_from(entry.offset).map_err(|_| DecodeError::ArithmeticOverflow {
            field: "OVK substream offset",
            offset: 4,
        })?;
        let size = usize::try_from(entry.size).map_err(|_| DecodeError::ArithmeticOverflow {
            field: "OVK substream size",
            offset: 4,
        })?;
        let end = start
            .checked_add(size)
            .ok_or(DecodeError::ArithmeticOverflow {
                field: "OVK substream range",
                offset: 4,
            })?;
        bytes.get(start..end).ok_or(DecodeError::InvalidRange {
            field: "OVK substream",
            offset: 4,
            start: u64::from(entry.offset),
            length: u64::from(entry.size),
            input_len: u64::try_from(bytes.len()).unwrap_or(u64::MAX),
        })
    }
}

/// Parse and validate an OVK file: `count * 16` table bytes plus every entry
/// range. `count == 0` is legal (an empty voice pack).
pub fn parse_ovk(bytes: &[u8]) -> Result<OvkIndex, DecodeError> {
    let mut reader = CheckedReader::new(bytes);
    let count = u64::from(reader.read_u32_le()?);
    let count_usize = usize::try_from(count).map_err(|_| DecodeError::ArithmeticOverflow {
        field: "OVK entries",
        offset: 0,
    })?;
    let table_size = checked_size(count_usize, 16, "OVK entry table", 4)?;
    if table_size > reader.remaining() {
        return Err(DecodeError::UnexpectedEof {
            field: "OVK entry table",
            offset: 4,
            needed: table_size,
            remaining: reader.remaining(),
        });
    }
    let mut entries = Vec::new();
    entries
        .try_reserve_exact(count_usize)
        .map_err(|_| DecodeError::AllocationFailed {
            field: "OVK entries",
        })?;
    for _ in 0..count_usize {
        let entry = OvkEntry {
            size: reader.read_u32_le()?,
            offset: reader.read_u32_le()?,
            no: reader.read_i32_le()?,
            sample_count: reader.read_i32_le()?,
        };
        let start = usize::try_from(entry.offset).map_err(|_| DecodeError::ArithmeticOverflow {
            field: "OVK data offset",
            offset: reader.offset() - 16,
        })?;
        let size = usize::try_from(entry.size).map_err(|_| DecodeError::ArithmeticOverflow {
            field: "OVK data size",
            offset: reader.offset() - 16,
        })?;
        let end = start
            .checked_add(size)
            .ok_or(DecodeError::ArithmeticOverflow {
                field: "OVK data range",
                offset: reader.offset() - 16,
            })?;
        if end > bytes.len() {
            return Err(DecodeError::InvalidRange {
                field: "OVK entry",
                offset: reader.offset() - 16,
                start: u64::from(entry.offset),
                length: u64::from(entry.size),
                input_len: u64::try_from(bytes.len()).unwrap_or(u64::MAX),
            });
        }
        entries.push(entry);
    }
    Ok(OvkIndex { entries })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ovk(entries: &[(u32, u32, i32, i32)], payload: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&u32::try_from(entries.len()).unwrap().to_le_bytes());
        for (size, offset, no, smp) in entries {
            bytes.extend_from_slice(&size.to_le_bytes());
            bytes.extend_from_slice(&offset.to_le_bytes());
            bytes.extend_from_slice(&no.to_le_bytes());
            bytes.extend_from_slice(&smp.to_le_bytes());
        }
        bytes.extend_from_slice(payload);
        bytes
    }

    #[test]
    fn parses_and_finds_entries_and_substreams() {
        let payload = b"OggS-vorbis...";
        let table_end = 4 + 2 * 16;
        let bytes = ovk(
            &[
                (10, table_end as u32, 11, 1000),
                (payload.len() as u32 - 10, (table_end + 10) as u32, 16, 2000),
            ],
            payload,
        );
        let index = parse_ovk(&bytes).unwrap();
        assert_eq!(index.entries().len(), 2);
        assert_eq!(index.find(11).unwrap().no, 11);
        assert!(index.find(999).is_none());
        assert_eq!(index.substream(&bytes, 16).unwrap(), &payload[10..]);
        assert!(matches!(
            index.substream(&bytes, 999),
            Err(DecodeError::InvalidValue { field, .. }) if field.contains("line number")
        ));
    }

    #[test]
    fn empty_pack_is_legal_and_never_matches() {
        let bytes = ovk(&[], &[]);
        let index = parse_ovk(&bytes).unwrap();
        assert!(index.entries().is_empty());
        assert!(index.find(1).is_none());
    }

    #[test]
    fn rejects_truncated_table() {
        // count says 3 but only one 16B entry present.
        let mut bytes = Vec::from(3_u32.to_le_bytes());
        bytes.extend_from_slice(&[0; 16]);
        assert!(matches!(
            parse_ovk(&bytes),
            Err(DecodeError::UnexpectedEof { field, .. }) if field.contains("entry table")
        ));
    }

    #[test]
    fn rejects_entry_range_escape() {
        // Entry claims bytes beyond the file.
        let bytes = ovk(&[(0xFFFF, 0xFFFF, 1, 0)], b"");
        assert!(matches!(
            parse_ovk(&bytes),
            Err(DecodeError::InvalidRange { field, .. }) if field.contains("entry")
        ));
        // A valid entry works, but the same index against a shorter buffer
        // must fail the substream range check instead of panicking.
        let bytes = ovk(&[(8, 20, 1, 0)], &[0; 8]);
        let index = parse_ovk(&bytes).unwrap();
        assert_eq!(index.substream(&bytes, 1).unwrap(), &[0; 8]);
        let short = &bytes[..4]; // entry range [20,28) escapes this buffer
        assert!(matches!(
            index.substream(short, 1),
            Err(DecodeError::InvalidRange { .. })
        ));
    }

    #[test]
    fn rejects_entry_count_exceeding_input_before_allocating() {
        let mut bytes = Vec::from(1_000_001_u32.to_le_bytes());
        bytes.extend_from_slice(&[0; 16]);
        assert!(matches!(
            parse_ovk(&bytes),
            Err(DecodeError::UnexpectedEof { field, .. }) if field.contains("entry table")
        ));
    }
}
