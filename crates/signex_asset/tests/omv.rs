use signex_asset::{
    OMV_HEADER_SIZE, OMV_PACKET_SIZE, OMV_PAGE_SIZE, OmvError, OmvTheoraType, parse_omv,
};

fn header(theora_type: i32, width: i32, height: i32, pages: i32, packets: i32) -> Vec<u8> {
    let mut bytes = vec![0_u8; OMV_HEADER_SIZE];
    bytes[..4].copy_from_slice(&(OMV_HEADER_SIZE as i32).to_le_bytes());
    bytes[4] = 1;
    bytes[5] = 1;
    bytes[40..44].copy_from_slice(&theora_type.to_le_bytes());
    bytes[44..48].copy_from_slice(&width.to_le_bytes());
    bytes[48..52].copy_from_slice(&height.to_le_bytes());
    bytes[60..64].copy_from_slice(&33_333_i32.to_le_bytes());
    bytes[64..68].copy_from_slice(&1_168_i32.to_le_bytes());
    bytes[68..72].copy_from_slice(&0_i32.to_le_bytes());
    bytes[72..76].copy_from_slice(&1_i32.to_le_bytes());
    bytes[76..80].copy_from_slice(&pages.to_le_bytes());
    bytes[80..84].copy_from_slice(&packets.to_le_bytes());
    bytes
}

#[test]
fn parses_header_without_reading_lists_or_ogg_stream() {
    let bytes = header(0, 2, 3, 2, 5);
    let parsed = parse_omv(&bytes).unwrap();
    assert_eq!(parsed.header().theora_type, OmvTheoraType::Rgb);
    assert_eq!(parsed.header().width(), 2);
    assert_eq!(parsed.header().height(), 3);
    assert_eq!(parsed.header().us_per_frame, 33_333);
    assert_eq!(
        parsed.data_offset(),
        (OMV_HEADER_SIZE + 2 * OMV_PAGE_SIZE + 5 * OMV_PACKET_SIZE) as u64
    );
    assert_eq!(parsed, parse_omv(&bytes).unwrap());
}

#[test]
fn accepts_counts_above_old_item_limit_without_allocating_lists() {
    let parsed = parse_omv(&header(2, 4, 4, 1_000_001, i32::MAX)).unwrap();
    assert_eq!(parsed.header().theora_type, OmvTheoraType::Yuv);
    assert_eq!(parsed.header().page_count, 1_000_001);
    assert_eq!(parsed.header().packet_count, i32::MAX);
    assert_eq!(
        parsed.data_offset(),
        OMV_HEADER_SIZE as u64
            + 1_000_001 * OMV_PAGE_SIZE as u64
            + i32::MAX as u64 * OMV_PACKET_SIZE as u64
    );
}

#[test]
fn rejects_truncated_and_invalid_headers() {
    assert!(matches!(
        parse_omv(&[0; OMV_HEADER_SIZE - 1]),
        Err(OmvError::UnexpectedEof { .. })
    ));

    let mut bytes = header(0, 2, 3, 2, 5);
    bytes[..4].copy_from_slice(&167_i32.to_le_bytes());
    assert!(matches!(
        parse_omv(&bytes),
        Err(OmvError::InvalidValue {
            field: "OMV header size",
            ..
        })
    ));
    assert!(matches!(
        parse_omv(&header(3, 2, 3, 2, 5)),
        Err(OmvError::InvalidValue {
            field: "OMV Theora type",
            ..
        })
    ));
    let mut bytes = header(0, 2, 3, 2, 5);
    bytes[4] = 2;
    assert!(matches!(
        parse_omv(&bytes),
        Err(OmvError::InvalidValue {
            field: "OMV version",
            ..
        })
    ));
    assert!(matches!(
        parse_omv(&header(0, 0, 3, 2, 5)),
        Err(OmvError::InvalidValue {
            field: "OMV logical size",
            ..
        })
    ));
    assert!(matches!(
        parse_omv(&header(1, 2, 1, 2, 5)),
        Err(OmvError::InvalidValue {
            field: "OMV RGBA height",
            ..
        })
    ));
    assert!(matches!(
        parse_omv(&header(0, 2, 3, -1, 5)),
        Err(OmvError::InvalidValue {
            field: "OMV pages",
            ..
        })
    ));
    assert!(matches!(
        parse_omv(&header(0, 2, 3, 2, -1)),
        Err(OmvError::InvalidValue {
            field: "OMV packets",
            ..
        })
    ));
}
