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

/// Codec timing and decode-assurance facts from complete Ogg validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OggTiming {
    /// Exact playable sample frames per channel after codec start/end trimming.
    ///
    /// One frame contains one sample for every channel. Interpret this integer only together with
    /// [`Self::duration_timebase_hz`]; no floating-point rounding is involved.
    pub duration_sample_frames: u64,
    /// Clock rate for [`Self::duration_sample_frames`]. Vorbis uses its identification-header
    /// sample rate. Ogg Opus always uses the normative 48 kHz playback clock, independently of the
    /// informational input sample rate carried by `OpusHead`.
    pub duration_timebase_hz: u32,
    /// Whether validation decoded the complete compressed audio stream to PCM.
    ///
    /// This is true for Vorbis. It is false for Opus because Opus validation proves packet framing
    /// and timing/granule consistency without decoding the SILK/CELT payload. Neither value is a
    /// loudness, perceptual-quality, desktop-audibility, or in-game qualification claim.
    pub pcm_decode_complete: bool,
}

/// Backward-compatible Ogg metadata plus additive timing facts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OggValidation {
    pub info: OggInfo,
    pub timing: OggTiming,
}

#[derive(Default)]
struct StreamState {
    next_sequence: u32,
    packet: Vec<u8>,
    first_packet_seen: bool,
    codec: Option<OggCodec>,
    completed_packets: usize,
    audio_packets: usize,
    opus_pre_skip: u16,
    opus_samples: u64,
    opus_granule_origin: Option<u64>,
    last_audio_granule: Option<u64>,
    eos_granule: Option<u64>,
    bos_completed_packets: usize,
    bos_granule: u64,
    bos_was_eos: bool,
    bos_had_partial_packet: bool,
    page_ranges: Vec<(usize, usize)>,
    eos: bool,
}

/// Validate an Ogg voice asset conservatively.
///
/// In addition to the page graph, checksums, and packet continuation, this requires exactly one
/// recognized audio stream, its complete codec-header sequence, at least one audio packet, and a
/// positive end granule. Vorbis is decode-probed to the end with `lewton` and must yield PCM.
/// Opus packet framing, packet durations, and granule positions are checked, but the compressed
/// SILK/CELT payload is not decoded. Consequently, successful Opus validation is strong structural
/// decodability evidence, not proof that every decoder can play the asset.
pub fn validate_ogg(data: &[u8], limits: &Limits) -> Result<OggInfo, OggError> {
    validate_ogg_with_timing(data, limits).map(|validation| validation.info)
}

