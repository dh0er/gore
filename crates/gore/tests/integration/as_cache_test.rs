use assert_cmd::Command;
use predicates::str::contains;

fn fixture_path() -> String {
    concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/cache_head_8k.bin").to_string()
}

#[test]
fn decode_header_prints_values() {
    Command::cargo_bin("gore")
        .unwrap()
        .args(["as", "decode-header", &fixture_path()])
        .assert()
        .success()
        .stdout(contains("magic      : 0x9e377abe"))
        .stdout(contains("type_count : 7264"))
        .stdout(contains("d54f0ffb10c1054b99f11446a43ed5dc"));
}
