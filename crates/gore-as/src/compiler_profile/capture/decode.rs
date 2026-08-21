use std::collections::{BTreeMap, BTreeSet};

use sha2::{Digest as _, Sha256};

use super::super::frontend::{ClassGeneratorConfigV1, CompilerOptionsV1, PreprocessorConfigV1};
use super::super::manifest::Sha256Digest;
use super::super::registry::{
    EnginePropertySettingV1, OrderedEnginePropertiesV1, PostBindEntryV1, PostBindResultV1,
    PostBindSnapshotV1, PostBindStateV1, RegistrationEntryV1, RegistrationTraceV1,
    ENGINE_PROPERTIES_SCHEMA, POST_BIND_SNAPSHOT_SCHEMA, REGISTRATION_TRACE_SCHEMA,
    REGISTRY_SCHEMA_VERSION,
};
use super::model::*;

const ENGINE_PROPERTY_PAYLOAD_BYTES: usize = 24;
const POINTER_TOKEN_PAYLOAD_BYTES: usize = 12;
const BIND_CALLBACK_PAYLOAD_BYTES: usize = 88;
const BUILD_JIT_PAYLOAD_BYTES: usize = 48;
const FRONTEND_BOUNDARY_PAYLOAD_BYTES: usize = 112;

pub fn decode_capture_v1(bytes: &[u8]) -> Result<DecodedCaptureV1, CaptureDecodeError> {
    if bytes.len() > MAX_CAPTURE_BYTES_V1 {
        return Err(CaptureDecodeError::InputTooLarge {
            actual: bytes.len(),
            max: MAX_CAPTURE_BYTES_V1,
        });
    }
    if bytes.len() < CAPTURE_HEADER_BYTES_V1 + CAPTURE_FOOTER_BYTES_V1 {
        return Err(CaptureDecodeError::Truncated {
            at: bytes.len(),
            needed: CAPTURE_HEADER_BYTES_V1 + CAPTURE_FOOTER_BYTES_V1,
        });
    }

    let header = parse_header(&bytes[..CAPTURE_HEADER_BYTES_V1])?;
    let stream_end = bytes.len() - CAPTURE_FOOTER_BYTES_V1;
    let footer = parse_footer(&bytes[stream_end..], stream_end)?;
    let mut hasher = Sha256::new();
    hasher.update(CAPTURE_HASH_DOMAIN_V1);
    hasher.update(&bytes[..stream_end]);
    let computed: [u8; 32] = hasher.finalize().into();
    if computed != footer.sha256 {
        return Err(CaptureDecodeError::DigestMismatch);
    }

    let mut decoder = StreamDecoder::new(header);
    let mut cursor = Cursor::new(&bytes[CAPTURE_HEADER_BYTES_V1..stream_end]);
    while !cursor.is_empty() {
        let record_offset = CAPTURE_HEADER_BYTES_V1 + cursor.position();
        if decoder.record_count >= MAX_CAPTURE_RECORDS_V1 {
            return Err(CaptureDecodeError::TooManyRecords {
                max: MAX_CAPTURE_RECORDS_V1,
            });
        }
        let kind_raw = cursor.u16()?;
        let kind_version = cursor.u16()?;
        let flags = cursor.u32()?;
        let payload_len = cursor.u32()? as usize;
        let reserved = cursor.u32()?;
        let ordinal = cursor.u64()?;
        if kind_version != CAPTURE_SCHEMA_VERSION_V1 || flags != 0 || reserved != 0 {
            return Err(CaptureDecodeError::RecordHeader {
                offset: record_offset,
            });
        }
        if ordinal != decoder.record_count {
            return Err(CaptureDecodeError::RecordOrdinal {
                expected: decoder.record_count,
                actual: ordinal,
            });
        }
        if payload_len > MAX_CAPTURE_RECORD_PAYLOAD_V1 {
            return Err(CaptureDecodeError::RecordTooLarge {
                actual: payload_len,
                max: MAX_CAPTURE_RECORD_PAYLOAD_V1,
            });
        }
        let kind =
            RecordKindV1::parse(kind_raw).ok_or(CaptureDecodeError::UnknownRecordKind(kind_raw))?;
        let payload = cursor.take(payload_len)?;
        decoder.apply(kind, ordinal, payload)?;
        decoder.record_count += 1;
    }
    if decoder.record_count != footer.record_count {
        return Err(CaptureDecodeError::FooterRecordCount {
            declared: footer.record_count,
            actual: decoder.record_count,
        });
    }
    decoder.finish(Sha256Digest::from_bytes(footer.sha256))
}

#[derive(Debug, Clone, Copy)]
struct CaptureFooterV1 {
    record_count: u64,
    sha256: [u8; 32],
}

fn parse_header(bytes: &[u8]) -> Result<CaptureHeaderV1, CaptureDecodeError> {
    let mut cursor = Cursor::new(bytes);
    expect_bytes("header magic", cursor.take(8)?, CAPTURE_MAGIC_V1)?;
    expect_u16(
        "capture schema version",
        cursor.u16()?,
        CAPTURE_SCHEMA_VERSION_V1,
    )?;
    expect_u16(
        "capture header length",
        cursor.u16()?,
        CAPTURE_HEADER_BYTES_V1 as u16,
    )?;
    expect_u32("header flags", cursor.u32()?, 0)?;
    expect_u32("Steam app id", cursor.u32()?, PINNED_STEAM_APP_ID)?;
    expect_u64("Steam build id", cursor.u64()?, PINNED_STEAM_BUILD_ID)?;
    expect_u32(
        "AngelScript version",
        cursor.u32()?,
        PINNED_ANGELSCRIPT_VERSION,
    )?;
    expect_u64(
        "executable byte length",
        cursor.u64()?,
        PINNED_EXECUTABLE_BYTES,
    )?;
    expect_bytes(
        "executable sha256",
        cursor.take(32)?,
        &PINNED_EXECUTABLE_SHA256,
    )?;
    expect_bytes(
        "CodeView guid",
        cursor.take(16)?,
        &PINNED_CODEVIEW_GUID_RSDS,
    )?;
    expect_u32("CodeView age", cursor.u32()?, PINNED_CODEVIEW_AGE)?;
    let capture_id: [u8; 16] = cursor.take(16)?.try_into().expect("fixed length");
    if capture_id == [0; 16] {
        return Err(CaptureDecodeError::Header("capture id is zero"));
    }
    expect_u32("header reserved field", cursor.u32()?, 0)?;
    if !cursor.is_empty() {
        return Err(CaptureDecodeError::Header("header has trailing bytes"));
    }
    Ok(CaptureHeaderV1 { capture_id })
}

fn parse_footer(
    bytes: &[u8],
    expected_stream_bytes: usize,
) -> Result<CaptureFooterV1, CaptureDecodeError> {
    let mut cursor = Cursor::new(bytes);
    expect_bytes("footer magic", cursor.take(8)?, CAPTURE_FOOTER_MAGIC_V1)?;
    let record_count = cursor.u64()?;
    expect_u64(
        "sealed stream length",
        cursor.u64()?,
        expected_stream_bytes as u64,
    )?;
    let sha256 = cursor.take(32)?.try_into().expect("fixed length");
    expect_u32(
        "footer schema version",
        cursor.u32()?,
        CAPTURE_SCHEMA_VERSION_V1 as u32,
    )?;
    expect_u32("footer reserved field", cursor.u32()?, 0)?;
    if !cursor.is_empty() {
        return Err(CaptureDecodeError::Footer("footer has trailing bytes"));
    }
    Ok(CaptureFooterV1 {
        record_count,
        sha256,
    })
}

struct StreamDecoder {
    header: CaptureHeaderV1,
    record_count: u64,
    engine_properties: Vec<CapturedEnginePropertyV1>,
    pointer_tokens: BTreeMap<u32, PointerTokenV1>,
    pointer_rvas: BTreeSet<u32>,
    registry_support: Option<RegistrySupportCaptureV1>,
    bind_callbacks: Vec<BindCallbackEventV1>,
    active_bind: Option<ActiveBind>,
    next_bind_ordinal: u32,
    last_bind_order: Option<i32>,
    last_registry_counts: Option<RegistryCountsV1>,
    last_registry_sha256: Option<Sha256Digest>,
    registry_deltas: Vec<RegistryDeltaCaptureV1>,
    post_bind_mutations: Vec<PostBindStateCaptureV1>,
    final_post_bind_states: Vec<PostBindStateCaptureV1>,
    final_state_keys: BTreeSet<(u8, u32)>,
    binds_finished: bool,
    build_jit: Option<BuildJitCaptureV1>,
    frontend_boundaries: Vec<FrontendBoundaryEventV1>,
    frontend_branch_seen: bool,
    preprocessor_config: Option<PreprocessorConfigV1>,
    class_generator_config: Option<ClassGeneratorConfigV1>,
    compiler_options: Option<CompilerOptionsV1>,
}

