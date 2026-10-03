//! CPU decoder for the four G00 image kinds.

use std::io::Cursor;

use jpeg_decoder::{Decoder as JpegDecoder, PixelFormat};

use crate::lzss::{decode as decode_lzss, decode_g00_opaque};
use crate::reader::{CheckedReader, ReadError};

/// The legacy G00 storage kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum G00Kind {
    Opaque,
    Paletted,
    Cut,
    Jpeg,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct G00Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct G00Image {
    pub kind: G00Kind,
    pub width: u32,
    pub height: u32,
    pub cuts: Vec<G00Cut>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct G00Cut {
    pub width: u32,
    pub height: u32,
    pub center: (i32, i32),
    pub rect: G00Rect,
    pub chips: Vec<G00Chip>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct G00Chip {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub sprite: bool,
    /// `width * height * 4` bytes in BGRA order.
    pub pixels: Vec<u8>,
}

#[derive(Debug, thiserror::Error)]
pub enum G00Error {
    #[error("truncated {field} at byte {offset}")]
    Truncated { field: &'static str, offset: u64 },
    #[error("invalid {field} at byte {offset}")]
    InvalidValue { field: &'static str, offset: u64 },
    #[error("invalid {field} range at byte {offset}")]
    InvalidRange { field: &'static str, offset: u64 },
    #[error("{field} size overflows at byte {offset}")]
    Overflow { field: &'static str, offset: u64 },
    #[error("unsupported G00 kind {0}")]
    UnsupportedKind(u8),
    #[error("unsupported G00 JPEG pixel format")]
    UnsupportedJpegFormat,
    #[error(transparent)]
    Lzss(#[from] crate::lzss::LzssError),
    #[error("invalid G00 JPEG: {0}")]
    Jpeg(#[from] jpeg_decoder::Error),
}

impl From<ReadError> for G00Error {
    fn from(error: ReadError) -> Self {
        match error {
            ReadError::UnexpectedEof { field, offset, .. } => Self::Truncated { field, offset },
            ReadError::ArithmeticOverflow { field, offset } => Self::Overflow { field, offset },
        }
    }
}

fn checked_usize_product(
    left: usize,
    right: usize,
    field: &'static str,
    offset: u64,
) -> Result<usize, G00Error> {
    left.checked_mul(right)
        .ok_or(G00Error::Overflow { field, offset })
}

fn checked_bgra_pixel_byte_length(width: u32, height: u32, offset: u64) -> Result<usize, G00Error> {
    let pixel_count = checked_usize_product(width as usize, height as usize, "pixel area", offset)?;
    checked_usize_product(pixel_count, 4, "pixel bytes", offset)
}

impl G00Cut {
    fn from_full_image(width: u32, height: u32, pixels: Vec<u8>) -> Self {
        Self {
            width,
            height,
            center: (0, 0),
            rect: G00Rect {
                left: 0,
                top: 0,
                right: width as i32,
                bottom: height as i32,
            },
            chips: vec![G00Chip {
                x: 0,
                y: 0,
                width,
                height,
                sprite: false,
                pixels,
            }],
        }
    }

    fn empty() -> Self {
        Self {
            width: 0,
            height: 0,
            center: (0, 0),
            rect: G00Rect::default(),
            chips: Vec::new(),
        }
    }
}

/// Decode one G00 image to immutable CPU data. The format's own field sizes
/// and the available bytes determine what can be decoded; no policy limits
/// are applied to input, output, or item counts.
pub fn decode_g00(bytes: &[u8]) -> Result<G00Image, G00Error> {
    let mut reader = CheckedReader::new(bytes);
    let kind = reader.read_u8()?;
    if kind > 3 {
        return Err(G00Error::UnsupportedKind(kind));
    }
    let width = u32::from(reader.read_u16_le()?);
    let height = u32::from(reader.read_u16_le()?);
    match kind {
        0 => {
            let expected = checked_bgra_pixel_byte_length(width, height, 1)?;
            let pixels = decode_g00_opaque(reader.rest(), expected)?;
            Ok(G00Image {
                kind: G00Kind::Opaque,
                width,
                height,
                cuts: vec![G00Cut::from_full_image(width, height, pixels)],
            })
        }
        1 => {
            let plain = decode_lzss(reader.rest())?;
            let mut data = CheckedReader::new(&plain);
            let palette_count = usize::from(data.read_u16_le()?);
            let palette_size = checked_usize_product(palette_count, 4, "palette", data.offset())?;
            let palette = data.read_exact_named(palette_size, "palette")?;
            let index_count =
                checked_usize_product(width as usize, height as usize, "palette indices", 1)?;
            let indices = data.read_exact_named(index_count, "palette indices")?;
            let mut pixels = Vec::with_capacity(checked_bgra_pixel_byte_length(width, height, 1)?);
            let index_offset = data.offset() - index_count as u64;
            for (position, &index) in indices.iter().enumerate() {
                let start = usize::from(index) * 4;
                let color = palette
                    .get(start..start + 4)
                    .ok_or(G00Error::InvalidValue {
                        field: "palette index",
                        offset: index_offset + position as u64,
                    })?;
                pixels.extend_from_slice(color);
            }
            Ok(G00Image {
                kind: G00Kind::Paletted,
                width,
                height,
                cuts: vec![G00Cut::from_full_image(width, height, pixels)],
            })
        }
        2 => decode_cuts(reader, width, height),
        3 => decode_jpeg(reader.rest(), width, height),
        _ => Err(G00Error::UnsupportedKind(kind)),
    }
}

fn decode_cuts(
    mut reader: CheckedReader<'_>,
    width: u32,
    height: u32,
) -> Result<G00Image, G00Error> {
    let count_offset = reader.offset();
    let cut_count = usize::try_from(reader.read_i32_le()?).map_err(|_| G00Error::InvalidValue {
        field: "cut count",
        offset: count_offset,
    })?;
    let database_size = checked_usize_product(cut_count, 24, "cut database", count_offset)?;
    reader.read_exact_named(database_size, "cut database")?;
    let plain = decode_lzss(reader.rest())?;
    let mut data = CheckedReader::new(&plain);
    if data.read_u32_le()? as usize != cut_count {
        return Err(G00Error::InvalidValue {
            field: "cut count",
            offset: 0,
        });
    }
    let table_size = checked_usize_product(cut_count, 8, "cut table", data.offset())?;
    let table = data.read_exact_named(table_size, "cut table")?;
    let mut cuts = Vec::with_capacity(cut_count);
    for (index, entry) in table.chunks_exact(8).enumerate() {
        let offset = u32::from_le_bytes(entry[..4].try_into().unwrap()) as usize;
        let cut_size = i32::from_le_bytes(entry[4..].try_into().unwrap());
        if offset == 0 || cut_size == 0 {
            cuts.push(G00Cut::empty());
            continue;
        }
        let entry_offset = 4 + index as u64 * 8;
        let cut_data = if cut_size < 0 {
            if offset >= plain.len() {
                return Err(G00Error::InvalidRange {
                    field: "linked cut",
                    offset: entry_offset,
                });
            }
            plain.get(offset..).ok_or(G00Error::InvalidRange {
                field: "linked cut",
                offset: entry_offset,
            })?
        } else {
            let end = offset
                .checked_add(cut_size as usize)
                .ok_or(G00Error::Overflow {
                    field: "cut data",
                    offset: entry_offset,
                })?;
            plain.get(offset..end).ok_or(G00Error::InvalidRange {
                field: "cut data",
                offset: entry_offset,
            })?
        };
        cuts.push(parse_cut(cut_data, offset as u64)?);
    }
    Ok(G00Image {
        kind: G00Kind::Cut,
        width,
        height,
        cuts,
    })
}

fn parse_cut(bytes: &[u8], base: u64) -> Result<G00Cut, G00Error> {
    let mut data = CheckedReader::from_slice(bytes, base);
    let _kind = data.read_u8()?;
    data.read_u8()?; // padding
    let chip_count = data.read_u16_le()?;
    let x = data.read_i32_le()?;
    let y = data.read_i32_le()?;
    let display_width = data.read_i32_le()?;
    let display_height = data.read_i32_le()?;
    let center = (data.read_i32_le()?, data.read_i32_le()?);
    let width_offset = data.offset();
    let width = u32::try_from(data.read_i32_le()?).map_err(|_| G00Error::InvalidValue {
        field: "cut width",
        offset: width_offset,
    })?;
    let height_offset = data.offset();
    let height = u32::try_from(data.read_i32_le()?).map_err(|_| G00Error::InvalidValue {
        field: "cut height",
        offset: height_offset,
    })?;
    data.read_exact_named(80, "cut header")?;
    let right = x.checked_add(display_width).ok_or(G00Error::Overflow {
        field: "cut rect",
        offset: base + 12,
    })?;
    let bottom = y.checked_add(display_height).ok_or(G00Error::Overflow {
        field: "cut rect",
        offset: base + 16,
    })?;
    let mut chips = Vec::with_capacity(usize::from(chip_count));
    for _ in 0..chip_count {
        chips.push(parse_chip(&mut data)?);
    }
    Ok(G00Cut {
        width,
        height,
        center,
        rect: G00Rect {
            left: x,
            top: y,
            right,
            bottom,
        },
        chips,
    })
}

fn parse_chip(data: &mut CheckedReader<'_>) -> Result<G00Chip, G00Error> {
    let x = u32::from(data.read_u16_le()?);
    let y = u32::from(data.read_u16_le()?);
    let sprite = data.read_u8()? == 1;
    data.read_u8()?; // padding
    let width = u32::from(data.read_u16_le()?);
    let height = u32::from(data.read_u16_le()?);
    data.read_u16_le()?; // padding
    data.read_exact_named(80, "chip header")?;
    let bytes = checked_bgra_pixel_byte_length(width, height, data.offset())?;
    let pixels = data.read_exact_named(bytes, "chip pixels")?.to_vec();
    Ok(G00Chip {
        x,
        y,
        width,
        height,
        sprite,
        pixels,
    })
}

fn decode_jpeg(payload: &[u8], width: u32, height: u32) -> Result<G00Image, G00Error> {
    let mut jpeg_bytes = payload.to_vec();
    for (index, byte) in jpeg_bytes.iter_mut().enumerate() {
        *byte ^= JPEG_XOR[index % JPEG_XOR.len()];
    }
    let mut metadata = JpegDecoder::new(Cursor::new(jpeg_bytes.as_slice()));
    metadata.read_info()?;
    let info = metadata.info().ok_or(G00Error::InvalidValue {
        field: "JPEG info",
        offset: 5,
    })?;
    if u32::from(info.width) != width || u32::from(info.height) != height {
        return Err(G00Error::InvalidValue {
            field: "JPEG dimensions",
            offset: 1,
        });
    }
    let mut decoder = JpegDecoder::new(Cursor::new(jpeg_bytes.as_slice()));
    let decoded = decoder.decode()?;
    let expected = checked_bgra_pixel_byte_length(width, height, 1)?;
    let area = checked_usize_product(width as usize, height as usize, "JPEG area", 1)?;
    let source_bytes =
        checked_usize_product(area, info.pixel_format.pixel_bytes(), "JPEG pixels", 1)?;
    if decoded.len() != source_bytes {
        return Err(G00Error::InvalidValue {
            field: "JPEG pixels",
            offset: 5,
        });
    }
    let mut pixels = Vec::with_capacity(expected);
    match info.pixel_format {
        PixelFormat::L8 => {
            for &luma in &decoded {
                pixels.extend_from_slice(&[luma, luma, luma, 255]);
            }
        }
        PixelFormat::RGB24 => {
            for rgb in decoded.chunks_exact(3) {
                pixels.extend_from_slice(&[rgb[2], rgb[1], rgb[0], 255]);
            }
        }
        PixelFormat::L16 | PixelFormat::CMYK32 => return Err(G00Error::UnsupportedJpegFormat),
    }
    Ok(G00Image {
        kind: G00Kind::Jpeg,
        width,
        height,
        cuts: vec![G00Cut::from_full_image(width, height, pixels)],
    })
}

// SiglusEngine/g00/g00/source/jpeg.cpp: Gv_jpeg_angou_table.
const JPEG_XOR: [u8; 256] = [
    0x45, 0x0c, 0x85, 0xc0, 0x75, 0x14, 0xe5, 0x5d, 0x8b, 0x55, 0xec, 0xc0, 0x5b, 0x8b, 0xc3, 0x8b,
    0x81, 0xff, 0x00, 0x00, 0x04, 0x00, 0x85, 0xff, 0x6a, 0x00, 0x76, 0xb0, 0x43, 0x00, 0x76, 0x49,
    0x00, 0x8b, 0x7d, 0xe8, 0x8b, 0x75, 0xa1, 0xe0, 0x0c, 0x85, 0xc0, 0xc0, 0x75, 0x78, 0x30, 0x44,
    0x00, 0x85, 0xff, 0x76, 0x37, 0x81, 0x1d, 0xd0, 0xff, 0x00, 0x00, 0x75, 0x44, 0x8b, 0xb0, 0x43,
    0x45, 0xf8, 0x8d, 0x55, 0xfc, 0x52, 0x00, 0x76, 0x68, 0x00, 0x00, 0x04, 0x00, 0x6a, 0x43, 0x8b,
    0xb1, 0x43, 0x00, 0x6a, 0x05, 0xff, 0x50, 0xff, 0xd3, 0xa1, 0xe0, 0x04, 0x00, 0x56, 0x15, 0x2c,
    0x44, 0x00, 0x85, 0xc0, 0x74, 0x09, 0xc3, 0xa1, 0x5f, 0x5e, 0x33, 0x8b, 0xe5, 0x5d, 0xe0, 0x30,
    0x04, 0x00, 0x81, 0xc6, 0x00, 0x00, 0x81, 0xef, 0x04, 0x00, 0x85, 0x30, 0x44, 0x00, 0x00, 0x00,
    0x5d, 0xc3, 0x8b, 0x55, 0xf8, 0x8d, 0x5e, 0x5b, 0x4d, 0xfc, 0x51, 0xc4, 0x04, 0x5f, 0x8b, 0xe5,
    0x43, 0x00, 0xeb, 0xd8, 0x8b, 0x45, 0xff, 0x15, 0xe8, 0x83, 0xc0, 0x57, 0x56, 0x52, 0x2c, 0xb1,
    0x01, 0x00, 0x8b, 0x7d, 0xe8, 0x89, 0x00, 0xe8, 0x45, 0xf4, 0x8b, 0x20, 0x50, 0x6a, 0x47, 0x28,
    0x00, 0x50, 0x53, 0xff, 0x15, 0x34, 0xe4, 0x6a, 0xb1, 0x43, 0x00, 0x0c, 0x8b, 0x45, 0x00, 0x6a,
    0x8b, 0x4d, 0xec, 0x89, 0x08, 0x8a, 0x85, 0xc0, 0x45, 0xf0, 0x84, 0x8b, 0x45, 0x10, 0x74, 0x05,
    0xf5, 0x28, 0x01, 0x00, 0x83, 0xc4, 0x52, 0x6a, 0x08, 0x89, 0x45, 0x83, 0xc2, 0x20, 0x00, 0xe8,
    0xe8, 0xf4, 0xfb, 0xff, 0xff, 0x8b, 0x8b, 0x5d, 0x45, 0x0c, 0x83, 0xc0, 0x74, 0xc5, 0xf8, 0x53,
    0xc4, 0x08, 0x85, 0xc0, 0x75, 0x56, 0x30, 0x44, 0x8b, 0x1d, 0xd0, 0xf0, 0xa1, 0xe0, 0x00, 0x83,
];

#[cfg(test)]
mod tests {
    use jpeg_encoder::{ColorType, Encoder};

    use super::{G00Error, G00Kind, JPEG_XOR, decode_g00};

    #[test]
    fn jpeg_pixels_and_dimensions() {
        let mut jpeg = Vec::new();
        Encoder::new(&mut jpeg, 100)
            .encode(&[255, 0, 0], 1, 1, ColorType::Rgb)
            .unwrap();
        let mut file = vec![3, 1, 0, 1, 0];
        for (index, &byte) in jpeg.iter().enumerate() {
            file.push(byte ^ JPEG_XOR[index % JPEG_XOR.len()]);
        }
        let image = decode_g00(&file).unwrap();
        assert_eq!(image.kind, G00Kind::Jpeg);
        assert_eq!(image.cuts[0].chips[0].pixels.len(), 4);
        assert_eq!(image.cuts[0].chips[0].pixels[3], 255);

        file[1] = 2;
        assert!(matches!(
            decode_g00(&file),
            Err(G00Error::InvalidValue {
                field: "JPEG dimensions",
                ..
            })
        ));
    }
}
