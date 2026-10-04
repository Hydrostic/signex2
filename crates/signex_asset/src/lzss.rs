#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LzssError {
    #[error("truncated {field}")]
    Truncated { field: &'static str },
    #[error("invalid archive size {size}")]
    InvalidArchiveSize { size: u32 },
    #[error("invalid back-reference distance {distance}")]
    InvalidDistance { distance: usize },
    #[error("back-reference exceeds output")]
    OutputOverflow,
    #[error("invalid LZSS32 output size {actual}, expected {expected}")]
    InvalidOutputSize { actual: u32, expected: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum LzssEncodeError {
    #[error("LZSS {field} exceeds the format size")]
    SizeOverflow { field: &'static str },
}

/// Encode standard LZSS using literal tokens.
pub(crate) fn encode(input: &[u8]) -> Result<Vec<u8>, LzssEncodeError> {
    let output_size = u32::try_from(input.len()).map_err(|_| LzssEncodeError::SizeOverflow {
        field: "output size",
    })?;
    let groups = input.len().div_ceil(8);
    let archive_size = 8usize
        .checked_add(input.len())
        .and_then(|size| size.checked_add(groups))
        .ok_or(LzssEncodeError::SizeOverflow {
            field: "archive size",
        })?;
    let archive_size = u32::try_from(archive_size).map_err(|_| LzssEncodeError::SizeOverflow {
        field: "archive size",
    })?;
    let mut output = Vec::with_capacity(archive_size as usize);
    output.extend_from_slice(&archive_size.to_le_bytes());
    output.extend_from_slice(&output_size.to_le_bytes());
    for chunk in input.chunks(8) {
        output.push(u8::MAX >> (8 - chunk.len()));
        output.extend_from_slice(chunk);
    }
    Ok(output)
}

pub fn decode(input: &[u8]) -> Result<Vec<u8>, LzssError> {
    fn read_u32(bytes: &[u8], offset: &mut usize, field: &'static str) -> Result<u32, LzssError> {
        let end = offset
            .checked_add(4)
            .ok_or(LzssError::Truncated { field })?;
        let value = bytes
            .get(*offset..end)
            .ok_or(LzssError::Truncated { field })?;
        *offset = end;
        Ok(u32::from_le_bytes(value.try_into().unwrap()))
    }
    let mut offset = 0;
    let archive_size = read_u32(input, &mut offset, "archive size")?;
    let output_size = read_u32(input, &mut offset, "output size")? as usize;
    if archive_size < 8 || archive_size as usize > input.len() {
        return Err(LzssError::InvalidArchiveSize { size: archive_size });
    }
    let stream_end = archive_size as usize;
    let stream = &input[..stream_end];
    let mut output = Vec::new();
    while output.len() < output_size {
        let flags = *stream
            .get(offset)
            .ok_or(LzssError::Truncated { field: "flags" })?;
        offset += 1;
        for bit in 0..8 {
            if output.len() == output_size {
                break;
            }
            if flags & (1 << bit) != 0 {
                output.push(
                    *stream
                        .get(offset)
                        .ok_or(LzssError::Truncated { field: "literal" })?,
                );
                offset += 1;
                continue;
            }
            let end = offset.checked_add(2).ok_or(LzssError::Truncated {
                field: "back-reference",
            })?;
            if end > stream_end {
                return Err(LzssError::Truncated {
                    field: "back-reference",
                });
            }
            let token = u16::from_le_bytes([stream[offset], stream[offset + 1]]);
            offset = end;
            let distance = (token >> 4) as usize;
            let length = (token & 0x0f) as usize + 2;
            if distance == 0 || distance > output.len() {
                return Err(LzssError::InvalidDistance { distance });
            }
            if output
                .len()
                .checked_add(length)
                .is_none_or(|end| end > output_size)
            {
                return Err(LzssError::OutputOverflow);
            }
            for _ in 0..length {
                let index = output.len() - distance;
                output.push(output[index]);
            }
        }
    }
    Ok(output)
}

/// G00 type 0: LZSS tokens operate on four-byte BGRA pixels. Literal tokens
/// store BGR and produce an opaque alpha byte.
pub(crate) fn decode_g00_opaque(input: &[u8], expected_size: usize) -> Result<Vec<u8>, LzssError> {
    let header = input.get(..8).ok_or(LzssError::Truncated {
        field: "LZSS32 header",
    })?;
    let archive_size = u32::from_le_bytes(header[..4].try_into().unwrap());
    let output_size = u32::from_le_bytes(header[4..8].try_into().unwrap());
    if archive_size < 8 || archive_size as usize > input.len() {
        return Err(LzssError::InvalidArchiveSize { size: archive_size });
    }
    if output_size as usize != expected_size {
        return Err(LzssError::InvalidOutputSize {
            actual: output_size,
            expected: expected_size,
        });
    }
    let stream = &input[..archive_size as usize];
    let mut cursor = 8;
    let mut output = Vec::new();
    while output.len() < expected_size {
        let flags = *stream.get(cursor).ok_or(LzssError::Truncated {
            field: "LZSS32 flags",
        })?;
        cursor += 1;
        for bit in 0..8 {
            if output.len() == expected_size {
                break;
            }
            if flags & (1 << bit) != 0 {
                let literal = stream.get(cursor..cursor + 3).ok_or(LzssError::Truncated {
                    field: "LZSS32 literal",
                })?;
                cursor += 3;
                output.extend_from_slice(&[literal[0], literal[1], literal[2], 255]);
            } else {
                let token_bytes = stream.get(cursor..cursor + 2).ok_or(LzssError::Truncated {
                    field: "LZSS32 back-reference",
                })?;
                cursor += 2;
                let token = u16::from_le_bytes(token_bytes.try_into().unwrap());
                let distance = usize::from(token >> 4) * 4;
                let length = (usize::from(token & 15) + 1) * 4;
                if distance == 0 || distance > output.len() {
                    return Err(LzssError::InvalidDistance { distance });
                }
                if output.len() + length > expected_size {
                    return Err(LzssError::OutputOverflow);
                }
                for _ in 0..length {
                    output.push(output[output.len() - distance]);
                }
            }
        }
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::{LzssError, decode};

    #[test]
    fn standard_lzss_cannot_read_past_declared_archive() {
        let mut block = Vec::new();
        block.extend_from_slice(&9u32.to_le_bytes());
        block.extend_from_slice(&1u32.to_le_bytes());
        block.extend_from_slice(&[1, 42]);
        assert!(matches!(
            decode(&block),
            Err(LzssError::Truncated { field: "literal" })
        ));
    }
}