/// Validate an Ogg voice asset and also derive exact codec timing.
pub fn validate_ogg_with_timing(data: &[u8], limits: &Limits) -> Result<OggValidation, OggError> {
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
        stream.page_ranges.push((offset, page_len));

        let granule = u64::from_le_bytes(page[6..14].try_into().expect("fixed slice"));
        let audio_packets_before_page = stream.audio_packets;

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
                    if stream.codec.is_some()
                        && stream.packet.len() > limits.max_ogg_codec_header_bytes
                    {
                        return Err(OggError::LimitExceeded {
                            kind: "codec header bytes",
                            actual: stream.packet.len(),
                            limit: limits.max_ogg_codec_header_bytes,
                        });
                    }
                    if matches!(&stream.codec, Some(OggCodec::Opus { .. })) {
                        stream.opus_pre_skip = u16::from_le_bytes(
                            stream.packet[10..12]
                                .try_into()
                                .expect("validated OpusHead"),
                        );
                    }
                    stream.first_packet_seen = true;
                } else if let Some(codec) = &stream.codec {
                    validate_codec_packet(
                        codec,
                        serial,
                        stream.completed_packets,
                        &stream.packet,
                        &mut stream.audio_packets,
                        &mut stream.opus_samples,
                        limits,
                    )?;
                }
                stream.completed_packets =
                    stream
                        .completed_packets
                        .checked_add(1)
                        .ok_or(OggError::LimitExceeded {
                            kind: "packet count",
                            actual: usize::MAX,
                            limit: limits.max_ogg_pages.saturating_mul(255),
                        })?;
                if sequence == 0 {
                    stream.bos_completed_packets += 1;
                }
                stream.packet.clear();
            }
        }

        if sequence == 0 {
            stream.bos_granule = granule;
            stream.bos_was_eos = flags & 0x04 != 0;
            stream.bos_had_partial_packet = !stream.packet.is_empty();
        }

        if matches!(&stream.codec, Some(OggCodec::Opus { .. })) {
            validate_opus_page_granule(
                serial,
                flags,
                granule,
                audio_packets_before_page,
                stream.audio_packets,
                stream.opus_samples,
                stream.completed_packets >= 2 && !stream.packet.is_empty(),
                &mut stream.opus_granule_origin,
                &mut stream.last_audio_granule,
            )?;
        }

        if flags & 0x04 != 0 {
            if !stream.packet.is_empty() {
                return Err(OggError::IncompletePacket { serial });
            }
            stream.eos_granule = Some(granule);
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
            let timing = validate_complete_audio_stream(*serial, stream, codec, data, limits)?;
            recognized.push((*serial, codec.clone(), timing));
        }
    }
    let (codec, timing) = match recognized.as_slice() {
        [] => {
            return Err(OggError::Identification(
                "no Vorbis or Opus audio logical stream was found",
            ));
        }
        [(_, codec, timing)] => (codec.clone(), *timing),
        _ => return Err(OggError::MultipleAudioStreams),
    };

    Ok(OggValidation {
        info: OggInfo {
            codec,
            pages,
            logical_streams: streams.len(),
        },
        timing,
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
    if channels > 2 {
        return Err(OggError::Identification(
            "Vorbis voice assets support at most two channels",
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
    if mapping_family != 0 {
        return Err(OggError::Identification(
            "mapped/multistream Opus requires unsupported self-delimited packet parsing",
        ));
    }
    if packet.len() != 19 || channels > 2 {
        return Err(OggError::Identification("invalid family-0 OpusHead packet"));
    }
    Ok(OggCodec::Opus {
        channels,
        input_sample_rate,
    })
}

fn validate_codec_packet(
    codec: &OggCodec,
    serial: u32,
    packet_index: usize,
    packet: &[u8],
    audio_packets: &mut usize,
    opus_samples: &mut u64,
    limits: &Limits,
) -> Result<(), OggError> {
    let malformed = |reason| OggError::AudioStructure { serial, reason };
    let is_header = match codec {
        OggCodec::Vorbis { .. } => matches!(packet_index, 1 | 2),
        OggCodec::Opus { .. } => packet_index == 1,
        OggCodec::Unknown => false,
    };
    if is_header && packet.len() > limits.max_ogg_codec_header_bytes {
        return Err(OggError::LimitExceeded {
            kind: "codec header bytes",
            actual: packet.len(),
            limit: limits.max_ogg_codec_header_bytes,
        });
    }

    let mut count_audio_packet = || -> Result<(), OggError> {
        let actual = audio_packets
            .checked_add(1)
            .ok_or(OggError::LimitExceeded {
                kind: "audio packet count",
                actual: usize::MAX,
                limit: limits.max_ogg_audio_packets,
            })?;
        if actual > limits.max_ogg_audio_packets {
            return Err(OggError::LimitExceeded {
                kind: "audio packet count",
                actual,
                limit: limits.max_ogg_audio_packets,
            });
        }
        *audio_packets = actual;
        Ok(())
    };
    match codec {
        OggCodec::Vorbis { .. } => match packet_index {
            1 if !packet.starts_with(b"\x03vorbis") => {
                Err(malformed("missing or malformed Vorbis comment header"))
            }
            2 if !packet.starts_with(b"\x05vorbis") => {
                Err(malformed("missing or malformed Vorbis setup header"))
            }
            2 => preflight_vorbis_setup(packet, serial, limits),
            1 => Ok(()),
            _ => {
                if packet.is_empty() || packet[0] & 1 != 0 {
                    return Err(malformed("malformed Vorbis audio packet"));
                }
                count_audio_packet()?;
                Ok(())
            }
        },
        OggCodec::Opus { .. } => {
            if packet_index == 1 {
                validate_opus_tags(packet).map_err(malformed)
            } else {
                let samples = opus_packet_samples(packet).map_err(malformed)?;
                *opus_samples = opus_samples
                    .checked_add(samples)
                    .ok_or_else(|| malformed("Opus sample duration overflowed"))?;
                count_audio_packet()?;
                Ok(())
            }
        }
        OggCodec::Unknown => Err(malformed("unsupported recognized audio codec")),
    }
}

/// Parse the allocation-driving portion of a Vorbis setup header before handing it to Lewton.
///
/// A setup packet can encode an ordered codebook with millions of entries in only a few bytes.
/// Lewton 0.10 allocates the declared codeword-length array and constructs a heap trie before it
/// can reject an invalid or excessive setup. This preflight mirrors the Vorbis codebook bit layout
/// without allocating from attacker-controlled counts and rejects every truncation or ambiguity.
fn preflight_vorbis_setup(packet: &[u8], serial: u32, limits: &Limits) -> Result<(), OggError> {
    let malformed = |reason| OggError::AudioStructure { serial, reason };
    let payload = packet
        .strip_prefix(b"\x05vorbis")
        .ok_or_else(|| malformed("missing or malformed Vorbis setup header"))?;
    let mut bits = VorbisSetupBits::new(payload);
    let codebook_count = usize::try_from(read_vorbis_setup_bits(&mut bits, 8, serial)?)
        .expect("eight bits fit usize")
        + 1;
    let mut total_entries = 0usize;
    let mut total_tree_work = 0usize;
    let mut total_vq_scalars = 0usize;

    for _ in 0..codebook_count {
        if read_vorbis_setup_bits(&mut bits, 24, serial)? != 0x56_43_42 {
            return Err(malformed("invalid Vorbis setup codebook sync pattern"));
        }
        let dimensions = usize::try_from(read_vorbis_setup_bits(&mut bits, 16, serial)?)
            .expect("sixteen bits fit usize");
        let entries = usize::try_from(read_vorbis_setup_bits(&mut bits, 24, serial)?)
            .expect("twenty-four bits fit usize");
        if dimensions == 0 || entries == 0 {
            return Err(malformed(
                "Vorbis setup codebook dimensions and entries must be non-zero",
            ));
        }
        let ordered = read_vorbis_setup_bits(&mut bits, 1, serial)? != 0;
        add_vorbis_preflight_total(
            &mut total_entries,
            entries,
            "Vorbis codebook entries",
            limits.max_vorbis_codebook_entries,
        )?;
        // One root plus the sum of active codeword lengths bounds both the number of transient trie
        // nodes and the insertion work, including malformed under-populated trees.
        add_vorbis_preflight_total(
            &mut total_tree_work,
            1,
            "Vorbis Huffman tree nodes",
            limits.max_vorbis_huffman_tree_nodes,
        )?;

        let mut active_entries = 0usize;
        if ordered {
            let mut current_entry = 0usize;
            let mut current_length = usize::try_from(read_vorbis_setup_bits(&mut bits, 5, serial)?)
                .expect("five bits fit usize")
                + 1;
            while current_entry < entries {
                if current_length > 32 {
                    return Err(malformed(
                        "ordered Vorbis codebook exceeds the 32-bit codeword limit",
                    ));
                }
                let remaining = entries - current_entry;
                let width = vorbis_ilog(remaining);
                let count = usize::try_from(read_vorbis_setup_bits(&mut bits, width, serial)?)
                    .expect("at most twenty-four bits fit usize");
                if count > remaining {
                    return Err(malformed(
                        "ordered Vorbis codebook run exceeds its declared entries",
                    ));
                }
                let work = count
                    .checked_mul(current_length)
                    .ok_or(OggError::LimitExceeded {
                        kind: "Vorbis Huffman tree nodes",
                        actual: usize::MAX,
                        limit: limits.max_vorbis_huffman_tree_nodes,
                    })?;
                add_vorbis_preflight_total(
                    &mut total_tree_work,
                    work,
                    "Vorbis Huffman tree nodes",
                    limits.max_vorbis_huffman_tree_nodes,
                )?;
                active_entries =
                    active_entries
                        .checked_add(count)
                        .ok_or(OggError::LimitExceeded {
                            kind: "Vorbis codebook entries",
                            actual: usize::MAX,
                            limit: limits.max_vorbis_codebook_entries,
                        })?;
                current_entry += count;
                current_length += 1;
            }
        } else {
            let sparse = read_vorbis_setup_bits(&mut bits, 1, serial)? != 0;
            for _ in 0..entries {
                let present = !sparse || read_vorbis_setup_bits(&mut bits, 1, serial)? != 0;
                if present {
                    let length = usize::try_from(read_vorbis_setup_bits(&mut bits, 5, serial)?)
                        .expect("five bits fit usize")
                        + 1;
                    add_vorbis_preflight_total(
                        &mut total_tree_work,
                        length,
                        "Vorbis Huffman tree nodes",
                        limits.max_vorbis_huffman_tree_nodes,
                    )?;
                    active_entries += 1;
                }
            }
        }
        if active_entries == 0 {
            return Err(malformed("Vorbis setup codebook has no active entries"));
        }

        let lookup_type = read_vorbis_setup_bits(&mut bits, 4, serial)?;
        if lookup_type > 2 {
            return Err(malformed("invalid Vorbis codebook lookup type"));
        }
        if lookup_type != 0 {
            // Lewton materializes entries*dimensions f32 values for both lookup types, even though
            // lookup type 1 encodes fewer multiplicands in the packet.
            let materialized = entries
                .checked_mul(dimensions)
                .ok_or(OggError::LimitExceeded {
                    kind: "Vorbis VQ scalars",
                    actual: usize::MAX,
                    limit: limits.max_vorbis_vq_scalars,
                })?;
            add_vorbis_preflight_total(
                &mut total_vq_scalars,
                materialized,
                "Vorbis VQ scalars",
                limits.max_vorbis_vq_scalars,
            )?;

            if !bits.skip(64) {
                return Err(malformed("truncated Vorbis setup codebook lookup header"));
            }
            let value_bits = usize::try_from(read_vorbis_setup_bits(&mut bits, 4, serial)?)
                .expect("four bits fit usize")
                + 1;
            read_vorbis_setup_bits(&mut bits, 1, serial)?; // sequence flag
            let encoded_values = if lookup_type == 1 {
                vorbis_lookup1_values(entries, dimensions)
            } else {
                materialized
            };
            let encoded_bits =
                encoded_values
                    .checked_mul(value_bits)
                    .ok_or(OggError::LimitExceeded {
                        kind: "Vorbis VQ scalar bits",
                        actual: usize::MAX,
                        limit: packet.len().saturating_mul(8),
                    })?;
            if !bits.skip(encoded_bits) {
                return Err(malformed("truncated Vorbis setup codebook multiplicands"));
            }
        }
    }
    Ok(())
}

fn add_vorbis_preflight_total(
    total: &mut usize,
    addition: usize,
    kind: &'static str,
    limit: usize,
) -> Result<(), OggError> {
    let actual = total.checked_add(addition).ok_or(OggError::LimitExceeded {
        kind,
        actual: usize::MAX,
        limit,
    })?;
    if actual > limit {
        return Err(OggError::LimitExceeded {
            kind,
            actual,
            limit,
        });
    }
    *total = actual;
    Ok(())
}

fn read_vorbis_setup_bits(
    bits: &mut VorbisSetupBits<'_>,
    count: u8,
    serial: u32,
) -> Result<u32, OggError> {
    bits.read(count).ok_or(OggError::AudioStructure {
        serial,
        reason: "truncated Vorbis setup codebook preflight",
    })
}

fn vorbis_ilog(value: usize) -> u8 {
    (usize::BITS - value.leading_zeros()) as u8
}

fn vorbis_lookup1_values(entries: usize, dimensions: usize) -> usize {
    if dimensions >= usize::BITS as usize {
        return 1;
    }
    let mut low = 1usize;
    let mut high = entries;
    while low < high {
        let candidate = low + (high - low).div_ceil(2);
        if power_leq(candidate, dimensions, entries) {
            low = candidate;
        } else {
            high = candidate - 1;
        }
    }
    low
}

fn power_leq(mut base: usize, mut exponent: usize, limit: usize) -> bool {
    let mut value = 1usize;
    while exponent != 0 {
        if exponent & 1 != 0 {
            let Some(product) = value.checked_mul(base) else {
                return false;
            };
            if product > limit {
                return false;
            }
            value = product;
        }
        exponent >>= 1;
        if exponent != 0 {
            let Some(square) = base.checked_mul(base) else {
                return false;
            };
            if square > limit {
                return false;
            }
            base = square;
        }
    }
    true
}

struct VorbisSetupBits<'a> {
    bytes: &'a [u8],
    bit_offset: usize,
}

impl<'a> VorbisSetupBits<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            bit_offset: 0,
        }
    }

    fn read(&mut self, count: u8) -> Option<u32> {
        if count > 32 {
            return None;
        }
        let count = usize::from(count);
        let end = self.bit_offset.checked_add(count)?;
        if end > self.bytes.len().checked_mul(8)? {
            return None;
        }
        let mut value = 0u32;
        for shift in 0..count {
            let absolute = self.bit_offset + shift;
            let bit = (self.bytes[absolute / 8] >> (absolute % 8)) & 1;
            value |= u32::from(bit) << shift;
        }
        self.bit_offset = end;
        Some(value)
    }

    fn skip(&mut self, count: usize) -> bool {
        let Some(end) = self.bit_offset.checked_add(count) else {
            return false;
        };
        let Some(total_bits) = self.bytes.len().checked_mul(8) else {
            return false;
        };
        if end > total_bits {
            return false;
        }
        self.bit_offset = end;
        true
    }
}

