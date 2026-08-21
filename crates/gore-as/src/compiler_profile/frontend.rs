//! Strict profile payloads for the G1R source frontend and compiler diagnostics.
//!
//! These values are read directly by the pinned preprocessor, class generator, or modified
//! AngelScript core. Settings which only influence bind registration are deliberately absent:
//! their effective result is already sealed in the ordered registration trace.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

use super::manifest::{FrontendProfileV1, SealedBlobV1, Sha256Digest};

pub const PREPROCESSOR_CONFIG_SCHEMA: &str = "gore.as.preprocessor-config";
pub const CLASS_GENERATOR_CONFIG_SCHEMA: &str = "gore.as.class-generator-config";
pub const COMPILER_OPTIONS_SCHEMA: &str = "gore.as.compiler-options";
pub const FRONTEND_SCHEMA_VERSION: u32 = 1;

pub const MAX_PREPROCESSOR_CONFIG_BYTES_V1: usize = 4 * 1024 * 1024;
pub const MAX_CLASS_GENERATOR_CONFIG_BYTES_V1: usize = 64 * 1024;
pub const MAX_COMPILER_OPTIONS_BYTES_V1: usize = 64 * 1024;

const MAX_PREPROCESSOR_FLAGS: usize = 4096;
const MAX_FLAG_BYTES: usize = 4096;
const MAX_BLUEPRINT_EVENT_ARGUMENT_SPECIALIZATIONS: usize = 4096;
const MAX_BLUEPRINT_EVENT_ARGUMENT_SPECIALIZATION_BYTES: usize = 4096;
const MAX_NATIVE_SUPER_TYPES: usize = 1_000_000;
const MAX_NATIVE_SUPER_TYPE_NAME_BYTES: usize = 4096;
const MAX_FNAME_COMPARISON_KEYS: usize = 1_000_000;
const MAX_EXTERNAL_CLASS_CAPTURES: usize = 1_000_000;
const MAX_EXTERNAL_GRAPH_CAPTURES: usize = 4096;
const MAX_EXTERNAL_GRAPH_MODULES: usize = 4096;
const MAX_EXTERNAL_NAME_BYTES: usize = 4096;
const MAX_EXTERNAL_GENERATED_BYTES: usize = 16 * 1024 * 1024;
const PREPROCESSOR_HASH_DOMAIN: &[u8] = b"gore-as-preprocessor-config-v1\0";
const EXTERNAL_GRAPH_OUTPUT_HASH_DOMAIN: &[u8] = b"gore-as-external-hook-graph-output-v1\0";
const CLASS_GENERATOR_HASH_DOMAIN: &[u8] = b"gore-as-class-generator-config-v1\0";
const COMPILER_OPTIONS_HASH_DOMAIN: &[u8] = b"gore-as-compiler-options-v1\0";
const BUILTIN_PREPROCESSOR_FLAGS: [&str; 6] = [
    "COOK_COMMANDLET",
    "EDITOR",
    "EDITORONLY_DATA",
    "RELEASE",
    "TEST",
    "WITH_SERVER_CODE",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EffectivePreprocessorFlagV1 {
    pub ordinal: u32,
    pub name: String,
    pub value: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PropertyEditSpecifierV1 {
    EditAnywhere,
    EditInstanceOnly,
    EditDefaultsOnly,
    NotEditable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PropertyBlueprintSpecifierV1 {
    BlueprintReadWrite,
    BlueprintReadOnly,
    BlueprintHidden,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StaticClassModeV1 {
    Allowed,
    Deprecated,
    Disallowed,
}

/// Most-specific native UClass category consulted by `AnalyzeClasses` when it
/// emits class helper functions. Every entry is also proof that the bound
/// AngelScript type resolves to a UClass rather than a struct/value type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeSuperKindV1 {
    Actor,
    ActorComponent,
    EngineSubsystem,
    EditorSubsystem,
    GameInstanceSubsystem,
    WorldSubsystem,
    LocalPlayerSubsystem,
    OtherUObject,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeSuperTypeV1 {
    pub ordinal: u32,
    pub angelscript_type_name: String,
    /// Stable `UClass::GetPathName()` value serialized as `CodeSuperClass`.
    pub unreal_class_path: String,
    /// `UClass::GetPropertiesSize()` used as the script shadow-layout offset.
    pub property_offset: u64,
    pub kind: NativeSuperKindV1,
    pub cannot_derive_angelscript: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FNameComparisonKeyV1 {
    pub ordinal: u32,
    pub spelling: String,
    pub comparison_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClassAnalyzeCaptureV1 {
    pub ordinal: u32,
    pub module_name: String,
    pub namespace: String,
    pub class_name: String,
    pub source_sha256: Sha256Digest,
    pub input_generated_statics_sha256: Sha256Digest,
    pub generated_statics: String,
    pub output_generated_statics_sha256: Sha256Digest,
    pub has_statics: bool,
    pub compose_onto_class: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClassAnalyzeHookProfileV1 {
    pub bound: bool,
    pub captures: Vec<ClassAnalyzeCaptureV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GraphHookModuleCaptureV1 {
    pub ordinal: u32,
    pub module_name: String,
    pub generated_declarations: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GraphHookCaptureV1 {
    pub ordinal: u32,
    pub input_graph_sha256: Sha256Digest,
    pub output_graph_sha256: Sha256Digest,
    pub modules: Vec<GraphHookModuleCaptureV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GraphHookProfileV1 {
    pub bound: bool,
    pub captures: Vec<GraphHookCaptureV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalFrontendHooksV1 {
    pub class_analyze: ClassAnalyzeHookProfileV1,
    pub process_chunks: GraphHookProfileV1,
    pub post_process_code: GraphHookProfileV1,
}

impl ExternalFrontendHooksV1 {
    pub fn unbound() -> Self {
        Self {
            class_analyze: ClassAnalyzeHookProfileV1 {
                bound: false,
                captures: Vec::new(),
            },
            process_chunks: GraphHookProfileV1 {
                bound: false,
                captures: Vec::new(),
            },
            post_process_code: GraphHookProfileV1 {
                bound: false,
                captures: Vec::new(),
            },
        }
    }
}

/// Effective constructor inputs for `FAngelscriptPreprocessor` plus the source-discovery mode.
///
/// `effective_flags` is the final map after built-ins, configured flags, and any donor overrides.
/// It is serialized in lexical name order so equivalent maps have one canonical representation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreprocessorConfigV1 {
    pub schema: String,
    pub schema_version: u32,
    pub automatic_imports: bool,
    pub warn_on_manual_import_statements: bool,
    pub use_editor_scripts: bool,
    pub effective_flags: Vec<EffectivePreprocessorFlagV1>,
    pub default_function_blueprint_callable: bool,
    pub default_property_edit_specifier: PropertyEditSpecifierV1,
    pub default_property_edit_specifier_for_structs: PropertyEditSpecifierV1,
    pub default_property_blueprint_specifier: PropertyBlueprintSpecifierV1,
    pub static_class_mode: StaticClassModeV1,
    pub script_float_is_float64: bool,
    /// Effective compile-time value of `WITH_ANGELSCRIPT_HAZE` in the game.
    pub angelscript_haze: bool,
    /// Effective compile-time value of `AS_ENFORCE_SERVER_RPC_VALIDATION`.
    pub enforce_server_rpc_validation: bool,
    /// Final membership set populated by `Bind_BlueprintEvent`. The
    /// preprocessor uses it to select typed `__Evt_PushArgument*` wrappers.
    /// It is canonicalized in strictly sorted, duplicate-free order.
    pub blueprint_event_argument_specializations: Vec<String>,
    /// Ordered lookup materialized after binds from `FAngelscriptType::GetClass`
    /// and native UClass ancestry/metadata.
    pub native_super_types: Vec<NativeSuperTypeV1>,
    /// Captured case-insensitive identities used by Unreal's FName comparison semantics.
    pub fname_comparison_keys: Vec<FNameComparisonKeyV1>,
    /// Exact replay material for host callbacks outside the portable preprocessor core.
    pub external_hooks: ExternalFrontendHooksV1,
    pub canonical_sha256: Sha256Digest,
}

impl PreprocessorConfigV1 {
    pub fn seal(&mut self) -> Result<(), FrontendProfileError> {
        self.validate_structure()?;
        self.canonical_sha256 = self.computed_digest()?;
        Ok(())
    }

    pub fn validate(&self) -> Result<(), FrontendProfileError> {
        self.validate_structure()?;
        check_digest(
            "preprocessor config",
            self.canonical_sha256,
            self.computed_digest()?,
        )
    }

    pub fn from_json(bytes: &[u8]) -> Result<Self, FrontendProfileError> {
        parse_bounded(
            bytes,
            MAX_PREPROCESSOR_CONFIG_BYTES_V1,
            "preprocessor config",
        )
    }

    pub fn to_json(&self) -> Result<Vec<u8>, FrontendProfileError> {
        self.validate()?;
        Ok(serde_json::to_vec_pretty(self)?)
    }

    fn validate_structure(&self) -> Result<(), FrontendProfileError> {
        check_schema(
            &self.schema,
            self.schema_version,
            PREPROCESSOR_CONFIG_SCHEMA,
        )?;
        if self.effective_flags.len() > MAX_PREPROCESSOR_FLAGS {
            return Err(FrontendProfileError::CountTooLarge {
                field: "effective preprocessor flags",
                actual: self.effective_flags.len(),
                max: MAX_PREPROCESSOR_FLAGS,
            });
        }

        let mut names = BTreeSet::new();
        let mut previous: Option<&str> = None;
        for (index, flag) in self.effective_flags.iter().enumerate() {
            if flag.ordinal as usize != index {
                return Err(FrontendProfileError::Order {
                    field: "effective preprocessor flag",
                    expected: index,
                    actual: flag.ordinal,
                });
            }
            validate_flag_name(&flag.name)?;
            if previous.is_some_and(|name| name >= flag.name.as_str()) {
                return invalid(
                    "effective_flags",
                    "must be strictly sorted by case-sensitive flag name",
                );
            }
            if !names.insert(flag.name.as_str()) {
                return invalid("effective_flags", "contains a duplicate flag name");
            }
            previous = Some(&flag.name);
        }
        for required in BUILTIN_PREPROCESSOR_FLAGS {
            if !names.contains(required) {
                return Err(FrontendProfileError::MissingBuiltinFlag(required));
            }
        }
        validate_sorted_text_set(
            &self.blueprint_event_argument_specializations,
            MAX_BLUEPRINT_EVENT_ARGUMENT_SPECIALIZATIONS,
            MAX_BLUEPRINT_EVENT_ARGUMENT_SPECIALIZATION_BYTES,
            "blueprint_event_argument_specializations",
        )?;
        if self.native_super_types.len() > MAX_NATIVE_SUPER_TYPES {
            return Err(FrontendProfileError::CountTooLarge {
                field: "native_super_types",
                actual: self.native_super_types.len(),
                max: MAX_NATIVE_SUPER_TYPES,
            });
        }
        let mut previous_native: Option<&str> = None;
        let mut native_paths = BTreeSet::new();
        for (index, native) in self.native_super_types.iter().enumerate() {
            if native.ordinal as usize != index {
                return Err(FrontendProfileError::Order {
                    field: "native super type",
                    expected: index,
                    actual: native.ordinal,
                });
            }
            validate_profile_text(
                &native.angelscript_type_name,
                MAX_NATIVE_SUPER_TYPE_NAME_BYTES,
                "native_super_types.angelscript_type_name",
            )?;
            validate_profile_text(
                &native.unreal_class_path,
                MAX_NATIVE_SUPER_TYPE_NAME_BYTES,
                "native_super_types.unreal_class_path",
            )?;
            if native.property_offset > i32::MAX as u64 {
                return invalid(
                    "native_super_types.property_offset",
                    "must fit the fork's signed class-layout offset",
                );
            }
            if !native_paths.insert(native.unreal_class_path.as_str()) {
                return invalid(
                    "native_super_types",
                    "must map each Unreal class path to exactly one bound type",
                );
            }
            if previous_native.is_some_and(|name| name >= native.angelscript_type_name.as_str()) {
                return invalid(
                    "native_super_types",
                    "must be strictly sorted by case-sensitive AngelScript type name",
                );
            }
            previous_native = Some(&native.angelscript_type_name);
        }
        validate_fname_comparison_keys(&self.fname_comparison_keys)?;
        validate_external_frontend_hooks(&self.external_hooks)?;
        Ok(())
    }

    fn computed_digest(&self) -> Result<Sha256Digest, FrontendProfileError> {
        let mut canonical = self.clone();
        canonical.canonical_sha256 = zero_digest();
        canonical_digest(PREPROCESSOR_HASH_DOMAIN, &canonical)
    }
}

/// The only `UAngelscriptSettings` value read directly by the class generator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClassGeneratorConfigV1 {
    pub schema: String,
    pub schema_version: u32,
    pub mark_non_uproperty_properties_as_transient: bool,
    pub canonical_sha256: Sha256Digest,
}

impl ClassGeneratorConfigV1 {
    pub fn seal(&mut self) -> Result<(), FrontendProfileError> {
        self.validate_structure()?;
        self.canonical_sha256 = self.computed_digest()?;
        Ok(())
    }

    pub fn validate(&self) -> Result<(), FrontendProfileError> {
        self.validate_structure()?;
        check_digest(
            "class generator config",
            self.canonical_sha256,
            self.computed_digest()?,
        )
    }

    pub fn from_json(bytes: &[u8]) -> Result<Self, FrontendProfileError> {
        parse_bounded(
            bytes,
            MAX_CLASS_GENERATOR_CONFIG_BYTES_V1,
            "class generator config",
        )
    }

    pub fn to_json(&self) -> Result<Vec<u8>, FrontendProfileError> {
        self.validate()?;
        Ok(serde_json::to_vec_pretty(self)?)
    }

    fn validate_structure(&self) -> Result<(), FrontendProfileError> {
        check_schema(
            &self.schema,
            self.schema_version,
            CLASS_GENERATOR_CONFIG_SCHEMA,
        )
    }

    fn computed_digest(&self) -> Result<Sha256Digest, FrontendProfileError> {
        let mut canonical = self.clone();
        canonical.canonical_sha256 = zero_digest();
        canonical_digest(CLASS_GENERATOR_HASH_DOMAIN, &canonical)
    }
}

/// Non-engine-property options read directly by the modified builder/compiler translation units.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompilerOptionsV1 {
    pub schema: String,
    pub schema_version: u32,
    pub error_on_incorrect_editor_only_code: bool,
    pub warn_on_divergent_comparison_operator_overloads: bool,
    pub warn_on_implicit_signed_unsigned_conversion: bool,
    pub warn_on_increment_decrement_in_complex_expression: bool,
    pub warn_on_unused_return_value_for_const_methods: bool,
    pub canonical_sha256: Sha256Digest,
}

impl CompilerOptionsV1 {
    pub fn seal(&mut self) -> Result<(), FrontendProfileError> {
        self.validate_structure()?;
        self.canonical_sha256 = self.computed_digest()?;
        Ok(())
    }

    pub fn validate(&self) -> Result<(), FrontendProfileError> {
        self.validate_structure()?;
        check_digest(
            "compiler options",
            self.canonical_sha256,
            self.computed_digest()?,
        )
    }

    pub fn from_json(bytes: &[u8]) -> Result<Self, FrontendProfileError> {
        parse_bounded(bytes, MAX_COMPILER_OPTIONS_BYTES_V1, "compiler options")
    }

    pub fn to_json(&self) -> Result<Vec<u8>, FrontendProfileError> {
        self.validate()?;
        Ok(serde_json::to_vec_pretty(self)?)
    }

    fn validate_structure(&self) -> Result<(), FrontendProfileError> {
        check_schema(&self.schema, self.schema_version, COMPILER_OPTIONS_SCHEMA)
    }

    fn computed_digest(&self) -> Result<Sha256Digest, FrontendProfileError> {
        let mut canonical = self.clone();
        canonical.canonical_sha256 = zero_digest();
        canonical_digest(COMPILER_OPTIONS_HASH_DOMAIN, &canonical)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedFrontendProfileV1 {
    pub preprocessor_config: PreprocessorConfigV1,
    pub class_generator_config: ClassGeneratorConfigV1,
    pub compiler_options: CompilerOptionsV1,
}

pub fn validate_frontend_profile_payloads(
    manifest: &FrontendProfileV1,
    preprocessor_json: &[u8],
    class_generator_json: &[u8],
    compiler_options_json: &[u8],
) -> Result<ValidatedFrontendProfileV1, FrontendProfileError> {
    check_blob(
        &manifest.preprocessor_config,
        preprocessor_json,
        "preprocessor config",
    )?;
    check_blob(
        &manifest.class_generator_config,
        class_generator_json,
        "class generator config",
    )?;
    check_blob(
        &manifest.compiler_options,
        compiler_options_json,
        "compiler options",
    )?;
    Ok(ValidatedFrontendProfileV1 {
        preprocessor_config: PreprocessorConfigV1::from_json(preprocessor_json)?,
        class_generator_config: ClassGeneratorConfigV1::from_json(class_generator_json)?,
        compiler_options: CompilerOptionsV1::from_json(compiler_options_json)?,
    })
}

fn validate_flag_name(value: &str) -> Result<(), FrontendProfileError> {
    if value.is_empty()
        || value.len() > MAX_FLAG_BYTES
        || value.contains('\0')
        || value.chars().any(char::is_control)
    {
        return invalid(
            "effective_flags.name",
            "must be nonempty, bounded UTF-8 without control characters",
        );
    }
    Ok(())
}

fn validate_sorted_text_set(
    values: &[String],
    max_count: usize,
    max_bytes: usize,
    field: &'static str,
) -> Result<(), FrontendProfileError> {
    if values.len() > max_count {
        return Err(FrontendProfileError::CountTooLarge {
            field,
            actual: values.len(),
            max: max_count,
        });
    }
    let mut previous: Option<&str> = None;
    for value in values {
        if value.is_empty()
            || value.len() > max_bytes
            || value.contains('\0')
            || value.chars().any(char::is_control)
        {
            return invalid(
                field,
                "must contain bounded nonempty text without control characters",
            );
        }
        if previous.is_some_and(|prior| prior >= value.as_str()) {
            return invalid(field, "must be strictly sorted and duplicate-free");
        }
        previous = Some(value);
    }
    Ok(())
}

fn validate_profile_text(
    value: &str,
    max_bytes: usize,
    field: &'static str,
) -> Result<(), FrontendProfileError> {
    if value.is_empty()
        || value.len() > max_bytes
        || value.contains('\0')
        || value.chars().any(char::is_control)
    {
        return invalid(
            field,
            "must be bounded nonempty text without control characters",
        );
    }
    Ok(())
}

fn validate_bounded_nul_free(
    value: &str,
    max_bytes: usize,
    field: &'static str,
    allow_empty: bool,
) -> Result<(), FrontendProfileError> {
    if (!allow_empty && value.is_empty()) || value.len() > max_bytes || value.contains('\0') {
        return invalid(field, "must be bounded UTF-8 without NUL bytes");
    }
    Ok(())
}

fn validate_fname_comparison_keys(
    values: &[FNameComparisonKeyV1],
) -> Result<(), FrontendProfileError> {
    if values.len() > MAX_FNAME_COMPARISON_KEYS {
        return Err(FrontendProfileError::CountTooLarge {
            field: "fname_comparison_keys",
            actual: values.len(),
            max: MAX_FNAME_COMPARISON_KEYS,
        });
    }
    let mut previous: Option<&str> = None;
    for (index, value) in values.iter().enumerate() {
        if value.ordinal as usize != index {
            return Err(FrontendProfileError::Order {
                field: "FName comparison key",
                expected: index,
                actual: value.ordinal,
            });
        }
        validate_bounded_nul_free(
            &value.spelling,
            MAX_EXTERNAL_NAME_BYTES,
            "fname_comparison_keys.spelling",
            false,
        )?;
        validate_bounded_nul_free(
            &value.comparison_key,
            MAX_EXTERNAL_NAME_BYTES,
            "fname_comparison_keys.comparison_key",
            false,
        )?;
        if previous.is_some_and(|prior| prior >= value.spelling.as_str()) {
            return invalid(
                "fname_comparison_keys",
                "must be strictly sorted by spelling and duplicate-free",
            );
        }
        previous = Some(&value.spelling);
    }
    Ok(())
}

fn validate_external_frontend_hooks(
    hooks: &ExternalFrontendHooksV1,
) -> Result<(), FrontendProfileError> {
    validate_class_analyze_hook(&hooks.class_analyze)?;
    validate_graph_hook("process_chunks", &hooks.process_chunks)?;
    validate_graph_hook("post_process_code", &hooks.post_process_code)
}

fn validate_class_analyze_hook(
    hook: &ClassAnalyzeHookProfileV1,
) -> Result<(), FrontendProfileError> {
    if !hook.bound && !hook.captures.is_empty() {
        return invalid(
            "external_hooks.class_analyze",
            "unbound hook must not carry captures",
        );
    }
    if hook.captures.len() > MAX_EXTERNAL_CLASS_CAPTURES {
        return Err(FrontendProfileError::CountTooLarge {
            field: "external_hooks.class_analyze.captures",
            actual: hook.captures.len(),
            max: MAX_EXTERNAL_CLASS_CAPTURES,
        });
    }
    let mut identities = BTreeSet::new();
    let mut generated_bytes = 0usize;
    for (index, capture) in hook.captures.iter().enumerate() {
        if capture.ordinal as usize != index {
            return Err(FrontendProfileError::Order {
                field: "ClassAnalyze capture",
                expected: index,
                actual: capture.ordinal,
            });
        }
        validate_bounded_nul_free(
            &capture.module_name,
            MAX_EXTERNAL_NAME_BYTES,
            "external_hooks.class_analyze.module_name",
            false,
        )?;
        validate_bounded_nul_free(
            &capture.namespace,
            MAX_EXTERNAL_NAME_BYTES,
            "external_hooks.class_analyze.namespace",
            true,
        )?;
        validate_bounded_nul_free(
            &capture.class_name,
            MAX_EXTERNAL_NAME_BYTES,
            "external_hooks.class_analyze.class_name",
            false,
        )?;
        validate_bounded_nul_free(
            &capture.compose_onto_class,
            MAX_EXTERNAL_NAME_BYTES,
            "external_hooks.class_analyze.compose_onto_class",
            true,
        )?;
        validate_bounded_nul_free(
            &capture.generated_statics,
            MAX_EXTERNAL_GENERATED_BYTES,
            "external_hooks.class_analyze.generated_statics",
            true,
        )?;
        let next_generated_bytes = generated_bytes.saturating_add(capture.generated_statics.len());
        if next_generated_bytes > MAX_EXTERNAL_GENERATED_BYTES {
            return Err(FrontendProfileError::CountTooLarge {
                field: "external_hooks.class_analyze generated bytes",
                actual: next_generated_bytes,
                max: MAX_EXTERNAL_GENERATED_BYTES,
            });
        }
        generated_bytes = next_generated_bytes;
        let actual_output =
            Sha256Digest::from_bytes(Sha256::digest(capture.generated_statics.as_bytes()).into());
        check_digest(
            "external_hooks.class_analyze.output_generated_statics_sha256",
            capture.output_generated_statics_sha256,
            actual_output,
        )?;
        if !identities.insert((
            capture.module_name.as_str(),
            capture.namespace.as_str(),
            capture.class_name.as_str(),
        )) {
            return invalid(
                "external_hooks.class_analyze.captures",
                "contains a duplicate module/namespace/class identity",
            );
        }
    }
    Ok(())
}

fn validate_graph_hook(
    field: &'static str,
    hook: &GraphHookProfileV1,
) -> Result<(), FrontendProfileError> {
    if !hook.bound && !hook.captures.is_empty() {
        return invalid(field, "unbound hook must not carry captures");
    }
    if hook.captures.len() > MAX_EXTERNAL_GRAPH_CAPTURES {
        return Err(FrontendProfileError::CountTooLarge {
            field,
            actual: hook.captures.len(),
            max: MAX_EXTERNAL_GRAPH_CAPTURES,
        });
    }
    let mut inputs = BTreeSet::new();
    for (index, capture) in hook.captures.iter().enumerate() {
        if capture.ordinal as usize != index {
            return Err(FrontendProfileError::Order {
                field,
                expected: index,
                actual: capture.ordinal,
            });
        }
        if !inputs.insert(capture.input_graph_sha256) {
            return invalid(field, "contains a duplicate input graph digest");
        }
        if capture.modules.len() > MAX_EXTERNAL_GRAPH_MODULES {
            return Err(FrontendProfileError::CountTooLarge {
                field: "external hook modules",
                actual: capture.modules.len(),
                max: MAX_EXTERNAL_GRAPH_MODULES,
            });
        }
        let mut module_names = BTreeSet::new();
        let mut generated_bytes = 0usize;
        for (module_index, module) in capture.modules.iter().enumerate() {
            if module.ordinal as usize != module_index {
                return Err(FrontendProfileError::Order {
                    field: "external hook module",
                    expected: module_index,
                    actual: module.ordinal,
                });
            }
            validate_bounded_nul_free(
                &module.module_name,
                MAX_EXTERNAL_NAME_BYTES,
                "external hook module_name",
                false,
            )?;
            validate_bounded_nul_free(
                &module.generated_declarations,
                MAX_EXTERNAL_GENERATED_BYTES,
                "external hook generated_declarations",
                true,
            )?;
            let next_generated_bytes =
                generated_bytes.saturating_add(module.generated_declarations.len());
            if next_generated_bytes > MAX_EXTERNAL_GENERATED_BYTES {
                return Err(FrontendProfileError::CountTooLarge {
                    field: "external hook generated bytes",
                    actual: next_generated_bytes,
                    max: MAX_EXTERNAL_GENERATED_BYTES,
                });
            }
            generated_bytes = next_generated_bytes;
            if !module_names.insert(module.module_name.as_str()) {
                return invalid(field, "contains a duplicate module name");
            }
        }
        check_digest(
            "external hook output_graph_sha256",
            capture.output_graph_sha256,
            graph_hook_output_digest(capture),
        )?;
    }
    Ok(())
}

fn graph_hook_output_digest(capture: &GraphHookCaptureV1) -> Sha256Digest {
    let mut hash = Sha256::new();
    hash.update(EXTERNAL_GRAPH_OUTPUT_HASH_DOMAIN);
    hash.update(capture.input_graph_sha256.as_bytes());
    hash.update((capture.modules.len() as u64).to_le_bytes());
    for module in &capture.modules {
        hash.update((module.module_name.len() as u64).to_le_bytes());
        hash.update(module.module_name.as_bytes());
        hash.update((module.generated_declarations.len() as u64).to_le_bytes());
        hash.update(module.generated_declarations.as_bytes());
    }
    Sha256Digest::from_bytes(hash.finalize().into())
}

fn check_schema(
    actual: &str,
    version: u32,
    expected: &'static str,
) -> Result<(), FrontendProfileError> {
    if actual != expected || version != FRONTEND_SCHEMA_VERSION {
        return Err(FrontendProfileError::Schema {
            expected: format!("{expected}/v{FRONTEND_SCHEMA_VERSION}"),
            actual: format!("{actual}/v{version}"),
        });
    }
    Ok(())
}

fn zero_digest() -> Sha256Digest {
    Sha256Digest::from_bytes([0; 32])
}

fn canonical_digest<T: Serialize>(
    domain: &[u8],
    value: &T,
) -> Result<Sha256Digest, FrontendProfileError> {
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
) -> Result<(), FrontendProfileError> {
    if expected == actual {
        Ok(())
    } else {
        Err(FrontendProfileError::DigestMismatch {
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
) -> Result<T, FrontendProfileError>
where
    T: for<'de> Deserialize<'de> + FrontendValidate,
{
    if bytes.len() > max {
        return Err(FrontendProfileError::InputTooLarge {
            label,
            actual: bytes.len(),
            max,
        });
    }
    let value: T = serde_json::from_slice(bytes)?;
    value.frontend_validate()?;
    Ok(value)
}

trait FrontendValidate {
    fn frontend_validate(&self) -> Result<(), FrontendProfileError>;
}

impl FrontendValidate for PreprocessorConfigV1 {
    fn frontend_validate(&self) -> Result<(), FrontendProfileError> {
        self.validate()
    }
}

impl FrontendValidate for ClassGeneratorConfigV1 {
    fn frontend_validate(&self) -> Result<(), FrontendProfileError> {
        self.validate()
    }
}

impl FrontendValidate for CompilerOptionsV1 {
    fn frontend_validate(&self) -> Result<(), FrontendProfileError> {
        self.validate()
    }
}

fn check_blob(
    seal: &SealedBlobV1,
    bytes: &[u8],
    label: &'static str,
) -> Result<(), FrontendProfileError> {
    if seal.byte_len != bytes.len() as u64 {
        return Err(FrontendProfileError::BlobSealMismatch {
            label,
            reason: "byte length",
        });
    }
    let actual = Sha256Digest::from_bytes(Sha256::digest(bytes).into());
    if seal.sha256 != actual {
        return Err(FrontendProfileError::BlobSealMismatch {
            label,
            reason: "sha256",
        });
    }
    Ok(())
}

fn invalid<T>(field: &'static str, reason: &'static str) -> Result<T, FrontendProfileError> {
    Err(FrontendProfileError::InvalidField { field, reason })
}

#[derive(Debug, thiserror::Error)]
pub enum FrontendProfileError {
    #[error("{label} JSON is {actual} bytes; maximum accepted size is {max}")]
    InputTooLarge {
        label: &'static str,
        actual: usize,
        max: usize,
    },
    #[error("frontend schema mismatch: expected {expected}, got {actual}")]
    Schema { expected: String, actual: String },
    #[error("{field} count {actual} exceeds maximum {max}")]
    CountTooLarge {
        field: &'static str,
        actual: usize,
        max: usize,
    },
    #[error("{field} is invalid: {reason}")]
    InvalidField {
        field: &'static str,
        reason: &'static str,
    },
    #[error("{field} is out of order: expected {expected}, got {actual}")]
    Order {
        field: &'static str,
        expected: usize,
        actual: u32,
    },
    #[error("effective preprocessor flags are missing donor built-in {0}")]
    MissingBuiltinFlag(&'static str),
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
    #[error("invalid frontend JSON: {0}")]
    Json(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flags() -> Vec<EffectivePreprocessorFlagV1> {
        BUILTIN_PREPROCESSOR_FLAGS
            .into_iter()
            .enumerate()
            .map(|(ordinal, name)| EffectivePreprocessorFlagV1 {
                ordinal: ordinal as u32,
                name: name.to_owned(),
                value: matches!(name, "RELEASE" | "WITH_SERVER_CODE"),
            })
            .collect()
    }

    fn preprocessor() -> PreprocessorConfigV1 {
        let mut value = PreprocessorConfigV1 {
            schema: PREPROCESSOR_CONFIG_SCHEMA.to_owned(),
            schema_version: FRONTEND_SCHEMA_VERSION,
            automatic_imports: true,
            warn_on_manual_import_statements: true,
            use_editor_scripts: false,
            effective_flags: flags(),
            default_function_blueprint_callable: true,
            default_property_edit_specifier: PropertyEditSpecifierV1::EditAnywhere,
            default_property_edit_specifier_for_structs: PropertyEditSpecifierV1::EditAnywhere,
            default_property_blueprint_specifier: PropertyBlueprintSpecifierV1::BlueprintReadWrite,
            static_class_mode: StaticClassModeV1::Allowed,
            script_float_is_float64: true,
            angelscript_haze: false,
            enforce_server_rpc_validation: false,
            blueprint_event_argument_specializations: vec!["FName".to_owned(), "int32".to_owned()],
            native_super_types: vec![
                NativeSuperTypeV1 {
                    ordinal: 0,
                    angelscript_type_name: "AActor".to_owned(),
                    unreal_class_path: "/Script/Engine.Actor".to_owned(),
                    property_offset: 0,
                    kind: NativeSuperKindV1::Actor,
                    cannot_derive_angelscript: false,
                },
                NativeSuperTypeV1 {
                    ordinal: 1,
                    angelscript_type_name: "UObject".to_owned(),
                    unreal_class_path: "/Script/CoreUObject.Object".to_owned(),
                    property_offset: 0,
                    kind: NativeSuperKindV1::OtherUObject,
                    cannot_derive_angelscript: false,
                },
            ],
            fname_comparison_keys: Vec::new(),
            external_hooks: ExternalFrontendHooksV1::unbound(),
            canonical_sha256: zero_digest(),
        };
        value.seal().unwrap();
        value
    }

    fn class_generator() -> ClassGeneratorConfigV1 {
        let mut value = ClassGeneratorConfigV1 {
            schema: CLASS_GENERATOR_CONFIG_SCHEMA.to_owned(),
            schema_version: FRONTEND_SCHEMA_VERSION,
            mark_non_uproperty_properties_as_transient: false,
            canonical_sha256: zero_digest(),
        };
        value.seal().unwrap();
        value
    }

    fn compiler_options() -> CompilerOptionsV1 {
        let mut value = CompilerOptionsV1 {
            schema: COMPILER_OPTIONS_SCHEMA.to_owned(),
            schema_version: FRONTEND_SCHEMA_VERSION,
            error_on_incorrect_editor_only_code: true,
            warn_on_divergent_comparison_operator_overloads: true,
            warn_on_implicit_signed_unsigned_conversion: true,
            warn_on_increment_decrement_in_complex_expression: true,
            warn_on_unused_return_value_for_const_methods: true,
            canonical_sha256: zero_digest(),
        };
        value.seal().unwrap();
        value
    }

    fn blob(path: &str, bytes: &[u8]) -> SealedBlobV1 {
        SealedBlobV1 {
            path: path.to_owned(),
            byte_len: bytes.len() as u64,
            sha256: Sha256Digest::from_bytes(Sha256::digest(bytes).into()),
        }
    }

    #[test]
    fn typed_frontend_payloads_round_trip_and_bind_to_manifest() {
        let preprocessor_json = preprocessor().to_json().unwrap();
        let class_generator_json = class_generator().to_json().unwrap();
        let compiler_options_json = compiler_options().to_json().unwrap();
        let manifest = FrontendProfileV1 {
            preprocessor_config: blob("frontend/preprocessor.json", &preprocessor_json),
            class_generator_config: blob("frontend/class-generator.json", &class_generator_json),
            compiler_options: blob("frontend/compiler-options.json", &compiler_options_json),
        };

        let validated = validate_frontend_profile_payloads(
            &manifest,
            &preprocessor_json,
            &class_generator_json,
            &compiler_options_json,
        )
        .unwrap();
        assert_eq!(validated.preprocessor_config, preprocessor());
        assert_eq!(validated.class_generator_config, class_generator());
        assert_eq!(validated.compiler_options, compiler_options());
    }

    #[test]
    fn frontend_payloads_reject_shape_order_and_seal_drift() {
        let mut value = serde_json::to_value(preprocessor()).unwrap();
        value["unexpected"] = serde_json::json!(true);
        assert!(matches!(
            PreprocessorConfigV1::from_json(&serde_json::to_vec(&value).unwrap()),
            Err(FrontendProfileError::Json(_))
        ));

        let mut missing = preprocessor();
        missing.effective_flags.remove(0);
        for (ordinal, flag) in missing.effective_flags.iter_mut().enumerate() {
            flag.ordinal = ordinal as u32;
        }
        assert!(matches!(
            missing.seal(),
            Err(FrontendProfileError::MissingBuiltinFlag("COOK_COMMANDLET"))
        ));

        let mut unsorted = preprocessor();
        unsorted.effective_flags.swap(0, 1);
        for (ordinal, flag) in unsorted.effective_flags.iter_mut().enumerate() {
            flag.ordinal = ordinal as u32;
        }
        assert!(matches!(
            unsorted.seal(),
            Err(FrontendProfileError::InvalidField {
                field: "effective_flags",
                ..
            })
        ));

        let mut unsorted_specializations = preprocessor();
        unsorted_specializations
            .blueprint_event_argument_specializations
            .swap(0, 1);
        assert!(matches!(
            unsorted_specializations.seal(),
            Err(FrontendProfileError::InvalidField {
                field: "blueprint_event_argument_specializations",
                ..
            })
        ));

        let mut duplicated_native = preprocessor();
        duplicated_native.native_super_types[1].angelscript_type_name = duplicated_native
            .native_super_types[0]
            .angelscript_type_name
            .clone();
        assert!(matches!(
            duplicated_native.seal(),
            Err(FrontendProfileError::InvalidField {
                field: "native_super_types",
                ..
            })
        ));

        let mut duplicated_native_path = preprocessor();
        duplicated_native_path.native_super_types[1].unreal_class_path =
            duplicated_native_path.native_super_types[0]
                .unreal_class_path
                .clone();
        assert!(matches!(
            duplicated_native_path.seal(),
            Err(FrontendProfileError::InvalidField {
                field: "native_super_types",
                ..
            })
        ));

        let mut oversized_property_offset = preprocessor();
        oversized_property_offset.native_super_types[0].property_offset =
            i32::MAX as u64 + 1;
        assert!(matches!(
            oversized_property_offset.seal(),
            Err(FrontendProfileError::InvalidField {
                field: "native_super_types.property_offset",
                ..
            })
        ));

        let preprocessor_json = preprocessor().to_json().unwrap();
        let class_generator_json = class_generator().to_json().unwrap();
        let compiler_options_json = compiler_options().to_json().unwrap();
        let mut manifest = FrontendProfileV1 {
            preprocessor_config: blob("frontend/preprocessor.json", &preprocessor_json),
            class_generator_config: blob("frontend/class-generator.json", &class_generator_json),
            compiler_options: blob("frontend/compiler-options.json", &compiler_options_json),
        };
        manifest.compiler_options.byte_len += 1;
        assert!(matches!(
            validate_frontend_profile_payloads(
                &manifest,
                &preprocessor_json,
                &class_generator_json,
                &compiler_options_json,
            ),
            Err(FrontendProfileError::BlobSealMismatch {
                label: "compiler options",
                reason: "byte length"
            })
        ));
    }

    #[test]
    fn external_frontend_semantics_are_exact_bounded_and_fail_closed() {
        let generated_statics = "int GeneratedValue = 7;\n".to_owned();
        let graph_input = Sha256Digest::from_bytes([0x31; 32]);
        let mut graph_capture = GraphHookCaptureV1 {
            ordinal: 0,
            input_graph_sha256: graph_input,
            output_graph_sha256: zero_digest(),
            modules: vec![GraphHookModuleCaptureV1 {
                ordinal: 0,
                module_name: "GoreMods.Example".to_owned(),
                generated_declarations: "void GeneratedHook();\n".to_owned(),
            }],
        };
        graph_capture.output_graph_sha256 = graph_hook_output_digest(&graph_capture);

        let mut value = preprocessor();
        value.fname_comparison_keys = vec![FNameComparisonKeyV1 {
            ordinal: 0,
            spelling: "Äpfel".to_owned(),
            comparison_key: "äpfel".to_owned(),
        }];
        value.external_hooks = ExternalFrontendHooksV1 {
            class_analyze: ClassAnalyzeHookProfileV1 {
                bound: true,
                captures: vec![ClassAnalyzeCaptureV1 {
                    ordinal: 0,
                    module_name: "GoreMods.Example".to_owned(),
                    namespace: "Example".to_owned(),
                    class_name: "UExample".to_owned(),
                    source_sha256: Sha256Digest::from_bytes([0x11; 32]),
                    input_generated_statics_sha256: Sha256Digest::from_bytes([0x22; 32]),
                    output_generated_statics_sha256: Sha256Digest::from_bytes(
                        Sha256::digest(generated_statics.as_bytes()).into(),
                    ),
                    generated_statics,
                    has_statics: true,
                    compose_onto_class: "UBaseExample".to_owned(),
                }],
            },
            process_chunks: GraphHookProfileV1 {
                bound: true,
                captures: vec![graph_capture.clone()],
            },
            post_process_code: GraphHookProfileV1 {
                bound: true,
                captures: vec![graph_capture],
            },
        };
        value.seal().unwrap();
        assert_eq!(
            PreprocessorConfigV1::from_json(&value.to_json().unwrap()).unwrap(),
            value
        );

        let mut unbound_with_capture = value.clone();
        unbound_with_capture.external_hooks.class_analyze.bound = false;
        assert!(matches!(
            unbound_with_capture.seal(),
            Err(FrontendProfileError::InvalidField {
                field: "external_hooks.class_analyze",
                ..
            })
        ));

        let mut forged_generated_output = value.clone();
        forged_generated_output
            .external_hooks
            .class_analyze
            .captures[0]
            .output_generated_statics_sha256 = zero_digest();
        assert!(matches!(
            forged_generated_output.seal(),
            Err(FrontendProfileError::DigestMismatch {
                field: "external_hooks.class_analyze.output_generated_statics_sha256",
                ..
            })
        ));

        let mut forged_graph_output = value.clone();
        forged_graph_output.external_hooks.process_chunks.captures[0].output_graph_sha256 =
            zero_digest();
        assert!(matches!(
            forged_graph_output.seal(),
            Err(FrontendProfileError::DigestMismatch {
                field: "external hook output_graph_sha256",
                ..
            })
        ));

        let mut missing_hooks = serde_json::to_value(value).unwrap();
        missing_hooks
            .as_object_mut()
            .unwrap()
            .remove("external_hooks");
        assert!(matches!(
            PreprocessorConfigV1::from_json(&serde_json::to_vec(&missing_hooks).unwrap()),
            Err(FrontendProfileError::Json(_))
        ));
    }
}
