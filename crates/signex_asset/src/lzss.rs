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
}

pub(crate) fn xor_cycle(bytes: &mut [u8], key: &[u8]) {
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte ^= key[index % key.len()];
    }
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
    let mut output = Vec::with_capacity(output_size);
    while output.len() < output_size {
        let flags = *input
            .get(offset)
            .ok_or(LzssError::Truncated { field: "flags" })?;
        offset += 1;
        for bit in 0..8 {
            if output.len() == output_size {
                break;
            }
            if flags & (1 << bit) != 0 {
                output.push(
                    *input
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
            let token = u16::from_le_bytes([input[offset], input[offset + 1]]);
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
