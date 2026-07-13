//! Deterministic, non-publishing story build plans.
//!
//! This crate deliberately stops before compilation, bundling, deployment, or runtime claims. It
//! accepts only byte-for-byte canonical schema-revision-2 project JSON through
//! [`gore_authoring::ProjectDocument::from_json`], regenerates exact story modules with the
//! authoring generators, and returns a sealed inspection plan. Runtime-unqualified drafts are
//! always blocking, including under the experimental validation profile.

pub mod revision3_quest;

#[cfg(test)]
mod revision3_quest_tests;

use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::fmt;
use std::io::{self, Write};
use std::marker::PhantomData;

use gore_authoring::{
    ContentSeal, Diagnostic, DiagnosticCode, DiagnosticSeverity, EntityId, ProjectDocument,
    ProjectDocumentError, ProjectId, ProjectRevision2, Revision2Entity as Entity,
    Revision2EntityKind as EntityKind, Revision2EntityPayload as EntityPayload,
    Revision2NpcDraft as NpcDraft, Revision2QuestDraft as QuestDraft,
    Revision2ScriptModule as ScriptModule, Revision2TypedRef as TypedRef, ScriptModuleStatus,
    Sha256Digest, ValidationProfile, MAX_PROJECT_JSON_BYTES,
};
use serde::de::{self, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest as _, Sha256};

const STORY_BUILD_PLAN_FORMAT: &str = "story_build_plan";
const STORY_BUILD_PLAN_SCHEMA_REVISION: u32 = 1;
const MAX_STORY_BUILD_MARKER_BYTES: usize = 64;

/// Hard limit inherited by the raw project boundary.
pub const MAX_STORY_BUILD_PROJECT_JSON_BYTES: usize = MAX_PROJECT_JSON_BYTES;
/// A plan repeats generated source and diagnostics, so it has a separate, still-bounded envelope.
pub const MAX_STORY_BUILD_PLAN_JSON_BYTES: usize = 32 * 1024 * 1024;
/// Independent defense against projects made from an excessive number of tiny story entities.
pub const MAX_STORY_BUILD_MODULES: usize = 4_096;
/// Aggregate UTF-8 source bytes admitted into one inspection plan.
pub const MAX_STORY_BUILD_SOURCE_BYTES: usize = 16 * 1024 * 1024;
/// Upper bound on validator output retained by one plan.
pub const MAX_STORY_BUILD_DIAGNOSTICS: usize = 65_536;
pub const MAX_STORY_BUILD_RELATED_ENTITIES_PER_DIAGNOSTIC: usize = 1_024;
pub const MAX_STORY_BUILD_RELATED_ENTITIES_TOTAL: usize = 65_536;
pub const MAX_STORY_BUILD_PROPERTY_PATH_BYTES: usize = 2 * 1_024;
pub const MAX_STORY_BUILD_DIAGNOSTIC_MESSAGE_BYTES: usize = 16 * 1_024;
pub const MAX_STORY_BUILD_GENERATOR_ID_BYTES: usize = 256;
pub const MAX_STORY_BUILD_MODULE_NAMESPACE_BYTES: usize = 512;
pub const MAX_STORY_BUILD_MODULE_RELATIVE_PATH_BYTES: usize = 2 * 1_024;
pub const MAX_STORY_BUILD_SEALED_INPUTS_PER_MODULE: usize = 16;
pub const MAX_STORY_BUILD_SEALED_INPUTS_TOTAL: usize =
    MAX_STORY_BUILD_MODULES * MAX_STORY_BUILD_SEALED_INPUTS_PER_MODULE;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct StoryBuildPlanFormat;

impl Serialize for StoryBuildPlanFormat {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(STORY_BUILD_PLAN_FORMAT)
    }
}

impl<'de> Deserialize<'de> for StoryBuildPlanFormat {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value =
            BoundedString::<MAX_STORY_BUILD_MARKER_BYTES>::deserialize(deserializer)?.into_inner();
        if value == STORY_BUILD_PLAN_FORMAT {
            Ok(Self)
        } else {
            Err(de::Error::custom(format!(
                "unsupported story build plan format {value:?}; expected {STORY_BUILD_PLAN_FORMAT:?}"
            )))
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct StoryBuildPlanSchemaRevision;

impl Serialize for StoryBuildPlanSchemaRevision {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u32(STORY_BUILD_PLAN_SCHEMA_REVISION)
    }
}

impl<'de> Deserialize<'de> for StoryBuildPlanSchemaRevision {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = u32::deserialize(deserializer)?;
        if value == STORY_BUILD_PLAN_SCHEMA_REVISION {
            Ok(Self)
        } else {
            Err(de::Error::custom(format!(
                "unsupported story build plan schema revision {value}; expected {STORY_BUILD_PLAN_SCHEMA_REVISION}"
            )))
        }
    }
}

/// Closed disposition of this revision. There is intentionally no publishable variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StoryBuildPublicationStatus {
    NotSupported,
}

/// Exact canonical project snapshot from which a plan was derived.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoryProjectProvenance {
    pub project_id: ProjectId,
    pub project_revision: u64,
    pub canonical_document: ContentSeal,
    pub target_executable: ContentSeal,
}

/// Exact project or entity property that contributed to a generated source.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(tag = "scope", rename_all = "snake_case", deny_unknown_fields)]
pub enum StoryPropertyProvenance {
    Project {
        project_id: ProjectId,
        project_revision: u64,
        property_path: String,
    },
    Entity {
        project_id: ProjectId,
        project_revision: u64,
        entity_id: EntityId,
        entity_revision: u64,
        entity_kind: EntityKind,
        property_path: String,
    },
}

/// One content seal together with the exact property that supplied it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SealedStoryProperty {
    pub provenance: StoryPropertyProvenance,
    pub content: ContentSeal,
}

/// One exact, persisted-and-regenerated story module in deterministic plan order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlannedScriptModule {
    /// Exact revision-2 reference from the owning draft to the persisted module entity.
    pub script_module: TypedRef,
    /// Canonical JSON seal of the draft's complete `payload.data.input` value.
    pub draft_input: SealedStoryProperty,
    /// Persisted `ScriptModule` source location and exact UTF-8 source seal.
    pub persisted_source: SealedStoryProperty,
    /// Every generation/catalog seal with its exact project or draft property path.
    pub sealed_inputs: Vec<SealedStoryProperty>,
    /// Exact deterministic regeneration result from `gore-authoring`.
    pub generated: ScriptModule,
}

#[derive(Debug)]
struct BoundedString<const LIMIT: usize>(String);

