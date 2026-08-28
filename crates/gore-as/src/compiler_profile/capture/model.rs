use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::super::frontend::{ClassGeneratorConfigV1, CompilerOptionsV1, PreprocessorConfigV1};
use super::super::manifest::Sha256Digest;
use super::super::registry::{
    DynamicScriptTypeOperationsV1, EnginePropertyV1, HostStubDescriptorV1,
    OrderedEnginePropertiesV1, PostBindResultV1, PostBindSnapshotV1, PostBindStateV1,
    PrimitiveTypeOperationsV1, RegistrationEntryV1, RegistrationTraceV1,
};

pub const CAPTURE_SCHEMA_VERSION_V1: u16 = 1;
pub const CAPTURE_HEADER_BYTES_V1: usize = 112;
pub const CAPTURE_RECORD_HEADER_BYTES_V1: usize = 24;
pub const CAPTURE_FOOTER_BYTES_V1: usize = 64;
pub const CAPTURE_MAGIC_V1: &[u8; 8] = b"GORASCAP";
pub const CAPTURE_FOOTER_MAGIC_V1: &[u8; 8] = b"GORESEAL";
pub const CAPTURE_HASH_DOMAIN_V1: &[u8] = b"gore-as-runtime-capture-v1\0";
pub const FRONTEND_CONFIG_SET_HASH_DOMAIN_V1: &[u8] = b"gore-as-captured-frontend-config-set-v1\0";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureTargetGenerationV1 {
    Build24539464,
    Build24878692,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaptureTargetV1 {
    pub generation: CaptureTargetGenerationV1,
    pub steam_app_id: u32,
    pub steam_build_id: u64,
    pub depot_id: u32,
    pub depot_manifest_gid: u64,
    pub angelscript_version: u32,
    pub executable_bytes: u64,
    pub pe_size_of_image: u32,
    pub executable_sha256: [u8; 32],
    /// Raw sixteen bytes following `RSDS`, not a host-endian GUID structure.
    pub codeview_guid_rsds: [u8; 16],
    pub codeview_guid: &'static str,
    pub codeview_age: u32,
    pub build_identifier: u32,
    /// Raw bytes serialized for `PrecompiledScript_Shipping.Cache`'s FGuid.
    pub precompiled_guid: [u8; 16],
    pub rva_set_engine_property: u32,
    pub rva_bind_callback_call: u32,
    pub rva_bind_callback_return: u32,
    pub rva_get_build_identifier: u32,
    pub rva_get_static_jit_info: u32,
    pub rva_initial_compile_enter: u32,
    pub rva_precompiled_descriptors_requested: u32,
    pub rva_preprocessor_constructed: u32,
    pub rva_initial_compile_return: u32,
}

/// Historical capture target retained so already-produced BuildID-24539464 captures and
/// packages remain independently authenticatable.
pub const CAPTURE_TARGET_24539464: CaptureTargetV1 = CaptureTargetV1 {
    generation: CaptureTargetGenerationV1::Build24539464,
    steam_app_id: 1_297_900,
    steam_build_id: 24_539_464,
    depot_id: 1_297_901,
    depot_manifest_gid: 1_585_071_322_101_748_861,
    angelscript_version: 23_300,
    executable_bytes: 171_784_704,
    pe_size_of_image: 0x0a7e_4000,
    executable_sha256: [
        0xc7, 0x1c, 0x04, 0xdd, 0x86, 0xe1, 0x1e, 0x3e, 0x94, 0x48, 0x3e, 0xa0, 0x2c, 0x26, 0xc6,
        0x12, 0xb6, 0x24, 0x3c, 0x14, 0x7f, 0x6d, 0x83, 0x97, 0x32, 0x33, 0xb3, 0xc8, 0xdd, 0xc5,
        0xde, 0x25,
    ],
    codeview_guid_rsds: [
        0xbd, 0x83, 0x0b, 0xcf, 0x23, 0xe0, 0x1b, 0x06, 0x21, 0x00, 0x0f, 0x0f, 0xcc, 0xf8, 0x71,
        0xd2,
    ],
    codeview_guid: "cf0b83bd-e023-061b-2100-0f0fccf871d2",
    codeview_age: 1,
    build_identifier: 0x9e37_7abe,
    precompiled_guid: [
        0xbe, 0x78, 0xfe, 0x0a, 0x46, 0xac, 0x66, 0x43, 0x96, 0x85, 0x97, 0xe8, 0x5c, 0x7e, 0x5b,
        0x3f,
    ],
    rva_set_engine_property: 0x47a_50f0,
    rva_bind_callback_call: 0x468_56fb,
    rva_bind_callback_return: 0x468_56fd,
    rva_get_build_identifier: 0x48d_3230,
    rva_get_static_jit_info: 0x48d_0f60,
    rva_initial_compile_enter: 0x468_4210,
    rva_precompiled_descriptors_requested: 0x468_42d0,
    rva_preprocessor_constructed: 0x468_435d,
    rva_initial_compile_return: 0x468_5a46,
};

/// Current production capture target. Every value is independently pinned; callers must not
/// infer this generation from a blanket RVA delta.
pub const CAPTURE_TARGET_24878692: CaptureTargetV1 = CaptureTargetV1 {
    generation: CaptureTargetGenerationV1::Build24878692,
    steam_app_id: 1_297_900,
    steam_build_id: 24_878_692,
    depot_id: 1_297_901,
    depot_manifest_gid: 382_135_126_159_906_494,
    angelscript_version: 23_300,
    executable_bytes: 171_792_384,
    pe_size_of_image: 0x0a7e_5000,
    executable_sha256: [
        0x82, 0x4f, 0xbc, 0x94, 0xf2, 0xac, 0x7f, 0x45, 0x92, 0x7a, 0x07, 0x54, 0x60, 0x56, 0x66,
        0xc3, 0x7a, 0xf8, 0x62, 0xd6, 0x61, 0x56, 0xa1, 0x5f, 0x8b, 0xf6, 0x81, 0x37, 0x59, 0xd9,
        0xe8, 0xe0,
    ],
    codeview_guid_rsds: [
        0xda, 0x4a, 0xca, 0xc2, 0x78, 0x48, 0x63, 0xd9, 0xe5, 0x67, 0x71, 0x7d, 0xc2, 0xc4, 0x83,
        0xa2,
    ],
    codeview_guid: "c2ca4ada-4878-d963-e567-717dc2c483a2",
    codeview_age: 1,
    build_identifier: 0x9e37_7abe,
    precompiled_guid: [
        0x78, 0x35, 0xbc, 0xc0, 0x9c, 0x5e, 0xee, 0x48, 0x8d, 0x72, 0xcb, 0x5f, 0xfb, 0x0f, 0xb0,
        0xc3,
    ],
    rva_set_engine_property: 0x47a_50b0,
    rva_bind_callback_call: 0x468_56bb,
    rva_bind_callback_return: 0x468_56bd,
    rva_get_build_identifier: 0x48d_31f0,
    rva_get_static_jit_info: 0x48d_0f20,
    rva_initial_compile_enter: 0x468_41d0,
    rva_precompiled_descriptors_requested: 0x468_4290,
    rva_preprocessor_constructed: 0x468_431d,
    rva_initial_compile_return: 0x468_5a06,
};

pub const SUPPORTED_CAPTURE_TARGETS_V1: [CaptureTargetV1; 2] =
    [CAPTURE_TARGET_24539464, CAPTURE_TARGET_24878692];

pub fn capture_target_for_steam_build_id_v1(
    steam_build_id: u64,
) -> Option<&'static CaptureTargetV1> {
    SUPPORTED_CAPTURE_TARGETS_V1
        .iter()
        .find(|target| target.steam_build_id == steam_build_id)
}

