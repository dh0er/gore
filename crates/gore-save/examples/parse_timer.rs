//! Scratch research driver: time a typed parse of a real save's private payload.
//! Read-only; writes nothing. Not shipped.

use std::time::Instant;

const PACKAGE_FILE_TAG: u32 = 0x9E2A83C1;

fn main() {
    let path = std::env::args().nth(1).expect("usage: parse_timer <save.sav>");
    let data = std::fs::read(&path).expect("read save");

    let tag = PACKAGE_FILE_TAG.to_le_bytes();
    let tag_at = (0..data.len() - 4)
        .find(|&i| data[i..i + 4] == tag)
        .expect("PACKAGE_FILE_TAG not found");
    let mut p = tag_at + 8; // skip tag + header version
    let max_chunk = u64::from_le_bytes(data[p..p + 8].try_into().unwrap()) as usize;
    p += 9; // max chunk + algorithm id
    let _sum_c = u64::from_le_bytes(data[p..p + 8].try_into().unwrap());
    p += 8;
    let sum_u = u64::from_le_bytes(data[p..p + 8].try_into().unwrap()) as usize;
    p += 8;
    let count = sum_u.div_ceil(max_chunk);
    let mut table = Vec::with_capacity(count);
    for _ in 0..count {
        let c = u64::from_le_bytes(data[p..p + 8].try_into().unwrap()) as usize;
        p += 8;
        let u = u64::from_le_bytes(data[p..p + 8].try_into().unwrap()) as usize;
        p += 8;
        table.push((c, u));
    }
    let mut payload = Vec::with_capacity(sum_u);
    let mut cursor = p;
    for (c, u) in table {
        payload.extend_from_slice(
            &gore_oodle::decompress(&data[cursor..cursor + c], u).expect("decode"),
        );
        cursor += c;
    }
    println!("payload: {} bytes", payload.len());

    for run in 0..3 {
        let start = Instant::now();
        let root = gore_save::properties::parse_private_root(&payload).expect("parse");
        let elapsed = start.elapsed();
        let count = count_properties(&root.properties);
        println!(
            "run {run}: {elapsed:?}  ({:.0} MB/s, {count} properties, {:.0} ns/property)",
            payload.len() as f64 / 1e6 / elapsed.as_secs_f64(),
            elapsed.as_nanos() as f64 / count as f64
        );
    }
}

fn count_properties(properties: &[gore_save::properties::Property]) -> usize {
    use gore_save::properties::PropertyValue as V;
    let mut total = properties.len();
    for property in properties {
        total += match &property.value {
            V::Struct(gore_save::properties::StructValue::Properties(inner)) => {
                count_properties(inner)
            }
            V::ObjectInstances(instances) => instances
                .iter()
                .map(|instance| count_properties(&instance.properties))
                .sum(),
            _ => 0,
        };
    }
    total
}
