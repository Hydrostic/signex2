//! NWA audio reader (raw and compressed 16-bit mono/stereo PCM).
//!
//! Wire contract (evidence: `SiglusEngine/nwa/Headers/nwa_pack.h`,
//! `nwa_unpack.cpp`, `nwa_unpack_unit.cpp`, and
//! `siglus-analysis-docs/engine_modules/03_resources_vfs_codecs.md` D05):
//!
//! * Fixed 44-byte little-endian header:
//!   `{u16 channels, u16 bits, u32 rate, i32 pack_mod, i32 zero_mod,
//!   u32 unit_cnt, u32 original_size, u32 pack_size, u32 sample_cnt,
//!   u32 unit_sample_cnt, u32 last_sample_cnt, u32 last_sample_pack_size}`.
//! * `pack_mod == -1`: raw PCM follows the header; `sample_cnt * 2 ==
//!   original_size` for 16-bit and the file must hold `44 + original_size`
//!   bytes.
//! * `pack_mod in 0..=5`: a `unit_cnt`-entry `u32` offset table follows the
//!   header. Offsets are relative to the start of the file (the encoder writes
//!   `44 + unit_cnt * 4` for the first unit). Unit lengths are the adjacent
//!   offset differences; the final unit length is `last_sample_pack_size`.
//! * Each compressed unit starts with a 16-bit predictor (mono: 2 bytes;
//!   stereo: 4 bytes) followed by an LSB-first bitstream. Stored `pack_mod`
//!   maps to the bit width as `0→5, 1→4, 2→3, 3→6, 4→7, 5→8` (the encoder
//!   already remaps 0/1/2 before writing).
//!
//! Only 16-bit mono/stereo is decoded. An 8-bit header is accepted by the
//! legacy `open` but never decodes; that profile is reported as
//! [`DecodeError::Unsupported`] rather than emitting partial PCM.
//!
//! Decoding is a pure function of `bytes`: identical input yields
//! identical `NwaAudio` or the same error category.

use super::{AudioDecodeError as DecodeError, checked_size};
use crate::reader::CheckedReader;

/// Raw NWA header plus the decoded unit-offset table (compressed mode only).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NwaHeader {
    /// 1 = mono, 2 = stereo.
    pub channels: u16,
    /// Sample bit depth (8 or 16 in the header; only 16 is decoded).
    pub bits: u16,
    /// Samples per second (per channel).
    pub rate: u32,
    /// -1 = raw PCM, 0..=5 = compressed quality modes.
    pub pack_mode: i32,
    /// 0 = no zero-run compression, 1 = zero-run compression enabled.
    pub zero_mode: i32,
    /// Number of compressed units (0 in raw mode).
    pub unit_count: u32,
    /// Byte size of the original PCM payload.
    pub original_size: u32,
    /// Encoder-written packed size (informational; not trusted for bounds).
    pub packed_size: u32,
    /// Total interleaved samples (frames * channels) in the decoded output.
    pub sample_count: u32,
    /// Samples per full unit.
    pub unit_sample_count: u32,
    /// Samples of the final unit.
    pub last_sample_count: u32,
    /// Byte size of the final compressed unit.
    pub last_sample_packed_size: u32,
    /// Compressed-mode unit offsets, relative to the start of the input.
    pub unit_offsets: Vec<u32>,
}

impl NwaHeader {
    /// Total interleaved PCM samples (`sample_count`).
    pub fn sample_count(&self) -> u32 {
        self.sample_count
    }

    /// Number of interleaved frames (`sample_count / channels`).
    pub fn frame_count(&self) -> u32 {
        self.sample_count / u32::from(self.channels.max(1))
    }
}

/// Decoded NWA audio: interleaved 16-bit little-endian PCM.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NwaAudio {
    channels: u16,
    rate: u32,
    samples: Vec<i16>,
}

impl NwaAudio {
    /// 1 = mono, 2 = stereo.
    pub fn channels(&self) -> u16 {
        self.channels
    }

    /// Samples per second (per channel).
    pub fn rate(&self) -> u32 {
        self.rate
    }

