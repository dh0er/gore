//! Closed, cache-only NPC archetype extraction for offline authoring.
//!
//! Records are joined only through assignments recovered from the shipped `__InitDefaults`
//! bytecode. Class-name prefixes and suffixes are never used as relationship evidence. The
//! result describes static cache structure; it does not qualify spawning, saves, dialogue, or
//! any other runtime behavior.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use sha2::{Digest, Sha256};

use super::binds::NativeApi;
use super::emit_all::{prepare_resolver_semantics, PreparedEmit};
use super::isa::{BcType, OpInfo, OPCODES};
use super::model::{self, Class, Func, Module};
use super::refs::RefResolver;
use super::tables::parse_tail_tables;
use super::walk_modules::module_names;
use super::wire::{Cursor, SiaBytes, WireError};

pub const MAX_NPC_CACHE_BYTES: usize = 256 * 1024 * 1024;
pub const MAX_NPC_BINDS_BYTES: usize = 32 * 1024 * 1024;
pub const MAX_NPC_MODULES: usize = 10_000;
pub const MAX_NPC_CLASSES: usize = 100_000;
pub const MAX_NPC_FUNCTIONS: usize = 500_000;
pub const MAX_NPC_FIELDS: usize = 1_000_000;
pub const MAX_NPC_ARRAY_ELEMENTS: usize = 40_000_000;
pub const MAX_NPC_FUNCTION_BYTECODE_DWORDS: usize = 1_048_576;
pub const MAX_NPC_TOTAL_BYTECODE_DWORDS: usize = 24 * 1024 * 1024;
pub const MAX_NPC_FUNCTION_INSTRUCTIONS: usize = 1_048_576;
pub const MAX_NPC_DEFAULT_EDGES_PER_FUNCTION: usize = 64;
pub const MAX_NPC_ARCHETYPES: usize = 4096;
pub const MAX_NPC_STRING_BYTES: usize = 1024;
pub const MAX_NPC_SERIALIZED_STRING_BYTES: usize = 64 * 1024;
pub const MAX_NPC_TOTAL_STRING_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_NPC_SOURCE_MODULE_INPUT_BYTES: usize = 32 * 1024 * 1024;
pub const MAX_NPC_SOURCE_MODULE_BYTECODE_DWORDS: usize = 4 * 1024 * 1024;
pub const MAX_NPC_SOURCE_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_NPC_TOTAL_SOURCE_BYTES: usize = 256 * 1024 * 1024;

const SPAWN_ROOT: &str = "USpawnAIAgentDefinition";
const HUMAN_AI_ROOT: &str = "UAIAgentConfig_Human";
const HUMAN_CHARACTER_ROOT: &str = "UCharacterDefinition_Human";
const INIT_DEFAULTS: &str = "__InitDefaults";
const SPAWN_AI_FIELD: &str = "AIAgentConfigClass";
const SPAWN_BLUEPRINT_FIELD: &str = "AIAgentCharacterClass";
const AI_CHARACTER_FIELD: &str = "m_CharacterDefinition";
const ASSIGN_FUNCTION: &str = "opAssign";
const STATIC_NAME_FUNCTION: &str = "__STATIC_NAME";
const AS_OBJ_NOINHERIT: u32 = 0x0080_0000;

const HUMAN_BASE_BLUEPRINT: &str =
    "Blueprint'/Game/AI/AIAgent/Human/AIAgentCharacter_Human_Base.AIAgentCharacter_Human_Base_C'";
const HUMAN_WOMAN_BLUEPRINT: &str =
    "Blueprint'/Game/AI/AIAgent/Human/AIAgentCharacter_Human_Woman.AIAgentCharacter_Human_Woman_C'";