#[derive(Debug, Clone, Copy)]
struct ActiveBind {
    callback_ordinal: u32,
    bind_order: i32,
    pointer_token: u32,
    begin_counts: RegistryCountsV1,
    delta_start: usize,
}

impl StreamDecoder {
    fn new(header: CaptureHeaderV1) -> Self {
        Self {
            header,
            record_count: 0,
            engine_properties: Vec::new(),
            pointer_tokens: BTreeMap::new(),
            pointer_rvas: BTreeSet::new(),
            registry_support: None,
            bind_callbacks: Vec::new(),
            active_bind: None,
            next_bind_ordinal: 0,
            last_bind_order: None,
            last_registry_counts: None,
            last_registry_sha256: None,
            registry_deltas: Vec::new(),
            post_bind_mutations: Vec::new(),
            final_post_bind_states: Vec::new(),
            final_state_keys: BTreeSet::new(),
            binds_finished: false,
            build_jit: None,
            frontend_boundaries: Vec::new(),
            frontend_branch_seen: false,
            preprocessor_config: None,
            class_generator_config: None,
            compiler_options: None,
        }
    }

    fn apply(
        &mut self,
        kind: RecordKindV1,
        ordinal: u64,
        payload: &[u8],
    ) -> Result<(), CaptureDecodeError> {
        match kind {
            RecordKindV1::EngineProperty => self.engine_property(ordinal, payload),
            RecordKindV1::PointerToken => self.pointer_token(payload),
            RecordKindV1::BindCallback => self.bind_callback(ordinal, payload),
            RecordKindV1::RegistryDeltaJson => self.registry_delta(payload),
            RecordKindV1::PostBindMutationJson => self.post_bind_state(payload, false),
            RecordKindV1::FinalPostBindStateJson => self.post_bind_state(payload, true),
            RecordKindV1::BuildJit => self.build_jit(payload),
            RecordKindV1::FrontendBoundary => self.frontend_boundary(ordinal, payload),
            RecordKindV1::FrontendConfigJson => self.frontend_config(payload),
            RecordKindV1::RegistrySupportJson => self.registry_support(payload),
        }
    }

    fn engine_property(&mut self, ordinal: u64, payload: &[u8]) -> Result<(), CaptureDecodeError> {
        if self.next_bind_ordinal != 0 || self.active_bind.is_some() || self.binds_finished {
            return order("engine property was recorded after bind capture began");
        }
        require_len("engine property", payload, ENGINE_PROPERTY_PAYLOAD_BYTES)?;
        let mut cursor = Cursor::new(payload);
        let property_id = cursor.u32()?;
        expect_u32("engine property reserved field", cursor.u32()?, 0)?;
        let value = cursor.u64()?;
        let call_rva = cursor.u32()?;
        expect_u32("engine property tail reserved field", cursor.u32()?, 0)?;
        if call_rva != RVA_SET_ENGINE_PROPERTY {
            return target("SetEngineProperty RVA");
        }
        let property = engine_property_from_id(property_id)
            .ok_or(CaptureDecodeError::UnknownEngineProperty(property_id))?;
        self.engine_properties.push(CapturedEnginePropertyV1 {
            ordinal,
            property,
            value,
            call_rva,
        });
        Ok(())
    }

    fn pointer_token(&mut self, payload: &[u8]) -> Result<(), CaptureDecodeError> {
        if self.binds_finished || self.build_jit.is_some() {
            return order("pointer token was defined after the bind phase");
        }
        require_len("pointer token", payload, POINTER_TOKEN_PAYLOAD_BYTES)?;
        let mut cursor = Cursor::new(payload);
        let token_id = cursor.u32()?;
        let primary_image_rva = cursor.u32()?;
        expect_u32("pointer token reserved field", cursor.u32()?, 0)?;
        if token_id != self.pointer_tokens.len() as u32 {
            return Err(CaptureDecodeError::PointerTokenOrdinal {
                expected: self.pointer_tokens.len() as u32,
                actual: token_id,
            });
        }
        if primary_image_rva == 0 || primary_image_rva >= PINNED_PE_SIZE_OF_IMAGE {
            return Err(CaptureDecodeError::PointerRva(primary_image_rva));
        }
        if !self.pointer_rvas.insert(primary_image_rva) {
            return Err(CaptureDecodeError::DuplicatePointerRva(primary_image_rva));
        }
        self.pointer_tokens.insert(
            token_id,
            PointerTokenV1 {
                token_id,
                primary_image_rva,
            },
        );
        Ok(())
    }

    fn bind_callback(&mut self, ordinal: u64, payload: &[u8]) -> Result<(), CaptureDecodeError> {
        if self.build_jit.is_some() || !self.final_post_bind_states.is_empty() {
            return order("bind callback was recorded after post-bind finalization");
        }
        require_len("bind callback", payload, BIND_CALLBACK_PAYLOAD_BYTES)?;
        let mut cursor = Cursor::new(payload);
        let callback_ordinal = cursor.u32()?;
        let phase = match cursor.u32()? {
            1 => BindCallbackPhaseV1::Begin,
            2 => BindCallbackPhaseV1::End,
            value => return Err(CaptureDecodeError::BindPhase(value)),
        };
        let bind_order = cursor.i32()?;
        let callback_pointer_token = cursor.u32()?;
        let observation_rva = cursor.u32()?;
        expect_u32("bind callback reserved field", cursor.u32()?, 0)?;
        let counts = RegistryCountsV1 {
            types: cursor.u32()?,
            functions: cursor.u32()?,
            object_properties: cursor.u32()?,
            global_properties: cursor.u32()?,
            enum_values: cursor.u32()?,
            funcdefs: cursor.u32()?,
            typedefs: cursor.u32()?,
            total_registrations: cursor.u32()?,
        };
        let registry_sha256 =
            Sha256Digest::from_bytes(cursor.take(32)?.try_into().expect("fixed digest length"));
        if registry_sha256 == zero_digest() {
            return Err(CaptureDecodeError::ZeroDigest("bind registry snapshot"));
        }
        if !self.pointer_tokens.contains_key(&callback_pointer_token) {
            return Err(CaptureDecodeError::UnknownPointerToken(
                callback_pointer_token,
            ));
        }

        match phase {
            BindCallbackPhaseV1::Begin => {
                if self.binds_finished || self.active_bind.is_some() {
                    return order("nested or late bind callback begin");
                }
                if callback_ordinal != self.next_bind_ordinal {
                    return Err(CaptureDecodeError::BindOrdinal {
                        expected: self.next_bind_ordinal,
                        actual: callback_ordinal,
                    });
                }
                if self
                    .last_bind_order
                    .is_some_and(|previous| bind_order < previous)
                {
                    return order("bind orders are not nondecreasing");
                }
                if observation_rva != RVA_BIND_CALLBACK_CALL {
                    return target("bind callback call RVA");
                }
                if let Some(previous) = self.last_registry_counts {
                    if counts != previous || Some(registry_sha256) != self.last_registry_sha256 {
                        return order("bind begin snapshot does not match previous bind end");
                    }
                }
                self.active_bind = Some(ActiveBind {
                    callback_ordinal,
                    bind_order,
                    pointer_token: callback_pointer_token,
                    begin_counts: counts,
                    delta_start: self.registry_deltas.len(),
                });
            }
            BindCallbackPhaseV1::End => {
                let active = self
                    .active_bind
                    .take()
                    .ok_or_else(|| CaptureDecodeError::Ordering("bind end without begin".into()))?;
                if callback_ordinal != active.callback_ordinal
                    || bind_order != active.bind_order
                    || callback_pointer_token != active.pointer_token
                {
                    return order("bind end identity differs from bind begin");
                }
                if observation_rva != RVA_BIND_CALLBACK_RETURN {
                    return target("bind callback return RVA");
                }
                check_counts_nondecreasing(active.begin_counts, counts)?;
                let added = self.registry_deltas.len() - active.delta_start;
                if counts.total_registrations
                    != active
                        .begin_counts
                        .total_registrations
                        .checked_add(added as u32)
                        .ok_or(CaptureDecodeError::CountOverflow)?
                {
                    return order("bind registration count does not match captured deltas");
                }
                self.last_bind_order = Some(bind_order);
                self.last_registry_counts = Some(counts);
                self.last_registry_sha256 = Some(registry_sha256);
                self.next_bind_ordinal += 1;
            }
        }
        self.bind_callbacks.push(BindCallbackEventV1 {
            ordinal,
            callback_ordinal,
            phase,
            bind_order,
            callback_pointer_token,
            observation_rva,
            counts,
            registry_sha256,
        });
        Ok(())
    }