    /// Interleaved PCM samples (mono: left only; stereo: left, right, ...).
    pub fn samples(&self) -> &[i16] {
        &self.samples
    }

    /// Number of interleaved frames (`samples / channels`).
    pub fn frame_count(&self) -> usize {
        self.samples.len() / usize::from(self.channels.max(1))
    }

    /// Bounded view of frames `[start, start + count)`, clamped to the buffer.
    pub fn frames(&self, start: usize, count: usize) -> &[i16] {
        let channels = usize::from(self.channels.max(1));
        let begin = start.saturating_mul(channels).min(self.samples.len());
        let end = start
            .saturating_add(count)
            .saturating_mul(channels)
            .min(self.samples.len());
        &self.samples[begin..end.max(begin)]
    }
}

/// Parse and validate the 44-byte NWA header (and, in compressed mode, the
/// unit-offset table). All ranges and counts are checked before any output is
/// allocated.
pub fn parse_nwa_header(bytes: &[u8]) -> Result<NwaHeader, DecodeError> {
    let mut reader = CheckedReader::new(bytes);
    let channels = reader.read_u16_le()?;
    let bits = reader.read_u16_le()?;
    let rate = reader.read_u32_le()?;
    let pack_mode = reader.read_i32_le()?;
    let zero_mode = reader.read_i32_le()?;
    let unit_count = reader.read_u32_le()?;
    let original_size = reader.read_u32_le()?;
    let packed_size = reader.read_u32_le()?;
    let sample_count = reader.read_u32_le()?;
    let unit_sample_count = reader.read_u32_le()?;
    let last_sample_count = reader.read_u32_le()?;
    let last_sample_packed_size = reader.read_u32_le()?;

    if !matches!(channels, 1 | 2) {
        return Err(DecodeError::InvalidValue {
            field: "NWA channels",
            offset: 0,
            value: i64::from(channels),
        });
    }
    if bits == 8 {
        return Err(DecodeError::Unsupported {
            feature: "NWA 8-bit profile",
            offset: 2,
        });
    }
    if bits != 16 {
        return Err(DecodeError::InvalidValue {
            field: "NWA bits per sample",
            offset: 2,
            value: i64::from(bits),
        });
    }
    if rate == 0 {
        return Err(DecodeError::InvalidValue {
            field: "NWA sample rate",
            offset: 4,
            value: 0,
        });
    }
    if !(-1..=5).contains(&pack_mode) {
        return Err(DecodeError::InvalidValue {
            field: "NWA pack mode",
            offset: 8,
            value: i64::from(pack_mode),
        });
    }
    if !matches!(zero_mode, 0 | 1) {
        return Err(DecodeError::InvalidValue {
            field: "NWA zero mode",
            offset: 12,
            value: i64::from(zero_mode),
        });
    }
    if sample_count == 0 {
        return Err(DecodeError::InvalidValue {
            field: "NWA sample count",
            offset: 28,
            value: 0,
        });
    }

    // 16-bit PCM consistency: original_size == sample_count * 2 (total
    // interleaved samples, matching the encoder's `original_size = size/2`).
    let expected_bytes = checked_size(
        usize::try_from(sample_count).map_err(|_| DecodeError::ArithmeticOverflow {
            field: "NWA sample count",
            offset: 28,
        })?,
        2,
        "NWA original size",
        28,
    )?;
    if u64::try_from(expected_bytes).unwrap_or(u64::MAX) != u64::from(original_size) {
        return Err(DecodeError::InvalidValue {
            field: "NWA original size",
            offset: 20,
            value: i64::from(original_size),
        });
    }
    let mut unit_offsets = Vec::new();
    if pack_mode == -1 {
        // Raw PCM: `44 + original_size` payload bytes must be present.
        let payload_end = 44_usize
            .checked_add(usize::try_from(original_size).map_err(|_| {
                DecodeError::ArithmeticOverflow {
                    field: "NWA raw payload",
                    offset: 20,
                }
            })?)
            .ok_or(DecodeError::ArithmeticOverflow {
                field: "NWA raw payload",
                offset: 20,
            })?;
        if payload_end > bytes.len() {
            return Err(DecodeError::UnexpectedEof {
                field: "NWA raw PCM",
                offset: 44,
                needed: usize::try_from(original_size).unwrap_or(usize::MAX),
                remaining: bytes.len().saturating_sub(44),
            });
        }
    } else {
        if unit_count == 0 {
            return Err(DecodeError::InvalidValue {
                field: "NWA unit count",
                offset: 16,
                value: 0,
            });
        }
        if unit_sample_count == 0 {
            return Err(DecodeError::InvalidValue {
                field: "NWA unit sample count",
                offset: 32,
                value: 0,
            });
        }
        if last_sample_count == 0 || last_sample_count > unit_sample_count {
            return Err(DecodeError::InvalidValue {
                field: "NWA last sample count",
                offset: 36,
                value: i64::from(last_sample_count),
            });
        }
        let total_samples = (u64::from(unit_count) - 1)
            .checked_mul(u64::from(unit_sample_count))
            .and_then(|count| count.checked_add(u64::from(last_sample_count)))
            .ok_or(DecodeError::ArithmeticOverflow {
                field: "NWA unit sample total",
                offset: 16,
            })?;
        if total_samples != u64::from(sample_count) {
            return Err(DecodeError::InvalidValue {
                field: "NWA unit sample total",
                offset: 28,
                value: i64::try_from(total_samples).unwrap_or(i64::MAX),
            });
        }
        let count = checked_size(
            usize::try_from(unit_count).map_err(|_| DecodeError::ArithmeticOverflow {
                field: "NWA unit count",
                offset: 16,
            })?,
            4,
            "NWA unit table",
            44,
        )?;
        let table_end = 44_usize
            .checked_add(count)
            .ok_or(DecodeError::ArithmeticOverflow {
                field: "NWA unit table",
                offset: 44,
            })?;
        if table_end > bytes.len() {
            return Err(DecodeError::UnexpectedEof {
                field: "NWA unit offsets",
                offset: 44,
                needed: count,
                remaining: bytes.len().saturating_sub(44),
            });
        }
        unit_offsets
            .try_reserve_exact(usize::try_from(unit_count).map_err(|_| {
                DecodeError::ArithmeticOverflow {
                    field: "NWA unit count",
                    offset: 16,
                }
            })?)
            .map_err(|_| DecodeError::AllocationFailed {
                field: "NWA unit offsets",
            })?;
        let mut previous = table_end;
        for _ in 0..unit_count {
            let offset = reader.read_u32_le()?;
            let position =
                usize::try_from(offset).map_err(|_| DecodeError::ArithmeticOverflow {
                    field: "NWA unit offset",
                    offset: reader.offset() - 4,
                })?;
            // Offsets are relative to the file start and must be monotonic
            // non-decreasing; unit lengths derive from adjacent differences.
            if position < previous || position >= bytes.len() {
                return Err(DecodeError::InvalidValue {
                    field: "NWA unit offsets",
                    offset: reader.offset() - 4,
                    value: i64::from(offset),
                });
            }
            previous = position;
            unit_offsets.push(offset);
        }
        // The final unit must be fully contained in the file.
        if last_sample_packed_size == 0 {
            return Err(DecodeError::InvalidValue {
                field: "NWA last unit size",
                offset: 40,
                value: i64::from(last_sample_packed_size),
            });
        }
        let final_end = previous
            .checked_add(usize::try_from(last_sample_packed_size).map_err(|_| {
                DecodeError::ArithmeticOverflow {
                    field: "NWA last unit size",
                    offset: 40,
                }
            })?)
            .ok_or(DecodeError::ArithmeticOverflow {
                field: "NWA last unit",
                offset: 40,
            })?;
        if final_end > bytes.len() {
            return Err(DecodeError::UnexpectedEof {
                field: "NWA final unit",
                offset: u64::try_from(previous).unwrap_or(u64::MAX),
                needed: usize::try_from(last_sample_packed_size).unwrap_or(usize::MAX),
                remaining: bytes.len().saturating_sub(previous),
            });
        }
    }

    Ok(NwaHeader {
        channels,
        bits,
        rate,
        pack_mode,
        zero_mode,
        unit_count,
        original_size,
        packed_size,
        sample_count,
        unit_sample_count,
        last_sample_count,
        last_sample_packed_size,
        unit_offsets,
    })
}