impl<const LIMIT: usize> BoundedString<LIMIT> {
    fn into_inner(self) -> String {
        self.0
    }
}

impl<'de, const LIMIT: usize> Deserialize<'de> for BoundedString<LIMIT> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct BoundedStringVisitor<const LIMIT: usize>;

        impl<const LIMIT: usize> Visitor<'_> for BoundedStringVisitor<LIMIT> {
            type Value = BoundedString<LIMIT>;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(formatter, "a UTF-8 string of at most {LIMIT} bytes")
            }

            fn visit_borrowed_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                bounded_string(value, LIMIT).map(|()| BoundedString(value.to_owned()))
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                bounded_string(value, LIMIT).map(|()| BoundedString(value.to_owned()))
            }

            fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                bounded_string(&value, LIMIT).map(|()| BoundedString(value))
            }
        }

        deserializer.deserialize_string(BoundedStringVisitor::<LIMIT>)
    }
}

fn bounded_string<E>(value: &str, limit: usize) -> Result<(), E>
where
    E: de::Error,
{
    if value.len() > limit {
        Err(E::custom(format!(
            "string is {} bytes; maximum is {limit}",
            value.len()
        )))
    } else {
        Ok(())
    }
}

#[derive(Debug)]
struct BoundedVec<T, const LIMIT: usize>(Vec<T>);

impl<T, const LIMIT: usize> Default for BoundedVec<T, LIMIT> {
    fn default() -> Self {
        Self(Vec::new())
    }
}

impl<'de, T, const LIMIT: usize> Deserialize<'de> for BoundedVec<T, LIMIT>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct BoundedVecVisitor<T, const LIMIT: usize>(PhantomData<T>);

        impl<'de, T, const LIMIT: usize> Visitor<'de> for BoundedVecVisitor<T, LIMIT>
        where
            T: Deserialize<'de>,
        {
            type Value = BoundedVec<T, LIMIT>;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(formatter, "a sequence of at most {LIMIT} items")
            }

            fn visit_seq<A>(self, mut access: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let hint = access.size_hint().unwrap_or(0);
                if hint > LIMIT {
                    return Err(de::Error::invalid_length(hint, &self));
                }
                let mut values = Vec::with_capacity(hint);
                loop {
                    if values.len() == LIMIT {
                        if access.next_element::<de::IgnoredAny>()?.is_some() {
                            return Err(de::Error::invalid_length(LIMIT + 1, &self));
                        }
                        break;
                    }
                    let Some(value) = access.next_element::<T>()? else {
                        break;
                    };
                    values.push(value);
                }
                Ok(BoundedVec(values))
            }
        }

        deserializer.deserialize_seq(BoundedVecVisitor::<T, LIMIT>(PhantomData))
    }
}

#[derive(Deserialize)]
#[serde(tag = "scope", rename_all = "snake_case", deny_unknown_fields)]
enum StoryPropertyProvenanceWire {
    Project {
        project_id: ProjectId,
        project_revision: u64,
        property_path: BoundedString<MAX_STORY_BUILD_PROPERTY_PATH_BYTES>,
    },
    Entity {
        project_id: ProjectId,
        project_revision: u64,
        entity_id: EntityId,
        entity_revision: u64,
        entity_kind: EntityKind,
        property_path: BoundedString<MAX_STORY_BUILD_PROPERTY_PATH_BYTES>,
    },
}