    fn registry_delta(&mut self, payload: &[u8]) -> Result<(), CaptureDecodeError> {
        let active = self
            .active_bind
            .ok_or_else(|| CaptureDecodeError::Ordering("registry delta outside bind".into()))?;
        check_json_size(payload)?;
        let delta: RegistryDeltaCaptureV1 = serde_json::from_slice(payload)
            .map_err(|error| CaptureDecodeError::Json(error.to_string()))?;
        if delta.schema != REGISTRY_DELTA_CAPTURE_SCHEMA
            || delta.schema_version != CAPTURE_JSON_SCHEMA_VERSION
        {
            return Err(CaptureDecodeError::JsonSchema("registry delta"));
        }
        if delta.bind_callback_ordinal != active.callback_ordinal {
            return order("registry delta names a different active bind callback");
        }
        let expected = self.registry_deltas.len() as u32;
        if delta.entry.ordinal() != expected || delta.entry.registration_id() != expected {
            return Err(CaptureDecodeError::RegistrationOrdinal {
                expected,
                entry: delta.entry.ordinal(),
                registration_id: delta.entry.registration_id(),
            });
        }
        if !registration_result_matches(&delta.entry, &delta.result) {
            return order("registry result kind does not match registration kind");
        }
        self.registry_deltas.push(delta);
        Ok(())
    }

    fn registry_support(&mut self, payload: &[u8]) -> Result<(), CaptureDecodeError> {
        if self.active_bind.is_some() || self.binds_finished || self.registry_support.is_some() {
            return order("registry support metadata is duplicated or outside setup");
        }
        check_json_size(payload)?;
        let support: RegistrySupportCaptureV1 = serde_json::from_slice(payload)
            .map_err(|error| CaptureDecodeError::Json(error.to_string()))?;
        if support.schema != REGISTRY_SUPPORT_CAPTURE_SCHEMA
            || support.schema_version != CAPTURE_JSON_SCHEMA_VERSION
        {
            return Err(CaptureDecodeError::JsonSchema("registry support"));
        }
        if support.host_stub_pointers.len() != support.host_stubs.len() {
            return order("registry support must map every host stub to pointer provenance");
        }
        for (ordinal, mapping) in support.host_stub_pointers.iter().enumerate() {
            if mapping.stub_id != ordinal as u32
                || support.host_stubs[ordinal].stub_id != mapping.stub_id
                || !self.pointer_tokens.contains_key(&mapping.pointer_token)
            {
                return order("registry support host-stub pointer mapping is invalid");
            }
        }
        self.registry_support = Some(support);
        Ok(())
    }

    fn post_bind_state(
        &mut self,
        payload: &[u8],
        final_state: bool,
    ) -> Result<(), CaptureDecodeError> {
        check_json_size(payload)?;
        let state: PostBindStateCaptureV1 = serde_json::from_slice(payload)
            .map_err(|error| CaptureDecodeError::Json(error.to_string()))?;
        if state.schema != POST_BIND_STATE_CAPTURE_SCHEMA
            || state.schema_version != CAPTURE_JSON_SCHEMA_VERSION
        {
            return Err(CaptureDecodeError::JsonSchema("post-bind state"));
        }
        validate_post_bind_state(&state.state)?;
        if final_state {
            if self.active_bind.is_some() || self.next_bind_ordinal == 0 || self.build_jit.is_some()
            {
                return order("final post-bind state is outside finalization window");
            }
            self.binds_finished = true;
            if state.bind_callback_ordinal.is_some()
                || state.state_ordinal != self.final_post_bind_states.len() as u32
            {
                return order("final post-bind state provenance or ordinal is invalid");
            }
            let key = post_bind_state_key(&state.state);
            if !self.final_state_keys.insert(key) {
                return Err(CaptureDecodeError::DuplicateFinalState(key.1));
            }
            self.final_post_bind_states.push(state);
        } else {
            let active = self.active_bind.ok_or_else(|| {
                CaptureDecodeError::Ordering("post-bind mutation outside bind".into())
            })?;
            if state.bind_callback_ordinal != Some(active.callback_ordinal)
                || state.state_ordinal != self.post_bind_mutations.len() as u32
            {
                return order("post-bind mutation provenance or ordinal is invalid");
            }
            self.post_bind_mutations.push(state);
        }
        Ok(())
    }

    fn build_jit(&mut self, payload: &[u8]) -> Result<(), CaptureDecodeError> {
        if self.active_bind.is_some()
            || !self.binds_finished
            || self.final_post_bind_states.is_empty()
            || self.build_jit.is_some()
        {
            return order("build/JIT fact is missing final bind state or is duplicated");
        }
        require_len("build/JIT", payload, BUILD_JIT_PAYLOAD_BYTES)?;
        let mut cursor = Cursor::new(payload);
        let build_identifier = cursor.u32()?;
        let flags = cursor.u32()?;
        if flags & !0x0f != 0 {
            return Err(CaptureDecodeError::ReservedBits("build/JIT flags"));
        }
        let precompiled_guid = cursor.take(16)?.try_into().expect("fixed guid length");
        let compiled_jit_guid = cursor.take(16)?.try_into().expect("fixed guid length");
        let get_build_identifier_rva = cursor.u32()?;
        let get_static_jit_info_rva = cursor.u32()?;
        let fact = BuildJitCaptureV1 {
            build_identifier,
            jit_info_present: flags & 1 != 0,
            jit_guid_matches: flags & 2 != 0,
            jit_database_cleared: flags & 4 != 0,
            shipping_cache_matches: flags & 8 != 0,
            precompiled_guid,
            compiled_jit_guid,
            get_build_identifier_rva,
            get_static_jit_info_rva,
        };
        if fact.build_identifier != PINNED_BUILD_IDENTIFIER
            || !fact.shipping_cache_matches
            || fact.precompiled_guid != PINNED_PRECOMPILED_GUID
            || fact.get_build_identifier_rva != RVA_GET_BUILD_IDENTIFIER
            || fact.get_static_jit_info_rva != RVA_GET_STATIC_JIT_INFO
            || fact.jit_database_cleared
        {
            return Err(CaptureDecodeError::Unqualified(
                "build/JIT identity mismatch",
            ));
        }
        if fact.jit_info_present {
            if !fact.jit_guid_matches || fact.compiled_jit_guid != fact.precompiled_guid {
                return Err(CaptureDecodeError::Unqualified(
                    "compiled JIT guid mismatch",
                ));
            }
        } else if fact.jit_guid_matches || fact.compiled_jit_guid != [0; 16] {
            return Err(CaptureDecodeError::Unqualified(
                "absent JIT contains a guid or match claim",
            ));
        }
        self.build_jit = Some(fact);
        Ok(())
    }