/// Decode a complete NWA file to interleaved 16-bit little-endian PCM.
///
/// Raw (`pack_mod == -1`) files are copied straight from the payload. Compressed
/// files decode every unit in order. The result is deterministic and the output
/// is checked against the sample count in the header.
pub fn decode_nwa(bytes: &[u8]) -> Result<NwaAudio, DecodeError> {
    let header = parse_nwa_header(bytes)?;
    let expected_samples =
        usize::try_from(header.sample_count).map_err(|_| DecodeError::ArithmeticOverflow {
            field: "NWA sample count",
            offset: 28,
        })?;
    let mut samples = Vec::new();
    if header.pack_mode == -1 {
        let original =
            usize::try_from(header.original_size).map_err(|_| DecodeError::ArithmeticOverflow {
                field: "NWA raw payload",
                offset: 20,
            })?;
        let payload = &bytes[44..44 + original];
        samples
            .try_reserve_exact(expected_samples)
            .map_err(|_| DecodeError::AllocationFailed {
                field: "NWA PCM samples",
            })?;
        samples.extend(
            payload
                .chunks_exact(2)
                .map(|pair| i16::from_le_bytes([pair[0], pair[1]])),
        );
    } else {
        let units = header.unit_offsets.len();
        for index in 0..units {
            let start = usize::try_from(header.unit_offsets[index]).map_err(|_| {
                DecodeError::ArithmeticOverflow {
                    field: "NWA unit offset",
                    offset: 44,
                }
            })?;
            let (end, unit_samples) = if index + 1 == units {
                let end =
                    start
                        .checked_add(usize::try_from(header.last_sample_packed_size).map_err(
                            |_| DecodeError::ArithmeticOverflow {
                                field: "NWA last unit size",
                                offset: 40,
                            },
                        )?)
                        .ok_or(DecodeError::ArithmeticOverflow {
                            field: "NWA last unit range",
                            offset: 44,
                        })?;
                (end, header.last_sample_count)
            } else {
                let end = usize::try_from(header.unit_offsets[index + 1]).map_err(|_| {
                    DecodeError::ArithmeticOverflow {
                        field: "NWA unit offset",
                        offset: 44,
                    }
                })?;
                (end, header.unit_sample_count)
            };
            let unit_samples_len =
                usize::try_from(unit_samples).map_err(|_| DecodeError::ArithmeticOverflow {
                    field: "NWA unit sample count",
                    offset: 32,
                })?;
            if unit_samples_len > expected_samples.saturating_sub(samples.len()) {
                return Err(DecodeError::InvalidValue {
                    field: "NWA unit sample total",
                    offset: 28,
                    value: i64::try_from(samples.len().saturating_add(unit_samples_len))
                        .unwrap_or(i64::MAX),
                });
            }
            samples
                .try_reserve(unit_samples_len)
                .map_err(|_| DecodeError::AllocationFailed {
                    field: "NWA PCM samples",
                })?;
            decode_nwa_unit(
                bytes.get(start..end).ok_or(DecodeError::InvalidRange {
                    field: "NWA unit",
                    offset: u64::try_from(start).unwrap_or(u64::MAX),
                    start: u64::try_from(start).unwrap_or(u64::MAX),
                    length: u64::try_from(end.saturating_sub(start)).unwrap_or(u64::MAX),
                    input_len: u64::try_from(bytes.len()).unwrap_or(u64::MAX),
                })?,
                start,
                unit_samples,
                &header,
                &mut samples,
            )?;
        }
    }
    if samples.len() != expected_samples {
        return Err(DecodeError::InvalidValue {
            field: "NWA decoded sample count",
            offset: 44,
            value: i64::try_from(samples.len()).unwrap_or(i64::MAX),
        });
    }
    Ok(NwaAudio {
        channels: header.channels,
        rate: header.rate,
        samples,
    })
}

