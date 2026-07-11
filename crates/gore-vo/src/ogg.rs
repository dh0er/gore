use std::collections::BTreeMap;

use crate::{Limits, OggError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OggCodec {
    Vorbis {
        channels: u8,
        sample_rate: u32,
    },
    Opus {
        channels: u8,
        input_sample_rate: u32,
    },
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OggInfo {
    pub codec: OggCodec,
    pub pages: usize,
    pub logical_streams: usize,
}

#[derive(Default)]
struct StreamState {
    next_sequence: u32,
    packet: Vec<u8>,
    first_packet_seen: bool,
    codec: Option<OggCodec>,
    eos: bool,
}

/// Validate the Ogg page graph, checksums, packet continuation, and an unambiguous
/// Vorbis/Opus identification header.
pub fn validate_ogg(data: &[u8], limits: &Limits) -> Result<OggInfo, OggError> {
    if data.is_empty() {
        return Err(OggError::Empty);
    }
    if data.len() > limits.max_ogg_bytes {
        return Err(OggError::LimitExceeded {
            kind: "stream bytes",
            actual: data.len(),
            limit: limits.max_ogg_bytes,
        });
    }

    let mut streams = BTreeMap::<u32, StreamState>::new();
    let mut offset = 0usize;
    let mut pages = 0usize;

    while offset < data.len() {
        pages += 1;
        if pages > limits.max_ogg_pages {
            return Err(OggError::LimitExceeded {
                kind: "page count",
                actual: pages,
                limit: limits.max_ogg_pages,
            });
        }
        if data.len() - offset < 27 {
            return Err(OggError::Truncated { offset });
        }
        if &data[offset..offset + 4] != b"OggS" {
            return Err(OggError::Capture { offset });
        }
        let version = data[offset + 4];
        if version != 0 {
            return Err(OggError::Version { offset, version });
        }
        let flags = data[offset + 5];
        if flags & !0x07 != 0 {
            return Err(OggError::HeaderFlags { offset, flags });
        }

        let segment_count = usize::from(data[offset + 26]);
        let header_len = 27usize
            .checked_add(segment_count)
            .ok_or(OggError::Truncated { offset })?;
        if data.len() - offset < header_len {
            return Err(OggError::Truncated { offset });
        }
        let lacing = &data[offset + 27..offset + header_len];
        let body_len = lacing
            .iter()
            .map(|value| usize::from(*value))
            .sum::<usize>();
        if body_len > limits.max_ogg_page_body_bytes {
            return Err(OggError::LimitExceeded {
                kind: "page body bytes",
                actual: body_len,
                limit: limits.max_ogg_page_body_bytes,
            });
        }
        let page_len = header_len
            .checked_add(body_len)
            .ok_or(OggError::Truncated { offset })?;
        if data.len() - offset < page_len {
            return Err(OggError::Truncated { offset });
        }
        let page = &data[offset..offset + page_len];
        let stored_checksum = u32::from_le_bytes(page[22..26].try_into().expect("fixed slice"));
        if ogg_crc(page) != stored_checksum {
            return Err(OggError::Checksum { offset });
        }

        let serial = u32::from_le_bytes(page[14..18].try_into().expect("fixed slice"));
        let sequence = u32::from_le_bytes(page[18..22].try_into().expect("fixed slice"));
        let is_new = !streams.contains_key(&serial);
        if is_new {
            if flags & 0x02 == 0 {
                return Err(OggError::MissingBos { serial });
            }
            if sequence != 0 {
                return Err(OggError::Sequence {
                    serial,
                    actual: sequence,
                    expected: 0,
                });
            }
            streams.insert(serial, StreamState::default());
        }

        let stream = streams.get_mut(&serial).expect("stream was inserted");
        if stream.eos {
            return Err(OggError::AfterEos { serial });
        }
        if !is_new && flags & 0x02 != 0 {
            return Err(OggError::UnexpectedBos { serial });
        }
        if sequence != stream.next_sequence {
            return Err(OggError::Sequence {
                serial,
                actual: sequence,
                expected: stream.next_sequence,
            });
        }
        stream.next_sequence = stream.next_sequence.wrapping_add(1);

        let is_continued = flags & 0x01 != 0;
        if is_continued == stream.packet.is_empty() {
            return Err(OggError::Continuation { serial });
        }

        let mut body_offset = header_len;
        for segment_len in lacing.iter().copied().map(usize::from) {
            let end = body_offset + segment_len;
            stream.packet.extend_from_slice(&page[body_offset..end]);
            if stream.packet.len() > limits.max_ogg_packet_bytes {
                return Err(OggError::LimitExceeded {
                    kind: "packet bytes",
                    actual: stream.packet.len(),
                    limit: limits.max_ogg_packet_bytes,
                });
            }
            body_offset = end;

            if segment_len < 255 {
                if !stream.first_packet_seen {
                    stream.codec = identify_packet(&stream.packet)?;
                    stream.first_packet_seen = true;
                }
                stream.packet.clear();
            }
        }

        if flags & 0x04 != 0 {
            if !stream.packet.is_empty() {
                return Err(OggError::IncompletePacket { serial });
            }
            stream.eos = true;
        }
        offset += page_len;
    }

    let mut recognized = Vec::new();
    for (serial, stream) in &streams {
        if !stream.packet.is_empty() {
            return Err(OggError::IncompletePacket { serial: *serial });
        }
        if !stream.eos {
            return Err(OggError::MissingEos { serial: *serial });
        }
        if !stream.first_packet_seen {
            return Err(OggError::Identification("logical stream has no packet"));
        }
        if let Some(codec) = &stream.codec {
            recognized.push(codec.clone());
        }
    }
    let codec = match recognized.as_slice() {
        [] => {
            return Err(OggError::Identification(
                "no Vorbis or Opus audio logical stream was found",
            ));
        }
        [codec] => codec.clone(),
        _ => return Err(OggError::MultipleAudioStreams),
    };

    Ok(OggInfo {
        codec,
        pages,
        logical_streams: streams.len(),
    })
}

fn identify_packet(packet: &[u8]) -> Result<Option<OggCodec>, OggError> {
    if packet.starts_with(b"\x01vorbis") {
        return identify_vorbis(packet).map(Some);
    }
    if packet.starts_with(b"OpusHead") {
        return identify_opus(packet).map(Some);
    }
    Ok(None)
}

fn identify_vorbis(packet: &[u8]) -> Result<OggCodec, OggError> {
    if packet.len() != 30 {
        return Err(OggError::Identification(
            "Vorbis identification header must be exactly 30 bytes",
        ));
    }
    let version = u32::from_le_bytes(packet[7..11].try_into().expect("fixed slice"));
    if version != 0 {
        return Err(OggError::Identification("unsupported Vorbis version"));
    }
    let channels = packet[11];
    let sample_rate = u32::from_le_bytes(packet[12..16].try_into().expect("fixed slice"));
    if channels == 0 || sample_rate == 0 {
        return Err(OggError::Identification(
            "Vorbis channels and sample rate must be non-zero",
        ));
    }
    let block_sizes = packet[28];
    let small = block_sizes & 0x0f;
    let large = block_sizes >> 4;
    if !(6..=13).contains(&small) || !(6..=13).contains(&large) || small > large {
        return Err(OggError::Identification("invalid Vorbis block sizes"));
    }
    if packet[29] & 0x01 == 0 {
        return Err(OggError::Identification(
            "Vorbis identification framing bit is not set",
        ));
    }
    Ok(OggCodec::Vorbis {
        channels,
        sample_rate,
    })
}

fn identify_opus(packet: &[u8]) -> Result<OggCodec, OggError> {
    if packet.len() < 19 {
        return Err(OggError::Identification("truncated OpusHead packet"));
    }
    let version = packet[8];
    if version == 0 || version & 0xf0 != 0 {
        return Err(OggError::Identification("unsupported OpusHead version"));
    }
    let channels = packet[9];
    if channels == 0 {
        return Err(OggError::Identification("Opus channel count is zero"));
    }
    let input_sample_rate = u32::from_le_bytes(packet[12..16].try_into().expect("fixed slice"));
    let mapping_family = packet[18];
    if mapping_family == 0 {
        if packet.len() != 19 || channels > 2 {
            return Err(OggError::Identification("invalid family-0 OpusHead packet"));
        }
    } else {
        let expected = 21usize + usize::from(channels);
        if packet.len() != expected {
            return Err(OggError::Identification(
                "invalid mapped OpusHead packet length",
            ));
        }
        let streams = packet[19];
        let coupled = packet[20];
        if streams == 0 || coupled > streams || u16::from(coupled) * 2 > u16::from(channels) {
            return Err(OggError::Identification(
                "invalid Opus stream/coupled counts",
            ));
        }
        let decoded_channels = u16::from(streams) + u16::from(coupled);
        if packet[21..]
            .iter()
            .any(|mapping| *mapping != u8::MAX && u16::from(*mapping) >= decoded_channels)
        {
            return Err(OggError::Identification("invalid Opus channel mapping"));
        }
    }
    Ok(OggCodec::Opus {
        channels,
        input_sample_rate,
    })
}

fn ogg_crc(page: &[u8]) -> u32 {
    let mut crc = 0u32;
    for (index, byte) in page.iter().copied().enumerate() {
        let byte = if (22..26).contains(&index) { 0 } else { byte };
        let table_index = ((crc >> 24) as u8 ^ byte) as usize;
        crc = (crc << 8) ^ OGG_CRC_TABLE[table_index];
    }
    crc
}

const OGG_CRC_TABLE: [u32; 256] = make_ogg_crc_table();

const fn make_ogg_crc_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    let mut index = 0usize;
    while index < table.len() {
        let mut value = (index as u32) << 24;
        let mut bit = 0;
        while bit < 8 {
            value = if value & 0x8000_0000 != 0 {
                (value << 1) ^ 0x04c1_1db7
            } else {
                value << 1
            };
            bit += 1;
        }
        table[index] = value;
        index += 1;
    }
    table
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn make_page(serial: u32, sequence: u32, flags: u8, packet: &[u8]) -> Vec<u8> {
        assert!(packet.len() < 255);
        make_page_with_lacing(serial, sequence, flags, &[packet.len() as u8], packet)
    }

    fn make_page_with_lacing(
        serial: u32,
        sequence: u32,
        flags: u8,
        lacing: &[u8],
        body: &[u8],
    ) -> Vec<u8> {
        assert_eq!(
            lacing
                .iter()
                .map(|value| usize::from(*value))
                .sum::<usize>(),
            body.len()
        );
        let mut page = Vec::with_capacity(27 + lacing.len() + body.len());
        page.extend_from_slice(b"OggS");
        page.push(0);
        page.push(flags);
        page.extend_from_slice(&0u64.to_le_bytes());
        page.extend_from_slice(&serial.to_le_bytes());
        page.extend_from_slice(&sequence.to_le_bytes());
        page.extend_from_slice(&0u32.to_le_bytes());
        page.push(lacing.len() as u8);
        page.extend_from_slice(lacing);
        page.extend_from_slice(body);
        let crc = ogg_crc(&page);
        page[22..26].copy_from_slice(&crc.to_le_bytes());
        page
    }

    pub(crate) fn vorbis_ogg(sample_rate: u32) -> Vec<u8> {
        let mut packet = Vec::with_capacity(30);
        packet.extend_from_slice(b"\x01vorbis");
        packet.extend_from_slice(&0u32.to_le_bytes());
        packet.push(1);
        packet.extend_from_slice(&sample_rate.to_le_bytes());
        packet.extend_from_slice(&0i32.to_le_bytes());
        packet.extend_from_slice(&0i32.to_le_bytes());
        packet.extend_from_slice(&0i32.to_le_bytes());
        packet.push(0x86);
        packet.push(1);
        make_page(7, 0, 0x02 | 0x04, &packet)
    }

    #[test]
    fn validates_vorbis_identification() {
        let info = validate_ogg(&vorbis_ogg(44_100), &Limits::default()).unwrap();
        assert_eq!(
            info.codec,
            OggCodec::Vorbis {
                channels: 1,
                sample_rate: 44_100
            }
        );
        assert_eq!(info.pages, 1);
    }

    #[test]
    fn validates_opus_identification() {
        let mut packet = b"OpusHead".to_vec();
        packet.extend_from_slice(&[1, 2]);
        packet.extend_from_slice(&312u16.to_le_bytes());
        packet.extend_from_slice(&48_000u32.to_le_bytes());
        packet.extend_from_slice(&0i16.to_le_bytes());
        packet.push(0);
        let data = make_page(9, 0, 0x02 | 0x04, &packet);
        let info = validate_ogg(&data, &Limits::default()).unwrap();
        assert_eq!(
            info.codec,
            OggCodec::Opus {
                channels: 2,
                input_sample_rate: 48_000
            }
        );
    }

    #[test]
    fn rejects_structurally_valid_ogg_without_recognized_audio() {
        let data = make_page(11, 0, 0x02 | 0x04, b"not a Vorbis or Opus header");
        assert!(matches!(
            validate_ogg(&data, &Limits::default()),
            Err(OggError::Identification(
                "no Vorbis or Opus audio logical stream was found"
            ))
        ));
    }

    #[test]
    fn rejects_bad_checksum_and_truncation() {
        let mut bad = vorbis_ogg(44_100);
        *bad.last_mut().unwrap() ^= 1;
        assert!(matches!(
            validate_ogg(&bad, &Limits::default()),
            Err(OggError::Checksum { .. })
        ));

        let truncated = &vorbis_ogg(44_100)[..26];
        assert!(matches!(
            validate_ogg(truncated, &Limits::default()),
            Err(OggError::Truncated { .. })
        ));
    }

    #[test]
    fn matches_known_ogg_crc_vector() {
        let page = hex_to_bytes("4f676753000600000000000000000100000000000000000000000103616263");
        assert_eq!(ogg_crc(&page), 0x98fb_7663);
    }

    #[test]
    fn enforces_ogg_limits() {
        let data = vorbis_ogg(44_100);
        let limits = Limits {
            max_ogg_bytes: data.len() - 1,
            ..Limits::default()
        };
        assert!(matches!(
            validate_ogg(&data, &limits),
            Err(OggError::LimitExceeded {
                kind: "stream bytes",
                ..
            })
        ));
    }

    #[test]
    fn validates_packet_continuation_across_pages() {
        let mut data = vorbis_ogg(44_100);
        data[5] &= !0x04;
        let crc = ogg_crc(&data);
        data[22..26].copy_from_slice(&crc.to_le_bytes());
        data.extend_from_slice(&make_page_with_lacing(7, 1, 0, &[255], &[1; 255]));
        data.extend_from_slice(&make_page_with_lacing(7, 2, 0x01 | 0x04, &[5], &[2; 5]));

        let info = validate_ogg(&data, &Limits::default()).unwrap();
        assert_eq!(info.pages, 3);
        assert_eq!(info.logical_streams, 1);
    }

    fn hex_to_bytes(hex: &str) -> Vec<u8> {
        hex.as_bytes()
            .chunks_exact(2)
            .map(|pair| {
                let text = std::str::from_utf8(pair).unwrap();
                u8::from_str_radix(text, 16).unwrap()
            })
            .collect()
    }
}