    fn frontend_boundary(
        &mut self,
        ordinal: u64,
        payload: &[u8],
    ) -> Result<(), CaptureDecodeError> {
        if self.build_jit.is_none() {
            return order("frontend boundary was captured before build/JIT identity");
        }
        require_len(
            "frontend boundary",
            payload,
            FRONTEND_BOUNDARY_PAYLOAD_BYTES,
        )?;
        let mut cursor = Cursor::new(payload);
        let kind = match cursor.u32()? {
            1 => FrontendBoundaryKindV1::InitialCompileEnter,
            2 => FrontendBoundaryKindV1::PrecompiledDescriptorsRequested,
            3 => FrontendBoundaryKindV1::PreprocessorConstructed,
            4 => FrontendBoundaryKindV1::InitialCompileReturn,
            value => return Err(CaptureDecodeError::FrontendBoundary(value)),
        };
        let observation_rva = cursor.u32()?;
        let module_count = cursor.u32()?;
        let result_code = cursor.i32()?;
        let config_sha256 = read_digest(&mut cursor)?;
        let input_sha256 = read_digest(&mut cursor)?;
        let output_sha256 = read_digest(&mut cursor)?;
        if config_sha256 == zero_digest() {
            return Err(CaptureDecodeError::ZeroDigest("frontend config"));
        }
        match kind {
            FrontendBoundaryKindV1::InitialCompileEnter => {
                if !self.frontend_boundaries.is_empty()
                    || observation_rva != RVA_INITIAL_COMPILE_ENTER
                    || module_count != 0
                    || output_sha256 != zero_digest()
                {
                    return order("invalid InitialCompile entry boundary");
                }
            }
            FrontendBoundaryKindV1::PrecompiledDescriptorsRequested => {
                if self.frontend_boundaries.len() != 1
                    || self.frontend_branch_seen
                    || observation_rva != RVA_PRECOMPILED_DESCRIPTORS_REQUESTED
                    || module_count == 0
                    || input_sha256 == zero_digest()
                    || output_sha256 == zero_digest()
                {
                    return order("invalid precompiled-descriptor boundary");
                }
                self.frontend_branch_seen = true;
            }
            FrontendBoundaryKindV1::PreprocessorConstructed => {
                if self.frontend_boundaries.len() != 1
                    || self.frontend_branch_seen
                    || observation_rva != RVA_PREPROCESSOR_CONSTRUCTED
                    || module_count != 0
                    || input_sha256 != zero_digest()
                    || output_sha256 != zero_digest()
                {
                    return order("invalid preprocessor-construction boundary");
                }
                self.frontend_branch_seen = true;
            }
            FrontendBoundaryKindV1::InitialCompileReturn => {
                if self.frontend_boundaries.len() != 2
                    || !self.frontend_branch_seen
                    || observation_rva != RVA_INITIAL_COMPILE_RETURN
                    || module_count == 0
                    || result_code != 0
                    || output_sha256 == zero_digest()
                {
                    return order("invalid InitialCompile return boundary");
                }
            }
        }
        self.frontend_boundaries.push(FrontendBoundaryEventV1 {
            ordinal,
            kind,
            observation_rva,
            module_count,
            result_code,
            config_sha256,
            input_sha256,
            output_sha256,
        });
        Ok(())
    }

    fn frontend_config(&mut self, payload: &[u8]) -> Result<(), CaptureDecodeError> {
        if self.build_jit.is_none() || self.frontend_boundaries.len() >= 3 {
            return order("frontend config is outside the frontend capture window");
        }
        if payload.len() < 4 {
            return Err(CaptureDecodeError::Truncated {
                at: payload.len(),
                needed: 4,
            });
        }
        let mut cursor = Cursor::new(payload);
        let kind_raw = cursor.u32()?;
        let kind = FrontendConfigKindV1::parse(kind_raw)
            .ok_or(CaptureDecodeError::FrontendConfig(kind_raw))?;
        let json_len = cursor.remaining();
        let json = cursor.take(json_len)?;
        check_json_size(json)?;
        match kind {
            FrontendConfigKindV1::Preprocessor => {
                if self.preprocessor_config.is_some() {
                    return order("duplicate preprocessor config");
                }
                self.preprocessor_config = Some(
                    PreprocessorConfigV1::from_json(json)
                        .map_err(|error| CaptureDecodeError::Json(error.to_string()))?,
                );
            }
            FrontendConfigKindV1::ClassGenerator => {
                if self.class_generator_config.is_some() {
                    return order("duplicate class-generator config");
                }
                self.class_generator_config = Some(
                    ClassGeneratorConfigV1::from_json(json)
                        .map_err(|error| CaptureDecodeError::Json(error.to_string()))?,
                );
            }
            FrontendConfigKindV1::CompilerOptions => {
                if self.compiler_options.is_some() {
                    return order("duplicate compiler options");
                }
                self.compiler_options = Some(
                    CompilerOptionsV1::from_json(json)
                        .map_err(|error| CaptureDecodeError::Json(error.to_string()))?,
                );
            }
        }
        Ok(())
    }

    fn finish(
        self,
        sealed_stream_sha256: Sha256Digest,
    ) -> Result<DecodedCaptureV1, CaptureDecodeError> {
        if self.active_bind.is_some()
            || self.engine_properties.is_empty()
            || self.next_bind_ordinal == 0
            || self.registry_deltas.is_empty()
            || self.final_post_bind_states.is_empty()
            || self.build_jit.is_none()
            || self.frontend_boundaries.len() != 3
            || !matches!(
                self.frontend_boundaries.last().map(|event| event.kind),
                Some(FrontendBoundaryKindV1::InitialCompileReturn)
            )
        {
            return Err(CaptureDecodeError::Incomplete);
        }
        let frontend_configs = CapturedFrontendConfigsV1 {
            preprocessor: self
                .preprocessor_config
                .ok_or(CaptureDecodeError::Incomplete)?,
            class_generator: self
                .class_generator_config
                .ok_or(CaptureDecodeError::Incomplete)?,
            compiler_options: self
                .compiler_options
                .ok_or(CaptureDecodeError::Incomplete)?,
        };
        let config_sha256 = frontend_config_set_digest(&frontend_configs);
        if self
            .frontend_boundaries
            .iter()
            .any(|boundary| boundary.config_sha256 != config_sha256)
        {
            return Err(CaptureDecodeError::Unqualified(
                "frontend boundary config digest mismatch",
            ));
        }
        let support = self
            .registry_support
            .ok_or(CaptureDecodeError::Incomplete)?;
        for delta in &self.registry_deltas {
            validate_registration_pointer_tokens(&delta.entry, &support, &self.pointer_tokens)?;
        }
        let mut ordered_engine_properties = OrderedEnginePropertiesV1 {
            schema: ENGINE_PROPERTIES_SCHEMA.to_owned(),
            schema_version: REGISTRY_SCHEMA_VERSION,
            settings: self
                .engine_properties
                .iter()
                .enumerate()
                .map(|(ordinal, captured)| EnginePropertySettingV1 {
                    ordinal: ordinal as u32,
                    property: captured.property,
                    value: captured.value,
                })
                .collect(),
            canonical_sha256: zero_digest(),
        };
        ordered_engine_properties
            .seal()
            .map_err(|error| CaptureDecodeError::RegistryProfile(error.to_string()))?;
        let mut registration_trace = RegistrationTraceV1 {
            schema: REGISTRATION_TRACE_SCHEMA.to_owned(),
            schema_version: REGISTRY_SCHEMA_VERSION,
            host_stubs: support.host_stubs,
            primitive_operations: support.primitive_operations,
            dynamic_script_operations: support.dynamic_script_operations,
            entries: self
                .registry_deltas
                .iter()
                .map(|delta| delta.entry.clone())
                .collect(),
            canonical_sha256: zero_digest(),
        };
        registration_trace
            .seal()
            .map_err(|error| CaptureDecodeError::RegistryProfile(error.to_string()))?;
        let mut post_bind_snapshot = PostBindSnapshotV1 {
            schema: POST_BIND_SNAPSHOT_SCHEMA.to_owned(),
            schema_version: REGISTRY_SCHEMA_VERSION,
            engine_properties_sha256: ordered_engine_properties.canonical_sha256,
            registration_trace_sha256: registration_trace.canonical_sha256,
            entries: self
                .registry_deltas
                .iter()
                .enumerate()
                .map(|(ordinal, delta)| PostBindEntryV1 {
                    ordinal: ordinal as u32,
                    trace_registration_id: delta.entry.registration_id(),
                    result: delta.result.clone(),
                })
                .collect(),
            final_states: self
                .final_post_bind_states
                .iter()
                .map(|state| state.state.clone())
                .collect(),
            canonical_sha256: zero_digest(),
        };
        post_bind_snapshot
            .seal()
            .and_then(|()| {
                post_bind_snapshot.validate_against(&ordered_engine_properties, &registration_trace)
            })
            .map_err(|error| CaptureDecodeError::RegistryProfile(error.to_string()))?;
        Ok(DecodedCaptureV1 {
            header: self.header,
            engine_properties: self.engine_properties,
            pointer_tokens: self.pointer_tokens,
            bind_callbacks: self.bind_callbacks,
            registry_deltas: self.registry_deltas,
            post_bind_mutations: self.post_bind_mutations,
            final_post_bind_states: self.final_post_bind_states,
            build_jit: self.build_jit.expect("checked above"),
            frontend_boundaries: self.frontend_boundaries,
            frontend_configs,
            ordered_engine_properties,
            registration_trace,
            post_bind_snapshot,
            sealed_stream_sha256,
        })
    }
}

