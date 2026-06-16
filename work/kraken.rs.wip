use crate::{CompressedStream, CoreError};
use serde_json::{Value, json};

pub(crate) const OODLE_COMPRESSOR_KRAKEN: u8 = 8;
pub(crate) const KRAKEN_BLOCK_LEN: usize = 0x20_000;
pub(crate) const KRAKEN_OPTIMAL_SCRATCH_BYTES: usize = 0x1804;
pub(crate) const NEWLZ_SUBJOB_WINDOW_BYTES: usize = 0x40_000;
pub(crate) const NEWLZ_SUBJOB_THRESHOLD_BYTES: usize = 0x44_001;
pub(crate) const RAW_LITERAL_PACKET_MAX_BYTES: usize = 0x3_FFFF;
pub(crate) const SHORT_LITERAL_RAW_MAX_BYTES: usize = 0x20;
pub(crate) const RAW_LITERAL_COST_BASE: f32 = 3.0;
pub(crate) const WRAPPED_SHORT_RAW_COST_DISCOUNT: f32 = 1.0;
pub(crate) const WRAPPED_ENTROPY_COST_DISCOUNT: f32 = 2.0;
pub(crate) const SINGLE_SYMBOL_DIRECT_COST_BASE: f32 = 6.0;
pub(crate) const SINGLE_SYMBOL_SPLIT_COST_BASE: f32 = 8.0;
pub(crate) const ENTROPY_LITERAL_HEADER_UNIT: u64 = 0x4_0000;
pub(crate) const SINGLE_SYMBOL_DIRECT_MODE_ID: u8 = 3;
pub(crate) const SINGLE_SYMBOL_SPLIT_MODE_ID: u8 = 2;
pub(crate) const SINGLE_SYMBOL_DIRECT_PAYLOAD_BYTES: usize = 1;
pub(crate) const SINGLE_SYMBOL_SPLIT_PAYLOAD_BYTES: usize = 3;
pub(crate) const MODEL_ARRAY_MODE2_ID: u8 = 2;
pub(crate) const MODEL_ARRAY_MODE4_ID: u8 = 4;
pub(crate) const MODEL_ARRAY_MODE4_ENABLE_FLAG: u8 = 0x01;
pub(crate) const MODEL_ARRAY_MODE4_SELECTION_BIAS: f32 = 6.3125;
pub(crate) const MODEL_ARRAY_FINAL_COST_BASE: f32 = 5.0;
pub(crate) const MODEL_ARRAY_ENTROPY_BYTES_BASE: usize = 0x0D;
pub(crate) const MODEL_ARRAY_SHUFFLED_HEADER_FLAG: u8 = 0x40;
pub(crate) const MODEL_ARRAY_PAYLOAD_CAPACITY_SLACK_BYTES: usize = 8;
pub(crate) const VARLEN_LITERAL_MIN_BYTES: usize = 0x60;
pub(crate) const VARLEN_LITERAL_LONG_THRESHOLD_BYTES: usize = 0x600;
pub(crate) const VARLEN_LITERAL_ALTERNATE_FLAG: u32 = 0x20;
pub(crate) const VARLEN_LITERAL_ALTERNATE_COST_BASE: f32 = 5.0;
pub(crate) const SHORT_VARLEN_LITERAL_SPLIT_PACKET_PREFIX: u8 = 0x02;
pub(crate) const SHORT_VARLEN_LITERAL_SPLIT_MIN_SIDE_BYTES: usize = 0x20;
pub(crate) const LONG_VARLEN_LITERAL_MIN_SEGMENTS: usize = 3;
pub(crate) const LONG_VARLEN_LITERAL_BASE_MAX_SEGMENTS: usize = 8;
pub(crate) const LONG_VARLEN_LITERAL_NORMAL_MAX_SEGMENTS: usize = 0x20;
pub(crate) const LONG_VARLEN_LITERAL_PARAM9_MAX_SEGMENTS: usize = 0x3E;
pub(crate) const LONG_VARLEN_LITERAL_HISTOGRAM_BYTES_PER_SEGMENT: usize = 0x400;
pub(crate) const LONG_VARLEN_LITERAL_RECORDS_PER_SEGMENT: usize = 3;
pub(crate) const LONG_VARLEN_LITERAL_MERGE_RECORD_BYTES: usize = 0x20;
pub(crate) const ALTERNATE_VARLEN_LITERAL_MIN_TOTAL_BYTES: usize = 0x60;
pub(crate) const ALTERNATE_VARLEN_LITERAL_MIN_SEGMENT_BYTES: usize = 0x20;
pub(crate) const ALTERNATE_VARLEN_LITERAL_MIN_WINDOW_BYTES: usize = 0x40;
pub(crate) const ALTERNATE_VARLEN_LITERAL_PRIMARY_SCRATCH_UNIT_BYTES: usize = 0x200;
pub(crate) const ALTERNATE_VARLEN_LITERAL_HISTOGRAM_BYTES_PER_SLOT: usize = 0x408;
pub(crate) const REPEATED_PATTERN_OPTIONAL_SUBSTREAM_FLAG: u32 = 0x08;
pub(crate) const REPEATED_PATTERN_OPTIONAL_SUBSTREAM_MIN_DATA_BYTES: usize = 0x20;
pub(crate) const REPEATED_PATTERN_OPTIONAL_SUBSTREAM_MAX_COMBINED_BYTES: usize = 0xC001;
pub(crate) const REPEATED_PATTERN_OPTIONAL_SUBSTREAM_ARENA_HEADER_BYTES: usize = 0x10;