fn validate_complete_audio_stream(
    serial: u32,
    stream: &StreamState,
    codec: &OggCodec,
    data: &[u8],
    limits: &Limits,
) -> Result<OggTiming, OggError> {
    let malformed = |reason| OggError::AudioStructure { serial, reason };
    if stream.bos_completed_packets != 1 || stream.bos_had_partial_packet {
        return Err(malformed(
            "audio BOS page must contain exactly one complete identification header",
        ));
    }
    if stream.bos_granule != 0 {
        return Err(malformed("audio BOS page granule must be zero"));
    }
    if stream.bos_was_eos {
        return Err(malformed(
            "audio identification page cannot also be the EOS page",
        ));
    }
    let eos_granule = stream
        .eos_granule
        .filter(|granule| *granule != u64::MAX && *granule > 0)
        .ok_or_else(|| malformed("audio EOS page has no positive duration granule"))?;

    match codec {
        OggCodec::Vorbis {
            channels,
            sample_rate,
        } => {
            if stream.completed_packets < 4 || stream.audio_packets == 0 {
                return Err(malformed(
                    "Vorbis requires identification, comment, setup, and audio packets",
                ));
            }
            let timeline = decode_probe_vorbis(serial, *channels, stream, data, limits)?;
            let duration_sample_frames = validate_vorbis_timing(serial, &timeline)?;
            Ok(OggTiming {
                duration_sample_frames,
                duration_timebase_hz: *sample_rate,
                pcm_decode_complete: true,
            })
        }
        OggCodec::Opus { .. } => {
            if stream.completed_packets < 3 || stream.audio_packets == 0 {
                return Err(malformed(
                    "Opus requires OpusHead, OpusTags, and audio packets",
                ));
            }
            let relative_eos = eos_granule
                .checked_sub(stream.opus_granule_origin.unwrap_or(0))
                .ok_or_else(|| malformed("Opus EOS granule precedes its granule origin"))?;
            if relative_eos <= u64::from(stream.opus_pre_skip) {
                return Err(malformed(
                    "Opus EOS granule leaves no positive duration after pre-skip",
                ));
            }
            if relative_eos > stream.opus_samples {
                return Err(malformed(
                    "Opus EOS granule exceeds the parsed packet duration",
                ));
            }
            let duration_sample_frames = relative_eos
                .checked_sub(u64::from(stream.opus_pre_skip))
                .filter(|frames| *frames > 0)
                .ok_or_else(|| {
                    malformed("Opus duration underflowed while applying its pre-skip")
                })?;
            Ok(OggTiming {
                duration_sample_frames,
                duration_timebase_hz: 48_000,
                pcm_decode_complete: false,
            })
        }
        OggCodec::Unknown => Err(malformed("unsupported recognized audio codec")),
    }
}

#[derive(Debug, Clone, Copy)]
struct VorbisAudioPageTiming {
    granule: u64,
    audio_packets_before: usize,
    audio_packets_after: usize,
    audio_packet_in_progress: bool,
    eos: bool,
}

struct DecodedVorbisTimeline {
    /// Cumulative untrimmed decoder output after each audio packet; element zero is the origin.
    packet_end_frames: Vec<u64>,
    audio_pages: Vec<VorbisAudioPageTiming>,
}

fn decode_probe_vorbis(
    serial: u32,
    channels: u8,
    stream: &StreamState,
    data: &[u8],
    limits: &Limits,
) -> Result<DecodedVorbisTimeline, OggError> {
    let malformed = |reason| OggError::AudioStructure { serial, reason };
    let mut packet = Vec::new();
    let mut packet_index = 0usize;
    let mut ident_header = None;
    let mut setup_header = None;
    let mut previous_window = lewton::audio::PreviousWindowRight::new();
    let mut decoded_samples_per_channel = 0usize;
    let mut packet_end_frames = Vec::with_capacity(stream.audio_packets.saturating_add(1));
    packet_end_frames.push(0);
    let mut audio_pages = Vec::with_capacity(stream.page_ranges.len());

    for (page_offset, page_len) in &stream.page_ranges {
        let page = &data[*page_offset..*page_offset + *page_len];
        let segment_count = usize::from(page[26]);
        let header_len = 27 + segment_count;
        let lacing = &page[27..header_len];
        let packet_index_before_page = packet_index;
        let audio_packets_before = packet_end_frames.len() - 1;
        let mut body_offset = header_len;

        for segment_len in lacing.iter().copied().map(usize::from) {
            let end = body_offset + segment_len;
            packet.extend_from_slice(&page[body_offset..end]);
            body_offset = end;
            if segment_len == 255 {
                continue;
            }

            match packet_index {
                0 => {
                    let ident = lewton::header::read_header_ident(&packet)
                        .map_err(|_| malformed("Vorbis identification failed the decode probe"))?;
                    if ident.audio_channels != channels {
                        return Err(malformed(
                            "Vorbis decode probe disagrees with the identification header",
                        ));
                    }
                    ident_header = Some(ident);
                }
                1 => {
                    lewton::header::read_header_comment(&packet)
                        .map_err(|_| malformed("Vorbis comment failed the decode probe"))?;
                }
                2 => {
                    let ident = ident_header
                        .as_ref()
                        .ok_or_else(|| malformed("Vorbis identification header is missing"))?;
                    setup_header = Some(
                        lewton::header::read_header_setup(
                            &packet,
                            ident.audio_channels,
                            (ident.blocksize_0, ident.blocksize_1),
                        )
                        .map_err(|_| malformed("Vorbis setup failed the decode probe"))?,
                    );
                }
                _ => {
                    let ident = ident_header
                        .as_ref()
                        .ok_or_else(|| malformed("Vorbis identification header is missing"))?;
                    let setup = setup_header
                        .as_ref()
                        .ok_or_else(|| malformed("Vorbis setup header is missing"))?;
                    let samples = lewton::audio::read_audio_packet(
                        ident,
                        setup,
                        &packet,
                        &mut previous_window,
                    )
                    .map_err(|_| malformed("Vorbis audio packet failed the decode probe"))?;
                    let packet_samples_per_channel = samples.first().map_or(0, Vec::len);
                    if samples.len() != usize::from(channels)
                        || samples
                            .iter()
                            .any(|channel| channel.len() != packet_samples_per_channel)
                    {
                        return Err(malformed(
                            "Vorbis decoder returned inconsistent channel frames",
                        ));
                    }
                    let actual = decoded_samples_per_channel
                        .checked_add(packet_samples_per_channel)
                        .ok_or(OggError::LimitExceeded {
                            kind: "decoded samples per channel",
                            actual: usize::MAX,
                            limit: limits.max_ogg_decoded_samples_per_channel,
                        })?;
                    if actual > limits.max_ogg_decoded_samples_per_channel {
                        return Err(OggError::LimitExceeded {
                            kind: "decoded samples per channel",
                            actual,
                            limit: limits.max_ogg_decoded_samples_per_channel,
                        });
                    }
                    decoded_samples_per_channel = actual;
                    packet_end_frames.push(u64::try_from(actual).map_err(|_| {
                        OggError::LimitExceeded {
                            kind: "decoded samples per channel",
                            actual,
                            limit: limits.max_ogg_decoded_samples_per_channel,
                        }
                    })?);
                }
            }
            packet_index = packet_index
                .checked_add(1)
                .ok_or_else(|| malformed("Vorbis packet index overflowed"))?;
            packet.clear();
        }

        let audio_packets_after = packet_end_frames.len() - 1;
        let granule = u64::from_le_bytes(page[6..14].try_into().expect("fixed slice"));
        let eos = page[5] & 0x04 != 0;
        if packet_index_before_page < 3 {
            if audio_packets_after != audio_packets_before
                || (packet_index >= 3 && !packet.is_empty())
            {
                return Err(malformed(
                    "Vorbis setup and audio packets must end on separate pages",
                ));
            }
            if granule != 0 {
                return Err(malformed("Vorbis header page granule must be zero"));
            }
        } else {
            if audio_packets_after == audio_packets_before {
                if granule != u64::MAX {
                    return Err(malformed(
                        "Vorbis audio page without a completed packet must use granule -1",
                    ));
                }
            } else if granule == u64::MAX || granule == 0 {
                return Err(malformed(
                    "Vorbis audio page with completed packets needs a positive granule",
                ));
            }
            audio_pages.push(VorbisAudioPageTiming {
                granule,
                audio_packets_before,
                audio_packets_after,
                audio_packet_in_progress: !packet.is_empty(),
                eos,
            });
        }
    }

    if !packet.is_empty()
        || packet_index != stream.completed_packets
        || packet_end_frames.len() != stream.audio_packets.saturating_add(1)
    {
        return Err(malformed(
            "Vorbis decode probe disagrees with parsed packet boundaries",
        ));
    }
    if decoded_samples_per_channel == 0 {
        return Err(malformed("Vorbis decode probe yielded no PCM samples"));
    }
    Ok(DecodedVorbisTimeline {
        packet_end_frames,
        audio_pages,
    })
}

