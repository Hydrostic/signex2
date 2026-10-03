//! Header reader for Siglus OMV movie containers.
//!
//! The fixed 168-byte header precedes 28-byte Theora page records and 32-byte
//! packet records. The stream starts after both lists. This module reads only
//! the fixed header, so callers can seek in the original file for decoding.

use crate::reader::{CheckedReader, ReadError};

/// Fixed OMV header size in bytes.
pub const OMV_HEADER_SIZE: usize = 168;
/// Size of one OMV page-list record.
pub const OMV_PAGE_SIZE: usize = 28;
/// Size of one OMV packet-list record.
pub const OMV_PACKET_SIZE: usize = 32;

/// How the three Theora planes are interpreted for the logical image.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OmvTheoraType {
    /// Plane bytes are written directly to B/G/R slots; alpha = 255.
    Rgb,
    /// Like RGB, with alpha packed in extra rows below each plane.
    Rgba,
    /// Planes are Y/U/V, converted to B/G/R; alpha = 255.
    Yuv,
}

impl OmvTheoraType {
    /// The value stored in the OMV header.
    pub const fn code(self) -> i32 {
        match self {
            Self::Rgb => 0,
            Self::Rgba => 1,
            Self::Yuv => 2,
        }
    }
}

/// Parsed fixed OMV header fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OmvHeader {
    pub header_size: i32,
    pub major: u8,
    pub minor: u8,
    pub theora_type: OmvTheoraType,
    pub width: i32,
    pub height: i32,
    pub center_x: i32,
    pub center_y: i32,
    pub us_per_frame: i32,
    pub theora_serial: i32,
    pub theora_header_page: i32,
    pub theora_subheader_page: i32,
    pub page_count: i32,
    pub packet_count: i32,
}

impl OmvHeader {
    /// Logical image width.
    pub const fn width(self) -> i32 {
        self.width
    }

    /// Logical image height.
    pub const fn height(self) -> i32 {
        self.height
    }
}

/// Header metadata and the byte offset of the Ogg stream in the original file.
///
/// Parsing the header does not validate the page or packet lists or the Ogg
/// stream. The offset can lie beyond the supplied slice, which may contain
/// just the fixed header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OmvContainer {
    header: OmvHeader,
    data_offset: u64,
}

impl OmvContainer {
    /// Parsed fixed OMV header.
    pub const fn header(&self) -> &OmvHeader {
        &self.header
    }

    /// Byte offset of the Ogg stream in the original OMV file.
    pub const fn data_offset(&self) -> u64 {
        self.data_offset
    }
}

/// Errors while parsing the fixed OMV header.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum OmvError {
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
    #[error("arithmetic overflow in `{field}` at byte {offset}")]
    ArithmeticOverflow { field: &'static str, offset: u64 },
}

impl From<ReadError> for OmvError {
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

/// Parse the fixed OMV header and calculate the following Ogg stream offset.
///
/// Only the first [`OMV_HEADER_SIZE`] bytes are read. No caller-provided input,
/// output, or item limit is imposed; the counts are checked for nonnegativity
/// and offset arithmetic is checked for overflow.
pub fn parse_omv(bytes: &[u8]) -> Result<OmvContainer, OmvError> {
    let mut reader = CheckedReader::new(bytes);
    let header_size = reader.read_i32_le()?;
    let major = reader.read_u8()?;
    let minor = reader.read_u8()?;
    reader.read_exact_named(2, "OMV dummy")?;
    reader.read_exact_named(32, "OMV keep00")?;
    let theora_type = match reader.read_i32_le()? {
        0 => OmvTheoraType::Rgb,
        1 => OmvTheoraType::Rgba,
        2 => OmvTheoraType::Yuv,
        value => {
            return Err(OmvError::InvalidValue {
                field: "OMV Theora type",
                offset: 40,
                value: i64::from(value),
            });
        }
    };
    let width = reader.read_i32_le()?;
    let height = reader.read_i32_le()?;
    let center_x = reader.read_i32_le()?;
    let center_y = reader.read_i32_le()?;
    let us_per_frame = reader.read_i32_le()?;
    let theora_serial = reader.read_i32_le()?;
    let theora_header_page = reader.read_i32_le()?;
    let theora_subheader_page = reader.read_i32_le()?;
    let page_count = reader.read_i32_le()?;
    let packet_count = reader.read_i32_le()?;
    reader.read_exact_named(32, "OMV keep01")?;
    reader.read_exact_named(20, "OMV Vorbis fields")?;
    reader.read_exact_named(32, "OMV keep02")?;

    if header_size != OMV_HEADER_SIZE as i32 {
        return Err(OmvError::InvalidValue {
            field: "OMV header size",
            offset: 0,
            value: i64::from(header_size),
        });
    }
    if major != 1 || minor != 1 {
        return Err(OmvError::InvalidValue {
            field: "OMV version",
            offset: 4,
            value: i64::from(major) * 10 + i64::from(minor),
        });
    }
    if width <= 0 || height <= 0 {
        return Err(OmvError::InvalidValue {
            field: "OMV logical size",
            offset: 44,
            value: i64::from(width) * 1_000_000 + i64::from(height),
        });
    }
    if matches!(theora_type, OmvTheoraType::Rgba) && height < 2 {
        return Err(OmvError::InvalidValue {
            field: "OMV RGBA height",
            offset: 48,
            value: i64::from(height),
        });
    }

    let page_count = u64::try_from(page_count).map_err(|_| OmvError::InvalidValue {
        field: "OMV pages",
        offset: 76,
        value: i64::from(page_count),
    })?;
    let packet_count = u64::try_from(packet_count).map_err(|_| OmvError::InvalidValue {
        field: "OMV packets",
        offset: 80,
        value: i64::from(packet_count),
    })?;
    let page_bytes =
        page_count
            .checked_mul(OMV_PAGE_SIZE as u64)
            .ok_or(OmvError::ArithmeticOverflow {
                field: "OMV page list",
                offset: 76,
            })?;
    let packet_bytes =
        packet_count
            .checked_mul(OMV_PACKET_SIZE as u64)
            .ok_or(OmvError::ArithmeticOverflow {
                field: "OMV packet list",
                offset: 80,
            })?;
    let data_offset = (OMV_HEADER_SIZE as u64)
        .checked_add(page_bytes)
        .and_then(|offset| offset.checked_add(packet_bytes))
        .ok_or(OmvError::ArithmeticOverflow {
            field: "OMV data offset",
            offset: 76,
        })?;

    Ok(OmvContainer {
        header: OmvHeader {
            header_size,
            major,
            minor,
            theora_type,
            width,
            height,
            center_x,
            center_y,
            us_per_frame,
            theora_serial,
            theora_header_page,
            theora_subheader_page,
            page_count: page_count as i32,
            packet_count: packet_count as i32,
        },
        data_offset,
    })
}