impl<'de> Deserialize<'de> for StoryPropertyProvenance {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(
            match StoryPropertyProvenanceWire::deserialize(deserializer)? {
                StoryPropertyProvenanceWire::Project {
                    project_id,
                    project_revision,
                    property_path,
                } => Self::Project {
                    project_id,
                    project_revision,
                    property_path: property_path.into_inner(),
                },
                StoryPropertyProvenanceWire::Entity {
                    project_id,
                    project_revision,
                    entity_id,
                    entity_revision,
                    entity_kind,
                    property_path,
                } => Self::Entity {
                    project_id,
                    project_revision,
                    entity_id,
                    entity_revision,
                    entity_kind,
                    property_path: property_path.into_inner(),
                },
            },
        )
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ScriptModuleWire {
    generator_id: BoundedString<MAX_STORY_BUILD_GENERATOR_ID_BYTES>,
    generator_version: u32,
    owner: TypedRef,
    module_namespace: BoundedString<MAX_STORY_BUILD_MODULE_NAMESPACE_BYTES>,
    module_relative_path: BoundedString<MAX_STORY_BUILD_MODULE_RELATIVE_PATH_BYTES>,
    source: BoundedString<MAX_STORY_BUILD_SOURCE_BYTES>,
    source_sha256: Sha256Digest,
    input_fingerprint: Sha256Digest,
    status: ScriptModuleStatus,
}

impl From<ScriptModuleWire> for ScriptModule {
    fn from(wire: ScriptModuleWire) -> Self {
        Self {
            generator_id: wire.generator_id.into_inner(),
            generator_version: wire.generator_version,
            owner: wire.owner,
            module_namespace: wire.module_namespace.into_inner(),
            module_relative_path: wire.module_relative_path.into_inner(),
            source: wire.source.into_inner(),
            source_sha256: wire.source_sha256,
            input_fingerprint: wire.input_fingerprint,
            status: wire.status,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PlannedScriptModuleWire {
    script_module: TypedRef,
    draft_input: SealedStoryProperty,
    persisted_source: SealedStoryProperty,
    sealed_inputs: BoundedVec<SealedStoryProperty, MAX_STORY_BUILD_SEALED_INPUTS_PER_MODULE>,
    generated: ScriptModuleWire,
}

impl<'de> Deserialize<'de> for PlannedScriptModule {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = PlannedScriptModuleWire::deserialize(deserializer)?;
        Ok(Self {
            script_module: wire.script_module,
            draft_input: wire.draft_input,
            persisted_source: wire.persisted_source,
            sealed_inputs: wire.sealed_inputs.0,
            generated: wire.generated.into(),
        })
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DiagnosticWire {
    code: DiagnosticCode,
    severity: DiagnosticSeverity,
    #[serde(default)]
    entity: Option<EntityId>,
    #[serde(default)]
    property_path: Option<BoundedString<MAX_STORY_BUILD_PROPERTY_PATH_BYTES>>,
    message: BoundedString<MAX_STORY_BUILD_DIAGNOSTIC_MESSAGE_BYTES>,
    #[serde(default)]
    related_entities: BoundedVec<EntityId, MAX_STORY_BUILD_RELATED_ENTITIES_PER_DIAGNOSTIC>,
    blocks_build: bool,
}

impl From<DiagnosticWire> for Diagnostic {
    fn from(wire: DiagnosticWire) -> Self {
        Self {
            code: wire.code,
            severity: wire.severity,
            entity: wire.entity,
            property_path: wire.property_path.map(BoundedString::into_inner),
            message: wire.message.into_inner(),
            related_entities: wire.related_entities.0,
            blocks_build: wire.blocks_build,
        }
    }
}

#[derive(Debug)]
struct BoundedModules(Vec<PlannedScriptModule>);

impl<'de> Deserialize<'de> for BoundedModules {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ModulesVisitor;

        impl<'de> Visitor<'de> for ModulesVisitor {
            type Value = BoundedModules;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(
                    formatter,
                    "at most {MAX_STORY_BUILD_MODULES} bounded story modules"
                )
            }

            fn visit_seq<A>(self, mut access: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let hint = access.size_hint().unwrap_or(0);
                if hint > MAX_STORY_BUILD_MODULES {
                    return Err(de::Error::invalid_length(hint, &self));
                }
                let mut modules = Vec::with_capacity(hint);
                let mut source_bytes = 0usize;
                let mut sealed_inputs = 0usize;
                loop {
                    if modules.len() == MAX_STORY_BUILD_MODULES {
                        if access.next_element::<de::IgnoredAny>()?.is_some() {
                            return Err(de::Error::invalid_length(
                                MAX_STORY_BUILD_MODULES + 1,
                                &self,
                            ));
                        }
                        break;
                    }
                    let Some(module) = access.next_element::<PlannedScriptModule>()? else {
                        break;
                    };
                    source_bytes = source_bytes
                        .checked_add(module.generated.source.len())
                        .ok_or_else(|| de::Error::custom("story source byte count overflowed"))?;
                    if source_bytes > MAX_STORY_BUILD_SOURCE_BYTES {
                        return Err(de::Error::custom(format!(
                            "story sources exceed the {MAX_STORY_BUILD_SOURCE_BYTES}-byte aggregate limit"
                        )));
                    }
                    sealed_inputs = sealed_inputs
                        .checked_add(module.sealed_inputs.len())
                        .ok_or_else(|| de::Error::custom("sealed input count overflowed"))?;
                    if sealed_inputs > MAX_STORY_BUILD_SEALED_INPUTS_TOTAL {
                        return Err(de::Error::custom(format!(
                            "sealed inputs exceed the {MAX_STORY_BUILD_SEALED_INPUTS_TOTAL}-item aggregate limit"
                        )));
                    }
                    modules.push(module);
                }
                Ok(BoundedModules(modules))
            }
        }

        deserializer.deserialize_seq(ModulesVisitor)
    }
}

#[derive(Debug)]
struct BoundedDiagnostics(Vec<Diagnostic>);

impl<'de> Deserialize<'de> for BoundedDiagnostics {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct DiagnosticsVisitor;

        impl<'de> Visitor<'de> for DiagnosticsVisitor {
            type Value = BoundedDiagnostics;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(
                    formatter,
                    "at most {MAX_STORY_BUILD_DIAGNOSTICS} bounded diagnostics"
                )
            }

            fn visit_seq<A>(self, mut access: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let hint = access.size_hint().unwrap_or(0);
                if hint > MAX_STORY_BUILD_DIAGNOSTICS {
                    return Err(de::Error::invalid_length(hint, &self));
                }
                let mut diagnostics = Vec::with_capacity(hint);
                let mut related_entities = 0usize;
                loop {
                    if diagnostics.len() == MAX_STORY_BUILD_DIAGNOSTICS {
                        if access.next_element::<de::IgnoredAny>()?.is_some() {
                            return Err(de::Error::invalid_length(
                                MAX_STORY_BUILD_DIAGNOSTICS + 1,
                                &self,
                            ));
                        }
                        break;
                    }
                    let Some(wire) = access.next_element::<DiagnosticWire>()? else {
                        break;
                    };
                    related_entities = related_entities
                        .checked_add(wire.related_entities.0.len())
                        .ok_or_else(|| de::Error::custom("related entity count overflowed"))?;
                    if related_entities > MAX_STORY_BUILD_RELATED_ENTITIES_TOTAL {
                        return Err(de::Error::custom(format!(
                            "related entities exceed the {MAX_STORY_BUILD_RELATED_ENTITIES_TOTAL}-item aggregate limit"
                        )));
                    }
                    diagnostics.push(wire.into());
                }
                Ok(BoundedDiagnostics(diagnostics))
            }
        }

        deserializer.deserialize_seq(DiagnosticsVisitor)
    }
}

/// A deterministic source inspection plan. It is not a compiler, bundle, or deployment artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StoryBuildPlan {
    #[serde(rename = "format")]
    format_marker: StoryBuildPlanFormat,
    schema_revision: StoryBuildPlanSchemaRevision,
    pub validation_profile: ValidationProfile,
    pub project: StoryProjectProvenance,
    pub publication_status: StoryBuildPublicationStatus,
    pub modules: Vec<PlannedScriptModule>,
    pub diagnostics: Vec<Diagnostic>,
    pub blocks_build: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StoryBuildPlanWire {
    #[serde(rename = "format")]
    format_marker: StoryBuildPlanFormat,
    schema_revision: StoryBuildPlanSchemaRevision,
    validation_profile: ValidationProfile,
    project: StoryProjectProvenance,
    publication_status: StoryBuildPublicationStatus,
    modules: BoundedModules,
    diagnostics: BoundedDiagnostics,
    blocks_build: bool,
}

impl From<StoryBuildPlanWire> for StoryBuildPlan {
    fn from(wire: StoryBuildPlanWire) -> Self {
        Self {
            format_marker: wire.format_marker,
            schema_revision: wire.schema_revision,
            validation_profile: wire.validation_profile,
            project: wire.project,
            publication_status: wire.publication_status,
            modules: wire.modules.0,
            diagnostics: wire.diagnostics.0,
            blocks_build: wire.blocks_build,
        }
    }
}

impl StoryBuildPlan {
    pub const fn format(&self) -> &'static str {
        STORY_BUILD_PLAN_FORMAT
    }

    pub const fn schema_revision(&self) -> u32 {
        STORY_BUILD_PLAN_SCHEMA_REVISION
    }