fn validate_vorbis_timing(serial: u32, timeline: &DecodedVorbisTimeline) -> Result<u64, OggError> {
    let malformed = |reason| OggError::AudioStructure { serial, reason };
    let timed_pages = timeline
        .audio_pages
        .iter()
        .filter(|page| page.audio_packets_after > page.audio_packets_before)
        .collect::<Vec<_>>();
    let first = *timed_pages
        .first()
        .ok_or_else(|| malformed("Vorbis audio stream has no timestamped packet page"))?;
    let eos = *timed_pages
        .last()
        .filter(|page| page.eos)
        .ok_or_else(|| malformed("Vorbis EOS page has no completed audio packet"))?;
    let total_packets = timeline.packet_end_frames.len() - 1;
    if eos.audio_packets_after != total_packets {
        return Err(malformed(
            "Vorbis EOS granule does not cover every decoded audio packet",
        ));
    }

    let decoded_at = |packet_count: usize| -> Result<i128, OggError> {
        timeline
            .packet_end_frames
            .get(packet_count)
            .copied()
            .map(i128::from)
            .ok_or_else(|| malformed("Vorbis page packet count exceeds the decoded timeline"))
    };

    if first.eos {
        let decoded_end = decoded_at(first.audio_packets_after)?;
        if i128::from(first.granule) > decoded_end {
            return Err(malformed(
                "Vorbis EOS granule exceeds the decoded PCM frame bound",
            ));
        }
        // With no earlier timestamp, a positive origin is indistinguishable from an inflated EOS
        // granule. A.2 provides an unambiguous non-zero origin only when packet two flushes an
        // earlier page. More than two packets on this page therefore has a zero origin and may
        // trim only the final decoded packet. With one or two packets, a smaller granule can also
        // be the spec-defined negative start trim; either interpretation has the same duration.
        if first.audio_packets_after > 2 {
            let final_packet_start = decoded_at(first.audio_packets_after - 1)?;
            if i128::from(first.granule) < final_packet_start {
                return Err(malformed(
                    "Vorbis EOS granule trims beyond the final decoded packet",
                ));
            }
        }
        return Ok(first.granule);
    }

    let first_decoded_end = decoded_at(first.audio_packets_after)?;
    let origin = i128::from(first.granule) - first_decoded_end;
    if origin != 0
        && (first.audio_packets_before != 0
            || first.audio_packets_after != 2
            || first.audio_packet_in_progress
            || total_packets <= 2)
    {
        return Err(malformed(
            "non-zero Vorbis PCM origin requires packet two to flush an earlier audio page",
        ));
    }

    let mut previous_granule = None;
    for page in timed_pages {
        if let Some(previous) = previous_granule {
            if page.granule < previous {
                return Err(malformed("Vorbis audio granules are not monotonic"));
            }
        }
        let nominal_end = origin + decoded_at(page.audio_packets_after)?;
        if page.eos {
            if i128::from(page.granule) > nominal_end {
                return Err(malformed(
                    "Vorbis EOS granule exceeds the decoded PCM frame bound",
                ));
            }
            let final_packet_start = origin + decoded_at(total_packets - 1)?;
            if i128::from(page.granule) < final_packet_start {
                return Err(malformed(
                    "Vorbis EOS granule trims beyond the final decoded packet",
                ));
            }
        } else if i128::from(page.granule) != nominal_end {
            return Err(malformed(
                "Vorbis intermediate granule disagrees with decoded packet timing",
            ));
        }
        previous_granule = Some(page.granule);
    }

    let duration = i128::from(eos.granule) - origin.max(0);
    if duration <= 0 || duration > i128::from(u64::MAX) {
        return Err(malformed(
            "Vorbis duration is not positive after applying its PCM origin",
        ));
    }
    Ok(duration as u64)
}

#[allow(clippy::too_many_arguments)]
fn validate_opus_page_granule(
    serial: u32,
    flags: u8,
    granule: u64,
    audio_packets_before: usize,
    audio_packets_after: usize,
    samples_after: u64,
    audio_packet_in_progress: bool,
    granule_origin: &mut Option<u64>,
    last_audio_granule: &mut Option<u64>,
) -> Result<(), OggError> {
    let malformed = |reason| OggError::AudioStructure { serial, reason };
    if audio_packets_after == audio_packets_before {
        let expected = if audio_packet_in_progress || audio_packets_before > 0 {
            u64::MAX
        } else {
            0
        };
        if granule != expected {
            return Err(malformed(
                "Opus page without a completed audio packet has an invalid granule",
            ));
        }
        return Ok(());
    }

    if granule == u64::MAX {
        return Err(malformed("Opus audio page has no duration granule"));
    }
    let is_first_audio_page = last_audio_granule.is_none();
    if let Some(previous) = *last_audio_granule {
        if granule < previous {
            return Err(malformed("Opus audio granules are not monotonic"));
        }
    }

    if is_first_audio_page {
        if flags & 0x04 == 0 && granule < samples_after {
            return Err(malformed(
                "first non-EOS Opus audio granule is smaller than its packet duration",
            ));
        }
        *granule_origin = Some(granule.saturating_sub(samples_after));
    } else {
        let origin =
            (*granule_origin).ok_or_else(|| malformed("Opus granule origin is missing"))?;
        let expected = origin
            .checked_add(samples_after)
            .ok_or_else(|| malformed("Opus granule position overflowed"))?;
        if flags & 0x04 == 0 && granule != expected {
            return Err(malformed(
                "non-EOS Opus page granule does not match its relative packet duration",
            ));
        }
        let previous = (*last_audio_granule).expect("non-first audio page has a prior granule");
        if flags & 0x04 != 0 && (granule < previous || granule > expected) {
            return Err(malformed(
                "Opus EOS granule is outside its relative end-trim interval",
            ));
        }
    }
    *last_audio_granule = Some(granule);
    Ok(())
}