impl CaptureTargetGenerationV1 {
    pub const fn target(self) -> &'static CaptureTargetV1 {
        match self {
            Self::Build24539464 => &CAPTURE_TARGET_24539464,
            Self::Build24878692 => &CAPTURE_TARGET_24878692,
        }
    }
}

// Compatibility aliases intentionally name the one production-live target. Historical decoding
// selects its typed descriptor from the capture header instead of consulting these aliases.
pub const PINNED_STEAM_APP_ID: u32 = CAPTURE_TARGET_24878692.steam_app_id;
pub const PINNED_STEAM_BUILD_ID: u64 = CAPTURE_TARGET_24878692.steam_build_id;
pub const PINNED_ANGELSCRIPT_VERSION: u32 = CAPTURE_TARGET_24878692.angelscript_version;
pub const PINNED_EXECUTABLE_BYTES: u64 = CAPTURE_TARGET_24878692.executable_bytes;
pub const PINNED_PE_SIZE_OF_IMAGE: u32 = CAPTURE_TARGET_24878692.pe_size_of_image;
pub const PINNED_EXECUTABLE_SHA256: [u8; 32] = CAPTURE_TARGET_24878692.executable_sha256;
pub const PINNED_CODEVIEW_GUID_RSDS: [u8; 16] = CAPTURE_TARGET_24878692.codeview_guid_rsds;
pub const PINNED_CODEVIEW_AGE: u32 = CAPTURE_TARGET_24878692.codeview_age;
pub const PINNED_BUILD_IDENTIFIER: u32 = CAPTURE_TARGET_24878692.build_identifier;
pub const PINNED_PRECOMPILED_GUID: [u8; 16] = CAPTURE_TARGET_24878692.precompiled_guid;