    /// Serialize the closed plan shape and enforce the output envelope.
    pub fn to_canonical_json(&self) -> Result<String, StoryBuildError> {
        self.validate_closed_invariants()?;
        let mut writer = BoundedJsonWriter::new(MAX_STORY_BUILD_PLAN_JSON_BYTES);
        if let Err(source) = serde_json::to_writer(&mut writer, self) {
            if let Some(actual) = writer.first_exceeded_size {
                return Err(StoryBuildError::PlanJsonTooLarge {
                    actual,
                    limit: MAX_STORY_BUILD_PLAN_JSON_BYTES,
                });
            }
            return Err(StoryBuildError::SerializePlan(source));
        }
        String::from_utf8(writer.bytes)
            .map_err(|_| StoryBuildError::Invariant("serializer emitted non-UTF-8 JSON".to_owned()))
    }

    /// Reopen only the exact canonical spelling produced by [`Self::to_canonical_json`].
    ///
    /// The closed wire structs reject duplicate fields; exact reserialization also rejects
    /// whitespace, reordered fields, alternate number spellings, and other non-canonical JSON.
    pub fn from_json(json: &str) -> Result<Self, StoryBuildError> {
        if json.len() > MAX_STORY_BUILD_PLAN_JSON_BYTES {
            return Err(StoryBuildError::PlanJsonTooLarge {
                actual: json.len(),
                limit: MAX_STORY_BUILD_PLAN_JSON_BYTES,
            });
        }
        let plan: Self = serde_json::from_str::<StoryBuildPlanWire>(json)
            .map(Self::from)
            .map_err(StoryBuildError::InvalidPlanJson)?;
        plan.validate_closed_invariants()?;
        let canonical = plan.to_canonical_json()?;
        if canonical.as_bytes() != json.as_bytes() {
            return Err(StoryBuildError::NonCanonicalPlanJson);
        }
        Ok(plan)
    }

    /// Seal the canonical plan bytes for external pinning. The seal is not embedded recursively.
    pub fn content_seal(&self) -> Result<ContentSeal, StoryBuildError> {
        self.to_canonical_json()
            .map(|json| seal_bytes(json.as_bytes()))
    }

    /// Re-plan from the exact source project and require full byte-level semantic equivalence.
    ///
    /// `from_json` proves the plan's closed shape and internal seals. This stronger check proves
    /// the project/catalog provenance too; callers should use it whenever the source project is
    /// available instead of treating an untrusted plan seal as authenticity evidence.
    pub fn verify_against_project_json(
        &self,
        canonical_project_json: &str,
    ) -> Result<(), StoryBuildError> {
        let expected = plan_story_build(canonical_project_json, self.validation_profile)?;
        if &expected != self {
            return Err(StoryBuildError::ProjectBindingMismatch);
        }
        Ok(())
    }

    fn validate_closed_invariants(&self) -> Result<(), StoryBuildError> {
        if self.project.canonical_document.byte_len == 0
            || self.project.canonical_document.byte_len
                > u64::try_from(MAX_STORY_BUILD_PROJECT_JSON_BYTES).unwrap_or(u64::MAX)
        {
            return invariant("canonical project document seal has an invalid byte length");
        }
        if self.modules.len() > MAX_STORY_BUILD_MODULES {
            return Err(StoryBuildError::TooManyModules {
                actual: self.modules.len(),
                limit: MAX_STORY_BUILD_MODULES,
            });
        }
        if self.diagnostics.len() > MAX_STORY_BUILD_DIAGNOSTICS {
            return Err(StoryBuildError::TooManyDiagnostics {
                actual: self.diagnostics.len(),
                limit: MAX_STORY_BUILD_DIAGNOSTICS,
            });
        }
        if self.publication_status != StoryBuildPublicationStatus::NotSupported {
            return invariant("story build plan publication must remain unsupported");
        }
        if !self.blocks_build {
            return invariant("story build plan must remain blocked");
        }

        let mut source_bytes = 0usize;
        let mut sealed_input_count = 0usize;
        for (index, module) in self.modules.iter().enumerate() {
            validate_module(self, module)?;
            source_bytes = source_bytes
                .checked_add(module.generated.source.len())
                .ok_or(StoryBuildError::SourceBytesOverflow)?;
            if source_bytes > MAX_STORY_BUILD_SOURCE_BYTES {
                return Err(StoryBuildError::SourceBytesTooLarge {
                    actual: source_bytes,
                    limit: MAX_STORY_BUILD_SOURCE_BYTES,
                });
            }
            sealed_input_count = sealed_input_count
                .checked_add(module.sealed_inputs.len())
                .ok_or_else(|| {
                    StoryBuildError::Invariant("sealed input count overflowed".to_owned())
                })?;
            if sealed_input_count > MAX_STORY_BUILD_SEALED_INPUTS_TOTAL {
                return invariant("sealed input aggregate exceeds its hard limit");
            }
            if index > 0 && compare_modules(&self.modules[index - 1], module) != Ordering::Less {
                return invariant("planned modules are not in strict canonical order");
            }
        }

        let mut related_entity_count = 0usize;
        for diagnostic in &self.diagnostics {
            validate_public_string(
                "diagnostic.message",
                &diagnostic.message,
                MAX_STORY_BUILD_DIAGNOSTIC_MESSAGE_BYTES,
            )?;
            if let Some(path) = &diagnostic.property_path {
                validate_public_string(
                    "diagnostic.property_path",
                    path,
                    MAX_STORY_BUILD_PROPERTY_PATH_BYTES,
                )?;
            }
            if diagnostic.related_entities.len() > MAX_STORY_BUILD_RELATED_ENTITIES_PER_DIAGNOSTIC {
                return invariant("diagnostic related entity list exceeds its hard limit");
            }
            related_entity_count = related_entity_count
                .checked_add(diagnostic.related_entities.len())
                .ok_or_else(|| {
                    StoryBuildError::Invariant("related entity count overflowed".to_owned())
                })?;
            if related_entity_count > MAX_STORY_BUILD_RELATED_ENTITIES_TOTAL {
                return invariant("related entity aggregate exceeds its hard limit");
            }
        }
        if self
            .diagnostics
            .windows(2)
            .any(|pair| compare_diagnostics(&pair[0], &pair[1]) != Ordering::Less)
        {
            return invariant("diagnostics are not in canonical order");
        }
        if self.diagnostics.iter().any(|diagnostic| {
            diagnostic
                .related_entities
                .windows(2)
                .any(|pair| pair[0] >= pair[1])
        }) {
            return invariant("diagnostic related entities are not strict and canonical");
        }
        if !self.diagnostics.iter().any(is_combined_validator_blocker) {
            return invariant("combined revision-2 validation blocker is missing");
        }
        let runtime_blocker_entities: BTreeSet<_> = self
            .diagnostics
            .iter()
            .filter(|diagnostic| {
                diagnostic.code == DiagnosticCode::RuntimeUnqualified
                    && diagnostic.severity == DiagnosticSeverity::Error
                    && diagnostic.blocks_build
            })
            .filter_map(|diagnostic| diagnostic.entity)
            .collect();
        for module in &self.modules {
            if !runtime_blocker_entities.contains(&module.generated.owner.id) {
                return invariant(format!(
                    "runtime-unqualified blocker is missing for story owner {}",
                    module.generated.owner.id
                ));
            }
        }
        Ok(())
    }
}