fn validate_opus_tags(packet: &[u8]) -> Result<(), &'static str> {
    if !packet.starts_with(b"OpusTags") {
        return Err("missing OpusTags comment header");
    }
    let mut offset = 8usize;
    let vendor_len = take_opus_u32(packet, &mut offset)?;
    take_opus_bytes(packet, &mut offset, vendor_len)?;
    let comment_count = take_opus_u32(packet, &mut offset)?;
    if comment_count > packet.len().saturating_sub(offset) / 4 {
        return Err("OpusTags comment count exceeds the packet");
    }
    for _ in 0..comment_count {
        let comment_len = take_opus_u32(packet, &mut offset)?;
        take_opus_bytes(packet, &mut offset, comment_len)?;
    }
    // RFC 7845 permits binary padding after the declared comments.
    Ok(())
}

fn take_opus_u32(packet: &[u8], offset: &mut usize) -> Result<usize, &'static str> {
    let end = offset.checked_add(4).ok_or("OpusTags length overflowed")?;
    let bytes = packet
        .get(*offset..end)
        .ok_or("truncated OpusTags packet")?;
    *offset = end;
    Ok(u32::from_le_bytes(bytes.try_into().expect("four bytes")) as usize)
}

fn take_opus_bytes<'a>(
    packet: &'a [u8],
    offset: &mut usize,
    len: usize,
) -> Result<&'a [u8], &'static str> {
    let end = offset
        .checked_add(len)
        .ok_or("OpusTags field length overflowed")?;
    let bytes = packet.get(*offset..end).ok_or("truncated OpusTags field")?;
    *offset = end;
    Ok(bytes)
}

fn opus_packet_samples(packet: &[u8]) -> Result<u64, &'static str> {
    let toc = *packet.first().ok_or("empty Opus audio packet")?;
    let frame_samples = match toc >> 3 {
        0..=11 => [480u64, 960, 1_920, 2_880][usize::from((toc >> 3) & 3)],
        12..=15 => [480u64, 960][usize::from((toc >> 3) & 1)],
        _ => [120u64, 240, 480, 960][usize::from((toc >> 3) & 3)],
    };

    let frame_count = match toc & 3 {
        0 => {
            validate_opus_frame_len(packet.len().saturating_sub(1))?;
            1usize
        }
        1 => {
            let payload = packet.len().saturating_sub(1);
            if !payload.is_multiple_of(2) {
                return Err("malformed two-frame CBR Opus packet");
            }
            validate_opus_frame_len(payload / 2)?;
            2
        }
        2 => {
            let mut offset = 1usize;
            let first_len = take_opus_frame_len(packet, &mut offset)?;
            validate_opus_frame_len(first_len)?;
            let remaining = packet
                .len()
                .checked_sub(offset + first_len)
                .ok_or("truncated two-frame VBR Opus packet")?;
            validate_opus_frame_len(remaining)?;
            2
        }
        3 => validate_opus_code_three(packet)?,
        _ => unreachable!(),
    };
    let total = frame_samples
        .checked_mul(frame_count as u64)
        .ok_or("Opus packet duration overflowed")?;
    if total > 5_760 {
        return Err("Opus packet duration exceeds 120 ms");
    }
    Ok(total)
}

fn validate_opus_code_three(packet: &[u8]) -> Result<usize, &'static str> {
    let control = *packet.get(1).ok_or("truncated multi-frame Opus packet")?;
    let vbr = control & 0x80 != 0;
    let has_padding = control & 0x40 != 0;
    let frame_count = usize::from(control & 0x3f);
    if frame_count == 0 || frame_count > 48 {
        return Err("invalid Opus frame count");
    }

    let mut offset = 2usize;
    let mut padding = 0usize;
    if has_padding {
        loop {
            let value = usize::from(*packet.get(offset).ok_or("truncated Opus padding length")?);
            offset += 1;
            padding = padding
                .checked_add(if value == 255 { 254 } else { value })
                .ok_or("Opus padding length overflowed")?;
            if value != 255 {
                break;
            }
        }
    }
    let payload_end = packet
        .len()
        .checked_sub(padding)
        .filter(|end| *end >= offset)
        .ok_or("Opus padding exceeds the packet")?;

    if vbr {
        let mut declared = 0usize;
        for _ in 0..frame_count - 1 {
            let len = take_opus_frame_len(&packet[..payload_end], &mut offset)?;
            validate_opus_frame_len(len)?;
            declared = declared
                .checked_add(len)
                .ok_or("Opus frame lengths overflowed")?;
        }
        let remaining = payload_end
            .checked_sub(offset + declared)
            .ok_or("Opus VBR frame lengths exceed the packet")?;
        validate_opus_frame_len(remaining)?;
    } else {
        let payload = payload_end - offset;
        if !payload.is_multiple_of(frame_count) {
            return Err("Opus CBR frame payload is not evenly divisible");
        }
        validate_opus_frame_len(payload / frame_count)?;
    }
    Ok(frame_count)
}

fn take_opus_frame_len(packet: &[u8], offset: &mut usize) -> Result<usize, &'static str> {
    let first = usize::from(*packet.get(*offset).ok_or("truncated Opus frame length")?);
    *offset += 1;
    if first < 252 {
        return Ok(first);
    }
    let second = usize::from(
        *packet
            .get(*offset)
            .ok_or("truncated extended Opus frame length")?,
    );
    *offset += 1;
    Ok(first + 4 * second)
}

