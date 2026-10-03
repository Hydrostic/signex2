//! Binary `Gameexe.dat` decryption and parsing.

use super::{GameexeError, GameexeNode, parse};
use crate::lzss::{LzssError, decode as decode_lzss, xor_cycle};
use std::time::Instant;

use tracing::trace;

const HEADER_SIZE: usize = 8;

#[derive(Debug, thiserror::Error)]
pub enum GameexeDecodeError {
    #[error("Gameexe.dat is truncated: need {needed} bytes, got {actual}")]
    Truncated { needed: usize, actual: usize },
    #[error("Gameexe.dat LZSS error: {0}")]
    Lzss(#[from] LzssError),
    #[error("Gameexe.dat UTF-16LE error")]
    Utf16,
    #[error("Gameexe.ini parse error: {0}")]
    Parse(#[from] GameexeError),
    #[error("Gameexe.dat requires the 16-byte EXE_EL key")]
    MissingKey,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedGameexe {
    pub version: i32,
    pub exe_el_mode: bool,
    pub text: String,
}

/// Decrypt and decode a binary Gameexe.dat into UTF-8 Gameexe.ini text.
pub fn decode_gameexe(
    bytes: &[u8],
    exe_el: Option<&[u8; 16]>,
) -> Result<DecodedGameexe, GameexeDecodeError> {
    if bytes.len() < HEADER_SIZE {
        return Err(GameexeDecodeError::Truncated {
            needed: HEADER_SIZE,
            actual: bytes.len(),
        });
    }
    let version = i32::from_le_bytes(bytes[0..4].try_into().expect("header checked"));
    let mode = i32::from_le_bytes(bytes[4..8].try_into().expect("header checked"));
    let mut payload = bytes[HEADER_SIZE..].to_vec();
    if mode != 0 {
        let key = exe_el.ok_or(GameexeDecodeError::MissingKey)?;
        xor_cycle(&mut payload, key);
    }
    xor_cycle(&mut payload, &super::angou::DAT_ANGOU);
    let utf16 = decode_lzss(&payload)?;
    if !utf16.len().is_multiple_of(2) {
        return Err(GameexeDecodeError::Utf16);
    }
    let units = utf16
        .chunks_exact(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]));
    let text =
        String::from_utf16(&units.collect::<Vec<_>>()).map_err(|_| GameexeDecodeError::Utf16)?;
    Ok(DecodedGameexe {
        version,
        exe_el_mode: mode != 0,
        text,
    })
}

/// Decrypt, decode, and parse a typed Gameexe model.
pub fn decrypt_and_parse_gameexe<T: GameexeNode>(
    bytes: &[u8],
    exe_el: Option<&[u8; 16]>,
) -> Result<T, GameexeDecodeError> {
    let decode_started = Instant::now();
    let decoded = decode_gameexe(bytes, exe_el)?;
    trace!(
        elapsed = ?decode_started.elapsed(),
        input_bytes = bytes.len(),
        decoded_text_bytes = decoded.text.len(),
        "decrypt and decode Gameexe.dat"
    );

    let parse_started = Instant::now();
    let parsed = parse::<T>(&decoded.text)?;
    trace!(
        elapsed = ?parse_started.elapsed(),
        text_bytes = decoded.text.len(),
        "parse Gameexe.ini"
    );
    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encode_literals(input: &[u8]) -> Vec<u8> {
        let groups = input.len().div_ceil(8);
        let archive_size = 8 + input.len() + groups;
        let mut output = Vec::with_capacity(archive_size);
        output.extend_from_slice(&(archive_size as u32).to_le_bytes());
        output.extend_from_slice(&(input.len() as u32).to_le_bytes());
        for chunk in input.chunks(8) {
            output.push(u8::MAX >> (8 - chunk.len()));
            output.extend_from_slice(chunk);
        }
        output
    }

    fn build_blob(text: &str, mode: i32, key: &[u8; 16]) -> Vec<u8> {
        let utf16 = text
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect::<Vec<_>>();
        let mut payload = encode_literals(&utf16);
        if mode != 0 {
            xor_cycle(&mut payload, key);
        }
        xor_cycle(&mut payload, &super::super::angou::DAT_ANGOU);
        let mut blob = Vec::new();
        blob.extend_from_slice(&1_i32.to_le_bytes());
        blob.extend_from_slice(&mode.to_le_bytes());
        blob.extend_from_slice(&payload);
        blob
    }

    #[test]
    fn decodes_plain_gameexe_payload() {
        let key = [0xabu8; 16];
        let decoded = decode_gameexe(&build_blob("#GAMENAME=\"demo\"\n", 0, &key), None).unwrap();
        assert_eq!(decoded.version, 1);
        assert!(!decoded.exe_el_mode);
        assert_eq!(decoded.text, "#GAMENAME=\"demo\"\n");
    }

    #[test]
    fn applies_exe_el_before_fixed_dat_key() {
        let key = [0x53u8; 16];
        let blob = build_blob("#GAMENAME=\"keyed\"\n", 2, &key);
        assert_eq!(
            decode_gameexe(&blob, Some(&key)).unwrap().text,
            "#GAMENAME=\"keyed\"\n"
        );
        assert!(matches!(
            decode_gameexe(&blob, None),
            Err(GameexeDecodeError::MissingKey)
        ));
    }

    #[test]
    fn rejects_truncated_header() {
        assert!(matches!(
            decode_gameexe(&[0; 7], None),
            Err(GameexeDecodeError::Truncated { .. })
        ));
    }
}