/// Parse one exact canonical Revision-2 project and produce a bounded source-only plan.
///
/// This function never writes files and exposes no compile, bundle, publish, or deployment path.
pub fn plan_story_build(
    canonical_project_json: &str,
    profile: ValidationProfile,
) -> Result<StoryBuildPlan, StoryBuildError> {
    if canonical_project_json.len() > MAX_STORY_BUILD_PROJECT_JSON_BYTES {
        return Err(StoryBuildError::ProjectJsonTooLarge {
            actual: canonical_project_json.len(),
            limit: MAX_STORY_BUILD_PROJECT_JSON_BYTES,
        });
    }

    // The authoring dispatch parser is the single duplicate-safe project parser.
    let document = ProjectDocument::from_json(canonical_project_json)
        .map_err(StoryBuildError::InvalidProjectDocument)?;
    let canonical = document
        .to_canonical_json()
        .map_err(StoryBuildError::SerializeProject)?;
    if canonical.as_bytes() != canonical_project_json.as_bytes() {
        return Err(StoryBuildError::NonCanonicalProjectJson);
    }
    let ProjectDocument::Revision2(project) = document else {
        return Err(StoryBuildError::Revision2Required);
    };

    build_plan(&project, canonical_project_json, profile)
}

fn build_plan(
    project: &ProjectRevision2,
    canonical_project_json: &str,
    profile: ValidationProfile,
) -> Result<StoryBuildPlan, StoryBuildError> {
    let mut modules = Vec::new();
    let mut source_bytes = 0usize;

    for (draft_id, entity) in &project.entities {
        let candidate = match &entity.payload {
            EntityPayload::NpcDraft(draft) => {
                planned_npc_module(project, *draft_id, entity, draft)?
            }
            EntityPayload::QuestDraft(draft) => {
                planned_quest_module(project, *draft_id, entity, draft)?
            }
            _ => None,
        };
        let Some(module) = candidate else {
            continue;
        };
        if modules.len() == MAX_STORY_BUILD_MODULES {
            return Err(StoryBuildError::TooManyModules {
                actual: modules.len() + 1,
                limit: MAX_STORY_BUILD_MODULES,
            });
        }
        source_bytes = source_bytes
            .checked_add(module.generated.source.len())
            .ok_or(StoryBuildError::SourceBytesOverflow)?;
        if source_bytes > MAX_STORY_BUILD_SOURCE_BYTES {
            return Err(StoryBuildError::SourceBytesTooLarge {
                actual: source_bytes,
                limit: MAX_STORY_BUILD_SOURCE_BYTES,
            });
        }
        modules.push(module);
    }
    modules.sort_by(compare_modules);

    let mut diagnostics = project.validate_story_entities_with_profile(profile);
    for diagnostic in &mut diagnostics {
        // A validation profile may affect editing UX, never build-plan qualification.
        if diagnostic.code == DiagnosticCode::RuntimeUnqualified {
            diagnostic.severity = DiagnosticSeverity::Error;
            diagnostic.blocks_build = true;
        }
    }
    diagnostics.push(combined_validator_blocker());
    sort_diagnostics(&mut diagnostics);
    diagnostics.dedup();
    if diagnostics.len() > MAX_STORY_BUILD_DIAGNOSTICS {
        return Err(StoryBuildError::TooManyDiagnostics {
            actual: diagnostics.len(),
            limit: MAX_STORY_BUILD_DIAGNOSTICS,
        });
    }

    ensure_omissions_have_causal_blockers(project, &modules, &diagnostics)?;

    let plan = StoryBuildPlan {
        format_marker: StoryBuildPlanFormat,
        schema_revision: StoryBuildPlanSchemaRevision,
        validation_profile: profile,
        project: StoryProjectProvenance {
            project_id: project.project_id,
            project_revision: project.revision,
            canonical_document: seal_bytes(canonical_project_json.as_bytes()),
            target_executable: project.target.executable.clone(),
        },
        publication_status: StoryBuildPublicationStatus::NotSupported,
        modules,
        diagnostics,
        blocks_build: true,
    };
    plan.validate_closed_invariants()?;
    let canonical_plan = plan.to_canonical_json()?;
    let reopened = StoryBuildPlan::from_json(&canonical_plan)?;
    if reopened != plan {
        return Err(StoryBuildError::CanonicalReopenMismatch);
    }
    Ok(plan)
}

fn planned_npc_module(
    project: &ProjectRevision2,
    draft_id: EntityId,
    draft_entity: &Entity,
    draft: &NpcDraft,
) -> Result<Option<PlannedScriptModule>, StoryBuildError> {
    let owner = TypedRef::new(project.project_id, draft_id, EntityKind::NpcDraft);
    let Ok(generated) = draft.regenerate_script_module(owner) else {
        return Ok(None);
    };
    exact_persisted_module(
        project,
        draft_entity,
        &draft.script_module,
        generated,
        serde_json::to_vec(&draft.input).map_err(StoryBuildError::SerializeDraftInput)?,
        vec![
            project_sealed_property(project, "target.executable", &project.target.executable),
            entity_sealed_property(
                project,
                draft_entity,
                "payload.data.input.parent_character_definition.generation.executable",
                &draft
                    .input
                    .parent_character_definition
                    .generation
                    .executable,
            ),
            entity_sealed_property(
                project,
                draft_entity,
                "payload.data.input.parent_character_definition.source_seal",
                &draft.input.parent_character_definition.source_seal,
            ),
            entity_sealed_property(
                project,
                draft_entity,
                "payload.data.input.parent_ai_agent_config.generation.executable",
                &draft.input.parent_ai_agent_config.generation.executable,
            ),
            entity_sealed_property(
                project,
                draft_entity,
                "payload.data.input.parent_ai_agent_config.source_seal",
                &draft.input.parent_ai_agent_config.source_seal,
            ),
            entity_sealed_property(
                project,
                draft_entity,
                "payload.data.input.parent_spawn_definition.generation.executable",
                &draft.input.parent_spawn_definition.generation.executable,
            ),
            entity_sealed_property(
                project,
                draft_entity,
                "payload.data.input.parent_spawn_definition.source_seal",
                &draft.input.parent_spawn_definition.source_seal,
            ),
        ],
    )
}

