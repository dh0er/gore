use gore_as::cache::header::CacheHeader;
use gore_as::cache::scan::scan_strings;

fn fixture() -> Vec<u8> {
    std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/cache_head_8k.bin"
    ))
    .expect("fixture present")
}

#[test]
fn finds_known_type_names() {
    let bytes = fixture();
    let found = scan_strings(&bytes, CacheHeader::SIZE, 50);
    let texts: Vec<&str> = found.iter().map(|s| s.text.as_str()).collect();
    assert!(texts.contains(&"AI.AIItemScoring"), "got {texts:?}");
    assert!(
        texts.contains(&"UGothicAIItemActionScoringEntry"),
        "got {texts:?}"
    );
}

#[test]
fn first_hit_is_at_header_end() {
    let bytes = fixture();
    let found = scan_strings(&bytes, CacheHeader::SIZE, 1);
    assert_eq!(found[0].offset, 0x18);
    assert_eq!(found[0].text, "AI.AIItemScoring");
}