/// Decode one compressed unit into `output`.
///
/// `unit_base` is the byte offset of the unit within the whole input and is used
/// for error reporting only.
fn decode_nwa_unit(
    bytes: &[u8],
    unit_base: usize,
    sample_count: u32,
    header: &NwaHeader,
    output: &mut Vec<i16>,
) -> Result<(), DecodeError> {
    let channels = usize::from(header.channels);
    let seed_bytes = channels
        .checked_mul(2)
        .ok_or(DecodeError::ArithmeticOverflow {
            field: "NWA predictor seed",
            offset: u64::try_from(unit_base).unwrap_or(u64::MAX),
        })?;
    if bytes.len() < seed_bytes {
        return Err(DecodeError::UnexpectedEof {
            field: "NWA predictor",
            offset: u64::try_from(unit_base).unwrap_or(u64::MAX),
            needed: seed_bytes,
            remaining: bytes.len(),
        });
    }
    let mut predictors = [0_i32; 2];
    for (channel, pair) in bytes[..seed_bytes].chunks_exact(2).enumerate() {
        predictors[channel] = i32::from(i16::from_le_bytes([pair[0], pair[1]]));
    }
    // Stored pack mode 0/1/2 is already the encoder remap (2/1/0); map the
    // stored mode to the decoder bit width.
    let mode = match header.pack_mode {
        0 => 5,
        1 => 4,
        2 => 3,
        3 => 6,
        4 => 7,
        5 => 8,
        _ => {
            return Err(DecodeError::InvalidValue {
                field: "NWA pack mode",
                offset: 8,
                value: i64::from(header.pack_mode),
            });
        }
    };
    let mut bits = LsbBits::new(&bytes[seed_bytes..], unit_base + seed_bytes);
    let mut zero_count = 0_u32;
    for index in 0..sample_count {
        let channel = index % u32::try_from(channels).unwrap_or(u32::MAX);
        let channel = usize::try_from(channel).unwrap_or(usize::MAX);
        let mut sample = predictors[channel];
        if zero_count > 0 {
            zero_count -= 1;
        } else {
            let code = bits.read(3)?;
            if code == 0 {
                if header.zero_mode != 0 && bits.read(1)? == 1 {
                    zero_count = bits.read(2)?;
                    if zero_count == 3 {
                        zero_count = bits.read(8)?;
                    }
                }
            } else if code == 7 {
                if bits.read(1)? == 1 {
                    sample = 0;
                } else {
                    let width = if mode <= 4 { mode + 3 } else { 8 };
                    let shift = if mode <= 4 { 14 - mode } else { 9 };
                    sample = sample.wrapping_add(signed_delta(bits.read(width)?, width, shift));
                }
            } else {
                let width = mode;
                let shift = match mode {
                    3 => code + 4,
                    4 => code + 3,
                    5 => code + 2,
                    6..=8 => code + 1,
                    _ => unreachable!("mode mapped to 3..=8"),
                };
                sample = sample.wrapping_add(signed_delta(bits.read(width)?, width, shift));
            }
        }
        predictors[channel] = sample;
        let wrapped = sample.to_le_bytes();
        output.push(i16::from_le_bytes([wrapped[0], wrapped[1]]));
    }
    Ok(())
}