fn planned_quest_module(
    project: &ProjectRevision2,
    draft_id: EntityId,
    draft_entity: &Entity,
    draft: &QuestDraft,
) -> Result<Option<PlannedScriptModule>, StoryBuildError> {
    let owner = TypedRef::new(project.project_id, draft_id, EntityKind::QuestDraft);
    let Ok(generated) = draft.regenerate_script_module(owner) else {
        return Ok(None);
    };
    exact_persisted_module(
        project,
        draft_entity,
        &draft.script_module,
        generated,
        serde_json::to_vec(&draft.input).map_err(StoryBuildError::SerializeDraftInput)?,
        vec![
            project_sealed_property(project, "target.executable", &project.target.executable),
            entity_sealed_property(
                project,
                draft_entity,
                "payload.data.input.parent_quest.generation.executable",
                &draft.input.parent_quest.generation.executable,
            ),
            entity_sealed_property(
                project,
                draft_entity,
                "payload.data.input.parent_quest.source_seal",
                &draft.input.parent_quest.source_seal,
            ),
            entity_sealed_property(
                project,
                draft_entity,
                "payload.data.input.giver.generation.executable",
                &draft.input.giver.generation.executable,
            ),
            entity_sealed_property(
                project,
                draft_entity,
                "payload.data.input.giver.source_seal",
                &draft.input.giver.source_seal,
            ),
            entity_sealed_property(
                project,
                draft_entity,
                "payload.data.input.collision_catalog.generation.executable",
                &draft.input.collision_catalog.generation.executable,
            ),
            entity_sealed_property(
                project,
                draft_entity,
                "payload.data.input.collision_catalog.source_seal",
                &draft.input.collision_catalog.source_seal,
            ),
        ],
    )
}

fn exact_persisted_module(
    project: &ProjectRevision2,
    draft_entity: &Entity,
    module_ref: &TypedRef,
    generated: ScriptModule,
    draft_input_json: Vec<u8>,
    mut sealed_inputs: Vec<SealedStoryProperty>,
) -> Result<Option<PlannedScriptModule>, StoryBuildError> {
    if module_ref.project_id != project.project_id
        || module_ref.expected_kind != EntityKind::ScriptModule
    {
        return Ok(None);
    }
    let Some(module_entity) = project.entities.get(&module_ref.id) else {
        return Ok(None);
    };
    let EntityPayload::ScriptModule(persisted) = &module_entity.payload else {
        return Ok(None);
    };
    if persisted != &generated {
        return Ok(None);
    }
    if sealed_inputs.len() > MAX_STORY_BUILD_SEALED_INPUTS_PER_MODULE {
        return invariant("too many sealed provenance inputs for one module");
    }
    sealed_inputs.sort_by(|left, right| left.provenance.cmp(&right.provenance));

    let source_seal = seal_bytes(generated.source.as_bytes());
    if source_seal.sha256 != generated.source_sha256 {
        return invariant("regenerated source hash disagrees with its ScriptModule seal");
    }
    Ok(Some(PlannedScriptModule {
        script_module: module_ref.clone(),
        draft_input: SealedStoryProperty {
            provenance: entity_property(project, draft_entity, "payload.data.input"),
            content: seal_bytes(&draft_input_json),
        },
        persisted_source: SealedStoryProperty {
            provenance: entity_property(project, module_entity, "payload.data.source"),
            content: source_seal,
        },
        sealed_inputs,
        generated,
    }))
}

fn project_sealed_property(
    project: &ProjectRevision2,
    property_path: &str,
    content: &ContentSeal,
) -> SealedStoryProperty {
    SealedStoryProperty {
        provenance: StoryPropertyProvenance::Project {
            project_id: project.project_id,
            project_revision: project.revision,
            property_path: property_path.to_owned(),
        },
        content: content.clone(),
    }
}

fn entity_sealed_property(
    project: &ProjectRevision2,
    entity: &Entity,
    property_path: &str,
    content: &ContentSeal,
) -> SealedStoryProperty {
    SealedStoryProperty {
        provenance: entity_property(project, entity, property_path),
        content: content.clone(),
    }
}

fn entity_property(
    project: &ProjectRevision2,
    entity: &Entity,
    property_path: &str,
) -> StoryPropertyProvenance {
    StoryPropertyProvenance::Entity {
        project_id: project.project_id,
        project_revision: project.revision,
        entity_id: entity.id,
        entity_revision: entity.revision,
        entity_kind: entity.kind(),
        property_path: property_path.to_owned(),
    }
}

fn validate_module(
    plan: &StoryBuildPlan,
    module: &PlannedScriptModule,
) -> Result<(), StoryBuildError> {
    if module.sealed_inputs.len() > MAX_STORY_BUILD_SEALED_INPUTS_PER_MODULE {
        return invariant("too many sealed provenance inputs for one module");
    }
    validate_public_string(
        "generated.generator_id",
        &module.generated.generator_id,
        MAX_STORY_BUILD_GENERATOR_ID_BYTES,
    )?;
    validate_public_string(
        "generated.module_namespace",
        &module.generated.module_namespace,
        MAX_STORY_BUILD_MODULE_NAMESPACE_BYTES,
    )?;
    validate_public_string(
        "generated.module_relative_path",
        &module.generated.module_relative_path,
        MAX_STORY_BUILD_MODULE_RELATIVE_PATH_BYTES,
    )?;
    validate_public_string(
        "generated.source",
        &module.generated.source,
        MAX_STORY_BUILD_SOURCE_BYTES,
    )?;
    validate_provenance_path(&module.draft_input.provenance)?;
    validate_provenance_path(&module.persisted_source.provenance)?;
    for input in &module.sealed_inputs {
        validate_provenance_path(&input.provenance)?;
    }
    if module.generated.status
        != gore_authoring::ScriptModuleStatus::OFFLINE_DRAFT_RUNTIME_UNQUALIFIED
    {
        return invariant("planned module is not an offline, runtime-unqualified draft");
    }
    if module.generated.owner.project_id != plan.project.project_id
        || !matches!(
            module.generated.owner.expected_kind,
            EntityKind::NpcDraft | EntityKind::QuestDraft
        )
    {
        return invariant("planned module owner is not a local NPC or quest draft");
    }
    if module.script_module.project_id != plan.project.project_id
        || module.script_module.expected_kind != EntityKind::ScriptModule
    {
        return invariant("planned ScriptModule reference is not local and exact");
    }
    validate_draft_input_provenance(plan, module)?;
    validate_source_provenance(plan, module)?;

    let actual_source_seal = seal_bytes(module.generated.source.as_bytes());
    if module.persisted_source.content != actual_source_seal
        || module.generated.source_sha256 != actual_source_seal.sha256
    {
        return invariant("planned source bytes do not match their content seals");
    }
    if module.draft_input.content.byte_len == 0 {
        return invariant("draft input content seal must have a non-zero byte length");
    }

    let mut sorted_inputs = module.sealed_inputs.clone();
    sorted_inputs.sort_by(|left, right| left.provenance.cmp(&right.provenance));
    if sorted_inputs != module.sealed_inputs
        || sorted_inputs
            .windows(2)
            .any(|pair| pair[0].provenance == pair[1].provenance)
    {
        return invariant("sealed input provenance is not strict and canonical");
    }
    let expected_locations = expected_sealed_input_locations(plan, module)?;
    let actual_locations: Vec<_> = module
        .sealed_inputs
        .iter()
        .map(|input| input.provenance.clone())
        .collect();
    if actual_locations != expected_locations {
        return invariant("sealed input property provenance is incomplete or unexpected");
    }
    if module
        .sealed_inputs
        .iter()
        .any(|input| input.content.byte_len == 0)
    {
        return invariant("sealed generation inputs must have non-zero byte lengths");
    }
    Ok(())
}