fn validate_opus_frame_len(len: usize) -> Result<(), &'static str> {
    // A zero-byte frame inside a non-empty Opus packet requests DTX/PLC and is valid. An empty Ogg
    // audio packet is rejected before TOC parsing in `opus_packet_samples`.
    if len > 1_275 {
        return Err("invalid Opus compressed frame length");
    }
    Ok(())
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
        make_page_with_lacing_and_granule(serial, sequence, flags, 0, lacing, body)
    }

    fn make_page_with_lacing_and_granule(
        serial: u32,
        sequence: u32,
        flags: u8,
        granule: u64,
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
        page.extend_from_slice(&granule.to_le_bytes());
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
        let mut data = include_bytes!("../testdata/tiny-vorbis.ogg").to_vec();
        let ident = find_bytes(&data, b"\x01vorbis").expect("fixture has Vorbis identification");
        data[ident + 12..ident + 16].copy_from_slice(&sample_rate.to_le_bytes());
        rewrite_page_checksums(&mut data);
        data
    }

    pub(crate) fn opus_ogg(input_sample_rate: u32) -> Vec<u8> {
        let mut data = include_bytes!("../testdata/tiny-opus.ogg").to_vec();
        let head = find_bytes(&data, b"OpusHead").expect("fixture has OpusHead");
        data[head + 12..head + 16].copy_from_slice(&input_sample_rate.to_le_bytes());
        rewrite_page_checksums(&mut data);
        data
    }

    fn decoded_vorbis_packet_ends(data: &[u8]) -> Vec<u64> {
        let mut reader = lewton::inside_ogg::OggStreamReader::new(std::io::Cursor::new(data))
            .expect("fixture headers decode");
        let channels = usize::from(reader.ident_hdr.audio_channels);
        let mut total = 0u64;
        let mut ends = vec![0];
        while let Some(samples) = reader
            .read_dec_packet_itl()
            .expect("fixture packet decodes")
        {
            assert!(samples.len().is_multiple_of(channels));
            total += u64::try_from(samples.len() / channels).unwrap();
            ends.push(total);
        }
        ends
    }

    fn split_final_vorbis_audio_page(
        data: &[u8],
        packet_ends: &[usize],
        granules: &[u64],
    ) -> Vec<u8> {
        assert_eq!(packet_ends.len(), granules.len());
        let final_page = *page_offsets(data).last().expect("fixture has pages");
        let serial = u32::from_le_bytes(data[final_page + 14..final_page + 18].try_into().unwrap());
        let sequence =
            u32::from_le_bytes(data[final_page + 18..final_page + 22].try_into().unwrap());
        let segment_count = usize::from(data[final_page + 26]);
        let lacing = &data[final_page + 27..final_page + 27 + segment_count];
        assert!(lacing.iter().all(|value| *value < 255));
        assert_eq!(packet_ends.last().copied(), Some(lacing.len()));
        let body = &data[final_page + 27 + segment_count..];

        let mut split = data[..final_page].to_vec();
        let mut packet_start = 0usize;
        let mut body_start = 0usize;
        for (page_index, (&packet_end, &granule)) in packet_ends.iter().zip(granules).enumerate() {
            let body_end = body_start
                + lacing[packet_start..packet_end]
                    .iter()
                    .map(|value| usize::from(*value))
                    .sum::<usize>();
            let is_last = page_index + 1 == packet_ends.len();
            split.extend_from_slice(&make_page_with_lacing_and_granule(
                serial,
                sequence + u32::try_from(page_index).unwrap(),
                if is_last { 0x04 } else { 0 },
                granule,
                &lacing[packet_start..packet_end],
                &body[body_start..body_end],
            ));
            packet_start = packet_end;
            body_start = body_end;
        }
        assert_eq!(body_start, body.len());
        split
    }

    #[test]
    fn validates_vorbis_identification() {
        let validation = validate_ogg_with_timing(&vorbis_ogg(44_100), &Limits::default()).unwrap();
        let info = &validation.info;
        assert_eq!(
            info.codec,
            OggCodec::Vorbis {
                channels: 1,
                sample_rate: 44_100
            }
        );
        assert!(info.pages >= 2);
        assert_eq!(validation.timing.duration_sample_frames, 3_840);
        assert_eq!(validation.timing.duration_timebase_hz, 44_100);
        assert!(validation.timing.pcm_decode_complete);
    }

    #[test]
    fn legacy_validation_shape_stays_metadata_only() {
        let legacy = validate_ogg(&vorbis_ogg(48_000), &Limits::default()).unwrap();
        let OggInfo {
            codec,
            pages,
            logical_streams,
        } = legacy.clone();
        assert_eq!(
            codec,
            OggCodec::Vorbis {
                channels: 1,
                sample_rate: 48_000
            }
        );
        assert_eq!(pages, 3);
        assert_eq!(logical_streams, 1);
        assert_eq!(
            validate_ogg_with_timing(&vorbis_ogg(48_000), &Limits::default())
                .unwrap()
                .info,
            legacy
        );
    }

    #[test]
    fn vorbis_voice_profile_accepts_stereo_and_rejects_more_channels() {
        let data = vorbis_ogg(48_000);
        let ident_offset = find_bytes(&data, b"\x01vorbis").expect("fixture has identification");
        let mut ident = data[ident_offset..ident_offset + 30].to_vec();
        ident[11] = 2;
        assert_eq!(
            identify_vorbis(&ident).unwrap(),
            OggCodec::Vorbis {
                channels: 2,
                sample_rate: 48_000,
            }
        );
        ident[11] = 3;
        assert!(matches!(
            identify_vorbis(&ident),
            Err(OggError::Identification(
                "Vorbis voice assets support at most two channels"
            ))
        ));
    }

    #[test]
    fn reports_exact_vorbis_eos_trim_and_rejects_unbacked_duration() {
        let mut trimmed = vorbis_ogg(48_000);
        let final_page = *page_offsets(&trimmed).last().expect("fixture has pages");
        trimmed[final_page + 6..final_page + 14].copy_from_slice(&3_700u64.to_le_bytes());
        rewrite_page_checksums(&mut trimmed);

        let trimmed_info = validate_ogg_with_timing(&trimmed, &Limits::default()).unwrap();
        assert_eq!(trimmed_info.timing.duration_sample_frames, 3_700);
        assert_eq!(trimmed_info.timing.duration_timebase_hz, 48_000);
        assert!(trimmed_info.timing.pcm_decode_complete);

        let mut over_trimmed = vorbis_ogg(48_000);
        let final_page = *page_offsets(&over_trimmed)
            .last()
            .expect("fixture has pages");
        over_trimmed[final_page + 6..final_page + 14].copy_from_slice(&3_000u64.to_le_bytes());
        rewrite_page_checksums(&mut over_trimmed);
        assert!(matches!(
            validate_ogg_with_timing(&over_trimmed, &Limits::default()),
            Err(OggError::AudioStructure {
                reason: "Vorbis EOS granule trims beyond the final decoded packet",
                ..
            })
        ));

        let mut inflated = vorbis_ogg(48_000);
        let final_page = *page_offsets(&inflated).last().expect("fixture has pages");
        inflated[final_page + 6..final_page + 14].copy_from_slice(&10_000u64.to_le_bytes());
        rewrite_page_checksums(&mut inflated);
        assert!(matches!(
            validate_ogg(&inflated, &Limits::default()),
            Err(OggError::AudioStructure {
                reason: "Vorbis EOS granule exceeds the decoded PCM frame bound",
                ..
            })
        ));
    }

    #[test]
    fn reports_vorbis_duration_relative_to_positive_and_negative_pcm_origins() {
        let data = vorbis_ogg(48_000);
        let final_page = *page_offsets(&data).last().expect("fixture has pages");
        let original_eos =
            u64::from_le_bytes(data[final_page + 6..final_page + 14].try_into().unwrap());
        let decoded_ends = decoded_vorbis_packet_ends(&data);
        assert_eq!(decoded_ends.len(), 7);
        assert!(decoded_ends[2] > 128);

        let positive_origin = 60_000u64;
        let positive = split_final_vorbis_audio_page(
            &data,
            &[2, 4, 6],
            &[
                positive_origin + decoded_ends[2],
                positive_origin + decoded_ends[4],
                positive_origin + original_eos,
            ],
        );
        let positive_info = validate_ogg_with_timing(&positive, &Limits::default()).unwrap();
        assert_eq!(positive_info.timing.duration_sample_frames, original_eos);

        let leading_trim = 128u64;
        let negative = split_final_vorbis_audio_page(
            &data,
            &[2, 4, 6],
            &[
                decoded_ends[2] - leading_trim,
                decoded_ends[4] - leading_trim,
                original_eos - leading_trim,
            ],
        );
        let negative_info = validate_ogg_with_timing(&negative, &Limits::default()).unwrap();
        assert_eq!(
            negative_info.timing.duration_sample_frames,
            original_eos - leading_trim
        );
    }

    #[test]
    fn rejects_malformed_vorbis_origin_layout_and_intermediate_granule() {
        let data = vorbis_ogg(48_000);
        let final_page = *page_offsets(&data).last().expect("fixture has pages");
        let original_eos =
            u64::from_le_bytes(data[final_page + 6..final_page + 14].try_into().unwrap());
        let decoded_ends = decoded_vorbis_packet_ends(&data);
        let origin = 60_000u64;

        let wrong_flush = split_final_vorbis_audio_page(
            &data,
            &[3, 6],
            &[origin + decoded_ends[3], origin + original_eos],
        );
        assert!(matches!(
            validate_ogg_with_timing(&wrong_flush, &Limits::default()),
            Err(OggError::AudioStructure {
                reason:
                    "non-zero Vorbis PCM origin requires packet two to flush an earlier audio page",
                ..
            })
        ));

        let mut bad_intermediate = split_final_vorbis_audio_page(
            &data,
            &[2, 4, 6],
            &[
                origin + decoded_ends[2],
                origin + decoded_ends[4],
                origin + original_eos,
            ],
        );
        let middle_page = page_offsets(&bad_intermediate)[3];
        let middle_granule = u64::from_le_bytes(
            bad_intermediate[middle_page + 6..middle_page + 14]
                .try_into()
                .unwrap(),
        );
        bad_intermediate[middle_page + 6..middle_page + 14]
            .copy_from_slice(&(middle_granule + 1).to_le_bytes());
        rewrite_page_checksums(&mut bad_intermediate);
        assert!(matches!(
            validate_ogg_with_timing(&bad_intermediate, &Limits::default()),
            Err(OggError::AudioStructure {
                reason: "Vorbis intermediate granule disagrees with decoded packet timing",
                ..
            })
        ));
    }

    #[test]
    fn validates_opus_identification() {
        let data = opus_ogg(44_100);
        let validation = validate_ogg_with_timing(&data, &Limits::default()).unwrap();
        let info = &validation.info;
        assert_eq!(
            info.codec,
            OggCodec::Opus {
                channels: 1,
                input_sample_rate: 44_100
            }
        );
        assert_eq!(validation.timing.duration_sample_frames, 3_840);
        assert_eq!(validation.timing.duration_timebase_hz, 48_000);
        assert!(!validation.timing.pcm_decode_complete);
    }

    #[test]
    fn rejects_mapped_opus_until_self_delimited_packets_are_parsed() {
        let mut mapped = b"OpusHead".to_vec();
        mapped.push(1);
        mapped.push(1);
        mapped.extend_from_slice(&312u16.to_le_bytes());
        mapped.extend_from_slice(&48_000u32.to_le_bytes());
        mapped.extend_from_slice(&0i16.to_le_bytes());
        mapped.push(1);
        mapped.extend_from_slice(&[1, 0, 0]);
        assert!(matches!(
            identify_opus(&mapped),
            Err(OggError::Identification(
                "mapped/multistream Opus requires unsupported self-delimited packet parsing"
            ))
        ));
    }

    #[test]
    fn rejects_identification_only_vorbis_and_opus() {
        let mut vorbis = Vec::with_capacity(30);
        vorbis.extend_from_slice(b"\x01vorbis");
        vorbis.extend_from_slice(&0u32.to_le_bytes());
        vorbis.push(1);
        vorbis.extend_from_slice(&48_000u32.to_le_bytes());
        vorbis.extend_from_slice(&0i32.to_le_bytes());
        vorbis.extend_from_slice(&0i32.to_le_bytes());
        vorbis.extend_from_slice(&0i32.to_le_bytes());
        vorbis.push(0x86);
        vorbis.push(1);
        assert!(matches!(
            validate_ogg(&make_page(7, 0, 0x02 | 0x04, &vorbis), &Limits::default()),
            Err(OggError::AudioStructure { .. })
        ));

        let mut opus = b"OpusHead".to_vec();
        opus.extend_from_slice(&[1, 1]);
        opus.extend_from_slice(&312u16.to_le_bytes());
        opus.extend_from_slice(&48_000u32.to_le_bytes());
        opus.extend_from_slice(&0i16.to_le_bytes());
        opus.push(0);
        assert!(matches!(
            validate_ogg(&make_page(9, 0, 0x02 | 0x04, &opus), &Limits::default()),
            Err(OggError::AudioStructure { .. })
        ));
    }

    #[test]
    fn rejects_opus_without_tags_or_positive_post_skip_duration() {
        let mut missing_tags = opus_ogg(48_000);
        let tags = find_bytes(&missing_tags, b"OpusTags").expect("fixture has OpusTags");
        missing_tags[tags] ^= 1;
        rewrite_page_checksums(&mut missing_tags);
        assert!(matches!(
            validate_ogg(&missing_tags, &Limits::default()),
            Err(OggError::AudioStructure {
                reason: "missing OpusTags comment header",
                ..
            })
        ));

        let mut no_post_skip_duration = opus_ogg(48_000);
        let final_page = page_offsets(&no_post_skip_duration)
            .last()
            .copied()
            .expect("fixture has pages");
        no_post_skip_duration[final_page + 6..final_page + 14]
            .copy_from_slice(&312u64.to_le_bytes());
        rewrite_page_checksums(&mut no_post_skip_duration);
        assert!(matches!(
            validate_ogg(&no_post_skip_duration, &Limits::default()),
            Err(OggError::AudioStructure {
                reason: "Opus EOS granule leaves no positive duration after pre-skip",
                ..
            })
        ));
    }

    #[test]
    fn accepts_opus_initial_granule_offset_and_relative_following_pages() {
        let data = opus_ogg(48_000);
        let original_info = validate_ogg_with_timing(&data, &Limits::default()).unwrap();
        let final_page = *page_offsets(&data).last().expect("fixture has pages");
        let serial = u32::from_le_bytes(data[final_page + 14..final_page + 18].try_into().unwrap());
        let sequence =
            u32::from_le_bytes(data[final_page + 18..final_page + 22].try_into().unwrap());
        let segment_count = usize::from(data[final_page + 26]);
        let lacing = &data[final_page + 27..final_page + 27 + segment_count];
        assert!(lacing.len() >= 2 && lacing.iter().all(|value| *value < 255));
        let body = &data[final_page + 27 + segment_count..];
        let first_body_len = usize::from(lacing[0]);
        let first_samples = opus_packet_samples(&body[..first_body_len]).unwrap();
        let total_samples = lacing
            .iter()
            .try_fold((0usize, 0u64), |(offset, samples), len| {
                let end = offset + usize::from(*len);
                opus_packet_samples(&body[offset..end]).map(|duration| (end, samples + duration))
            })
            .unwrap()
            .1;
        let original_eos =
            u64::from_le_bytes(data[final_page + 6..final_page + 14].try_into().unwrap());
        assert!(original_eos <= total_samples);

        let origin = 60_000u64;
        let mut shifted = data[..final_page].to_vec();
        shifted.extend_from_slice(&make_page_with_lacing_and_granule(
            serial,
            sequence,
            0,
            origin + first_samples,
            &lacing[..1],
            &body[..first_body_len],
        ));
        shifted.extend_from_slice(&make_page_with_lacing_and_granule(
            serial,
            sequence + 1,
            0x04,
            origin + original_eos,
            &lacing[1..],
            &body[first_body_len..],
        ));

        let shifted_info = validate_ogg_with_timing(&shifted, &Limits::default()).unwrap();
        assert_eq!(
            shifted_info.timing.duration_sample_frames,
            original_info.timing.duration_sample_frames
        );
        assert_eq!(shifted_info.timing.duration_timebase_hz, 48_000);
        assert!(!shifted_info.timing.pcm_decode_complete);
    }

    #[test]
    fn reports_exact_opus_pre_skip_and_eos_trim_duration() {
        let mut changed_pre_skip = opus_ogg(48_000);
        let head = find_bytes(&changed_pre_skip, b"OpusHead").expect("fixture has OpusHead");
        changed_pre_skip[head + 10..head + 12].copy_from_slice(&1_000u16.to_le_bytes());
        rewrite_page_checksums(&mut changed_pre_skip);
        let changed_pre_skip_info =
            validate_ogg_with_timing(&changed_pre_skip, &Limits::default()).unwrap();
        assert_eq!(changed_pre_skip_info.timing.duration_sample_frames, 3_152);
        assert_eq!(changed_pre_skip_info.timing.duration_timebase_hz, 48_000);

        let mut eos_trimmed = opus_ogg(48_000);
        let final_page = *page_offsets(&eos_trimmed)
            .last()
            .expect("fixture has pages");
        let pre_skip = 312u64;
        let retained_frames = 777u64;
        eos_trimmed[final_page + 6..final_page + 14]
            .copy_from_slice(&(pre_skip + retained_frames).to_le_bytes());
        rewrite_page_checksums(&mut eos_trimmed);
        let eos_trimmed_info = validate_ogg_with_timing(&eos_trimmed, &Limits::default()).unwrap();
        assert_eq!(
            eos_trimmed_info.timing.duration_sample_frames,
            retained_frames
        );
        assert_eq!(eos_trimmed_info.timing.duration_timebase_hz, 48_000);
        assert!(!eos_trimmed_info.timing.pcm_decode_complete);
    }

    #[test]
    fn accepts_zero_length_opus_frames_but_not_an_empty_ogg_packet() {
        let data = opus_ogg(48_000);
        let final_page = *page_offsets(&data).last().expect("fixture has pages");
        let serial = u32::from_le_bytes(data[final_page + 14..final_page + 18].try_into().unwrap());
        let sequence =
            u32::from_le_bytes(data[final_page + 18..final_page + 22].try_into().unwrap());

        // Code 1 describes two equal-size frames. A TOC-only packet therefore contains two
        // zero-byte DTX/PLC frames while the enclosing Ogg packet itself remains non-empty.
        let toc_only = [0b1001_1001u8];
        let duration = opus_packet_samples(&toc_only).unwrap();
        let mut dtx = data[..final_page].to_vec();
        dtx.extend_from_slice(&make_page_with_lacing_and_granule(
            serial,
            sequence,
            0x04,
            duration,
            &[1],
            &toc_only,
        ));
        validate_ogg(&dtx, &Limits::default()).unwrap();

        let mut empty = data[..final_page].to_vec();
        empty.extend_from_slice(&make_page_with_lacing_and_granule(
            serial,
            sequence,
            0x04,
            duration,
            &[0],
            &[],
        ));
        assert!(matches!(
            validate_ogg(&empty, &Limits::default()),
            Err(OggError::AudioStructure {
                reason: "empty Opus audio packet",
                ..
            })
        ));
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
    fn limits_vorbis_and_opus_identification_headers() {
        for (data, limit) in [(vorbis_ogg(44_100), 29), (opus_ogg(48_000), 18)] {
            let limits = Limits {
                max_ogg_codec_header_bytes: limit,
                ..Limits::default()
            };
            assert!(matches!(
                validate_ogg(&data, &limits),
                Err(OggError::LimitExceeded {
                    kind: "codec header bytes",
                    actual,
                    limit: actual_limit,
                }) if actual > actual_limit && actual_limit == limit
            ));
        }
    }

    #[test]
    fn limits_comment_and_setup_headers() {
        for (data, limit) in [(vorbis_ogg(44_100), 30), (opus_ogg(48_000), 19)] {
            let limits = Limits {
                max_ogg_codec_header_bytes: limit,
                ..Limits::default()
            };
            assert!(matches!(
                validate_ogg(&data, &limits),
                Err(OggError::LimitExceeded {
                    kind: "codec header bytes",
                    actual,
                    limit: actual_limit,
                }) if actual > actual_limit && actual_limit == limit
            ));
        }

        // The fixture's 48-byte comment passes, then its larger setup header is rejected.
        let limits = Limits {
            max_ogg_codec_header_bytes: 48,
            ..Limits::default()
        };
        assert!(matches!(
            validate_ogg(&vorbis_ogg(44_100), &limits),
            Err(OggError::LimitExceeded {
                kind: "codec header bytes",
                actual,
                limit: 48,
            }) if actual > 48
        ));
    }

    #[test]
    fn limits_vorbis_and_opus_audio_packet_counts() {
        for data in [vorbis_ogg(44_100), opus_ogg(48_000)] {
            let limits = Limits {
                max_ogg_audio_packets: 1,
                ..Limits::default()
            };
            assert!(matches!(
                validate_ogg(&data, &limits),
                Err(OggError::LimitExceeded {
                    kind: "audio packet count",
                    actual: 2,
                    limit: 1,
                })
            ));
        }
    }

    #[test]
    fn limits_decoded_vorbis_samples_per_channel() {
        let limits = Limits {
            max_ogg_decoded_samples_per_channel: 1,
            ..Limits::default()
        };
        assert!(matches!(
            validate_ogg(&vorbis_ogg(44_100), &limits),
            Err(OggError::LimitExceeded {
                kind: "decoded samples per channel",
                actual,
                limit: 1,
            }) if actual > 1
        ));
    }

    #[test]
    fn preflight_rejects_compact_ordered_and_unordered_codebook_entry_bombs() {
        let default_limits = Limits::default();
        let declared = default_limits.max_vorbis_codebook_entries + 1;
        for ordered in [true, false] {
            let packet = finish_vorbis_setup(vorbis_setup_codebook_bits(1, declared, ordered));
            assert!(
                packet.len() < 32,
                "the adversarial declaration must stay compact"
            );
            assert!(matches!(
                preflight_vorbis_setup(&packet, 7, &default_limits),
                Err(OggError::LimitExceeded {
                    kind: "Vorbis codebook entries",
                    actual,
                    limit,
                }) if actual == declared && limit == default_limits.max_vorbis_codebook_entries
            ));
        }
    }

    #[test]
    fn preflight_bounds_huffman_trie_work_and_materialized_vq_scalars() {
        let mut tree_bits = vorbis_setup_codebook_bits(1, 2, true);
        tree_bits.push(0, 5); // Initial codeword length = 1.
        tree_bits.push(2, vorbis_ilog(2)); // Two active entries of length 1.
        tree_bits.push(0, 4); // Lookup type 0.
        let tree = finish_vorbis_setup(tree_bits);
        let tree_limits = Limits {
            max_vorbis_huffman_tree_nodes: 2,
            ..Limits::default()
        };
        assert!(matches!(
            preflight_vorbis_setup(&tree, 7, &tree_limits),
            Err(OggError::LimitExceeded {
                kind: "Vorbis Huffman tree nodes",
                actual: 3,
                limit: 2,
            })
        ));

        let mut vq_bits = vorbis_setup_codebook_bits(16, 1, true);
        vq_bits.push(0, 5); // Initial codeword length = 1.
        vq_bits.push(1, vorbis_ilog(1));
        vq_bits.push(1, 4); // Lookup type 1 materializes entries*dimensions values.
        let vq = finish_vorbis_setup(vq_bits);
        let vq_limits = Limits {
            max_vorbis_vq_scalars: 15,
            ..Limits::default()
        };
        assert!(matches!(
            preflight_vorbis_setup(&vq, 7, &vq_limits),
            Err(OggError::LimitExceeded {
                kind: "Vorbis VQ scalars",
                actual: 16,
                limit: 15,
            })
        ));
    }

    #[test]
    fn preflight_fails_closed_when_a_bounded_codebook_is_truncated() {
        let mut bits = vorbis_setup_codebook_bits(1, 1, true);
        bits.push(0, 5);
        bits.push(1, vorbis_ilog(1));
        bits.push(1, 4); // Lookup type 1, but its fixed lookup fields are absent.
        let packet = finish_vorbis_setup(bits);
        assert!(matches!(
            preflight_vorbis_setup(&packet, 91, &Limits::default()),
            Err(OggError::AudioStructure {
                serial: 91,
                reason: "truncated Vorbis setup codebook lookup header",
            })
        ));
    }

    #[test]
    fn validates_packet_continuation_across_pages() {
        let mut data = vorbis_ogg(44_100);
        data.extend_from_slice(&make_page_with_lacing(77, 0, 0x02, &[255], &[1; 255]));
        data.extend_from_slice(&make_page_with_lacing(77, 1, 0x01 | 0x04, &[5], &[2; 5]));

        let info = validate_ogg(&data, &Limits::default()).unwrap();
        assert!(info.pages >= 4);
        assert_eq!(info.logical_streams, 2);
    }

    fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
        haystack
            .windows(needle.len())
            .position(|window| window == needle)
    }

    fn vorbis_setup_codebook_bits(
        dimensions: usize,
        entries: usize,
        ordered: bool,
    ) -> LsbBitWriter {
        assert!(dimensions <= u16::MAX as usize);
        assert!(entries <= 0x00ff_ffff);
        let mut bits = LsbBitWriter::default();
        bits.push(0, 8); // One codebook.
        bits.push(0x56_43_42, 24);
        bits.push(dimensions as u32, 16);
        bits.push(entries as u32, 24);
        bits.push(u32::from(ordered), 1);
        bits
    }

    fn finish_vorbis_setup(bits: LsbBitWriter) -> Vec<u8> {
        let mut packet = b"\x05vorbis".to_vec();
        packet.extend_from_slice(&bits.finish());
        packet
    }

    #[derive(Default)]
    struct LsbBitWriter {
        bytes: Vec<u8>,
        bit_offset: usize,
    }

    impl LsbBitWriter {
        fn push(&mut self, value: u32, count: u8) {
            assert!(count <= 32);
            for shift in 0..usize::from(count) {
                let byte = self.bit_offset / 8;
                let bit = self.bit_offset % 8;
                if byte == self.bytes.len() {
                    self.bytes.push(0);
                }
                self.bytes[byte] |= (((value >> shift) & 1) as u8) << bit;
                self.bit_offset += 1;
            }
        }

        fn finish(self) -> Vec<u8> {
            self.bytes
        }
    }

    fn rewrite_page_checksums(data: &mut [u8]) {
        for offset in page_offsets(data) {
            assert_eq!(&data[offset..offset + 4], b"OggS");
            let segment_count = usize::from(data[offset + 26]);
            let header_len = 27 + segment_count;
            let body_len = data[offset + 27..offset + header_len]
                .iter()
                .map(|value| usize::from(*value))
                .sum::<usize>();
            let page_len = header_len + body_len;
            data[offset + 22..offset + 26].fill(0);
            let checksum = ogg_crc(&data[offset..offset + page_len]);
            data[offset + 22..offset + 26].copy_from_slice(&checksum.to_le_bytes());
        }
    }

    fn page_offsets(data: &[u8]) -> Vec<usize> {
        let mut offsets = Vec::new();
        let mut offset = 0usize;
        while offset < data.len() {
            offsets.push(offset);
            let segment_count = usize::from(data[offset + 26]);
            let header_len = 27 + segment_count;
            let body_len = data[offset + 27..offset + header_len]
                .iter()
                .map(|value| usize::from(*value))
                .sum::<usize>();
            offset += header_len + body_len;
        }
        offsets
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
