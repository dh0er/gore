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

    println!(
        "size_of: Property {} B, PropertyValue {} B, Descriptor {} B",
        std::mem::size_of::<gore_save::properties::Property>(),
        std::mem::size_of::<gore_save::properties::PropertyValue>(),
        std::mem::size_of::<gore_save::properties::Descriptor>(),
    );

    let mut tally = Tally::default();
    for run in 0..3 {
        let start = Instant::now();
        let root = gore_save::properties::parse_private_root(&payload).expect("parse");
        let elapsed = start.elapsed();
        if run == 0 {
            count_properties(&root.properties, &mut tally);
        }
        println!(
            "run {run}: {elapsed:?}  ({:.0} MB/s, {:.0} ns/property)",
            payload.len() as f64 / 1e6 / elapsed.as_secs_f64(),
            elapsed.as_nanos() as f64 / tally.properties.max(1) as f64
        );
    }
    let bytes = tally.properties * std::mem::size_of::<gore_save::properties::Property>();
    println!(
        "{} properties, {} strings, {} vectors, {} opaque bytes",
        tally.properties, tally.strings, tally.vectors, tally.opaque_bytes
    );
    println!(
        "~{:.0} MB of Property nodes, ~{} heap allocations",
        bytes as f64 / 1e6,
        tally.strings + tally.vectors
    );
}

#[derive(Default, Debug)]
struct Tally {
    properties: usize,
    strings: usize,
    vectors: usize,
    opaque_bytes: usize,
}

fn count_properties(properties: &[gore_save::properties::Property], tally: &mut Tally) {
    use gore_save::properties::Descriptor;
    tally.properties += properties.len();
    tally.vectors += 1;
    for property in properties {
        // name + type_name
        tally.strings += 2;
        let Descriptor {
            struct_type,
            enum_type,
            inner,
            map,
        } = &property.descriptor;
        tally.strings += struct_type.iter().count() * 2 + enum_type.iter().count() * 3;
        tally.strings += inner.iter().count() + map.iter().count() * 2;
        count_value(&property.value, tally);
    }
}

fn count_value(value: &gore_save::properties::PropertyValue, tally: &mut Tally) {
    use gore_save::properties::{PropertyValue as V, StructValue as S};
    match value {
        V::Str(_) | V::Name(_) | V::Object(_) | V::Enum(_) => tally.strings += 1,
        V::SoftObject(_) => tally.strings += 3,
        V::Opaque(bytes) => {
            tally.vectors += 1;
            tally.opaque_bytes += bytes.len();
        }
        V::Array { elements } | V::Set { elements, .. } => {
            tally.vectors += 1;
            for element in elements {
                count_value(element, tally);
            }
        }
        V::Map { entries, .. } => {
            tally.vectors += 1;
            for (key, entry) in entries {
                count_value(key, tally);
                count_value(entry, tally);
            }
        }
        V::ObjectInstances(instances) => {
            tally.vectors += 1;
            for instance in instances {
                tally.strings += 1;
                count_properties(&instance.properties, tally);
            }
        }
        V::Struct(S::Properties(inner)) => count_properties(inner, tally),
        V::Struct(S::GameplayTagContainer(tags)) => {
            tally.vectors += 1;
            tally.strings += tags.len();
        }
        V::Struct(S::Instanced(Some(instanced))) => {
            tally.strings += 1;
            count_properties(&instanced.properties, tally);
        }
        _ => {}
    }
}