fn validate_draft_input_provenance(
    plan: &StoryBuildPlan,
    module: &PlannedScriptModule,
) -> Result<(), StoryBuildError> {
    let StoryPropertyProvenance::Entity {
        project_id,
        project_revision,
        entity_id,
        entity_kind,
        property_path,
        ..
    } = &module.draft_input.provenance
    else {
        return invariant("draft input provenance must identify an entity property");
    };
    if *project_id != plan.project.project_id
        || *project_revision != plan.project.project_revision
        || *entity_id != module.generated.owner.id
        || *entity_kind != module.generated.owner.expected_kind
        || property_path != "payload.data.input"
    {
        return invariant("draft input provenance does not match the generated owner");
    }
    Ok(())
}

fn validate_source_provenance(
    plan: &StoryBuildPlan,
    module: &PlannedScriptModule,
) -> Result<(), StoryBuildError> {
    let StoryPropertyProvenance::Entity {
        project_id,
        project_revision,
        entity_id,
        entity_kind,
        property_path,
        ..
    } = &module.persisted_source.provenance
    else {
        return invariant("persisted source provenance must identify an entity property");
    };
    if *project_id != plan.project.project_id
        || *project_revision != plan.project.project_revision
        || *entity_id != module.script_module.id
        || *entity_kind != EntityKind::ScriptModule
        || property_path != "payload.data.source"
    {
        return invariant(
            "persisted source provenance does not identify a local ScriptModule source",
        );
    }
    Ok(())
}

fn expected_sealed_input_locations(
    plan: &StoryBuildPlan,
    module: &PlannedScriptModule,
) -> Result<Vec<StoryPropertyProvenance>, StoryBuildError> {
    let StoryPropertyProvenance::Entity {
        entity_revision, ..
    } = module.draft_input.provenance
    else {
        return invariant("draft input provenance must identify an entity");
    };
    let mut paths = match module.generated.owner.expected_kind {
        EntityKind::NpcDraft => vec![
            "payload.data.input.parent_character_definition.generation.executable",
            "payload.data.input.parent_character_definition.source_seal",
            "payload.data.input.parent_ai_agent_config.generation.executable",
            "payload.data.input.parent_ai_agent_config.source_seal",
            "payload.data.input.parent_spawn_definition.generation.executable",
            "payload.data.input.parent_spawn_definition.source_seal",
        ],
        EntityKind::QuestDraft => vec![
            "payload.data.input.parent_quest.generation.executable",
            "payload.data.input.parent_quest.source_seal",
            "payload.data.input.giver.generation.executable",
            "payload.data.input.giver.source_seal",
            "payload.data.input.collision_catalog.generation.executable",
            "payload.data.input.collision_catalog.source_seal",
        ],
        _ => return invariant("sealed inputs require an NPC or quest owner"),
    };
    let mut locations = vec![StoryPropertyProvenance::Project {
        project_id: plan.project.project_id,
        project_revision: plan.project.project_revision,
        property_path: "target.executable".to_owned(),
    }];
    locations.extend(
        paths
            .drain(..)
            .map(|property_path| StoryPropertyProvenance::Entity {
                project_id: plan.project.project_id,
                project_revision: plan.project.project_revision,
                entity_id: module.generated.owner.id,
                entity_revision,
                entity_kind: module.generated.owner.expected_kind,
                property_path: property_path.to_owned(),
            }),
    );
    locations.sort();
    Ok(locations)
}

fn compare_modules(left: &PlannedScriptModule, right: &PlannedScriptModule) -> Ordering {
    (
        left.generated.module_relative_path.as_str(),
        left.generated.module_namespace.as_str(),
        left.generated.owner.id,
        left.script_module.id,
    )
        .cmp(&(
            right.generated.module_relative_path.as_str(),
            right.generated.module_namespace.as_str(),
            right.generated.owner.id,
            right.script_module.id,
        ))
}

fn combined_validator_blocker() -> Diagnostic {
    Diagnostic {
        code: DiagnosticCode::Revision2CombinedValidationUnavailable,
        severity: DiagnosticSeverity::Error,
        entity: None,
        property_path: Some("schema_revision".to_owned()),
        message: "schema revision 2 is not build-ready until combined story, voice, localization, and asset validation is implemented".to_owned(),
        related_entities: Vec::new(),
        blocks_build: true,
    }
}

fn is_combined_validator_blocker(diagnostic: &Diagnostic) -> bool {
    diagnostic == &combined_validator_blocker()
}

fn ensure_omissions_have_causal_blockers(
    project: &ProjectRevision2,
    modules: &[PlannedScriptModule],
    diagnostics: &[Diagnostic],
) -> Result<(), StoryBuildError> {
    let planned_owners: BTreeSet<_> = modules
        .iter()
        .map(|module| module.generated.owner.id)
        .collect();
    let causal_blocker_entities: BTreeSet<_> = diagnostics
        .iter()
        .filter(|diagnostic| {
            diagnostic.blocks_build && diagnostic.code != DiagnosticCode::RuntimeUnqualified
        })
        .filter_map(|diagnostic| diagnostic.entity)
        .collect();
    for (draft_id, entity) in &project.entities {
        let module_id = match &entity.payload {
            EntityPayload::NpcDraft(draft) => draft.script_module.id,
            EntityPayload::QuestDraft(draft) => draft.script_module.id,
            _ => continue,
        };
        if planned_owners.contains(draft_id) {
            continue;
        }
        if !causal_blocker_entities.contains(draft_id)
            && !causal_blocker_entities.contains(&module_id)
        {
            return Err(StoryBuildError::OmittedDraftWithoutBlockingDiagnostic {
                draft_id: *draft_id,
                script_module_id: module_id,
            });
        }
    }
    Ok(())
}

