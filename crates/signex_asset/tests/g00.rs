use signex_asset::{G00Error, G00Kind, G00Rect, decode_g00};

fn lzss_literals(plain: &[u8]) -> Vec<u8> {
    let archive_size = 8 + plain.len() + plain.len().div_ceil(8);
    let mut block = Vec::new();
    block.extend_from_slice(&(archive_size as u32).to_le_bytes());
    block.extend_from_slice(&(plain.len() as u32).to_le_bytes());
    for group in plain.chunks(8) {
        block.push(((1u16 << group.len()) - 1) as u8);
        block.extend_from_slice(group);
    }
    block
}

#[test]
fn opaque_literals_and_overlapping_back_reference() {
    let mut file = vec![0];
    file.extend_from_slice(&4u16.to_le_bytes());
    file.extend_from_slice(&1u16.to_le_bytes());
    file.extend_from_slice(&14u32.to_le_bytes());
    file.extend_from_slice(&16u32.to_le_bytes());
    file.extend_from_slice(&[1, 3, 2, 1, 0x12, 0]);

    let image = decode_g00(&file).unwrap();
    assert_eq!(image.kind, G00Kind::Opaque);
    assert_eq!(
        image.cuts[0].rect,
        G00Rect {
            left: 0,
            top: 0,
            right: 4,
            bottom: 1
        }
    );
    assert_eq!(image.cuts[0].chips[0].pixels, [3, 2, 1, 255].repeat(4));
}

#[test]
fn palette_expands_bgra_and_rejects_missing_color() {
    let mut plain = Vec::new();
    plain.extend_from_slice(&2u16.to_le_bytes());
    plain.extend_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]);
    plain.extend_from_slice(&[1, 0]);
    let mut file = vec![1, 2, 0, 1, 0];
    file.extend_from_slice(&lzss_literals(&plain));
    let image = decode_g00(&file).unwrap();
    assert_eq!(image.kind, G00Kind::Paletted);
    assert_eq!(image.cuts[0].chips[0].pixels, [5, 6, 7, 8, 1, 2, 3, 4]);

    plain[11] = 2;
    file.truncate(5);
    file.extend_from_slice(&lzss_literals(&plain));
    assert!(matches!(
        decode_g00(&file),
        Err(G00Error::InvalidValue {
            field: "palette index",
            ..
        })
    ));
}

fn cut_payload() -> Vec<u8> {
    let mut cut = vec![0, 0];
    cut.extend_from_slice(&1u16.to_le_bytes());
    for value in [-3i32, 4, 2, 1, 7, 8, 2, 1] {
        cut.extend_from_slice(&value.to_le_bytes());
    }
    cut.extend_from_slice(&[0; 80]);
    cut.extend_from_slice(&1u16.to_le_bytes());
    cut.extend_from_slice(&0u16.to_le_bytes());
    cut.extend_from_slice(&[1, 0]);
    cut.extend_from_slice(&2u16.to_le_bytes());
    cut.extend_from_slice(&1u16.to_le_bytes());
    cut.extend_from_slice(&0u16.to_le_bytes());
    cut.extend_from_slice(&[0; 80]);
    cut.extend_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]);
    assert_eq!(cut.len(), 216);
    cut
}

fn cut_file(second_offset: u32, second_size: i32) -> Vec<u8> {
    let mut plain = Vec::new();
    plain.extend_from_slice(&2u32.to_le_bytes());
    plain.extend_from_slice(&20u32.to_le_bytes());
    plain.extend_from_slice(&216i32.to_le_bytes());
    plain.extend_from_slice(&second_offset.to_le_bytes());
    plain.extend_from_slice(&second_size.to_le_bytes());
    plain.extend_from_slice(&cut_payload());
    let mut file = vec![2, 16, 0, 16, 0];
    file.extend_from_slice(&2i32.to_le_bytes());
    file.extend_from_slice(&[0; 48]);
    file.extend_from_slice(&lzss_literals(&plain));
    file
}

#[test]
fn cut_geometry_pixels_and_negative_size_link() {
    let image = decode_g00(&cut_file(20, -1)).unwrap();
    assert_eq!(image.kind, G00Kind::Cut);
    assert_eq!(image.cuts[0], image.cuts[1]);
    let cut = &image.cuts[0];
    assert_eq!(cut.center, (7, 8));
    assert_eq!(
        cut.rect,
        G00Rect {
            left: -3,
            top: 4,
            right: -1,
            bottom: 5
        }
    );
    assert_eq!(cut.chips[0].pixels, [1, 2, 3, 4, 5, 6, 7, 8]);
    assert!(cut.chips[0].sprite);
}

#[test]
fn cut_link_outside_plaintext_fails() {
    assert!(matches!(
        decode_g00(&cut_file(1000, -1)),
        Err(G00Error::InvalidRange {
            field: "linked cut",
            ..
        })
    ));
    assert!(matches!(
        decode_g00(&cut_file(236, -1)),
        Err(G00Error::InvalidRange {
            field: "linked cut",
            ..
        })
    ));
    assert!(matches!(
        decode_g00(&cut_file(20, i32::MAX)),
        Err(G00Error::InvalidRange {
            field: "cut data",
            ..
        })
    ));
}

#[test]
fn malformed_header_and_unknown_kind_fail() {
    assert!(matches!(
        decode_g00(&[0, 1]),
        Err(G00Error::Truncated { .. })
    ));
    assert!(matches!(
        decode_g00(&[9]),
        Err(G00Error::UnsupportedKind(9))
    ));
}

#[test]
fn zero_sized_opaque_image_has_no_pixels() {
    let mut file = vec![0, 0, 0, 1, 0];
    file.extend_from_slice(&8u32.to_le_bytes());
    file.extend_from_slice(&0u32.to_le_bytes());
    let image = decode_g00(&file).unwrap();
    assert!(image.cuts[0].chips[0].pixels.is_empty());
}
