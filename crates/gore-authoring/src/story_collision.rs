//! Deterministic, source-independent collision identities for exact authoring project snapshots.

use std::collections::BTreeMap;

use sha2::{Digest as _, Sha256};

use crate::model_revision2::{EntityKind, EntityPayload, TypedRef};
use crate::model_revision3::{
    EntityKind as Revision3EntityKind, EntityPayload as Revision3EntityPayload,
    TypedRef as Revision3TypedRef,
};
use crate::{
    ContentSeal, DiagnosticCode, EntityId, GameGenerationAnchor, ProjectId, ProjectRevision2,
    ProjectRevision3, ProjectRevision3JsonError, Sha256Digest, StoryRegenerationError,
    ValidationProfile,
};

/// Closed collision identities regenerated from every typed NPC/Quest draft in one exact project.
///
/// Values are canonical lowercase collision keys. Persisted ScriptModule names/source are never
/// parsed or trusted as identity input; normal project validation first proves that persisted
/// generated modules still equal deterministic regeneration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectStoryCollisionIdentities {
    project_id: ProjectId,
    project_revision: u64,
    target: GameGenerationAnchor,
    canonical_project: ContentSeal,
    modules: BTreeMap<String, EntityId>,
    relative_paths: BTreeMap<String, EntityId>,
    symbols: BTreeMap<String, EntityId>,
}

impl ProjectStoryCollisionIdentities {
    pub const fn project_id(&self) -> ProjectId {
        self.project_id
    }

    pub const fn project_revision(&self) -> u64 {
        self.project_revision
    }

    pub fn target(&self) -> &GameGenerationAnchor {
        &self.target
    }

    pub fn canonical_project(&self) -> &ContentSeal {
        &self.canonical_project
    }

    pub fn modules(&self) -> &BTreeMap<String, EntityId> {
        &self.modules
    }

    pub fn relative_paths(&self) -> &BTreeMap<String, EntityId> {
        &self.relative_paths
    }

    pub fn symbols(&self) -> &BTreeMap<String, EntityId> {
        &self.symbols
    }