fn validate_provenance_path(provenance: &StoryPropertyProvenance) -> Result<(), StoryBuildError> {
    let property_path = match provenance {
        StoryPropertyProvenance::Project { property_path, .. }
        | StoryPropertyProvenance::Entity { property_path, .. } => property_path,
    };
    validate_public_string(
        "provenance.property_path",
        property_path,
        MAX_STORY_BUILD_PROPERTY_PATH_BYTES,
    )
}

fn validate_public_string(label: &str, value: &str, limit: usize) -> Result<(), StoryBuildError> {
    if value.len() > limit {
        invariant(format!(
            "{label} is {} bytes; maximum is {limit}",
            value.len()
        ))
    } else {
        Ok(())
    }
}

fn sort_diagnostics(diagnostics: &mut [Diagnostic]) {
    diagnostics.sort_by(compare_diagnostics);
}

fn compare_diagnostics(left: &Diagnostic, right: &Diagnostic) -> Ordering {
    (
        left.severity,
        left.entity,
        left.property_path.as_deref(),
        left.code,
        left.message.as_str(),
        left.related_entities.as_slice(),
        left.blocks_build,
    )
        .cmp(&(
            right.severity,
            right.entity,
            right.property_path.as_deref(),
            right.code,
            right.message.as_str(),
            right.related_entities.as_slice(),
            right.blocks_build,
        ))
}

fn seal_bytes(bytes: &[u8]) -> ContentSeal {
    ContentSeal {
        byte_len: bytes.len() as u64,
        sha256: Sha256Digest::from_bytes(Sha256::digest(bytes).into()),
    }
}

fn invariant<T>(message: impl Into<String>) -> Result<T, StoryBuildError> {
    Err(StoryBuildError::Invariant(message.into()))
}

struct BoundedJsonWriter {
    bytes: Vec<u8>,
    limit: usize,
    first_exceeded_size: Option<usize>,
}

impl BoundedJsonWriter {
    fn new(limit: usize) -> Self {
        Self {
            bytes: Vec::with_capacity(limit.min(64 * 1_024)),
            limit,
            first_exceeded_size: None,
        }
    }
}

impl Write for BoundedJsonWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        let attempted = self.bytes.len().saturating_add(buffer.len());
        if attempted > self.limit {
            self.first_exceeded_size = Some(attempted);
            return Err(io::Error::new(
                io::ErrorKind::FileTooLarge,
                "story build plan JSON exceeded its bounded writer limit",
            ));
        }
        self.bytes.extend_from_slice(buffer);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum StoryBuildError {
    #[error("story build project JSON exceeds the {limit}-byte limit: {actual} bytes")]
    ProjectJsonTooLarge { actual: usize, limit: usize },
    #[error("invalid authoring project document: {0}")]
    InvalidProjectDocument(#[source] ProjectDocumentError),
    #[error("could not canonicalize authoring project: {0}")]
    SerializeProject(#[source] serde_json::Error),
    #[error("story build planning requires the exact canonical project JSON spelling")]
    NonCanonicalProjectJson,
    #[error("story build planning requires authoring schema revision 2")]
    Revision2Required,
    #[error("could not serialize a story draft input: {0}")]
    SerializeDraftInput(#[source] serde_json::Error),
    #[error("story build plan has {actual} modules; maximum is {limit}")]
    TooManyModules { actual: usize, limit: usize },
    #[error("story build plan source byte count overflowed")]
    SourceBytesOverflow,
    #[error("story build plan has {actual} source bytes; maximum is {limit}")]
    SourceBytesTooLarge { actual: usize, limit: usize },
    #[error("story build plan has {actual} diagnostics; maximum is {limit}")]
    TooManyDiagnostics { actual: usize, limit: usize },
    #[error("could not serialize story build plan: {0}")]
    SerializePlan(#[source] serde_json::Error),
    #[error("invalid story build plan JSON: {0}")]
    InvalidPlanJson(#[source] serde_json::Error),
    #[error("story build plan JSON exceeds the {limit}-byte limit: {actual} bytes")]
    PlanJsonTooLarge { actual: usize, limit: usize },
    #[error("story build plan JSON is not in its exact canonical spelling")]
    NonCanonicalPlanJson,
    #[error("story build plan invariant failed: {0}")]
    Invariant(String),
    #[error("canonical story build plan did not reopen to the same value")]
    CanonicalReopenMismatch,
    #[error("story build plan does not match the supplied canonical source project")]
    ProjectBindingMismatch,
    #[error(
        "story draft {draft_id} / ScriptModule {script_module_id} was omitted without a causal blocking diagnostic"
    )]
    OmittedDraftWithoutBlockingDiagnostic {
        draft_id: EntityId,
        script_module_id: EntityId,
    },
}

#[cfg(test)]
mod bounded_tests {
    use super::*;

    #[test]
    fn bounded_string_rejects_plain_and_escaped_overflow() {
        assert!(serde_json::from_str::<BoundedString<3>>(r#""four""#).is_err());
        assert!(serde_json::from_str::<BoundedString<3>>(r#""f\u006fur""#).is_err());
        assert_eq!(
            serde_json::from_str::<BoundedString<3>>(r#""fox""#)
                .unwrap()
                .into_inner(),
            "fox"
        );
    }

    #[test]
    fn bounded_sequence_rejects_before_retaining_an_extra_item() {
        let error = serde_json::from_str::<BoundedVec<u8, 2>>("[1,2,3]").unwrap_err();
        assert!(error.to_string().contains("invalid length 3"));
        assert_eq!(
            serde_json::from_str::<BoundedVec<u8, 2>>("[1,2]")
                .unwrap()
                .0,
            vec![1, 2]
        );
    }

    #[test]
    fn bounded_writer_never_retains_bytes_past_its_limit() {
        let mut writer = BoundedJsonWriter::new(4);
        writer.write_all(b"1234").unwrap();
        assert!(writer.write_all(b"5").is_err());
        assert_eq!(writer.bytes, b"1234");
        assert_eq!(writer.first_exceeded_size, Some(5));
    }
}