fn frontend_config_set_digest(configs: &CapturedFrontendConfigsV1) -> Sha256Digest {
    let mut hasher = Sha256::new();
    hasher.update(FRONTEND_CONFIG_SET_HASH_DOMAIN_V1);
    hasher.update(configs.preprocessor.canonical_sha256.as_bytes());
    hasher.update(configs.class_generator.canonical_sha256.as_bytes());
    hasher.update(configs.compiler_options.canonical_sha256.as_bytes());
    Sha256Digest::from_bytes(hasher.finalize().into())
}

fn registration_result_matches(entry: &RegistrationEntryV1, result: &PostBindResultV1) -> bool {
    matches!(
        (entry, result),
        (
            RegistrationEntryV1::ObjectType { .. },
            PostBindResultV1::ObjectType { .. }
        ) | (
            RegistrationEntryV1::Interface { .. },
            PostBindResultV1::Interface { .. }
        ) | (
            RegistrationEntryV1::InterfaceMethod { .. },
            PostBindResultV1::InterfaceMethod { .. }
        ) | (
            RegistrationEntryV1::ObjectProperty { .. },
            PostBindResultV1::ObjectProperty { .. }
        ) | (
            RegistrationEntryV1::ObjectMethod { .. },
            PostBindResultV1::ObjectMethod { .. }
        ) | (
            RegistrationEntryV1::ObjectBehaviour { .. },
            PostBindResultV1::ObjectBehaviour { .. }
        ) | (
            RegistrationEntryV1::GlobalProperty { .. },
            PostBindResultV1::GlobalProperty { .. }
        ) | (
            RegistrationEntryV1::GlobalFunction { .. },
            PostBindResultV1::GlobalFunction { .. }
        ) | (
            RegistrationEntryV1::Enum { .. },
            PostBindResultV1::Enum { .. }
        ) | (
            RegistrationEntryV1::EnumValue { .. },
            PostBindResultV1::EnumValue { .. }
        ) | (
            RegistrationEntryV1::Funcdef { .. },
            PostBindResultV1::Funcdef { .. }
        ) | (
            RegistrationEntryV1::Typedef { .. },
            PostBindResultV1::Typedef { .. }
        ) | (
            RegistrationEntryV1::StringFactory { .. },
            PostBindResultV1::StringFactory { .. }
        ) | (
            RegistrationEntryV1::DefaultArrayType { .. },
            PostBindResultV1::DefaultArrayType { .. }
        )
    )
}

fn validate_registration_pointer_tokens(
    entry: &RegistrationEntryV1,
    support: &RegistrySupportCaptureV1,
    pointer_tokens: &BTreeMap<u32, PointerTokenV1>,
) -> Result<(), CaptureDecodeError> {
    fn require(
        support: &RegistrySupportCaptureV1,
        pointer_tokens: &BTreeMap<u32, PointerTokenV1>,
        stub_id: u32,
    ) -> Result<(), CaptureDecodeError> {
        let token_id = support
            .host_stub_pointers
            .get(stub_id as usize)
            .filter(|mapping| mapping.stub_id == stub_id)
            .map(|mapping| mapping.pointer_token)
            .ok_or(CaptureDecodeError::UnknownHostStub(stub_id))?;
        pointer_tokens
            .contains_key(&token_id)
            .then_some(())
            .ok_or(CaptureDecodeError::UnknownPointerToken(token_id))
    }
    let require_pair = |callable: u32, auxiliary: Option<u32>| {
        require(support, pointer_tokens, callable)?;
        if let Some(token_id) = auxiliary {
            require(support, pointer_tokens, token_id)?;
        }
        Ok(())
    };
    match entry {
        RegistrationEntryV1::ObjectMethod {
            callable_stub_id,
            auxiliary_object_stub_id,
            ..
        }
        | RegistrationEntryV1::ObjectBehaviour {
            callable_stub_id,
            auxiliary_object_stub_id,
            ..
        }
        | RegistrationEntryV1::GlobalFunction {
            callable_stub_id,
            auxiliary_object_stub_id,
            ..
        } => require_pair(*callable_stub_id, *auxiliary_object_stub_id),
        RegistrationEntryV1::GlobalProperty {
            storage_stub_id, ..
        } => require(support, pointer_tokens, *storage_stub_id),
        RegistrationEntryV1::StringFactory {
            factory_object_stub_id,
            ..
        } => require(support, pointer_tokens, *factory_object_stub_id),
        _ => Ok(()),
    }
}

fn validate_post_bind_state(state: &PostBindStateV1) -> Result<(), CaptureDecodeError> {
    match state {
        PostBindStateV1::ObjectType {
            byte_size,
            alignment,
            interface_type_ids,
            interface_vft_offsets,
            ..
        } => {
            if *byte_size > 64 * 1024 * 1024
                || !valid_alignment(*alignment)
                || interface_type_ids.len() != interface_vft_offsets.len()
                || interface_type_ids.len() > 65_536
            {
                return Err(CaptureDecodeError::PostBindState);
            }
        }
        PostBindStateV1::ObjectProperty {
            byte_offset,
            composite_offset,
            exposed_type,
            ..
        } => {
            if *byte_offset > 256 * 1024 * 1024
                || *composite_offset > 256 * 1024 * 1024
                || *exposed_type > u8::MAX as u32
            {
                return Err(CaptureDecodeError::PostBindState);
            }
        }
        PostBindStateV1::Function {
            exposed_type,
            hidden_argument_default,
            ..
        } => {
            if *exposed_type > u8::MAX as u32
                || hidden_argument_default
                    .as_ref()
                    .is_some_and(|value| value.len() > 64 * 1024 || value.contains('\0'))
            {
                return Err(CaptureDecodeError::PostBindState);
            }
        }
        PostBindStateV1::GlobalProperty {
            is_pure_constant,
            pure_constant_value,
            ..
        } => {
            if *is_pure_constant != pure_constant_value.is_some() {
                return Err(CaptureDecodeError::PostBindState);
            }
        }
    }
    Ok(())
}

fn post_bind_state_key(state: &PostBindStateV1) -> (u8, u32) {
    match state {
        PostBindStateV1::ObjectType { type_id, .. } => (1, *type_id),
        PostBindStateV1::ObjectProperty { property_id, .. } => (2, *property_id),
        PostBindStateV1::Function { function_id, .. } => (3, *function_id),
        PostBindStateV1::GlobalProperty { property_id, .. } => (4, *property_id),
    }
}

fn valid_alignment(value: u32) -> bool {
    value != 0 && value <= 4096 && value.is_power_of_two()
}

fn check_counts_nondecreasing(
    before: RegistryCountsV1,
    after: RegistryCountsV1,
) -> Result<(), CaptureDecodeError> {
    if after.types < before.types
        || after.functions < before.functions
        || after.object_properties < before.object_properties
        || after.global_properties < before.global_properties
        || after.enum_values < before.enum_values
        || after.funcdefs < before.funcdefs
        || after.typedefs < before.typedefs
        || after.total_registrations < before.total_registrations
    {
        return order("registry counts decreased inside a bind callback");
    }
    Ok(())
}

fn read_digest(cursor: &mut Cursor<'_>) -> Result<Sha256Digest, CaptureDecodeError> {
    Ok(Sha256Digest::from_bytes(
        cursor.take(32)?.try_into().expect("fixed digest length"),
    ))
}

fn check_json_size(payload: &[u8]) -> Result<(), CaptureDecodeError> {
    if payload.len() > MAX_CAPTURE_JSON_PAYLOAD_V1 {
        Err(CaptureDecodeError::RecordTooLarge {
            actual: payload.len(),
            max: MAX_CAPTURE_JSON_PAYLOAD_V1,
        })
    } else {
        Ok(())
    }
}

fn require_len(
    label: &'static str,
    payload: &[u8],
    expected: usize,
) -> Result<(), CaptureDecodeError> {
    if payload.len() == expected {
        Ok(())
    } else {
        Err(CaptureDecodeError::PayloadLength {
            label,
            expected,
            actual: payload.len(),
        })
    }
}