fn varlen_literal_param_cap(param_7: u8) -> usize {
    if param_7 == 9 {
        LONG_VARLEN_LITERAL_PARAM9_MAX_SEGMENTS
    } else {
        (1usize << (param_7 & 0x1F))
            .min(LONG_VARLEN_LITERAL_NORMAL_MAX_SEGMENTS)
            .max(LONG_VARLEN_LITERAL_BASE_MAX_SEGMENTS)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ShortVarLenLiteralSplitPlan {
    pub packet_prefix: u8,
    pub min_side_bytes: usize,
    pub probes: Vec<usize>,
}

pub(crate) fn short_varlen_literal_split_plan(
    literal_len: usize,
) -> Result<ShortVarLenLiteralSplitPlan, CoreError> {
    if !(VARLEN_LITERAL_MIN_BYTES..VARLEN_LITERAL_LONG_THRESHOLD_BYTES).contains(&literal_len) {
        return Err(CoreError::Codec(format!(
            "short varlen literal split length {literal_len} is outside the reference range"
        )));
    }

    let probe_count = ((literal_len + 0x80) >> 8).clamp(1, 8);
    let probe_divisor = probe_count + 1;
    let probes = (1..=probe_count)
        .map(|probe_index| (literal_len * probe_index) / probe_divisor)
        .collect();

    Ok(ShortVarLenLiteralSplitPlan {
        packet_prefix: SHORT_VARLEN_LITERAL_SPLIT_PACKET_PREFIX,
        min_side_bytes: SHORT_VARLEN_LITERAL_SPLIT_MIN_SIDE_BYTES,
        probes,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LongVarLenLiteralInitialSegmentPlan {
    pub segment_count: usize,
    pub histogram_scratch_bytes: usize,
    pub merge_record_bytes: usize,
    pub segment_offsets: Vec<usize>,
    pub segment_lengths: Vec<usize>,
}

pub(crate) fn long_varlen_literal_initial_segment_plan(
    literal_len: usize,
    param_7: u8,
) -> Result<LongVarLenLiteralInitialSegmentPlan, CoreError> {
    if literal_len < VARLEN_LITERAL_LONG_THRESHOLD_BYTES {
        return Err(CoreError::Codec(format!(
            "long varlen literal length {literal_len} is below the reference threshold"
        )));
    }

    let rounded_units = literal_len
        .checked_add(0x100)
        .ok_or_else(|| CoreError::Codec("long varlen literal length overflow".to_string()))?
        >> 9;
    let max_segments = varlen_literal_param_cap(param_7);
    let segment_count = rounded_units
        .min(max_segments)
        .max(LONG_VARLEN_LITERAL_MIN_SEGMENTS);
    let histogram_scratch_bytes = segment_count
        .checked_mul(LONG_VARLEN_LITERAL_HISTOGRAM_BYTES_PER_SEGMENT)
        .ok_or_else(|| CoreError::Codec("long varlen histogram scratch overflow".to_string()))?;
    let merge_record_bytes = segment_count
        .checked_mul(LONG_VARLEN_LITERAL_RECORDS_PER_SEGMENT)
        .and_then(|value| value.checked_mul(LONG_VARLEN_LITERAL_MERGE_RECORD_BYTES))
        .ok_or_else(|| CoreError::Codec("long varlen merge record overflow".to_string()))?;

    let base_segment_len = literal_len / segment_count;
    let mut remaining_len = literal_len;
    let mut offset = 0usize;
    let mut segment_offsets = Vec::with_capacity(segment_count);
    let mut segment_lengths = Vec::with_capacity(segment_count);
    for segment_index in 0..segment_count {
        segment_offsets.push(offset);
        let segment_len = if segment_index == segment_count - 1 {
            remaining_len
        } else {
            base_segment_len
        };
        segment_lengths.push(segment_len);
        offset = offset
            .checked_add(base_segment_len)
            .ok_or_else(|| CoreError::Codec("long varlen segment offset overflow".to_string()))?;
        remaining_len = remaining_len
            .checked_sub(base_segment_len)
            .ok_or_else(|| CoreError::Codec("long varlen segment length underflow".to_string()))?;
    }

    Ok(LongVarLenLiteralInitialSegmentPlan {
        segment_count,
        histogram_scratch_bytes,
        merge_record_bytes,
        segment_offsets,
        segment_lengths,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AlternateVarLenLiteralContextPlan {
    pub enabled: bool,
    pub total_len: usize,
    pub max_segment_len: usize,
    pub cap: usize,
    pub double_cap: usize,
    pub window_floor: usize,
    pub primary_scratch_bytes: usize,
    pub histogram_scratch_bytes: usize,
}

pub(crate) fn alternate_varlen_literal_context_plan(
    segment_lengths: &[usize],
    param_11: u8,
) -> Result<AlternateVarLenLiteralContextPlan, CoreError> {
    let mut total_len = 0usize;
    let mut max_segment_len = 0usize;
    for segment_len in segment_lengths {
        total_len = total_len.checked_add(*segment_len).ok_or_else(|| {
            CoreError::Codec("alternate varlen total length overflow".to_string())
        })?;
        max_segment_len = max_segment_len.max(*segment_len);
    }

    let cap = varlen_literal_param_cap(param_11);
    let double_cap = cap
        .checked_mul(2)
        .ok_or_else(|| CoreError::Codec("alternate varlen cap overflow".to_string()))?;
    let enabled = total_len >= ALTERNATE_VARLEN_LITERAL_MIN_TOTAL_BYTES
        && max_segment_len >= ALTERNATE_VARLEN_LITERAL_MIN_SEGMENT_BYTES;
    let window_floor = ALTERNATE_VARLEN_LITERAL_MIN_WINDOW_BYTES.max(total_len / 100);
    let primary_scratch_bytes = if enabled {
        double_cap
            .checked_add(1)
            .and_then(|value| {
                value.checked_mul(ALTERNATE_VARLEN_LITERAL_PRIMARY_SCRATCH_UNIT_BYTES)
            })
            .ok_or_else(|| {
                CoreError::Codec("alternate varlen primary scratch overflow".to_string())
            })?
    } else {
        0
    };
    let histogram_scratch_bytes = if enabled {
        double_cap
            .checked_mul(ALTERNATE_VARLEN_LITERAL_HISTOGRAM_BYTES_PER_SLOT)
            .ok_or_else(|| {
                CoreError::Codec("alternate varlen histogram scratch overflow".to_string())
            })?
    } else {
        0
    };

    Ok(AlternateVarLenLiteralContextPlan {
        enabled,
        total_len,
        max_segment_len,
        cap,
        double_cap,
        window_floor,
        primary_scratch_bytes,
        histogram_scratch_bytes,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum VarLenLiteralEntropyKind {
    ShortRange,
    LongRange,
    Alternate,
}

impl VarLenLiteralEntropyKind {
    fn as_str(self) -> &'static str {
        match self {
            VarLenLiteralEntropyKind::ShortRange => "short_range",
            VarLenLiteralEntropyKind::LongRange => "long_range",
            VarLenLiteralEntropyKind::Alternate => "alternate",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct VarLenLiteralEntropyCandidate {
    pub encoded_bytes: usize,
    pub cost: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct VarLenLiteralEntropyDispatch {
    pub kind: VarLenLiteralEntropyKind,
    pub encoded_bytes: usize,
    pub cost: f32,
}

pub(crate) trait VarLenLiteralEntropyBuilder {
    fn encode_short_range(
        &mut self,
        literal_len: usize,
    ) -> Result<Option<VarLenLiteralEntropyCandidate>, CoreError>;

    fn encode_long_range(
        &mut self,
        literal_len: usize,
    ) -> Result<Option<VarLenLiteralEntropyCandidate>, CoreError>;

    fn encode_alternate(
        &mut self,
        literal_len: usize,
        baseline_cost_without_base: f32,
    ) -> Result<Option<VarLenLiteralEntropyCandidate>, CoreError>;
}

pub(crate) fn dispatch_varlen_literal_entropy(
    literal_len: usize,
    flags: u32,
    inherited_best_cost: f32,
    candidate_cost: &mut f32,
    builder: &mut dyn VarLenLiteralEntropyBuilder,
) -> Result<Option<VarLenLiteralEntropyDispatch>, CoreError> {
    if literal_len < VARLEN_LITERAL_MIN_BYTES {
        return Ok(None);
    }

    let mut result = if literal_len < VARLEN_LITERAL_LONG_THRESHOLD_BYTES {
        builder.encode_short_range(literal_len)?.map(|candidate| {
            *candidate_cost = candidate.cost;
            VarLenLiteralEntropyDispatch {
                kind: VarLenLiteralEntropyKind::ShortRange,
                encoded_bytes: candidate.encoded_bytes,
                cost: candidate.cost,
            }
        })
    } else {
        builder.encode_long_range(literal_len)?.map(|candidate| {
            *candidate_cost = candidate.cost;
            VarLenLiteralEntropyDispatch {
                kind: VarLenLiteralEntropyKind::LongRange,
                encoded_bytes: candidate.encoded_bytes,
                cost: candidate.cost,
            }
        })
    };

    if flags & VARLEN_LITERAL_ALTERNATE_FLAG != 0 {
        let baseline_cost =
            (*candidate_cost).min(inherited_best_cost) - VARLEN_LITERAL_ALTERNATE_COST_BASE;
        if let Some(candidate) = builder.encode_alternate(literal_len, baseline_cost)? {
            let final_cost = candidate.cost + VARLEN_LITERAL_ALTERNATE_COST_BASE;
            *candidate_cost = final_cost;
            result = Some(VarLenLiteralEntropyDispatch {
                kind: VarLenLiteralEntropyKind::Alternate,
                encoded_bytes: candidate.encoded_bytes,
                cost: final_cost,
            });
        }
    }

    Ok(result)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BlockEncoderKind {
    GenericNewLz,
    KrakenChunkOptimal,
}

impl BlockEncoderKind {
    fn as_str(self) -> &'static str {
        match self {
            BlockEncoderKind::GenericNewLz => "generic_newlz",
            BlockEncoderKind::KrakenChunkOptimal => "kraken_chunk_optimal",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SecondaryHelperKind {
    None,
    KrakenLevel5,
    KrakenLevel6Plus,
}

impl SecondaryHelperKind {
    fn as_str(self) -> &'static str {
        match self {
            SecondaryHelperKind::None => "none",
            SecondaryHelperKind::KrakenLevel5 => "kraken_level_5",
            SecondaryHelperKind::KrakenLevel6Plus => "kraken_level_6_plus",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct KrakenEncoderContext {
    pub compressor_id: u8,
    pub compression_level: u8,
    pub block_len: usize,
    pub primary_block_encoder: BlockEncoderKind,
    pub secondary_helper: SecondaryHelperKind,
    pub high_compression_flag: bool,
    pub encode_scratch_bytes: usize,
}

impl KrakenEncoderContext {
    pub fn for_g1r_level(compression_level: u8) -> Self {
        let (primary_block_encoder, secondary_helper, high_compression_flag) =
            if compression_level > 5 {
                (
                    BlockEncoderKind::KrakenChunkOptimal,
                    SecondaryHelperKind::KrakenLevel6Plus,
                    true,
                )
            } else if compression_level == 5 {
                (
                    BlockEncoderKind::KrakenChunkOptimal,
                    SecondaryHelperKind::KrakenLevel5,
                    false,
                )
            } else {
                (
                    BlockEncoderKind::GenericNewLz,
                    SecondaryHelperKind::None,
                    false,
                )
            };

        Self {
            compressor_id: OODLE_COMPRESSOR_KRAKEN,
            compression_level,
            block_len: KRAKEN_BLOCK_LEN,
            primary_block_encoder,
            secondary_helper,
            high_compression_flag,
            encode_scratch_bytes: KRAKEN_OPTIMAL_SCRATCH_BYTES,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EmissionStateLayout {
    pub capacity: usize,
    pub half_plus_header: usize,
    pub third: usize,
    pub fifth: usize,
    pub byte_table_bytes: usize,
    pub backing_len: usize,
    pub cursor_starts: [usize; 7],
    pub byte_table_end: usize,
}

impl EmissionStateLayout {
    pub fn for_capacity(capacity: usize) -> Self {
        let half_plus_header = capacity / 2 + 8;
        let third = capacity / 3;
        let fifth = capacity / 5;
        let byte_table_bytes = (capacity >> 8) * 4;
        let backing_len =
            third * 5 + (capacity + 0x88) * 2 + byte_table_bytes + fifth + half_plus_header;

        let first = 0;
        let second = first + capacity + 8;
        let third_cursor = second + capacity + 8;
        let fourth = third_cursor + half_plus_header;
        let fifth_cursor = align4(fourth + third);
        let sixth = fifth_cursor + third * 4;
        let seventh = align4(sixth + fifth);
        let byte_table_end = seventh + byte_table_bytes;

        Self {
            capacity,
            half_plus_header,
            third,
            fifth,
            byte_table_bytes,
            backing_len,
            cursor_starts: [
                first,
                second,
                third_cursor,
                fourth,
                fifth_cursor,
                sixth,
                seventh,
            ],
            byte_table_end,
        }
    }
}

fn align4(value: usize) -> usize {
    (value + 3) & !3
}

fn align16_checked(value: usize) -> Result<usize, CoreError> {
    value
        .checked_add(0x0F)
        .map(|value| value & !0x0F)
        .ok_or_else(|| CoreError::Codec("16-byte alignment overflow".to_string()))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EmissionState {
    layout: EmissionStateLayout,
    backing: Vec<u8>,
    stream_spans: [(usize, usize); 7],
    cursor_offsets: [usize; 7],
    pub source_base: usize,
    pub user_tag: u32,
}

impl EmissionState {
    pub fn new(capacity: usize, source_base: usize, user_tag: u32) -> Self {
        let layout = EmissionStateLayout::for_capacity(capacity);
        let starts = layout.cursor_starts;
        let stream_spans = [
            (starts[0], starts[1]),
            (starts[1], starts[2]),
            (starts[2], starts[3]),
            (starts[3], starts[4]),
            (starts[4], starts[5]),
            (starts[5], starts[6]),
            (starts[6], layout.byte_table_end),
        ];

        Self {
            backing: vec![0; layout.backing_len],
            cursor_offsets: starts,
            stream_spans,
            layout,
            source_base,
            user_tag,
        }
    }

    pub fn backing_len(&self) -> usize {
        self.backing.len()
    }

    pub fn backing(&self) -> &[u8] {
        &self.backing
    }

    pub fn stream_spans(&self) -> [(usize, usize); 7] {
        self.stream_spans
    }

    pub fn cursor_offsets(&self) -> [usize; 7] {
        self.cursor_offsets
    }

    pub fn write_stream(&mut self, stream_index: usize, bytes: &[u8]) -> Result<(), CoreError> {
        let (_start, end) = self
            .stream_spans
            .get(stream_index)
            .copied()
            .ok_or_else(|| {
                CoreError::Codec(format!("emission stream {stream_index} does not exist"))
            })?;
        let cursor = self.cursor_offsets[stream_index];
        let new_cursor = cursor.checked_add(bytes.len()).ok_or_else(|| {
            CoreError::Codec(format!(
                "emission stream {stream_index} cursor arithmetic overflow"
            ))
        })?;

        if new_cursor > end {
            return Err(CoreError::Codec(format!(
                "emission stream {stream_index} overflow: cursor {cursor:#X}, bytes {}, end {end:#X}",
                bytes.len()
            )));
        }

        self.backing[cursor..new_cursor].copy_from_slice(bytes);
        self.cursor_offsets[stream_index] = new_cursor;
        Ok(())
    }

    pub fn byte_table_span(&self) -> (usize, usize) {
        (self.layout.cursor_starts[6], self.layout.byte_table_end)
    }

    pub fn tail_len(&self) -> usize {
        self.layout.backing_len - self.layout.byte_table_end
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct JobHandle(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum JobDispatchMode {
    Inline,
    Scheduled,
}

impl JobDispatchMode {
    fn as_str(self) -> &'static str {
        match self {
            JobDispatchMode::Inline => "inline",
            JobDispatchMode::Scheduled => "scheduled",
        }
    }
}

pub(crate) fn dispatch_kraken_job(
    output_handle: &mut Option<JobHandle>,
    run_async: bool,
    dependency_a: Option<JobHandle>,
    dependency_b: Option<JobHandle>,
    scheduler: Option<&mut dyn KrakenJobScheduler>,
    callback: impl FnOnce() -> Result<(), CoreError>,
) -> Result<JobDispatchMode, CoreError> {
    if run_async {
        let scheduler = scheduler.ok_or_else(|| {
            CoreError::Codec("async Kraken job scheduling requires a scheduler".to_string())
        })?;
        let dependencies = compact_job_dependencies(dependency_a, dependency_b);
        let handle = scheduler.schedule(&dependencies)?;
        *output_handle = Some(handle);
        return Ok(JobDispatchMode::Scheduled);
    }

    callback()?;
    *output_handle = None;
    Ok(JobDispatchMode::Inline)
}

fn compact_job_dependencies(
    dependency_a: Option<JobHandle>,
    dependency_b: Option<JobHandle>,
) -> Vec<JobHandle> {
    [dependency_a, dependency_b].into_iter().flatten().collect()
}

pub(crate) trait KrakenJobScheduler {
    fn schedule(&mut self, dependencies: &[JobHandle]) -> Result<JobHandle, CoreError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SubjobPlan {
    pub raw_len: usize,
    pub subjob_count: usize,
    pub leading_window_bytes: usize,
    pub scratch_allocation_bytes: usize,
}

impl SubjobPlan {
    pub fn from_raw_len(
        raw_len: usize,
        max_scheduler_jobs: Option<usize>,
        record_stride: usize,
    ) -> Result<Self, CoreError> {
        let mut max_jobs = max_scheduler_jobs.unwrap_or(1).max(1);
        if raw_len < NEWLZ_SUBJOB_THRESHOLD_BYTES {
            max_jobs = 1;
        }

        let window_count = ceil_div(raw_len, NEWLZ_SUBJOB_WINDOW_BYTES).max(1);
        let subjob_count = window_count.min(max_jobs);
        let leading_window_cap = subjob_count
            .checked_mul(NEWLZ_SUBJOB_WINDOW_BYTES)
            .ok_or_else(|| CoreError::Codec("subjob leading window size overflow".to_string()))?;
        let leading_window_bytes = raw_len.min(leading_window_cap);
        let scratch_allocation_bytes = leading_window_bytes
            .checked_mul(record_stride)
            .and_then(|value| value.checked_mul(8))
            .ok_or_else(|| {
                CoreError::Codec("subjob scratch allocation size overflow".to_string())
            })?;

        Ok(Self {
            raw_len,
            subjob_count,
            leading_window_bytes,
            scratch_allocation_bytes,
        })
    }
}

fn ceil_div(value: usize, divisor: usize) -> usize {
    if value == 0 {
        0
    } else {
        1 + (value - 1) / divisor
    }
}

pub(crate) fn write_raw_literal_packet(
    output: &mut Vec<u8>,
    output_capacity: usize,
    literal: &[u8],
) -> Result<usize, CoreError> {
    if literal.len() > RAW_LITERAL_PACKET_MAX_BYTES {
        return Err(CoreError::Codec(format!(
            "raw literal packet length exceeds reference 0x3FFFF limit: {}",
            literal.len()
        )));
    }

    let encoded_len = literal
        .len()
        .checked_add(3)
        .ok_or_else(|| CoreError::Codec("raw literal packet size overflow".to_string()))?;
    let remaining = output_capacity.saturating_sub(output.len());
    if remaining < encoded_len {
        return Err(CoreError::Codec(format!(
            "raw literal packet output capacity exceeded: remaining {remaining}, required {encoded_len}"
        )));
    }

    output.push((literal.len() >> 16) as u8);
    output.push((literal.len() >> 8) as u8);
    output.push(literal.len() as u8);
    output.extend_from_slice(literal);
    Ok(encoded_len)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct LiteralPacketResult {
    pub mode: LiteralPacketMode,
    pub encoded_bytes: usize,
    pub cost: f32,
}

pub(crate) trait LiteralEntropyPacketEncoder {
    fn encode_literal_packet(
        &mut self,
        output: &mut Vec<u8>,
        output_capacity: usize,
        literal: &[u8],
        histogram: &[u32; 256],
    ) -> Result<LiteralPacketResult, CoreError>;
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LiteralEntropyModelCandidate {
    pub mode_id: u8,
    pub payload: Vec<u8>,
    pub cost: f32,
}

pub(crate) trait LiteralEntropyModelBuilder {
    fn encode_model_array_candidate(
        &mut self,
        literal: &[u8],
        histogram: &[u32; 256],
        baseline_cost: f32,
    ) -> Result<Option<LiteralEntropyModelCandidate>, CoreError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LiteralEntropyTablePlan {
    pub tail_len: usize,
    pub table_bits: usize,
    pub table_size: usize,
    pub effective_symbol_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LiteralEntropyTableCandidate {
    pub state_count: usize,
    pub payload: Vec<u8>,
    pub cost: f32,
}

pub(crate) trait LiteralEntropyTableBuilder {
    fn encode_table_candidate(
        &mut self,
        literal: &[u8],
        adjusted_histogram: &[u32; 256],
        plan: LiteralEntropyTablePlan,
        current_best_cost: f32,
    ) -> Result<Option<LiteralEntropyTableCandidate>, CoreError>;
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct LiteralEntropyRepeatedPatternPlan {
    pub payload_budget: usize,
    pub baseline_cost: f32,
    pub pre_cost: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LiteralEntropyRepeatedPatternCandidate {
    pub payload: Vec<u8>,
    pub cost: f32,
}

pub(crate) trait LiteralEntropyRepeatedPatternBuilder {
    fn encode_repeated_pattern_candidate(
        &mut self,
        literal: &[u8],
        plan: LiteralEntropyRepeatedPatternPlan,
    ) -> Result<Option<LiteralEntropyRepeatedPatternCandidate>, CoreError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RepeatedPatternPayload {
    pub payload: Vec<u8>,
    pub data_bytes: usize,
    pub control_bytes: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct RepeatedPatternOptionalSubstreamPlan {
    pub data_bytes: usize,
    pub control_bytes: usize,
    pub combined_bytes: usize,
    pub aligned_data_bytes: usize,
    pub arena_header_bytes: usize,
    pub scratch_bytes: usize,
    pub baseline_cost: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RepeatedPatternOptionalSubstreamCandidate {
    pub payload: Vec<u8>,
    pub data_packet_bytes: usize,
    pub control_bytes: usize,
    pub substream_cost: f32,
    pub total_cost: f32,
    pub baseline_cost: f32,
    pub mode: LiteralPacketMode,
}

pub(crate) fn repeated_pattern_optional_substream_plan(
    flags: u32,
    data_bytes: usize,
    control_bytes: usize,
    arena_tracks_allocations: bool,
) -> Result<Option<RepeatedPatternOptionalSubstreamPlan>, CoreError> {
    if flags & REPEATED_PATTERN_OPTIONAL_SUBSTREAM_FLAG == 0
        || data_bytes < REPEATED_PATTERN_OPTIONAL_SUBSTREAM_MIN_DATA_BYTES
    {
        return Ok(None);
    }

    let combined_bytes = data_bytes.checked_add(control_bytes).ok_or_else(|| {
        CoreError::Codec("repeated-pattern optional substream size overflow".to_string())
    })?;
    if combined_bytes >= REPEATED_PATTERN_OPTIONAL_SUBSTREAM_MAX_COMBINED_BYTES {
        return Ok(None);
    }

    let aligned_data_bytes = align16_checked(data_bytes)?;
    let arena_header_bytes = if arena_tracks_allocations {
        REPEATED_PATTERN_OPTIONAL_SUBSTREAM_ARENA_HEADER_BYTES
    } else {
        0
    };
    let scratch_bytes = aligned_data_bytes
        .checked_add(arena_header_bytes)
        .ok_or_else(|| {
            CoreError::Codec("repeated-pattern optional substream scratch overflow".to_string())
        })?;

    Ok(Some(RepeatedPatternOptionalSubstreamPlan {
        data_bytes,
        control_bytes,
        combined_bytes,
        aligned_data_bytes,
        arena_header_bytes,
        scratch_bytes,
        baseline_cost: data_bytes as f32 + 1.0,
    }))
}

pub(crate) fn encode_repeated_pattern_optional_single_symbol_substream(
    data: &[u8],
    control: &[u8],
    flags: u32,
    model_cost: f32,
    cost_scale: f32,
    arena_tracks_allocations: bool,
) -> Result<Option<RepeatedPatternOptionalSubstreamCandidate>, CoreError> {
    let Some(plan) = repeated_pattern_optional_substream_plan(
        flags,
        data.len(),
        control.len(),
        arena_tracks_allocations,
    )?
    else {
        return Ok(None);
    };
    if max_literal_frequency(data) != data.len() {
        return Ok(None);
    }

    let mut payload = Vec::new();
    let result = write_entropy_single_symbol_packet(
        &mut payload,
        plan.scratch_bytes,
        data,
        SingleSymbolPacketMode::DirectSymbol,
        model_cost,
        cost_scale,
        plan.baseline_cost,
    )?;
    let result = compact_wrapped_literal_packet(&mut payload, 0, result)?;
    if result.cost >= plan.baseline_cost {
        return Ok(None);
    }

    let data_packet_bytes = result.encoded_bytes;
    payload.extend_from_slice(control);
    let total_cost = result.cost + control.len() as f32;
    Ok(Some(RepeatedPatternOptionalSubstreamCandidate {
        payload,
        data_packet_bytes,
        control_bytes: control.len(),
        substream_cost: result.cost,
        total_cost,
        baseline_cost: plan.baseline_cost,
        mode: result.mode,
    }))
}

pub(crate) fn encode_repeated_pattern_optional_model_array_substream(
    data: &[u8],
    control: &[u8],
    flags: u32,
    arena_tracks_allocations: bool,
    builder: &mut dyn LiteralEntropyModelBuilder,
) -> Result<Option<RepeatedPatternOptionalSubstreamCandidate>, CoreError> {
    let Some(plan) = repeated_pattern_optional_substream_plan(
        flags,
        data.len(),
        control.len(),
        arena_tracks_allocations,
    )?
    else {
        return Ok(None);
    };

    let mut histogram = [0u32; 256];
    build_literal_histogram(data, &mut histogram, true);
    if histogram.iter().copied().max().unwrap_or(0) as usize == data.len() {
        return Ok(None);
    }

    let Some(candidate) =
        builder.encode_model_array_candidate(data, &histogram, plan.baseline_cost)?
    else {
        return Ok(None);
    };
    if candidate.payload.len() >= data.len() {
        return Ok(None);
    }

    let packet_len = candidate.payload.len().checked_add(5).ok_or_else(|| {
        CoreError::Codec("repeated-pattern optional substream packet overflow".to_string())
    })?;
    if packet_len > plan.scratch_bytes {
        return Err(CoreError::Codec(format!(
            "repeated-pattern optional substream scratch capacity exceeded: scratch {}, required {packet_len}",
            plan.scratch_bytes
        )));
    }

    let mut payload = Vec::new();
    let header =
        entropy_literal_header_bytes(candidate.mode_id, data.len(), candidate.payload.len())?;
    payload.extend_from_slice(&header);
    payload.extend_from_slice(&candidate.payload);
    let result = compact_wrapped_literal_packet(
        &mut payload,
        0,
        LiteralPacketResult {
            mode: LiteralPacketMode::EntropyCandidate,
            encoded_bytes: packet_len,
            cost: candidate.cost,
        },
    )?;
    if result.cost >= plan.baseline_cost {
        return Ok(None);
    }

    let data_packet_bytes = result.encoded_bytes;
    payload.extend_from_slice(control);
    let total_cost = result.cost + control.len() as f32;
    Ok(Some(RepeatedPatternOptionalSubstreamCandidate {
        payload,
        data_packet_bytes,
        control_bytes: control.len(),
        substream_cost: result.cost,
        total_cost,
        baseline_cost: plan.baseline_cost,
        mode: result.mode,
    }))
}

fn encode_model_array_single_symbol_candidate(
    literal: &[u8],
    histogram: &[u32; 256],
    baseline_cost: f32,
    model_cost: f32,
    cost_scale: f32,
) -> Result<Option<LiteralEntropyModelCandidate>, CoreError> {
    if literal.is_empty() || max_literal_frequency(literal) != literal.len() {
        return Ok(None);
    }

    let symbol_count = histogram
        .iter()
        .filter(|count| **count != 0)
        .take(2)
        .count();
    if symbol_count >= 2 {
        return Ok(None);
    }

    let cost = model_cost * cost_scale + SINGLE_SYMBOL_SPLIT_COST_BASE;
    if cost >= baseline_cost {
        return Ok(None);
    }

    Ok(Some(LiteralEntropyModelCandidate {
        mode_id: SINGLE_SYMBOL_SPLIT_MODE_ID,
        payload: vec![0, (literal[0] >> 2) | 0x40, literal[0] << 6],
        cost,
    }))
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct LiteralEntropyModelArraySelection {
    pub mode_id: u8,
    pub symbol_count: usize,
    pub ranked_symbol_count: usize,
    pub shuffled_pair_count: usize,
    pub entropy_bits: u32,
    pub entropy_bits_per_byte: f32,
    pub side_header_bytes: usize,
    pub entropy_bytes_with_header: usize,
    pub scaled_cost_delta_with_bias: Option<f32>,
    pub selected_model_cost: f32,
    pub final_cost: f32,
    pub projected_cost: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct LiteralEntropyModelArrayCostInputs {
    pub literal_len: usize,
    pub symbol_count: usize,
    pub ranked_symbol_count: usize,
    pub entropy_bits: u32,
    pub flags: u8,
    pub cost_scale: f32,
    pub model_cost_limit: f32,
    pub current_best_cost: f32,
    pub mode2_model_cost: f32,
    pub mode4_model_cost: f32,
}

pub(crate) fn select_model_array_mode(
    inputs: LiteralEntropyModelArrayCostInputs,
) -> Result<Option<LiteralEntropyModelArraySelection>, CoreError> {
    if inputs.literal_len == 0 {
        return Err(CoreError::Codec(
            "model-array mode selection requires non-empty literal".to_string(),
        ));
    }
    if inputs.symbol_count < 2 {
        return Ok(None);
    }

    let entropy_bits_per_byte = inputs.entropy_bits as f32 / inputs.literal_len as f32;
    let shuffled_pair_count = inputs.ranked_symbol_count.saturating_sub(1) / 2;
    let mut mode_id = MODEL_ARRAY_MODE2_ID;
    let mut selected_model_cost = inputs.mode2_model_cost;
    let mut scaled_cost_delta_with_bias = None;

    if inputs.flags & MODEL_ARRAY_MODE4_ENABLE_FLAG != 0 {
        let delta = (inputs.mode4_model_cost - inputs.mode2_model_cost) * inputs.cost_scale
            + MODEL_ARRAY_MODE4_SELECTION_BIAS;
        scaled_cost_delta_with_bias = Some(delta);
        if delta < 0.0 {
            mode_id = MODEL_ARRAY_MODE4_ID;
            selected_model_cost = inputs.mode4_model_cost;
        }
    }

    if selected_model_cost > inputs.model_cost_limit {
        return Ok(None);
    }

    let final_cost = selected_model_cost * inputs.cost_scale + MODEL_ARRAY_FINAL_COST_BASE;
    let side_header_bytes = (inputs
        .symbol_count
        .checked_mul(2)
        .and_then(|value| value.checked_add(0x0F))
        .ok_or_else(|| CoreError::Codec("model-array side header overflow".to_string()))?)
        >> 3;
    let entropy_bytes_with_header = ((inputs.entropy_bits as usize + 7) >> 3)
        .checked_add(MODEL_ARRAY_ENTROPY_BYTES_BASE)
        .ok_or_else(|| CoreError::Codec("model-array entropy byte count overflow".to_string()))?;
    let projected_cost = side_header_bytes as f32 + entropy_bytes_with_header as f32 + final_cost;
    if projected_cost >= inputs.current_best_cost {
        return Ok(None);
    }

    Ok(Some(LiteralEntropyModelArraySelection {
        mode_id,
        symbol_count: inputs.symbol_count,
        ranked_symbol_count: inputs.ranked_symbol_count,
        shuffled_pair_count,
        entropy_bits: inputs.entropy_bits,
        entropy_bits_per_byte,
        side_header_bytes,
        entropy_bytes_with_header,
        scaled_cost_delta_with_bias,
        selected_model_cost,
        final_cost,
        projected_cost,
    }))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LiteralEntropyModelArraySideHeaderPlan {
    pub uses_shuffled_ranks: bool,
    pub symbol_count: usize,
    pub ranked_symbol_count: usize,
    pub shuffled_pair_count: usize,
    pub prefix_value: u8,
    pub prefix_bits: u8,
    pub initial_bit_cursor: u8,
    pub side_header_bytes: usize,
    pub payload_capacity_slack_bytes: usize,
}

impl LiteralEntropyModelArraySideHeaderPlan {
    fn path(self) -> &'static str {
        if self.uses_shuffled_ranks {
            "shuffled"
        } else {
            "plain"
        }
    }
}

pub(crate) fn model_array_side_header_plan(
    symbol_count: usize,
    ranked_symbol_count: usize,
    flags: u8,
) -> Result<Option<LiteralEntropyModelArraySideHeaderPlan>, CoreError> {
    if symbol_count < 2 {
        return Ok(None);
    }

    let uses_shuffled_ranks = symbol_count >= 5 && (flags & MODEL_ARRAY_SHUFFLED_HEADER_FLAG) != 0;
    let (prefix_value, prefix_bits, initial_bit_cursor) = if uses_shuffled_ranks {
        (2, 2, 0x3D)
    } else {
        (0, 1, 0x3E)
    };
    let side_header_bytes = symbol_count
        .checked_mul(2)
        .and_then(|value| value.checked_add(0x0F))
        .ok_or_else(|| CoreError::Codec("model-array side header overflow".to_string()))?
        >> 3;

    Ok(Some(LiteralEntropyModelArraySideHeaderPlan {
        uses_shuffled_ranks,
        symbol_count,
        ranked_symbol_count,
        shuffled_pair_count: ranked_symbol_count.saturating_sub(1) / 2,
        prefix_value,
        prefix_bits,
        initial_bit_cursor,
        side_header_bytes,
        payload_capacity_slack_bytes: MODEL_ARRAY_PAYLOAD_CAPACITY_SLACK_BYTES,
    }))
}

pub(crate) fn write_model_array_side_header_seed(
    plan: LiteralEntropyModelArraySideHeaderPlan,
) -> Vec<u8> {
    let mut output = vec![0u8; usize::from(plan.prefix_bits).div_ceil(8)];
    for bit_index in 0..plan.prefix_bits {
        let bit_shift = plan.prefix_bits - bit_index - 1;
        let bit = (plan.prefix_value >> bit_shift) & 1;
        if bit != 0 {
            output[usize::from(bit_index / 8)] |= 0x80 >> (bit_index % 8);
        }
    }
    output
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LiteralEntropyModelArrayPlainSmallHeader {
    pub alphabet_size: usize,
    pub symbol_count: usize,
    pub single_symbol_index: Option<usize>,
    pub max_code_len: u8,
    pub code_lengths: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LiteralEntropyModelArrayPlainLargeHeader {
    pub alphabet_size: usize,
    pub symbol_count: usize,
    pub code_lengths: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LiteralEntropyModelArraySideHeaderBytes {
    pub path: &'static str,
    pub bytes: Vec<u8>,
    pub bit_len: usize,
    pub symbol_index_bits: u8,
    pub code_len_bits: Option<u8>,
    pub nonzero_symbols: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LiteralEntropyModelArrayPlainLargePreamble {
    pub path: &'static str,
    pub bytes: Vec<u8>,
    pub bit_len: usize,
    pub symbol_index_bits: u8,
    pub selected_predictor: u8,
    pub predictor_scores: [u32; 4],
    pub residual_histogram: [u32; 32],
    pub residuals: Vec<u8>,
    pub nonzero_symbols: usize,
    pub first_symbol_has_code_len: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LiteralEntropyModelArrayPlainLargeRun {
    pub kind: &'static str,
    pub len: usize,
    pub descriptor_bits: String,
    pub residual_bits: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LiteralEntropyModelArrayPlainLargeBody {
    pub path: &'static str,
    pub bytes: Vec<u8>,
    pub bit_len: usize,
    pub selected_predictor: u8,
    pub runs: Vec<LiteralEntropyModelArrayPlainLargeRun>,
}

fn model_array_bit_width(value_count: usize) -> u8 {
    if value_count <= 1 {
        0
    } else {
        usize::BITS as u8 - (value_count - 1).leading_zeros() as u8
    }
}

fn append_model_array_bits(
    output: &mut Vec<u8>,
    bit_len: &mut usize,
    value: u64,
    bit_count: u8,
) -> Result<(), CoreError> {
    if bit_count == 0 {
        if value != 0 {
            return Err(CoreError::Codec(format!(
                "model-array side header value {value} does not fit zero bits"
            )));
        }
        return Ok(());
    }
    if bit_count < 64 && value >= (1u64 << bit_count) {
        return Err(CoreError::Codec(format!(
            "model-array side header value {value} does not fit {bit_count} bits"
        )));
    }

    for shift in (0..bit_count).rev() {
        let byte_index = *bit_len / 8;
        let bit_index = *bit_len % 8;
        if byte_index == output.len() {
            output.push(0);
        }
        if ((value >> shift) & 1) != 0 {
            output[byte_index] |= 0x80 >> bit_index;
        }
        *bit_len += 1;
    }
    Ok(())
}

fn append_model_array_bit_string(
    output: &mut Vec<u8>,
    bit_len: &mut usize,
    bits: &str,
) -> Result<(), CoreError> {
    for bit in bits.bytes() {
        match bit {
            b'0' => append_model_array_bits(output, bit_len, 0, 1)?,
            b'1' => append_model_array_bits(output, bit_len, 1, 1)?,
            _ => {
                return Err(CoreError::Codec(
                    "model-array bit string contains a non-bit character".to_string(),
                ));
            }
        }
    }
    Ok(())
}

pub(crate) fn write_model_array_plain_small_side_header(
    plan: LiteralEntropyModelArraySideHeaderPlan,
    header: LiteralEntropyModelArrayPlainSmallHeader,
) -> Result<LiteralEntropyModelArraySideHeaderBytes, CoreError> {
    if plan.uses_shuffled_ranks {
        return Err(CoreError::Codec(
            "plain-small model-array side header cannot use shuffled ranks".to_string(),
        ));
    }
    if header.symbol_count != plan.symbol_count || header.symbol_count > 4 {
        return Err(CoreError::Codec(format!(
            "plain-small model-array symbol count mismatch: plan {}, header {}",
            plan.symbol_count, header.symbol_count
        )));
    }
    if header.code_lengths.len() != header.alphabet_size {
        return Err(CoreError::Codec(format!(
            "plain-small model-array alphabet length mismatch: alphabet {}, code lengths {}",
            header.alphabet_size,
            header.code_lengths.len()
        )));
    }

    let symbol_index_bits = model_array_bit_width(header.alphabet_size);
    let mut bytes = Vec::new();
    let mut bit_len = 0usize;
    append_model_array_bits(
        &mut bytes,
        &mut bit_len,
        u64::from(plan.prefix_value),
        plan.prefix_bits,
    )?;
    append_model_array_bits(
        &mut bytes,
        &mut bit_len,
        header.symbol_count as u64,
        symbol_index_bits,
    )?;

    let mut code_len_bits = None;
    let nonzero_symbols = header
        .code_lengths
        .iter()
        .filter(|code_len| **code_len != 0)
        .count();

    if header.symbol_count == 1 {
        let Some(index) = header.single_symbol_index else {
            return Err(CoreError::Codec(
                "plain-small single-symbol model-array header requires a symbol index".to_string(),
            ));
        };
        append_model_array_bits(&mut bytes, &mut bit_len, index as u64, symbol_index_bits)?;
    } else if header.symbol_count > 1 {
        if nonzero_symbols != header.symbol_count {
            return Err(CoreError::Codec(format!(
                "plain-small model-array nonzero symbol mismatch: expected {}, got {}",
                header.symbol_count, nonzero_symbols
            )));
        }
        let len_bits = model_array_bit_width(header.max_code_len as usize);
        if len_bits > 7 {
            return Err(CoreError::Codec(format!(
                "plain-small model-array code-length bit width {len_bits} exceeds the three-bit field"
            )));
        }
        code_len_bits = Some(len_bits);
        append_model_array_bits(&mut bytes, &mut bit_len, u64::from(len_bits), 3)?;
        for (symbol_index, code_len) in header.code_lengths.iter().copied().enumerate() {
            if code_len == 0 {
                continue;
            }
            if code_len > header.max_code_len {
                return Err(CoreError::Codec(format!(
                    "plain-small model-array code length {code_len} exceeds max {}",
                    header.max_code_len
                )));
            }
            append_model_array_bits(
                &mut bytes,
                &mut bit_len,
                symbol_index as u64,
                symbol_index_bits,
            )?;
            append_model_array_bits(&mut bytes, &mut bit_len, u64::from(code_len - 1), len_bits)?;
        }
    }

    Ok(LiteralEntropyModelArraySideHeaderBytes {
        path: "plain_small",
        bytes,
        bit_len,
        symbol_index_bits,
        code_len_bits,
        nonzero_symbols,
    })
}

fn model_array_predictor_score(histogram: &[u32; 32], predictor: u8) -> u32 {
    histogram
        .iter()
        .copied()
        .enumerate()
        .map(|(index, count)| {
            if count == 0 {
                0
            } else {
                count * (((index as u32) >> predictor) + u32::from(predictor) + 1)
            }
        })
        .sum()
}

pub(crate) fn write_model_array_plain_large_side_header_preamble(
    plan: LiteralEntropyModelArraySideHeaderPlan,
    header: LiteralEntropyModelArrayPlainLargeHeader,
) -> Result<LiteralEntropyModelArrayPlainLargePreamble, CoreError> {
    if plan.uses_shuffled_ranks {
        return Err(CoreError::Codec(
            "plain-large model-array side header cannot use shuffled ranks".to_string(),
        ));
    }
    if header.symbol_count != plan.symbol_count || header.symbol_count <= 4 {
        return Err(CoreError::Codec(format!(
            "plain-large model-array symbol count mismatch: plan {}, header {}",
            plan.symbol_count, header.symbol_count
        )));
    }
    if header.code_lengths.is_empty() || header.code_lengths.len() > header.alphabet_size {
        return Err(CoreError::Codec(format!(
            "plain-large model-array code-length span is invalid: alphabet {}, span {}",
            header.alphabet_size,
            header.code_lengths.len()
        )));
    }

    let symbol_index_bits = model_array_bit_width(header.alphabet_size);
    let mut predictor_accumulator = i32::from(symbol_index_bits) * 4;
    let mut residual_histogram = [0u32; 32];
    let mut residuals = Vec::new();
    for code_len in header.code_lengths.iter().copied() {
        if code_len == 0 {
            continue;
        }

        let predicted = (predictor_accumulator + 2) >> 2;
        let delta = i32::from(code_len) - predicted;
        let residual = ((delta * 2) ^ (delta >> 31)) as usize;
        let Some(slot) = residual_histogram.get_mut(residual) else {
            return Err(CoreError::Codec(format!(
                "plain-large model-array residual {residual} exceeds reference 0x20 histogram"
            )));
        };
        *slot += 1;
        residuals.push(residual as u8);
        predictor_accumulator = ((predictor_accumulator * 3 + 2) >> 2) + i32::from(code_len);
    }

    let predictor_scores = [
        model_array_predictor_score(&residual_histogram, 0),
        model_array_predictor_score(&residual_histogram, 1),
        model_array_predictor_score(&residual_histogram, 2),
        model_array_predictor_score(&residual_histogram, 3),
    ];
    let selected_predictor = predictor_scores
        .iter()
        .copied()
        .enumerate()
        .min_by_key(|(_, score)| *score)
        .map(|(predictor, _)| predictor as u8)
        .unwrap_or(0);

    let mut bytes = Vec::new();
    let mut bit_len = 0usize;
    append_model_array_bits(
        &mut bytes,
        &mut bit_len,
        u64::from(plan.prefix_value),
        plan.prefix_bits,
    )?;
    append_model_array_bits(&mut bytes, &mut bit_len, 1, 1)?;
    append_model_array_bits(&mut bytes, &mut bit_len, u64::from(selected_predictor), 2)?;
    let first_symbol_has_code_len = header.code_lengths.first().copied().unwrap_or(0) != 0;
    append_model_array_bits(
        &mut bytes,
        &mut bit_len,
        u64::from(first_symbol_has_code_len),
        1,
    )?;

    let nonzero_symbols = residuals.len();
    Ok(LiteralEntropyModelArrayPlainLargePreamble {
        path: "plain_large_preamble",
        bytes,
        bit_len,
        symbol_index_bits,
        selected_predictor,
        predictor_scores,
        residual_histogram,
        residuals,
        nonzero_symbols,
        first_symbol_has_code_len,
    })
}

fn model_array_large_run_descriptor_bits(run_len: usize) -> Result<String, CoreError> {
    if run_len == 0 {
        return Err(CoreError::Codec(
            "plain-large model-array run descriptor requires a non-empty run".to_string(),
        ));
    }
    let encoded_len = ((run_len - 1) >> 1) + 1;
    let width = model_array_bit_width(encoded_len + 1) - 1;
    let remainder = encoded_len - (1usize << width);
    let mut bits = String::new();
    bits.extend(std::iter::repeat_n('0', usize::from(width)));
    bits.push('1');
    if width > 0 {
        bits.push_str(&format!("{remainder:0width$b}", width = usize::from(width)));
    }
    bits.push(if (run_len - 1) & 1 == 0 { '0' } else { '1' });
    Ok(bits)
}

fn model_array_large_residual_bits(residual: u8, predictor: u8) -> String {
    let quotient = residual >> predictor;
    let low_mask = if predictor == 0 {
        0
    } else {
        (1u8 << predictor) - 1
    };
    let low_bits = residual & low_mask;
    let mut bits = String::new();
    bits.extend(std::iter::repeat_n('0', usize::from(quotient)));
    bits.push('1');
    if predictor > 0 {
        bits.push_str(&format!(
            "{low_bits:0width$b}",
            width = usize::from(predictor)
        ));
    }
    bits
}

pub(crate) fn write_model_array_plain_large_side_header_body(
    plan: LiteralEntropyModelArraySideHeaderPlan,
    header: LiteralEntropyModelArrayPlainLargeHeader,
) -> Result<LiteralEntropyModelArrayPlainLargeBody, CoreError> {
    if header.code_lengths.len() != header.alphabet_size {
        return Err(CoreError::Codec(format!(
            "plain-large model-array code-length table mismatch: alphabet {}, code lengths {}",
            header.alphabet_size,
            header.code_lengths.len()
        )));
    }

    let preamble = write_model_array_plain_large_side_header_preamble(plan, header.clone())?;
    let mut bytes = preamble.bytes.clone();
    let mut bit_len = preamble.bit_len;
    let mut runs = Vec::new();
    let mut residual_cursor = 0usize;
    let mut index = 0usize;

    while index < header.code_lengths.len() {
        let is_nonzero = header.code_lengths[index] != 0;
        let run_start = index;
        while index < header.code_lengths.len() && (header.code_lengths[index] != 0) == is_nonzero {
            index += 1;
        }
        let run_len = index - run_start;
        let descriptor_bits = model_array_large_run_descriptor_bits(run_len)?;
        append_model_array_bit_string(&mut bytes, &mut bit_len, &descriptor_bits)?;

        let mut residual_bits = Vec::new();
        if is_nonzero {
            for _ in 0..run_len {
                let residual = *preamble.residuals.get(residual_cursor).ok_or_else(|| {
                    CoreError::Codec(
                        "plain-large model-array residual stream ended early".to_string(),
                    )
                })?;
                residual_cursor += 1;
                let bits = model_array_large_residual_bits(residual, preamble.selected_predictor);
                append_model_array_bit_string(&mut bytes, &mut bit_len, &bits)?;
                residual_bits.push(bits);
            }
        }

        runs.push(LiteralEntropyModelArrayPlainLargeRun {
            kind: if is_nonzero { "nonzero" } else { "zero" },
            len: run_len,
            descriptor_bits,
            residual_bits,
        });
    }

    if residual_cursor != preamble.residuals.len() {
        return Err(CoreError::Codec(
            "plain-large model-array residual stream was not fully consumed".to_string(),
        ));
    }

    Ok(LiteralEntropyModelArrayPlainLargeBody {
        path: "plain_large_body",
        bytes,
        bit_len,
        selected_predictor: preamble.selected_predictor,
        runs,
    })
}

pub(crate) struct NativeLiteralEntropyModelBuilder {
    pub single_symbol_model_cost: f32,
    pub cost_scale: f32,
}

impl LiteralEntropyModelBuilder for NativeLiteralEntropyModelBuilder {
    fn encode_model_array_candidate(
        &mut self,
        literal: &[u8],
        histogram: &[u32; 256],
        baseline_cost: f32,
    ) -> Result<Option<LiteralEntropyModelCandidate>, CoreError> {
        encode_model_array_single_symbol_candidate(
            literal,
            histogram,
            baseline_cost,
            self.single_symbol_model_cost,
            self.cost_scale,
        )
    }
}

pub(crate) struct NativeLiteralEntropyRepeatedPatternBuilder;

impl LiteralEntropyRepeatedPatternBuilder for NativeLiteralEntropyRepeatedPatternBuilder {
    fn encode_repeated_pattern_candidate(
        &mut self,
        literal: &[u8],
        plan: LiteralEntropyRepeatedPatternPlan,
    ) -> Result<Option<LiteralEntropyRepeatedPatternCandidate>, CoreError> {
        let Some(payload) = encode_repeated_byte_pattern_payload(literal, plan.payload_budget)?
        else {
            return Ok(None);
        };
        Ok(Some(LiteralEntropyRepeatedPatternCandidate {
            cost: payload.payload.len() as f32 + plan.pre_cost,
            payload: payload.payload,
        }))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LiteralPacketSpan {
    pub header_nibble: u8,
    pub decoded_bytes: usize,
    pub payload_bytes: usize,
    pub encoded_bytes: usize,
}

pub(crate) fn decode_literal_packet_span(
    packet: &[u8],
    max_decoded_bytes: usize,
) -> Result<LiteralPacketSpan, CoreError> {
    if packet.len() < 2 {
        return Err(CoreError::Codec(
            "literal packet span requires at least two bytes".to_string(),
        ));
    }

    let first = packet[0];
    let header_nibble = first >> 4;
    if first < 0x80 {
        if header_nibble == 0 {
            if packet.len() < 3 {
                return Err(CoreError::Codec(
                    "literal packet span raw header is truncated".to_string(),
                ));
            }
            let decoded_bytes = (usize::from(packet[0]) << 16)
                | (usize::from(packet[1]) << 8)
                | usize::from(packet[2]);
            if decoded_bytes > RAW_LITERAL_PACKET_MAX_BYTES {
                return Err(CoreError::Codec(format!(
                    "literal packet span raw length exceeds 0x3FFFF: {decoded_bytes}"
                )));
            }
            if decoded_bytes > max_decoded_bytes {
                return Err(CoreError::Codec(format!(
                    "literal packet span decoded length {decoded_bytes} exceeds max {max_decoded_bytes}"
                )));
            }
            let encoded_bytes = decoded_bytes
                .checked_add(3)
                .ok_or_else(|| CoreError::Codec("literal packet span overflow".to_string()))?;
            if packet.len() < encoded_bytes {
                return Err(CoreError::Codec(format!(
                    "literal packet span raw payload is truncated: have {}, need {encoded_bytes}",
                    packet.len()
                )));
            }
            return Ok(LiteralPacketSpan {
                header_nibble,
                decoded_bytes,
                payload_bytes: decoded_bytes,
                encoded_bytes,
            });
        }

        if header_nibble > 5 {
            return Err(CoreError::Codec(format!(
                "literal packet span entropy mode {header_nibble} exceeds reference limit"
            )));
        }
        if packet.len() < 5 {
            return Err(CoreError::Codec(
                "literal packet span entropy header is truncated".to_string(),
            ));
        }

        let header = (u64::from(packet[0]) << 32)
            | (u64::from(packet[1]) << 24)
            | (u64::from(packet[2]) << 16)
            | (u64::from(packet[3]) << 8)
            | u64::from(packet[4]);
        let payload_bytes = (header & (ENTROPY_LITERAL_HEADER_UNIT - 1)) as usize;
        let decoded_bytes = (((header >> 18) & (ENTROPY_LITERAL_HEADER_UNIT - 1)) + 1) as usize;
        if decoded_bytes > max_decoded_bytes {
            return Err(CoreError::Codec(format!(
                "literal packet span decoded length {decoded_bytes} exceeds max {max_decoded_bytes}"
            )));
        }
        if decoded_bytes <= payload_bytes {
            return Err(CoreError::Codec(format!(
                "literal packet span decoded length {decoded_bytes} does not exceed payload {payload_bytes}"
            )));
        }
        let encoded_bytes = payload_bytes
            .checked_add(5)
            .ok_or_else(|| CoreError::Codec("literal packet span overflow".to_string()))?;
        if packet.len() < encoded_bytes {
            return Err(CoreError::Codec(format!(
                "literal packet span entropy payload is truncated: have {}, need {encoded_bytes}",
                packet.len()
            )));
        }
        return Ok(LiteralPacketSpan {
            header_nibble,
            decoded_bytes,
            payload_bytes,
            encoded_bytes,
        });
    }

    if (header_nibble & 7) == 0 {
        let decoded_bytes = ((usize::from(packet[0]) << 8) | usize::from(packet[1])) & 0xFFF;
        if decoded_bytes > max_decoded_bytes {
            return Err(CoreError::Codec(format!(
                "literal packet span decoded length {decoded_bytes} exceeds max {max_decoded_bytes}"
            )));
        }
        let encoded_bytes = decoded_bytes
            .checked_add(2)
            .ok_or_else(|| CoreError::Codec("literal packet span overflow".to_string()))?;
        if packet.len() < encoded_bytes {
            return Err(CoreError::Codec(format!(
                "literal packet span compact raw payload is truncated: have {}, need {encoded_bytes}",
                packet.len()
            )));
        }
        return Ok(LiteralPacketSpan {
            header_nibble,
            decoded_bytes,
            payload_bytes: decoded_bytes,
            encoded_bytes,
        });
    }

    if (header_nibble & 7) > 5 {
        return Err(CoreError::Codec(format!(
            "literal packet span compact entropy mode {} exceeds reference limit",
            header_nibble & 7
        )));
    }
    if packet.len() < 4 {
        return Err(CoreError::Codec(
            "literal packet span compact entropy packet is truncated".to_string(),
        ));
    }

    let payload_bytes = ((usize::from(packet[1]) & 0x03) << 8) | usize::from(packet[2]);
    let omitted_bytes = ((usize::from(packet[0]) << 6) | (usize::from(packet[1]) >> 2)) & 0x3FF;
    let decoded_bytes = payload_bytes
        .checked_add(omitted_bytes)
        .and_then(|value| value.checked_add(1))
        .ok_or_else(|| CoreError::Codec("literal packet span overflow".to_string()))?;
    if decoded_bytes > max_decoded_bytes {
        return Err(CoreError::Codec(format!(
            "literal packet span decoded length {decoded_bytes} exceeds max {max_decoded_bytes}"
        )));
    }
    let encoded_bytes = payload_bytes
        .checked_add(3)
        .ok_or_else(|| CoreError::Codec("literal packet span overflow".to_string()))?;
    if packet.len() < encoded_bytes {
        return Err(CoreError::Codec(format!(
            "literal packet span compact entropy payload is truncated: have {}, need {encoded_bytes}",
            packet.len()
        )));
    }
    Ok(LiteralPacketSpan {
        header_nibble,
        decoded_bytes,
        payload_bytes,
        encoded_bytes,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LiteralPacketMode {
    RawFallback,
    WrappedShortRaw,
    WrappedEntropy,
    SingleSymbolDirect,
    SingleSymbolSplit,
    EntropyCandidate,
}

impl LiteralPacketMode {
    fn as_str(self) -> &'static str {
        match self {
            LiteralPacketMode::RawFallback => "raw_fallback",
            LiteralPacketMode::WrappedShortRaw => "wrapped_short_raw",
            LiteralPacketMode::WrappedEntropy => "wrapped_entropy",
            LiteralPacketMode::SingleSymbolDirect => "single_symbol_direct",
            LiteralPacketMode::SingleSymbolSplit => "single_symbol_split",
            LiteralPacketMode::EntropyCandidate => "entropy_candidate",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SingleSymbolPacketMode {
    DirectSymbol,
    SplitFirstSymbol,
}

pub(crate) fn write_short_literal_packet(
    output: &mut Vec<u8>,
    output_capacity: usize,
    literal: &[u8],
) -> Result<LiteralPacketResult, CoreError> {
    if literal.len() > SHORT_LITERAL_RAW_MAX_BYTES {
        return Err(CoreError::Codec(format!(
            "literal length {} requires the entropy literal path from 0x6F91E80",
            literal.len()
        )));
    }

    let encoded_bytes = write_raw_literal_packet(output, output_capacity, literal)?;
    Ok(LiteralPacketResult {
        mode: LiteralPacketMode::RawFallback,
        encoded_bytes,
        cost: literal.len() as f32 + RAW_LITERAL_COST_BASE,
    })
}

pub(crate) fn write_wrapped_literal_packet(
    output: &mut Vec<u8>,
    output_capacity: usize,
    literal: &[u8],
) -> Result<LiteralPacketResult, CoreError> {
    let start = output.len();
    let result = write_short_literal_packet(output, output_capacity, literal)?;
    compact_wrapped_literal_packet(output, start, result)
}

pub(crate) fn write_wrapped_literal_packet_with_entropy_encoder(
    output: &mut Vec<u8>,
    output_capacity: usize,
    literal: &[u8],
    histogram_out: Option<&mut [u32; 256]>,
    encoder: &mut dyn LiteralEntropyPacketEncoder,
) -> Result<LiteralPacketResult, CoreError> {
    if literal.len() <= SHORT_LITERAL_RAW_MAX_BYTES {
        return write_wrapped_literal_packet(output, output_capacity, literal);
    }

    let start = output.len();
    let mut histogram = [0u32; 256];
    build_literal_histogram(literal, &mut histogram, true);
    if let Some(histogram_out) = histogram_out {
        *histogram_out = histogram;
    }

    let result = encoder.encode_literal_packet(output, output_capacity, literal, &histogram)?;
    compact_wrapped_literal_packet(output, start, result)
}

pub(crate) fn compact_wrapped_literal_packet(
    output: &mut Vec<u8>,
    start: usize,
    result: LiteralPacketResult,
) -> Result<LiteralPacketResult, CoreError> {
    if result.encoded_bytes > 0x1004 {
        return Ok(result);
    }

    let end = start
        .checked_add(result.encoded_bytes)
        .ok_or_else(|| CoreError::Codec("wrapped literal packet range overflow".to_string()))?;
    if end > output.len() {
        return Err(CoreError::Codec(format!(
            "wrapped literal packet range exceeds output: end {end}, len {}",
            output.len()
        )));
    }

    let span = decode_literal_packet_span(&output[start..end], KRAKEN_BLOCK_LEN)?;
    if span.header_nibble != 0 || span.decoded_bytes > 0xFFF {
        if output[start] >= 0x80 {
            return Ok(result);
        }
        if span.header_nibble == 0 {
            return Ok(result);
        }

        let payload_bytes = span.payload_bytes;
        let omitted_bytes = span
            .decoded_bytes
            .checked_sub(payload_bytes)
            .and_then(|value| value.checked_sub(1))
            .ok_or_else(|| CoreError::Codec("wrapped entropy span underflow".to_string()))?;
        if payload_bytes > 0x3FF || omitted_bytes > 0x3FF {
            return Ok(result);
        }

        let compact_len = payload_bytes
            .checked_add(3)
            .ok_or_else(|| CoreError::Codec("wrapped entropy packet size overflow".to_string()))?;
        output[start] = ((span.header_nibble | 0x08) << 4) | ((omitted_bytes >> 6) as u8 & 0x0F);
        output[start + 1] =
            (((omitted_bytes & 0x3F) << 2) as u8) | ((payload_bytes >> 8) as u8 & 0x03);
        output[start + 2] = payload_bytes as u8;
        output.copy_within(start + 5..start + span.encoded_bytes, start + 3);
        output.drain(start + compact_len..end);

        return Ok(LiteralPacketResult {
            mode: LiteralPacketMode::WrappedEntropy,
            encoded_bytes: compact_len,
            cost: result.cost - WRAPPED_ENTROPY_COST_DISCOUNT,
        });
    }

    let compact_len = span
        .decoded_bytes
        .checked_add(2)
        .ok_or_else(|| CoreError::Codec("wrapped literal packet size overflow".to_string()))?;
    output[start] = 0x80 | ((span.decoded_bytes >> 8) as u8 & 0x0F);
    output[start + 1] = span.decoded_bytes as u8;
    output.copy_within(start + 3..start + span.encoded_bytes, start + 2);
    output.drain(start + compact_len..end);

    Ok(LiteralPacketResult {
        mode: LiteralPacketMode::WrappedShortRaw,
        encoded_bytes: compact_len,
        cost: result.cost - WRAPPED_SHORT_RAW_COST_DISCOUNT,
    })
}

pub(crate) fn write_entropy_single_symbol_packet(
    output: &mut Vec<u8>,
    output_capacity: usize,
    literal: &[u8],
    mode: SingleSymbolPacketMode,
    model_cost: f32,
    cost_scale: f32,
    current_best_cost: f32,
) -> Result<LiteralPacketResult, CoreError> {
    if literal.is_empty() {
        return Err(CoreError::Codec(
            "single-symbol literal packet requires non-empty input".to_string(),
        ));
    }
    if max_literal_frequency(literal) != literal.len() {
        return Err(CoreError::Codec(
            "single-symbol literal packet requires all bytes to match".to_string(),
        ));
    }

    let (packet_len, mode_id, payload_len, cost_base, result_mode) = match mode {
        SingleSymbolPacketMode::DirectSymbol => (
            6,
            SINGLE_SYMBOL_DIRECT_MODE_ID,
            SINGLE_SYMBOL_DIRECT_PAYLOAD_BYTES,
            SINGLE_SYMBOL_DIRECT_COST_BASE,
            LiteralPacketMode::SingleSymbolDirect,
        ),
        SingleSymbolPacketMode::SplitFirstSymbol => (
            8,
            SINGLE_SYMBOL_SPLIT_MODE_ID,
            SINGLE_SYMBOL_SPLIT_PAYLOAD_BYTES,
            SINGLE_SYMBOL_SPLIT_COST_BASE,
            LiteralPacketMode::SingleSymbolSplit,
        ),
    };
    let cost = model_cost * cost_scale + cost_base;
    if current_best_cost <= cost {
        return Err(CoreError::Codec(format!(
            "single-symbol literal packet cost {cost} is not better than current best {current_best_cost}"
        )));
    }
    let remaining = output_capacity.saturating_sub(output.len());
    if remaining < packet_len {
        return Err(CoreError::Codec(format!(
            "single-symbol literal packet output capacity exceeded: remaining {remaining}, required {packet_len}"
        )));
    }

    let header = entropy_literal_header_bytes(mode_id, literal.len(), payload_len)?;
    output.extend_from_slice(&header);
    match mode {
        SingleSymbolPacketMode::DirectSymbol => output.push(literal[0]),
        SingleSymbolPacketMode::SplitFirstSymbol => {
            output.push(0);
            output.push((literal[0] >> 2) | 0x40);
            output.push(literal[0] << 6);
        }
    }

    Ok(LiteralPacketResult {
        mode: result_mode,
        encoded_bytes: packet_len,
        cost,
    })
}

pub(crate) fn write_entropy_literal_candidate_packet(
    output: &mut Vec<u8>,
    output_capacity: usize,
    literal_len: usize,
    payload: &[u8],
    mode_id: u8,
    candidate_cost: f32,
    raw_fallback_cost: f32,
) -> Result<LiteralPacketResult, CoreError> {
    if candidate_cost >= raw_fallback_cost {
        return Err(CoreError::Codec(format!(
            "entropy literal packet cost {candidate_cost} is not better than raw fallback {raw_fallback_cost}"
        )));
    }

    let encoded_len = payload
        .len()
        .checked_add(5)
        .ok_or_else(|| CoreError::Codec("entropy literal packet size overflow".to_string()))?;
    let remaining = output_capacity.saturating_sub(output.len());
    if remaining < encoded_len {
        return Err(CoreError::Codec(format!(
            "entropy literal packet output capacity exceeded: remaining {remaining}, required {encoded_len}"
        )));
    }

    let header = entropy_literal_header_bytes(mode_id, literal_len, payload.len())?;
    output.extend_from_slice(&header);
    output.extend_from_slice(payload);
    Ok(LiteralPacketResult {
        mode: LiteralPacketMode::EntropyCandidate,
        encoded_bytes: encoded_len,
        cost: candidate_cost,
    })
}

pub(crate) fn write_entropy_model_array_candidate_packet(
    output: &mut Vec<u8>,
    output_capacity: usize,
    literal: &[u8],
    histogram: &[u32; 256],
    current_best_cost: f32,
    builder: &mut dyn LiteralEntropyModelBuilder,
) -> Result<Option<LiteralPacketResult>, CoreError> {
    if literal.len() <= 0x1F {
        return Ok(None);
    }

    let raw_fallback_cost = literal.len() as f32 + RAW_LITERAL_COST_BASE;
    let baseline_cost = raw_fallback_cost.min(current_best_cost);
    let Some(candidate) =
        builder.encode_model_array_candidate(literal, histogram, baseline_cost)?
    else {
        return Ok(None);
    };
    if candidate.payload.len() > literal.len() {
        return Ok(None);
    }

    write_entropy_literal_candidate_packet(
        output,
        output_capacity,
        literal.len(),
        &candidate.payload,
        candidate.mode_id,
        candidate.cost,
        raw_fallback_cost,
    )
    .map(Some)
}

pub(crate) fn write_entropy_table_candidate_packet(
    output: &mut Vec<u8>,
    output_capacity: usize,
    literal: &[u8],
    histogram: &[u32; 256],
    current_best_cost: f32,
    builder: &mut dyn LiteralEntropyTableBuilder,
) -> Result<Option<LiteralPacketResult>, CoreError> {
    if literal.len() < 0x20 {
        return Ok(None);
    }

    let tail_len = literal.len() - 5;
    let mut adjusted_histogram = *histogram;
    for byte in &literal[tail_len..] {
        let count = &mut adjusted_histogram[*byte as usize];
        *count = count.checked_sub(1).ok_or_else(|| {
            CoreError::Codec("entropy table candidate histogram underflow".to_string())
        })?;
    }

    let table_bits = ((tail_len.ilog2() as usize).saturating_sub(2)).clamp(8, 11);
    let table_size = 1usize << table_bits;
    let effective_symbol_count = adjusted_histogram
        .iter()
        .rposition(|count| *count != 0)
        .map_or(0, |index| index + 1);
    let plan = LiteralEntropyTablePlan {
        tail_len,
        table_bits,
        table_size,
        effective_symbol_count,
    };

    let Some(candidate) =
        builder.encode_table_candidate(literal, &adjusted_histogram, plan, current_best_cost)?
    else {
        return Ok(None);
    };
    if candidate.state_count < 2 || candidate.payload.len() > literal.len() {
        return Ok(None);
    }

    let raw_fallback_cost = literal.len() as f32 + RAW_LITERAL_COST_BASE;
    write_entropy_literal_candidate_packet(
        output,
        output_capacity,
        literal.len(),
        &candidate.payload,
        1,
        candidate.cost,
        raw_fallback_cost,
    )
    .map(Some)
}

pub(crate) fn encode_repeated_byte_pattern_payload(
    literal: &[u8],
    payload_capacity: usize,
) -> Result<Option<RepeatedPatternPayload>, CoreError> {
    if literal.is_empty() || payload_capacity <= 4 {
        return Ok(None);
    }

    let mut data = Vec::with_capacity(literal.len().min(payload_capacity));
    let mut control = Vec::new();
    data.push(0);

    let mut literal_start = 0usize;
    let mut scan_pos = 0usize;
    let mut current_symbol = 0u8;
    let search_limit = literal.len().saturating_sub(0x12);

    while scan_pos < search_limit {
        let Some((run_start, run_len)) =
            find_next_repeated_byte_run(literal, scan_pos, search_limit)
        else {
            break;
        };
        scan_pos = run_start + run_len;

        let raw_len = run_start
            .checked_sub(literal_start)
            .ok_or_else(|| CoreError::Codec("repeated-pattern raw span underflow".to_string()))?;
        let required_gap = raw_len
            .checked_add(0x12)
            .ok_or_else(|| CoreError::Codec("repeated-pattern run gap overflow".to_string()))?;
        if repeated_pattern_payload_gap(payload_capacity, data.len(), control.len()) < required_gap
        {
            return Ok(None);
        }

        let symbol = literal[run_start];
        if symbol != current_symbol {
            if run_len < 8 {
                continue;
            }
            prepend_repeated_pattern_control(&mut control, &[1]);
            data.push(symbol);
            current_symbol = symbol;
        }

        data.extend_from_slice(&literal[literal_start..run_start]);
        prepend_repeated_pattern_span_control(&mut control, raw_len, run_len);
        literal_start = scan_pos;
    }

    let trailing_raw_len = literal
        .len()
        .checked_sub(literal_start)
        .ok_or_else(|| CoreError::Codec("repeated-pattern tail underflow".to_string()))?;
    if trailing_raw_len != 0 {
        let required_gap = trailing_raw_len
            .checked_add(0x10)
            .ok_or_else(|| CoreError::Codec("repeated-pattern tail gap overflow".to_string()))?;
        if repeated_pattern_payload_gap(payload_capacity, data.len(), control.len()) < required_gap
        {
            return Ok(None);
        }
    }
    data.extend_from_slice(&literal[literal_start..]);
    prepend_repeated_pattern_final_raw_control(&mut control, trailing_raw_len);

    let data_bytes = data.len() - 1;
    let control_bytes = control.len();
    let payload_len = data
        .len()
        .checked_add(control_bytes)
        .ok_or_else(|| CoreError::Codec("repeated-pattern payload size overflow".to_string()))?;
    if payload_len > payload_capacity {
        return Ok(None);
    }

    data.extend_from_slice(&control);
    Ok(Some(RepeatedPatternPayload {
        payload: data,
        data_bytes,
        control_bytes,
    }))
}

fn repeated_pattern_payload_gap(
    payload_capacity: usize,
    data_len: usize,
    control_len: usize,
) -> usize {
    payload_capacity.saturating_sub(data_len.saturating_add(control_len))
}

fn find_next_repeated_byte_run(
    literal: &[u8],
    mut scan_pos: usize,
    search_limit: usize,
) -> Option<(usize, usize)> {
    while scan_pos < search_limit {
        for offset in 0..16 {
            let start = scan_pos + offset;
            if start >= search_limit {
                return None;
            }
            if literal[start] == literal[start + 1] && literal[start] == literal[start + 2] {
                let symbol = literal[start];
                let mut end = start + 3;
                while end < literal.len() && literal[end] == symbol {
                    end += 1;
                }
                return Some((start, end - start));
            }
        }
        scan_pos += 16;
    }
    None
}

fn prepend_repeated_pattern_control(control: &mut Vec<u8>, token: &[u8]) {
    control.splice(0..0, token.iter().copied());
}

fn prepend_repeated_pattern_compact_pair(control: &mut Vec<u8>, raw_len: usize, run_len: usize) {
    let run_high = (run_len as u8).wrapping_mul(0x10);
    if raw_len < 0x10 {
        let raw_code = 0x0f - raw_len as u8;
        if run_len < 0x10 {
            prepend_repeated_pattern_control(control, &[raw_code | run_high]);
        } else {
            let run_even = (run_len as u8) & 0x1e;
            prepend_repeated_pattern_control(
                control,
                &[
                    run_high.wrapping_sub(run_even.wrapping_mul(8)) | 0x0f,
                    raw_code | run_even.wrapping_mul(8),
                ],
            );
        }
    } else {
        prepend_repeated_pattern_control(control, &[0x1e - raw_len as u8 | run_high, 0]);
    }
}

fn prepend_repeated_pattern_raw_chunks(control: &mut Vec<u8>, raw_len: &mut usize) {
    while *raw_len > 0x3f {
        let chunk = (*raw_len >> 6).min(0x700);
        let code = chunk - 1;
        prepend_repeated_pattern_control(control, &[code as u8, ((code >> 8) as u8) + 2]);
        *raw_len -= chunk << 6;
    }
}

fn prepend_repeated_pattern_span_control(control: &mut Vec<u8>, raw_len: usize, run_len: usize) {
    if raw_len < 0x1f && (run_len < 0x10 || raw_len < 0x10) && run_len < 0x1f {
        prepend_repeated_pattern_compact_pair(control, raw_len, run_len);
        return;
    }

    let mut raw_remaining = raw_len;
    if (0x40..0x4f).contains(&raw_remaining) {
        prepend_repeated_pattern_control(control, &[0]);
        raw_remaining -= 0x0f;
    }
    prepend_repeated_pattern_raw_chunks(control, &mut raw_remaining);

    let run_low = run_len & 0x7f;
    if run_low < 3
        || raw_remaining > 0x1e
        || (run_low > 0x0f && raw_remaining > 0x0f)
        || run_low > 0x1e
    {
        if run_low != 0 || raw_remaining != 0 {
            let value = (run_low << 6) | raw_remaining;
            prepend_repeated_pattern_control(control, &[value as u8, ((value >> 8) as u8) + 0x10]);
        }
    } else {
        prepend_repeated_pattern_compact_pair(control, raw_remaining, run_low);
    }

    let mut run_remaining = run_len;
    while run_remaining > 0x7f {
        let chunk = (run_remaining >> 7).min(0x700);
        let code = chunk - 1;
        prepend_repeated_pattern_control(control, &[code as u8, ((code >> 8) as u8) + 9]);
        run_remaining -= chunk << 7;
    }
}

fn prepend_repeated_pattern_final_raw_control(control: &mut Vec<u8>, raw_len: usize) {
    let mut raw_remaining = raw_len;
    if raw_remaining == 0 {
        return;
    }
    if raw_remaining < 0x40 {
        prepend_repeated_pattern_control(
            control,
            &[raw_remaining as u8, ((raw_remaining >> 8) as u8) + 0x10],
        );
        return;
    }
    if raw_remaining < 0x4f {
        prepend_repeated_pattern_control(control, &[0]);
        raw_remaining -= 0x0f;
        if raw_remaining == 0 {
            return;
        }
        prepend_repeated_pattern_control(
            control,
            &[raw_remaining as u8, ((raw_remaining >> 8) as u8) + 0x10],
        );
        return;
    }

    prepend_repeated_pattern_raw_chunks(control, &mut raw_remaining);
    if raw_remaining != 0 {
        prepend_repeated_pattern_control(
            control,
            &[raw_remaining as u8, ((raw_remaining >> 8) as u8) + 0x10],
        );
    }
}

pub(crate) fn write_entropy_repeated_pattern_candidate_packet(
    output: &mut Vec<u8>,
    output_capacity: usize,
    literal: &[u8],
    current_best_cost: f32,
    pattern_model_cost: f32,
    cost_scale: f32,
    builder: &mut dyn LiteralEntropyRepeatedPatternBuilder,
) -> Result<Option<LiteralPacketResult>, CoreError> {
    let remaining = output_capacity.saturating_sub(output.len());
    if literal.is_empty() || remaining <= 5 {
        return Ok(None);
    }

    let raw_fallback_cost = literal.len() as f32 + RAW_LITERAL_COST_BASE;
    let baseline_cost = raw_fallback_cost.min(current_best_cost);
    let pre_cost = pattern_model_cost * cost_scale + VARLEN_LITERAL_ALTERNATE_COST_BASE;
    let cost_budget = (baseline_cost - pre_cost).floor();
    if cost_budget <= 0.0 {
        return Ok(None);
    }

    let payload_budget = (remaining - 5).min(cost_budget as usize);
    if payload_budget == 0 {
        return Ok(None);
    }

    let plan = LiteralEntropyRepeatedPatternPlan {
        payload_budget,
        baseline_cost,
        pre_cost,
    };
    let Some(candidate) = builder.encode_repeated_pattern_candidate(literal, plan)? else {
        return Ok(None);
    };
    if candidate.payload.len() > payload_budget || candidate.cost >= baseline_cost {
        return Ok(None);
    }

    write_entropy_literal_candidate_packet(
        output,
        output_capacity,
        literal.len(),
        &candidate.payload,
        3,
        candidate.cost,
        raw_fallback_cost,
    )
    .map(Some)
}

fn entropy_literal_header_bytes(
    mode_id: u8,
    literal_len: usize,
    payload_len: usize,
) -> Result<[u8; 5], CoreError> {
    let header = u64::from(mode_id)
        .checked_mul(ENTROPY_LITERAL_HEADER_UNIT)
        .and_then(|value| value.checked_add(literal_len as u64))
        .and_then(|value| value.checked_mul(ENTROPY_LITERAL_HEADER_UNIT))
        .and_then(|value| value.checked_sub(ENTROPY_LITERAL_HEADER_UNIT))
        .and_then(|value| value.checked_add(payload_len as u64))
        .ok_or_else(|| CoreError::Codec("entropy literal packet header overflow".to_string()))?;
    let low = (header as u32).to_be_bytes();
    Ok([(header >> 32) as u8, low[0], low[1], low[2], low[3]])
}

pub(crate) fn build_literal_histogram(
    literal: &[u8],
    histogram: &mut [u32; 256],
    clear_first: bool,
) {
    if clear_first {
        histogram.fill(0);
    }

    let chunk_len = literal.len() & !3;
    for chunk in literal[..chunk_len].chunks_exact(4) {
        histogram[chunk[0] as usize] += 1;
        histogram[chunk[1] as usize] += 1;
        histogram[chunk[2] as usize] += 1;
        histogram[chunk[3] as usize] += 1;
    }
    for byte in &literal[chunk_len..] {
        histogram[*byte as usize] += 1;
    }
}

fn max_literal_frequency(literal: &[u8]) -> usize {
    let mut histogram = [0u32; 256];
    build_literal_histogram(literal, &mut histogram, true);
    histogram.into_iter().max().unwrap_or(0) as usize
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg(test)]
pub enum PrivateEdit {
    ReplaceFString,
}

pub fn codec_status() -> Value {
    let context = KrakenEncoderContext::for_g1r_level(6);
    let emission_layout = EmissionStateLayout::for_capacity(context.encode_scratch_bytes);
    let mut emission_state = EmissionState::new(context.encode_scratch_bytes, KRAKEN_BLOCK_LEN, 0);
    let empty_write_ok = emission_state.write_stream(0, &[]).is_ok();
    let sample_subjob_plan = SubjobPlan::from_raw_len(KRAKEN_BLOCK_LEN * 4, Some(3), 4).ok();
    let mut raw_literal_self_check = Vec::new();
    let raw_literal_self_check_bytes =
        write_raw_literal_packet(&mut raw_literal_self_check, 16, b"G1R").ok();
    let mut short_literal_self_check = Vec::new();
    let short_literal_self_check_result =
        write_short_literal_packet(&mut short_literal_self_check, 16, b"G1R").ok();
    let mut wrapped_short_raw_self_check = Vec::new();
    let wrapped_short_raw_self_check_result =
        write_wrapped_literal_packet(&mut wrapped_short_raw_self_check, 16, b"G1R").ok();
    let single_symbol_literal = vec![0xAB; SHORT_LITERAL_RAW_MAX_BYTES + 1];
    let mut single_symbol_direct_self_check = Vec::new();
    let single_symbol_direct_result = write_entropy_single_symbol_packet(
        &mut single_symbol_direct_self_check,
        16,
        &single_symbol_literal,
        SingleSymbolPacketMode::DirectSymbol,
        4.0,
        2.0,
        100.0,
    )
    .ok();
    let mut wrapped_entropy_self_check = Vec::new();
    let wrapped_entropy_self_check_result = write_entropy_single_symbol_packet(
        &mut wrapped_entropy_self_check,
        16,
        &single_symbol_literal,
        SingleSymbolPacketMode::DirectSymbol,
        4.0,
        2.0,
        100.0,
    )
    .and_then(|result| compact_wrapped_literal_packet(&mut wrapped_entropy_self_check, 0, result))
    .ok();
    struct CodecStatusLiteralEntropyPacketEncoder;

    impl LiteralEntropyPacketEncoder for CodecStatusLiteralEntropyPacketEncoder {
        fn encode_literal_packet(
            &mut self,
            output: &mut Vec<u8>,
            output_capacity: usize,
            literal: &[u8],
            _histogram: &[u32; 256],
        ) -> Result<LiteralPacketResult, CoreError> {
            write_entropy_single_symbol_packet(
                output,
                output_capacity,
                literal,
                SingleSymbolPacketMode::DirectSymbol,
                4.0,
                2.0,
                100.0,
            )
        }
    }

    let mut wrapped_entropy_long_route_self_check = Vec::new();
    let mut wrapped_entropy_long_route_histogram = [0u32; 256];
    let mut wrapped_entropy_long_route_encoder = CodecStatusLiteralEntropyPacketEncoder;
    let wrapped_entropy_long_route_result = write_wrapped_literal_packet_with_entropy_encoder(
        &mut wrapped_entropy_long_route_self_check,
        16,
        &single_symbol_literal,
        Some(&mut wrapped_entropy_long_route_histogram),
        &mut wrapped_entropy_long_route_encoder,
    )
    .ok();
    let mut literal_histogram_self_check = [0u32; 256];
    build_literal_histogram(b"AABA\xFF", &mut literal_histogram_self_check, true);
    let mut single_symbol_split_self_check = Vec::new();
    let single_symbol_split_result = write_entropy_single_symbol_packet(
        &mut single_symbol_split_self_check,
        16,
        &single_symbol_literal,
        SingleSymbolPacketMode::SplitFirstSymbol,
        4.0,
        2.0,
        100.0,
    )
    .ok();
    let entropy_final_header_self_check = entropy_literal_header_bytes(5, 0x41, 0x0A)
        .map(|bytes| bytes_to_upper_hex(&bytes))
        .ok();
    let mut entropy_final_packet_self_check = Vec::new();
    let entropy_final_packet_self_check_result = write_entropy_literal_candidate_packet(
        &mut entropy_final_packet_self_check,
        16,
        0x41,
        &[0xBA, 0xAD],
        5,
        12.0,
        68.0,
    )
    .ok();

    struct CodecStatusLiteralEntropyModelBuilder {
        baseline_cost: Option<f32>,
    }

    impl LiteralEntropyModelBuilder for CodecStatusLiteralEntropyModelBuilder {
        fn encode_model_array_candidate(
            &mut self,
            _literal: &[u8],
            _histogram: &[u32; 256],
            baseline_cost: f32,
        ) -> Result<Option<LiteralEntropyModelCandidate>, CoreError> {
            self.baseline_cost = Some(baseline_cost);
            Ok(Some(LiteralEntropyModelCandidate {
                mode_id: 4,
                payload: vec![0xDE, 0xAD],
                cost: 20.0,
            }))
        }
    }

    let model_array_literal = b"AAAABBBBCCCCDDDDEEEEFFFFGGGGHHHHIIIIJJJJKKKKLLLLMMMMNNNNOOOOPPPP";
    let mut model_array_histogram = [0u32; 256];
    build_literal_histogram(model_array_literal, &mut model_array_histogram, true);
    let mut model_array_self_check = Vec::new();
    let mut model_array_builder = CodecStatusLiteralEntropyModelBuilder {
        baseline_cost: None,
    };
    let model_array_self_check_result = write_entropy_model_array_candidate_packet(
        &mut model_array_self_check,
        16,
        model_array_literal,
        &model_array_histogram,
        1000.0,
        &mut model_array_builder,
    )
    .ok()
    .flatten();
    let model_array_native_single_symbol_literal = vec![0xAB; SHORT_LITERAL_RAW_MAX_BYTES + 1];
    let mut model_array_native_single_symbol_histogram = [0u32; 256];
    build_literal_histogram(
        &model_array_native_single_symbol_literal,
        &mut model_array_native_single_symbol_histogram,
        true,
    );
    let mut model_array_native_single_symbol_self_check = Vec::new();
    let mut model_array_native_single_symbol_builder = NativeLiteralEntropyModelBuilder {
        single_symbol_model_cost: 4.0,
        cost_scale: 2.0,
    };
    let model_array_native_single_symbol_self_check_result =
        write_entropy_model_array_candidate_packet(
            &mut model_array_native_single_symbol_self_check,
            16,
            &model_array_native_single_symbol_literal,
            &model_array_native_single_symbol_histogram,
            100.0,
            &mut model_array_native_single_symbol_builder,
        )
        .ok()
        .flatten();
    let model_array_native_single_symbol_payload_self_check_hex =
        model_array_native_single_symbol_self_check_result.and_then(|result| {
            model_array_native_single_symbol_self_check
                .get(5..result.encoded_bytes)
                .map(bytes_to_upper_hex)
        });
    let model_array_mode_selection_mode4_self_check =
        select_model_array_mode(LiteralEntropyModelArrayCostInputs {
            literal_len: 0x40,
            symbol_count: 6,
            ranked_symbol_count: 7,
            entropy_bits: 0x100,
            flags: MODEL_ARRAY_MODE4_ENABLE_FLAG,
            cost_scale: 2.0,
            model_cost_limit: 100.0,
            current_best_cost: 200.0,
            mode2_model_cost: 50.0,
            mode4_model_cost: 45.0,
        })
        .ok()
        .flatten();
    let model_array_mode_selection_mode2_bias_self_check =
        select_model_array_mode(LiteralEntropyModelArrayCostInputs {
            literal_len: 0x40,
            symbol_count: 6,
            ranked_symbol_count: 7,
            entropy_bits: 0x100,
            flags: MODEL_ARRAY_MODE4_ENABLE_FLAG,
            cost_scale: 2.0,
            model_cost_limit: 100.0,
            current_best_cost: 200.0,
            mode2_model_cost: 50.0,
            mode4_model_cost: 48.0,
        })
        .ok()
        .flatten();
    let model_array_side_header_plain_self_check =
        model_array_side_header_plan(4, 9, MODEL_ARRAY_SHUFFLED_HEADER_FLAG)
            .ok()
            .flatten();
    let model_array_side_header_shuffled_self_check =
        model_array_side_header_plan(5, 9, MODEL_ARRAY_SHUFFLED_HEADER_FLAG)
            .ok()
            .flatten();
    let model_array_plain_small_side_header_self_check = model_array_side_header_plan(3, 3, 0)
        .ok()
        .flatten()
        .and_then(|plan| {
            write_model_array_plain_small_side_header(
                plan,
                LiteralEntropyModelArrayPlainSmallHeader {
                    alphabet_size: 4,
                    symbol_count: 3,
                    single_symbol_index: None,
                    max_code_len: 4,
                    code_lengths: vec![1, 2, 0, 3],
                },
            )
            .ok()
        });
    let model_array_plain_large_side_header_preamble_self_check =
        model_array_side_header_plan(5, 8, 0)
            .ok()
            .flatten()
            .and_then(|plan| {
                write_model_array_plain_large_side_header_preamble(
                    plan,
                    LiteralEntropyModelArrayPlainLargeHeader {
                        alphabet_size: 8,
                        symbol_count: 5,
                        code_lengths: vec![1, 2, 0, 3, 4, 0, 2, 1],
                    },
                )
                .ok()
            });
    let model_array_plain_large_side_header_body_self_check = model_array_side_header_plan(5, 8, 0)
        .ok()
        .flatten()
        .and_then(|plan| {
            write_model_array_plain_large_side_header_body(
                plan,
                LiteralEntropyModelArrayPlainLargeHeader {
                    alphabet_size: 8,
                    symbol_count: 5,
                    code_lengths: vec![1, 2, 0, 3, 4, 0, 2, 1],
                },
            )
            .ok()
        });

    struct CodecStatusLiteralEntropyTableBuilder {
        plan: Option<LiteralEntropyTablePlan>,
    }

    impl LiteralEntropyTableBuilder for CodecStatusLiteralEntropyTableBuilder {
        fn encode_table_candidate(
            &mut self,
            _literal: &[u8],
            _adjusted_histogram: &[u32; 256],
            plan: LiteralEntropyTablePlan,
            _current_best_cost: f32,
        ) -> Result<Option<LiteralEntropyTableCandidate>, CoreError> {
            self.plan = Some(plan);
            Ok(Some(LiteralEntropyTableCandidate {
                state_count: 2,
                payload: vec![0xFE, 0xED, 0xFA],
                cost: 21.0,
            }))
        }
    }

    let mut table_candidate_literal = vec![b'A'; 59];
    table_candidate_literal.extend_from_slice(b"BCDEF");
    let mut table_candidate_histogram = [0u32; 256];
    build_literal_histogram(
        &table_candidate_literal,
        &mut table_candidate_histogram,
        true,
    );
    let mut table_candidate_self_check = Vec::new();
    let mut table_candidate_builder = CodecStatusLiteralEntropyTableBuilder { plan: None };
    let table_candidate_self_check_result = write_entropy_table_candidate_packet(
        &mut table_candidate_self_check,
        16,
        &table_candidate_literal,
        &table_candidate_histogram,
        1000.0,
        &mut table_candidate_builder,
    )
    .ok()
    .flatten();

    struct CodecStatusLiteralEntropyRepeatedPatternBuilder {
        plan: Option<LiteralEntropyRepeatedPatternPlan>,
    }

    impl LiteralEntropyRepeatedPatternBuilder for CodecStatusLiteralEntropyRepeatedPatternBuilder {
        fn encode_repeated_pattern_candidate(
            &mut self,
            _literal: &[u8],
            plan: LiteralEntropyRepeatedPatternPlan,
        ) -> Result<Option<LiteralEntropyRepeatedPatternCandidate>, CoreError> {
            self.plan = Some(plan);
            Ok(Some(LiteralEntropyRepeatedPatternCandidate {
                payload: vec![0xFA, 0xCE, 0x01],
                cost: 20.0,
            }))
        }
    }

    let repeated_pattern_literal = vec![b'A'; 0x40];
    let mut repeated_pattern_self_check = Vec::new();
    let mut repeated_pattern_builder =
        CodecStatusLiteralEntropyRepeatedPatternBuilder { plan: None };
    let repeated_pattern_self_check_result = write_entropy_repeated_pattern_candidate_packet(
        &mut repeated_pattern_self_check,
        16,
        &repeated_pattern_literal,
        100.0,
        4.0,
        2.0,
        &mut repeated_pattern_builder,
    )
    .ok()
    .flatten();

    let repeated_pattern_native_literal = vec![b'A'; 0x20];
    let mut repeated_pattern_native_self_check = Vec::new();
    let mut repeated_pattern_native_builder = NativeLiteralEntropyRepeatedPatternBuilder;
    let repeated_pattern_native_self_check_result =
        write_entropy_repeated_pattern_candidate_packet(
            &mut repeated_pattern_native_self_check,
            24,
            &repeated_pattern_native_literal,
            100.0,
            4.0,
            2.0,
            &mut repeated_pattern_native_builder,
        )
        .ok()
        .flatten();
    let repeated_pattern_native_payload_self_check_hex = repeated_pattern_native_self_check_result
        .and_then(|result| {
            repeated_pattern_native_self_check
                .get(5..result.encoded_bytes)
                .map(bytes_to_upper_hex)
        });
    let mut repeated_pattern_search_boundary_literal = b"abcdefghijklmn".to_vec();
    repeated_pattern_search_boundary_literal.extend(std::iter::repeat_n(b'A', 0x12));
    let repeated_pattern_search_boundary_self_check =
        encode_repeated_byte_pattern_payload(&repeated_pattern_search_boundary_literal, 0x40)
            .ok()
            .flatten();
    let repeated_pattern_run_gap_too_tight_rejected =
        encode_repeated_byte_pattern_payload(&repeated_pattern_native_literal, 5)
            .ok()
            .flatten()
            .is_none();
    let repeated_pattern_tail_gap_literal = b"abcdefghijklmnopqrstuvwxyzABCDEF".to_vec();
    let repeated_pattern_tail_gap_too_tight_rejected =
        encode_repeated_byte_pattern_payload(&repeated_pattern_tail_gap_literal, 0x23)
            .ok()
            .flatten()
            .is_none();
    let mut repeated_pattern_raw_span_literal = b"xyz".to_vec();
    repeated_pattern_raw_span_literal.extend(std::iter::repeat_n(b'A', 0x20));
    let mut repeated_pattern_raw_span_self_check = Vec::new();
    let mut repeated_pattern_raw_span_builder = NativeLiteralEntropyRepeatedPatternBuilder;
    let repeated_pattern_raw_span_self_check_result =
        write_entropy_repeated_pattern_candidate_packet(
            &mut repeated_pattern_raw_span_self_check,
            32,
            &repeated_pattern_raw_span_literal,
            100.0,
            4.0,
            2.0,
            &mut repeated_pattern_raw_span_builder,
        )
        .ok()
        .flatten();
    let repeated_pattern_raw_span_payload_self_check_hex =
        repeated_pattern_raw_span_self_check_result.and_then(|result| {
            repeated_pattern_raw_span_self_check
                .get(5..result.encoded_bytes)
                .map(bytes_to_upper_hex)
        });
    let mut repeated_pattern_symbol_reuse_literal = vec![b'A'; 0x20];
    repeated_pattern_symbol_reuse_literal.extend_from_slice(b"bc");
    repeated_pattern_symbol_reuse_literal.extend(std::iter::repeat_n(b'A', 0x20));
    let mut repeated_pattern_symbol_reuse_self_check = Vec::new();
    let mut repeated_pattern_symbol_reuse_builder = NativeLiteralEntropyRepeatedPatternBuilder;
    let repeated_pattern_symbol_reuse_self_check_result =
        write_entropy_repeated_pattern_candidate_packet(
            &mut repeated_pattern_symbol_reuse_self_check,
            40,
            &repeated_pattern_symbol_reuse_literal,
            100.0,
            4.0,
            2.0,
            &mut repeated_pattern_symbol_reuse_builder,
        )
        .ok()
        .flatten();
    let repeated_pattern_symbol_reuse_payload_self_check_hex =
        repeated_pattern_symbol_reuse_self_check_result.and_then(|result| {
            repeated_pattern_symbol_reuse_self_check
                .get(5..result.encoded_bytes)
                .map(bytes_to_upper_hex)
        });
    let mut repeated_pattern_compact_pair_literal = vec![b'A'; 8];
    repeated_pattern_compact_pair_literal.extend_from_slice(b"bc");
    repeated_pattern_compact_pair_literal.extend(std::iter::repeat_n(b'A', 3));
    repeated_pattern_compact_pair_literal.extend(std::iter::repeat_n(b'B', 22));
    let mut repeated_pattern_compact_pair_self_check = Vec::new();
    let mut repeated_pattern_compact_pair_builder = NativeLiteralEntropyRepeatedPatternBuilder;
    let repeated_pattern_compact_pair_self_check_result =
        write_entropy_repeated_pattern_candidate_packet(
            &mut repeated_pattern_compact_pair_self_check,
            40,
            &repeated_pattern_compact_pair_literal,
            100.0,
            4.0,
            2.0,
            &mut repeated_pattern_compact_pair_builder,
        )
        .ok()
        .flatten();
    let repeated_pattern_compact_pair_payload_self_check_hex =
        repeated_pattern_compact_pair_self_check_result.and_then(|result| {
            repeated_pattern_compact_pair_self_check
                .get(5..result.encoded_bytes)
                .map(bytes_to_upper_hex)
        });
    let repeated_pattern_continuation_payload_capacity = 0x100 - 5;
    let mut repeated_pattern_continuation_literal: Vec<u8> =
        (0..0x80).map(|index| b'a' + (index % 26) as u8).collect();
    repeated_pattern_continuation_literal.extend(std::iter::repeat_n(b'Z', 0x100));
    let repeated_pattern_continuation_payload_self_check = encode_repeated_byte_pattern_payload(
        &repeated_pattern_continuation_literal,
        repeated_pattern_continuation_payload_capacity,
    )
    .ok()
    .flatten();
    let repeated_pattern_continuation_control_self_check_hex =
        repeated_pattern_continuation_payload_self_check
            .as_ref()
            .map(|payload| {
                let control_start = 1 + payload.data_bytes;
                bytes_to_upper_hex(&payload.payload[control_start..])
            });
    let mut repeated_pattern_continuation_self_check = Vec::new();
    let mut repeated_pattern_continuation_builder = NativeLiteralEntropyRepeatedPatternBuilder;
    let repeated_pattern_continuation_self_check_result =
        write_entropy_repeated_pattern_candidate_packet(
            &mut repeated_pattern_continuation_self_check,
            0x100,
            &repeated_pattern_continuation_literal,
            1000.0,
            4.0,
            2.0,
            &mut repeated_pattern_continuation_builder,
        )
        .ok()
        .flatten();
    let repeated_pattern_continuation_self_check_header_hex =
        repeated_pattern_continuation_self_check_result.and_then(|result| {
            repeated_pattern_continuation_self_check
                .get(..5.min(result.encoded_bytes))
                .map(bytes_to_upper_hex)
        });
    let repeated_pattern_multi_chunk_continuation_raw_len = 0x1C040usize;
    let repeated_pattern_multi_chunk_continuation_output_capacity = 0x1C060usize;
    let mut repeated_pattern_multi_chunk_continuation_literal: Vec<u8> = (0
        ..repeated_pattern_multi_chunk_continuation_raw_len)
        .map(|index| b'a' + (index % 26) as u8)
        .collect();
    repeated_pattern_multi_chunk_continuation_literal.extend(std::iter::repeat_n(b'Z', 0x100));
    let repeated_pattern_multi_chunk_continuation_payload_self_check =
        encode_repeated_byte_pattern_payload(
            &repeated_pattern_multi_chunk_continuation_literal,
            repeated_pattern_multi_chunk_continuation_output_capacity - 5,
        )
        .ok()
        .flatten();
    let repeated_pattern_multi_chunk_continuation_control_self_check_hex =
        repeated_pattern_multi_chunk_continuation_payload_self_check
            .as_ref()
            .map(|payload| {
                let control_start = 1 + payload.data_bytes;
                bytes_to_upper_hex(&payload.payload[control_start..])
            });
    let mut repeated_pattern_multi_chunk_continuation_self_check = Vec::new();
    let mut repeated_pattern_multi_chunk_continuation_builder =
        NativeLiteralEntropyRepeatedPatternBuilder;
    let repeated_pattern_multi_chunk_continuation_self_check_result =
        write_entropy_repeated_pattern_candidate_packet(
            &mut repeated_pattern_multi_chunk_continuation_self_check,
            repeated_pattern_multi_chunk_continuation_output_capacity,
            &repeated_pattern_multi_chunk_continuation_literal,
            200_000.0,
            4.0,
            2.0,
            &mut repeated_pattern_multi_chunk_continuation_builder,
        )
        .ok()
        .flatten();
    let repeated_pattern_multi_chunk_continuation_self_check_header_hex =
        repeated_pattern_multi_chunk_continuation_self_check_result.and_then(|result| {
            repeated_pattern_multi_chunk_continuation_self_check
                .get(..5.min(result.encoded_bytes))
                .map(bytes_to_upper_hex)
        });
    let repeated_pattern_optional_substream_self_check = repeated_pattern_optional_substream_plan(
        REPEATED_PATTERN_OPTIONAL_SUBSTREAM_FLAG,
        0x21,
        0x0F,
        true,
    )
    .ok()
    .flatten();
    let repeated_pattern_optional_substream_disabled_without_flag =
        repeated_pattern_optional_substream_plan(0, 0x21, 0x0F, true)
            .ok()
            .flatten()
            .is_none();
    let repeated_pattern_optional_substream_too_short_rejected =
        repeated_pattern_optional_substream_plan(
            REPEATED_PATTERN_OPTIONAL_SUBSTREAM_FLAG,
            0x1F,
            0,
            true,
        )
        .ok()
        .flatten()
        .is_none();
    let repeated_pattern_optional_substream_too_large_rejected =
        repeated_pattern_optional_substream_plan(
            REPEATED_PATTERN_OPTIONAL_SUBSTREAM_FLAG,
            REPEATED_PATTERN_OPTIONAL_SUBSTREAM_MAX_COMBINED_BYTES - 1,
            1,
            true,
        )
        .ok()
        .flatten()
        .is_none();
    let repeated_pattern_optional_substream_plan_status = json!({
        "sourceFlag": REPEATED_PATTERN_OPTIONAL_SUBSTREAM_FLAG,
        "minDataBytes": REPEATED_PATTERN_OPTIONAL_SUBSTREAM_MIN_DATA_BYTES,
        "maxCombinedBytesExclusive": REPEATED_PATTERN_OPTIONAL_SUBSTREAM_MAX_COMBINED_BYTES,
        "selfCheckEnabled": repeated_pattern_optional_substream_self_check.is_some(),
        "selfCheckDataBytes": repeated_pattern_optional_substream_self_check.map(|plan| plan.data_bytes),
        "selfCheckControlBytes": repeated_pattern_optional_substream_self_check.map(|plan| plan.control_bytes),
        "selfCheckCombinedBytes": repeated_pattern_optional_substream_self_check.map(|plan| plan.combined_bytes),
        "selfCheckAlignedDataBytes": repeated_pattern_optional_substream_self_check.map(|plan| plan.aligned_data_bytes),
        "selfCheckArenaHeaderBytes": repeated_pattern_optional_substream_self_check.map(|plan| plan.arena_header_bytes),
        "selfCheckScratchBytes": repeated_pattern_optional_substream_self_check.map(|plan| plan.scratch_bytes),
        "selfCheckBaselineCost": repeated_pattern_optional_substream_self_check.map(|plan| plan.baseline_cost),
        "disabledWithoutFlag": repeated_pattern_optional_substream_disabled_without_flag,
        "tooShortRejected": repeated_pattern_optional_substream_too_short_rejected,
        "tooLargeRejected": repeated_pattern_optional_substream_too_large_rejected
    });
    let repeated_pattern_optional_substream_single_symbol_self_check =
        encode_repeated_pattern_optional_single_symbol_substream(
            &[0xAB; 0x21],
            &[0xAA, 0x55],
            REPEATED_PATTERN_OPTIONAL_SUBSTREAM_FLAG,
            4.0,
            2.0,
            true,
        )
        .ok()
        .flatten();
    let repeated_pattern_optional_substream_single_symbol_self_check_hex =
        repeated_pattern_optional_substream_single_symbol_self_check
            .as_ref()
            .map(|candidate| bytes_to_upper_hex(&candidate.payload));

    #[derive(Default)]
    struct CodecStatusOptionalSubstreamModelBuilder {
        calls: Vec<(usize, u32, u32, f32)>,
    }

    impl LiteralEntropyModelBuilder for CodecStatusOptionalSubstreamModelBuilder {
        fn encode_model_array_candidate(
            &mut self,
            literal: &[u8],
            histogram: &[u32; 256],
            baseline_cost: f32,
        ) -> Result<Option<LiteralEntropyModelCandidate>, CoreError> {
            self.calls.push((
                literal.len(),
                histogram[0xAB],
                histogram[0xCD],
                baseline_cost,
            ));
            Ok(Some(LiteralEntropyModelCandidate {
                mode_id: 4,
                payload: vec![0xDE, 0xAD],
                cost: 18.0,
            }))
        }
    }

    let mut repeated_pattern_optional_substream_model_array_data = vec![0xAB; 0x10];
    repeated_pattern_optional_substream_model_array_data.extend(std::iter::repeat_n(0xCD, 0x11));
    let mut repeated_pattern_optional_substream_model_array_builder =
        CodecStatusOptionalSubstreamModelBuilder::default();
    let repeated_pattern_optional_substream_model_array_self_check =
        encode_repeated_pattern_optional_model_array_substream(
            &repeated_pattern_optional_substream_model_array_data,
            &[0xAA, 0x55],
            REPEATED_PATTERN_OPTIONAL_SUBSTREAM_FLAG,
            true,
            &mut repeated_pattern_optional_substream_model_array_builder,
        )
        .ok()
        .flatten();
    let repeated_pattern_optional_substream_model_array_self_check_hex =
        repeated_pattern_optional_substream_model_array_self_check
            .as_ref()
            .map(|candidate| bytes_to_upper_hex(&candidate.payload));
    let repeated_pattern_optional_substream_model_array_builder_calls =
        repeated_pattern_optional_substream_model_array_builder
            .calls
            .iter()
            .map(|(literal_len, ab_count, cd_count, baseline_cost)| {
                json!([literal_len, ab_count, cd_count, baseline_cost])
            })
            .collect::<Vec<_>>();

    #[derive(Default)]
    struct CodecStatusVarLenLiteralEntropyBuilder {
        short_result: Option<VarLenLiteralEntropyCandidate>,
        long_result: Option<VarLenLiteralEntropyCandidate>,
        alternate_result: Option<VarLenLiteralEntropyCandidate>,
        alternate_calls: Vec<(usize, f32)>,
    }

    impl VarLenLiteralEntropyBuilder for CodecStatusVarLenLiteralEntropyBuilder {
        fn encode_short_range(
            &mut self,
            _literal_len: usize,
        ) -> Result<Option<VarLenLiteralEntropyCandidate>, CoreError> {
            Ok(self.short_result)
        }

        fn encode_long_range(
            &mut self,
            _literal_len: usize,
        ) -> Result<Option<VarLenLiteralEntropyCandidate>, CoreError> {
            Ok(self.long_result)
        }

        fn encode_alternate(
            &mut self,
            literal_len: usize,
            baseline_cost_without_base: f32,
        ) -> Result<Option<VarLenLiteralEntropyCandidate>, CoreError> {
            self.alternate_calls
                .push((literal_len, baseline_cost_without_base));
            Ok(self.alternate_result)
        }
    }

    let mut varlen_dispatch_builder = CodecStatusVarLenLiteralEntropyBuilder {
        short_result: Some(VarLenLiteralEntropyCandidate {
            encoded_bytes: 17,
            cost: 40.0,
        }),
        alternate_result: Some(VarLenLiteralEntropyCandidate {
            encoded_bytes: 9,
            cost: 12.0,
        }),
        ..CodecStatusVarLenLiteralEntropyBuilder::default()
    };
    let mut varlen_dispatch_cost = 1000.0;
    let varlen_dispatch_self_check = dispatch_varlen_literal_entropy(
        0x100,
        VARLEN_LITERAL_ALTERNATE_FLAG,
        35.0,
        &mut varlen_dispatch_cost,
        &mut varlen_dispatch_builder,
    )
    .ok()
    .flatten();
    let varlen_dispatch_baseline = varlen_dispatch_builder
        .alternate_calls
        .first()
        .map(|(_, baseline_cost)| *baseline_cost);
    let short_varlen_split_plan = short_varlen_literal_split_plan(0x5FF).ok();
    let long_varlen_segment_plan =
        long_varlen_literal_initial_segment_plan(KRAKEN_BLOCK_LEN, 9).ok();
    let alternate_varlen_context_plan =
        alternate_varlen_literal_context_plan(&[0x20, 0x40], 6).ok();
    let varlen_dispatch_status = json!({
        "sourceRva": "0x6F9F580",
        "minBytes": VARLEN_LITERAL_MIN_BYTES,
        "longThresholdBytes": VARLEN_LITERAL_LONG_THRESHOLD_BYTES,
        "alternateFlag": VARLEN_LITERAL_ALTERNATE_FLAG,
        "alternateCostBase": VARLEN_LITERAL_ALTERNATE_COST_BASE,
        "selfCheckKind": varlen_dispatch_self_check.map(|result| result.kind.as_str()),
        "selfCheckEncodedBytes": varlen_dispatch_self_check.map(|result| result.encoded_bytes),
        "selfCheckCost": varlen_dispatch_self_check.map(|result| result.cost),
        "selfCheckBuilderBaseline": varlen_dispatch_baseline
    });
    let short_varlen_split_status = json!({
        "sourceRva": "0x6F9F760",
        "packetPrefix": short_varlen_split_plan.as_ref().map(|plan| plan.packet_prefix),
        "minSideBytes": short_varlen_split_plan.as_ref().map(|plan| plan.min_side_bytes),
        "selfCheckLiteralLen": 0x5FF,
        "selfCheckProbes": short_varlen_split_plan.as_ref().map(|plan| plan.probes.clone())
    });
    let long_varlen_segment_status = json!({
        "sourceRva": "0x6F9FE00",
        "selfCheckLiteralLen": KRAKEN_BLOCK_LEN,
        "selfCheckParam7": 9,
        "segmentCount": long_varlen_segment_plan.as_ref().map(|plan| plan.segment_count),
        "histogramScratchBytes": long_varlen_segment_plan.as_ref().map(|plan| plan.histogram_scratch_bytes),
        "mergeRecordBytes": long_varlen_segment_plan.as_ref().map(|plan| plan.merge_record_bytes),
        "firstSegmentLength": long_varlen_segment_plan.as_ref().and_then(|plan| plan.segment_lengths.first().copied()),
        "lastSegmentOffset": long_varlen_segment_plan.as_ref().and_then(|plan| plan.segment_offsets.last().copied()),
        "lastSegmentLength": long_varlen_segment_plan.as_ref().and_then(|plan| plan.segment_lengths.last().copied())
    });
    let alternate_varlen_context_status = json!({
        "sourceRva": "0x6FA13C0",
        "selfCheckSegmentLengths": [0x20, 0x40],
        "selfCheckParam11": 6,
        "enabled": alternate_varlen_context_plan.as_ref().map(|plan| plan.enabled),
        "totalLen": alternate_varlen_context_plan.as_ref().map(|plan| plan.total_len),
        "maxSegmentLen": alternate_varlen_context_plan.as_ref().map(|plan| plan.max_segment_len),
        "cap": alternate_varlen_context_plan.as_ref().map(|plan| plan.cap),
        "doubleCap": alternate_varlen_context_plan.as_ref().map(|plan| plan.double_cap),
        "windowFloor": alternate_varlen_context_plan.as_ref().map(|plan| plan.window_floor),
        "primaryScratchBytes": alternate_varlen_context_plan.as_ref().map(|plan| plan.primary_scratch_bytes),
        "histogramScratchBytes": alternate_varlen_context_plan.as_ref().map(|plan| plan.histogram_scratch_bytes)
    });

    let mut inline_job_ran = false;
    let mut inline_job_handle = Some(JobHandle(0));
    let inline_job_mode =
        dispatch_kraken_job(&mut inline_job_handle, false, None, None, None, || {
            inline_job_ran = true;
            Ok(())
        })
        .ok();
    let mut packet_writer = json!({
        "sourceRva": "0x6F92A40",
        "rawLiteralMaxBytes": RAW_LITERAL_PACKET_MAX_BYTES,
        "rawLiteralSelfCheckBytes": raw_literal_self_check_bytes,
        "rawLiteralSelfCheckHex": raw_literal_self_check.iter().map(|byte| format!("{byte:02X}")).collect::<Vec<_>>().join(""),
        "shortLiteralSourceRva": "0x6F91D00",
        "shortLiteralMaxBytes": SHORT_LITERAL_RAW_MAX_BYTES,
        "shortLiteralCostBase": RAW_LITERAL_COST_BASE,
        "shortLiteralSelfCheckBytes": short_literal_self_check_result.map(|result| result.encoded_bytes),
        "shortLiteralSelfCheckCost": short_literal_self_check_result.map(|result| result.cost),
        "shortLiteralSelfCheckMode": short_literal_self_check_result.map(|result| result.mode.as_str()),
        "shortLiteralSelfCheckHex": short_literal_self_check.iter().map(|byte| format!("{byte:02X}")).collect::<Vec<_>>().join(""),
        "wrappedLiteralSourceRva": "0x6F927C0",
        "wrappedShortRawCostDiscount": WRAPPED_SHORT_RAW_COST_DISCOUNT,
        "wrappedShortRawSelfCheckBytes": wrapped_short_raw_self_check_result.map(|result| result.encoded_bytes),
        "wrappedShortRawSelfCheckCost": wrapped_short_raw_self_check_result.map(|result| result.cost),
        "wrappedShortRawSelfCheckMode": wrapped_short_raw_self_check_result.map(|result| result.mode.as_str()),
        "wrappedShortRawSelfCheckHex": bytes_to_upper_hex(&wrapped_short_raw_self_check),
        "singleSymbolSourceRva": "0x6F91E80",
        "singleSymbolDirectCostBase": SINGLE_SYMBOL_DIRECT_COST_BASE,
        "singleSymbolSplitCostBase": SINGLE_SYMBOL_SPLIT_COST_BASE,
        "singleSymbolDirectSelfCheckBytes": single_symbol_direct_result.map(|result| result.encoded_bytes),
        "singleSymbolDirectSelfCheckCost": single_symbol_direct_result.map(|result| result.cost),
        "singleSymbolDirectSelfCheckMode": single_symbol_direct_result.map(|result| result.mode.as_str()),
        "singleSymbolDirectSelfCheckHex": single_symbol_direct_self_check.iter().map(|byte| format!("{byte:02X}")).collect::<Vec<_>>().join(""),
        "singleSymbolSplitSelfCheckBytes": single_symbol_split_result.map(|result| result.encoded_bytes),
        "singleSymbolSplitSelfCheckCost": single_symbol_split_result.map(|result| result.cost),
        "singleSymbolSplitSelfCheckMode": single_symbol_split_result.map(|result| result.mode.as_str()),
        "singleSymbolSplitSelfCheckHex": single_symbol_split_self_check.iter().map(|byte| format!("{byte:02X}")).collect::<Vec<_>>().join(""),
        "entropyFinalHeaderSourceRva": "0x6F92609",
        "entropyFinalHeaderUnit": ENTROPY_LITERAL_HEADER_UNIT,
        "entropyFinalHeaderSelfCheckHex": entropy_final_header_self_check,
        "entropyFinalPacketSelfCheckBytes": entropy_final_packet_self_check_result.map(|result| result.encoded_bytes),
        "entropyFinalPacketSelfCheckCost": entropy_final_packet_self_check_result.map(|result| result.cost),
        "entropyFinalPacketSelfCheckMode": entropy_final_packet_self_check_result.map(|result| result.mode.as_str()),
        "entropyFinalPacketSelfCheckHex": bytes_to_upper_hex(&entropy_final_packet_self_check),
        "varLenLiteralDispatch": varlen_dispatch_status,
        "shortVarLenLiteralSplitPlan": short_varlen_split_status,
        "longVarLenLiteralInitialSegmentPlan": long_varlen_segment_status,
        "alternateVarLenLiteralContextPlan": alternate_varlen_context_status
    });
    packet_writer["wrappedEntropyCostDiscount"] = json!(WRAPPED_ENTROPY_COST_DISCOUNT);
    packet_writer["wrappedEntropySelfCheckBytes"] =
        json!(wrapped_entropy_self_check_result.map(|result| result.encoded_bytes));
    packet_writer["wrappedEntropySelfCheckCost"] =
        json!(wrapped_entropy_self_check_result.map(|result| result.cost));
    packet_writer["wrappedEntropySelfCheckMode"] =
        json!(wrapped_entropy_self_check_result.map(|result| result.mode.as_str()));
    packet_writer["wrappedEntropySelfCheckHex"] =
        json!(bytes_to_upper_hex(&wrapped_entropy_self_check));
    packet_writer["wrappedEntropyLongRouteSelfCheckBytes"] =
        json!(wrapped_entropy_long_route_result.map(|result| result.encoded_bytes));
    packet_writer["wrappedEntropyLongRouteSelfCheckCost"] =
        json!(wrapped_entropy_long_route_result.map(|result| result.cost));
    packet_writer["wrappedEntropyLongRouteSelfCheckMode"] =
        json!(wrapped_entropy_long_route_result.map(|result| result.mode.as_str()));
    packet_writer["wrappedEntropyLongRouteSelfCheckHex"] =
        json!(bytes_to_upper_hex(&wrapped_entropy_long_route_self_check));
    packet_writer["wrappedEntropyLongRouteHistogramAB"] =
        json!(wrapped_entropy_long_route_histogram[0xAB]);
    packet_writer["literalHistogramSourceRva"] = json!("0x6F8FA20");
    packet_writer["literalHistogramSelfCheckCounts"] = json!({
        "A": literal_histogram_self_check[b'A' as usize],
        "B": literal_histogram_self_check[b'B' as usize],
        "ff": literal_histogram_self_check[0xFF],
        "C": literal_histogram_self_check[b'C' as usize]
    });
    packet_writer["modelArraySourceRva"] = json!("0x6FB4100");
    packet_writer["modelArraySelfCheckBaseline"] = json!(model_array_builder.baseline_cost);
    packet_writer["modelArraySelfCheckBytes"] =
        json!(model_array_self_check_result.map(|result| result.encoded_bytes));
    packet_writer["modelArraySelfCheckCost"] =
        json!(model_array_self_check_result.map(|result| result.cost));
    packet_writer["modelArraySelfCheckMode"] =
        json!(model_array_self_check_result.map(|result| result.mode.as_str()));
    packet_writer["modelArraySelfCheckHex"] = json!(bytes_to_upper_hex(&model_array_self_check));
    packet_writer["modelArrayNativeSingleSymbolPayloadSelfCheckHex"] =
        json!(model_array_native_single_symbol_payload_self_check_hex);
    packet_writer["modelArrayNativeSingleSymbolSelfCheckBytes"] = json!(
        model_array_native_single_symbol_self_check_result.map(|result| result.encoded_bytes)
    );
    packet_writer["modelArrayNativeSingleSymbolSelfCheckCost"] =
        json!(model_array_native_single_symbol_self_check_result.map(|result| result.cost));
    packet_writer["modelArrayNativeSingleSymbolSelfCheckMode"] = json!(
        model_array_native_single_symbol_self_check_result.map(|result| result.mode.as_str())
    );
    packet_writer["modelArrayNativeSingleSymbolSelfCheckHex"] = json!(bytes_to_upper_hex(
        &model_array_native_single_symbol_self_check
    ));
    packet_writer["modelArrayModeSelectionMode4SelfCheck"] = json!({
        "sourceRva": "0x6FB4100",
        "mode4EnableFlag": MODEL_ARRAY_MODE4_ENABLE_FLAG,
        "mode4SelectionBias": MODEL_ARRAY_MODE4_SELECTION_BIAS,
        "selectedMode": model_array_mode_selection_mode4_self_check.map(|selection| selection.mode_id),
        "symbolCount": model_array_mode_selection_mode4_self_check.map(|selection| selection.symbol_count),
        "rankedSymbolCount": model_array_mode_selection_mode4_self_check.map(|selection| selection.ranked_symbol_count),
        "shuffledPairCount": model_array_mode_selection_mode4_self_check.map(|selection| selection.shuffled_pair_count),
        "entropyBits": model_array_mode_selection_mode4_self_check.map(|selection| selection.entropy_bits),
        "entropyBitsPerByte": model_array_mode_selection_mode4_self_check.map(|selection| selection.entropy_bits_per_byte),
        "sideHeaderBytes": model_array_mode_selection_mode4_self_check.map(|selection| selection.side_header_bytes),
        "entropyBytesWithHeader": model_array_mode_selection_mode4_self_check.map(|selection| selection.entropy_bytes_with_header),
        "scaledCostDeltaWithBias": model_array_mode_selection_mode4_self_check.and_then(|selection| selection.scaled_cost_delta_with_bias),
        "selectedModelCost": model_array_mode_selection_mode4_self_check.map(|selection| selection.selected_model_cost),
        "finalCost": model_array_mode_selection_mode4_self_check.map(|selection| selection.final_cost),
        "projectedCost": model_array_mode_selection_mode4_self_check.map(|selection| selection.projected_cost)
    });
    packet_writer["modelArrayModeSelectionMode2BiasSelfCheck"] = json!({
        "sourceRva": "0x6FB4100",
        "mode4EnableFlag": MODEL_ARRAY_MODE4_ENABLE_FLAG,
        "mode4SelectionBias": MODEL_ARRAY_MODE4_SELECTION_BIAS,
        "selectedMode": model_array_mode_selection_mode2_bias_self_check.map(|selection| selection.mode_id),
        "symbolCount": model_array_mode_selection_mode2_bias_self_check.map(|selection| selection.symbol_count),
        "rankedSymbolCount": model_array_mode_selection_mode2_bias_self_check.map(|selection| selection.ranked_symbol_count),
        "shuffledPairCount": model_array_mode_selection_mode2_bias_self_check.map(|selection| selection.shuffled_pair_count),
        "entropyBits": model_array_mode_selection_mode2_bias_self_check.map(|selection| selection.entropy_bits),
        "entropyBitsPerByte": model_array_mode_selection_mode2_bias_self_check.map(|selection| selection.entropy_bits_per_byte),
        "sideHeaderBytes": model_array_mode_selection_mode2_bias_self_check.map(|selection| selection.side_header_bytes),
        "entropyBytesWithHeader": model_array_mode_selection_mode2_bias_self_check.map(|selection| selection.entropy_bytes_with_header),
        "scaledCostDeltaWithBias": model_array_mode_selection_mode2_bias_self_check.and_then(|selection| selection.scaled_cost_delta_with_bias),
        "selectedModelCost": model_array_mode_selection_mode2_bias_self_check.map(|selection| selection.selected_model_cost),
        "finalCost": model_array_mode_selection_mode2_bias_self_check.map(|selection| selection.final_cost),
        "projectedCost": model_array_mode_selection_mode2_bias_self_check.map(|selection| selection.projected_cost)
    });
    packet_writer["modelArraySideHeaderPlainSelfCheck"] = json!({
        "sourceRva": "0x6FB4100",
        "shuffleFlag": MODEL_ARRAY_SHUFFLED_HEADER_FLAG,
        "path": model_array_side_header_plain_self_check.map(LiteralEntropyModelArraySideHeaderPlan::path),
        "usesShuffledRanks": model_array_side_header_plain_self_check.map(|plan| plan.uses_shuffled_ranks),
        "symbolCount": model_array_side_header_plain_self_check.map(|plan| plan.symbol_count),
        "rankedSymbolCount": model_array_side_header_plain_self_check.map(|plan| plan.ranked_symbol_count),
        "shuffledPairCount": model_array_side_header_plain_self_check.map(|plan| plan.shuffled_pair_count),
        "prefixValue": model_array_side_header_plain_self_check.map(|plan| plan.prefix_value),
        "prefixBits": model_array_side_header_plain_self_check.map(|plan| plan.prefix_bits),
        "seedHex": model_array_side_header_plain_self_check.map(|plan| bytes_to_upper_hex(&write_model_array_side_header_seed(plan))),
        "seedBytes": model_array_side_header_plain_self_check.map(|plan| write_model_array_side_header_seed(plan).len()),
        "initialBitCursor": model_array_side_header_plain_self_check.map(|plan| plan.initial_bit_cursor),
        "sideHeaderBytes": model_array_side_header_plain_self_check.map(|plan| plan.side_header_bytes),
        "payloadCapacitySlackBytes": model_array_side_header_plain_self_check.map(|plan| plan.payload_capacity_slack_bytes)
    });
    packet_writer["modelArrayPlainSmallSideHeaderSelfCheck"] = json!({
        "sourceRva": "0x6F8D6A0",
        "path": model_array_plain_small_side_header_self_check.as_ref().map(|header| header.path),
        "symbolIndexBits": model_array_plain_small_side_header_self_check.as_ref().map(|header| header.symbol_index_bits),
        "codeLenBits": model_array_plain_small_side_header_self_check.as_ref().and_then(|header| header.code_len_bits),
        "nonzeroSymbols": model_array_plain_small_side_header_self_check.as_ref().map(|header| header.nonzero_symbols),
        "bitLen": model_array_plain_small_side_header_self_check.as_ref().map(|header| header.bit_len),
        "hex": model_array_plain_small_side_header_self_check.as_ref().map(|header| bytes_to_upper_hex(&header.bytes))
    });
    packet_writer["modelArrayPlainLargeSideHeaderPreambleSelfCheck"] = json!({
        "sourceRva": "0x6F8D6A0",
        "sourceScorerRva": "0x6FB2860",
        "path": model_array_plain_large_side_header_preamble_self_check.as_ref().map(|header| header.path),
        "symbolIndexBits": model_array_plain_large_side_header_preamble_self_check.as_ref().map(|header| header.symbol_index_bits),
        "selectedPredictor": model_array_plain_large_side_header_preamble_self_check.as_ref().map(|header| header.selected_predictor),
        "predictorScores": model_array_plain_large_side_header_preamble_self_check.as_ref().map(|header| header.predictor_scores),
        "residuals": model_array_plain_large_side_header_preamble_self_check.as_ref().map(|header| header.residuals.clone()),
        "residualHistogramPrefix": model_array_plain_large_side_header_preamble_self_check.as_ref().map(|header| header.residual_histogram[..4].to_vec()),
        "nonzeroSymbols": model_array_plain_large_side_header_preamble_self_check.as_ref().map(|header| header.nonzero_symbols),
        "firstSymbolHasCodeLen": model_array_plain_large_side_header_preamble_self_check.as_ref().map(|header| header.first_symbol_has_code_len),
        "bitLen": model_array_plain_large_side_header_preamble_self_check.as_ref().map(|header| header.bit_len),
        "hex": model_array_plain_large_side_header_preamble_self_check.as_ref().map(|header| bytes_to_upper_hex(&header.bytes))
    });
    packet_writer["modelArrayPlainLargeSideHeaderBodySelfCheck"] = json!({
        "sourceRva": "0x6F8D6A0",
        "path": model_array_plain_large_side_header_body_self_check.as_ref().map(|body| body.path),
        "selectedPredictor": model_array_plain_large_side_header_body_self_check.as_ref().map(|body| body.selected_predictor),
        "runKinds": model_array_plain_large_side_header_body_self_check.as_ref().map(|body| body.runs.iter().map(|run| run.kind).collect::<Vec<_>>()),
        "runLengths": model_array_plain_large_side_header_body_self_check.as_ref().map(|body| body.runs.iter().map(|run| run.len).collect::<Vec<_>>()),
        "runDescriptorBits": model_array_plain_large_side_header_body_self_check.as_ref().map(|body| body.runs.iter().map(|run| run.descriptor_bits.clone()).collect::<Vec<_>>()),
        "residualBits": model_array_plain_large_side_header_body_self_check.as_ref().map(|body| body.runs.iter().map(|run| run.residual_bits.clone()).collect::<Vec<_>>()),
        "bitLen": model_array_plain_large_side_header_body_self_check.as_ref().map(|body| body.bit_len),
        "hex": model_array_plain_large_side_header_body_self_check.as_ref().map(|body| bytes_to_upper_hex(&body.bytes))
    });
    packet_writer["modelArraySideHeaderShuffledSelfCheck"] = json!({
        "sourceRva": "0x6FB4100",
        "shuffleFlag": MODEL_ARRAY_SHUFFLED_HEADER_FLAG,
        "path": model_array_side_header_shuffled_self_check.map(LiteralEntropyModelArraySideHeaderPlan::path),
        "usesShuffledRanks": model_array_side_header_shuffled_self_check.map(|plan| plan.uses_shuffled_ranks),
        "symbolCount": model_array_side_header_shuffled_self_check.map(|plan| plan.symbol_count),
        "rankedSymbolCount": model_array_side_header_shuffled_self_check.map(|plan| plan.ranked_symbol_count),
        "shuffledPairCount": model_array_side_header_shuffled_self_check.map(|plan| plan.shuffled_pair_count),
        "prefixValue": model_array_side_header_shuffled_self_check.map(|plan| plan.prefix_value),
        "prefixBits": model_array_side_header_shuffled_self_check.map(|plan| plan.prefix_bits),
        "seedHex": model_array_side_header_shuffled_self_check.map(|plan| bytes_to_upper_hex(&write_model_array_side_header_seed(plan))),
        "seedBytes": model_array_side_header_shuffled_self_check.map(|plan| write_model_array_side_header_seed(plan).len()),
        "initialBitCursor": model_array_side_header_shuffled_self_check.map(|plan| plan.initial_bit_cursor),
        "sideHeaderBytes": model_array_side_header_shuffled_self_check.map(|plan| plan.side_header_bytes),
        "payloadCapacitySlackBytes": model_array_side_header_shuffled_self_check.map(|plan| plan.payload_capacity_slack_bytes)
    });
    packet_writer["tableCandidateSourceRva"] = json!("0x6FB6420");
    packet_writer["tableCandidateSelfCheckTailLen"] =
        json!(table_candidate_builder.plan.map(|plan| plan.tail_len));
    packet_writer["tableCandidateSelfCheckTableBits"] =
        json!(table_candidate_builder.plan.map(|plan| plan.table_bits));
    packet_writer["tableCandidateSelfCheckTableSize"] =
        json!(table_candidate_builder.plan.map(|plan| plan.table_size));
    packet_writer["tableCandidateSelfCheckBytes"] =
        json!(table_candidate_self_check_result.map(|result| result.encoded_bytes));
    packet_writer["tableCandidateSelfCheckCost"] =
        json!(table_candidate_self_check_result.map(|result| result.cost));
    packet_writer["tableCandidateSelfCheckMode"] =
        json!(table_candidate_self_check_result.map(|result| result.mode.as_str()));
    packet_writer["tableCandidateSelfCheckHex"] =
        json!(bytes_to_upper_hex(&table_candidate_self_check));
    packet_writer["repeatedPatternSourceRva"] = json!("0x6FB8E60");
    packet_writer["repeatedPatternSelfCheckPayloadBudget"] = json!(
        repeated_pattern_builder
            .plan
            .map(|plan| plan.payload_budget)
    );
    packet_writer["repeatedPatternSelfCheckPreCost"] =
        json!(repeated_pattern_builder.plan.map(|plan| plan.pre_cost));
    packet_writer["repeatedPatternSelfCheckBaseline"] =
        json!(repeated_pattern_builder.plan.map(|plan| plan.baseline_cost));
    packet_writer["repeatedPatternSelfCheckBytes"] =
        json!(repeated_pattern_self_check_result.map(|result| result.encoded_bytes));
    packet_writer["repeatedPatternSelfCheckCost"] =
        json!(repeated_pattern_self_check_result.map(|result| result.cost));
    packet_writer["repeatedPatternSelfCheckMode"] =
        json!(repeated_pattern_self_check_result.map(|result| result.mode.as_str()));
    packet_writer["repeatedPatternSelfCheckHex"] =
        json!(bytes_to_upper_hex(&repeated_pattern_self_check));
    packet_writer["repeatedPatternNativePayloadSelfCheckHex"] =
        json!(repeated_pattern_native_payload_self_check_hex);
    packet_writer["repeatedPatternNativeSelfCheckBytes"] =
        json!(repeated_pattern_native_self_check_result.map(|result| result.encoded_bytes));
    packet_writer["repeatedPatternNativeSelfCheckCost"] =
        json!(repeated_pattern_native_self_check_result.map(|result| result.cost));
    packet_writer["repeatedPatternNativeSelfCheckMode"] =
        json!(repeated_pattern_native_self_check_result.map(|result| result.mode.as_str()));
    packet_writer["repeatedPatternNativeSelfCheckHex"] =
        json!(bytes_to_upper_hex(&repeated_pattern_native_self_check));
    packet_writer["repeatedPatternSearchBoundarySelfCheckHex"] = json!(
        repeated_pattern_search_boundary_self_check
            .as_ref()
            .map(|payload| bytes_to_upper_hex(&payload.payload))
    );
    packet_writer["repeatedPatternSearchBoundarySelfCheckDataBytes"] = json!(
        repeated_pattern_search_boundary_self_check
            .as_ref()
            .map(|payload| payload.data_bytes)
    );
    packet_writer["repeatedPatternSearchBoundarySelfCheckControlBytes"] = json!(
        repeated_pattern_search_boundary_self_check
            .as_ref()
            .map(|payload| payload.control_bytes)
    );
    packet_writer["repeatedPatternRunGapTooTightSelfCheckRejected"] =
        json!(repeated_pattern_run_gap_too_tight_rejected);
    packet_writer["repeatedPatternTailGapTooTightSelfCheckRejected"] =
        json!(repeated_pattern_tail_gap_too_tight_rejected);
    packet_writer["repeatedPatternRawSpanPayloadSelfCheckHex"] =
        json!(repeated_pattern_raw_span_payload_self_check_hex);
    packet_writer["repeatedPatternRawSpanSelfCheckBytes"] =
        json!(repeated_pattern_raw_span_self_check_result.map(|result| result.encoded_bytes));
    packet_writer["repeatedPatternRawSpanSelfCheckCost"] =
        json!(repeated_pattern_raw_span_self_check_result.map(|result| result.cost));
    packet_writer["repeatedPatternRawSpanSelfCheckMode"] =
        json!(repeated_pattern_raw_span_self_check_result.map(|result| result.mode.as_str()));
    packet_writer["repeatedPatternRawSpanSelfCheckHex"] =
        json!(bytes_to_upper_hex(&repeated_pattern_raw_span_self_check));
    packet_writer["repeatedPatternSymbolReusePayloadSelfCheckHex"] =
        json!(repeated_pattern_symbol_reuse_payload_self_check_hex);
    packet_writer["repeatedPatternSymbolReuseSelfCheckBytes"] =
        json!(repeated_pattern_symbol_reuse_self_check_result.map(|result| result.encoded_bytes));
    packet_writer["repeatedPatternSymbolReuseSelfCheckCost"] =
        json!(repeated_pattern_symbol_reuse_self_check_result.map(|result| result.cost));
    packet_writer["repeatedPatternSymbolReuseSelfCheckMode"] =
        json!(repeated_pattern_symbol_reuse_self_check_result.map(|result| result.mode.as_str()));
    packet_writer["repeatedPatternSymbolReuseSelfCheckHex"] = json!(bytes_to_upper_hex(
        &repeated_pattern_symbol_reuse_self_check
    ));
    packet_writer["repeatedPatternCompactPairPayloadSelfCheckHex"] =
        json!(repeated_pattern_compact_pair_payload_self_check_hex);
    packet_writer["repeatedPatternCompactPairSelfCheckBytes"] =
        json!(repeated_pattern_compact_pair_self_check_result.map(|result| result.encoded_bytes));
    packet_writer["repeatedPatternCompactPairSelfCheckCost"] =
        json!(repeated_pattern_compact_pair_self_check_result.map(|result| result.cost));
    packet_writer["repeatedPatternCompactPairSelfCheckMode"] =
        json!(repeated_pattern_compact_pair_self_check_result.map(|result| result.mode.as_str()));
    packet_writer["repeatedPatternCompactPairSelfCheckHex"] = json!(bytes_to_upper_hex(
        &repeated_pattern_compact_pair_self_check
    ));
    packet_writer["repeatedPatternContinuationControlSelfCheckHex"] =
        json!(repeated_pattern_continuation_control_self_check_hex);
    packet_writer["repeatedPatternContinuationSelfCheckHeaderHex"] =
        json!(repeated_pattern_continuation_self_check_header_hex);
    packet_writer["repeatedPatternContinuationSelfCheckPayloadBytes"] = json!(
        repeated_pattern_continuation_payload_self_check
            .as_ref()
            .map(|payload| payload.payload.len())
    );
    packet_writer["repeatedPatternContinuationSelfCheckDataBytes"] = json!(
        repeated_pattern_continuation_payload_self_check
            .as_ref()
            .map(|payload| payload.data_bytes)
    );
    packet_writer["repeatedPatternContinuationSelfCheckControlBytes"] = json!(
        repeated_pattern_continuation_payload_self_check
            .as_ref()
            .map(|payload| payload.control_bytes)
    );
    packet_writer["repeatedPatternContinuationSelfCheckBytes"] =
        json!(repeated_pattern_continuation_self_check_result.map(|result| result.encoded_bytes));
    packet_writer["repeatedPatternContinuationSelfCheckCost"] =
        json!(repeated_pattern_continuation_self_check_result.map(|result| result.cost));
    packet_writer["repeatedPatternContinuationSelfCheckMode"] =
        json!(repeated_pattern_continuation_self_check_result.map(|result| result.mode.as_str()));
    packet_writer["repeatedPatternMultiChunkContinuationControlSelfCheckHex"] =
        json!(repeated_pattern_multi_chunk_continuation_control_self_check_hex);
    packet_writer["repeatedPatternMultiChunkContinuationSelfCheckHeaderHex"] =
        json!(repeated_pattern_multi_chunk_continuation_self_check_header_hex);
    packet_writer["repeatedPatternMultiChunkContinuationSelfCheckPayloadBytes"] = json!(
        repeated_pattern_multi_chunk_continuation_payload_self_check
            .as_ref()
            .map(|payload| payload.payload.len())
    );
    packet_writer["repeatedPatternMultiChunkContinuationSelfCheckDataBytes"] = json!(
        repeated_pattern_multi_chunk_continuation_payload_self_check
            .as_ref()
            .map(|payload| payload.data_bytes)
    );
    packet_writer["repeatedPatternMultiChunkContinuationSelfCheckControlBytes"] = json!(
        repeated_pattern_multi_chunk_continuation_payload_self_check
            .as_ref()
            .map(|payload| payload.control_bytes)
    );
    packet_writer["repeatedPatternMultiChunkContinuationSelfCheckBytes"] = json!(
        repeated_pattern_multi_chunk_continuation_self_check_result
            .map(|result| result.encoded_bytes)
    );
    packet_writer["repeatedPatternMultiChunkContinuationSelfCheckCost"] = json!(
        repeated_pattern_multi_chunk_continuation_self_check_result.map(|result| result.cost)
    );
    packet_writer["repeatedPatternMultiChunkContinuationSelfCheckMode"] = json!(
        repeated_pattern_multi_chunk_continuation_self_check_result
            .map(|result| result.mode.as_str())
    );
    packet_writer["repeatedPatternOptionalSubstreamPlan"] =
        repeated_pattern_optional_substream_plan_status;
    packet_writer["repeatedPatternOptionalSubstreamSingleSymbolSelfCheckHex"] =
        json!(repeated_pattern_optional_substream_single_symbol_self_check_hex);
    packet_writer["repeatedPatternOptionalSubstreamSingleSymbolDataPacketBytes"] = json!(
        repeated_pattern_optional_substream_single_symbol_self_check
            .as_ref()
            .map(|candidate| candidate.data_packet_bytes)
    );
    packet_writer["repeatedPatternOptionalSubstreamSingleSymbolControlBytes"] = json!(
        repeated_pattern_optional_substream_single_symbol_self_check
            .as_ref()
            .map(|candidate| candidate.control_bytes)
    );
    packet_writer["repeatedPatternOptionalSubstreamSingleSymbolBytes"] = json!(
        repeated_pattern_optional_substream_single_symbol_self_check
            .as_ref()
            .map(|candidate| candidate.payload.len())
    );
    packet_writer["repeatedPatternOptionalSubstreamSingleSymbolSubstreamCost"] = json!(
        repeated_pattern_optional_substream_single_symbol_self_check
            .as_ref()
            .map(|candidate| candidate.substream_cost)
    );
    packet_writer["repeatedPatternOptionalSubstreamSingleSymbolTotalCost"] = json!(
        repeated_pattern_optional_substream_single_symbol_self_check
            .as_ref()
            .map(|candidate| candidate.total_cost)
    );
    packet_writer["repeatedPatternOptionalSubstreamSingleSymbolBaselineCost"] = json!(
        repeated_pattern_optional_substream_single_symbol_self_check
            .as_ref()
            .map(|candidate| candidate.baseline_cost)
    );
    packet_writer["repeatedPatternOptionalSubstreamSingleSymbolMode"] = json!(
        repeated_pattern_optional_substream_single_symbol_self_check
            .as_ref()
            .map(|candidate| candidate.mode.as_str())
    );
    packet_writer["repeatedPatternOptionalSubstreamModelArraySelfCheckHex"] =
        json!(repeated_pattern_optional_substream_model_array_self_check_hex);
    packet_writer["repeatedPatternOptionalSubstreamModelArrayBuilderCalls"] =
        json!(repeated_pattern_optional_substream_model_array_builder_calls);
    packet_writer["repeatedPatternOptionalSubstreamModelArrayDataPacketBytes"] = json!(
        repeated_pattern_optional_substream_model_array_self_check
            .as_ref()
            .map(|candidate| candidate.data_packet_bytes)
    );
    packet_writer["repeatedPatternOptionalSubstreamModelArrayControlBytes"] = json!(
        repeated_pattern_optional_substream_model_array_self_check
            .as_ref()
            .map(|candidate| candidate.control_bytes)
    );
    packet_writer["repeatedPatternOptionalSubstreamModelArrayBytes"] = json!(
        repeated_pattern_optional_substream_model_array_self_check
            .as_ref()
            .map(|candidate| candidate.payload.len())
    );
    packet_writer["repeatedPatternOptionalSubstreamModelArraySubstreamCost"] = json!(
        repeated_pattern_optional_substream_model_array_self_check
            .as_ref()
            .map(|candidate| candidate.substream_cost)
    );
    packet_writer["repeatedPatternOptionalSubstreamModelArrayTotalCost"] = json!(
        repeated_pattern_optional_substream_model_array_self_check
            .as_ref()
            .map(|candidate| candidate.total_cost)
    );
    packet_writer["repeatedPatternOptionalSubstreamModelArrayBaselineCost"] = json!(
        repeated_pattern_optional_substream_model_array_self_check
            .as_ref()
            .map(|candidate| candidate.baseline_cost)
    );
    packet_writer["repeatedPatternOptionalSubstreamModelArrayMode"] = json!(
        repeated_pattern_optional_substream_model_array_self_check
            .as_ref()
            .map(|candidate| candidate.mode.as_str())
    );

    json!({
        "available": false,
        "canDecompress": false,
        "canCompress": false,
        "status": "native_encoder_in_progress",
        "adapter": "pure_rust_kraken",
        "encoderDetails": {
            "oodleCompressorId": OODLE_COMPRESSOR_KRAKEN,
            "blockLength": KRAKEN_BLOCK_LEN,
            "optimalScratchBytes": KRAKEN_OPTIMAL_SCRATCH_BYTES,
            "g1rLevel6Context": {
                "compressionLevel": context.compression_level,
                "compressorId": context.compressor_id,
                "blockLength": context.block_len,
                "primaryBlockEncoder": context.primary_block_encoder.as_str(),
                "secondaryHelper": context.secondary_helper.as_str(),
                "highCompressionFlag": context.high_compression_flag,
                "encodeScratchBytes": context.encode_scratch_bytes
            },
            "emissionStateLayout": {
                "capacity": emission_layout.capacity,
                "halfPlusHeader": emission_layout.half_plus_header,
                "third": emission_layout.third,
                "fifth": emission_layout.fifth,
                "byteTableBytes": emission_layout.byte_table_bytes,
                "backingLength": emission_layout.backing_len,
                "cursorStarts": emission_layout.cursor_starts,
                "byteTableEnd": emission_layout.byte_table_end
            },
            "emissionState": {
                "backingLength": emission_state.backing_len(),
                "streamSpans": emission_state.stream_spans(),
                "cursorOffsets": emission_state.cursor_offsets(),
                "byteTableSpan": emission_state.byte_table_span(),
                "tailBytes": emission_state.tail_len(),
                "sourceBase": emission_state.source_base,
                "userTag": emission_state.user_tag,
                "zeroInitialized": emission_state.backing().iter().all(|byte| *byte == 0),
                "emptyWriteOk": empty_write_ok
            },
            "jobWrapper": {
                "sourceRva": "0x6FB9700",
                "inlineRunOk": inline_job_ran,
                "inlineMode": inline_job_mode.map(JobDispatchMode::as_str),
                "inlineHandleCleared": inline_job_handle.is_none(),
                "maxDependencies": 2
            },
            "encodeDriver": {
                "sourceRva": "0x6F94CC0",
                "subjobWindowBytes": NEWLZ_SUBJOB_WINDOW_BYTES,
                "subjobThresholdBytes": NEWLZ_SUBJOB_THRESHOLD_BYTES,
                "sampleSubjobPlan": sample_subjob_plan.map(|plan| json!({
                    "rawLength": plan.raw_len,
                    "subjobCount": plan.subjob_count,
                    "leadingWindowBytes": plan.leading_window_bytes,
                    "scratchAllocationBytes": plan.scratch_allocation_bytes
                }))
            },
            "packetWriter": packet_writer
        },
        "message": "Native private payload encoder is not available yet."
    })
}

fn bytes_to_upper_hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect::<Vec<_>>()
        .join("")
}

pub fn inspect_private_payload(
    _data: &[u8],
    stream: &CompressedStream,
) -> Result<Value, CoreError> {
    Ok(json!({
        "status": "native_encoder_in_progress",
        "message": "Private payload decoding and writing require verified codec support.",
        "method": stream.method,
        "algorithmId": stream.algorithm_id,
        "chunkCount": stream.chunk_count,
        "compressedSize": stream.summary_compressed_size,
        "uncompressedSize": stream.summary_uncompressed_size,
        "writable": [],
    }))
}

#[cfg(test)]
pub fn apply_private_edits(
    _data: &[u8],
    _stream: &CompressedStream,
    edits: &[PrivateEdit],
) -> Result<Vec<u8>, CoreError> {
    if edits.is_empty() {
        return Err(CoreError::InvalidRequest(
            "private edit list is empty".to_string(),
        ));
    }
    Err(CoreError::Codec(
        "native private payload encoder is not implemented yet".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct RecordingVarLenLiteralEntropyBuilder {
        short_calls: Vec<usize>,
        long_calls: Vec<usize>,
        alternate_calls: Vec<(usize, f32)>,
        short_result: Option<VarLenLiteralEntropyCandidate>,
        long_result: Option<VarLenLiteralEntropyCandidate>,
        alternate_result: Option<VarLenLiteralEntropyCandidate>,
    }

    impl VarLenLiteralEntropyBuilder for RecordingVarLenLiteralEntropyBuilder {
        fn encode_short_range(
            &mut self,
            literal_len: usize,
        ) -> Result<Option<VarLenLiteralEntropyCandidate>, CoreError> {
            self.short_calls.push(literal_len);
            Ok(self.short_result)
        }

        fn encode_long_range(
            &mut self,
            literal_len: usize,
        ) -> Result<Option<VarLenLiteralEntropyCandidate>, CoreError> {
            self.long_calls.push(literal_len);
            Ok(self.long_result)
        }

        fn encode_alternate(
            &mut self,
            literal_len: usize,
            baseline_cost_without_base: f32,
        ) -> Result<Option<VarLenLiteralEntropyCandidate>, CoreError> {
            self.alternate_calls
                .push((literal_len, baseline_cost_without_base));
            Ok(self.alternate_result)
        }
    }

    #[derive(Default)]
    struct RecordingLiteralEntropyPacketEncoder {
        calls: Vec<(usize, u32, u32)>,
    }

    impl LiteralEntropyPacketEncoder for RecordingLiteralEntropyPacketEncoder {
        fn encode_literal_packet(
            &mut self,
            output: &mut Vec<u8>,
            output_capacity: usize,
            literal: &[u8],
            histogram: &[u32; 256],
        ) -> Result<LiteralPacketResult, CoreError> {
            self.calls
                .push((literal.len(), histogram[0xAB], histogram[0xCD]));
            write_entropy_single_symbol_packet(
                output,
                output_capacity,
                literal,
                SingleSymbolPacketMode::DirectSymbol,
                4.0,
                2.0,
                100.0,
            )
        }
    }

    #[derive(Default)]
    struct RecordingLiteralEntropyModelBuilder {
        calls: Vec<(usize, f32, u32, u32)>,
        result: Option<LiteralEntropyModelCandidate>,
    }

    impl LiteralEntropyModelBuilder for RecordingLiteralEntropyModelBuilder {
        fn encode_model_array_candidate(
            &mut self,
            literal: &[u8],
            histogram: &[u32; 256],
            baseline_cost: f32,
        ) -> Result<Option<LiteralEntropyModelCandidate>, CoreError> {
            self.calls.push((
                literal.len(),
                baseline_cost,
                histogram[b'A' as usize],
                histogram[b'B' as usize],
            ));
            Ok(self.result.clone())
        }
    }

    #[derive(Default)]
    struct RecordingLiteralEntropyTableBuilder {
        calls: Vec<(usize, usize, usize, usize, u32, u32)>,
        result: Option<LiteralEntropyTableCandidate>,
    }

    impl LiteralEntropyTableBuilder for RecordingLiteralEntropyTableBuilder {
        fn encode_table_candidate(
            &mut self,
            literal: &[u8],
            adjusted_histogram: &[u32; 256],
            plan: LiteralEntropyTablePlan,
            current_best_cost: f32,
        ) -> Result<Option<LiteralEntropyTableCandidate>, CoreError> {
            self.calls.push((
                literal.len(),
                plan.tail_len,
                plan.table_bits,
                plan.effective_symbol_count,
                adjusted_histogram[b'A' as usize],
                adjusted_histogram[b'F' as usize],
            ));
            assert_eq!(current_best_cost, 1000.0);
            Ok(self.result.clone())
        }
    }

    #[derive(Default)]
    struct RecordingLiteralEntropyRepeatedPatternBuilder {
        calls: Vec<(usize, usize, f32, f32)>,
        result: Option<LiteralEntropyRepeatedPatternCandidate>,
    }

    impl LiteralEntropyRepeatedPatternBuilder for RecordingLiteralEntropyRepeatedPatternBuilder {
        fn encode_repeated_pattern_candidate(
            &mut self,
            literal: &[u8],
            plan: LiteralEntropyRepeatedPatternPlan,
        ) -> Result<Option<LiteralEntropyRepeatedPatternCandidate>, CoreError> {
            self.calls.push((
                literal.len(),
                plan.payload_budget,
                plan.baseline_cost,
                plan.pre_cost,
            ));
            Ok(self.result.clone())
        }
    }

    fn stream() -> CompressedStream {
        CompressedStream {
            stream_offset: 0,
            uncompressed_size_prefix: 131_072,
            method: "Oodle".to_string(),
            package_tag: 0x9E2A83C1,
            header_version: 0x22222222,
            max_chunk_size: 131_072,
            algorithm_id: Some(2),
            summary_compressed_size: 1024,
            summary_uncompressed_size: 131_072,
            chunk_count: 1,
            compressed_payload_offset: 128,
            compressed_payload_size: 1024,
            stream_end_offset: 1152,
            trailing_size: 0,
            chunks: Vec::new(),
        }
    }

    #[test]
    fn codec_status_declares_native_encoder_without_external_adapter() {
        let status = codec_status();

        assert_eq!(status["available"], false);
        assert_eq!(status["canCompress"], false);
        assert_eq!(status["status"], "native_encoder_in_progress");
        assert_eq!(status["adapter"], "pure_rust_kraken");
        assert!(!status.to_string().contains("oo2core"));
        assert!(!status.to_string().contains("GORESAVE_OODLE_DLL"));
    }

    #[test]
    fn codec_status_exposes_encoder_layout() {
        let status = codec_status();
        let context = &status["encoderDetails"]["g1rLevel6Context"];
        let emission = &status["encoderDetails"]["emissionStateLayout"];
        let state = &status["encoderDetails"]["emissionState"];
        let packet_writer = &status["encoderDetails"]["packetWriter"];

        assert_eq!(context["compressionLevel"], 6);
        assert_eq!(context["blockLength"], KRAKEN_BLOCK_LEN);
        assert_eq!(context["primaryBlockEncoder"], "kraken_chunk_optimal");
        assert_eq!(context["secondaryHelper"], "kraken_level_6_plus");
        assert_eq!(emission["capacity"], KRAKEN_OPTIMAL_SCRATCH_BYTES);
        assert_eq!(emission["backingLength"], 0x6A54);
        assert_eq!(emission["byteTableEnd"], 0x6958);
        assert_eq!(state["backingLength"], 0x6A54);
        assert_eq!(state["tailBytes"], 0xFC);
        assert_eq!(state["zeroInitialized"], true);
        assert_eq!(state["emptyWriteOk"], true);
        assert_eq!(
            packet_writer["rawLiteralMaxBytes"],
            RAW_LITERAL_PACKET_MAX_BYTES
        );
        assert_eq!(packet_writer["rawLiteralSelfCheckBytes"], 6);
        assert_eq!(
            packet_writer["shortLiteralMaxBytes"],
            SHORT_LITERAL_RAW_MAX_BYTES
        );
        assert_eq!(packet_writer["shortLiteralSelfCheckMode"], "raw_fallback");
        assert_eq!(packet_writer["shortLiteralSelfCheckCost"], 6.0);
        assert_eq!(packet_writer["wrappedLiteralSourceRva"], "0x6F927C0");
        assert_eq!(packet_writer["wrappedShortRawSelfCheckHex"], "8003473152");
        assert_eq!(packet_writer["wrappedShortRawSelfCheckBytes"], 5);
        assert_eq!(packet_writer["wrappedShortRawSelfCheckCost"], 5.0);
        assert_eq!(
            packet_writer["wrappedEntropyCostDiscount"],
            WRAPPED_ENTROPY_COST_DISCOUNT
        );
        assert_eq!(
            packet_writer["wrappedEntropySelfCheckMode"],
            "wrapped_entropy"
        );
        assert_eq!(packet_writer["wrappedEntropySelfCheckHex"], "B07C01AB");
        assert_eq!(packet_writer["wrappedEntropySelfCheckBytes"], 4);
        assert_eq!(packet_writer["wrappedEntropySelfCheckCost"], 12.0);
        assert_eq!(
            packet_writer["wrappedEntropyLongRouteSelfCheckHex"],
            "B07C01AB"
        );
        assert_eq!(packet_writer["wrappedEntropyLongRouteHistogramAB"], 0x21);
        assert_eq!(packet_writer["wrappedEntropyLongRouteSelfCheckBytes"], 4);
        assert_eq!(packet_writer["literalHistogramSourceRva"], "0x6F8FA20");
        assert_eq!(
            packet_writer["literalHistogramSelfCheckCounts"],
            serde_json::json!({
                "A": 3,
                "B": 1,
                "ff": 1,
                "C": 0
            })
        );
        assert_eq!(packet_writer["modelArraySourceRva"], "0x6FB4100");
        assert_eq!(packet_writer["modelArraySelfCheckBaseline"], 67.0);
        assert_eq!(packet_writer["modelArraySelfCheckHex"], "4000FC0002DEAD");
        assert_eq!(packet_writer["modelArraySelfCheckBytes"], 7);
        assert_eq!(packet_writer["modelArraySelfCheckCost"], 20.0);
        assert_eq!(
            packet_writer["modelArrayNativeSingleSymbolPayloadSelfCheckHex"],
            "006AC0"
        );
        assert_eq!(
            packet_writer["modelArrayNativeSingleSymbolSelfCheckHex"],
            "2000800003006AC0"
        );
        assert_eq!(
            packet_writer["modelArrayNativeSingleSymbolSelfCheckBytes"],
            8
        );
        assert_eq!(
            packet_writer["modelArrayNativeSingleSymbolSelfCheckCost"],
            16.0
        );
        let mode4_plan = &packet_writer["modelArrayModeSelectionMode4SelfCheck"];
        assert_eq!(mode4_plan["mode4EnableFlag"], 0x01);
        assert_eq!(mode4_plan["mode4SelectionBias"], 6.3125);
        assert_eq!(mode4_plan["selectedMode"], 4);
        assert_eq!(mode4_plan["symbolCount"], 6);
        assert_eq!(mode4_plan["rankedSymbolCount"], 7);
        assert_eq!(mode4_plan["shuffledPairCount"], 3);
        assert_eq!(mode4_plan["entropyBits"], 0x100);
        assert_eq!(mode4_plan["entropyBitsPerByte"], 4.0);
        assert_eq!(mode4_plan["sideHeaderBytes"], 3);
        assert_eq!(mode4_plan["entropyBytesWithHeader"], 45);
        assert_eq!(mode4_plan["scaledCostDeltaWithBias"], -3.6875);
        assert_eq!(mode4_plan["selectedModelCost"], 45.0);
        assert_eq!(mode4_plan["finalCost"], 95.0);
        assert_eq!(mode4_plan["projectedCost"], 143.0);
        let mode2_plan = &packet_writer["modelArrayModeSelectionMode2BiasSelfCheck"];
        assert_eq!(mode2_plan["selectedMode"], 2);
        assert_eq!(mode2_plan["scaledCostDeltaWithBias"], 2.3125);
        assert_eq!(mode2_plan["selectedModelCost"], 50.0);
        assert_eq!(mode2_plan["finalCost"], 105.0);
        assert_eq!(mode2_plan["projectedCost"], 153.0);
        let plain_header = &packet_writer["modelArraySideHeaderPlainSelfCheck"];
        assert_eq!(plain_header["path"], "plain");
        assert_eq!(plain_header["shuffleFlag"], 0x40);
        assert_eq!(plain_header["usesShuffledRanks"], false);
        assert_eq!(plain_header["symbolCount"], 4);
        assert_eq!(plain_header["rankedSymbolCount"], 9);
        assert_eq!(plain_header["prefixValue"], 0);
        assert_eq!(plain_header["prefixBits"], 1);
        assert_eq!(plain_header["seedHex"], "00");
        assert_eq!(plain_header["seedBytes"], 1);
        assert_eq!(plain_header["initialBitCursor"], 0x3E);
        assert_eq!(plain_header["sideHeaderBytes"], 2);
        assert_eq!(plain_header["payloadCapacitySlackBytes"], 8);
        let plain_small_header = &packet_writer["modelArrayPlainSmallSideHeaderSelfCheck"];
        assert_eq!(plain_small_header["path"], "plain_small");
        assert_eq!(plain_small_header["symbolIndexBits"], 2);
        assert_eq!(plain_small_header["codeLenBits"], 2);
        assert_eq!(plain_small_header["nonzeroSymbols"], 3);
        assert_eq!(plain_small_header["bitLen"], 18);
        assert_eq!(plain_small_header["hex"], "681780");
        let plain_large_preamble =
            &packet_writer["modelArrayPlainLargeSideHeaderPreambleSelfCheck"];
        assert_eq!(plain_large_preamble["path"], "plain_large_preamble");
        assert_eq!(plain_large_preamble["sourceScorerRva"], "0x6FB2860");
        assert_eq!(plain_large_preamble["symbolIndexBits"], 3);
        assert_eq!(plain_large_preamble["selectedPredictor"], 1);
        assert_eq!(
            plain_large_preamble["predictorScores"],
            serde_json::json!([16, 15, 18, 24])
        );
        assert_eq!(
            plain_large_preamble["residuals"],
            serde_json::json!([3, 1, 0, 2, 1, 3])
        );
        assert_eq!(
            plain_large_preamble["residualHistogramPrefix"],
            serde_json::json!([1, 2, 1, 2])
        );
        assert_eq!(plain_large_preamble["firstSymbolHasCodeLen"], true);
        assert_eq!(plain_large_preamble["bitLen"], 5);
        assert_eq!(plain_large_preamble["hex"], "58");
        let plain_large_body = &packet_writer["modelArrayPlainLargeSideHeaderBodySelfCheck"];
        assert_eq!(plain_large_body["path"], "plain_large_body");
        assert_eq!(plain_large_body["selectedPredictor"], 1);
        assert_eq!(
            plain_large_body["runKinds"],
            serde_json::json!(["nonzero", "zero", "nonzero", "zero", "nonzero"])
        );
        assert_eq!(
            plain_large_body["runLengths"],
            serde_json::json!([2, 1, 2, 1, 2])
        );
        assert_eq!(
            plain_large_body["runDescriptorBits"],
            serde_json::json!(["11", "10", "11", "10", "11"])
        );
        assert_eq!(
            plain_large_body["residualBits"],
            serde_json::json!([["011", "11"], [], ["10", "010"], [], ["11", "011"]])
        );
        assert_eq!(plain_large_body["bitLen"], 30);
        assert_eq!(plain_large_body["hex"], "5EFB95EC");
        let shuffled_header = &packet_writer["modelArraySideHeaderShuffledSelfCheck"];
        assert_eq!(shuffled_header["path"], "shuffled");
        assert_eq!(shuffled_header["shuffleFlag"], 0x40);
        assert_eq!(shuffled_header["usesShuffledRanks"], true);
        assert_eq!(shuffled_header["symbolCount"], 5);
        assert_eq!(shuffled_header["rankedSymbolCount"], 9);
        assert_eq!(shuffled_header["shuffledPairCount"], 4);
        assert_eq!(shuffled_header["prefixValue"], 2);
        assert_eq!(shuffled_header["prefixBits"], 2);
        assert_eq!(shuffled_header["seedHex"], "80");
        assert_eq!(shuffled_header["seedBytes"], 1);
        assert_eq!(shuffled_header["initialBitCursor"], 0x3D);
        assert_eq!(shuffled_header["sideHeaderBytes"], 3);
        assert_eq!(shuffled_header["payloadCapacitySlackBytes"], 8);
        assert_eq!(packet_writer["tableCandidateSourceRva"], "0x6FB6420");
        assert_eq!(packet_writer["tableCandidateSelfCheckTailLen"], 59);
        assert_eq!(packet_writer["tableCandidateSelfCheckTableBits"], 8);
        assert_eq!(
            packet_writer["tableCandidateSelfCheckHex"],
            "1000FC0003FEEDFA"
        );
        assert_eq!(packet_writer["tableCandidateSelfCheckBytes"], 8);
        assert_eq!(packet_writer["tableCandidateSelfCheckCost"], 21.0);
        assert_eq!(packet_writer["repeatedPatternSourceRva"], "0x6FB8E60");
        assert_eq!(packet_writer["repeatedPatternSelfCheckPayloadBudget"], 11);
        assert_eq!(packet_writer["repeatedPatternSelfCheckPreCost"], 13.0);
        assert_eq!(
            packet_writer["repeatedPatternSelfCheckHex"],
            "3000FC0003FACE01"
        );
        assert_eq!(packet_writer["repeatedPatternSelfCheckBytes"], 8);
        assert_eq!(packet_writer["repeatedPatternSelfCheckCost"], 20.0);
        assert_eq!(
            packet_writer["repeatedPatternNativePayloadSelfCheckHex"],
            "0041001801"
        );
        assert_eq!(
            packet_writer["repeatedPatternNativeSelfCheckHex"],
            "30007C00050041001801"
        );
        assert_eq!(packet_writer["repeatedPatternNativeSelfCheckBytes"], 10);
        assert_eq!(packet_writer["repeatedPatternNativeSelfCheckCost"], 18.0);
        assert_eq!(
            packet_writer["repeatedPatternSearchBoundarySelfCheckHex"],
            "006162636465666768696A6B6C6D6E4141414141414141414141414141414141412010"
        );
        assert_eq!(
            packet_writer["repeatedPatternSearchBoundarySelfCheckDataBytes"],
            0x20
        );
        assert_eq!(
            packet_writer["repeatedPatternSearchBoundarySelfCheckControlBytes"],
            2
        );
        assert_eq!(
            packet_writer["repeatedPatternRunGapTooTightSelfCheckRejected"],
            true
        );
        assert_eq!(
            packet_writer["repeatedPatternTailGapTooTightSelfCheckRejected"],
            true
        );
        assert_eq!(
            packet_writer["repeatedPatternRawSpanPayloadSelfCheckHex"],
            "004178797A031801"
        );
        assert_eq!(
            packet_writer["repeatedPatternRawSpanSelfCheckHex"],
            "3000880008004178797A031801"
        );
        assert_eq!(packet_writer["repeatedPatternRawSpanSelfCheckBytes"], 13);
        assert_eq!(packet_writer["repeatedPatternRawSpanSelfCheckCost"], 21.0);
        assert_eq!(
            packet_writer["repeatedPatternSymbolReusePayloadSelfCheckHex"],
            "004162630218001801"
        );
        assert_eq!(
            packet_writer["repeatedPatternSymbolReuseSelfCheckHex"],
            "3001040009004162630218001801"
        );
        assert_eq!(
            packet_writer["repeatedPatternSymbolReuseSelfCheckBytes"],
            14
        );
        assert_eq!(
            packet_writer["repeatedPatternSymbolReuseSelfCheckCost"],
            22.0
        );
        assert_eq!(
            packet_writer["repeatedPatternCompactPairPayloadSelfCheckHex"],
            "0041626342BFBF013D8F01"
        );
        assert_eq!(
            packet_writer["repeatedPatternCompactPairSelfCheckHex"],
            "300088000B0041626342BFBF013D8F01"
        );
        assert_eq!(
            packet_writer["repeatedPatternCompactPairSelfCheckBytes"],
            16
        );
        assert_eq!(
            packet_writer["repeatedPatternCompactPairSelfCheckCost"],
            24.0
        );
        assert_eq!(
            packet_writer["repeatedPatternContinuationControlSelfCheckHex"],
            "0109010201"
        );
        assert_eq!(
            packet_writer["repeatedPatternContinuationSelfCheckHeaderHex"],
            "3005FC0087"
        );
        assert_eq!(
            packet_writer["repeatedPatternContinuationSelfCheckPayloadBytes"],
            0x87
        );
        assert_eq!(
            packet_writer["repeatedPatternContinuationSelfCheckDataBytes"],
            0x81
        );
        assert_eq!(
            packet_writer["repeatedPatternContinuationSelfCheckControlBytes"],
            5
        );
        assert_eq!(
            packet_writer["repeatedPatternContinuationSelfCheckBytes"],
            140
        );
        assert_eq!(
            packet_writer["repeatedPatternContinuationSelfCheckCost"],
            148.0
        );
        assert_eq!(
            packet_writer["repeatedPatternMultiChunkContinuationControlSelfCheckHex"],
            "01090002FF0801"
        );
        assert_eq!(
            packet_writer["repeatedPatternMultiChunkContinuationSelfCheckHeaderHex"],
            "3704FDC049"
        );
        assert_eq!(
            packet_writer["repeatedPatternMultiChunkContinuationSelfCheckPayloadBytes"],
            0x1C049
        );
        assert_eq!(
            packet_writer["repeatedPatternMultiChunkContinuationSelfCheckDataBytes"],
            0x1C041
        );
        assert_eq!(
            packet_writer["repeatedPatternMultiChunkContinuationSelfCheckControlBytes"],
            7
        );
        assert_eq!(
            packet_writer["repeatedPatternMultiChunkContinuationSelfCheckBytes"],
            0x1C04E
        );
        assert_eq!(
            packet_writer["repeatedPatternMultiChunkContinuationSelfCheckCost"],
            114774.0
        );
        let optional_plan = &packet_writer["repeatedPatternOptionalSubstreamPlan"];
        assert_eq!(optional_plan["sourceFlag"], 0x08);
        assert_eq!(optional_plan["minDataBytes"], 0x20);
        assert_eq!(optional_plan["maxCombinedBytesExclusive"], 0xC001);
        assert_eq!(optional_plan["selfCheckEnabled"], true);
        assert_eq!(optional_plan["selfCheckDataBytes"], 0x21);
        assert_eq!(optional_plan["selfCheckControlBytes"], 0x0F);
        assert_eq!(optional_plan["selfCheckCombinedBytes"], 0x30);
        assert_eq!(optional_plan["selfCheckAlignedDataBytes"], 0x30);
        assert_eq!(optional_plan["selfCheckArenaHeaderBytes"], 0x10);
        assert_eq!(optional_plan["selfCheckScratchBytes"], 0x40);
        assert_eq!(optional_plan["selfCheckBaselineCost"], 34.0);
        assert_eq!(optional_plan["disabledWithoutFlag"], true);
        assert_eq!(optional_plan["tooShortRejected"], true);
        assert_eq!(optional_plan["tooLargeRejected"], true);
        assert_eq!(
            packet_writer["repeatedPatternOptionalSubstreamSingleSymbolSelfCheckHex"],
            "B07C01ABAA55"
        );
        assert_eq!(
            packet_writer["repeatedPatternOptionalSubstreamSingleSymbolDataPacketBytes"],
            4
        );
        assert_eq!(
            packet_writer["repeatedPatternOptionalSubstreamSingleSymbolControlBytes"],
            2
        );
        assert_eq!(
            packet_writer["repeatedPatternOptionalSubstreamSingleSymbolBytes"],
            6
        );
        assert_eq!(
            packet_writer["repeatedPatternOptionalSubstreamSingleSymbolSubstreamCost"],
            12.0
        );
        assert_eq!(
            packet_writer["repeatedPatternOptionalSubstreamSingleSymbolTotalCost"],
            14.0
        );
        assert_eq!(
            packet_writer["repeatedPatternOptionalSubstreamSingleSymbolBaselineCost"],
            34.0
        );
        assert_eq!(
            packet_writer["repeatedPatternOptionalSubstreamModelArraySelfCheckHex"],
            "C07802DEADAA55"
        );
        assert_eq!(
            packet_writer["repeatedPatternOptionalSubstreamModelArrayBuilderCalls"],
            serde_json::json!([[0x21, 0x10, 0x11, 34.0]])
        );
        assert_eq!(
            packet_writer["repeatedPatternOptionalSubstreamModelArrayDataPacketBytes"],
            5
        );
        assert_eq!(
            packet_writer["repeatedPatternOptionalSubstreamModelArrayControlBytes"],
            2
        );
        assert_eq!(
            packet_writer["repeatedPatternOptionalSubstreamModelArrayBytes"],
            7
        );
        assert_eq!(
            packet_writer["repeatedPatternOptionalSubstreamModelArraySubstreamCost"],
            16.0
        );
        assert_eq!(
            packet_writer["repeatedPatternOptionalSubstreamModelArrayTotalCost"],
            18.0
        );
        assert_eq!(
            packet_writer["repeatedPatternOptionalSubstreamModelArrayBaselineCost"],
            34.0
        );
        assert_eq!(
            packet_writer["repeatedPatternOptionalSubstreamModelArrayMode"],
            "wrapped_entropy"
        );
        assert_eq!(
            packet_writer["singleSymbolDirectSelfCheckHex"],
            "3000800001AB"
        );
        assert_eq!(
            packet_writer["singleSymbolSplitSelfCheckHex"],
            "2000800003006AC0"
        );
        assert_eq!(
            packet_writer["entropyFinalHeaderSelfCheckHex"],
            "500100000A"
        );
        assert_eq!(
            packet_writer["entropyFinalPacketSelfCheckHex"],
            "5001000002BAAD"
        );

        let varlen_dispatch = &packet_writer["varLenLiteralDispatch"];
        assert_eq!(varlen_dispatch["sourceRva"], "0x6F9F580");
        assert_eq!(varlen_dispatch["minBytes"], 0x60);
        assert_eq!(varlen_dispatch["longThresholdBytes"], 0x600);
        assert_eq!(varlen_dispatch["alternateFlag"], 0x20);
        assert_eq!(varlen_dispatch["alternateCostBase"], 5.0);
        assert_eq!(varlen_dispatch["selfCheckKind"], "alternate");
        assert_eq!(varlen_dispatch["selfCheckEncodedBytes"], 9);
        assert_eq!(varlen_dispatch["selfCheckCost"], 17.0);
        assert_eq!(varlen_dispatch["selfCheckBuilderBaseline"], 30.0);

        let short_split_plan = &packet_writer["shortVarLenLiteralSplitPlan"];
        assert_eq!(short_split_plan["sourceRva"], "0x6F9F760");
        assert_eq!(short_split_plan["packetPrefix"], 0x02);
        assert_eq!(short_split_plan["minSideBytes"], 0x20);
        assert_eq!(short_split_plan["selfCheckLiteralLen"], 0x5FF);
        assert_eq!(
            short_split_plan["selfCheckProbes"],
            serde_json::json!([0xDB, 0x1B6, 0x291, 0x36D, 0x448, 0x523])
        );

        let long_segment_plan = &packet_writer["longVarLenLiteralInitialSegmentPlan"];
        assert_eq!(long_segment_plan["sourceRva"], "0x6F9FE00");
        assert_eq!(long_segment_plan["selfCheckLiteralLen"], KRAKEN_BLOCK_LEN);
        assert_eq!(long_segment_plan["selfCheckParam7"], 9);
        assert_eq!(long_segment_plan["segmentCount"], 0x3E);
        assert_eq!(long_segment_plan["histogramScratchBytes"], 0xF800);
        assert_eq!(long_segment_plan["mergeRecordBytes"], 0x1740);
        assert_eq!(long_segment_plan["firstSegmentLength"], 0x842);
        assert_eq!(long_segment_plan["lastSegmentOffset"], 0x1F7BA);
        assert_eq!(long_segment_plan["lastSegmentLength"], 0x846);

        let alternate_plan = &packet_writer["alternateVarLenLiteralContextPlan"];
        assert_eq!(alternate_plan["sourceRva"], "0x6FA13C0");
        assert_eq!(
            alternate_plan["selfCheckSegmentLengths"],
            serde_json::json!([0x20, 0x40])
        );
        assert_eq!(alternate_plan["selfCheckParam11"], 6);
        assert_eq!(alternate_plan["enabled"], true);
        assert_eq!(alternate_plan["totalLen"], 0x60);
        assert_eq!(alternate_plan["maxSegmentLen"], 0x40);
        assert_eq!(alternate_plan["cap"], 0x20);
        assert_eq!(alternate_plan["doubleCap"], 0x40);
        assert_eq!(alternate_plan["windowFloor"], 0x40);
        assert_eq!(alternate_plan["primaryScratchBytes"], 0x8200);
        assert_eq!(alternate_plan["histogramScratchBytes"], 0x10200);
    }

    #[test]
    fn private_inspection_reports_compressed_state() {
        let value = inspect_private_payload(&[], &stream()).unwrap();

        assert_eq!(value["status"], "native_encoder_in_progress");
        assert_eq!(value["method"], "Oodle");
        assert_eq!(value["algorithmId"], 2);
        assert_eq!(value["writable"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn private_edits_are_blocked_until_native_encoder_exists() {
        let err = apply_private_edits(&[], &stream(), &[PrivateEdit::ReplaceFString]).unwrap_err();

        assert!(matches!(err, CoreError::Codec(_)));
        assert!(err.to_string().contains("native private payload encoder"));
    }

    #[test]
    fn kraken_context_level_five_and_six_match_verified_dispatch() {
        let level_five = KrakenEncoderContext::for_g1r_level(5);
        let level_six = KrakenEncoderContext::for_g1r_level(6);

        assert_eq!(level_five.compressor_id, OODLE_COMPRESSOR_KRAKEN);
        assert_eq!(level_five.block_len, KRAKEN_BLOCK_LEN);
        assert_eq!(
            level_five.primary_block_encoder,
            BlockEncoderKind::KrakenChunkOptimal
        );
        assert_eq!(
            level_five.secondary_helper,
            SecondaryHelperKind::KrakenLevel5
        );
        assert!(!level_five.high_compression_flag);
        assert_eq!(
            level_five.encode_scratch_bytes,
            KRAKEN_OPTIMAL_SCRATCH_BYTES
        );

        assert_eq!(level_six.compressor_id, OODLE_COMPRESSOR_KRAKEN);
        assert_eq!(level_six.block_len, KRAKEN_BLOCK_LEN);
        assert_eq!(
            level_six.primary_block_encoder,
            BlockEncoderKind::KrakenChunkOptimal
        );
        assert_eq!(
            level_six.secondary_helper,
            SecondaryHelperKind::KrakenLevel6Plus
        );
        assert!(level_six.high_compression_flag);
        assert_eq!(level_six.encode_scratch_bytes, KRAKEN_OPTIMAL_SCRATCH_BYTES);
    }

    #[test]
    fn emission_state_layout_for_0x1804_matches_reference_allocator() {
        let layout = EmissionStateLayout::for_capacity(KRAKEN_OPTIMAL_SCRATCH_BYTES);

        assert_eq!(layout.half_plus_header, 0xC0A);
        assert_eq!(layout.third, 0x801);
        assert_eq!(layout.fifth, 0x4CD);
        assert_eq!(layout.byte_table_bytes, 0x60);
        assert_eq!(layout.backing_len, 0x6A54);
        assert_eq!(
            layout.cursor_starts,
            [0, 0x180C, 0x3018, 0x3C22, 0x4424, 0x6428, 0x68F8]
        );
        assert_eq!(layout.byte_table_end, 0x6958);
    }

    #[test]
    fn emission_state_allocates_owned_backing_and_stream_spans() {
        let state = EmissionState::new(KRAKEN_OPTIMAL_SCRATCH_BYTES, 0x20_000, 0xAABBCCDD);

        assert_eq!(state.backing_len(), 0x6A54);
        assert_eq!(state.source_base, 0x20_000);
        assert_eq!(state.user_tag, 0xAABBCCDD);
        assert_eq!(
            state.stream_spans(),
            [
                (0x0000, 0x180C),
                (0x180C, 0x3018),
                (0x3018, 0x3C22),
                (0x3C22, 0x4424),
                (0x4424, 0x6428),
                (0x6428, 0x68F8),
                (0x68F8, 0x6958),
            ]
        );
        assert_eq!(
            state.cursor_offsets(),
            [0x0000, 0x180C, 0x3018, 0x3C22, 0x4424, 0x6428, 0x68F8]
        );
        assert_eq!(state.byte_table_span(), (0x68F8, 0x6958));
        assert_eq!(state.tail_len(), 0xFC);
        assert!(state.backing().iter().all(|byte| *byte == 0));
    }

    #[test]
    fn emission_state_writes_stream_bytes_and_rejects_overflow() {
        let mut state = EmissionState::new(KRAKEN_OPTIMAL_SCRATCH_BYTES, 0, 0);

        state.write_stream(0, &[1, 2, 3]).unwrap();

        assert_eq!(state.cursor_offsets()[0], 3);
        assert_eq!(&state.backing()[0..3], &[1, 2, 3]);

        let fill = vec![0xAA; 0x180C - 3];
        state.write_stream(0, &fill).unwrap();
        assert_eq!(state.cursor_offsets()[0], 0x180C);

        let overflow = state.write_stream(0, &[0xBB]).unwrap_err();
        assert!(matches!(overflow, CoreError::Codec(_)));
        assert!(overflow.to_string().contains("stream 0 overflow"));

        let invalid_stream = state.write_stream(7, &[0xCC]).unwrap_err();
        assert!(matches!(invalid_stream, CoreError::Codec(_)));
        assert!(invalid_stream.to_string().contains("stream 7"));
    }

    #[test]
    fn job_wrapper_runs_callback_inline_without_scheduler_handle() {
        let mut ran = false;
        let mut output_handle = Some(JobHandle(0xDEAD));

        let mode = dispatch_kraken_job(
            &mut output_handle,
            false,
            Some(JobHandle(1)),
            Some(JobHandle(2)),
            None,
            || {
                ran = true;
                Ok(())
            },
        )
        .unwrap();

        assert_eq!(mode, JobDispatchMode::Inline);
        assert!(ran);
        assert_eq!(output_handle, None);
    }

    #[derive(Default)]
    struct RecordingScheduler {
        dependencies: Vec<JobHandle>,
    }

    impl KrakenJobScheduler for RecordingScheduler {
        fn schedule(&mut self, dependencies: &[JobHandle]) -> Result<JobHandle, CoreError> {
            self.dependencies = dependencies.to_vec();
            Ok(JobHandle(0xCAFE))
        }
    }

    #[test]
    fn job_wrapper_schedules_with_compacted_nonzero_dependencies() {
        let mut ran_inline = false;
        let mut output_handle = None;
        let mut scheduler = RecordingScheduler::default();

        let mode = dispatch_kraken_job(
            &mut output_handle,
            true,
            None,
            Some(JobHandle(0x77)),
            Some(&mut scheduler),
            || {
                ran_inline = true;
                Ok(())
            },
        )
        .unwrap();

        assert_eq!(mode, JobDispatchMode::Scheduled);
        assert!(!ran_inline);
        assert_eq!(scheduler.dependencies, vec![JobHandle(0x77)]);
        assert_eq!(output_handle, Some(JobHandle(0xCAFE)));
    }

    #[test]
    fn subjob_plan_matches_reference_window_threshold_and_clamp() {
        let short = SubjobPlan::from_raw_len(0x44000, Some(3), 4).unwrap();
        assert_eq!(short.subjob_count, 1);
        assert_eq!(short.leading_window_bytes, 0x40000);

        let two_windows = SubjobPlan::from_raw_len(0x80000, Some(3), 4).unwrap();
        assert_eq!(two_windows.subjob_count, 2);
        assert_eq!(two_windows.leading_window_bytes, 0x80000);
        assert_eq!(two_windows.scratch_allocation_bytes, 0x1_000000);

        let clamped = SubjobPlan::from_raw_len(0x100000, Some(3), 4).unwrap();
        assert_eq!(clamped.subjob_count, 3);
        assert_eq!(clamped.leading_window_bytes, 0xC0000);

        let no_scheduler = SubjobPlan::from_raw_len(0x100000, None, 4).unwrap();
        assert_eq!(no_scheduler.subjob_count, 1);
    }

    #[test]
    fn subjob_plan_rejects_leading_window_overflow() {
        let err = SubjobPlan::from_raw_len(usize::MAX, Some(usize::MAX), 1).unwrap_err();

        assert!(matches!(err, CoreError::Codec(_)));
        assert!(err.to_string().contains("leading window"));
    }

    #[test]
    fn raw_literal_packet_writes_24bit_length_header_and_payload() {
        let mut output = Vec::new();

        let written = write_raw_literal_packet(&mut output, 16, b"G1R").unwrap();

        assert_eq!(written, 6);
        assert_eq!(output, vec![0x00, 0x00, 0x03, b'G', b'1', b'R']);
    }

    #[test]
    fn raw_literal_packet_rejects_insufficient_output_capacity_without_mutation() {
        let mut output = vec![0xFE];

        let err = write_raw_literal_packet(&mut output, 6, b"G1R").unwrap_err();

        assert!(matches!(err, CoreError::Codec(_)));
        assert!(
            err.to_string()
                .contains("raw literal packet output capacity")
        );
        assert_eq!(output, vec![0xFE]);
    }

    #[test]
    fn raw_literal_packet_rejects_lengths_above_reference_limit() {
        let literal = vec![0; 0x40000];
        let mut output = Vec::new();

        let err = write_raw_literal_packet(&mut output, literal.len() + 3, &literal).unwrap_err();

        assert!(matches!(err, CoreError::Codec(_)));
        assert!(err.to_string().contains("0x3FFFF"));
        assert!(output.is_empty());
    }

    #[test]
    fn short_literal_wrapper_uses_raw_packet_and_reports_cost() {
        let mut output = Vec::new();

        let result = write_short_literal_packet(&mut output, 16, b"G1R").unwrap();

        assert_eq!(result.mode, LiteralPacketMode::RawFallback);
        assert_eq!(result.encoded_bytes, 6);
        assert_eq!(result.cost, 6.0);
        assert_eq!(output, vec![0x00, 0x00, 0x03, b'G', b'1', b'R']);
    }

    #[test]
    fn wrapped_literal_packet_repackages_short_raw_literal_header() {
        let mut output = Vec::new();

        let result = write_wrapped_literal_packet(&mut output, 16, b"G1R").unwrap();

        assert_eq!(result.mode, LiteralPacketMode::WrappedShortRaw);
        assert_eq!(result.encoded_bytes, 5);
        assert_eq!(result.cost, 5.0);
        assert_eq!(output, vec![0x80, 0x03, b'G', b'1', b'R']);
    }

    #[test]
    fn wrapped_literal_packet_with_entropy_encoder_keeps_short_branch_local() {
        let mut output = Vec::new();
        let mut exported_histogram = [0xFFFF_FFFFu32; 256];
        let mut encoder = RecordingLiteralEntropyPacketEncoder::default();

        let result = write_wrapped_literal_packet_with_entropy_encoder(
            &mut output,
            16,
            b"G1R",
            Some(&mut exported_histogram),
            &mut encoder,
        )
        .unwrap();

        assert_eq!(result.mode, LiteralPacketMode::WrappedShortRaw);
        assert_eq!(output, vec![0x80, 0x03, b'G', b'1', b'R']);
        assert!(encoder.calls.is_empty());
        assert_eq!(exported_histogram[0], 0xFFFF_FFFF);
    }

    #[test]
    fn wrapped_literal_packet_with_entropy_encoder_builds_histogram_and_compacts_long_branch() {
        let literal = vec![0xAB; SHORT_LITERAL_RAW_MAX_BYTES + 1];
        let mut output = Vec::new();
        let mut exported_histogram = [0u32; 256];
        let mut encoder = RecordingLiteralEntropyPacketEncoder::default();

        let result = write_wrapped_literal_packet_with_entropy_encoder(
            &mut output,
            16,
            &literal,
            Some(&mut exported_histogram),
            &mut encoder,
        )
        .unwrap();

        assert_eq!(encoder.calls, vec![(0x21, 0x21, 0)]);
        assert_eq!(exported_histogram[0xAB], 0x21);
        assert_eq!(exported_histogram[0xCD], 0);
        assert_eq!(result.mode, LiteralPacketMode::WrappedEntropy);
        assert_eq!(result.encoded_bytes, 4);
        assert_eq!(result.cost, 12.0);
        assert_eq!(output, vec![0xB0, 0x7C, 0x01, 0xAB]);
    }

    #[test]
    fn literal_packet_span_decodes_raw_and_compact_short_forms() {
        let raw = [0x00, 0x00, 0x03, b'G', b'1', b'R'];
        let compact = [0x80, 0x03, b'G', b'1', b'R'];

        let raw_span = decode_literal_packet_span(&raw, KRAKEN_BLOCK_LEN).unwrap();
        let compact_span = decode_literal_packet_span(&compact, KRAKEN_BLOCK_LEN).unwrap();

        assert_eq!(
            raw_span,
            LiteralPacketSpan {
                header_nibble: 0,
                decoded_bytes: 3,
                payload_bytes: 3,
                encoded_bytes: 6,
            }
        );
        assert_eq!(
            compact_span,
            LiteralPacketSpan {
                header_nibble: 8,
                decoded_bytes: 3,
                payload_bytes: 3,
                encoded_bytes: 5,
            }
        );
    }

    #[test]
    fn literal_packet_span_decodes_entropy_and_compact_entropy_forms() {
        let entropy = [0x30, 0x00, 0x80, 0x00, 0x01, 0xAB];
        let compact = [0xB0, 0x7C, 0x01, 0xAB];

        let entropy_span = decode_literal_packet_span(&entropy, KRAKEN_BLOCK_LEN).unwrap();
        let compact_span = decode_literal_packet_span(&compact, KRAKEN_BLOCK_LEN).unwrap();

        assert_eq!(
            entropy_span,
            LiteralPacketSpan {
                header_nibble: 3,
                decoded_bytes: 0x21,
                payload_bytes: 1,
                encoded_bytes: 6,
            }
        );
        assert_eq!(
            compact_span,
            LiteralPacketSpan {
                header_nibble: 0xB,
                decoded_bytes: 0x21,
                payload_bytes: 1,
                encoded_bytes: 4,
            }
        );
    }

    #[test]
    fn literal_packet_span_rejects_truncated_packet_without_guessing() {
        let truncated = [0x30, 0x00, 0x80, 0x00, 0x01];

        let err = decode_literal_packet_span(&truncated, KRAKEN_BLOCK_LEN).unwrap_err();

        assert!(matches!(err, CoreError::Codec(_)));
        assert!(err.to_string().contains("literal packet span"));
    }

    #[test]
    fn wrapped_literal_packet_repackages_entropy_header() {
        let literal = vec![0xAB; SHORT_LITERAL_RAW_MAX_BYTES + 1];
        let mut output = Vec::new();
        let result = write_entropy_single_symbol_packet(
            &mut output,
            16,
            &literal,
            SingleSymbolPacketMode::DirectSymbol,
            4.0,
            2.0,
            100.0,
        )
        .unwrap();

        let wrapped = compact_wrapped_literal_packet(&mut output, 0, result).unwrap();

        assert_eq!(wrapped.mode, LiteralPacketMode::WrappedEntropy);
        assert_eq!(wrapped.encoded_bytes, 4);
        assert_eq!(wrapped.cost, 12.0);
        assert_eq!(output, vec![0xB0, 0x7C, 0x01, 0xAB]);
    }

    #[test]
    fn literal_histogram_clears_then_counts_low_length_scalar_path() {
        let mut histogram = [99u32; 256];

        build_literal_histogram(b"AABA\xFF", &mut histogram, true);

        assert_eq!(histogram[b'A' as usize], 3);
        assert_eq!(histogram[b'B' as usize], 1);
        assert_eq!(histogram[0xFF], 1);
        assert_eq!(histogram[b'C' as usize], 0);
    }

    #[test]
    fn literal_histogram_accumulates_without_clear() {
        let mut histogram = [0u32; 256];
        histogram[b'A' as usize] = 10;
        histogram[b'B' as usize] = 1;

        build_literal_histogram(b"ABBA", &mut histogram, false);

        assert_eq!(histogram[b'A' as usize], 12);
        assert_eq!(histogram[b'B' as usize], 3);
    }

    #[test]
    fn short_literal_wrapper_accepts_reference_0x20_boundary() {
        let literal = vec![0xAB; SHORT_LITERAL_RAW_MAX_BYTES];
        let mut output = Vec::new();

        let result = write_short_literal_packet(&mut output, 64, &literal).unwrap();

        assert_eq!(result.mode, LiteralPacketMode::RawFallback);
        assert_eq!(result.encoded_bytes, 0x23);
        assert_eq!(result.cost, 35.0);
        assert_eq!(&output[..3], &[0x00, 0x00, 0x20]);
        assert_eq!(&output[3..], literal.as_slice());
    }

    #[test]
    fn short_literal_branch_rejects_lengths_outside_reference_short_limit_without_mutation() {
        let literal = vec![0xCD; SHORT_LITERAL_RAW_MAX_BYTES + 1];
        let mut output = vec![0xEE];

        let err = write_short_literal_packet(&mut output, 64, &literal).unwrap_err();

        assert!(matches!(err, CoreError::Codec(_)));
        assert!(err.to_string().contains("0x6F91E80"));
        assert_eq!(output, vec![0xEE]);
    }

    #[test]
    fn entropy_single_symbol_direct_packet_uses_0x80_form() {
        let literal = vec![0xAB; SHORT_LITERAL_RAW_MAX_BYTES + 1];
        let mut output = Vec::new();

        let result = write_entropy_single_symbol_packet(
            &mut output,
            16,
            &literal,
            SingleSymbolPacketMode::DirectSymbol,
            4.0,
            2.0,
            100.0,
        )
        .unwrap();

        assert_eq!(result.mode, LiteralPacketMode::SingleSymbolDirect);
        assert_eq!(result.encoded_bytes, 6);
        assert_eq!(result.cost, 14.0);
        assert_eq!(output, vec![0x30, 0x00, 0x80, 0x00, 0x01, 0xAB]);
    }

    #[test]
    fn entropy_single_symbol_split_packet_uses_first_symbol_bits() {
        let literal = vec![0xAB; SHORT_LITERAL_RAW_MAX_BYTES + 1];
        let mut output = Vec::new();

        let result = write_entropy_single_symbol_packet(
            &mut output,
            16,
            &literal,
            SingleSymbolPacketMode::SplitFirstSymbol,
            4.0,
            2.0,
            100.0,
        )
        .unwrap();

        assert_eq!(result.mode, LiteralPacketMode::SingleSymbolSplit);
        assert_eq!(result.encoded_bytes, 8);
        assert_eq!(result.cost, 16.0);
        assert_eq!(output, vec![0x20, 0x00, 0x80, 0x00, 0x03, 0x00, 0x6A, 0xC0]);
    }

    #[test]
    fn entropy_model_array_candidate_is_skipped_below_0x20() {
        let literal = vec![b'A'; 0x1F];
        let mut histogram = [0u32; 256];
        build_literal_histogram(&literal, &mut histogram, true);
        let mut output = vec![0xEE];
        let mut builder = RecordingLiteralEntropyModelBuilder::default();

        let result = write_entropy_model_array_candidate_packet(
            &mut output,
            64,
            &literal,
            &histogram,
            100.0,
            &mut builder,
        )
        .unwrap();

        assert_eq!(result, None);
        assert!(builder.calls.is_empty());
        assert_eq!(output, vec![0xEE]);
    }

    #[test]
    fn entropy_model_array_candidate_uses_raw_baseline_and_finalizes_header() {
        let literal = b"AAAABBBBCCCCDDDDEEEEFFFFGGGGHHHHIIIIJJJJKKKKLLLLMMMMNNNNOOOOPPPP";
        let mut histogram = [0u32; 256];
        build_literal_histogram(literal, &mut histogram, true);
        let mut output = Vec::new();
        let mut builder = RecordingLiteralEntropyModelBuilder {
            result: Some(LiteralEntropyModelCandidate {
                mode_id: 4,
                payload: vec![0xDE, 0xAD],
                cost: 20.0,
            }),
            ..RecordingLiteralEntropyModelBuilder::default()
        };

        let result = write_entropy_model_array_candidate_packet(
            &mut output,
            16,
            literal,
            &histogram,
            1000.0,
            &mut builder,
        )
        .unwrap()
        .unwrap();

        assert_eq!(builder.calls, vec![(0x40, 67.0, 4, 4)]);
        assert_eq!(result.mode, LiteralPacketMode::EntropyCandidate);
        assert_eq!(result.encoded_bytes, 7);
        assert_eq!(result.cost, 20.0);
        assert_eq!(output, vec![0x40, 0x00, 0xFC, 0x00, 0x02, 0xDE, 0xAD]);
    }

    #[test]
    fn model_array_plain_small_header_body_packs_symbol_and_code_length_fields() {
        let plan = model_array_side_header_plan(3, 3, 0).unwrap().unwrap();

        let header = write_model_array_plain_small_side_header(
            plan,
            LiteralEntropyModelArrayPlainSmallHeader {
                alphabet_size: 4,
                symbol_count: 3,
                single_symbol_index: None,
                max_code_len: 4,
                code_lengths: vec![1, 2, 0, 3],
            },
        )
        .unwrap();

        assert_eq!(header.path, "plain_small");
        assert_eq!(header.symbol_index_bits, 2);
        assert_eq!(header.code_len_bits, Some(2));
        assert_eq!(header.nonzero_symbols, 3);
        assert_eq!(header.bit_len, 18);
        assert_eq!(bytes_to_upper_hex(&header.bytes), "681780");
    }

    #[test]
    fn model_array_plain_large_preamble_scores_residual_histogram() {
        let plan = model_array_side_header_plan(5, 8, 0).unwrap().unwrap();

        let preamble = write_model_array_plain_large_side_header_preamble(
            plan,
            LiteralEntropyModelArrayPlainLargeHeader {
                alphabet_size: 8,
                symbol_count: 5,
                code_lengths: vec![1, 2, 0, 3, 4, 0, 2, 1],
            },
        )
        .unwrap();

        assert_eq!(preamble.path, "plain_large_preamble");
        assert_eq!(preamble.symbol_index_bits, 3);
        assert_eq!(preamble.selected_predictor, 1);
        assert_eq!(preamble.predictor_scores, [16, 15, 18, 24]);
        assert_eq!(preamble.residuals, vec![3, 1, 0, 2, 1, 3]);
        assert_eq!(&preamble.residual_histogram[..4], &[1, 2, 1, 2]);
        assert_eq!(preamble.nonzero_symbols, 6);
        assert_eq!(preamble.first_symbol_has_code_len, true);
        assert_eq!(preamble.bit_len, 5);
        assert_eq!(bytes_to_upper_hex(&preamble.bytes), "58");
    }

    #[test]
    fn model_array_plain_large_body_encodes_run_descriptors_and_residual_codes() {
        let plan = model_array_side_header_plan(5, 8, 0).unwrap().unwrap();

        let body = write_model_array_plain_large_side_header_body(
            plan,
            LiteralEntropyModelArrayPlainLargeHeader {
                alphabet_size: 8,
                symbol_count: 5,
                code_lengths: vec![1, 2, 0, 3, 4, 0, 2, 1],
            },
        )
        .unwrap();

        assert_eq!(body.path, "plain_large_body");
        assert_eq!(body.selected_predictor, 1);
        assert_eq!(body.runs.len(), 5);
        assert_eq!(body.runs[0].kind, "nonzero");
        assert_eq!(body.runs[0].len, 2);
        assert_eq!(body.runs[0].descriptor_bits, "11");
        assert_eq!(body.runs[0].residual_bits, vec!["011", "11"]);
        assert_eq!(body.runs[1].kind, "zero");
        assert_eq!(body.runs[1].len, 1);
        assert_eq!(body.runs[1].descriptor_bits, "10");
        assert_eq!(body.runs[2].residual_bits, vec!["10", "010"]);
        assert_eq!(body.runs[4].residual_bits, vec!["11", "011"]);
        assert_eq!(body.bit_len, 30);
        assert_eq!(bytes_to_upper_hex(&body.bytes), "5EFB95EC");
    }

    #[test]
    fn entropy_table_candidate_is_skipped_below_0x20() {
        let literal = vec![b'A'; 0x1F];
        let mut histogram = [0u32; 256];
        build_literal_histogram(&literal, &mut histogram, true);
        let original_histogram = histogram;
        let mut output = vec![0xEE];
        let mut builder = RecordingLiteralEntropyTableBuilder::default();

        let result = write_entropy_table_candidate_packet(
            &mut output,
            64,
            &literal,
            &histogram,
            1000.0,
            &mut builder,
        )
        .unwrap();

        assert_eq!(result, None);
        assert!(builder.calls.is_empty());
        assert_eq!(histogram, original_histogram);
        assert_eq!(output, vec![0xEE]);
    }

    #[test]
    fn entropy_table_candidate_subtracts_tail_and_finalizes_mode_one_header() {
        let mut literal = vec![b'A'; 59];
        literal.extend_from_slice(b"BCDEF");
        let mut histogram = [0u32; 256];
        build_literal_histogram(&literal, &mut histogram, true);
        let original_histogram = histogram;
        let mut output = Vec::new();
        let mut builder = RecordingLiteralEntropyTableBuilder {
            result: Some(LiteralEntropyTableCandidate {
                state_count: 2,
                payload: vec![0xFE, 0xED, 0xFA],
                cost: 21.0,
            }),
            ..RecordingLiteralEntropyTableBuilder::default()
        };

        let result = write_entropy_table_candidate_packet(
            &mut output,
            16,
            &literal,
            &histogram,
            1000.0,
            &mut builder,
        )
        .unwrap()
        .unwrap();

        assert_eq!(builder.calls, vec![(0x40, 59, 8, 66, 59, 0)]);
        assert_eq!(histogram, original_histogram);
        assert_eq!(result.mode, LiteralPacketMode::EntropyCandidate);
        assert_eq!(result.encoded_bytes, 8);
        assert_eq!(result.cost, 21.0);
        assert_eq!(output, vec![0x10, 0x00, 0xFC, 0x00, 0x03, 0xFE, 0xED, 0xFA]);
    }

    #[test]
    fn entropy_table_candidate_rejects_builder_state_count_below_two() {
        let mut literal = vec![b'A'; 59];
        literal.extend_from_slice(b"BCDEF");
        let mut histogram = [0u32; 256];
        build_literal_histogram(&literal, &mut histogram, true);
        let mut output = vec![0xEE];
        let mut builder = RecordingLiteralEntropyTableBuilder {
            result: Some(LiteralEntropyTableCandidate {
                state_count: 1,
                payload: vec![0xFE, 0xED, 0xFA],
                cost: 21.0,
            }),
            ..RecordingLiteralEntropyTableBuilder::default()
        };

        let result = write_entropy_table_candidate_packet(
            &mut output,
            16,
            &literal,
            &histogram,
            1000.0,
            &mut builder,
        )
        .unwrap();

        assert_eq!(result, None);
        assert_eq!(output, vec![0xEE]);
    }

    #[test]
    fn entropy_repeated_pattern_candidate_limits_budget_and_finalizes_mode_three_header() {
        let literal = vec![b'A'; 0x40];
        let mut output = Vec::new();
        let mut builder = RecordingLiteralEntropyRepeatedPatternBuilder {
            result: Some(LiteralEntropyRepeatedPatternCandidate {
                payload: vec![0xFA, 0xCE, 0x01],
                cost: 20.0,
            }),
            ..RecordingLiteralEntropyRepeatedPatternBuilder::default()
        };

        let result = write_entropy_repeated_pattern_candidate_packet(
            &mut output,
            16,
            &literal,
            100.0,
            4.0,
            2.0,
            &mut builder,
        )
        .unwrap()
        .unwrap();

        assert_eq!(builder.calls, vec![(0x40, 11, 67.0, 13.0)]);
        assert_eq!(result.mode, LiteralPacketMode::EntropyCandidate);
        assert_eq!(result.encoded_bytes, 8);
        assert_eq!(result.cost, 20.0);
        assert_eq!(output, vec![0x30, 0x00, 0xFC, 0x00, 0x03, 0xFA, 0xCE, 0x01]);
    }

    #[test]
    fn entropy_repeated_pattern_candidate_skips_when_budget_is_not_positive() {
        let literal = vec![b'A'; 0x40];
        let mut output = vec![0xEE];
        let mut builder = RecordingLiteralEntropyRepeatedPatternBuilder::default();

        let result = write_entropy_repeated_pattern_candidate_packet(
            &mut output,
            16,
            &literal,
            12.0,
            4.0,
            2.0,
            &mut builder,
        )
        .unwrap();

        assert_eq!(result, None);
        assert!(builder.calls.is_empty());
        assert_eq!(output, vec![0xEE]);
    }

    #[test]
    fn repeated_pattern_payload_encodes_single_long_run_with_symbol_marker() {
        let literal = vec![b'A'; 0x20];

        let payload = encode_repeated_byte_pattern_payload(&literal, 0x20)
            .unwrap()
            .unwrap();

        assert_eq!(payload.data_bytes, 1);
        assert_eq!(payload.control_bytes, 3);
        assert_eq!(payload.payload, vec![0x00, b'A', 0x00, 0x18, 0x01]);
    }

    #[test]
    fn repeated_pattern_payload_treats_search_boundary_run_as_tail_raw() {
        let mut literal = b"abcdefghijklmn".to_vec();
        literal.extend(std::iter::repeat_n(b'A', 0x12));

        let payload = encode_repeated_byte_pattern_payload(&literal, 0x40)
            .unwrap()
            .unwrap();

        let mut expected = vec![0x00];
        expected.extend_from_slice(&literal);
        expected.extend_from_slice(&[0x20, 0x10]);
        assert_eq!(payload.data_bytes, 0x20);
        assert_eq!(payload.control_bytes, 2);
        assert_eq!(payload.payload, expected);
    }

    #[test]
    fn repeated_pattern_payload_requires_reference_gap_before_run() {
        let literal = vec![b'A'; 0x20];

        let too_tight = encode_repeated_byte_pattern_payload(&literal, 5).unwrap();
        let enough_gap = encode_repeated_byte_pattern_payload(&literal, 0x13)
            .unwrap()
            .unwrap();

        assert_eq!(too_tight, None);
        assert_eq!(enough_gap.payload, vec![0x00, b'A', 0x00, 0x18, 0x01]);
    }

    #[test]
    fn repeated_pattern_payload_requires_reference_gap_before_tail_raw() {
        let literal = b"abcdefghijklmnopqrstuvwxyzABCDEF".to_vec();

        let too_tight = encode_repeated_byte_pattern_payload(&literal, 0x23).unwrap();
        let enough_gap = encode_repeated_byte_pattern_payload(&literal, 0x31)
            .unwrap()
            .unwrap();

        let mut expected = vec![0x00];
        expected.extend_from_slice(&literal);
        expected.extend_from_slice(&[0x20, 0x10]);
        assert_eq!(too_tight, None);
        assert_eq!(enough_gap.payload, expected);
    }

    #[test]
    fn native_repeated_pattern_builder_uses_payload_len_plus_pre_cost() {
        let literal = vec![b'A'; 0x20];
        let mut output = Vec::new();
        let mut builder = NativeLiteralEntropyRepeatedPatternBuilder;

        let result = write_entropy_repeated_pattern_candidate_packet(
            &mut output,
            24,
            &literal,
            100.0,
            4.0,
            2.0,
            &mut builder,
        )
        .unwrap()
        .unwrap();

        assert_eq!(result.mode, LiteralPacketMode::EntropyCandidate);
        assert_eq!(result.encoded_bytes, 10);
        assert_eq!(result.cost, 18.0);
        assert_eq!(
            output,
            vec![0x30, 0x00, 0x7C, 0x00, 0x05, 0x00, b'A', 0x00, 0x18, 0x01]
        );
    }

    #[test]
    fn entropy_single_symbol_packet_rejects_mixed_literals_without_mutation() {
        let literal = [0xAB, 0xAB, 0xCD, 0xAB, 0xAB, 0xAB];
        let mut output = vec![0xEE];

        let err = write_entropy_single_symbol_packet(
            &mut output,
            16,
            &literal,
            SingleSymbolPacketMode::DirectSymbol,
            1.0,
            1.0,
            100.0,
        )
        .unwrap_err();

        assert!(matches!(err, CoreError::Codec(_)));
        assert!(err.to_string().contains("all bytes"));
        assert_eq!(output, vec![0xEE]);
    }

    #[test]
    fn entropy_single_symbol_packet_rejects_cost_that_is_not_better_without_mutation() {
        let literal = vec![0xAB; SHORT_LITERAL_RAW_MAX_BYTES + 1];
        let mut output = vec![0xEE];

        let err = write_entropy_single_symbol_packet(
            &mut output,
            16,
            &literal,
            SingleSymbolPacketMode::DirectSymbol,
            4.0,
            2.0,
            14.0,
        )
        .unwrap_err();

        assert!(matches!(err, CoreError::Codec(_)));
        assert!(err.to_string().contains("not better"));
        assert_eq!(output, vec![0xEE]);
    }

    #[test]
    fn entropy_single_symbol_packet_rejects_insufficient_capacity_without_mutation() {
        let literal = vec![0xAB; SHORT_LITERAL_RAW_MAX_BYTES + 1];
        let mut output = vec![0xEE];

        let err = write_entropy_single_symbol_packet(
            &mut output,
            7,
            &literal,
            SingleSymbolPacketMode::SplitFirstSymbol,
            1.0,
            1.0,
            100.0,
        )
        .unwrap_err();

        assert!(matches!(err, CoreError::Codec(_)));
        assert!(err.to_string().contains("output capacity"));
        assert_eq!(output, vec![0xEE]);
    }

    #[test]
    fn varlen_literal_entropy_dispatch_rejects_0x5f_boundary_without_builder_calls() {
        let mut builder = RecordingVarLenLiteralEntropyBuilder::default();
        let mut candidate_cost = 123.0;

        let result =
            dispatch_varlen_literal_entropy(0x5F, 0, 77.0, &mut candidate_cost, &mut builder)
                .unwrap();

        assert_eq!(result, None);
        assert_eq!(candidate_cost, 123.0);
        assert!(builder.short_calls.is_empty());
        assert!(builder.long_calls.is_empty());
        assert!(builder.alternate_calls.is_empty());
    }

    #[test]
    fn varlen_literal_entropy_dispatch_uses_short_builder_below_0x600() {
        let mut builder = RecordingVarLenLiteralEntropyBuilder {
            short_result: Some(VarLenLiteralEntropyCandidate {
                encoded_bytes: 17,
                cost: 42.25,
            }),
            ..RecordingVarLenLiteralEntropyBuilder::default()
        };
        let mut candidate_cost = 1000.0;

        let result =
            dispatch_varlen_literal_entropy(0x60, 0, 88.0, &mut candidate_cost, &mut builder)
                .unwrap();

        assert_eq!(
            result,
            Some(VarLenLiteralEntropyDispatch {
                kind: VarLenLiteralEntropyKind::ShortRange,
                encoded_bytes: 17,
                cost: 42.25,
            })
        );
        assert_eq!(candidate_cost, 42.25);
        assert_eq!(builder.short_calls, vec![0x60]);
        assert!(builder.long_calls.is_empty());
        assert!(builder.alternate_calls.is_empty());
    }

    #[test]
    fn varlen_literal_entropy_dispatch_uses_long_builder_at_0x600() {
        let mut builder = RecordingVarLenLiteralEntropyBuilder {
            long_result: Some(VarLenLiteralEntropyCandidate {
                encoded_bytes: 31,
                cost: 64.5,
            }),
            ..RecordingVarLenLiteralEntropyBuilder::default()
        };
        let mut candidate_cost = 1000.0;

        let result =
            dispatch_varlen_literal_entropy(0x600, 0, 88.0, &mut candidate_cost, &mut builder)
                .unwrap();

        assert_eq!(
            result,
            Some(VarLenLiteralEntropyDispatch {
                kind: VarLenLiteralEntropyKind::LongRange,
                encoded_bytes: 31,
                cost: 64.5,
            })
        );
        assert_eq!(candidate_cost, 64.5);
        assert!(builder.short_calls.is_empty());
        assert_eq!(builder.long_calls, vec![0x600]);
        assert!(builder.alternate_calls.is_empty());
    }

    #[test]
    fn varlen_literal_entropy_dispatch_uses_alternate_flag_and_adds_cost_base() {
        let mut builder = RecordingVarLenLiteralEntropyBuilder {
            short_result: Some(VarLenLiteralEntropyCandidate {
                encoded_bytes: 17,
                cost: 40.0,
            }),
            alternate_result: Some(VarLenLiteralEntropyCandidate {
                encoded_bytes: 9,
                cost: 12.0,
            }),
            ..RecordingVarLenLiteralEntropyBuilder::default()
        };
        let mut candidate_cost = 1000.0;

        let result =
            dispatch_varlen_literal_entropy(0x100, 0x20, 35.0, &mut candidate_cost, &mut builder)
                .unwrap();

        assert_eq!(
            result,
            Some(VarLenLiteralEntropyDispatch {
                kind: VarLenLiteralEntropyKind::Alternate,
                encoded_bytes: 9,
                cost: 17.0,
            })
        );
        assert_eq!(candidate_cost, 17.0);
        assert_eq!(builder.short_calls, vec![0x100]);
        assert!(builder.long_calls.is_empty());
        assert_eq!(builder.alternate_calls, vec![(0x100, 30.0)]);
    }

    #[test]
    fn short_varlen_literal_split_plan_matches_reference_probe_schedule() {
        let plan = short_varlen_literal_split_plan(0x5FF).unwrap();

        assert_eq!(plan.packet_prefix, 0x02);
        assert_eq!(plan.min_side_bytes, 0x20);
        assert_eq!(plan.probes, vec![0xDB, 0x1B6, 0x291, 0x36D, 0x448, 0x523]);
    }

    #[test]
    fn long_varlen_literal_initial_segment_plan_matches_reference_caps_and_chunks() {
        let plan = long_varlen_literal_initial_segment_plan(0x600, 6).unwrap();

        assert_eq!(plan.segment_count, 3);
        assert_eq!(plan.histogram_scratch_bytes, 0xC00);
        assert_eq!(plan.merge_record_bytes, 0x120);
        assert_eq!(plan.segment_offsets, vec![0, 0x200, 0x400]);
        assert_eq!(plan.segment_lengths, vec![0x200, 0x200, 0x200]);

        let param9_plan = long_varlen_literal_initial_segment_plan(KRAKEN_BLOCK_LEN, 9).unwrap();

        assert_eq!(param9_plan.segment_count, 0x3E);
        assert_eq!(param9_plan.histogram_scratch_bytes, 0xF800);
        assert_eq!(param9_plan.merge_record_bytes, 0x1740);
        assert_eq!(param9_plan.segment_offsets[0], 0);
        assert_eq!(param9_plan.segment_lengths[0], 0x842);
        assert_eq!(*param9_plan.segment_offsets.last().unwrap(), 0x1F7BA);
        assert_eq!(*param9_plan.segment_lengths.last().unwrap(), 0x846);
    }

    #[test]
    fn alternate_varlen_literal_context_plan_matches_reference_guards_and_scratch() {
        let plan = alternate_varlen_literal_context_plan(&[0x20, 0x40], 6).unwrap();

        assert!(plan.enabled);
        assert_eq!(plan.total_len, 0x60);
        assert_eq!(plan.max_segment_len, 0x40);
        assert_eq!(plan.cap, 0x20);
        assert_eq!(plan.double_cap, 0x40);
        assert_eq!(plan.window_floor, 0x40);
        assert_eq!(plan.primary_scratch_bytes, 0x8200);
        assert_eq!(plan.histogram_scratch_bytes, 0x10200);

        let param9_plan =
            alternate_varlen_literal_context_plan(&[KRAKEN_BLOCK_LEN / 2, KRAKEN_BLOCK_LEN / 2], 9)
                .unwrap();

        assert!(param9_plan.enabled);
        assert_eq!(param9_plan.cap, 0x3E);
        assert_eq!(param9_plan.double_cap, 0x7C);
        assert_eq!(param9_plan.window_floor, 0x51E);
        assert_eq!(param9_plan.primary_scratch_bytes, 0xFA00);
        assert_eq!(param9_plan.histogram_scratch_bytes, 0x1F3E0);
    }
}