    /// Consume the regenerated identity evidence and move its potentially large collision maps.
    ///
    /// Snapshot metadata remains available through the getters before this call. This conversion
    /// exists so downstream closed capabilities never need to duplicate every identity string.
    pub fn into_collision_maps(
        self,
    ) -> (
        BTreeMap<String, EntityId>,
        BTreeMap<String, EntityId>,
        BTreeMap<String, EntityId>,
    ) {
        (self.modules, self.relative_paths, self.symbols)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum StoryCollisionCollectionError {
    #[error("could not serialize the exact revision-2 project snapshot: {0}")]
    SerializeProject(#[source] serde_json::Error),
    #[error("could not serialize the exact native revision-3 project snapshot: {0}")]
    SerializeRevision3Project(#[source] ProjectRevision3JsonError),
    #[error("project has {count} non-runtime Story validation blockers")]
    InvalidProject { count: usize },
    #[error("invalid native revision-3 Quest-free Story basis: {reason}")]
    InvalidRevision3Basis { reason: String },
    #[error("could not regenerate Story draft {owner}: {source}")]
    Regeneration {
        owner: EntityId,
        #[source]
        source: StoryRegenerationError,
    },
    #[error(
        "project Story {kind} identity {value:?} collides between {first_owner} and {second_owner}"
    )]
    Collision {
        kind: &'static str,
        value: String,
        first_owner: EntityId,
        second_owner: EntityId,
    },
    #[error("Story draft {owner} has a foreign or mistyped generated ScriptModule reference")]
    InvalidModuleReference { owner: EntityId },
    #[error(
        "generated ScriptModule {module} is claimed by both Story drafts {first_owner} and {second_owner}"
    )]
    SharedModule {
        module: EntityId,
        first_owner: EntityId,
        second_owner: EntityId,
    },
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct StoryCollisionCollectionLimits {
    pub max_count: usize,
    pub max_bytes: usize,
    pub max_value_bytes: usize,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum BoundedStoryCollisionCollectionError {
    #[error(transparent)]
    Collection(#[from] StoryCollisionCollectionError),
    #[error("Story collision resource limit exceeded for {kind}: {actual} > {limit}")]
    ResourceLimit {
        kind: &'static str,
        actual: usize,
        limit: usize,
    },
}

/// Regenerate the complete Story collision footprint for one exact revision-2 project.
pub fn collect_project_story_collision_identities(
    project: &ProjectRevision2,
) -> Result<ProjectStoryCollisionIdentities, StoryCollisionCollectionError> {
    collect_project_story_collision_identities_inner(project, None).map_err(|error| match error {
        BoundedStoryCollisionCollectionError::Collection(error) => error,
        BoundedStoryCollisionCollectionError::ResourceLimit { .. } => {
            unreachable!("the unbounded public collector has no resource budget")
        }
    })
}

#[cfg(test)]
pub(crate) fn collect_project_story_collision_identities_bounded(
    project: &ProjectRevision2,
    limits: StoryCollisionCollectionLimits,
) -> Result<ProjectStoryCollisionIdentities, BoundedStoryCollisionCollectionError> {
    collect_project_story_collision_identities_inner(project, Some(limits))
}

/// Regenerate the complete Story collision footprint directly from one exact, Quest-free
/// revision-3 project.
///
/// The native R3 canonical project is sealed as-is. No revision-2 projection or migration occurs.
pub fn collect_revision3_story_collision_identities(
    project: &ProjectRevision3,
) -> Result<ProjectStoryCollisionIdentities, StoryCollisionCollectionError> {
    collect_revision3_story_collision_identities_inner(project, None).map_err(|error| match error {
        BoundedStoryCollisionCollectionError::Collection(error) => error,
        BoundedStoryCollisionCollectionError::ResourceLimit { .. } => {
            unreachable!("the unbounded public collector has no resource budget")
        }
    })
}

pub(crate) fn collect_revision3_story_collision_identities_bounded(
    project: &ProjectRevision3,
    limits: StoryCollisionCollectionLimits,
) -> Result<ProjectStoryCollisionIdentities, BoundedStoryCollisionCollectionError> {
    collect_revision3_story_collision_identities_inner(project, Some(limits))
}

fn collect_revision3_story_collision_identities_inner(
    project: &ProjectRevision3,
    limits: Option<StoryCollisionCollectionLimits>,
) -> Result<ProjectStoryCollisionIdentities, BoundedStoryCollisionCollectionError> {
    crate::validate_revision3_quest_free_basis(project).map_err(|error| {
        StoryCollisionCollectionError::InvalidRevision3Basis {
            reason: error.to_string(),
        }
    })?;

    let canonical = project
        .to_canonical_json()
        .map_err(StoryCollisionCollectionError::SerializeRevision3Project)?;
    let canonical_project = seal_bytes(canonical.as_bytes());
    let mut modules = BTreeMap::new();
    let mut relative_paths = BTreeMap::new();
    let mut symbols = BTreeMap::new();
    let mut claimed_modules = BTreeMap::new();
    let mut budget = limits.map(StoryCollisionBudget::new);

    for (owner, entity) in &project.entities {
        let Revision3EntityPayload::NpcDraft(draft) = &entity.payload else {
            continue;
        };
        let module_ref = &draft.script_module;
        let identity = draft
            .regenerate_script_module_with_identity(Revision3TypedRef::new(
                project.project_id,
                *owner,
                Revision3EntityKind::NpcDraft,
            ))
            .map_err(|source| StoryCollisionCollectionError::Regeneration {
                owner: *owner,
                source,
            })?
            .1;
        if module_ref.project_id != project.project_id
            || module_ref.expected_kind != Revision3EntityKind::ScriptModule
        {
            return Err(
                StoryCollisionCollectionError::InvalidModuleReference { owner: *owner }.into(),
            );
        }
        if let Some(first_owner) = claimed_modules.insert(module_ref.id, *owner) {
            return Err(StoryCollisionCollectionError::SharedModule {
                module: module_ref.id,
                first_owner,
                second_owner: *owner,
            }
            .into());
        }
        insert_identity(
            "module",
            &mut modules,
            identity.module_namespace,
            *owner,
            &mut budget,
        )?;
        insert_identity(
            "relative path",
            &mut relative_paths,
            identity.module_relative_path,
            *owner,
            &mut budget,
        )?;
        for symbol in identity.symbols {
            insert_identity("symbol", &mut symbols, symbol, *owner, &mut budget)?;
        }
    }

    Ok(ProjectStoryCollisionIdentities {
        project_id: project.project_id,
        project_revision: project.revision,
        target: project.target.clone(),
        canonical_project,
        modules,
        relative_paths,
        symbols,
    })
}

fn collect_project_story_collision_identities_inner(
    project: &ProjectRevision2,
    limits: Option<StoryCollisionCollectionLimits>,
) -> Result<ProjectStoryCollisionIdentities, BoundedStoryCollisionCollectionError> {
    let diagnostics = if limits.is_some() {
        project.validate_story_entities_for_bounded_collision_collection(
            ValidationProfile::Experimental,
        )
    } else {
        project.validate_story_entities_with_profile(ValidationProfile::Experimental)
    };
    let blockers = diagnostics
        .into_iter()
        .filter(|diagnostic| diagnostic.code != DiagnosticCode::RuntimeUnqualified)
        .count();
    if blockers != 0 {
        return Err(StoryCollisionCollectionError::InvalidProject { count: blockers }.into());
    }

    let canonical = project
        .to_canonical_json()
        .map_err(StoryCollisionCollectionError::SerializeProject)?;
    let canonical_project = seal_bytes(canonical.as_bytes());
    let mut modules = BTreeMap::new();
    let mut relative_paths = BTreeMap::new();
    let mut symbols = BTreeMap::new();
    let mut claimed_modules = BTreeMap::new();
    let mut budget = limits.map(StoryCollisionBudget::new);

    for (owner, entity) in &project.entities {
        let (module_ref, identity) = match &entity.payload {
            EntityPayload::NpcDraft(draft) => (
                &draft.script_module,
                draft
                    .regenerate_script_module_with_identity(TypedRef::new(
                        project.project_id,
                        *owner,
                        EntityKind::NpcDraft,
                    ))
                    .map_err(|source| StoryCollisionCollectionError::Regeneration {
                        owner: *owner,
                        source,
                    })?
                    .1,
            ),
            EntityPayload::QuestDraft(draft) => (
                &draft.script_module,
                draft
                    .regenerate_script_module_with_identity(TypedRef::new(
                        project.project_id,
                        *owner,
                        EntityKind::QuestDraft,
                    ))
                    .map_err(|source| StoryCollisionCollectionError::Regeneration {
                        owner: *owner,
                        source,
                    })?
                    .1,
            ),
            _ => continue,
        };
        if module_ref.project_id != project.project_id
            || module_ref.expected_kind != EntityKind::ScriptModule
        {
            return Err(
                StoryCollisionCollectionError::InvalidModuleReference { owner: *owner }.into(),
            );
        }
        if let Some(first_owner) = claimed_modules.insert(module_ref.id, *owner) {
            return Err(StoryCollisionCollectionError::SharedModule {
                module: module_ref.id,
                first_owner,
                second_owner: *owner,
            }
            .into());
        }
        insert_identity(
            "module",
            &mut modules,
            identity.module_namespace,
            *owner,
            &mut budget,
        )?;
        insert_identity(
            "relative path",
            &mut relative_paths,
            identity.module_relative_path,
            *owner,
            &mut budget,
        )?;
        for symbol in identity.symbols {
            insert_identity("symbol", &mut symbols, symbol, *owner, &mut budget)?;
        }
    }

    Ok(ProjectStoryCollisionIdentities {
        project_id: project.project_id,
        project_revision: project.revision,
        target: project.target.clone(),
        canonical_project,
        modules,
        relative_paths,
        symbols,
    })
}

fn insert_identity(
    kind: &'static str,
    identities: &mut BTreeMap<String, EntityId>,
    value: String,
    owner: EntityId,
    budget: &mut Option<StoryCollisionBudget>,
) -> Result<(), BoundedStoryCollisionCollectionError> {
    if let Some(budget) = budget {
        budget.charge(&value)?;
    }
    let value = value.to_ascii_lowercase();
    if let Some(first_owner) = identities.insert(value.clone(), owner) {
        return Err(StoryCollisionCollectionError::Collision {
            kind,
            value,
            first_owner,
            second_owner: owner,
        }
        .into());
    }
    Ok(())
}

struct StoryCollisionBudget {
    limits: StoryCollisionCollectionLimits,
    count: usize,
    bytes: usize,
}

impl StoryCollisionBudget {
    fn new(limits: StoryCollisionCollectionLimits) -> Self {
        Self {
            limits,
            count: 0,
            bytes: 0,
        }
    }

    fn charge(&mut self, value: &str) -> Result<(), BoundedStoryCollisionCollectionError> {
        if value.len() > self.limits.max_value_bytes {
            return Err(BoundedStoryCollisionCollectionError::ResourceLimit {
                kind: "single collision identity bytes",
                actual: value.len(),
                limit: self.limits.max_value_bytes,
            });
        }
        self.count = self.count.saturating_add(1);
        if self.count > self.limits.max_count {
            return Err(BoundedStoryCollisionCollectionError::ResourceLimit {
                kind: "collision identity count",
                actual: self.count,
                limit: self.limits.max_count,
            });
        }
        self.bytes = self.bytes.saturating_add(value.len());
        if self.bytes > self.limits.max_bytes {
            return Err(BoundedStoryCollisionCollectionError::ResourceLimit {
                kind: "collision identity bytes",
                actual: self.bytes,
                limit: self.limits.max_bytes,
            });
        }
        Ok(())
    }
}

fn seal_bytes(bytes: &[u8]) -> ContentSeal {
    ContentSeal {
        byte_len: bytes.len() as u64,
        sha256: Sha256Digest::from_bytes(Sha256::digest(bytes).into()),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use crate::model_revision2::{
        Entity, EntityKind, EntityPayload, NpcDraft, NpcDraftInput, NpcParentClassInput, OriginRef,
        ProjectRevision2, SchemaRevisionV2, TypedRef,
    };
    use crate::{
        AssetStoreIndex, FormatV2, ProjectMeta, LOGICAL_NPC_CLONE_GENERATOR_ID,
        LOGICAL_NPC_CLONE_GENERATOR_VERSION,
    };

    use super::*;

    fn project_id(value: u8) -> ProjectId {
        ProjectId::from_bytes([value; 16])
    }

    fn entity_id(value: u8) -> EntityId {
        EntityId::from_bytes([value; 16])
    }

    fn seal(value: u8) -> ContentSeal {
        ContentSeal {
            byte_len: 1_000,
            sha256: Sha256Digest::from_bytes([value; 32]),
        }
    }

    fn target() -> GameGenerationAnchor {
        GameGenerationAnchor {
            executable: seal(1),
        }
    }

    fn empty_project() -> ProjectRevision2 {
        ProjectRevision2 {
            format: FormatV2,
            schema_revision: SchemaRevisionV2,
            project_id: project_id(1),
            revision: 7,
            meta: ProjectMeta {
                name: "collision collection".into(),
                version: "0.1.0".into(),
                author: "test".into(),
            },
            target: target(),
            authoring_locales: BTreeSet::new(),
            entities: BTreeMap::new(),
            asset_store: AssetStoreIndex::default(),
        }
    }

    fn parent(
        target: &GameGenerationAnchor,
        seal_value: u8,
        runtime_class: &str,
    ) -> NpcParentClassInput {
        NpcParentClassInput {
            generation: target.clone(),
            source_seal: seal(seal_value),
            catalog_layer: "base-game.test.characters".into(),
            canonical_selector: format!("Catalog{runtime_class}"),
            runtime_class: runtime_class.into(),
        }
    }

    fn add_npc(
        project: &mut ProjectRevision2,
        owner_value: u8,
        module_value: u8,
        module_namespace: &str,
        unique_name: &str,
    ) {
        let owner_id = entity_id(owner_value);
        let module_id = entity_id(module_value);
        let owner = TypedRef::new(project.project_id, owner_id, EntityKind::NpcDraft);
        let draft = NpcDraft {
            generator_id: LOGICAL_NPC_CLONE_GENERATOR_ID.into(),
            generator_version: LOGICAL_NPC_CLONE_GENERATOR_VERSION,
            input: NpcDraftInput {
                target: project.target.clone(),
                module_namespace: module_namespace.into(),
                unique_name: unique_name.into(),
                parent_character_definition: parent(
                    &project.target,
                    owner_value,
                    "UCharacterDefinition_Human_Base",
                ),
                parent_ai_agent_config: parent(
                    &project.target,
                    owner_value.wrapping_add(1),
                    "UAIAgentConfig_Human_Base",
                ),
                parent_spawn_definition: parent(
                    &project.target,
                    owner_value.wrapping_add(2),
                    "USpawnAIAgentDefinition_Base",
                ),
            },
            script_module: TypedRef::new(project.project_id, module_id, EntityKind::ScriptModule),
        };
        let script = draft.regenerate_script_module(owner.clone()).unwrap();
        project.entities.insert(
            owner_id,
            Entity {
                id: owner_id,
                display_name: unique_name.into(),
                origin: OriginRef::New {
                    authored_runtime_id: unique_name.into(),
                },
                revision: 0,
                payload: EntityPayload::NpcDraft(draft),
            },
        );
        project.entities.insert(
            module_id,
            Entity {
                id: module_id,
                display_name: format!("{unique_name} script"),
                origin: OriginRef::Generated {
                    generator_id: script.generator_id.clone(),
                    generator_version: script.generator_version,
                    owner,
                },
                revision: 0,
                payload: EntityPayload::ScriptModule(script),
            },
        );
    }

    #[test]
    fn regenerated_collision_footprint_is_closed_folded_and_deterministic() {
        let mut project = empty_project();
        add_npc(&mut project, 10, 11, "GoreMods.Npcs.TestClone", "TestClone");

        let first = collect_project_story_collision_identities(&project).unwrap();
        let second = collect_project_story_collision_identities(&project).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.project_id(), project.project_id);
        assert_eq!(first.project_revision(), 7);
        assert_eq!(first.target(), &project.target);
        assert_eq!(
            first.modules().keys().collect::<Vec<_>>(),
            vec![&"goremods.npcs.testclone".to_owned()]
        );
        assert_eq!(
            first.relative_paths().keys().collect::<Vec<_>>(),
            vec![&"goremods/npcs/testclone.as".to_owned()]
        );
        assert!(first
            .symbols()
            .contains_key("ucharacterdefinition_human_testclone"));
        assert!(first
            .symbols()
            .contains_key("uaiagentconfig_human_testclone"));
        assert!(first
            .symbols()
            .contains_key("uspawnaiagentdefinition_testclone"));
    }

    #[test]
    fn persisted_script_identity_is_never_accepted_as_an_authority() {
        let mut project = empty_project();
        add_npc(&mut project, 10, 11, "GoreMods.Npcs.Safe", "SafeNpc");
        let EntityPayload::ScriptModule(module) =
            &mut project.entities.get_mut(&entity_id(11)).unwrap().payload
        else {
            panic!("expected generated script module");
        };
        module.module_namespace = "Attacker.Controlled.Identity".into();

        assert!(matches!(
            collect_project_story_collision_identities(&project),
            Err(StoryCollisionCollectionError::InvalidProject { .. })
        ));
    }

    #[test]
    fn intra_project_case_insensitive_collisions_fail_closed() {
        let mut project = empty_project();
        add_npc(&mut project, 10, 11, "GoreMods.Npcs.Same", "FirstNpc");
        add_npc(&mut project, 20, 21, "goremods.npcs.same", "SecondNpc");

        assert!(matches!(
            collect_project_story_collision_identities(&project),
            Err(StoryCollisionCollectionError::InvalidProject { .. })
                | Err(StoryCollisionCollectionError::Collision {
                    kind: "module" | "relative path",
                    ..
                })
        ));
    }

    #[test]
    fn exact_snapshot_seal_changes_with_project_revision() {
        let project = empty_project();
        let first = collect_project_story_collision_identities(&project).unwrap();
        assert!(first.modules().is_empty());
        assert!(first.relative_paths().is_empty());
        assert!(first.symbols().is_empty());

        let mut next = project;
        next.revision += 1;
        let second = collect_project_story_collision_identities(&next).unwrap();
        assert_ne!(first.canonical_project(), second.canonical_project());
    }

    #[test]
    fn bounded_collection_debits_before_growing_complete_identity_maps() {
        let mut project = empty_project();
        add_npc(&mut project, 10, 11, "GoreMods.Npcs.Bounded", "BoundedNpc");

        assert!(matches!(
            collect_project_story_collision_identities_bounded(
                &project,
                StoryCollisionCollectionLimits {
                    max_count: 1,
                    max_bytes: 16 * 1024,
                    max_value_bytes: 512,
                },
            ),
            Err(BoundedStoryCollisionCollectionError::ResourceLimit {
                kind: "collision identity count",
                actual: 2,
                limit: 1,
            })
        ));
        assert!(matches!(
            collect_project_story_collision_identities_bounded(
                &project,
                StoryCollisionCollectionLimits {
                    max_count: 100,
                    max_bytes: 16 * 1024,
                    max_value_bytes: 3,
                },
            ),
            Err(BoundedStoryCollisionCollectionError::ResourceLimit {
                kind: "single collision identity bytes",
                ..
            })
        ));
    }
}