fn zero_digest() -> Sha256Digest {
    Sha256Digest::from_bytes([0; 32])
}

fn expect_bytes(
    field: &'static str,
    actual: &[u8],
    expected: &[u8],
) -> Result<(), CaptureDecodeError> {
    if actual == expected {
        Ok(())
    } else {
        target(field)
    }
}

fn expect_u16(field: &'static str, actual: u16, expected: u16) -> Result<(), CaptureDecodeError> {
    if actual == expected {
        Ok(())
    } else {
        target(field)
    }
}

fn expect_u32(field: &'static str, actual: u32, expected: u32) -> Result<(), CaptureDecodeError> {
    if actual == expected {
        Ok(())
    } else {
        target(field)
    }
}

fn expect_u64(field: &'static str, actual: u64, expected: u64) -> Result<(), CaptureDecodeError> {
    if actual == expected {
        Ok(())
    } else {
        target(field)
    }
}

fn order<T>(message: impl Into<String>) -> Result<T, CaptureDecodeError> {
    Err(CaptureDecodeError::Ordering(message.into()))
}

fn target<T>(field: &'static str) -> Result<T, CaptureDecodeError> {
    Err(CaptureDecodeError::Target(field))
}

struct Cursor<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> Cursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }

    fn position(&self) -> usize {
        self.position
    }

    fn remaining(&self) -> usize {
        self.bytes.len() - self.position
    }

    fn is_empty(&self) -> bool {
        self.remaining() == 0
    }

    fn take(&mut self, len: usize) -> Result<&'a [u8], CaptureDecodeError> {
        let end = self
            .position
            .checked_add(len)
            .ok_or(CaptureDecodeError::CountOverflow)?;
        if end > self.bytes.len() {
            return Err(CaptureDecodeError::Truncated {
                at: self.position,
                needed: len,
            });
        }
        let out = &self.bytes[self.position..end];
        self.position = end;
        Ok(out)
    }

    fn u16(&mut self) -> Result<u16, CaptureDecodeError> {
        Ok(u16::from_le_bytes(
            self.take(2)?.try_into().expect("fixed length"),
        ))
    }

    fn u32(&mut self) -> Result<u32, CaptureDecodeError> {
        Ok(u32::from_le_bytes(
            self.take(4)?.try_into().expect("fixed length"),
        ))
    }

    fn i32(&mut self) -> Result<i32, CaptureDecodeError> {
        Ok(i32::from_le_bytes(
            self.take(4)?.try_into().expect("fixed length"),
        ))
    }

    fn u64(&mut self) -> Result<u64, CaptureDecodeError> {
        Ok(u64::from_le_bytes(
            self.take(8)?.try_into().expect("fixed length"),
        ))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CaptureDecodeError {
    #[error("capture is {actual} bytes; maximum accepted size is {max}")]
    InputTooLarge { actual: usize, max: usize },
    #[error("capture is truncated at {at}; need {needed} more/total bytes")]
    Truncated { at: usize, needed: usize },
    #[error("invalid capture header: {0}")]
    Header(&'static str),
    #[error("capture does not match pinned target field {0}")]
    Target(&'static str),
    #[error("invalid capture footer: {0}")]
    Footer(&'static str),
    #[error("sealed capture digest mismatch")]
    DigestMismatch,
    #[error("record header at byte {offset} uses unsupported flags, version, or reserved bits")]
    RecordHeader { offset: usize },
    #[error("record ordinal mismatch: expected {expected}, got {actual}")]
    RecordOrdinal { expected: u64, actual: u64 },
    #[error("record payload is {actual} bytes; maximum is {max}")]
    RecordTooLarge { actual: usize, max: usize },
    #[error("unsupported capture record kind {0}")]
    UnknownRecordKind(u16),
    #[error("capture contains more than {max} records")]
    TooManyRecords { max: u64 },
    #[error("footer record count {declared} differs from decoded count {actual}")]
    FooterRecordCount { declared: u64, actual: u64 },
    #[error("{label} payload length mismatch: expected {expected}, got {actual}")]
    PayloadLength {
        label: &'static str,
        expected: usize,
        actual: usize,
    },
    #[error("unknown AngelScript engine property id {0}")]
    UnknownEngineProperty(u32),
    #[error("pointer token ordinal mismatch: expected {expected}, got {actual}")]
    PointerTokenOrdinal { expected: u32, actual: u32 },
    #[error("pointer token RVA 0x{0:x} is outside the pinned primary image")]
    PointerRva(u32),
    #[error("primary-image RVA 0x{0:x} was assigned multiple pointer tokens")]
    DuplicatePointerRva(u32),
    #[error("capture references unknown pointer token {0}")]
    UnknownPointerToken(u32),
    #[error("capture references unknown host stub {0}")]
    UnknownHostStub(u32),
    #[error("unknown bind callback phase {0}")]
    BindPhase(u32),
    #[error("bind callback ordinal mismatch: expected {expected}, got {actual}")]
    BindOrdinal { expected: u32, actual: u32 },
    #[error("capture event order is invalid: {0}")]
    Ordering(String),
    #[error("captured JSON is invalid: {0}")]
    Json(String),
    #[error("captured {0} JSON uses an unsupported schema")]
    JsonSchema(&'static str),
    #[error(
        "registration ordinal mismatch: expected {expected}, entry {entry}, id {registration_id}"
    )]
    RegistrationOrdinal {
        expected: u32,
        entry: u32,
        registration_id: u32,
    },
    #[error("post-bind state exceeds closed bounds or has inconsistent fields")]
    PostBindState,
    #[error("duplicate final post-bind state identity {0}")]
    DuplicateFinalState(u32),
    #[error("integer count overflow")]
    CountOverflow,
    #[error("unknown or reserved bits are set in {0}")]
    ReservedBits(&'static str),
    #[error("capture is not qualified: {0}")]
    Unqualified(&'static str),
    #[error("unknown frontend boundary kind {0}")]
    FrontendBoundary(u32),
    #[error("unknown frontend config kind {0}")]
    FrontendConfig(u32),
    #[error("required {0} digest is zero")]
    ZeroDigest(&'static str),
    #[error("capture is structurally valid but incomplete")]
    Incomplete,
    #[error("captured registry cannot form a replayable profile: {0}")]
    RegistryProfile(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler_profile::frontend::{
        ClassGeneratorConfigV1, CompilerOptionsV1, EffectivePreprocessorFlagV1,
        PreprocessorConfigV1, PropertyBlueprintSpecifierV1, PropertyEditSpecifierV1,
        StaticClassModeV1, CLASS_GENERATOR_CONFIG_SCHEMA, COMPILER_OPTIONS_SCHEMA,
        FRONTEND_SCHEMA_VERSION, PREPROCESSOR_CONFIG_SCHEMA,
    };
    use crate::compiler_profile::registry::{
        CallConventionV1, CompileOnlyStubPurposeV1, CompileOutModeV1,
        DynamicScriptTypeOperationsV1, FirstParamMetadataV1, FixedTypeOperationsV1,
        HostStubDescriptorV1, HostStubKindV1, PrimitiveTypeOperationsV1, PrimitiveTypeV1,
        RegistrationContextV1,
    };

    fn u16le(out: &mut Vec<u8>, value: u16) {
        out.extend_from_slice(&value.to_le_bytes());
    }

    fn u32le(out: &mut Vec<u8>, value: u32) {
        out.extend_from_slice(&value.to_le_bytes());
    }

    fn i32le(out: &mut Vec<u8>, value: i32) {
        out.extend_from_slice(&value.to_le_bytes());
    }

    fn u64le(out: &mut Vec<u8>, value: u64) {
        out.extend_from_slice(&value.to_le_bytes());
    }

    fn digest(byte: u8) -> [u8; 32] {
        [byte; 32]
    }

    fn record(out: &mut Vec<u8>, kind: RecordKindV1, payload: &[u8]) {
        let ordinal = ((out.len() - CAPTURE_HEADER_BYTES_V1) > 0) as u64;
        let ordinal = if ordinal == 0 {
            0
        } else {
            let mut cursor = Cursor::new(&out[CAPTURE_HEADER_BYTES_V1..]);
            let mut count = 0;
            while !cursor.is_empty() {
                cursor.take(8).unwrap();
                let len = cursor.u32().unwrap() as usize;
                cursor.take(4 + 8 + len).unwrap();
                count += 1;
            }
            count
        };
        u16le(out, kind as u16);
        u16le(out, CAPTURE_SCHEMA_VERSION_V1);
        u32le(out, 0);
        u32le(out, payload.len() as u32);
        u32le(out, 0);
        u64le(out, ordinal);
        out.extend_from_slice(payload);
    }

    fn reseal(mut stream: Vec<u8>, record_count: u64) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(CAPTURE_HASH_DOMAIN_V1);
        hasher.update(&stream);
        let seal: [u8; 32] = hasher.finalize().into();
        stream.extend_from_slice(CAPTURE_FOOTER_MAGIC_V1);
        u64le(&mut stream, record_count);
        let stream_bytes = (stream.len() - 16) as u64;
        u64le(&mut stream, stream_bytes);
        stream.extend_from_slice(&seal);
        u32le(&mut stream, CAPTURE_SCHEMA_VERSION_V1 as u32);
        u32le(&mut stream, 0);
        stream
    }

    fn frontend_configs() -> CapturedFrontendConfigsV1 {
        let names = [
            "COOK_COMMANDLET",
            "EDITOR",
            "EDITORONLY_DATA",
            "RELEASE",
            "TEST",
            "WITH_SERVER_CODE",
        ];
        let mut preprocessor = PreprocessorConfigV1 {
            schema: PREPROCESSOR_CONFIG_SCHEMA.to_owned(),
            schema_version: FRONTEND_SCHEMA_VERSION,
            automatic_imports: true,
            warn_on_manual_import_statements: true,
            use_editor_scripts: false,
            effective_flags: names
                .into_iter()
                .enumerate()
                .map(|(ordinal, name)| EffectivePreprocessorFlagV1 {
                    ordinal: ordinal as u32,
                    name: name.to_owned(),
                    value: matches!(name, "RELEASE" | "WITH_SERVER_CODE"),
                })
                .collect(),
            default_function_blueprint_callable: true,
            default_property_edit_specifier: PropertyEditSpecifierV1::EditAnywhere,
            default_property_edit_specifier_for_structs: PropertyEditSpecifierV1::EditAnywhere,
            default_property_blueprint_specifier: PropertyBlueprintSpecifierV1::BlueprintReadWrite,
            static_class_mode: StaticClassModeV1::Allowed,
            script_float_is_float64: true,
            angelscript_haze: false,
            enforce_server_rpc_validation: false,
            blueprint_event_argument_specializations: Vec::new(),
            native_super_types: Vec::new(),
            fname_comparison_keys: Vec::new(),
            external_hooks: crate::compiler_profile::frontend::ExternalFrontendHooksV1::unbound(),
            canonical_sha256: zero_digest(),
        };
        preprocessor.seal().unwrap();
        let mut class_generator = ClassGeneratorConfigV1 {
            schema: CLASS_GENERATOR_CONFIG_SCHEMA.to_owned(),
            schema_version: FRONTEND_SCHEMA_VERSION,
            mark_non_uproperty_properties_as_transient: false,
            canonical_sha256: zero_digest(),
        };
        class_generator.seal().unwrap();
        let mut compiler_options = CompilerOptionsV1 {
            schema: COMPILER_OPTIONS_SCHEMA.to_owned(),
            schema_version: FRONTEND_SCHEMA_VERSION,
            error_on_incorrect_editor_only_code: true,
            warn_on_divergent_comparison_operator_overloads: true,
            warn_on_implicit_signed_unsigned_conversion: true,
            warn_on_increment_decrement_in_complex_expression: true,
            warn_on_unused_return_value_for_const_methods: true,
            canonical_sha256: zero_digest(),
        };
        compiler_options.seal().unwrap();
        CapturedFrontendConfigsV1 {
            preprocessor,
            class_generator,
            compiler_options,
        }
    }

    fn fixed_operations(value_size: u32, value_alignment: u32) -> FixedTypeOperationsV1 {
        FixedTypeOperationsV1 {
            can_be_template_subtype: true,
            can_construct: true,
            need_construct: false,
            can_destruct: true,
            need_destruct: false,
            can_copy: true,
            need_copy: false,
            can_compare: true,
            can_hash_value: true,
            value_size,
            value_alignment,
            is_object_pointer: false,
        }
    }

    fn registry_support() -> RegistrySupportCaptureV1 {
        let primitives = [
            PrimitiveTypeV1::Bool,
            PrimitiveTypeV1::Int8,
            PrimitiveTypeV1::Int16,
            PrimitiveTypeV1::Int32,
            PrimitiveTypeV1::Int64,
            PrimitiveTypeV1::Uint8,
            PrimitiveTypeV1::Uint16,
            PrimitiveTypeV1::Uint32,
            PrimitiveTypeV1::Uint64,
            PrimitiveTypeV1::Float32,
            PrimitiveTypeV1::Float64,
        ];
        RegistrySupportCaptureV1 {
            schema: REGISTRY_SUPPORT_CAPTURE_SCHEMA.to_owned(),
            schema_version: CAPTURE_JSON_SCHEMA_VERSION,
            host_stubs: vec![HostStubDescriptorV1 {
                stub_id: 0,
                purpose: CompileOnlyStubPurposeV1::CompileOnlyNeverInvoke,
                descriptor: HostStubKindV1::Callable {
                    signature_sha256: Sha256Digest::from_bytes(digest(0x51)),
                },
            }],
            host_stub_pointers: vec![HostStubPointerCaptureV1 {
                stub_id: 0,
                pointer_token: 1,
            }],
            primitive_operations: primitives
                .into_iter()
                .enumerate()
                .map(|(ordinal, primitive)| PrimitiveTypeOperationsV1 {
                    ordinal: ordinal as u32,
                    primitive,
                    operations: fixed_operations(if ordinal == 0 { 1 } else { 8 }, 1),
                })
                .collect(),
            dynamic_script_operations: DynamicScriptTypeOperationsV1 {
                delegate: fixed_operations(16, 8),
                multicast_delegate: fixed_operations(16, 8),
            },
        }
    }

    fn bind_payload(phase: u32, counts: RegistryCountsV1, snapshot: [u8; 32]) -> Vec<u8> {
        let mut payload = Vec::new();
        u32le(&mut payload, 0);
        u32le(&mut payload, phase);
        i32le(&mut payload, 10);
        u32le(&mut payload, 0);
        u32le(
            &mut payload,
            if phase == 1 {
                RVA_BIND_CALLBACK_CALL
            } else {
                RVA_BIND_CALLBACK_RETURN
            },
        );
        u32le(&mut payload, 0);
        for value in [
            counts.types,
            counts.functions,
            counts.object_properties,
            counts.global_properties,
            counts.enum_values,
            counts.funcdefs,
            counts.typedefs,
            counts.total_registrations,
        ] {
            u32le(&mut payload, value);
        }
        payload.extend_from_slice(&snapshot);
        payload
    }

    fn frontend_boundary(
        kind: u32,
        rva: u32,
        module_count: u32,
        config_digest: Sha256Digest,
    ) -> Vec<u8> {
        let mut payload = Vec::new();
        u32le(&mut payload, kind);
        u32le(&mut payload, rva);
        u32le(&mut payload, module_count);
        i32le(&mut payload, 0);
        payload.extend_from_slice(config_digest.as_bytes());
        let input = if kind == 3 { [0; 32] } else { digest(0x61) };
        let output = if kind == 1 || kind == 3 {
            [0; 32]
        } else {
            digest(0x62)
        };
        payload.extend_from_slice(&input);
        payload.extend_from_slice(&output);
        payload
    }

    fn fixture_stream() -> (Vec<u8>, u64) {
        let mut out = Vec::new();
        out.extend_from_slice(CAPTURE_MAGIC_V1);
        u16le(&mut out, CAPTURE_SCHEMA_VERSION_V1);
        u16le(&mut out, CAPTURE_HEADER_BYTES_V1 as u16);
        u32le(&mut out, 0);
        u32le(&mut out, PINNED_STEAM_APP_ID);
        u64le(&mut out, PINNED_STEAM_BUILD_ID);
        u32le(&mut out, PINNED_ANGELSCRIPT_VERSION);
        u64le(&mut out, PINNED_EXECUTABLE_BYTES);
        out.extend_from_slice(&PINNED_EXECUTABLE_SHA256);
        out.extend_from_slice(&PINNED_CODEVIEW_GUID_RSDS);
        u32le(&mut out, PINNED_CODEVIEW_AGE);
        out.extend_from_slice(&[0x42; 16]);
        u32le(&mut out, 0);
        assert_eq!(out.len(), CAPTURE_HEADER_BYTES_V1);

        let mut property = Vec::new();
        u32le(&mut property, 2);
        u32le(&mut property, 0);
        u64le(&mut property, 1);
        u32le(&mut property, RVA_SET_ENGINE_PROPERTY);
        u32le(&mut property, 0);
        record(&mut out, RecordKindV1::EngineProperty, &property);

        for (token, rva) in [(0, 0x1000), (1, 0x2000)] {
            let mut payload = Vec::new();
            u32le(&mut payload, token);
            u32le(&mut payload, rva);
            u32le(&mut payload, 0);
            record(&mut out, RecordKindV1::PointerToken, &payload);
        }
        record(
            &mut out,
            RecordKindV1::RegistrySupportJson,
            &serde_json::to_vec(&registry_support()).unwrap(),
        );

        let empty = RegistryCountsV1 {
            types: 0,
            functions: 0,
            object_properties: 0,
            global_properties: 0,
            enum_values: 0,
            funcdefs: 0,
            typedefs: 0,
            total_registrations: 0,
        };
        record(
            &mut out,
            RecordKindV1::BindCallback,
            &bind_payload(1, empty, digest(0x31)),
        );
        let delta = RegistryDeltaCaptureV1 {
            schema: REGISTRY_DELTA_CAPTURE_SCHEMA.to_owned(),
            schema_version: CAPTURE_JSON_SCHEMA_VERSION,
            bind_callback_ordinal: 0,
            entry: RegistrationEntryV1::GlobalFunction {
                ordinal: 0,
                registration_id: 0,
                context: RegistrationContextV1 {
                    namespace: String::new(),
                    config_group: None,
                    access_mask: 0,
                },
                function_id: 7,
                declaration: "void Fixture()".to_owned(),
                call_convention: CallConventionV1::Cdecl,
                callable_stub_id: 0,
                auxiliary_object_stub_id: None,
            },
            result: PostBindResultV1::GlobalFunction {
                engine_function_id: 7,
            },
        };
        record(
            &mut out,
            RecordKindV1::RegistryDeltaJson,
            &serde_json::to_vec(&delta).unwrap(),
        );
        let state = PostBindStateV1::Function {
            function_id: 7,
            trait_bits: 0,
            exposed_type: 0,
            hidden_argument_index: None,
            hidden_argument_default: None,
            determines_output_type_argument_index: None,
            compile_out_mode: CompileOutModeV1::CompileCalls,
            first_param_metadata: FirstParamMetadataV1::None,
        };
        let mutation = PostBindStateCaptureV1 {
            schema: POST_BIND_STATE_CAPTURE_SCHEMA.to_owned(),
            schema_version: CAPTURE_JSON_SCHEMA_VERSION,
            bind_callback_ordinal: Some(0),
            state_ordinal: 0,
            state: state.clone(),
        };
        record(
            &mut out,
            RecordKindV1::PostBindMutationJson,
            &serde_json::to_vec(&mutation).unwrap(),
        );
        let after = RegistryCountsV1 {
            functions: 1,
            total_registrations: 1,
            ..empty
        };
        record(
            &mut out,
            RecordKindV1::BindCallback,
            &bind_payload(2, after, digest(0x32)),
        );
        let final_state = PostBindStateCaptureV1 {
            schema: POST_BIND_STATE_CAPTURE_SCHEMA.to_owned(),
            schema_version: CAPTURE_JSON_SCHEMA_VERSION,
            bind_callback_ordinal: None,
            state_ordinal: 0,
            state,
        };
        record(
            &mut out,
            RecordKindV1::FinalPostBindStateJson,
            &serde_json::to_vec(&final_state).unwrap(),
        );

        let mut build = Vec::new();
        u32le(&mut build, PINNED_BUILD_IDENTIFIER);
        u32le(&mut build, 8);
        build.extend_from_slice(&PINNED_PRECOMPILED_GUID);
        build.extend_from_slice(&[0; 16]);
        u32le(&mut build, RVA_GET_BUILD_IDENTIFIER);
        u32le(&mut build, RVA_GET_STATIC_JIT_INFO);
        record(&mut out, RecordKindV1::BuildJit, &build);

        let configs = frontend_configs();
        for (kind, json) in [
            (1, configs.preprocessor.to_json().unwrap()),
            (2, configs.class_generator.to_json().unwrap()),
            (3, configs.compiler_options.to_json().unwrap()),
        ] {
            let mut payload = Vec::new();
            u32le(&mut payload, kind);
            payload.extend_from_slice(&json);
            record(&mut out, RecordKindV1::FrontendConfigJson, &payload);
        }
        let config_digest = frontend_config_set_digest(&configs);
        for (kind, rva, modules) in [
            (1, RVA_INITIAL_COMPILE_ENTER, 0),
            (3, RVA_PREPROCESSOR_CONSTRUCTED, 0),
            (4, RVA_INITIAL_COMPILE_RETURN, 1),
        ] {
            record(
                &mut out,
                RecordKindV1::FrontendBoundary,
                &frontend_boundary(kind, rva, modules, config_digest),
            );
        }
        (out, 16)
    }

    #[test]
    fn decodes_complete_pointer_neutral_fixture() {
        let (stream, count) = fixture_stream();
        let capture = reseal(stream, count);
        let decoded = decode_capture_v1(&capture).unwrap();
        assert_eq!(decoded.header.capture_id, [0x42; 16]);
        assert_eq!(decoded.pointer_tokens.len(), 2);
        assert_eq!(decoded.bind_callbacks.len(), 2);
        assert_eq!(decoded.registry_deltas.len(), 1);
        assert_eq!(decoded.post_bind_mutations.len(), 1);
        assert_eq!(decoded.final_post_bind_states.len(), 1);
        assert_eq!(decoded.frontend_boundaries.len(), 3);
        assert!(decoded.build_jit.shipping_cache_matches);
        assert!(!decoded.build_jit.jit_info_present);
    }

    #[test]
    fn rejects_corruption_resealed_absolute_pointer_and_incomplete_capture() {
        let (stream, count) = fixture_stream();
        let mut corrupted = reseal(stream.clone(), count);
        corrupted[CAPTURE_HEADER_BYTES_V1 + CAPTURE_RECORD_HEADER_BYTES_V1 + 8] ^= 1;
        assert!(matches!(
            decode_capture_v1(&corrupted),
            Err(CaptureDecodeError::DigestMismatch)
        ));

        let mut invalid_pointer = stream.clone();
        let first_record_bytes = CAPTURE_RECORD_HEADER_BYTES_V1 + ENGINE_PROPERTY_PAYLOAD_BYTES;
        let second_record_payload =
            CAPTURE_HEADER_BYTES_V1 + first_record_bytes + CAPTURE_RECORD_HEADER_BYTES_V1;
        invalid_pointer[second_record_payload + 4..second_record_payload + 8]
            .copy_from_slice(&PINNED_PE_SIZE_OF_IMAGE.to_le_bytes());
        let invalid_pointer = reseal(invalid_pointer, count);
        assert!(matches!(
            decode_capture_v1(&invalid_pointer),
            Err(CaptureDecodeError::PointerRva(_))
        ));

        let mut mismatched_final_state = stream.clone();
        let needle = b"\"function_id\":7";
        let final_position = mismatched_final_state
            .windows(needle.len())
            .rposition(|window| window == needle)
            .expect("fixture final function state");
        mismatched_final_state[final_position + needle.len() - 1] = b'8';
        let mismatched_final_state = reseal(mismatched_final_state, count);
        assert!(matches!(
            decode_capture_v1(&mismatched_final_state),
            Err(CaptureDecodeError::RegistryProfile(_))
        ));

        let truncated_stream =
            stream[..stream.len() - (CAPTURE_RECORD_HEADER_BYTES_V1 + 112)].to_vec();
        let incomplete = reseal(truncated_stream, count - 1);
        assert!(matches!(
            decode_capture_v1(&incomplete),
            Err(CaptureDecodeError::Incomplete)
        ));
    }
}
