//! Closed base-game plus exact-project Quest collision capability.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{self, Write};

use gore_authoring::{
    collect_project_story_collision_identities, ContentSeal as AuthoringContentSeal, EntityId,
    GameGenerationAnchor, ProjectId, ProjectRevision2, QuestCollisionCatalogInput,
    Revision2QuestGiverInput as QuestGiverInput, Revision2QuestParentInput as QuestParentInput,
    Sha256Digest as AuthoringSha256Digest, StoryCollisionCollectionError,
};
use gore_story_catalog::{CatalogError, StoryCatalogFile};
use serde::Serialize;
use sha2::{Digest as _, Sha256};

use super::{
    BaseGameCollisionInventory, ContentSeal, MAX_COLLISION_ENTRIES, MAX_COLLISION_TOTAL_BYTES,
    MAX_INVENTORY_JSON_BYTES,
};

/// Honest layer identity: pristine base game plus one exact canonical authoring project only.
pub const BASE_GAME_AND_EXACT_PROJECT_COLLISION_LAYER: &str =
    "base-game-plus-exact-project.story-collisions.v1";
const COMBINED_SEAL_DOMAIN: &[u8] =
    b"gore-story-inventory.quest-collision-capability.v1.combined-payload\0";
const COMBINED_FORMAT: &str = "quest_collision_capability";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestCollisionCoverage {
    BaseGameAndExactProjectOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestCollisionRuntimeQualification {
    RuntimeUnqualified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestCollisionBuildStatus {
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestCollisionPublicationStatus {
    NotSupported,
}

#[derive(Debug, PartialEq, Eq)]
pub struct VerifiedQuestCollisionCapability {
    project_id: ProjectId,
    project_revision: u64,
    project_target: GameGenerationAnchor,
    canonical_project: AuthoringContentSeal,
    base_inventory_payload_seal: ContentSeal,
    story_catalog_seal: ContentSeal,
    combined_source_seal: ContentSeal,
    modules: BTreeSet<String>,
    relative_paths: BTreeSet<String>,
    symbols: BTreeSet<String>,
    parents: BTreeMap<String, QuestParentInput>,
    givers: BTreeMap<String, QuestGiverInput>,
}

impl VerifiedQuestCollisionCapability {
    /// Bind closed base/catalog capabilities to one exact canonical project snapshot.
    ///
    /// The verified base inventory is consumed so its potentially multi-megabyte collision
    /// strings move into this capability instead of being cloned.
    pub fn bind(
        base: BaseGameCollisionInventory,
        catalog: &StoryCatalogFile,
        project: &ProjectRevision2,
    ) -> Result<Self, QuestCollisionCapabilityError> {
        let selections = catalog.authoring_selections()?;
        if base.generation() != catalog.generation()
            || base.story_catalog_seal() != catalog.catalog_seal()
        {
            return Err(QuestCollisionCapabilityError::CatalogBindingMismatch);
        }
        let expected_target = authoring_generation(catalog.generation());
        if project.target != expected_target {
            return Err(QuestCollisionCapabilityError::TargetMismatch);
        }

        let project_identities = collect_project_story_collision_identities(project)?;
        let project_id = project_identities.project_id();
        let project_revision = project_identities.project_revision();
        let project_target = project_identities.target().clone();
        let canonical_project = project_identities.canonical_project().clone();
        let (project_modules, project_relative_paths, project_symbols) =
            project_identities.into_collision_maps();
        let base_inventory_payload_seal = base.payload_seal().clone();
        let story_catalog_seal = base.story_catalog_seal().clone();
        let (base_modules, base_relative_paths, base_symbols) = base.into_collision_domains();
        let mut modules = base_modules.into_iter().collect::<BTreeSet<_>>();
        let mut relative_paths = base_relative_paths.into_iter().collect::<BTreeSet<_>>();
        let mut symbols = base_symbols.into_iter().collect::<BTreeSet<_>>();
        merge_project_domain("module", &mut modules, project_modules)?;
        merge_project_domain("relative path", &mut relative_paths, project_relative_paths)?;
        merge_project_domain("symbol", &mut symbols, project_symbols)?;
        enforce_combined_limits(&modules, &relative_paths, &symbols)?;

        let generation = authoring_generation(&selections.generation);
        let parents = selections
            .quest_parents
            .into_iter()
            .map(|parent| {
                (
                    parent.catalog_id,
                    QuestParentInput {
                        generation: generation.clone(),
                        source_seal: authoring_seal(&parent.quest_class.source_seal),
                        catalog_layer: parent.quest_class.catalog_layer,
                        canonical_selector: parent.quest_class.authoring_selector,
                        runtime_class: parent.quest_class.runtime_class,
                    },
                )
            })
            .collect();
        let givers = selections
            .npcs
            .into_iter()
            .map(|npc| {
                (
                    npc.catalog_id,
                    QuestGiverInput {
                        generation: generation.clone(),
                        source_seal: authoring_seal(&npc.quest_giver.source_seal),
                        catalog_layer: npc.quest_giver.catalog_layer,
                        canonical_selector: npc.quest_giver.authoring_selector,
                        runtime_unique_name: npc.quest_giver.runtime_unique_name,
                    },
                )
            })
            .collect();

        let combined_source_seal = seal_combined_payload(&CombinedPayload {
            format: COMBINED_FORMAT,
            schema_revision: 1,
            coverage: QuestCollisionCoverage::BaseGameAndExactProjectOnly,
            catalog_layer: BASE_GAME_AND_EXACT_PROJECT_COLLISION_LAYER,
            runtime_qualification: QuestCollisionRuntimeQualification::RuntimeUnqualified,
            build_status: QuestCollisionBuildStatus::Blocked,
            publication_status: QuestCollisionPublicationStatus::NotSupported,
            base_inventory_payload_seal: &base_inventory_payload_seal,
            story_catalog_seal: &story_catalog_seal,
            project_id,
            project_revision,
            project_target: &project_target,
            canonical_project: &canonical_project,
            modules: &modules,
            relative_paths: &relative_paths,
            symbols: &symbols,
        })?;
        Ok(Self {
            project_id,
            project_revision,
            project_target,
            canonical_project,
            base_inventory_payload_seal,
            story_catalog_seal,
            combined_source_seal,
            modules,
            relative_paths,
            symbols,
            parents,
            givers,
        })
    }

    pub const fn project_id(&self) -> ProjectId {
        self.project_id
    }

    pub const fn project_revision(&self) -> u64 {
        self.project_revision
    }

    pub fn project_target(&self) -> &GameGenerationAnchor {
        &self.project_target
    }

    pub fn canonical_project(&self) -> &AuthoringContentSeal {
        &self.canonical_project
    }

    pub fn base_inventory_payload_seal(&self) -> &ContentSeal {
        &self.base_inventory_payload_seal
    }

    pub fn story_catalog_seal(&self) -> &ContentSeal {
        &self.story_catalog_seal
    }

    pub fn combined_source_seal(&self) -> &ContentSeal {
        &self.combined_source_seal
    }

    pub const fn catalog_layer(&self) -> &'static str {
        BASE_GAME_AND_EXACT_PROJECT_COLLISION_LAYER
    }

    pub const fn coverage(&self) -> QuestCollisionCoverage {
        QuestCollisionCoverage::BaseGameAndExactProjectOnly
    }

    pub const fn runtime_qualification(&self) -> QuestCollisionRuntimeQualification {
        QuestCollisionRuntimeQualification::RuntimeUnqualified
    }

    pub const fn build_status(&self) -> QuestCollisionBuildStatus {
        QuestCollisionBuildStatus::Blocked
    }

    pub const fn publication_status(&self) -> QuestCollisionPublicationStatus {
        QuestCollisionPublicationStatus::NotSupported
    }

    pub fn contains_module(&self, value: &str) -> bool {
        self.modules.contains(&value.to_ascii_lowercase())
    }

    pub fn contains_relative_path(&self, value: &str) -> bool {
        self.relative_paths.contains(&value.to_ascii_lowercase())
    }

    pub fn contains_symbol(&self, value: &str) -> bool {
        self.symbols.contains(&value.to_ascii_lowercase())
    }

    pub fn resolve_parent(
        &self,
        catalog_id: &str,
    ) -> Result<QuestParentInput, QuestCollisionCapabilityError> {
        self.parents
            .get(catalog_id)
            .cloned()
            .ok_or_else(|| QuestCollisionCapabilityError::UnknownParent(catalog_id.to_owned()))
    }

    pub fn resolve_giver(
        &self,
        catalog_id: &str,
    ) -> Result<QuestGiverInput, QuestCollisionCapabilityError> {
        self.givers
            .get(catalog_id)
            .cloned()
            .ok_or_else(|| QuestCollisionCapabilityError::UnknownGiver(catalog_id.to_owned()))
    }

    /// Consume the verified capability into the existing revision-2 Quest input without cloning
    /// the multi-megabyte collision sets. The exact project is re-collected first so a stale
    /// capability cannot be applied to a different head.
    pub fn into_quest_collision_input(
        self,
        project: &ProjectRevision2,
    ) -> Result<QuestCollisionCatalogInput, QuestCollisionCapabilityError> {
        let current = collect_project_story_collision_identities(project)?;
        if current.project_id() != self.project_id
            || current.project_revision() != self.project_revision
            || current.target() != &self.project_target
            || current.canonical_project() != &self.canonical_project
        {
            return Err(QuestCollisionCapabilityError::ProjectDrift);
        }
        Ok(QuestCollisionCatalogInput {
            generation: self.project_target,
            source_seal: authoring_seal(&self.combined_source_seal),
            catalog_layer: BASE_GAME_AND_EXACT_PROJECT_COLLISION_LAYER.to_owned(),
            modules: self.modules,
            relative_paths: self.relative_paths,
            symbols: self.symbols,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum QuestCollisionCapabilityError {
    #[error("base inventory and trusted Story catalog are not exactly bound")]
    CatalogBindingMismatch,
    #[error("authoring project target does not match the trusted base-game generation")]
    TargetMismatch,
    #[error("verified Quest collision capability no longer matches the exact project head")]
    ProjectDrift,
    #[error("project {kind} identity {value:?} owned by {owner} collides with the base game")]
    BaseProjectCollision {
        kind: &'static str,
        value: String,
        owner: EntityId,
    },
    #[error("combined Quest collision capability exceeds {kind}: {actual} > {max}")]
    Limit {
        kind: &'static str,
        actual: usize,
        max: usize,
    },
    #[error("unknown trusted Story catalog Quest parent {0:?}")]
    UnknownParent(String),
    #[error("unknown trusted Story catalog Quest giver {0:?}")]
    UnknownGiver(String),
    #[error("could not serialize combined Quest collision provenance: {0}")]
    Serialize(#[source] serde_json::Error),
    #[error(transparent)]
    Catalog(#[from] CatalogError),
    #[error(transparent)]
    Project(#[from] StoryCollisionCollectionError),
}

fn merge_project_domain(
    kind: &'static str,
    combined: &mut BTreeSet<String>,
    project: BTreeMap<String, EntityId>,
) -> Result<(), QuestCollisionCapabilityError> {
    for (value, owner) in project {
        if combined.contains(&value) {
            return Err(QuestCollisionCapabilityError::BaseProjectCollision { kind, value, owner });
        }
        combined.insert(value);
    }
    Ok(())
}

fn enforce_combined_limits(
    modules: &BTreeSet<String>,
    relative_paths: &BTreeSet<String>,
    symbols: &BTreeSet<String>,
) -> Result<(), QuestCollisionCapabilityError> {
    let count = modules
        .len()
        .checked_add(relative_paths.len())
        .and_then(|count| count.checked_add(symbols.len()))
        .unwrap_or(usize::MAX);
    if count > MAX_COLLISION_ENTRIES {
        return Err(QuestCollisionCapabilityError::Limit {
            kind: "entry count",
            actual: count,
            max: MAX_COLLISION_ENTRIES,
        });
    }
    let bytes = modules
        .iter()
        .chain(relative_paths)
        .chain(symbols)
        .try_fold(0usize, |total, value| total.checked_add(value.len()))
        .unwrap_or(usize::MAX);
    if bytes > MAX_COLLISION_TOTAL_BYTES {
        return Err(QuestCollisionCapabilityError::Limit {
            kind: "aggregate entry bytes",
            actual: bytes,
            max: MAX_COLLISION_TOTAL_BYTES,
        });
    }
    Ok(())
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct CombinedPayload<'a> {
    format: &'static str,
    schema_revision: u32,
    coverage: QuestCollisionCoverage,
    catalog_layer: &'static str,
    runtime_qualification: QuestCollisionRuntimeQualification,
    build_status: QuestCollisionBuildStatus,
    publication_status: QuestCollisionPublicationStatus,
    base_inventory_payload_seal: &'a ContentSeal,
    story_catalog_seal: &'a ContentSeal,
    project_id: ProjectId,
    project_revision: u64,
    project_target: &'a GameGenerationAnchor,
    canonical_project: &'a AuthoringContentSeal,
    modules: &'a BTreeSet<String>,
    relative_paths: &'a BTreeSet<String>,
    symbols: &'a BTreeSet<String>,
}

fn seal_combined_payload(
    payload: &CombinedPayload<'_>,
) -> Result<ContentSeal, QuestCollisionCapabilityError> {
    let mut counter = BoundedCountingWriter::new(MAX_INVENTORY_JSON_BYTES);
    let counted = serde_json::to_writer(&mut counter, &payload);
    if let Some(actual) = counter.first_exceeded_size {
        return Err(QuestCollisionCapabilityError::Limit {
            kind: "combined canonical payload bytes",
            actual,
            max: MAX_INVENTORY_JSON_BYTES,
        });
    }
    counted.map_err(QuestCollisionCapabilityError::Serialize)?;
    let payload_len = counter.bytes_written;
    let mut hasher = Sha256::new();
    hasher.update(COMBINED_SEAL_DOMAIN);
    hasher.update((payload_len as u64).to_be_bytes());
    serde_json::to_writer(HashWriter(&mut hasher), &payload)
        .map_err(QuestCollisionCapabilityError::Serialize)?;
    Ok(ContentSeal {
        byte_len: payload_len as u64,
        sha256: super::Sha256Digest::from_bytes(hasher.finalize().into()),
    })
}

struct BoundedCountingWriter {
    bytes_written: usize,
    limit: usize,
    first_exceeded_size: Option<usize>,
}

impl BoundedCountingWriter {
    const fn new(limit: usize) -> Self {
        Self {
            bytes_written: 0,
            limit,
            first_exceeded_size: None,
        }
    }
}

impl Write for BoundedCountingWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let actual = self.bytes_written.saturating_add(bytes.len());
        if actual > self.limit {
            self.first_exceeded_size.get_or_insert(actual);
            return Err(io::Error::other(
                "combined canonical payload byte limit exceeded",
            ));
        }
        self.bytes_written = actual;
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

struct HashWriter<'a>(&'a mut Sha256);

impl Write for HashWriter<'_> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.update(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn authoring_generation(
    generation: &gore_story_catalog::GameGenerationSeal,
) -> GameGenerationAnchor {
    GameGenerationAnchor {
        executable: authoring_seal(&generation.executable),
    }
}

fn authoring_seal(seal: &ContentSeal) -> AuthoringContentSeal {
    AuthoringContentSeal {
        byte_len: seal.byte_len,
        sha256: AuthoringSha256Digest::from_bytes(*seal.sha256.as_bytes()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    trait AmbiguousIfClone<Marker> {
        fn marker() {}
    }

    impl<T: ?Sized> AmbiguousIfClone<()> for T {}
    impl<T: Clone> AmbiguousIfClone<u8> for T {}

    #[test]
    fn capability_is_not_clone_and_bind_consumes_the_base_inventory() {
        let _ = <VerifiedQuestCollisionCapability as AmbiguousIfClone<_>>::marker as fn();
        let _bind: fn(
            BaseGameCollisionInventory,
            &StoryCatalogFile,
            &ProjectRevision2,
        )
            -> Result<VerifiedQuestCollisionCapability, QuestCollisionCapabilityError> =
            VerifiedQuestCollisionCapability::bind;
    }

    #[test]
    fn streaming_seal_preserves_the_exact_canonical_payload_semantics() {
        let base_inventory_payload_seal = ContentSeal {
            byte_len: 11,
            sha256: crate::Sha256Digest::from_bytes([1; 32]),
        };
        let story_catalog_seal = ContentSeal {
            byte_len: 12,
            sha256: crate::Sha256Digest::from_bytes([2; 32]),
        };
        let project_id = ProjectId::from_bytes([3; 16]);
        let project_revision = 9;
        let project_target = GameGenerationAnchor {
            executable: AuthoringContentSeal {
                byte_len: 13,
                sha256: AuthoringSha256Digest::from_bytes([4; 32]),
            },
        };
        let canonical_project = AuthoringContentSeal {
            byte_len: 14,
            sha256: AuthoringSha256Digest::from_bytes([5; 32]),
        };
        let modules = BTreeSet::from(["base.module".to_owned(), "project.module".to_owned()]);
        let relative_paths =
            BTreeSet::from(["base/module.as".to_owned(), "project/module.as".to_owned()]);
        let symbols = BTreeSet::from(["ubase".to_owned(), "uproject".to_owned()]);

        let payload = CombinedPayload {
            format: COMBINED_FORMAT,
            schema_revision: 1,
            coverage: QuestCollisionCoverage::BaseGameAndExactProjectOnly,
            catalog_layer: BASE_GAME_AND_EXACT_PROJECT_COLLISION_LAYER,
            runtime_qualification: QuestCollisionRuntimeQualification::RuntimeUnqualified,
            build_status: QuestCollisionBuildStatus::Blocked,
            publication_status: QuestCollisionPublicationStatus::NotSupported,
            base_inventory_payload_seal: &base_inventory_payload_seal,
            story_catalog_seal: &story_catalog_seal,
            project_id,
            project_revision,
            project_target: &project_target,
            canonical_project: &canonical_project,
            modules: &modules,
            relative_paths: &relative_paths,
            symbols: &symbols,
        };
        let streamed = seal_combined_payload(&payload).unwrap();
        let reference = serde_json::to_vec(&payload).unwrap();
        let mut hasher = Sha256::new();
        hasher.update(COMBINED_SEAL_DOMAIN);
        hasher.update((reference.len() as u64).to_be_bytes());
        hasher.update(&reference);
        assert_eq!(streamed.byte_len, reference.len() as u64);
        assert_eq!(streamed.byte_len, 950);
        assert_eq!(
            streamed.sha256,
            crate::Sha256Digest::from_bytes(hasher.finalize().into())
        );
        assert_eq!(
            serde_json::to_value(streamed.sha256).unwrap(),
            serde_json::json!("4c6f8868c2dd31ac881d7afae79f544f3d1ccee8495286a76737b7bfc57efa3c")
        );
    }

    #[test]
    fn combined_union_is_bounded_by_count_and_aggregate_bytes() {
        let too_many = (0..=MAX_COLLISION_ENTRIES)
            .map(|index| format!("module{index:06}"))
            .collect::<BTreeSet<_>>();
        assert!(matches!(
            enforce_combined_limits(&too_many, &BTreeSet::new(), &BTreeSet::new()),
            Err(QuestCollisionCapabilityError::Limit {
                kind: "entry count",
                actual,
                max: MAX_COLLISION_ENTRIES,
            }) if actual == MAX_COLLISION_ENTRIES + 1
        ));

        let chunk = "a".repeat(crate::MAX_COLLISION_ENTRY_BYTES - 8);
        let too_large = (0..=(MAX_COLLISION_TOTAL_BYTES / crate::MAX_COLLISION_ENTRY_BYTES))
            .map(|index| format!("{index:08}{chunk}"))
            .collect::<BTreeSet<_>>();
        assert!(too_large.len() < MAX_COLLISION_ENTRIES);
        assert!(matches!(
            enforce_combined_limits(&BTreeSet::new(), &BTreeSet::new(), &too_large),
            Err(QuestCollisionCapabilityError::Limit {
                kind: "aggregate entry bytes",
                actual,
                max: MAX_COLLISION_TOTAL_BYTES,
            }) if actual > MAX_COLLISION_TOTAL_BYTES
        ));
    }
}