const HUMAN_XARDAS_SLEEPER_BLUEPRINT: &str =
    "Blueprint'/Game/AI/AIAgent/Human/AIAgentCharacter_Human_XardasSleeper.AIAgentCharacter_Human_XardasSleeper_C'";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NpcArchetypeSeal {
    pub byte_len: u64,
    pub sha256: [u8; 32],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NpcBlueprintFamily {
    /// The sealed male Human family, including the Xardas-Sleeper specialization.
    HumanBase,
    /// The sealed woman Human family.
    HumanWoman,
    /// A structurally valid Human archetype with an unclassified actor Blueprint.
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NpcArchetypeClassEvidence {
    pub class_name: String,
    pub super_class: Option<String>,
    pub module_name: String,
    pub relative_path: String,
    pub source_seal: NpcArchetypeSeal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NpcArchetypeDefaultEdgeEvidence {
    pub owner_class: String,
    pub field_name: String,
    pub assigned_value: String,
    pub instruction_offset_dwords: usize,
    pub init_defaults_bytecode_seal: NpcArchetypeSeal,
    pub evidence_sha256: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NpcArchetypeRecord {
    pub spawn: NpcArchetypeClassEvidence,
    pub ai_config: NpcArchetypeClassEvidence,
    pub character_definition: NpcArchetypeClassEvidence,
    pub actor_blueprint: String,
    pub blueprint_family: NpcBlueprintFamily,
    pub spawn_ai_edge: NpcArchetypeDefaultEdgeEvidence,
    pub spawn_blueprint_edge: NpcArchetypeDefaultEdgeEvidence,
    pub ai_character_edge: NpcArchetypeDefaultEdgeEvidence,
    pub evidence_sha256: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NpcArchetypeRejection {
    pub spawn_class: String,
    pub reason: NpcArchetypeRejectionReason,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NpcArchetypeRejectionReason {
    MissingInitDefaults {
        owner_class: String,
    },
    AmbiguousInitDefaults {
        owner_class: String,
        count: usize,
    },
    InvalidInitDefaultsBytecode {
        owner_class: String,
        detail: String,
    },
    MissingDefaultEdge {
        owner_class: String,
        field_name: String,
    },
    AmbiguousDefaultEdge {
        owner_class: String,
        field_name: String,
        count: usize,
    },
    MissingReferencedClass {
        role: &'static str,
        class_name: String,
    },
    WrongAncestry {
        role: &'static str,
        class_name: String,
        required_base: &'static str,
    },
    InheritanceCycle {
        role: &'static str,
        class_name: String,
    },
    NonInheritableClass {
        role: &'static str,
        class_name: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NpcArchetypeCollection {
    pub shipping_cache_seal: NpcArchetypeSeal,
    pub binds_cache_seal: NpcArchetypeSeal,
    pub records: Vec<NpcArchetypeRecord>,
    pub rejections: Vec<NpcArchetypeRejection>,
    pub records_sha256: [u8; 32],
}

#[derive(Debug, thiserror::Error)]
pub enum NpcArchetypeError {
    #[error("Shipping cache is {actual} bytes; maximum is {max}")]
    ShippingCacheTooLarge { actual: usize, max: usize },
    #[error("Binds cache is {actual} bytes; maximum is {max}")]
    BindsCacheTooLarge { actual: usize, max: usize },
    #[error("Shipping cache does not match its trusted generation seal")]
    ShippingSealMismatch,
    #[error("Binds cache does not match its trusted generation seal")]
    BindsSealMismatch,
    #[error("invalid Shipping cache: {0}")]
    InvalidShippingCache(String),
    #[error("invalid or incomplete Binds cache")]
    InvalidBindsCache,
    #[error("Shipping module key {key:?} disagrees with embedded module name {embedded:?}")]
    ModuleIdentityMismatch { key: String, embedded: String },
    #[error("invalid Shipping module layout: {0}")]
    InvalidModuleLayout(String),
    #[error("NPC extraction parsed {actual} classes; maximum is {max}")]
    TooManyClasses { actual: usize, max: usize },
    #[error("NPC structural preflight {kind} count is {actual}; maximum is {max}")]
    StructuralLimitExceeded {
        kind: &'static str,
        actual: usize,
        max: usize,
    },
    #[error("duplicate class identity {class_name:?}")]
    DuplicateClass { class_name: String },
    #[error("NPC extraction produced more than {max} accepted records")]
    TooManyRecords { max: usize },
    #[error("NPC {kind} string is {actual} bytes; maximum is {max}")]
    StringTooLong {
        kind: &'static str,
        actual: usize,
        max: usize,
    },
    #[error("NPC extraction exceeds {max} aggregate retained string bytes")]
    TooManyStringBytes { max: usize },
    #[error("NPC source evidence is incomplete")]
    MissingSourceEvidence,
    #[error("NPC source module {module:?} {kind} is {actual}; maximum is {max}")]
    SourceGenerationLimitExceeded {
        module: String,
        kind: &'static str,
        actual: usize,
        max: usize,
    },
    #[error("NPC emitted sources exceed {max} aggregate bytes")]
    TooManySourceBytes { max: usize },
}

/// Extract deterministic Spawn -> Human AI -> Human Character records from exact cache bytes.
///
/// The function performs no filesystem access, compilation, deployment, game launch, or runtime
/// qualification. Both byte buffers are bounded and must match caller-supplied trusted SHA-256
/// seals before parsing. Malformed individual Spawn candidates become typed rejections; malformed
/// container identity, duplicate class identity, and exhausted global bounds fail the collection.
pub fn collect_npc_archetypes(
    shipping_cache: &[u8],
    binds_cache: &[u8],
    expected_shipping_sha256: [u8; 32],
    expected_binds_sha256: [u8; 32],
) -> Result<NpcArchetypeCollection, NpcArchetypeError> {
    collect_npc_archetypes_with_limits(
        shipping_cache,
        binds_cache,
        expected_shipping_sha256,
        expected_binds_sha256,
        NpcLimits::default(),
    )
}

fn collect_npc_archetypes_with_limits(
    shipping_cache: &[u8],
    binds_cache: &[u8],
    expected_shipping_sha256: [u8; 32],
    expected_binds_sha256: [u8; 32],
    limits: NpcLimits,
) -> Result<NpcArchetypeCollection, NpcArchetypeError> {
    if shipping_cache.len() > limits.shipping_bytes {
        return Err(NpcArchetypeError::ShippingCacheTooLarge {
            actual: shipping_cache.len(),
            max: limits.shipping_bytes,
        });
    }
    if binds_cache.len() > limits.binds_bytes {
        return Err(NpcArchetypeError::BindsCacheTooLarge {
            actual: binds_cache.len(),
            max: limits.binds_bytes,
        });
    }

    let shipping_sha256: [u8; 32] = Sha256::digest(shipping_cache).into();
    if shipping_sha256 != expected_shipping_sha256 {
        return Err(NpcArchetypeError::ShippingSealMismatch);
    }
    let binds_sha256: [u8; 32] = Sha256::digest(binds_cache).into();
    if binds_sha256 != expected_binds_sha256 {
        return Err(NpcArchetypeError::BindsSealMismatch);
    }

    // Allocation-free structural accounting runs before every parser that materializes strings,
    // classes, functions, bytecode vectors, reference tables, or emitted source.
    let preflight = preflight_cache(shipping_cache, limits)?;
    let tail = preflight.module_end;
    let tables = parse_tail_tables(shipping_cache, tail)
        .map_err(|error| NpcArchetypeError::InvalidShippingCache(error.to_string()))?;
    if tables.end != shipping_cache.len() {
        return Err(NpcArchetypeError::InvalidShippingCache(format!(
            "tail tables end at {}, input has {} bytes",
            tables.end,
            shipping_cache.len()
        )));
    }

    let keys = module_names(shipping_cache)
        .map_err(|error| NpcArchetypeError::InvalidShippingCache(error.to_string()))?;
    let modules = model::parse_modules(shipping_cache)
        .map_err(|error| NpcArchetypeError::InvalidShippingCache(error.to_string()))?;
    validate_module_keys(&keys, &modules)?;

    let native = NativeApi::from_bytes(binds_cache).ok_or(NpcArchetypeError::InvalidBindsCache)?;
    let mut refs = RefResolver::build(shipping_cache)
        .map_err(|error| NpcArchetypeError::InvalidShippingCache(error.to_string()))?;
    prepare_resolver_semantics(&modules, &mut refs, Some(native));

    let discovery = discover(&modules, &refs, limits)?;
    let used_modules = discovery.used_module_indices();
    let prepared = PreparedEmit::new(&modules, &mut refs, None)
        .map_err(|error| NpcArchetypeError::InvalidModuleLayout(error.to_string()))?;
    let relative_paths = prepared
        .collision_relative_paths()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if relative_paths.len() != modules.len() {
        return Err(NpcArchetypeError::MissingSourceEvidence);
    }
    let mut sources = BTreeMap::new();
    let mut total_source_bytes = 0usize;
    for module_index in used_modules {
        let module_stats = preflight
            .modules
            .get(module_index)
            .ok_or(NpcArchetypeError::MissingSourceEvidence)?;
        let module_name = &modules[module_index].name;
        validate_source_generation_input(module_name, *module_stats, limits)?;
        let source = prepared
            .emit_module(module_index)
            .map_err(|error| NpcArchetypeError::InvalidModuleLayout(error.to_string()))?;
        charge_source_output(module_name, source.len(), &mut total_source_bytes, limits)?;
        sources.insert(
            module_index,
            ModuleSourceEvidence {
                relative_path: relative_paths[module_index].clone(),
                source_seal: seal_bytes(source.as_bytes()),
            },
        );
    }

    materialize(
        &modules,
        &sources,
        discovery,
        seal_bytes(shipping_cache),
        seal_bytes(binds_cache),
        limits,
    )
}

fn validate_source_generation_input(
    module_name: &str,
    module_stats: ModulePreflight,
    limits: NpcLimits,
) -> Result<(), NpcArchetypeError> {
    if module_stats.serialized_bytes > limits.source_module_input_bytes {
        return Err(NpcArchetypeError::SourceGenerationLimitExceeded {
            module: module_name.to_owned(),
            kind: "serialized input bytes",
            actual: module_stats.serialized_bytes,
            max: limits.source_module_input_bytes,
        });
    }
    if module_stats.bytecode_dwords > limits.source_module_bytecode_dwords {
        return Err(NpcArchetypeError::SourceGenerationLimitExceeded {
            module: module_name.to_owned(),
            kind: "bytecode dwords",
            actual: module_stats.bytecode_dwords,
            max: limits.source_module_bytecode_dwords,
        });
    }
    Ok(())
}

fn charge_source_output(
    module_name: &str,
    source_bytes: usize,
    total_source_bytes: &mut usize,
    limits: NpcLimits,
) -> Result<(), NpcArchetypeError> {
    if source_bytes > limits.source_bytes {
        return Err(NpcArchetypeError::SourceGenerationLimitExceeded {
            module: module_name.to_owned(),
            kind: "emitted source bytes",
            actual: source_bytes,
            max: limits.source_bytes,
        });
    }
    let next = total_source_bytes.saturating_add(source_bytes);
    if next > limits.total_source_bytes {
        return Err(NpcArchetypeError::TooManySourceBytes {
            max: limits.total_source_bytes,
        });
    }
    *total_source_bytes = next;
    Ok(())
}

fn validate_module_keys(keys: &[String], modules: &[Module]) -> Result<(), NpcArchetypeError> {
    if keys.len() != modules.len() {
        return Err(NpcArchetypeError::InvalidShippingCache(format!(
            "module key/model count mismatch: {} != {}",
            keys.len(),
            modules.len()
        )));
    }
    for (key, module) in keys.iter().zip(modules) {
        if key != &module.name {
            return Err(NpcArchetypeError::ModuleIdentityMismatch {
                key: key.clone(),
                embedded: module.name.clone(),
            });
        }
    }
    Ok(())
}

const PREFLIGHT_DATA_TYPE_BYTES: usize = 36;

#[derive(Debug, Clone, Copy)]
struct ModulePreflight {
    serialized_bytes: usize,
    bytecode_dwords: usize,
}

struct CachePreflight {
    module_end: usize,
    modules: Vec<ModulePreflight>,
}

struct PreflightCursor<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> PreflightCursor<'a> {
    fn at(bytes: &'a [u8], position: usize) -> Self {
        Self { bytes, position }
    }

    fn remaining(&self) -> usize {
        self.bytes.len().saturating_sub(self.position)
    }

    fn skip(&mut self, bytes: usize) -> Result<(), NpcArchetypeError> {
        let end = self.position.checked_add(bytes).ok_or_else(|| {
            invalid_wire(WireError::BadLen {
                pos: self.position,
                len: bytes as i64,
                field: "preflight skip",
            })
        })?;
        if end > self.bytes.len() {
            return Err(invalid_wire(WireError::Eof {
                pos: self.position,
                need: bytes,
                have: self.remaining(),
            }));
        }
        self.position = end;
        Ok(())
    }

    fn read_i32(&mut self) -> Result<i32, NpcArchetypeError> {
        if self.remaining() < 4 {
            return Err(invalid_wire(WireError::Eof {
                pos: self.position,
                need: 4,
                have: self.remaining(),
            }));
        }
        let value = i32::from_le_bytes(
            self.bytes[self.position..self.position + 4]
                .try_into()
                .unwrap(),
        );
        self.position += 4;
        Ok(value)
    }

    fn read_bool(&mut self) -> Result<bool, NpcArchetypeError> {
        Ok(self.read_i32()? != 0)
    }

    fn read_canonical_bool(&mut self) -> Result<bool, NpcArchetypeError> {
        let position = self.position;
        match self.read_i32()? {
            0 => Ok(false),
            1 => Ok(true),
            value => Err(invalid_wire(WireError::BadLen {
                pos: position,
                len: i64::from(value),
                field: "bool",
            })),
        }
    }
}

struct PreflightBudget {
    limits: NpcLimits,
    functions: usize,
    classes: usize,
    fields: usize,
    array_elements: usize,
    bytecode_dwords: usize,
    string_payload_bytes: usize,
}

impl PreflightBudget {
    fn new(limits: NpcLimits) -> Self {
        Self {
            limits,
            functions: 0,
            classes: 0,
            fields: 0,
            array_elements: 0,
            bytecode_dwords: 0,
            string_payload_bytes: 0,
        }
    }

    fn add(
        value: &mut usize,
        amount: usize,
        max: usize,
        kind: &'static str,
    ) -> Result<(), NpcArchetypeError> {
        let next = value.saturating_add(amount);
        if next > max {
            return Err(NpcArchetypeError::StructuralLimitExceeded {
                kind,
                actual: next,
                max,
            });
        }
        *value = next;
        Ok(())
    }

    fn count(
        &mut self,
        cursor: &mut PreflightCursor<'_>,
        field: &'static str,
    ) -> Result<usize, NpcArchetypeError> {
        let raw = cursor.read_i32()?;
        let count = usize::try_from(raw).map_err(|_| {
            invalid_wire(WireError::BadLen {
                pos: cursor.position.saturating_sub(4),
                len: raw as i64,
                field,
            })
        })?;
        Self::add(
            &mut self.array_elements,
            count,
            self.limits.array_elements,
            "array elements",
        )?;
        Ok(count)
    }

    fn function(&mut self) -> Result<(), NpcArchetypeError> {
        Self::add(&mut self.functions, 1, self.limits.functions, "functions")
    }

    fn class(&mut self) -> Result<(), NpcArchetypeError> {
        Self::add(&mut self.classes, 1, self.limits.classes, "classes")
    }

    fn field(&mut self) -> Result<(), NpcArchetypeError> {
        Self::add(&mut self.fields, 1, self.limits.fields, "fields")
    }

    fn string(
        &mut self,
        cursor: &mut PreflightCursor<'_>,
        fstring: bool,
    ) -> Result<(), NpcArchetypeError> {
        let position = cursor.position;
        let raw = cursor.read_i32()?;
        let (payload_bytes, content_bytes) = match raw.cmp(&0) {
            std::cmp::Ordering::Equal => (0, 0),
            std::cmp::Ordering::Greater if fstring => {
                let payload = raw as usize;
                (payload, payload.saturating_sub(1))
            }
            std::cmp::Ordering::Greater => {
                let content = raw as usize;
                (content.saturating_add(1), content)
            }
            std::cmp::Ordering::Less if fstring => {
                let units = (-(raw as i64)) as usize;
                (
                    units.saturating_mul(2),
                    units.saturating_sub(1).saturating_mul(2),
                )
            }
            std::cmp::Ordering::Less => {
                let units = (-(raw as i64)) as usize;
                (
                    units.saturating_mul(2).saturating_add(2),
                    units.saturating_mul(2),
                )
            }
        };
        self.charge_serialized_string(content_bytes)?;
        cursor.skip(payload_bytes).map_err(|error| match error {
            NpcArchetypeError::InvalidShippingCache(_) => invalid_wire(WireError::Eof {
                pos: position,
                need: payload_bytes.saturating_add(4),
                have: cursor.remaining().saturating_add(4),
            }),
            other => other,
        })
    }

    /// Parse one allocation-free SIA payload while retaining its raw bytes for a later contextual
    /// encoding decision. GlobalReference.bIsString follows Name/Module/Namespace on the wire.
    fn sia_bytes<'a>(
        &mut self,
        cursor: &mut PreflightCursor<'a>,
    ) -> Result<(SiaBytes<'a>, usize), NpcArchetypeError> {
        let position = cursor.position;
        let mut wire = Cursor::at(cursor.bytes, position);
        let encoded = wire.read_sia_bytes().map_err(invalid_wire)?;
        self.charge_serialized_string(encoded.len())?;
        cursor.position = wire.pos();
        Ok((encoded, position))
    }

    fn charge_serialized_string(&mut self, content_bytes: usize) -> Result<(), NpcArchetypeError> {
        if content_bytes > self.limits.serialized_string_bytes {
            return Err(NpcArchetypeError::StringTooLong {
                kind: "serialized cache string",
                actual: content_bytes,
                max: self.limits.serialized_string_bytes,
            });
        }
        let next = self.string_payload_bytes.saturating_add(content_bytes);
        if next > self.limits.shipping_bytes {
            return Err(NpcArchetypeError::StructuralLimitExceeded {
                kind: "serialized string bytes",
                actual: next,
                max: self.limits.shipping_bytes,
            });
        }
        self.string_payload_bytes = next;
        Ok(())
    }

    fn fixed_array(
        &mut self,
        cursor: &mut PreflightCursor<'_>,
        element_bytes: usize,
        field: &'static str,
    ) -> Result<usize, NpcArchetypeError> {
        let count = self.count(cursor, field)?;
        let bytes = count.checked_mul(element_bytes).ok_or_else(|| {
            invalid_wire(WireError::BadLen {
                pos: cursor.position.saturating_sub(4),
                len: count as i64,
                field,
            })
        })?;
        cursor.skip(bytes)?;
        Ok(count)
    }

    fn string_array(
        &mut self,
        cursor: &mut PreflightCursor<'_>,
        field: &'static str,
    ) -> Result<(), NpcArchetypeError> {
        let count = self.count(cursor, field)?;
        for _ in 0..count {
            self.string(cursor, false)?;
        }
        Ok(())
    }
}

fn invalid_wire(error: WireError) -> NpcArchetypeError {
    NpcArchetypeError::InvalidShippingCache(error.to_string())
}

fn preflight_cache(bytes: &[u8], limits: NpcLimits) -> Result<CachePreflight, NpcArchetypeError> {
    if bytes.len() < 24 {
        return Err(invalid_wire(WireError::Eof {
            pos: 0,
            need: 24,
            have: bytes.len(),
        }));
    }
    let raw_modules = u32::from_le_bytes(bytes[20..24].try_into().unwrap()) as usize;
    if raw_modules > limits.modules {
        return Err(NpcArchetypeError::StructuralLimitExceeded {
            kind: "modules",
            actual: raw_modules,
            max: limits.modules,
        });
    }
    let mut cursor = PreflightCursor::at(bytes, 24);
    let mut budget = PreflightBudget::new(limits);
    let mut modules = Vec::with_capacity(raw_modules);
    for _ in 0..raw_modules {
        let start = cursor.position;
        budget.string(&mut cursor, true)?;
        let mut module_bytecode = 0usize;
        preflight_module(&mut cursor, &mut budget, &mut module_bytecode)?;
        modules.push(ModulePreflight {
            serialized_bytes: cursor.position - start,
            bytecode_dwords: module_bytecode,
        });
    }
    let module_end = cursor.position;
    preflight_tail(&mut cursor, &mut budget)?;
    if cursor.position != bytes.len() {
        return Err(NpcArchetypeError::InvalidShippingCache(format!(
            "preflight ended at {}, input has {} bytes",
            cursor.position,
            bytes.len()
        )));
    }
    Ok(CachePreflight {
        module_end,
        modules,
    })
}

fn preflight_function(
    cursor: &mut PreflightCursor<'_>,
    budget: &mut PreflightBudget,
    module_bytecode: &mut usize,
) -> Result<(), NpcArchetypeError> {
    budget.function()?;
    budget.string(cursor, false)?;
    budget.string(cursor, false)?;
    cursor.skip(PREFLIGHT_DATA_TYPE_BYTES)?;
    budget.fixed_array(cursor, PREFLIGHT_DATA_TYPE_BYTES, "ParameterTypes")?;
    budget.string_array(cursor, "ParameterNames")?;
    budget.fixed_array(cursor, 4, "ParameterFlags")?;
    budget.string_array(cursor, "ParameterDefaultArgs")?;
    cursor.skip(4)?;
    let dwords = budget.count(cursor, "ByteCode")?;
    if dwords > budget.limits.function_bytecode_dwords {
        return Err(NpcArchetypeError::StructuralLimitExceeded {
            kind: "function bytecode dwords",
            actual: dwords,
            max: budget.limits.function_bytecode_dwords,
        });
    }
    PreflightBudget::add(
        &mut budget.bytecode_dwords,
        dwords,
        budget.limits.total_bytecode_dwords,
        "total bytecode dwords",
    )?;
    *module_bytecode = module_bytecode.saturating_add(dwords);
    cursor.skip(dwords.checked_mul(4).ok_or_else(|| {
        invalid_wire(WireError::BadLen {
            pos: cursor.position.saturating_sub(4),
            len: dwords as i64,
            field: "ByteCode",
        })
    })?)?;
    budget.fixed_array(cursor, 4, "ByteCodeReferences")?;
    cursor.skip(4)?;
    budget.fixed_array(cursor, 8, "ObjVariableTypes")?;
    budget.fixed_array(cursor, 4, "ObjVariablePos")?;
    cursor.skip(4)?;
    budget.fixed_array(cursor, 4, "VarInfoProgramPos")?;
    budget.fixed_array(cursor, 4, "VarInfoOffset")?;
    budget.fixed_array(cursor, 4, "VarInfoOption")?;
    cursor.skip(12)?;
    budget.fixed_array(cursor, 4, "LineNumbers")?;
    if cursor.read_bool()? {
        budget.string(cursor, false)?;
        budget.string_array(cursor, "UF.MetaSpec")?;
        budget.string_array(cursor, "UF.MetaValues")?;
        cursor.skip(18 * 4)?;
    }
    Ok(())
}

fn preflight_property(
    cursor: &mut PreflightCursor<'_>,
    budget: &mut PreflightBudget,
) -> Result<(), NpcArchetypeError> {
    budget.field()?;
    budget.string(cursor, false)?;
    cursor.skip(PREFLIGHT_DATA_TYPE_BYTES + 8)?;
    if cursor.read_bool()? {
        budget.string_array(cursor, "UP.MetaSpec")?;
        budget.string_array(cursor, "UP.MetaValues")?;
        cursor.skip(9 * 4)?;
        let replicated = cursor.read_bool()?;
        cursor.skip(4 * 3)?;
        if replicated {
            cursor.skip(8)?;
        }
        cursor.skip(4 * 3)?;
    }
    Ok(())
}

fn preflight_class(
    cursor: &mut PreflightCursor<'_>,
    budget: &mut PreflightBudget,
    module_bytecode: &mut usize,
) -> Result<(), NpcArchetypeError> {
    budget.class()?;
    budget.string(cursor, false)?;
    budget.string(cursor, false)?;
    cursor.skip(4)?;
    let fields = budget.count(cursor, "Class.Properties")?;
    for _ in 0..fields {
        preflight_property(cursor, budget)?;
    }
    let methods = budget.count(cursor, "Class.Methods")?;
    for _ in 0..methods {
        preflight_function(cursor, budget, module_bytecode)?;
    }
    budget.fixed_array(cursor, 4, "Class.MethodTable")?;
    cursor.skip(16)?;
    let constructors = budget.count(cursor, "Class.Constructors")?;
    for _ in 0..constructors {
        preflight_function(cursor, budget, module_bytecode)?;
    }
    budget.fixed_array(cursor, 8, "Class.FactoryRefs")?;
    budget.fixed_array(cursor, 8, "Class.BehaviorRefs")?;
    let behaviors = budget.count(cursor, "Class.BehaviorFunctions")?;
    for _ in 0..behaviors {
        preflight_function(cursor, budget, module_bytecode)?;
    }
    budget.fixed_array(cursor, 4, "Class.BehaviorFunctionTypes")?;
    if cursor.read_bool()? {
        budget.string(cursor, false)?;
        budget.string(cursor, false)?;
        for _ in 0..7 {
            cursor.read_bool()?;
        }
        budget.string(cursor, false)?; // ConfigName
        budget.string(cursor, false)?;
        cursor.read_bool()?;
        budget.string_array(cursor, "Class.MetaSpec")?;
        budget.string_array(cursor, "Class.MetaValues")?;
        budget.string(cursor, false)?;
    }
    Ok(())
}

fn preflight_enum(
    cursor: &mut PreflightCursor<'_>,
    budget: &mut PreflightBudget,
) -> Result<(), NpcArchetypeError> {
    budget.string(cursor, false)?;
    budget.string(cursor, false)?;
    budget.string_array(cursor, "Enum.Names")?;
    budget.fixed_array(cursor, 4, "Enum.Values")?;
    Ok(())
}

fn preflight_global(
    cursor: &mut PreflightCursor<'_>,
    budget: &mut PreflightBudget,
    module_bytecode: &mut usize,
) -> Result<(), NpcArchetypeError> {
    budget.string(cursor, false)?;
    budget.string(cursor, false)?;
    cursor.skip(PREFLIGHT_DATA_TYPE_BYTES)?;
    if !cursor.read_bool()? {
        if cursor.read_bool()? {
            cursor.skip(8)?;
        } else {
            cursor.read_bool()?; // bHasInitFunction
            preflight_function(cursor, budget, module_bytecode)?;
        }
    }
    Ok(())
}

fn preflight_import(
    cursor: &mut PreflightCursor<'_>,
    budget: &mut PreflightBudget,
) -> Result<(), NpcArchetypeError> {
    budget.string(cursor, false)?;
    budget.string(cursor, false)?;
    budget.string(cursor, false)?;
    budget.fixed_array(cursor, PREFLIGHT_DATA_TYPE_BYTES, "Import.ParameterTypes")?;
    budget.fixed_array(cursor, 4, "Import.ParameterFlags")?;
    budget.string_array(cursor, "Import.ParameterDefaultArgs")?;
    cursor.skip(PREFLIGHT_DATA_TYPE_BYTES)?;
    Ok(())
}

fn preflight_module(
    cursor: &mut PreflightCursor<'_>,
    budget: &mut PreflightBudget,
    module_bytecode: &mut usize,
) -> Result<(), NpcArchetypeError> {
    budget.string(cursor, false)?;
    let functions = budget.count(cursor, "Module.Functions")?;
    for _ in 0..functions {
        preflight_function(cursor, budget, module_bytecode)?;
    }
    let classes = budget.count(cursor, "Module.Classes")?;
    for _ in 0..classes {
        preflight_class(cursor, budget, module_bytecode)?;
    }
    let enums = budget.count(cursor, "Module.Enums")?;
    for _ in 0..enums {
        preflight_enum(cursor, budget)?;
    }
    let globals = budget.count(cursor, "Module.GlobalVariables")?;
    for _ in 0..globals {
        preflight_global(cursor, budget, module_bytecode)?;
    }
    let imports = budget.count(cursor, "Module.FunctionImports")?;
    for _ in 0..imports {
        preflight_import(cursor, budget)?;
    }
    cursor.skip(8)?;
    budget.string_array(cursor, "Module.ImportedModules")?;
    budget.string(cursor, false)?;
    budget.string_array(cursor, "Module.DeclaredEvents")?;
    budget.string_array(cursor, "Module.DeclaredDelegates")?;
    budget.string(cursor, false)?;
    budget.string_array(cursor, "Module.PostInitFunctions")?;
    Ok(())
}

fn preflight_tail_map(
    cursor: &mut PreflightCursor<'_>,
    budget: &mut PreflightBudget,
    field: &'static str,
    mut entry: impl FnMut(
        &mut PreflightCursor<'_>,
        &mut PreflightBudget,
    ) -> Result<(), NpcArchetypeError>,
) -> Result<(), NpcArchetypeError> {
    let count = budget.count(cursor, field)?;
    for _ in 0..count {
        entry(cursor, budget)?;
    }
    Ok(())
}

fn preflight_tail(
    cursor: &mut PreflightCursor<'_>,
    budget: &mut PreflightBudget,
) -> Result<(), NpcArchetypeError> {
    preflight_tail_map(cursor, budget, "TypeReferences", |cursor, budget| {
        cursor.skip(8)?;
        budget.string(cursor, false)?;
        budget.string(cursor, false)?;
        budget.string(cursor, false)?;
        budget.fixed_array(cursor, PREFLIGHT_DATA_TYPE_BYTES, "TypeRef.SubTypes")?;
        Ok(())
    })?;
    budget.fixed_array(cursor, 12, "TypeIdReferenceToPointer")?;
    preflight_tail_map(cursor, budget, "FunctionReferences", |cursor, budget| {
        cursor.skip(8)?;
        budget.string(cursor, false)?;
        budget.string(cursor, false)?;
        budget.string(cursor, false)?;
        cursor.skip(20)?;
        budget.fixed_array(cursor, PREFLIGHT_DATA_TYPE_BYTES, "FuncRef.ParameterTypes")?;
        cursor.skip(PREFLIGHT_DATA_TYPE_BYTES)?;
        Ok(())
    })?;
    budget.fixed_array(cursor, 12, "FunctionIdReferenceToPointer")?;
    preflight_tail_map(cursor, budget, "GlobalReferences", |cursor, budget| {
        cursor.skip(8)?;
        let (name, name_position) = budget.sia_bytes(cursor)?;
        budget.string(cursor, false)?;
        budget.string(cursor, false)?;
        if cursor.read_canonical_bool()? {
            name.decode_utf8(name_position).map_err(invalid_wire)?;
        }
        Ok(())
    })?;
    budget.string_array(cursor, "StaticNames")?;
    preflight_tail_map(cursor, budget, "PropertyReferences", |cursor, budget| {
        cursor.skip(8)?;
        budget.string(cursor, false)?;
        cursor.skip(4)?;
        Ok(())
    })
}

#[derive(Debug, Clone, Copy)]
struct NpcLimits {
    shipping_bytes: usize,
    binds_bytes: usize,
    modules: usize,
    classes: usize,
    functions: usize,
    fields: usize,
    array_elements: usize,
    function_bytecode_dwords: usize,
    total_bytecode_dwords: usize,
    function_instructions: usize,
    default_edges_per_function: usize,
    records: usize,
    string_bytes: usize,
    serialized_string_bytes: usize,
    total_string_bytes: usize,
    source_module_input_bytes: usize,
    source_module_bytecode_dwords: usize,
    source_bytes: usize,
    total_source_bytes: usize,
}

impl Default for NpcLimits {
    fn default() -> Self {
        Self {
            shipping_bytes: MAX_NPC_CACHE_BYTES,
            binds_bytes: MAX_NPC_BINDS_BYTES,
            modules: MAX_NPC_MODULES,
            classes: MAX_NPC_CLASSES,
            functions: MAX_NPC_FUNCTIONS,
            fields: MAX_NPC_FIELDS,
            array_elements: MAX_NPC_ARRAY_ELEMENTS,
            function_bytecode_dwords: MAX_NPC_FUNCTION_BYTECODE_DWORDS,
            total_bytecode_dwords: MAX_NPC_TOTAL_BYTECODE_DWORDS,
            function_instructions: MAX_NPC_FUNCTION_INSTRUCTIONS,
            default_edges_per_function: MAX_NPC_DEFAULT_EDGES_PER_FUNCTION,
            records: MAX_NPC_ARCHETYPES,
            string_bytes: MAX_NPC_STRING_BYTES,
            serialized_string_bytes: MAX_NPC_SERIALIZED_STRING_BYTES,
            total_string_bytes: MAX_NPC_TOTAL_STRING_BYTES,
            source_module_input_bytes: MAX_NPC_SOURCE_MODULE_INPUT_BYTES,
            source_module_bytecode_dwords: MAX_NPC_SOURCE_MODULE_BYTECODE_DWORDS,
            source_bytes: MAX_NPC_SOURCE_BYTES,
            total_source_bytes: MAX_NPC_TOTAL_SOURCE_BYTES,
        }
    }
}

fn seal_bytes(bytes: &[u8]) -> NpcArchetypeSeal {
    NpcArchetypeSeal {
        byte_len: bytes.len() as u64,
        sha256: Sha256::digest(bytes).into(),
    }
}

trait NpcEdgeResolver {
    fn staticclass_class_by_id(&self, id: i32) -> Option<&str>;
    fn member(&self, type_id: i32, offset: i32) -> Option<&str>;
    fn func_by_ptr(&self, ptr: i64) -> Option<&str>;
    fn static_name(&self, id: i64) -> Option<&str>;
}

impl NpcEdgeResolver for RefResolver {
    fn staticclass_class_by_id(&self, id: i32) -> Option<&str> {
        RefResolver::staticclass_class_by_id(self, id)
    }

    fn member(&self, type_id: i32, offset: i32) -> Option<&str> {
        RefResolver::member(self, type_id, offset)
    }

    fn func_by_ptr(&self, ptr: i64) -> Option<&str> {
        RefResolver::func_by_ptr(self, ptr)
    }

    fn static_name(&self, id: i64) -> Option<&str> {
        RefResolver::static_name(self, id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct ClassLocation {
    module_index: usize,
    class_index: usize,
}

#[derive(Debug, Clone)]
struct EdgeHit {
    field_name: String,
    assigned_value: String,
    instruction_offset_dwords: usize,
    bytecode_seal: NpcArchetypeSeal,
}

#[derive(Debug)]
struct RawRecord {
    spawn: ClassLocation,
    ai_config: ClassLocation,
    character_definition: ClassLocation,
    actor_blueprint: String,
    spawn_ai_edge: EdgeHit,
    spawn_blueprint_edge: EdgeHit,
    ai_character_edge: EdgeHit,
}

#[derive(Debug)]
struct RawRejection {
    spawn: ClassLocation,
    reason: NpcArchetypeRejectionReason,
}

#[derive(Debug)]
struct Discovery {
    records: Vec<RawRecord>,
    rejections: Vec<RawRejection>,
}

impl Discovery {
    fn used_module_indices(&self) -> BTreeSet<usize> {
        self.records
            .iter()
            .flat_map(|record| {
                [
                    record.spawn.module_index,
                    record.ai_config.module_index,
                    record.character_definition.module_index,
                ]
            })
            .collect()
    }
}

#[derive(Debug, Clone)]
struct ModuleSourceEvidence {
    relative_path: String,
    source_seal: NpcArchetypeSeal,
}

fn discover(
    modules: &[Module],
    resolver: &impl NpcEdgeResolver,
    limits: NpcLimits,
) -> Result<Discovery, NpcArchetypeError> {
    let mut class_count = 0usize;
    let mut identity_bytes = 0usize;
    let mut classes = BTreeMap::<String, ClassLocation>::new();
    for (module_index, module) in modules.iter().enumerate() {
        validate_identity_string("module name", &module.name, limits, &mut identity_bytes)?;
        validate_identity_string("module path", &module.file, limits, &mut identity_bytes)?;
        for (class_index, class) in module.classes.iter().enumerate() {
            class_count = class_count.saturating_add(1);
            if class_count > limits.classes {
                return Err(NpcArchetypeError::TooManyClasses {
                    actual: class_count,
                    max: limits.classes,
                });
            }
            validate_identity_string("class name", &class.name, limits, &mut identity_bytes)?;
            if let Some(super_class) = &class.super_class {
                validate_identity_string(
                    "super class name",
                    super_class,
                    limits,
                    &mut identity_bytes,
                )?;
            }
            let location = ClassLocation {
                module_index,
                class_index,
            };
            if classes.insert(class.name.clone(), location).is_some() {
                return Err(NpcArchetypeError::DuplicateClass {
                    class_name: class.name.clone(),
                });
            }
        }
    }

    let mut records = Vec::new();
    let mut rejections = Vec::new();
    for (spawn_name, &spawn_location) in &classes {
        if spawn_name == SPAWN_ROOT
            || !matches!(
                ancestry(spawn_name, SPAWN_ROOT, modules, &classes),
                Ancestry::Matches
            )
        {
            continue;
        }
        match discover_spawn(spawn_location, modules, &classes, resolver, limits) {
            Ok(record) => {
                if records.len() >= limits.records {
                    return Err(NpcArchetypeError::TooManyRecords {
                        max: limits.records,
                    });
                }
                records.push(record);
            }
            Err(reason) => rejections.push(RawRejection {
                spawn: spawn_location,
                reason,
            }),
        }
    }
    Ok(Discovery {
        records,
        rejections,
    })
}

fn validate_identity_string(
    kind: &'static str,
    value: &str,
    limits: NpcLimits,
    total: &mut usize,
) -> Result<(), NpcArchetypeError> {
    validate_string_len(kind, value, limits)?;
    *total = total.saturating_add(value.len());
    if *total > limits.total_string_bytes {
        return Err(NpcArchetypeError::TooManyStringBytes {
            max: limits.total_string_bytes,
        });
    }
    Ok(())
}

fn validate_string_len(
    kind: &'static str,
    value: &str,
    limits: NpcLimits,
) -> Result<(), NpcArchetypeError> {
    if value.len() > limits.string_bytes {
        return Err(NpcArchetypeError::StringTooLong {
            kind,
            actual: value.len(),
            max: limits.string_bytes,
        });
    }
    Ok(())
}

fn class_at(modules: &[Module], location: ClassLocation) -> &Class {
    &modules[location.module_index].classes[location.class_index]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ancestry {
    Matches,
    Wrong,
    Cycle,
}

fn ancestry(
    class_name: &str,
    required_base: &str,
    modules: &[Module],
    classes: &BTreeMap<String, ClassLocation>,
) -> Ancestry {
    let mut current = class_name;
    let mut seen = BTreeSet::new();
    loop {
        if current == required_base {
            return Ancestry::Matches;
        }
        if !seen.insert(current) {
            return Ancestry::Cycle;
        }
        let Some(location) = classes.get(current) else {
            return Ancestry::Wrong;
        };
        let Some(parent) = class_at(modules, *location).super_class.as_deref() else {
            return Ancestry::Wrong;
        };
        current = parent;
    }
}

fn discover_spawn(
    spawn_location: ClassLocation,
    modules: &[Module],
    classes: &BTreeMap<String, ClassLocation>,
    resolver: &impl NpcEdgeResolver,
    limits: NpcLimits,
) -> Result<RawRecord, NpcArchetypeRejectionReason> {
    let spawn = class_at(modules, spawn_location);
    reject_non_inheritable("spawn", spawn)?;
    let spawn_defaults = unique_init_defaults(spawn)?;
    let spawn_edges = scan_default_edges(
        &spawn.name,
        spawn_defaults,
        resolver,
        &[SPAWN_AI_FIELD, SPAWN_BLUEPRINT_FIELD],
        limits,
    )?;
    let spawn_ai_edge = unique_edge(spawn, SPAWN_AI_FIELD, &spawn_edges)?;
    let spawn_blueprint_edge = unique_edge(spawn, SPAWN_BLUEPRINT_FIELD, &spawn_edges)?;
    let ai_name = &spawn_ai_edge.assigned_value;
    let &ai_location = classes.get(ai_name).ok_or_else(|| {
        NpcArchetypeRejectionReason::MissingReferencedClass {
            role: "AI config",
            class_name: ai_name.clone(),
        }
    })?;
    require_ancestry("AI config", ai_name, HUMAN_AI_ROOT, modules, classes)?;

    let ai_config = class_at(modules, ai_location);
    reject_non_inheritable("AI config", ai_config)?;
    let ai_defaults = unique_init_defaults(ai_config)?;
    let ai_edges = scan_default_edges(
        &ai_config.name,
        ai_defaults,
        resolver,
        &[AI_CHARACTER_FIELD],
        limits,
    )?;
    let ai_character_edge = unique_edge(ai_config, AI_CHARACTER_FIELD, &ai_edges)?;
    let character_name = &ai_character_edge.assigned_value;
    let &character_location = classes.get(character_name).ok_or_else(|| {
        NpcArchetypeRejectionReason::MissingReferencedClass {
            role: "character definition",
            class_name: character_name.clone(),
        }
    })?;
    require_ancestry(
        "character definition",
        character_name,
        HUMAN_CHARACTER_ROOT,
        modules,
        classes,
    )?;
    reject_non_inheritable(
        "character definition",
        class_at(modules, character_location),
    )?;

    Ok(RawRecord {
        spawn: spawn_location,
        ai_config: ai_location,
        character_definition: character_location,
        actor_blueprint: spawn_blueprint_edge.assigned_value.clone(),
        spawn_ai_edge,
        spawn_blueprint_edge,
        ai_character_edge,
    })
}

fn require_ancestry(
    role: &'static str,
    class_name: &str,
    required_base: &'static str,
    modules: &[Module],
    classes: &BTreeMap<String, ClassLocation>,
) -> Result<(), NpcArchetypeRejectionReason> {
    match ancestry(class_name, required_base, modules, classes) {
        Ancestry::Matches => Ok(()),
        Ancestry::Wrong => Err(NpcArchetypeRejectionReason::WrongAncestry {
            role,
            class_name: class_name.to_owned(),
            required_base,
        }),
        Ancestry::Cycle => Err(NpcArchetypeRejectionReason::InheritanceCycle {
            role,
            class_name: class_name.to_owned(),
        }),
    }
}

fn reject_non_inheritable(
    role: &'static str,
    class: &Class,
) -> Result<(), NpcArchetypeRejectionReason> {
    if class.flags & AS_OBJ_NOINHERIT != 0 {
        Err(NpcArchetypeRejectionReason::NonInheritableClass {
            role,
            class_name: class.name.clone(),
        })
    } else {
        Ok(())
    }
}

fn unique_init_defaults(class: &Class) -> Result<&Func, NpcArchetypeRejectionReason> {
    let mut defaults = class
        .methods
        .iter()
        .filter(|method| method.name == INIT_DEFAULTS);
    let Some(first) = defaults.next() else {
        return Err(NpcArchetypeRejectionReason::MissingInitDefaults {
            owner_class: class.name.clone(),
        });
    };
    let remaining = defaults.count();
    if remaining == 0 {
        Ok(first)
    } else {
        Err(NpcArchetypeRejectionReason::AmbiguousInitDefaults {
            owner_class: class.name.clone(),
            count: remaining + 1,
        })
    }
}

fn unique_edge(
    owner: &Class,
    field_name: &str,
    edges: &BTreeMap<String, Vec<EdgeHit>>,
) -> Result<EdgeHit, NpcArchetypeRejectionReason> {
    match edges.get(field_name).map(Vec::as_slice).unwrap_or_default() {
        [edge] => Ok(edge.clone()),
        [] => Err(NpcArchetypeRejectionReason::MissingDefaultEdge {
            owner_class: owner.name.clone(),
            field_name: field_name.to_owned(),
        }),
        many => Err(NpcArchetypeRejectionReason::AmbiguousDefaultEdge {
            owner_class: owner.name.clone(),
            field_name: field_name.to_owned(),
            count: many.len(),
        }),
    }
}

fn scan_default_edges(
    owner_class: &str,
    function: &Func,
    resolver: &impl NpcEdgeResolver,
    wanted_fields: &[&str],
    limits: NpcLimits,
) -> Result<BTreeMap<String, Vec<EdgeHit>>, NpcArchetypeRejectionReason> {
    if function.bytecode.len() > limits.function_bytecode_dwords {
        return Err(invalid_bytecode_rejection(
            owner_class,
            format!(
                "function has {} bytecode dwords; maximum is {}",
                function.bytecode.len(),
                limits.function_bytecode_dwords
            ),
        ));
    }
    let bytecode_seal = seal_bytecode(&function.bytecode);
    let mut edges = BTreeMap::<String, Vec<EdgeHit>>::new();
    let mut window = VecDeque::<CompactInstr>::with_capacity(6);
    let mut offset_dwords = 0usize;
    let mut instruction_count = 0usize;
    let mut edge_count = 0usize;

    while offset_dwords < function.bytecode.len() {
        instruction_count = instruction_count.saturating_add(1);
        if instruction_count > limits.function_instructions {
            return Err(invalid_bytecode_rejection(
                owner_class,
                format!(
                    "function has more than {} instructions",
                    limits.function_instructions
                ),
            ));
        }
        let instruction = decode_compact_instruction(&function.bytecode, offset_dwords)
            .map_err(|detail| invalid_bytecode_rejection(owner_class, detail))?;
        offset_dwords += instruction.op.size_dwords as usize;
        if window.len() == 6 {
            window.pop_front();
        }
        window.push_back(instruction);
        if window.len() != 6 {
            continue;
        }

        if let Some((field_name, assigned_value)) = class_assignment(&window, resolver) {
            if wanted_fields.contains(&field_name) {
                push_scanned_edge(
                    owner_class,
                    field_name,
                    assigned_value,
                    "assigned class",
                    window[0].offset_dw,
                    bytecode_seal,
                    &mut edges,
                    &mut edge_count,
                    limits,
                )?;
            }
        }
        if let Some((field_name, assigned_value)) = static_name_assignment(&window, resolver) {
            if wanted_fields.contains(&field_name) {
                push_scanned_edge(
                    owner_class,
                    field_name,
                    assigned_value,
                    "assigned name",
                    window[0].offset_dw,
                    bytecode_seal,
                    &mut edges,
                    &mut edge_count,
                    limits,
                )?;
            }
        }
    }
    Ok(edges)
}

fn invalid_bytecode_rejection(owner_class: &str, detail: String) -> NpcArchetypeRejectionReason {
    NpcArchetypeRejectionReason::InvalidInitDefaultsBytecode {
        owner_class: owner_class.to_owned(),
        detail,
    }
}

#[allow(clippy::too_many_arguments)]
fn push_scanned_edge(
    owner_class: &str,
    field_name: &str,
    assigned_value: &str,
    value_kind: &'static str,
    instruction_offset_dwords: usize,
    bytecode_seal: NpcArchetypeSeal,
    edges: &mut BTreeMap<String, Vec<EdgeHit>>,
    edge_count: &mut usize,
    limits: NpcLimits,
) -> Result<(), NpcArchetypeRejectionReason> {
    *edge_count = edge_count.saturating_add(1);
    if *edge_count > limits.default_edges_per_function {
        return Err(invalid_bytecode_rejection(
            owner_class,
            format!(
                "function has more than {} relevant default edges",
                limits.default_edges_per_function
            ),
        ));
    }
    validate_string_len("default field", field_name, limits)
        .map_err(|error| invalid_bytecode_rejection(owner_class, error.to_string()))?;
    validate_string_len(value_kind, assigned_value, limits)
        .map_err(|error| invalid_bytecode_rejection(owner_class, error.to_string()))?;
    edges
        .entry(field_name.to_owned())
        .or_default()
        .push(EdgeHit {
            field_name: field_name.to_owned(),
            assigned_value: assigned_value.to_owned(),
            instruction_offset_dwords,
            bytecode_seal,
        });
    Ok(())
}

#[derive(Clone, Copy)]
struct CompactInstr {
    offset_dw: usize,
    op: &'static OpInfo,
    words: [u16; 3],
    word_count: u8,
    dwords: [u32; 2],
    dword_count: u8,
    qword: Option<u64>,
}

impl CompactInstr {
    fn word(&self, index: usize) -> Option<u16> {
        (index < self.word_count as usize).then_some(self.words[index])
    }

    fn dword(&self, index: usize) -> Option<u32> {
        (index < self.dword_count as usize).then_some(self.dwords[index])
    }
}

fn decode_compact_instruction(bytecode: &[i32], offset_dw: usize) -> Result<CompactInstr, String> {
    use BcType::*;

    let opcode = (*bytecode
        .get(offset_dw)
        .ok_or_else(|| format!("missing opcode at dword {offset_dw}"))? as u32
        & 0xff) as usize;
    let op = OPCODES
        .get(opcode)
        .ok_or_else(|| format!("unknown opcode {opcode} at dword {offset_dw}"))?;
    let size = op.size_dwords as usize;
    if size == 0 || offset_dw.saturating_add(size) > bytecode.len() {
        return Err(format!(
            "truncated instruction at dword {offset_dw}: need {}, have {}",
            size.max(1),
            bytecode.len().saturating_sub(offset_dw)
        ));
    }
    let get = |index: usize| bytecode[offset_dw + index] as u32;
    let high = |dword: u32| ((dword >> 16) & 0xffff) as u16;
    let low = |dword: u32| (dword & 0xffff) as u16;
    let qword = |low: u32, high: u32| low as u64 | ((high as u64) << 32);
    let mut decoded = CompactInstr {
        offset_dw,
        op,
        words: [0; 3],
        word_count: 0,
        dwords: [0; 2],
        dword_count: 0,
        qword: None,
    };
    let mut word = |value| {
        decoded.words[decoded.word_count as usize] = value;
        decoded.word_count += 1;
    };
    let mut dword = |value| {
        decoded.dwords[decoded.dword_count as usize] = value;
        decoded.dword_count += 1;
    };
    match op.fmt {
        INFO | NO_ARG => {}
        W_ARG | wW_ARG | rW_ARG => word(high(get(0))),
        DW_ARG => dword(get(1)),
        rW_DW_ARG | wW_DW_ARG | W_DW_ARG => {
            word(high(get(0)));
            dword(get(1));
        }
        QW_ARG => decoded.qword = Some(qword(get(1), get(2))),
        DW_DW_ARG => {
            dword(get(1));
            dword(get(2));
        }
        wW_rW_rW_ARG => {
            word(high(get(0)));
            word(low(get(1)));
            word(high(get(1)));
        }
        wW_QW_ARG | rW_QW_ARG => {
            word(high(get(0)));
            decoded.qword = Some(qword(get(1), get(2)));
        }
        wW_rW_ARG | rW_rW_ARG | wW_W_ARG | W_rW_ARG => {
            word(high(get(0)));
            word(low(get(1)));
        }
        wW_rW_DW_ARG | rW_W_DW_ARG => {
            word(high(get(0)));
            word(low(get(1)));
            dword(get(2));
        }
        QW_DW_ARG => {
            decoded.qword = Some(qword(get(1), get(2)));
            dword(get(3));
        }
        rW_DW_DW_ARG => {
            word(high(get(0)));
            dword(get(1));
            dword(get(2));
        }
    }
    Ok(decoded)
}

fn class_assignment<'a>(
    window: &VecDeque<CompactInstr>,
    resolver: &'a impl NpcEdgeResolver,
) -> Option<(&'a str, &'a str)> {
    if window[0].op.name != "CALL"
        || window[1].op.name != "STOREOBJ"
        || window[2].op.name != "PshVPtr"
        || window[3].op.name != "PshVPtr"
        || window[4].op.name != "ADDSi"
        || window[5].op.name != "CALLSYS"
        || window[1].word(0) != window[2].word(0)
        || window[3].word(0) != Some(0)
        || resolver.func_by_ptr(window[5].qword? as i64) != Some(ASSIGN_FUNCTION)
    {
        return None;
    }
    let assigned = resolver.staticclass_class_by_id(window[0].dword(0)? as i32)?;
    let offset = window[4].word(0)? as i32;
    let type_id = window[4].dword(0)? as i32;
    let field = resolver.member(type_id, offset)?;
    Some((field, assigned))
}

fn static_name_assignment<'a>(
    window: &VecDeque<CompactInstr>,
    resolver: &'a impl NpcEdgeResolver,
) -> Option<(&'a str, &'a str)> {
    if window[0].op.name != "PshC4"
        || window[1].op.name != "CALLSYS"
        || window[2].op.name != "PshRPtr"
        || window[3].op.name != "PshVPtr"
        || window[4].op.name != "ADDSi"
        || window[5].op.name != "CALLSYS"
        || window[3].word(0) != Some(0)
        || resolver.func_by_ptr(window[1].qword? as i64) != Some(STATIC_NAME_FUNCTION)
        || resolver.func_by_ptr(window[5].qword? as i64) != Some(ASSIGN_FUNCTION)
    {
        return None;
    }
    let assigned = resolver.static_name(window[0].dword(0)? as i64)?;
    let offset = window[4].word(0)? as i32;
    let type_id = window[4].dword(0)? as i32;
    let field = resolver.member(type_id, offset)?;
    Some((field, assigned))
}

fn seal_bytecode(bytecode: &[i32]) -> NpcArchetypeSeal {
    let mut hasher = Sha256::new();
    for dword in bytecode {
        hasher.update(dword.to_le_bytes());
    }
    NpcArchetypeSeal {
        byte_len: bytecode.len().saturating_mul(4) as u64,
        sha256: hasher.finalize().into(),
    }
}

fn materialize(
    modules: &[Module],
    sources: &BTreeMap<usize, ModuleSourceEvidence>,
    discovery: Discovery,
    shipping_cache_seal: NpcArchetypeSeal,
    binds_cache_seal: NpcArchetypeSeal,
    limits: NpcLimits,
) -> Result<NpcArchetypeCollection, NpcArchetypeError> {
    let mut budget = StringBudget::new(limits);
    let mut records = Vec::with_capacity(discovery.records.len());
    for raw in discovery.records {
        let spawn = materialize_class(modules, sources, raw.spawn, &mut budget)?;
        let ai_config = materialize_class(modules, sources, raw.ai_config, &mut budget)?;
        let character_definition =
            materialize_class(modules, sources, raw.character_definition, &mut budget)?;
        let actor_blueprint = budget.keep("actor blueprint", &raw.actor_blueprint)?;
        let blueprint_family = blueprint_family(&actor_blueprint);
        let spawn_ai_edge = materialize_edge(&spawn.class_name, raw.spawn_ai_edge, &mut budget)?;
        let spawn_blueprint_edge =
            materialize_edge(&spawn.class_name, raw.spawn_blueprint_edge, &mut budget)?;
        let ai_character_edge =
            materialize_edge(&ai_config.class_name, raw.ai_character_edge, &mut budget)?;
        let evidence_sha256 = hash_record(
            shipping_cache_seal,
            binds_cache_seal,
            &spawn,
            &ai_config,
            &character_definition,
            &actor_blueprint,
            blueprint_family,
            &spawn_ai_edge,
            &spawn_blueprint_edge,
            &ai_character_edge,
        );
        records.push(NpcArchetypeRecord {
            spawn,
            ai_config,
            character_definition,
            actor_blueprint,
            blueprint_family,
            spawn_ai_edge,
            spawn_blueprint_edge,
            ai_character_edge,
            evidence_sha256,
        });
    }
    records.sort_by(|left, right| left.spawn.class_name.cmp(&right.spawn.class_name));

    let mut rejections = Vec::with_capacity(discovery.rejections.len());
    for raw in discovery.rejections {
        let spawn_class =
            budget.keep("rejected spawn class", &class_at(modules, raw.spawn).name)?;
        charge_rejection_reason(&raw.reason, &mut budget)?;
        rejections.push(NpcArchetypeRejection {
            spawn_class,
            reason: raw.reason,
        });
    }
    rejections.sort_by(|left, right| left.spawn_class.cmp(&right.spawn_class));

    let records_sha256 = hash_collection(shipping_cache_seal, binds_cache_seal, &records);
    Ok(NpcArchetypeCollection {
        shipping_cache_seal,
        binds_cache_seal,
        records,
        rejections,
        records_sha256,
    })
}

fn materialize_class(
    modules: &[Module],
    sources: &BTreeMap<usize, ModuleSourceEvidence>,
    location: ClassLocation,
    budget: &mut StringBudget,
) -> Result<NpcArchetypeClassEvidence, NpcArchetypeError> {
    let module = &modules[location.module_index];
    let class = class_at(modules, location);
    let source = sources
        .get(&location.module_index)
        .ok_or(NpcArchetypeError::MissingSourceEvidence)?;
    Ok(NpcArchetypeClassEvidence {
        class_name: budget.keep("class name", &class.name)?,
        super_class: class
            .super_class
            .as_deref()
            .map(|value| budget.keep("super class name", value))
            .transpose()?,
        module_name: budget.keep("module name", &module.name)?,
        relative_path: budget.keep("relative path", &source.relative_path)?,
        source_seal: source.source_seal,
    })
}

fn materialize_edge(
    owner_class: &str,
    raw: EdgeHit,
    budget: &mut StringBudget,
) -> Result<NpcArchetypeDefaultEdgeEvidence, NpcArchetypeError> {
    let field_name = budget.keep("default field", &raw.field_name)?;
    let assigned_value = budget.keep("assigned default", &raw.assigned_value)?;
    let evidence_sha256 = hash_edge(
        owner_class,
        &field_name,
        &assigned_value,
        raw.instruction_offset_dwords,
        raw.bytecode_seal,
    );
    Ok(NpcArchetypeDefaultEdgeEvidence {
        owner_class: budget.keep("edge owner class", owner_class)?,
        field_name,
        assigned_value,
        instruction_offset_dwords: raw.instruction_offset_dwords,
        init_defaults_bytecode_seal: raw.bytecode_seal,
        evidence_sha256,
    })
}

fn charge_rejection_reason(
    reason: &NpcArchetypeRejectionReason,
    budget: &mut StringBudget,
) -> Result<(), NpcArchetypeError> {
    match reason {
        NpcArchetypeRejectionReason::MissingInitDefaults { owner_class }
        | NpcArchetypeRejectionReason::AmbiguousInitDefaults { owner_class, .. }
        | NpcArchetypeRejectionReason::InvalidInitDefaultsBytecode { owner_class, .. }
        | NpcArchetypeRejectionReason::MissingDefaultEdge { owner_class, .. }
        | NpcArchetypeRejectionReason::AmbiguousDefaultEdge { owner_class, .. } => {
            budget.charge("rejection owner", owner_class)?;
        }
        NpcArchetypeRejectionReason::MissingReferencedClass { class_name, .. }
        | NpcArchetypeRejectionReason::WrongAncestry { class_name, .. }
        | NpcArchetypeRejectionReason::InheritanceCycle { class_name, .. }
        | NpcArchetypeRejectionReason::NonInheritableClass { class_name, .. } => {
            budget.charge("rejected class", class_name)?;
        }
    }
    match reason {
        NpcArchetypeRejectionReason::InvalidInitDefaultsBytecode { detail, .. } => {
            budget.charge("bytecode error", detail)?;
        }
        NpcArchetypeRejectionReason::MissingDefaultEdge { field_name, .. }
        | NpcArchetypeRejectionReason::AmbiguousDefaultEdge { field_name, .. } => {
            budget.charge("rejected field", field_name)?;
        }
        _ => {}
    }
    Ok(())
}

fn blueprint_family(value: &str) -> NpcBlueprintFamily {
    match value {
        HUMAN_BASE_BLUEPRINT | HUMAN_XARDAS_SLEEPER_BLUEPRINT => NpcBlueprintFamily::HumanBase,
        HUMAN_WOMAN_BLUEPRINT => NpcBlueprintFamily::HumanWoman,
        _ => NpcBlueprintFamily::Other,
    }
}

struct StringBudget {
    retained_bytes: usize,
    limits: NpcLimits,
}

impl StringBudget {
    fn new(limits: NpcLimits) -> Self {
        Self {
            retained_bytes: 0,
            limits,
        }
    }

    fn keep(&mut self, kind: &'static str, value: &str) -> Result<String, NpcArchetypeError> {
        self.charge(kind, value)?;
        Ok(value.to_owned())
    }

    fn charge(&mut self, kind: &'static str, value: &str) -> Result<(), NpcArchetypeError> {
        validate_string_len(kind, value, self.limits)?;
        let next = self.retained_bytes.saturating_add(value.len());
        if next > self.limits.total_string_bytes {
            return Err(NpcArchetypeError::TooManyStringBytes {
                max: self.limits.total_string_bytes,
            });
        }
        self.retained_bytes = next;
        Ok(())
    }
}

struct EvidenceHasher(Sha256);

impl EvidenceHasher {
    fn new(domain: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update((domain.len() as u64).to_le_bytes());
        hasher.update(domain);
        Self(hasher)
    }

    fn bytes(&mut self, value: &[u8]) {
        self.0.update((value.len() as u64).to_le_bytes());
        self.0.update(value);
    }

    fn string(&mut self, value: &str) {
        self.bytes(value.as_bytes());
    }

    fn u64(&mut self, value: u64) {
        self.0.update(value.to_le_bytes());
    }

    fn seal(&mut self, value: NpcArchetypeSeal) {
        self.u64(value.byte_len);
        self.bytes(&value.sha256);
    }

    fn finish(self) -> [u8; 32] {
        self.0.finalize().into()
    }
}

fn hash_edge(
    owner_class: &str,
    field_name: &str,
    assigned_value: &str,
    instruction_offset_dwords: usize,
    bytecode_seal: NpcArchetypeSeal,
) -> [u8; 32] {
    let mut hash = EvidenceHasher::new(b"gore-as:npc-default-edge:v1");
    hash.string(owner_class);
    hash.string(field_name);
    hash.string(assigned_value);
    hash.u64(instruction_offset_dwords as u64);
    hash.seal(bytecode_seal);
    hash.finish()
}

#[allow(clippy::too_many_arguments)]
fn hash_record(
    shipping_cache_seal: NpcArchetypeSeal,
    binds_cache_seal: NpcArchetypeSeal,
    spawn: &NpcArchetypeClassEvidence,
    ai_config: &NpcArchetypeClassEvidence,
    character_definition: &NpcArchetypeClassEvidence,
    actor_blueprint: &str,
    blueprint_family: NpcBlueprintFamily,
    spawn_ai_edge: &NpcArchetypeDefaultEdgeEvidence,
    spawn_blueprint_edge: &NpcArchetypeDefaultEdgeEvidence,
    ai_character_edge: &NpcArchetypeDefaultEdgeEvidence,
) -> [u8; 32] {
    let mut hash = EvidenceHasher::new(b"gore-as:npc-archetype-record:v1");
    hash.seal(shipping_cache_seal);
    hash.seal(binds_cache_seal);
    for class in [spawn, ai_config, character_definition] {
        hash.string(&class.class_name);
        hash.string(class.super_class.as_deref().unwrap_or_default());
        hash.string(&class.module_name);
        hash.string(&class.relative_path);
        hash.seal(class.source_seal);
    }
    hash.string(actor_blueprint);
    hash.u64(match blueprint_family {
        NpcBlueprintFamily::HumanBase => 0,
        NpcBlueprintFamily::HumanWoman => 1,
        NpcBlueprintFamily::Other => 2,
    });
    for edge in [spawn_ai_edge, spawn_blueprint_edge, ai_character_edge] {
        hash.bytes(&edge.evidence_sha256);
    }
    hash.finish()
}

fn hash_collection(
    shipping_cache_seal: NpcArchetypeSeal,
    binds_cache_seal: NpcArchetypeSeal,
    records: &[NpcArchetypeRecord],
) -> [u8; 32] {
    let mut hash = EvidenceHasher::new(b"gore-as:npc-archetype-collection:v1");
    hash.seal(shipping_cache_seal);
    hash.seal(binds_cache_seal);
    hash.u64(records.len() as u64);
    for record in records {
        hash.bytes(&record.evidence_sha256);
    }
    hash.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::isa::OPCODES;
    use crate::cache::model::{Field, Param};
    use crate::cache::types::DataType;

    const ASSIGN_CLASS_PTR: i64 = 100;
    const ASSIGN_NAME_PTR: i64 = 101;
    const STATIC_NAME_PTR: i64 = 200;
    const OWNER_TYPE_ID: i32 = 700;
    const AI_OFFSET: u16 = 48;
    const BLUEPRINT_OFFSET: u16 = 56;
    const CHARACTER_OFFSET: u16 = 64;

    #[derive(Default)]
    struct FakeResolver {
        classes: BTreeMap<i32, String>,
        members: BTreeMap<(i32, i32), String>,
        functions: BTreeMap<i64, String>,
        names: BTreeMap<i64, String>,
    }

    impl NpcEdgeResolver for FakeResolver {
        fn staticclass_class_by_id(&self, id: i32) -> Option<&str> {
            self.classes.get(&id).map(String::as_str)
        }

        fn member(&self, type_id: i32, offset: i32) -> Option<&str> {
            self.members.get(&(type_id, offset)).map(String::as_str)
        }

        fn func_by_ptr(&self, ptr: i64) -> Option<&str> {
            self.functions.get(&ptr).map(String::as_str)
        }

        fn static_name(&self, id: i64) -> Option<&str> {
            self.names.get(&id).map(String::as_str)
        }
    }

    fn op(name: &str) -> u32 {
        OPCODES
            .iter()
            .find(|info| info.name == name)
            .unwrap()
            .opcode as u32
    }

    fn push_no_arg(code: &mut Vec<i32>, name: &str) {
        code.push(op(name) as i32);
    }

    fn push_word(code: &mut Vec<i32>, name: &str, word: u16) {
        code.push((op(name) | ((word as u32) << 16)) as i32);
    }

    fn push_dword(code: &mut Vec<i32>, name: &str, value: i32) {
        code.push(op(name) as i32);
        code.push(value);
    }

    fn push_qword(code: &mut Vec<i32>, name: &str, value: i64) {
        let value = value as u64;
        code.push(op(name) as i32);
        code.push(value as u32 as i32);
        code.push((value >> 32) as u32 as i32);
    }

    fn push_word_dword(code: &mut Vec<i32>, name: &str, word: u16, value: i32) {
        code.push((op(name) | ((word as u32) << 16)) as i32);
        code.push(value);
    }

    fn push_class_edge(code: &mut Vec<i32>, class_id: i32, field_offset: u16) {
        push_dword(code, "CALL", class_id);
        push_word(code, "STOREOBJ", 2);
        push_word(code, "PshVPtr", 2);
        push_word(code, "PshVPtr", 0);
        push_word_dword(code, "ADDSi", field_offset, OWNER_TYPE_ID);
        push_qword(code, "CALLSYS", ASSIGN_CLASS_PTR);
    }

    fn push_name_edge(code: &mut Vec<i32>, name_id: i32, field_offset: u16) {
        push_dword(code, "PshC4", name_id);
        push_qword(code, "CALLSYS", STATIC_NAME_PTR);
        push_no_arg(code, "PshRPtr");
        push_word(code, "PshVPtr", 0);
        push_word_dword(code, "ADDSi", field_offset, OWNER_TYPE_ID);
        push_qword(code, "CALLSYS", ASSIGN_NAME_PTR);
    }

    fn finish(code: &mut Vec<i32>) {
        push_word(code, "RET", 2);
    }

    fn function(name: &str, bytecode: Vec<i32>) -> Func {
        Func {
            variable_space: 0,
            name: name.to_owned(),
            namespace: String::new(),
            param_defaults: Vec::new(),
            ret: DataType::default(),
            params: Vec::<Param>::new(),
            bytecode,
            obj_locals: Vec::new(),
            is_ufunction: false,
            traits: 0,
        }
    }

    fn class(name: &str, super_class: Option<&str>, methods: Vec<Func>) -> Class {
        Class {
            name: name.to_owned(),
            namespace: String::new(),
            super_class: super_class.map(str::to_owned),
            fields: Vec::<Field>::new(),
            methods,
            ctors: Vec::new(),
            flags: 1,
        }
    }

    struct Fixture {
        modules: Vec<Module>,
        resolver: FakeResolver,
    }

    fn fixture() -> Fixture {
        let mut resolver = FakeResolver::default();
        resolver
            .classes
            .insert(1, "UAi_Without_Name_Affinity".into());
        resolver
            .classes
            .insert(2, "UCharacter_Completely_Different".into());
        resolver
            .members
            .insert((OWNER_TYPE_ID, AI_OFFSET as i32), SPAWN_AI_FIELD.into());
        resolver.members.insert(
            (OWNER_TYPE_ID, BLUEPRINT_OFFSET as i32),
            SPAWN_BLUEPRINT_FIELD.into(),
        );
        resolver.members.insert(
            (OWNER_TYPE_ID, CHARACTER_OFFSET as i32),
            AI_CHARACTER_FIELD.into(),
        );
        resolver
            .functions
            .insert(ASSIGN_CLASS_PTR, ASSIGN_FUNCTION.into());
        resolver
            .functions
            .insert(ASSIGN_NAME_PTR, ASSIGN_FUNCTION.into());
        resolver
            .functions
            .insert(STATIC_NAME_PTR, STATIC_NAME_FUNCTION.into());
        resolver.names.insert(7, HUMAN_BASE_BLUEPRINT.into());

        let mut spawn_code = Vec::new();
        push_class_edge(&mut spawn_code, 1, AI_OFFSET);
        push_name_edge(&mut spawn_code, 7, BLUEPRINT_OFFSET);
        finish(&mut spawn_code);
        let mut ai_code = Vec::new();
        push_class_edge(&mut ai_code, 2, CHARACTER_OFFSET);
        finish(&mut ai_code);

        let classes = vec![
            class(SPAWN_ROOT, None, Vec::new()),
            class(HUMAN_AI_ROOT, None, Vec::new()),
            class(HUMAN_CHARACTER_ROOT, None, Vec::new()),
            class(
                "USpawn_Unrelated_Label",
                Some(SPAWN_ROOT),
                vec![function(INIT_DEFAULTS, spawn_code)],
            ),
            class(
                "UAi_Without_Name_Affinity",
                Some(HUMAN_AI_ROOT),
                vec![function(INIT_DEFAULTS, ai_code)],
            ),
            class(
                "UCharacter_Completely_Different",
                Some(HUMAN_CHARACTER_ROOT),
                Vec::new(),
            ),
        ];
        Fixture {
            modules: vec![Module {
                name: "Synthetic.Npcs".into(),
                file: "Synthetic/Npcs.as".into(),
                functions: Vec::new(),
                classes,
                enums: Vec::new(),
                globals: Vec::new(),
            }],
            resolver,
        }
    }

    fn synthetic_sources(modules: &[Module]) -> BTreeMap<usize, ModuleSourceEvidence> {
        modules
            .iter()
            .enumerate()
            .map(|(index, module)| {
                (
                    index,
                    ModuleSourceEvidence {
                        relative_path: module.file.clone(),
                        source_seal: seal_bytes(module.name.as_bytes()),
                    },
                )
            })
            .collect()
    }

    fn collect_fixture(
        fixture: &Fixture,
        limits: NpcLimits,
    ) -> Result<NpcArchetypeCollection, NpcArchetypeError> {
        let discovery = discover(&fixture.modules, &fixture.resolver, limits)?;
        materialize(
            &fixture.modules,
            &synthetic_sources(&fixture.modules),
            discovery,
            seal_bytes(b"shipping"),
            seal_bytes(b"binds"),
            limits,
        )
    }

    fn class_mut<'a>(fixture: &'a mut Fixture, name: &str) -> &'a mut Class {
        fixture.modules[0]
            .classes
            .iter_mut()
            .find(|class| class.name == name)
            .unwrap()
    }

    fn push_preflight_sia(out: &mut Vec<u8>, value: &[u8]) {
        if value.is_empty() {
            out.extend_from_slice(&0i32.to_le_bytes());
        } else {
            out.extend_from_slice(&(value.len() as i32).to_le_bytes());
            out.extend_from_slice(value);
            out.push(0);
        }
    }

    fn preflight_tail_with_global_name(
        name: &[u8],
        is_string: i32,
    ) -> Result<(), NpcArchetypeError> {
        let mut tail = Vec::new();
        tail.extend_from_slice(&0i32.to_le_bytes()); // TypeReferences
        tail.extend_from_slice(&0i32.to_le_bytes()); // TypeIdReferenceToPointer
        tail.extend_from_slice(&0i32.to_le_bytes()); // FunctionReferences
        tail.extend_from_slice(&0i32.to_le_bytes()); // FunctionIdReferenceToPointer
        tail.extend_from_slice(&1i32.to_le_bytes()); // GlobalReferences
        tail.extend_from_slice(&0x3000i64.to_le_bytes());
        push_preflight_sia(&mut tail, name);
        push_preflight_sia(&mut tail, b""); // Module
        push_preflight_sia(&mut tail, b""); // Namespace
        tail.extend_from_slice(&is_string.to_le_bytes());
        tail.extend_from_slice(&0i32.to_le_bytes()); // StaticNames
        tail.extend_from_slice(&0i32.to_le_bytes()); // PropertyReferences

        let mut cursor = PreflightCursor::at(&tail, 0);
        let mut budget = PreflightBudget::new(NpcLimits::default());
        preflight_tail(&mut cursor, &mut budget)?;
        assert_eq!(cursor.position, tail.len());
        Ok(())
    }

    #[test]
    fn exact_bytecode_edges_accept_non_suffix_triple_deterministically() {
        let fixture = fixture();
        let first = collect_fixture(&fixture, NpcLimits::default()).unwrap();
        let second = collect_fixture(&fixture, NpcLimits::default()).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.records.len(), 1);
        assert!(first.rejections.is_empty());
        let record = &first.records[0];
        assert_eq!(record.spawn.class_name, "USpawn_Unrelated_Label");
        assert_eq!(record.ai_config.class_name, "UAi_Without_Name_Affinity");
        assert_eq!(
            record.character_definition.class_name,
            "UCharacter_Completely_Different"
        );
        assert_eq!(record.blueprint_family, NpcBlueprintFamily::HumanBase);
        assert_ne!(record.spawn_ai_edge.evidence_sha256, [0; 32]);
        assert_ne!(record.evidence_sha256, [0; 32]);
    }

    #[test]
    fn two_spawn_variants_pointing_to_one_ai_are_preserved() {
        let mut fixture = fixture();
        let original = class_mut(&mut fixture, "USpawn_Unrelated_Label").clone();
        let mut variant = original;
        variant.name = "UAnotherSpawnVariant".into();
        fixture.modules[0].classes.push(variant);
        let collection = collect_fixture(&fixture, NpcLimits::default()).unwrap();
        assert_eq!(collection.records.len(), 2);
        assert_eq!(
            collection
                .records
                .iter()
                .map(|record| record.ai_config.class_name.as_str())
                .collect::<BTreeSet<_>>(),
            BTreeSet::from(["UAi_Without_Name_Affinity"])
        );
    }

    #[test]
    fn missing_and_ambiguous_edges_reject_only_the_candidate() {
        let mut missing = fixture();
        let method = &mut class_mut(&mut missing, "USpawn_Unrelated_Label").methods[0];
        let mut code = Vec::new();
        push_class_edge(&mut code, 1, AI_OFFSET);
        finish(&mut code);
        method.bytecode = code;
        let collection = collect_fixture(&missing, NpcLimits::default()).unwrap();
        assert!(collection.records.is_empty());
        assert!(matches!(
            collection.rejections[0].reason,
            NpcArchetypeRejectionReason::MissingDefaultEdge { ref field_name, .. }
                if field_name == SPAWN_BLUEPRINT_FIELD
        ));

        let mut ambiguous = fixture();
        let method = &mut class_mut(&mut ambiguous, "USpawn_Unrelated_Label").methods[0];
        let mut code = Vec::new();
        push_class_edge(&mut code, 1, AI_OFFSET);
        push_class_edge(&mut code, 1, AI_OFFSET);
        push_name_edge(&mut code, 7, BLUEPRINT_OFFSET);
        finish(&mut code);
        method.bytecode = code;
        let collection = collect_fixture(&ambiguous, NpcLimits::default()).unwrap();
        assert!(matches!(
            collection.rejections[0].reason,
            NpcArchetypeRejectionReason::AmbiguousDefaultEdge { count: 2, .. }
        ));
    }

    #[test]
    fn wrong_ancestry_missing_reference_and_non_inheritable_reject_per_record() {
        let mut wrong = fixture();
        class_mut(&mut wrong, "UAi_Without_Name_Affinity").super_class = None;
        let collection = collect_fixture(&wrong, NpcLimits::default()).unwrap();
        assert!(matches!(
            collection.rejections[0].reason,
            NpcArchetypeRejectionReason::WrongAncestry {
                role: "AI config",
                ..
            }
        ));

        let mut missing = fixture();
        missing.resolver.classes.insert(1, "UMissingAi".into());
        let collection = collect_fixture(&missing, NpcLimits::default()).unwrap();
        assert!(matches!(
            collection.rejections[0].reason,
            NpcArchetypeRejectionReason::MissingReferencedClass {
                role: "AI config",
                ..
            }
        ));

        let mut final_class = fixture();
        class_mut(&mut final_class, "UAi_Without_Name_Affinity").flags |= AS_OBJ_NOINHERIT;
        let collection = collect_fixture(&final_class, NpcLimits::default()).unwrap();
        assert!(matches!(
            collection.rejections[0].reason,
            NpcArchetypeRejectionReason::NonInheritableClass {
                role: "AI config",
                ..
            }
        ));
    }

    #[test]
    fn duplicate_classes_and_global_bounds_fail_closed() {
        let mut duplicate = fixture();
        let duplicate_class = duplicate.modules[0].classes[3].clone();
        duplicate.modules[0].classes.push(duplicate_class);
        assert!(matches!(
            collect_fixture(&duplicate, NpcLimits::default()),
            Err(NpcArchetypeError::DuplicateClass { .. })
        ));

        let limits = NpcLimits {
            classes: 2,
            ..NpcLimits::default()
        };
        assert!(matches!(
            collect_fixture(&fixture(), limits),
            Err(NpcArchetypeError::TooManyClasses { max: 2, .. })
        ));

        let limits = NpcLimits {
            records: 0,
            ..NpcLimits::default()
        };
        assert!(matches!(
            collect_fixture(&fixture(), limits),
            Err(NpcArchetypeError::TooManyRecords { max: 0 })
        ));

        let limits = NpcLimits {
            string_bytes: 8,
            ..NpcLimits::default()
        };
        assert!(matches!(
            collect_fixture(&fixture(), limits),
            Err(NpcArchetypeError::StringTooLong { max: 8, .. })
        ));
    }

    #[test]
    fn raw_entrypoint_checks_bounds_and_seals_before_parsing() {
        let cache = [1u8, 2, 3];
        let binds = [4u8, 5, 6];
        let limits = NpcLimits {
            shipping_bytes: 2,
            ..NpcLimits::default()
        };
        assert!(matches!(
            collect_npc_archetypes_with_limits(&cache, &binds, [0; 32], [0; 32], limits),
            Err(NpcArchetypeError::ShippingCacheTooLarge { max: 2, .. })
        ));
        assert!(matches!(
            collect_npc_archetypes(&cache, &binds, [0; 32], [0; 32]),
            Err(NpcArchetypeError::ShippingSealMismatch)
        ));
        let cache_seal: [u8; 32] = Sha256::digest(cache).into();
        assert!(matches!(
            collect_npc_archetypes(&cache, &binds, cache_seal, [0; 32]),
            Err(NpcArchetypeError::BindsSealMismatch)
        ));
    }

    #[test]
    fn structural_preflight_rejects_counts_and_strings_before_payload_allocation() {
        let mut excessive_modules = vec![0u8; 24];
        excessive_modules[20..24].copy_from_slice(&3u32.to_le_bytes());
        let limits = NpcLimits {
            modules: 2,
            ..NpcLimits::default()
        };
        assert!(matches!(
            preflight_cache(&excessive_modules, limits),
            Err(NpcArchetypeError::StructuralLimitExceeded {
                kind: "modules",
                actual: 3,
                max: 2,
            })
        ));

        let limits = NpcLimits {
            serialized_string_bytes: 8,
            ..NpcLimits::default()
        };
        let mut excessive_key = vec![0u8; 24];
        excessive_key[20..24].copy_from_slice(&1u32.to_le_bytes());
        excessive_key.extend_from_slice(&10i32.to_le_bytes());
        assert!(matches!(
            preflight_cache(&excessive_key, limits),
            Err(NpcArchetypeError::StringTooLong {
                kind: "serialized cache string",
                actual: 9,
                max: 8,
            })
        ));
    }

    #[test]
    fn structural_preflight_validates_global_name_with_contextual_encoding() {
        preflight_tail_with_global_name("Grüße 世界".as_bytes(), 1).unwrap();
        preflight_tail_with_global_name(&[0xff], 0).unwrap();

        assert!(matches!(
            preflight_tail_with_global_name(&[0xff], 1),
            Err(NpcArchetypeError::InvalidShippingCache(message))
                if message.contains("script string literal is not valid UTF-8")
        ));
        assert!(matches!(
            preflight_tail_with_global_name(b"literal", 2),
            Err(NpcArchetypeError::InvalidShippingCache(message))
                if message.contains("field bool")
        ));
    }

    #[test]
    fn structural_preflight_rejects_bytecode_count_before_reading_payload() {
        let limits = NpcLimits {
            function_bytecode_dwords: 4,
            ..NpcLimits::default()
        };
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&0i32.to_le_bytes()); // function name
        bytes.extend_from_slice(&0i32.to_le_bytes()); // namespace
        bytes.extend_from_slice(&[0u8; PREFLIGHT_DATA_TYPE_BYTES]);
        for _ in 0..4 {
            bytes.extend_from_slice(&0i32.to_le_bytes()); // params/names/flags/default args
        }
        bytes.extend_from_slice(&0i32.to_le_bytes()); // traits
        bytes.extend_from_slice(&5i32.to_le_bytes()); // bytecode count, no payload
        let mut cursor = PreflightCursor::at(&bytes, 0);
        let mut budget = PreflightBudget::new(limits);
        let mut module_bytecode = 0;
        assert!(matches!(
            preflight_function(&mut cursor, &mut budget, &mut module_bytecode),
            Err(NpcArchetypeError::StructuralLimitExceeded {
                kind: "function bytecode dwords",
                actual: 5,
                max: 4,
            })
        ));
        assert_eq!(cursor.position, bytes.len());
        assert_eq!(module_bytecode, 0);
    }

    #[test]
    fn rolling_scanner_and_source_generation_have_independent_hard_caps() {
        let instruction_limits = NpcLimits {
            function_instructions: 5,
            ..NpcLimits::default()
        };
        let collection = collect_fixture(&fixture(), instruction_limits).unwrap();
        assert!(matches!(
            collection.rejections[0].reason,
            NpcArchetypeRejectionReason::InvalidInitDefaultsBytecode { .. }
        ));

        let edge_limits = NpcLimits {
            default_edges_per_function: 1,
            ..NpcLimits::default()
        };
        let collection = collect_fixture(&fixture(), edge_limits).unwrap();
        assert!(matches!(
            collection.rejections[0].reason,
            NpcArchetypeRejectionReason::InvalidInitDefaultsBytecode { .. }
        ));

        let source_limits = NpcLimits {
            source_module_input_bytes: 10,
            source_module_bytecode_dwords: 10,
            source_bytes: 10,
            total_source_bytes: 15,
            ..NpcLimits::default()
        };
        assert!(matches!(
            validate_source_generation_input(
                "Large",
                ModulePreflight {
                    serialized_bytes: 11,
                    bytecode_dwords: 0,
                },
                source_limits,
            ),
            Err(NpcArchetypeError::SourceGenerationLimitExceeded {
                kind: "serialized input bytes",
                actual: 11,
                max: 10,
                ..
            })
        ));
        let mut total = 0;
        charge_source_output("One", 10, &mut total, source_limits).unwrap();
        assert_eq!(total, 10);
        assert!(matches!(
            charge_source_output("Two", 6, &mut total, source_limits),
            Err(NpcArchetypeError::TooManySourceBytes { max: 15 })
        ));
        assert_eq!(total, 10);
    }

    #[test]
    #[ignore = "requires explicit GORE_AS_REAL_CACHE and GORE_AS_BINDS sealed fixtures"]
    fn configured_real_npc_golden_is_stable() {
        let cache_path = std::env::var_os("GORE_AS_REAL_CACHE")
            .expect("GORE_AS_REAL_CACHE is required for the ignored real NPC golden");
        let binds_path = std::env::var_os("GORE_AS_BINDS")
            .expect("GORE_AS_BINDS is required for the ignored real NPC golden");
        let cache = std::fs::read(cache_path).expect("read configured Shipping cache");
        let binds = std::fs::read(binds_path).expect("read configured Binds cache");
        let expected_shipping_sha256 = [
            0x10, 0x18, 0xf1, 0xcf, 0xe6, 0xb9, 0x9a, 0x65, 0x0e, 0xec, 0xb3, 0x3a, 0xfb, 0x96,
            0x75, 0x2d, 0x69, 0x1d, 0x20, 0x88, 0xea, 0xd2, 0x78, 0x08, 0x97, 0x1b, 0x81, 0x2f,
            0x04, 0xec, 0xb4, 0xc2,
        ];
        let expected_binds_sha256 = [
            0x46, 0xe6, 0x62, 0x9a, 0xd5, 0xca, 0xcc, 0x11, 0x2b, 0x99, 0x22, 0xd4, 0x8a, 0x1a,
            0xa9, 0x48, 0xf4, 0x05, 0x72, 0xd7, 0x28, 0x57, 0x05, 0xb9, 0x81, 0xc3, 0xec, 0xa3,
            0xdc, 0x61, 0x5f, 0xea,
        ];
        let first = collect_npc_archetypes(
            &cache,
            &binds,
            expected_shipping_sha256,
            expected_binds_sha256,
        )
        .expect("collect real NPC archetypes");
        let second = collect_npc_archetypes(
            &cache,
            &binds,
            expected_shipping_sha256,
            expected_binds_sha256,
        )
        .expect("repeat real NPC archetypes");
        assert_eq!(first, second);
        assert_eq!(first.records.len(), 634);
        assert_eq!(first.rejections.len(), 416);
        assert_eq!(
            first
                .records
                .iter()
                .map(|record| record.ai_config.class_name.as_str())
                .collect::<BTreeSet<_>>()
                .len(),
            621
        );
        assert_eq!(
            first
                .records
                .iter()
                .map(|record| record.character_definition.class_name.as_str())
                .collect::<BTreeSet<_>>()
                .len(),
            621
        );

        // Diagnostic historical subset only. Production joining never consults these names:
        // ancestry plus bytecode edges also retain 12 base-endpoint records and five specialized
        // Human descendants that the earlier name inventory omitted.
        let named_human = first
            .records
            .iter()
            .filter(|record| {
                record
                    .character_definition
                    .class_name
                    .starts_with("UCharacterDefinition_Human_")
            })
            .collect::<Vec<_>>();
        assert_eq!(named_human.len(), 617);
        assert_eq!(
            named_human
                .iter()
                .map(|record| record.ai_config.class_name.as_str())
                .collect::<BTreeSet<_>>()
                .len(),
            615
        );
        assert_eq!(
            named_human
                .iter()
                .map(|record| record.character_definition.class_name.as_str())
                .collect::<BTreeSet<_>>()
                .len(),
            615
        );
        let non_suffix = named_human
            .iter()
            .filter(|record| {
                let spawn = record
                    .spawn
                    .class_name
                    .strip_prefix("USpawnAIAgentDefinition_");
                let ai = record
                    .ai_config
                    .class_name
                    .strip_prefix("UAIAgentConfig_Human_")
                    .or_else(|| record.ai_config.class_name.strip_prefix("UAIAgentConfig_"));
                spawn != ai
            })
            .count();
        assert_eq!(non_suffix, 8);
        let mut variants = BTreeMap::<&str, usize>::new();
        for record in &named_human {
            *variants.entry(&record.ai_config.class_name).or_default() += 1;
        }
        assert_eq!(variants.values().filter(|&&count| count > 1).count(), 2);
        let named_base = named_human
            .iter()
            .filter(|record| record.blueprint_family == NpcBlueprintFamily::HumanBase)
            .count();
        assert_eq!(
            named_base,
            610,
            "non-base records: {:?}",
            named_human
                .iter()
                .filter(|record| record.blueprint_family != NpcBlueprintFamily::HumanBase)
                .map(|record| (
                    &record.spawn.class_name,
                    &record.actor_blueprint,
                    record.blueprint_family,
                ))
                .collect::<Vec<_>>()
        );
        assert_eq!(
            named_human
                .iter()
                .filter(|record| record.blueprint_family == NpcBlueprintFamily::HumanWoman)
                .count(),
            7
        );
        assert_eq!(
            first.records_sha256,
            [
                0xc2, 0xe8, 0x2d, 0x49, 0x57, 0x2b, 0x00, 0xf0, 0x14, 0x90, 0xe1, 0xb1, 0xb2, 0xc1,
                0x2d, 0x58, 0x09, 0x9c, 0x5f, 0x40, 0x57, 0xb3, 0x9b, 0xed, 0x2d, 0x4e, 0x5b, 0xb8,
                0x29, 0x36, 0x75, 0x36,
            ]
        );
    }
}