pub const RVA_SET_ENGINE_PROPERTY: u32 = CAPTURE_TARGET_24878692.rva_set_engine_property;
pub const RVA_BIND_CALLBACK_CALL: u32 = CAPTURE_TARGET_24878692.rva_bind_callback_call;
pub const RVA_BIND_CALLBACK_RETURN: u32 = CAPTURE_TARGET_24878692.rva_bind_callback_return;
pub const RVA_GET_BUILD_IDENTIFIER: u32 = CAPTURE_TARGET_24878692.rva_get_build_identifier;
pub const RVA_GET_STATIC_JIT_INFO: u32 = CAPTURE_TARGET_24878692.rva_get_static_jit_info;
pub const RVA_INITIAL_COMPILE_ENTER: u32 = CAPTURE_TARGET_24878692.rva_initial_compile_enter;
pub const RVA_PRECOMPILED_DESCRIPTORS_REQUESTED: u32 =
    CAPTURE_TARGET_24878692.rva_precompiled_descriptors_requested;
pub const RVA_PREPROCESSOR_CONSTRUCTED: u32 = CAPTURE_TARGET_24878692.rva_preprocessor_constructed;
pub const RVA_INITIAL_COMPILE_RETURN: u32 = CAPTURE_TARGET_24878692.rva_initial_compile_return;

pub const MAX_CAPTURE_BYTES_V1: usize = 512 * 1024 * 1024;
pub const MAX_CAPTURE_RECORDS_V1: u64 = 2_000_000;
pub const MAX_CAPTURE_RECORD_PAYLOAD_V1: usize = 256 * 1024 * 1024;
pub const MAX_CAPTURE_JSON_PAYLOAD_V1: usize = 256 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureHeaderV1 {
    pub capture_id: [u8; 16],
    pub target_generation: CaptureTargetGenerationV1,
}

