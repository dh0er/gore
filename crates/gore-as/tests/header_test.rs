use gore_as::cache::header::{CacheHeader, HeaderError, CACHE_MAGIC};

fn fixture() -> Vec<u8> {
    std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/cache_head_8k.bin"
    ))
    .expect("fixture present")
}

#[test]
fn parses_outer_header() {
    let bytes = fixture();
    let h = CacheHeader::parse(&bytes).expect("header parses");
    assert_eq!(
        h.hash,
        [
            0xd5, 0x4f, 0x0f, 0xfb, 0x10, 0xc1, 0x05, 0x4b, 0x99, 0xf1, 0x14, 0x46, 0xa4, 0x3e,
            0xd5, 0xdc
        ]
    );
    assert_eq!(h.magic, CACHE_MAGIC);
    assert_eq!(h.magic, 0x9e37_7abe);
    assert_eq!(h.type_count, 7264);
}

#[test]
fn rejects_short_input() {
    let err = CacheHeader::parse(&[0u8; 10]).unwrap_err();
    assert!(matches!(err, HeaderError::TooShort { .. }));
}

#[test]
fn rejects_bad_magic() {
    let mut bytes = vec![0u8; CacheHeader::SIZE];
    // valid length, wrong magic at 0x10
    bytes[16..20].copy_from_slice(&0xdead_beef_u32.to_le_bytes());
    let err = CacheHeader::parse(&bytes).unwrap_err();
    assert!(matches!(err, HeaderError::BadMagic { .. }));
}