/// Signed fixed-width delta with the encoder's shift/sign convention.
fn signed_delta(raw: u32, width: u32, shift: u32) -> i32 {
    let sign = 1_u32 << (width - 1);
    let magnitude = raw & (sign - 1);
    let value = i32::try_from(magnitude << shift).unwrap_or(i32::MAX);
    if raw & sign == 0 {
        value
    } else {
        value.wrapping_neg()
    }
}

/// LSB-first bit reader with absolute-offset error reporting.
struct LsbBits<'a> {
    bytes: &'a [u8],
    bit: usize,
    base: usize,
}

impl<'a> LsbBits<'a> {
    fn new(bytes: &'a [u8], base: usize) -> Self {
        Self {
            bytes,
            bit: 0,
            base,
        }
    }

    fn read(&mut self, count: u32) -> Result<u32, DecodeError> {
        let mut value = 0_u32;
        for index in 0..count {
            let byte_index = self.bit / 8;
            let byte = self
                .bytes
                .get(byte_index)
                .ok_or(DecodeError::UnexpectedEof {
                    field: "NWA unit bitstream",
                    offset: u64::try_from(self.base + byte_index).unwrap_or(u64::MAX),
                    needed: 1,
                    remaining: self.bytes.len().saturating_sub(byte_index),
                })?;
            value |= u32::from((byte >> (self.bit % 8)) & 1) << index;
            self.bit += 1;
        }
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw_nwa(channels: u16, samples: &[i16]) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&channels.to_le_bytes());
        bytes.extend_from_slice(&16_u16.to_le_bytes());
        bytes.extend_from_slice(&44_100_u32.to_le_bytes());
        bytes.extend_from_slice(&(-1_i32).to_le_bytes());
        bytes.extend_from_slice(&0_i32.to_le_bytes());
        bytes.extend_from_slice(&0_u32.to_le_bytes());
        bytes.extend_from_slice(&(u32::try_from(samples.len()).unwrap() * 2).to_le_bytes());
        bytes
            .extend_from_slice(&(44_u32 + u32::try_from(samples.len()).unwrap() * 2).to_le_bytes());
        bytes.extend_from_slice(&u32::try_from(samples.len()).unwrap().to_le_bytes());
        bytes.extend_from_slice(&0_u32.to_le_bytes());
        bytes.extend_from_slice(&0_u32.to_le_bytes());
        bytes.extend_from_slice(&0_u32.to_le_bytes());
        for sample in samples {
            bytes.extend_from_slice(&sample.to_le_bytes());
        }
        bytes
    }

    #[test]
    fn raw_nwa_decodes_and_frames_are_interleaved() {
        let bytes = raw_nwa(2, &[1, 2, 3, 4, 5, 6]);
        let audio = decode_nwa(&bytes).unwrap();
        assert_eq!(audio.channels(), 2);
        assert_eq!(audio.rate(), 44_100);
        assert_eq!(audio.samples(), &[1, 2, 3, 4, 5, 6]);
        assert_eq!(audio.frame_count(), 3);
        assert_eq!(audio.frames(1, 1), [3, 4]);
        assert_eq!(audio.frames(2, 99), [5, 6]);
        assert_eq!(audio.frames(99, 4), [] as [i16; 0]);
    }

    #[test]
    fn raw_nwa_mono_uses_one_sample_per_frame() {
        let bytes = raw_nwa(1, &[10, 20, 30]);
        let audio = decode_nwa(&bytes).unwrap();
        assert_eq!(audio.frame_count(), 3);
        assert_eq!(audio.frames(1, 2), [20, 30]);
    }

    #[test]
    fn raw_nwa_larger_than_former_policy_limit_decodes() {
        let samples = vec![42_i16; (1 << 20) / 2 + 1];
        let bytes = raw_nwa(1, &samples);
        let audio = decode_nwa(&bytes).unwrap();
        assert_eq!(audio.samples(), samples);
    }

    #[allow(clippy::too_many_arguments)]
    fn compressed_nwa(
        channels: u16,
        pack_mode: i32,
        zero_mode: i32,
        unit_count: u32,
        sample_count: u32,
        unit_sample_count: u32,
        last_sample_count: u32,
        units: &[Vec<u8>],
    ) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&channels.to_le_bytes());
        bytes.extend_from_slice(&16_u16.to_le_bytes());
        bytes.extend_from_slice(&22_050_u32.to_le_bytes());
        bytes.extend_from_slice(&pack_mode.to_le_bytes());
        bytes.extend_from_slice(&zero_mode.to_le_bytes());
        bytes.extend_from_slice(&unit_count.to_le_bytes());
        bytes.extend_from_slice(&(sample_count * 2).to_le_bytes());
        bytes.extend_from_slice(&0_u32.to_le_bytes());
        bytes.extend_from_slice(&sample_count.to_le_bytes());
        bytes.extend_from_slice(&unit_sample_count.to_le_bytes());
        bytes.extend_from_slice(&last_sample_count.to_le_bytes());
        // Compute unit offsets relative to the file start.
        let mut offsets = Vec::with_capacity(unit_count as usize);
        let mut cursor = 44 + usize::try_from(unit_count).unwrap() * 4;
        for unit in units {
            offsets.push(u32::try_from(cursor).unwrap());
            cursor += unit.len();
        }
        let last_pack = u32::try_from(units.last().map(Vec::len).unwrap_or(0)).unwrap();
        bytes.extend_from_slice(&last_pack.to_le_bytes());
        for offset in &offsets {
            bytes.extend_from_slice(&offset.to_le_bytes());
        }
        for unit in units {
            bytes.extend_from_slice(unit);
        }
        bytes
    }

    #[test]
    fn compressed_nwa_predictor_and_stereo_channels_follow_lsb_stream() {
        // Mono, stored pack mode 2 -> width 3. Predictor 100, mode 1 + delta
        // 1<<5 = 32, then mode 0 (no zero compression) keeps the predictor.
        let mono = compressed_nwa(1, 2, 0, 1, 2, 2, 2, &[vec![100, 0, 0b0000_1001, 0]]);
        assert_eq!(decode_nwa(&mono).unwrap().samples(), [132, 132]);

        // Stereo predictors are [100, 200]; mode 0 keeps each channel's value.
        let stereo = compressed_nwa(2, 2, 0, 1, 2, 2, 2, &[vec![100, 0, 200, 0, 0]]);
        assert_eq!(decode_nwa(&stereo).unwrap().samples(), [100, 200]);
    }

    #[test]
    fn compressed_nwa_decodes_adjacent_units() {
        let bytes = compressed_nwa(1, 2, 0, 2, 2, 1, 1, &[vec![100, 0, 0], vec![200, 0, 0]]);
        assert_eq!(decode_nwa(&bytes).unwrap().samples(), [100, 200]);
    }

    #[test]
    fn zero_run_compression_repeats_predictor() {
        // zero_mode 1, mono. Predictor 0, then a 3-sample zero run.
        // bits: mode(000) flag(1) count2(10) => 0b0001_1000 LSB-first = 0x18.
        let unit = vec![0, 0, 0b0001_1000, 0];
        let bytes = compressed_nwa(1, 2, 1, 1, 3, 3, 3, &[unit]);
        assert_eq!(decode_nwa(&bytes).unwrap().samples(), [0, 0, 0]);
    }

    #[test]
    fn absolute_zero_mode_resets_predictor() {
        // mode 7 + 1 bit set -> absolute zero. Predictor 100 becomes 0.
        // bits: mode(111) then 1 -> 0b_1111 = 0x0F LSB-first in first byte.
        let unit = vec![100, 0, 0b0000_1111, 0];
        let bytes = compressed_nwa(1, 2, 0, 1, 1, 1, 1, &[unit]);
        assert_eq!(decode_nwa(&bytes).unwrap().samples(), [0]);
    }

    #[test]
    fn rejects_8_bit_profile_as_unsupported() {
        let mut bytes = raw_nwa(1, &[1, 2]);
        bytes[2..4].copy_from_slice(&8_u16.to_le_bytes());
        assert!(matches!(
            decode_nwa(&bytes),
            Err(DecodeError::Unsupported { feature, .. }) if feature.contains("8-bit")
        ));
    }

    #[test]
    fn rejects_bad_channels_bits_rate_and_pack_mode() {
        let mut bytes = raw_nwa(1, &[1, 2]);
        bytes[0] = 3;
        assert!(matches!(
            decode_nwa(&bytes),
            Err(DecodeError::InvalidValue { field, .. }) if field.contains("channels")
        ));
        let mut bytes = raw_nwa(1, &[1, 2]);
        bytes[2..4].copy_from_slice(&12_u16.to_le_bytes());
        assert!(matches!(
            decode_nwa(&bytes),
            Err(DecodeError::InvalidValue { field, .. }) if field.contains("bits")
        ));
        let mut bytes = raw_nwa(1, &[1, 2]);
        bytes[4..8].copy_from_slice(&0_u32.to_le_bytes());
        assert!(matches!(
            decode_nwa(&bytes),
            Err(DecodeError::InvalidValue { field, .. }) if field.contains("rate")
        ));
        let mut bytes = raw_nwa(1, &[1, 2]);
        bytes[8..12].copy_from_slice(&6_i32.to_le_bytes());
        assert!(matches!(
            decode_nwa(&bytes),
            Err(DecodeError::InvalidValue { field, .. }) if field.contains("pack mode")
        ));
    }

    #[test]
    fn rejects_truncated_header_and_payload() {
        assert!(matches!(
            parse_nwa_header(&[0; 43]),
            Err(DecodeError::UnexpectedEof { field, .. }) if field.contains("u16") || field.contains("i32") || field.contains("u32")
        ));
        // Raw payload truncated.
        let bytes = raw_nwa(1, &[1, 2, 3]);
        let truncated = &bytes[..bytes.len() - 1];
        assert!(matches!(
            decode_nwa(truncated),
            Err(DecodeError::UnexpectedEof { field, .. }) if field.contains("raw PCM")
        ));
    }

    #[test]
    fn rejects_inconsistent_original_size() {
        // original_size lives at header offset 20.
        let mut bytes = raw_nwa(1, &[1, 2]);
        bytes[20..24].copy_from_slice(&999_u32.to_le_bytes());
        assert!(matches!(
            decode_nwa(&bytes),
            Err(DecodeError::InvalidValue { field, .. }) if field.contains("original size")
        ));
    }

    #[test]
    fn rejects_compressed_nonmonotonic_offsets_and_overflowing_table() {
        // Two units; patch the second offset to a smaller value than the first
        // (strictly non-monotonic) while staying inside the file.
        let mut bytes = compressed_nwa(1, 2, 0, 2, 4, 2, 2, &[vec![1, 2, 0], vec![3, 4, 0]]);
        let first = u32::from_le_bytes(bytes[44..48].try_into().unwrap());
        bytes[48..52].copy_from_slice(&(first - 1).to_le_bytes());
        assert!(matches!(
            parse_nwa_header(&bytes),
            Err(DecodeError::InvalidValue { field, .. }) if field.contains("unit offsets")
        ));
        // Huge unit count fails validation before allocation.
        let mut bytes = raw_nwa(1, &[1]);
        bytes[8..12].copy_from_slice(&1_i32.to_le_bytes()); // compressed
        bytes[16..20].copy_from_slice(&0xFFFF_FF00_u32.to_le_bytes()); // unit_cnt
        bytes[32..36].copy_from_slice(&512_u32.to_le_bytes()); // unit_sample_cnt
        assert!(matches!(
            parse_nwa_header(&bytes),
            Err(DecodeError::UnexpectedEof { .. }
                | DecodeError::InvalidValue { .. }
                | DecodeError::ArithmeticOverflow { .. })
        ));
    }

    #[test]
    fn rejects_inconsistent_compressed_sample_total() {
        let bytes = compressed_nwa(1, 2, 0, 1, 3, 2, 2, &[vec![1, 0, 0]]);
        assert!(matches!(
            parse_nwa_header(&bytes),
            Err(DecodeError::InvalidValue { field, .. }) if field.contains("unit sample total")
        ));
    }

    #[test]
    fn decode_is_deterministic() {
        let unit = vec![100, 0, 0b0000_1001, 0];
        let bytes = compressed_nwa(1, 2, 0, 1, 2, 2, 2, &[unit]);
        let first = decode_nwa(&bytes).unwrap();
        let second = decode_nwa(&bytes).unwrap();
        assert_eq!(first, second);
    }
}