impl CaptureHeaderV1 {
    pub const fn target(&self) -> &'static CaptureTargetV1 {
        self.target_generation.target()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct RegistryCountsV1 {
    pub types: u32,
    pub functions: u32,
    pub object_properties: u32,
    pub global_properties: u32,
    pub enum_values: u32,
    pub funcdefs: u32,
    pub typedefs: u32,
    pub total_registrations: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapturedEnginePropertyV1 {
    pub ordinal: u64,
    pub property: EnginePropertyV1,
    pub value: u64,
    pub call_rva: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PointerTokenV1 {
    pub token_id: u32,
    pub primary_image_rva: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindCallbackPhaseV1 {
    Begin,
    End,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindCallbackEventV1 {
    pub ordinal: u64,
    pub callback_ordinal: u32,
    pub phase: BindCallbackPhaseV1,
    pub bind_order: i32,
    pub callback_pointer_token: u32,
    pub observation_rva: u32,
    pub counts: RegistryCountsV1,
    pub registry_sha256: Sha256Digest,
}

pub const REGISTRY_DELTA_CAPTURE_SCHEMA: &str = "gore.as.capture.registry-delta";
pub const POST_BIND_STATE_CAPTURE_SCHEMA: &str = "gore.as.capture.post-bind-state";
pub const REGISTRY_SUPPORT_CAPTURE_SCHEMA: &str = "gore.as.capture.registry-support";
pub const CAPTURE_JSON_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegistrySupportCaptureV1 {
    pub schema: String,
    pub schema_version: u32,
    pub host_stubs: Vec<HostStubDescriptorV1>,
    pub host_stub_pointers: Vec<HostStubPointerCaptureV1>,
    pub primitive_operations: Vec<PrimitiveTypeOperationsV1>,
    pub dynamic_script_operations: DynamicScriptTypeOperationsV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostStubPointerCaptureV1 {
    pub stub_id: u32,
    pub pointer_token: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegistryDeltaCaptureV1 {
    pub schema: String,
    pub schema_version: u32,
    pub bind_callback_ordinal: u32,
    pub entry: RegistrationEntryV1,
    pub result: PostBindResultV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PostBindStateCaptureV1 {
    pub schema: String,
    pub schema_version: u32,
    /// Present for a state mutation observed inside one bind callback; absent for final state.
    pub bind_callback_ordinal: Option<u32>,
    pub state_ordinal: u32,
    pub state: PostBindStateV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuildJitCaptureV1 {
    pub build_identifier: u32,
    pub shipping_cache_matches: bool,
    pub jit_info_present: bool,
    pub jit_guid_matches: bool,
    pub jit_database_cleared: bool,
    pub as_reference_debugging: bool,
    pub fork_opcode_table_201_212_present: bool,
    pub reference_debug_opcodes_emittable: bool,
    pub resolve_object_ptr_callback_registered: bool,
    pub precompiled_guid: [u8; 16],
    pub compiled_jit_guid: [u8; 16],
    pub get_build_identifier_rva: u32,
    pub get_static_jit_info_rva: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FrontendBoundaryKindV1 {
    InitialCompileEnter,
    PrecompiledDescriptorsRequested,
    PreprocessorConstructed,
    InitialCompileReturn,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrontendBoundaryEventV1 {
    pub ordinal: u64,
    pub kind: FrontendBoundaryKindV1,
    pub observation_rva: u32,
    pub module_count: u32,
    pub result_code: i32,
    pub config_sha256: Sha256Digest,
    pub input_sha256: Sha256Digest,
    pub output_sha256: Sha256Digest,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapturedFrontendConfigsV1 {
    pub preprocessor: PreprocessorConfigV1,
    pub class_generator: ClassGeneratorConfigV1,
    pub compiler_options: CompilerOptionsV1,
}

#[derive(Debug, Clone, PartialEq, Eq)]
// This private unit is a decoder-provenance capability, not a forward-compatibility marker.
#[allow(clippy::manual_non_exhaustive)]
pub struct DecodedCaptureV1 {
    /// Capability marker: only the sibling strict decoder may construct this projection.
    pub(super) _decoder_validated: (),
    pub header: CaptureHeaderV1,
    pub engine_properties: Vec<CapturedEnginePropertyV1>,
    pub pointer_tokens: BTreeMap<u32, PointerTokenV1>,
    pub bind_callbacks: Vec<BindCallbackEventV1>,
    pub registry_deltas: Vec<RegistryDeltaCaptureV1>,
    pub post_bind_mutations: Vec<PostBindStateCaptureV1>,
    pub final_post_bind_states: Vec<PostBindStateCaptureV1>,
    pub build_jit: BuildJitCaptureV1,
    pub frontend_boundaries: Vec<FrontendBoundaryEventV1>,
    pub frontend_configs: CapturedFrontendConfigsV1,
    /// Replay-ready projection; construction succeeds only after the existing strict registry
    /// validators bind every stub, registration result, and final state 1:1.
    pub ordered_engine_properties: OrderedEnginePropertiesV1,
    pub registration_trace: RegistrationTraceV1,
    pub post_bind_snapshot: PostBindSnapshotV1,
    pub sealed_stream_sha256: Sha256Digest,
}

#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RecordKindV1 {
    EngineProperty = 1,
    PointerToken = 2,
    BindCallback = 3,
    RegistryDeltaJson = 4,
    PostBindMutationJson = 5,
    FinalPostBindStateJson = 6,
    BuildJit = 7,
    FrontendBoundary = 8,
    FrontendConfigJson = 9,
    RegistrySupportJson = 10,
}

impl RecordKindV1 {
    pub(crate) fn parse(value: u16) -> Option<Self> {
        Some(match value {
            1 => Self::EngineProperty,
            2 => Self::PointerToken,
            3 => Self::BindCallback,
            4 => Self::RegistryDeltaJson,
            5 => Self::PostBindMutationJson,
            6 => Self::FinalPostBindStateJson,
            7 => Self::BuildJit,
            8 => Self::FrontendBoundary,
            9 => Self::FrontendConfigJson,
            10 => Self::RegistrySupportJson,
            _ => return None,
        })
    }
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FrontendConfigKindV1 {
    Preprocessor = 1,
    ClassGenerator = 2,
    CompilerOptions = 3,
}

impl FrontendConfigKindV1 {
    pub(crate) fn parse(value: u32) -> Option<Self> {
        Some(match value {
            1 => Self::Preprocessor,
            2 => Self::ClassGenerator,
            3 => Self::CompilerOptions,
            _ => return None,
        })
    }
}

pub(crate) fn engine_property_from_id(id: u32) -> Option<EnginePropertyV1> {
    use EnginePropertyV1::*;
    Some(match id {
        1 => AllowUnsafeReferences,
        2 => OptimizeBytecode,
        3 => CopyScriptSections,
        4 => MaxStackSize,
        5 => UseCharacterLiterals,
        6 => AllowMultilineStrings,
        7 => AllowImplicitHandleTypes,
        8 => BuildWithoutLineCues,
        9 => InitGlobalVarsAfterBuild,
        10 => RequireEnumScope,
        11 => ScriptScanner,
        12 => IncludeJitInstructions,
        13 => StringEncoding,
        14 => PropertyAccessorMode,
        15 => ExpandDefaultArrayToTemplate,
        16 => AutoGarbageCollect,
        17 => DisallowGlobalVars,
        18 => AlwaysImplementDefaultConstruct,
        19 => CompilerWarnings,
        20 => DisallowValueAssignForRefType,
        21 => AlterSyntaxNamedArgs,
        22 => DisableIntegerDivision,
        23 => DisallowEmptyListElements,
        24 => PrivatePropertyAsProtected,
        25 => AllowUnicodeIdentifiers,
        26 => HeredocTrimMode,
        27 => MaxNestedCalls,
        28 => GenericCallMode,
        29 => AutomaticImports,
        30 => TypecheckSwitchEnums,
        31 => AllowDoubleType,
        32 => FloatIsFloat64,
        33 => WarnOnFloatConstantsForDoubles,
        34 => WarnIntegerDivision,
        _ => return None,
    })
}
