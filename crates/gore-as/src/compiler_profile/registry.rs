//! Strict, versioned data model for replaying an AngelScript engine registry.
//!
//! These types describe profile payloads; they do not claim that a G1R payload has been
//! captured. Host pointers are deliberately unrepresentable. Registrations refer only to
//! compile-only stub descriptors which a standalone compiler must never invoke.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

use super::manifest::{EngineProfileV1, SealedBlobV1, Sha256Digest};

pub const ENGINE_PROPERTIES_SCHEMA: &str = "gore.as.engine-properties";
pub const REGISTRATION_TRACE_SCHEMA: &str = "gore.as.registration-trace";
pub const POST_BIND_SNAPSHOT_SCHEMA: &str = "gore.as.post-bind-snapshot";
pub const REGISTRY_SCHEMA_VERSION: u32 = 1;

const MAX_PROPERTIES_JSON_BYTES: usize = 1024 * 1024;
const MAX_TRACE_JSON_BYTES: usize = 256 * 1024 * 1024;
const MAX_SNAPSHOT_JSON_BYTES: usize = 128 * 1024 * 1024;
const MAX_ENGINE_PROPERTIES: usize = 4096;
const MAX_HOST_STUBS: usize = 1_000_000;
const MAX_REGISTRATIONS: usize = 2_000_000;
const MAX_DECLARATION_BYTES: usize = 64 * 1024;
const MAX_OBJECT_BYTES: u32 = 64 * 1024 * 1024;
const MAX_ALIGNMENT: u32 = 4096;
const MAX_OFFSET: u32 = 256 * 1024 * 1024;
const PUBLIC_OBJECT_FLAG_MASK: u32 = 0x003f_ffff;
const FUNCTION_TRAIT_MASK: u32 = 0x00ff_ffff;
const PROPERTIES_HASH_DOMAIN: &[u8] = b"gore-as-engine-properties-v1\0";
const TRACE_HASH_DOMAIN: &[u8] = b"gore-as-registration-trace-v1\0";
const SNAPSHOT_HASH_DOMAIN: &[u8] = b"gore-as-post-bind-snapshot-v1\0";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnginePropertyV1 {
    AllowUnsafeReferences,
    OptimizeBytecode,
    CopyScriptSections,
    MaxStackSize,
    UseCharacterLiterals,
    AllowMultilineStrings,
    AllowImplicitHandleTypes,
    BuildWithoutLineCues,
    InitGlobalVarsAfterBuild,
    RequireEnumScope,
    ScriptScanner,
    IncludeJitInstructions,
    StringEncoding,
    PropertyAccessorMode,
    ExpandDefaultArrayToTemplate,
    AutoGarbageCollect,
    DisallowGlobalVars,
    AlwaysImplementDefaultConstruct,
    CompilerWarnings,
    DisallowValueAssignForRefType,
    AlterSyntaxNamedArgs,
    DisableIntegerDivision,
    DisallowEmptyListElements,
    PrivatePropertyAsProtected,
    AllowUnicodeIdentifiers,
    HeredocTrimMode,
    MaxNestedCalls,
    GenericCallMode,
    AutomaticImports,
    TypecheckSwitchEnums,
    AllowDoubleType,
    FloatIsFloat64,
    WarnOnFloatConstantsForDoubles,
    WarnIntegerDivision,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnginePropertySettingV1 {
    pub ordinal: u32,
    pub property: EnginePropertyV1,
    pub value: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrderedEnginePropertiesV1 {
    pub schema: String,
    pub schema_version: u32,
    pub settings: Vec<EnginePropertySettingV1>,
    pub canonical_sha256: Sha256Digest,
}

impl OrderedEnginePropertiesV1 {
    pub fn seal(&mut self) -> Result<(), RegistryProfileError> {
        self.validate_structure()?;
        self.canonical_sha256 = self.computed_digest()?;
        Ok(())
    }

    pub fn validate(&self) -> Result<(), RegistryProfileError> {
        self.validate_structure()?;
        check_digest(
            "engine properties",
            self.canonical_sha256,
            self.computed_digest()?,
        )
    }

    pub fn from_json(bytes: &[u8]) -> Result<Self, RegistryProfileError> {
        parse_bounded(bytes, MAX_PROPERTIES_JSON_BYTES, "engine properties")
    }

    pub fn to_json(&self) -> Result<Vec<u8>, RegistryProfileError> {
        self.validate()?;
        Ok(serde_json::to_vec_pretty(self)?)
    }

    fn validate_structure(&self) -> Result<(), RegistryProfileError> {
        check_schema(&self.schema, self.schema_version, ENGINE_PROPERTIES_SCHEMA)?;
        check_count(
            "engine property settings",
            self.settings.len(),
            MAX_ENGINE_PROPERTIES,
        )?;
        for (index, setting) in self.settings.iter().enumerate() {
            check_ordinal("engine property setting", index, setting.ordinal)?;
        }
        Ok(())
    }

    fn computed_digest(&self) -> Result<Sha256Digest, RegistryProfileError> {
        let mut canonical = self.clone();
        canonical.canonical_sha256 = zero_digest();
        canonical_digest(PROPERTIES_HASH_DOMAIN, &canonical)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CallConventionV1 {
    Cdecl,
    Stdcall,
    ThiscallAsGlobal,
    Thiscall,
    CdeclObjectLast,
    CdeclObjectFirst,
    Generic,
    ThiscallObjectLast,
    ThiscallObjectFirst,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObjectBehaviourV1 {
    Construct,
    ListConstruct,
    Destruct,
    Factory,
    ListFactory,
    AddRef,
    Release,
    GetWeakrefFlag,
    TemplateCallback,
    GetRefCount,
    SetGcFlag,
    GetGcFlag,
    EnumRefs,
    ReleaseRefs,
}

/// Public `asEObjTypeFlags` bits supplied to `RegisterObjectType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ObjectTypeFlagsV1(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompileOnlyStubPurposeV1 {
    /// The descriptor exists only so registration can complete; execution must be rejected.
    CompileOnlyNeverInvoke,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum HostStubKindV1 {
    Callable { signature_sha256: Sha256Digest },
    Storage { byte_len: u32, alignment: u32 },
    Object { interface_sha256: Sha256Digest },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostStubDescriptorV1 {
    pub stub_id: u32,
    pub purpose: CompileOnlyStubPurposeV1,
    pub descriptor: HostStubKindV1,
}

/// Effective AngelScript configuration at the instant a registration call is made.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegistrationContextV1 {
    pub namespace: String,
    pub config_group: Option<String>,
    pub access_mask: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RegistrationEntryV1 {
    ObjectType {
        ordinal: u32,
        registration_id: u32,
        context: RegistrationContextV1,
        type_id: u32,
        declaration: String,
        byte_size: u32,
        alignment: u32,
        flags: ObjectTypeFlagsV1,
    },
    Interface {
        ordinal: u32,
        registration_id: u32,
        context: RegistrationContextV1,
        type_id: u32,
        declaration: String,
    },
    InterfaceMethod {
        ordinal: u32,
        registration_id: u32,
        context: RegistrationContextV1,
        function_id: u32,
        owner_type_id: u32,
        declaration: String,
    },
    ObjectProperty {
        ordinal: u32,
        registration_id: u32,
        context: RegistrationContextV1,
        property_id: u32,
        owner_type_id: u32,
        declaration: String,
        byte_offset: u32,
        composite_offset: u32,
        is_composite_indirect: bool,
        accessor_type: u32,
        is_protected: bool,
    },
    ObjectMethod {
        ordinal: u32,
        registration_id: u32,
        context: RegistrationContextV1,
        function_id: u32,
        owner_type_id: u32,
        declaration: String,
        call_convention: CallConventionV1,
        callable_stub_id: u32,
        auxiliary_object_stub_id: Option<u32>,
        composite_offset: u32,
        is_composite_indirect: bool,
        accessor_type: u32,
    },
    ObjectBehaviour {
        ordinal: u32,
        registration_id: u32,
        context: RegistrationContextV1,
        function_id: u32,
        owner_type_id: u32,
        behaviour: ObjectBehaviourV1,
        declaration: String,
        call_convention: CallConventionV1,
        callable_stub_id: u32,
        auxiliary_object_stub_id: Option<u32>,
        composite_offset: u32,
        is_composite_indirect: bool,
    },
    GlobalProperty {
        ordinal: u32,
        registration_id: u32,
        context: RegistrationContextV1,
        property_id: u32,
        declaration: String,
        storage_stub_id: u32,
    },
    GlobalFunction {
        ordinal: u32,
        registration_id: u32,
        context: RegistrationContextV1,
        function_id: u32,
        declaration: String,
        call_convention: CallConventionV1,
        callable_stub_id: u32,
        auxiliary_object_stub_id: Option<u32>,
    },
    Enum {
        ordinal: u32,
        registration_id: u32,
        context: RegistrationContextV1,
        type_id: u32,
        declaration: String,
    },
    EnumValue {
        ordinal: u32,
        registration_id: u32,
        context: RegistrationContextV1,
        owner_type_id: u32,
        name: String,
        value: i32,
    },
    Funcdef {
        ordinal: u32,
        registration_id: u32,
        context: RegistrationContextV1,
        type_id: u32,
        declaration: String,
    },
    Typedef {
        ordinal: u32,
        registration_id: u32,
        context: RegistrationContextV1,
        type_id: u32,
        name: String,
        target_declaration: String,
    },
    StringFactory {
        ordinal: u32,
        registration_id: u32,
        context: RegistrationContextV1,
        string_type_declaration: String,
        factory_object_stub_id: u32,
    },
    DefaultArrayType {
        ordinal: u32,
        registration_id: u32,
        context: RegistrationContextV1,
        type_declaration: String,
    },
}

impl RegistrationEntryV1 {
    pub fn ordinal(&self) -> u32 {
        match self {
            Self::ObjectType { ordinal, .. }
            | Self::Interface { ordinal, .. }
            | Self::InterfaceMethod { ordinal, .. }
            | Self::ObjectProperty { ordinal, .. }
            | Self::ObjectMethod { ordinal, .. }
            | Self::ObjectBehaviour { ordinal, .. }
            | Self::GlobalProperty { ordinal, .. }
            | Self::GlobalFunction { ordinal, .. }
            | Self::Enum { ordinal, .. }
            | Self::EnumValue { ordinal, .. }
            | Self::Funcdef { ordinal, .. }
            | Self::Typedef { ordinal, .. }
            | Self::StringFactory { ordinal, .. }
            | Self::DefaultArrayType { ordinal, .. } => *ordinal,
        }
    }

    pub fn registration_id(&self) -> u32 {
        match self {
            Self::ObjectType {
                registration_id, ..
            }
            | Self::Interface {
                registration_id, ..
            }
            | Self::InterfaceMethod {
                registration_id, ..
            }
            | Self::ObjectProperty {
                registration_id, ..
            }
            | Self::ObjectMethod {
                registration_id, ..
            }
            | Self::ObjectBehaviour {
                registration_id, ..
            }
            | Self::GlobalProperty {
                registration_id, ..
            }
            | Self::GlobalFunction {
                registration_id, ..
            }
            | Self::Enum {
                registration_id, ..
            }
            | Self::EnumValue {
                registration_id, ..
            }
            | Self::Funcdef {
                registration_id, ..
            }
            | Self::Typedef {
                registration_id, ..
            }
            | Self::StringFactory {
                registration_id, ..
            }
            | Self::DefaultArrayType {
                registration_id, ..
            } => *registration_id,
        }
    }

    /// Effective default namespace at the point of this registration.
    pub fn context(&self) -> &RegistrationContextV1 {
        match self {
            Self::ObjectType { context, .. }
            | Self::Interface { context, .. }
            | Self::InterfaceMethod { context, .. }
            | Self::ObjectProperty { context, .. }
            | Self::ObjectMethod { context, .. }
            | Self::ObjectBehaviour { context, .. }
            | Self::GlobalProperty { context, .. }
            | Self::GlobalFunction { context, .. }
            | Self::Enum { context, .. }
            | Self::EnumValue { context, .. }
            | Self::Funcdef { context, .. }
            | Self::Typedef { context, .. }
            | Self::StringFactory { context, .. }
            | Self::DefaultArrayType { context, .. } => context,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegistrationTraceV1 {
    pub schema: String,
    pub schema_version: u32,
    pub host_stubs: Vec<HostStubDescriptorV1>,
    pub entries: Vec<RegistrationEntryV1>,
    pub canonical_sha256: Sha256Digest,
}

impl RegistrationTraceV1 {
    pub fn seal(&mut self) -> Result<(), RegistryProfileError> {
        self.validate_structure()?;
        self.canonical_sha256 = self.computed_digest()?;
        Ok(())
    }

    pub fn validate(&self) -> Result<(), RegistryProfileError> {
        self.validate_structure()?;
        check_digest(
            "registration trace",
            self.canonical_sha256,
            self.computed_digest()?,
        )
    }

    pub fn from_json(bytes: &[u8]) -> Result<Self, RegistryProfileError> {
        parse_bounded(bytes, MAX_TRACE_JSON_BYTES, "registration trace")
    }

    pub fn to_json(&self) -> Result<Vec<u8>, RegistryProfileError> {
        self.validate()?;
        Ok(serde_json::to_vec_pretty(self)?)
    }

    fn computed_digest(&self) -> Result<Sha256Digest, RegistryProfileError> {
        let mut canonical = self.clone();
        canonical.canonical_sha256 = zero_digest();
        canonical_digest(TRACE_HASH_DOMAIN, &canonical)
    }

    fn validate_structure(&self) -> Result<(), RegistryProfileError> {
        check_schema(&self.schema, self.schema_version, REGISTRATION_TRACE_SCHEMA)?;
        check_count("host stubs", self.host_stubs.len(), MAX_HOST_STUBS)?;
        check_count(
            "registration entries",
            self.entries.len(),
            MAX_REGISTRATIONS,
        )?;
        if self.entries.is_empty() {
            return invalid("entries", "registration trace must not be empty");
        }

        let mut stub_kinds = BTreeMap::new();
        for (index, stub) in self.host_stubs.iter().enumerate() {
            check_ordinal("host stub", index, stub.stub_id)?;
            match &stub.descriptor {
                HostStubKindV1::Callable { .. } | HostStubKindV1::Object { .. } => {}
                HostStubKindV1::Storage {
                    byte_len,
                    alignment,
                } => {
                    if *byte_len > MAX_OBJECT_BYTES {
                        return invalid("host_stubs.storage.byte_len", "storage is too large");
                    }
                    check_alignment("host_stubs.storage.alignment", *alignment)?;
                }
            }
            stub_kinds.insert(stub.stub_id, &stub.descriptor);
        }

        let mut used_stubs = BTreeSet::new();
        let mut type_ids = BTreeSet::new();
        let mut object_types = BTreeMap::new();
        let mut interface_types = BTreeSet::new();
        let mut enum_types = BTreeSet::new();
        let mut function_ids = BTreeSet::new();
        let mut property_ids = BTreeSet::new();
        let mut string_factory_seen = false;
        let mut default_array_seen = false;

        for (index, entry) in self.entries.iter().enumerate() {
            check_ordinal("registration entry", index, entry.ordinal())?;
            check_ordinal("registration id", index, entry.registration_id())?;
            validate_registration_context(entry.context())?;
            match entry {
                RegistrationEntryV1::ObjectType {
                    type_id,
                    declaration,
                    byte_size,
                    alignment,
                    flags,
                    ..
                } => {
                    unique("type_id", &mut type_ids, *type_id)?;
                    validate_declaration("object_type.declaration", declaration)?;
                    if *byte_size > MAX_OBJECT_BYTES {
                        return invalid("object_type.byte_size", "object is too large");
                    }
                    check_alignment("object_type.alignment", *alignment)?;
                    if flags.0 & !PUBLIC_OBJECT_FLAG_MASK != 0 {
                        return invalid("object_type.flags", "contains non-public or unknown bits");
                    }
                    object_types.insert(*type_id, (*byte_size, *alignment));
                }
                RegistrationEntryV1::Interface {
                    type_id,
                    declaration,
                    ..
                } => {
                    unique("type_id", &mut type_ids, *type_id)?;
                    validate_declaration("interface.declaration", declaration)?;
                    interface_types.insert(*type_id);
                }
                RegistrationEntryV1::InterfaceMethod {
                    function_id,
                    owner_type_id,
                    declaration,
                    ..
                } => {
                    unique("function_id", &mut function_ids, *function_id)?;
                    if !interface_types.contains(owner_type_id) {
                        return reference(
                            "interface_method.owner_type_id",
                            *owner_type_id,
                            "earlier interface",
                        );
                    }
                    validate_declaration("interface_method.declaration", declaration)?;
                }
                RegistrationEntryV1::ObjectProperty {
                    property_id,
                    owner_type_id,
                    declaration,
                    byte_offset,
                    composite_offset,
                    accessor_type,
                    ..
                } => {
                    unique("property_id", &mut property_ids, *property_id)?;
                    require_object(
                        &object_types,
                        *owner_type_id,
                        "object_property.owner_type_id",
                    )?;
                    validate_declaration("object_property.declaration", declaration)?;
                    if *byte_offset > MAX_OFFSET {
                        return invalid("object_property.byte_offset", "offset is too large");
                    }
                    if *composite_offset > MAX_OFFSET {
                        return invalid("object_property.composite_offset", "offset is too large");
                    }
                    if *accessor_type > u8::MAX as u32 {
                        return invalid("object_property.accessor_type", "must fit in uint8");
                    }
                }
                RegistrationEntryV1::ObjectMethod {
                    function_id,
                    owner_type_id,
                    declaration,
                    callable_stub_id,
                    auxiliary_object_stub_id,
                    composite_offset,
                    accessor_type,
                    ..
                } => {
                    if *composite_offset > MAX_OFFSET {
                        return invalid("object_method.composite_offset", "offset is too large");
                    }
                    if *accessor_type > u8::MAX as u32 {
                        return invalid("object_method.accessor_type", "must fit in uint8");
                    }
                    validate_object_callable(
                        &object_types,
                        &stub_kinds,
                        &mut used_stubs,
                        &mut function_ids,
                        *function_id,
                        *owner_type_id,
                        declaration,
                        *callable_stub_id,
                        *auxiliary_object_stub_id,
                    )?;
                }
                RegistrationEntryV1::ObjectBehaviour {
                    function_id,
                    owner_type_id,
                    declaration,
                    callable_stub_id,
                    auxiliary_object_stub_id,
                    composite_offset,
                    ..
                } => {
                    if *composite_offset > MAX_OFFSET {
                        return invalid("object_behaviour.composite_offset", "offset is too large");
                    }
                    validate_object_callable(
                        &object_types,
                        &stub_kinds,
                        &mut used_stubs,
                        &mut function_ids,
                        *function_id,
                        *owner_type_id,
                        declaration,
                        *callable_stub_id,
                        *auxiliary_object_stub_id,
                    )?;
                }
                RegistrationEntryV1::GlobalProperty {
                    property_id,
                    declaration,
                    storage_stub_id,
                    ..
                } => {
                    unique("property_id", &mut property_ids, *property_id)?;
                    validate_declaration("global_property.declaration", declaration)?;
                    require_stub(
                        &stub_kinds,
                        &mut used_stubs,
                        *storage_stub_id,
                        StubClass::Storage,
                    )?;
                }
                RegistrationEntryV1::GlobalFunction {
                    function_id,
                    declaration,
                    callable_stub_id,
                    auxiliary_object_stub_id,
                    ..
                } => {
                    unique("function_id", &mut function_ids, *function_id)?;
                    validate_declaration("global_function.declaration", declaration)?;
                    require_stub(
                        &stub_kinds,
                        &mut used_stubs,
                        *callable_stub_id,
                        StubClass::Callable,
                    )?;
                    if let Some(stub_id) = auxiliary_object_stub_id {
                        require_stub(&stub_kinds, &mut used_stubs, *stub_id, StubClass::Object)?;
                    }
                }
                RegistrationEntryV1::Enum {
                    type_id,
                    declaration,
                    ..
                } => {
                    unique("type_id", &mut type_ids, *type_id)?;
                    validate_declaration("enum.declaration", declaration)?;
                    enum_types.insert(*type_id);
                }
                RegistrationEntryV1::EnumValue {
                    owner_type_id,
                    name,
                    ..
                } => {
                    if !enum_types.contains(owner_type_id) {
                        return reference(
                            "enum_value.owner_type_id",
                            *owner_type_id,
                            "earlier enum",
                        );
                    }
                    validate_identifier("enum_value.name", name)?;
                }
                RegistrationEntryV1::Funcdef {
                    type_id,
                    declaration,
                    ..
                } => {
                    unique("type_id", &mut type_ids, *type_id)?;
                    validate_declaration("funcdef.declaration", declaration)?;
                }
                RegistrationEntryV1::Typedef {
                    type_id,
                    name,
                    target_declaration,
                    ..
                } => {
                    unique("type_id", &mut type_ids, *type_id)?;
                    validate_identifier("typedef.name", name)?;
                    validate_declaration("typedef.target_declaration", target_declaration)?;
                }
                RegistrationEntryV1::StringFactory {
                    string_type_declaration,
                    factory_object_stub_id,
                    ..
                } => {
                    if string_factory_seen {
                        return invalid("string_factory", "registered more than once");
                    }
                    string_factory_seen = true;
                    validate_declaration(
                        "string_factory.string_type_declaration",
                        string_type_declaration,
                    )?;
                    require_stub(
                        &stub_kinds,
                        &mut used_stubs,
                        *factory_object_stub_id,
                        StubClass::Object,
                    )?;
                }
                RegistrationEntryV1::DefaultArrayType {
                    type_declaration, ..
                } => {
                    if default_array_seen {
                        return invalid("default_array_type", "registered more than once");
                    }
                    default_array_seen = true;
                    validate_declaration("default_array_type.type_declaration", type_declaration)?;
                }
            }
        }
        if used_stubs.len() != self.host_stubs.len() {
            return invalid("host_stubs", "contains an unreferenced descriptor");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompileOutModeV1 {
    CompileCalls,
    CompileOutEntirely,
    ReplaceWithFirstParam,
    CompileOutAsMethodChain,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FirstParamMetadataV1 {
    None,
    ScriptFunction,
    ScriptObjectType,
}

/// Complete compile-relevant state after G1R's bind callbacks and direct fork mutations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PostBindStateV1 {
    ObjectType {
        type_id: u32,
        byte_size: u32,
        alignment: u32,
        flags: u32,
        base_type_id: Option<u32>,
        shadow_type_id: Option<u32>,
        interface_type_ids: Vec<u32>,
        interface_vft_offsets: Vec<u32>,
        has_implicit_constructors: bool,
        accepts_value_subtype: bool,
        accepts_reference_subtype: bool,
        is_invalid_generated_type: bool,
    },
    ObjectProperty {
        property_id: u32,
        byte_offset: u32,
        access_mask: u32,
        composite_offset: u32,
        is_composite_indirect: bool,
        is_private: bool,
        is_protected: bool,
        is_app_bind_property: bool,
        exposed_type: u32,
    },
    Function {
        function_id: u32,
        trait_bits: u32,
        exposed_type: u32,
        hidden_argument_index: Option<u8>,
        hidden_argument_default: Option<String>,
        determines_output_type_argument_index: Option<u8>,
        compile_out_mode: CompileOutModeV1,
        first_param_metadata: FirstParamMetadataV1,
    },
    GlobalProperty {
        property_id: u32,
        is_pure_constant: bool,
        pure_constant_value: Option<u64>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PostBindResultV1 {
    ObjectType {
        engine_type_id: u32,
    },
    Interface {
        engine_type_id: u32,
    },
    InterfaceMethod {
        owner_engine_type_id: u32,
        engine_function_id: u32,
    },
    ObjectProperty {
        owner_engine_type_id: u32,
        property_index: u32,
    },
    ObjectMethod {
        owner_engine_type_id: u32,
        engine_function_id: u32,
    },
    ObjectBehaviour {
        owner_engine_type_id: u32,
        engine_function_id: u32,
    },
    GlobalProperty {
        global_property_index: u32,
    },
    GlobalFunction {
        engine_function_id: u32,
    },
    Enum {
        engine_type_id: u32,
    },
    EnumValue {
        owner_engine_type_id: u32,
        value_index: u32,
    },
    Funcdef {
        engine_type_id: u32,
    },
    Typedef {
        engine_type_id: u32,
    },
    StringFactory {
        installed: bool,
    },
    DefaultArrayType {
        installed: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PostBindEntryV1 {
    pub ordinal: u32,
    pub trace_registration_id: u32,
    pub result: PostBindResultV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PostBindSnapshotV1 {
    pub schema: String,
    pub schema_version: u32,
    pub engine_properties_sha256: Sha256Digest,
    pub registration_trace_sha256: Sha256Digest,
    pub entries: Vec<PostBindEntryV1>,
    pub final_states: Vec<PostBindStateV1>,
    pub canonical_sha256: Sha256Digest,
}

impl PostBindSnapshotV1 {
    pub fn seal(&mut self) -> Result<(), RegistryProfileError> {
        self.validate_structure()?;
        self.canonical_sha256 = self.computed_digest()?;
        Ok(())
    }

    pub fn validate(&self) -> Result<(), RegistryProfileError> {
        self.validate_structure()?;
        check_digest(
            "post-bind snapshot",
            self.canonical_sha256,
            self.computed_digest()?,
        )
    }

    pub fn validate_against(
        &self,
        properties: &OrderedEnginePropertiesV1,
        trace: &RegistrationTraceV1,
    ) -> Result<(), RegistryProfileError> {
        properties.validate()?;
        trace.validate()?;
        self.validate()?;
        check_digest(
            "snapshot engine-properties reference",
            self.engine_properties_sha256,
            properties.canonical_sha256,
        )?;
        check_digest(
            "snapshot registration-trace reference",
            self.registration_trace_sha256,
            trace.canonical_sha256,
        )?;
        if self.entries.len() != trace.entries.len() {
            return invalid(
                "post_bind.entries",
                "must cover every trace entry exactly once",
            );
        }

        let mut type_results = BTreeMap::new();
        let mut engine_type_ids = BTreeSet::new();
        let mut engine_function_ids = BTreeSet::new();
        let mut global_property_indices = BTreeSet::new();
        let mut member_indices = BTreeSet::new();
        for (trace_entry, snapshot_entry) in trace.entries.iter().zip(&self.entries) {
            if snapshot_entry.trace_registration_id != trace_entry.registration_id() {
                return invalid(
                    "post_bind.trace_registration_id",
                    "does not match trace order",
                );
            }
            validate_snapshot_pair(
                trace_entry,
                &snapshot_entry.result,
                &mut type_results,
                &mut engine_type_ids,
                &mut engine_function_ids,
                &mut global_property_indices,
                &mut member_indices,
            )?;
        }
        validate_final_states(trace, &self.final_states)?;
        Ok(())
    }

    pub fn from_json(bytes: &[u8]) -> Result<Self, RegistryProfileError> {
        parse_bounded(bytes, MAX_SNAPSHOT_JSON_BYTES, "post-bind snapshot")
    }

    pub fn to_json(&self) -> Result<Vec<u8>, RegistryProfileError> {
        self.validate()?;
        Ok(serde_json::to_vec_pretty(self)?)
    }

    fn validate_structure(&self) -> Result<(), RegistryProfileError> {
        check_schema(&self.schema, self.schema_version, POST_BIND_SNAPSHOT_SCHEMA)?;
        check_count("post-bind entries", self.entries.len(), MAX_REGISTRATIONS)?;
        check_count(
            "post-bind final states",
            self.final_states.len(),
            MAX_REGISTRATIONS,
        )?;
        for (index, entry) in self.entries.iter().enumerate() {
            check_ordinal("post-bind entry", index, entry.ordinal)?;
            check_ordinal(
                "post-bind trace registration",
                index,
                entry.trace_registration_id,
            )?;
            match &entry.result {
                PostBindResultV1::StringFactory { installed }
                | PostBindResultV1::DefaultArrayType { installed }
                    if !installed =>
                {
                    return invalid(
                        "post_bind.installed",
                        "successful snapshot result must be installed",
                    );
                }
                _ => {}
            }
        }
        validate_final_state_bounds(&self.final_states)?;
        Ok(())
    }

    fn computed_digest(&self) -> Result<Sha256Digest, RegistryProfileError> {
        let mut canonical = self.clone();
        canonical.canonical_sha256 = zero_digest();
        canonical_digest(SNAPSHOT_HASH_DOMAIN, &canonical)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedRegistryProfileV1 {
    pub engine_properties: OrderedEnginePropertiesV1,
    pub registration_trace: RegistrationTraceV1,
    pub post_bind_snapshot: PostBindSnapshotV1,
}

/// Parse and bind all three sealed engine payloads to the manifest.
pub fn validate_engine_profile_payloads(
    manifest: &EngineProfileV1,
    engine_properties_json: &[u8],
    registration_trace_json: &[u8],
    post_bind_snapshot_json: &[u8],
) -> Result<ValidatedRegistryProfileV1, RegistryProfileError> {
    check_blob(
        &manifest.ordered_engine_properties,
        engine_properties_json,
        "ordered engine properties",
    )?;
    check_blob(
        &manifest.registration_trace,
        registration_trace_json,
        "registration trace",
    )?;
    check_blob(
        &manifest.post_bind_snapshot,
        post_bind_snapshot_json,
        "post-bind snapshot",
    )?;
    let engine_properties = OrderedEnginePropertiesV1::from_json(engine_properties_json)?;
    let registration_trace = RegistrationTraceV1::from_json(registration_trace_json)?;
    if manifest.registration_trace_count != registration_trace.entries.len() as u64 {
        return invalid(
            "engine.registration_trace_count",
            "does not match typed trace length",
        );
    }
    let post_bind_snapshot = PostBindSnapshotV1::from_json(post_bind_snapshot_json)?;
    post_bind_snapshot.validate_against(&engine_properties, &registration_trace)?;
    Ok(ValidatedRegistryProfileV1 {
        engine_properties,
        registration_trace,
        post_bind_snapshot,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StubClass {
    Callable,
    Storage,
    Object,
}

fn require_stub<'a>(
    stubs: &BTreeMap<u32, &'a HostStubKindV1>,
    used: &mut BTreeSet<u32>,
    stub_id: u32,
    expected: StubClass,
) -> Result<(), RegistryProfileError> {
    let descriptor = stubs
        .get(&stub_id)
        .ok_or_else(|| RegistryProfileError::InvalidReference {
            field: "stub_id".to_owned(),
            id: stub_id,
            expected: "host stub".to_owned(),
        })?;
    let actual = match descriptor {
        HostStubKindV1::Callable { .. } => StubClass::Callable,
        HostStubKindV1::Storage { .. } => StubClass::Storage,
        HostStubKindV1::Object { .. } => StubClass::Object,
    };
    if actual != expected {
        return invalid("stub_id", "references the wrong descriptor kind");
    }
    used.insert(stub_id);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn validate_object_callable(
    object_types: &BTreeMap<u32, (u32, u32)>,
    stub_kinds: &BTreeMap<u32, &HostStubKindV1>,
    used_stubs: &mut BTreeSet<u32>,
    function_ids: &mut BTreeSet<u32>,
    function_id: u32,
    owner_type_id: u32,
    declaration: &str,
    callable_stub_id: u32,
    auxiliary_object_stub_id: Option<u32>,
) -> Result<(), RegistryProfileError> {
    unique("function_id", function_ids, function_id)?;
    require_object(object_types, owner_type_id, "object callable owner_type_id")?;
    validate_declaration("object callable declaration", declaration)?;
    require_stub(
        stub_kinds,
        used_stubs,
        callable_stub_id,
        StubClass::Callable,
    )?;
    if let Some(stub_id) = auxiliary_object_stub_id {
        require_stub(stub_kinds, used_stubs, stub_id, StubClass::Object)?;
    }
    Ok(())
}

fn validate_final_state_bounds(states: &[PostBindStateV1]) -> Result<(), RegistryProfileError> {
    for state in states {
        match state {
            PostBindStateV1::ObjectType {
                byte_size,
                alignment,
                interface_type_ids,
                interface_vft_offsets,
                ..
            } => {
                if *byte_size > MAX_OBJECT_BYTES {
                    return invalid("post_bind.object_type.byte_size", "object is too large");
                }
                check_alignment("post_bind.object_type.alignment", *alignment)?;
                check_count(
                    "post-bind object interfaces",
                    interface_type_ids.len(),
                    MAX_REGISTRATIONS,
                )?;
                if interface_type_ids.len() != interface_vft_offsets.len() {
                    return invalid(
                        "post_bind.object_type.interface_vft_offsets",
                        "must correspond 1:1 with interface_type_ids",
                    );
                }
            }
            PostBindStateV1::ObjectProperty {
                byte_offset,
                composite_offset,
                exposed_type,
                ..
            } => {
                if *byte_offset > MAX_OFFSET || *composite_offset > MAX_OFFSET {
                    return invalid("post_bind.object_property.offset", "offset is too large");
                }
                if *exposed_type > u8::MAX as u32 {
                    return invalid(
                        "post_bind.object_property.exposed_type",
                        "must fit in uint8",
                    );
                }
            }
            PostBindStateV1::Function {
                trait_bits,
                exposed_type,
                hidden_argument_index,
                hidden_argument_default,
                ..
            } => {
                if *trait_bits & !FUNCTION_TRAIT_MASK != 0 {
                    return invalid(
                        "post_bind.function.trait_bits",
                        "contains unknown fork trait bits",
                    );
                }
                if *exposed_type > u8::MAX as u32 {
                    return invalid("post_bind.function.exposed_type", "must fit in uint8");
                }
                if hidden_argument_index.is_some() != hidden_argument_default.is_some() {
                    return invalid(
                        "post_bind.function.hidden_argument_default",
                        "must be present exactly when hidden_argument_index is present",
                    );
                }
                if let Some(default) = hidden_argument_default {
                    validate_declaration("post_bind.function.hidden_argument_default", default)?;
                }
            }
            PostBindStateV1::GlobalProperty {
                is_pure_constant,
                pure_constant_value,
                ..
            } => {
                if *is_pure_constant != pure_constant_value.is_some() {
                    return invalid(
                        "post_bind.global_property.pure_constant_value",
                        "must be present exactly for a pure constant",
                    );
                }
            }
        }
    }
    Ok(())
}

fn validate_final_states(
    trace: &RegistrationTraceV1,
    states: &[PostBindStateV1],
) -> Result<(), RegistryProfileError> {
    let mut expected_types = BTreeSet::new();
    let mut expected_interfaces = BTreeSet::new();
    let mut expected_properties = BTreeSet::new();
    let mut expected_functions = BTreeSet::new();
    let mut expected_globals = BTreeSet::new();
    for entry in &trace.entries {
        match entry {
            RegistrationEntryV1::ObjectType { type_id, .. } => {
                expected_types.insert(*type_id);
            }
            RegistrationEntryV1::Interface { type_id, .. } => {
                expected_types.insert(*type_id);
                expected_interfaces.insert(*type_id);
            }
            RegistrationEntryV1::ObjectProperty { property_id, .. } => {
                expected_properties.insert(*property_id);
            }
            RegistrationEntryV1::InterfaceMethod { function_id, .. }
            | RegistrationEntryV1::ObjectMethod { function_id, .. }
            | RegistrationEntryV1::ObjectBehaviour { function_id, .. }
            | RegistrationEntryV1::GlobalFunction { function_id, .. } => {
                expected_functions.insert(*function_id);
            }
            RegistrationEntryV1::GlobalProperty { property_id, .. } => {
                expected_globals.insert(*property_id);
            }
            _ => {}
        }
    }

    let mut actual_types = BTreeSet::new();
    let mut actual_properties = BTreeSet::new();
    let mut actual_functions = BTreeSet::new();
    let mut actual_globals = BTreeSet::new();
    for state in states {
        match state {
            PostBindStateV1::ObjectType {
                type_id,
                base_type_id,
                shadow_type_id,
                interface_type_ids,
                ..
            } => {
                unique("post-bind type state", &mut actual_types, *type_id)?;
                for (field, reference_id) in [
                    ("post_bind.object_type.base_type_id", *base_type_id),
                    ("post_bind.object_type.shadow_type_id", *shadow_type_id),
                ] {
                    if let Some(reference_id) = reference_id {
                        if !expected_types.contains(&reference_id) {
                            return reference(field, reference_id, "registered object/interface");
                        }
                    }
                }
                let mut seen_interfaces = BTreeSet::new();
                for interface_id in interface_type_ids {
                    unique(
                        "post-bind object interface",
                        &mut seen_interfaces,
                        *interface_id,
                    )?;
                    if !expected_interfaces.contains(interface_id) {
                        return reference(
                            "post_bind.object_type.interface_type_ids",
                            *interface_id,
                            "registered interface",
                        );
                    }
                }
            }
            PostBindStateV1::ObjectProperty { property_id, .. } => {
                unique(
                    "post-bind object property state",
                    &mut actual_properties,
                    *property_id,
                )?;
            }
            PostBindStateV1::Function { function_id, .. } => {
                unique(
                    "post-bind function state",
                    &mut actual_functions,
                    *function_id,
                )?;
            }
            PostBindStateV1::GlobalProperty { property_id, .. } => {
                unique(
                    "post-bind global property state",
                    &mut actual_globals,
                    *property_id,
                )?;
            }
        }
    }
    for (field, expected, actual) in [
        (
            "post_bind.object_type_states",
            &expected_types,
            &actual_types,
        ),
        (
            "post_bind.object_property_states",
            &expected_properties,
            &actual_properties,
        ),
        (
            "post_bind.function_states",
            &expected_functions,
            &actual_functions,
        ),
        (
            "post_bind.global_property_states",
            &expected_globals,
            &actual_globals,
        ),
    ] {
        if expected != actual {
            return invalid(
                field,
                "must cover every corresponding registration exactly once",
            );
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn validate_snapshot_pair(
    trace: &RegistrationEntryV1,
    result: &PostBindResultV1,
    type_results: &mut BTreeMap<u32, u32>,
    engine_type_ids: &mut BTreeSet<u32>,
    engine_function_ids: &mut BTreeSet<u32>,
    global_property_indices: &mut BTreeSet<u32>,
    member_indices: &mut BTreeSet<(u32, u32)>,
) -> Result<(), RegistryProfileError> {
    match (trace, result) {
        (
            RegistrationEntryV1::ObjectType { type_id, .. },
            PostBindResultV1::ObjectType { engine_type_id },
        ) => {
            unique("engine_type_id", engine_type_ids, *engine_type_id)?;
            type_results.insert(*type_id, *engine_type_id);
        }
        (
            RegistrationEntryV1::Interface { type_id, .. },
            PostBindResultV1::Interface { engine_type_id },
        ) => {
            unique("engine_type_id", engine_type_ids, *engine_type_id)?;
            type_results.insert(*type_id, *engine_type_id);
        }
        (
            RegistrationEntryV1::InterfaceMethod { owner_type_id, .. },
            PostBindResultV1::InterfaceMethod {
                owner_engine_type_id,
                engine_function_id,
            },
        ) => {
            require_owner_result(type_results, *owner_type_id, *owner_engine_type_id)?;
            unique(
                "engine_function_id",
                engine_function_ids,
                *engine_function_id,
            )?;
        }
        (
            RegistrationEntryV1::ObjectProperty { owner_type_id, .. },
            PostBindResultV1::ObjectProperty {
                owner_engine_type_id,
                property_index,
            },
        ) => {
            require_owner_result(type_results, *owner_type_id, *owner_engine_type_id)?;
            unique(
                "object property index",
                member_indices,
                (*owner_engine_type_id, *property_index),
            )?;
        }
        (
            RegistrationEntryV1::ObjectMethod { owner_type_id, .. },
            PostBindResultV1::ObjectMethod {
                owner_engine_type_id,
                engine_function_id,
            },
        )
        | (
            RegistrationEntryV1::ObjectBehaviour { owner_type_id, .. },
            PostBindResultV1::ObjectBehaviour {
                owner_engine_type_id,
                engine_function_id,
            },
        ) => {
            require_owner_result(type_results, *owner_type_id, *owner_engine_type_id)?;
            unique(
                "engine_function_id",
                engine_function_ids,
                *engine_function_id,
            )?;
        }
        (
            RegistrationEntryV1::GlobalProperty { .. },
            PostBindResultV1::GlobalProperty {
                global_property_index,
            },
        ) => unique(
            "global_property_index",
            global_property_indices,
            *global_property_index,
        )?,
        (
            RegistrationEntryV1::GlobalFunction { .. },
            PostBindResultV1::GlobalFunction { engine_function_id },
        ) => unique(
            "engine_function_id",
            engine_function_ids,
            *engine_function_id,
        )?,
        (RegistrationEntryV1::Enum { type_id, .. }, PostBindResultV1::Enum { engine_type_id })
        | (
            RegistrationEntryV1::Funcdef { type_id, .. },
            PostBindResultV1::Funcdef { engine_type_id },
        )
        | (
            RegistrationEntryV1::Typedef { type_id, .. },
            PostBindResultV1::Typedef { engine_type_id },
        ) => {
            unique("engine_type_id", engine_type_ids, *engine_type_id)?;
            type_results.insert(*type_id, *engine_type_id);
        }
        (
            RegistrationEntryV1::EnumValue { owner_type_id, .. },
            PostBindResultV1::EnumValue {
                owner_engine_type_id,
                value_index,
            },
        ) => {
            require_owner_result(type_results, *owner_type_id, *owner_engine_type_id)?;
            unique(
                "enum value index",
                member_indices,
                (*owner_engine_type_id, *value_index),
            )?;
        }
        (
            RegistrationEntryV1::StringFactory { .. },
            PostBindResultV1::StringFactory { installed: true },
        )
        | (
            RegistrationEntryV1::DefaultArrayType { .. },
            PostBindResultV1::DefaultArrayType { installed: true },
        ) => {}
        _ => {
            return invalid(
                "post_bind.result.kind",
                "does not match its trace entry kind",
            )
        }
    }
    Ok(())
}

fn require_owner_result(
    results: &BTreeMap<u32, u32>,
    type_id: u32,
    actual: u32,
) -> Result<(), RegistryProfileError> {
    match results.get(&type_id) {
        Some(expected) if *expected == actual => Ok(()),
        _ => invalid(
            "post_bind.owner_engine_type_id",
            "does not match the trace owner type",
        ),
    }
}

fn require_object(
    objects: &BTreeMap<u32, (u32, u32)>,
    id: u32,
    field: &'static str,
) -> Result<(), RegistryProfileError> {
    if objects.contains_key(&id) {
        Ok(())
    } else {
        reference(field, id, "earlier object type")
    }
}

fn validate_declaration(field: &'static str, value: &str) -> Result<(), RegistryProfileError> {
    validate_text(field, value, false)
}
fn validate_identifier(field: &'static str, value: &str) -> Result<(), RegistryProfileError> {
    validate_text(field, value, true)
}
fn validate_registration_context(
    context: &RegistrationContextV1,
) -> Result<(), RegistryProfileError> {
    validate_namespace(&context.namespace)?;
    if let Some(config_group) = &context.config_group {
        validate_text("registration_context.config_group", config_group, false)?;
    }
    Ok(())
}
fn validate_namespace(value: &str) -> Result<(), RegistryProfileError> {
    if value.is_empty() {
        return Ok(());
    }
    if value.len() > MAX_DECLARATION_BYTES
        || value
            .split("::")
            .any(|part| part.is_empty() || !part.chars().all(|c| c == '_' || c.is_alphanumeric()))
    {
        return invalid(
            "registration.namespace",
            "must be empty or a bounded ::-separated identifier",
        );
    }
    Ok(())
}
fn validate_text(
    field: &'static str,
    value: &str,
    identifier: bool,
) -> Result<(), RegistryProfileError> {
    if value.is_empty()
        || value.len() > MAX_DECLARATION_BYTES
        || value.contains('\0')
        || value.chars().any(|c| c.is_control())
    {
        return invalid(
            field,
            "must be non-empty, bounded UTF-8 without control characters",
        );
    }
    if identifier && !value.chars().all(|c| c == '_' || c.is_alphanumeric()) {
        return invalid(field, "is not a neutral identifier");
    }
    Ok(())
}

fn check_alignment(field: &'static str, alignment: u32) -> Result<(), RegistryProfileError> {
    if alignment == 0 || alignment > MAX_ALIGNMENT || !alignment.is_power_of_two() {
        invalid(field, "must be a bounded non-zero power of two")
    } else {
        Ok(())
    }
}

fn check_schema(
    actual: &str,
    version: u32,
    expected: &'static str,
) -> Result<(), RegistryProfileError> {
    if actual != expected || version != REGISTRY_SCHEMA_VERSION {
        Err(RegistryProfileError::Schema {
            expected: format!("{expected}@{REGISTRY_SCHEMA_VERSION}"),
            actual: format!("{actual}@{version}"),
        })
    } else {
        Ok(())
    }
}

fn check_count(field: &'static str, actual: usize, max: usize) -> Result<(), RegistryProfileError> {
    if actual > max {
        Err(RegistryProfileError::CountTooLarge { field, actual, max })
    } else {
        Ok(())
    }
}

fn check_ordinal(
    field: &'static str,
    expected: usize,
    actual: u32,
) -> Result<(), RegistryProfileError> {
    if usize::try_from(actual).ok() == Some(expected) {
        Ok(())
    } else {
        Err(RegistryProfileError::Order {
            field,
            expected,
            actual,
        })
    }
}

fn unique<T: Ord + Copy + std::fmt::Debug>(
    field: &'static str,
    values: &mut BTreeSet<T>,
    value: T,
) -> Result<(), RegistryProfileError> {
    if values.insert(value) {
        Ok(())
    } else {
        invalid(field, "duplicate id or index")
    }
}

fn reference(
    field: &'static str,
    id: u32,
    expected: &'static str,
) -> Result<(), RegistryProfileError> {
    Err(RegistryProfileError::InvalidReference {
        field: field.to_owned(),
        id,
        expected: expected.to_owned(),
    })
}

fn invalid<T>(field: &'static str, reason: &'static str) -> Result<T, RegistryProfileError> {
    Err(RegistryProfileError::InvalidField {
        field: field.to_owned(),
        reason: reason.to_owned(),
    })
}

fn zero_digest() -> Sha256Digest {
    Sha256Digest::from_bytes([0; 32])
}

fn canonical_digest<T: Serialize>(
    domain: &[u8],
    value: &T,
) -> Result<Sha256Digest, RegistryProfileError> {
    let bytes = serde_json::to_vec(value)?;
    let mut digest = Sha256::new();
    digest.update(domain);
    digest.update((bytes.len() as u64).to_le_bytes());
    digest.update(bytes);
    Ok(Sha256Digest::from_bytes(digest.finalize().into()))
}

fn check_digest(
    field: &'static str,
    expected: Sha256Digest,
    actual: Sha256Digest,
) -> Result<(), RegistryProfileError> {
    if expected == actual {
        Ok(())
    } else {
        Err(RegistryProfileError::DigestMismatch {
            field,
            expected,
            actual,
        })
    }
}

fn parse_bounded<T>(
    bytes: &[u8],
    max: usize,
    label: &'static str,
) -> Result<T, RegistryProfileError>
where
    T: for<'de> Deserialize<'de> + RegistryValidate,
{
    if bytes.len() > max {
        return Err(RegistryProfileError::InputTooLarge {
            label,
            actual: bytes.len(),
            max,
        });
    }
    let value: T = serde_json::from_slice(bytes)?;
    value.registry_validate()?;
    Ok(value)
}

trait RegistryValidate {
    fn registry_validate(&self) -> Result<(), RegistryProfileError>;
}
impl RegistryValidate for OrderedEnginePropertiesV1 {
    fn registry_validate(&self) -> Result<(), RegistryProfileError> {
        self.validate()
    }
}
impl RegistryValidate for RegistrationTraceV1 {
    fn registry_validate(&self) -> Result<(), RegistryProfileError> {
        self.validate()
    }
}
impl RegistryValidate for PostBindSnapshotV1 {
    fn registry_validate(&self) -> Result<(), RegistryProfileError> {
        self.validate()
    }
}

fn check_blob(
    seal: &SealedBlobV1,
    bytes: &[u8],
    label: &'static str,
) -> Result<(), RegistryProfileError> {
    if seal.byte_len != bytes.len() as u64 {
        return Err(RegistryProfileError::BlobSealMismatch {
            label,
            reason: "byte length",
        });
    }
    let actual = Sha256Digest::from_bytes(Sha256::digest(bytes).into());
    if seal.sha256 != actual {
        return Err(RegistryProfileError::BlobSealMismatch {
            label,
            reason: "sha256",
        });
    }
    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum RegistryProfileError {
    #[error("{label} JSON is {actual} bytes; maximum accepted size is {max}")]
    InputTooLarge {
        label: &'static str,
        actual: usize,
        max: usize,
    },
    #[error("registry schema mismatch: expected {expected}, got {actual}")]
    Schema { expected: String, actual: String },
    #[error("{field} count {actual} exceeds maximum {max}")]
    CountTooLarge {
        field: &'static str,
        actual: usize,
        max: usize,
    },
    #[error("{field} is invalid: {reason}")]
    InvalidField { field: String, reason: String },
    #[error("{field} is out of order: expected {expected}, got {actual}")]
    Order {
        field: &'static str,
        expected: usize,
        actual: u32,
    },
    #[error("{field} id {id} does not reference {expected}")]
    InvalidReference {
        field: String,
        id: u32,
        expected: String,
    },
    #[error("{field} digest mismatch: expected {expected}, got {actual}")]
    DigestMismatch {
        field: &'static str,
        expected: Sha256Digest,
        actual: Sha256Digest,
    },
    #[error("sealed {label} payload has a mismatched {reason}")]
    BlobSealMismatch {
        label: &'static str,
        reason: &'static str,
    },
    #[error("invalid registry JSON: {0}")]
    Json(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(byte: u8) -> Sha256Digest {
        Sha256Digest::from_bytes([byte; 32])
    }

    fn context(namespace: &str) -> RegistrationContextV1 {
        RegistrationContextV1 {
            namespace: namespace.into(),
            config_group: Some("G1R".into()),
            access_mask: u32::MAX,
        }
    }

    fn properties() -> OrderedEnginePropertiesV1 {
        let mut value = OrderedEnginePropertiesV1 {
            schema: ENGINE_PROPERTIES_SCHEMA.into(),
            schema_version: 1,
            settings: vec![EnginePropertySettingV1 {
                ordinal: 0,
                property: EnginePropertyV1::OptimizeBytecode,
                value: 1,
            }],
            canonical_sha256: zero_digest(),
        };
        value.seal().unwrap();
        value
    }

    fn trace() -> RegistrationTraceV1 {
        let callable = |id| HostStubDescriptorV1 {
            stub_id: id,
            purpose: CompileOnlyStubPurposeV1::CompileOnlyNeverInvoke,
            descriptor: HostStubKindV1::Callable {
                signature_sha256: digest(id as u8 + 1),
            },
        };
        let mut value = RegistrationTraceV1 {
            schema: REGISTRATION_TRACE_SCHEMA.into(),
            schema_version: 1,
            host_stubs: vec![
                callable(0),
                callable(1),
                callable(2),
                HostStubDescriptorV1 {
                    stub_id: 3,
                    purpose: CompileOnlyStubPurposeV1::CompileOnlyNeverInvoke,
                    descriptor: HostStubKindV1::Storage {
                        byte_len: 8,
                        alignment: 8,
                    },
                },
                HostStubDescriptorV1 {
                    stub_id: 4,
                    purpose: CompileOnlyStubPurposeV1::CompileOnlyNeverInvoke,
                    descriptor: HostStubKindV1::Object {
                        interface_sha256: digest(9),
                    },
                },
            ],
            entries: vec![
                RegistrationEntryV1::ObjectType {
                    ordinal: 0,
                    registration_id: 0,
                    context: context(""),
                    type_id: 10,
                    declaration: "Base".into(),
                    byte_size: 8,
                    alignment: 8,
                    flags: ObjectTypeFlagsV1(2),
                },
                RegistrationEntryV1::ObjectType {
                    ordinal: 1,
                    registration_id: 1,
                    context: context("Game"),
                    type_id: 11,
                    declaration: "Derived".into(),
                    byte_size: 16,
                    alignment: 8,
                    flags: ObjectTypeFlagsV1(2),
                },
                RegistrationEntryV1::ObjectProperty {
                    ordinal: 2,
                    registration_id: 2,
                    context: context("Game"),
                    property_id: 20,
                    owner_type_id: 11,
                    declaration: "int value".into(),
                    byte_offset: 8,
                    composite_offset: 0,
                    is_composite_indirect: false,
                    accessor_type: 255,
                    is_protected: false,
                },
                RegistrationEntryV1::ObjectMethod {
                    ordinal: 3,
                    registration_id: 3,
                    context: context("Game"),
                    function_id: 30,
                    owner_type_id: 11,
                    declaration: "void Run()".into(),
                    call_convention: CallConventionV1::CdeclObjectLast,
                    callable_stub_id: 0,
                    auxiliary_object_stub_id: None,
                    composite_offset: 0,
                    is_composite_indirect: false,
                    accessor_type: 255,
                },
                RegistrationEntryV1::ObjectBehaviour {
                    ordinal: 4,
                    registration_id: 4,
                    context: context("Game"),
                    function_id: 31,
                    owner_type_id: 11,
                    behaviour: ObjectBehaviourV1::Construct,
                    declaration: "void f()".into(),
                    call_convention: CallConventionV1::CdeclObjectLast,
                    callable_stub_id: 1,
                    auxiliary_object_stub_id: None,
                    composite_offset: 0,
                    is_composite_indirect: false,
                },
                RegistrationEntryV1::GlobalProperty {
                    ordinal: 5,
                    registration_id: 5,
                    context: context(""),
                    property_id: 21,
                    declaration: "uint64 Tick".into(),
                    storage_stub_id: 3,
                },
                RegistrationEntryV1::GlobalFunction {
                    ordinal: 6,
                    registration_id: 6,
                    context: context(""),
                    function_id: 32,
                    declaration: "void Log()".into(),
                    call_convention: CallConventionV1::Cdecl,
                    callable_stub_id: 2,
                    auxiliary_object_stub_id: None,
                },
                RegistrationEntryV1::Enum {
                    ordinal: 7,
                    registration_id: 7,
                    context: context("Game"),
                    type_id: 12,
                    declaration: "EState".into(),
                },
                RegistrationEntryV1::EnumValue {
                    ordinal: 8,
                    registration_id: 8,
                    context: context("Game"),
                    owner_type_id: 12,
                    name: "Ready".into(),
                    value: 1,
                },
                RegistrationEntryV1::Funcdef {
                    ordinal: 9,
                    registration_id: 9,
                    context: context(""),
                    type_id: 13,
                    declaration: "void Callback(int)".into(),
                },
                RegistrationEntryV1::Typedef {
                    ordinal: 10,
                    registration_id: 10,
                    context: context(""),
                    type_id: 14,
                    name: "Count".into(),
                    target_declaration: "uint32".into(),
                },
                RegistrationEntryV1::StringFactory {
                    ordinal: 11,
                    registration_id: 11,
                    context: context(""),
                    string_type_declaration: "string".into(),
                    factory_object_stub_id: 4,
                },
                RegistrationEntryV1::DefaultArrayType {
                    ordinal: 12,
                    registration_id: 12,
                    context: context(""),
                    type_declaration: "array<T>".into(),
                },
                RegistrationEntryV1::Interface {
                    ordinal: 13,
                    registration_id: 13,
                    context: context("Game"),
                    type_id: 15,
                    declaration: "IRunnable".into(),
                },
                RegistrationEntryV1::InterfaceMethod {
                    ordinal: 14,
                    registration_id: 14,
                    context: context("Game"),
                    function_id: 33,
                    owner_type_id: 15,
                    declaration: "void Run()".into(),
                },
            ],
            canonical_sha256: zero_digest(),
        };
        value.seal().unwrap();
        value
    }

    fn snapshot(
        properties: &OrderedEnginePropertiesV1,
        trace: &RegistrationTraceV1,
    ) -> PostBindSnapshotV1 {
        let results = vec![
            PostBindResultV1::ObjectType {
                engine_type_id: 100,
            },
            PostBindResultV1::ObjectType {
                engine_type_id: 101,
            },
            PostBindResultV1::ObjectProperty {
                owner_engine_type_id: 101,
                property_index: 0,
            },
            PostBindResultV1::ObjectMethod {
                owner_engine_type_id: 101,
                engine_function_id: 200,
            },
            PostBindResultV1::ObjectBehaviour {
                owner_engine_type_id: 101,
                engine_function_id: 201,
            },
            PostBindResultV1::GlobalProperty {
                global_property_index: 0,
            },
            PostBindResultV1::GlobalFunction {
                engine_function_id: 202,
            },
            PostBindResultV1::Enum {
                engine_type_id: 102,
            },
            PostBindResultV1::EnumValue {
                owner_engine_type_id: 102,
                value_index: 0,
            },
            PostBindResultV1::Funcdef {
                engine_type_id: 103,
            },
            PostBindResultV1::Typedef {
                engine_type_id: 104,
            },
            PostBindResultV1::StringFactory { installed: true },
            PostBindResultV1::DefaultArrayType { installed: true },
            PostBindResultV1::Interface {
                engine_type_id: 105,
            },
            PostBindResultV1::InterfaceMethod {
                owner_engine_type_id: 105,
                engine_function_id: 203,
            },
        ];
        let mut value = PostBindSnapshotV1 {
            schema: POST_BIND_SNAPSHOT_SCHEMA.into(),
            schema_version: 1,
            engine_properties_sha256: properties.canonical_sha256,
            registration_trace_sha256: trace.canonical_sha256,
            entries: results
                .into_iter()
                .enumerate()
                .map(|(i, result)| PostBindEntryV1 {
                    ordinal: i as u32,
                    trace_registration_id: i as u32,
                    result,
                })
                .collect(),
            final_states: vec![
                PostBindStateV1::ObjectType {
                    type_id: 10,
                    byte_size: 8,
                    alignment: 8,
                    flags: 2,
                    base_type_id: None,
                    shadow_type_id: None,
                    interface_type_ids: vec![],
                    interface_vft_offsets: vec![],
                    has_implicit_constructors: false,
                    accepts_value_subtype: false,
                    accepts_reference_subtype: false,
                    is_invalid_generated_type: false,
                },
                PostBindStateV1::ObjectType {
                    type_id: 11,
                    byte_size: 16,
                    alignment: 8,
                    flags: 2,
                    base_type_id: Some(10),
                    shadow_type_id: None,
                    interface_type_ids: vec![],
                    interface_vft_offsets: vec![],
                    has_implicit_constructors: true,
                    accepts_value_subtype: false,
                    accepts_reference_subtype: false,
                    is_invalid_generated_type: false,
                },
                PostBindStateV1::ObjectProperty {
                    property_id: 20,
                    byte_offset: 8,
                    access_mask: u32::MAX,
                    composite_offset: 0,
                    is_composite_indirect: false,
                    is_private: false,
                    is_protected: false,
                    is_app_bind_property: true,
                    exposed_type: 255,
                },
                PostBindStateV1::Function {
                    function_id: 30,
                    trait_bits: 0x200,
                    exposed_type: 255,
                    hidden_argument_index: None,
                    hidden_argument_default: None,
                    determines_output_type_argument_index: None,
                    compile_out_mode: CompileOutModeV1::CompileCalls,
                    first_param_metadata: FirstParamMetadataV1::None,
                },
                PostBindStateV1::Function {
                    function_id: 31,
                    trait_bits: 0x1,
                    exposed_type: 255,
                    hidden_argument_index: None,
                    hidden_argument_default: None,
                    determines_output_type_argument_index: None,
                    compile_out_mode: CompileOutModeV1::CompileCalls,
                    first_param_metadata: FirstParamMetadataV1::None,
                },
                PostBindStateV1::Function {
                    function_id: 32,
                    trait_bits: 0x20_0000,
                    exposed_type: 255,
                    hidden_argument_index: Some(0),
                    hidden_argument_default: Some("__WorldContext".into()),
                    determines_output_type_argument_index: Some(1),
                    compile_out_mode: CompileOutModeV1::CompileOutEntirely,
                    first_param_metadata: FirstParamMetadataV1::ScriptFunction,
                },
                PostBindStateV1::GlobalProperty {
                    property_id: 21,
                    is_pure_constant: true,
                    pure_constant_value: Some(42),
                },
                PostBindStateV1::ObjectType {
                    type_id: 15,
                    byte_size: 0,
                    alignment: 1,
                    flags: 0x0040_0001,
                    base_type_id: None,
                    shadow_type_id: None,
                    interface_type_ids: vec![],
                    interface_vft_offsets: vec![],
                    has_implicit_constructors: false,
                    accepts_value_subtype: false,
                    accepts_reference_subtype: false,
                    is_invalid_generated_type: false,
                },
                PostBindStateV1::Function {
                    function_id: 33,
                    trait_bits: 0,
                    exposed_type: 255,
                    hidden_argument_index: None,
                    hidden_argument_default: None,
                    determines_output_type_argument_index: None,
                    compile_out_mode: CompileOutModeV1::CompileCalls,
                    first_param_metadata: FirstParamMetadataV1::None,
                },
            ],
            canonical_sha256: zero_digest(),
        };
        value.seal().unwrap();
        value
    }

    #[test]
    fn all_three_documents_round_trip_and_cross_validate() {
        let p = properties();
        let t = trace();
        let s = snapshot(&p, &t);
        assert_eq!(
            OrderedEnginePropertiesV1::from_json(&p.to_json().unwrap()).unwrap(),
            p
        );
        assert_eq!(
            RegistrationTraceV1::from_json(&t.to_json().unwrap()).unwrap(),
            t
        );
        assert_eq!(
            PostBindSnapshotV1::from_json(&s.to_json().unwrap()).unwrap(),
            s
        );
        s.validate_against(&p, &t).unwrap();
        assert_ne!(p.canonical_sha256, t.canonical_sha256);
        assert_ne!(t.canonical_sha256, s.canonical_sha256);
    }

    #[test]
    fn unknown_fields_versions_and_order_fail_closed() {
        let p = properties();
        let mut json = serde_json::to_value(&p).unwrap();
        json["unknown"] = serde_json::json!(1);
        assert!(matches!(
            OrderedEnginePropertiesV1::from_json(&serde_json::to_vec(&json).unwrap()),
            Err(RegistryProfileError::Json(_))
        ));
        let mut bad = p.clone();
        bad.schema_version = 2;
        assert!(matches!(
            bad.validate(),
            Err(RegistryProfileError::Schema { .. })
        ));
        let mut bad = trace();
        bad.entries[1] = match bad.entries[1].clone() {
            RegistrationEntryV1::ObjectType {
                registration_id,
                context,
                type_id,
                declaration,
                byte_size,
                alignment,
                flags,
                ..
            } => RegistrationEntryV1::ObjectType {
                ordinal: 9,
                registration_id,
                context,
                type_id,
                declaration,
                byte_size,
                alignment,
                flags,
            },
            _ => unreachable!(),
        };
        assert!(matches!(
            bad.seal(),
            Err(RegistryProfileError::Order { .. })
        ));
    }

    #[test]
    fn references_stub_kinds_and_snapshot_pairing_fail_closed() {
        let mut bad = trace();
        if let RegistrationEntryV1::GlobalProperty {
            storage_stub_id, ..
        } = &mut bad.entries[5]
        {
            *storage_stub_id = 0;
        }
        assert!(matches!(
            bad.seal(),
            Err(RegistryProfileError::InvalidField { .. })
        ));
        let p = properties();
        let t = trace();
        let mut s = snapshot(&p, &t);
        s.entries[3].result = PostBindResultV1::GlobalFunction {
            engine_function_id: 200,
        };
        s.seal().unwrap();
        assert!(matches!(
            s.validate_against(&p, &t),
            Err(RegistryProfileError::InvalidField { .. })
        ));

        let mut s = snapshot(&p, &t);
        s.final_states.retain(|state| {
            !matches!(
                state,
                PostBindStateV1::Function {
                    function_id: 32,
                    ..
                }
            )
        });
        s.seal().unwrap();
        assert!(matches!(
            s.validate_against(&p, &t),
            Err(RegistryProfileError::InvalidField { .. })
        ));
    }

    #[test]
    fn digest_tampering_and_live_address_shaped_fields_fail_closed() {
        let mut p = properties();
        p.settings[0].value = 0;
        assert!(matches!(
            p.validate(),
            Err(RegistryProfileError::DigestMismatch { .. })
        ));
        let t = trace();
        let mut json = serde_json::to_value(t).unwrap();
        json["host_stubs"][0]["address"] = serde_json::json!("0x7ff612341234");
        assert!(matches!(
            RegistrationTraceV1::from_json(&serde_json::to_vec(&json).unwrap()),
            Err(RegistryProfileError::Json(_))
        ));
    }

    #[test]
    fn manifest_seals_and_trace_count_bind_the_typed_payloads() {
        let p = properties();
        let t = trace();
        let s = snapshot(&p, &t);
        let p_json = p.to_json().unwrap();
        let t_json = t.to_json().unwrap();
        let s_json = s.to_json().unwrap();
        let blob = |path: &str, bytes: &[u8]| SealedBlobV1 {
            path: path.to_owned(),
            byte_len: bytes.len() as u64,
            sha256: Sha256Digest::from_bytes(Sha256::digest(bytes).into()),
        };
        let manifest = EngineProfileV1 {
            as_create_version: 1,
            ordered_engine_properties: blob("engine/properties.json", &p_json),
            registration_trace: blob("engine/registrations.json", &t_json),
            registration_trace_count: t.entries.len() as u64,
            post_bind_snapshot: blob("engine/post-bind.json", &s_json),
        };

        let validated =
            validate_engine_profile_payloads(&manifest, &p_json, &t_json, &s_json).unwrap();
        assert_eq!(validated.registration_trace, t);

        let mut bad_count = manifest.clone();
        bad_count.registration_trace_count += 1;
        assert!(matches!(
            validate_engine_profile_payloads(&bad_count, &p_json, &t_json, &s_json),
            Err(RegistryProfileError::InvalidField { .. })
        ));

        let mut bad_seal = manifest;
        bad_seal.post_bind_snapshot.byte_len += 1;
        assert!(matches!(
            validate_engine_profile_payloads(&bad_seal, &p_json, &t_json, &s_json),
            Err(RegistryProfileError::BlobSealMismatch { .. })
        ));
    }
}
