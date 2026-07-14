//! Filesystem-free closed-model validation for schema revision 3.

use sha2::{Digest as _, Sha256};

use crate::model_revision3::{
    quest_collision_artifact_media_for_layer, EntityKind, EntityPayload, OriginRef,
    ProjectRevision3, ProjectRevision3ValidationError, ScriptModuleStatus,
    MAX_QUEST_COLLISION_ARTIFACT_BYTES, MAX_REVISION3_ASSETS, MAX_REVISION3_ENTITIES,
    MAX_REVISION3_ENTITY_JSON_BYTES, MAX_REVISION3_REFERENCED_ASSET_BYTES,
    MAX_REVISION3_SNAPSHOT_BYTES, REVISION3_QUEST_GENERATOR_ID, REVISION3_QUEST_GENERATOR_VERSION,
};
use crate::{
    Revision3TypedRef, LOGICAL_NPC_CLONE_GENERATOR_ID, LOGICAL_NPC_CLONE_GENERATOR_VERSION,
};

impl ProjectRevision3 {
    /// Validate every invariant available without opening the content-addressed artifact.
    ///
    /// This validates the exact reference/index relationship but does not treat hashes as
    /// authenticity evidence and does not regenerate Quest source. Artifact parsing, source
    /// re-extraction, build readiness, and runtime qualification remain outside this model slice.
    pub fn validate_closed_model(&self) -> Result<(), ProjectRevision3ValidationError> {
        if self.project_id.as_bytes().iter().all(|byte| *byte == 0) {
            return Err(ProjectRevision3ValidationError::ZeroProjectId);
        }
        if self.target.executable.byte_len == 0 {
            return Err(ProjectRevision3ValidationError::InvalidTarget);
        }
        if self.entities.len() > MAX_REVISION3_ENTITIES {
            return Err(ProjectRevision3ValidationError::TooManyEntities {
                actual: self.entities.len(),
                max: MAX_REVISION3_ENTITIES,
            });
        }
        if self.asset_store.assets.len() > MAX_REVISION3_ASSETS {
            return Err(ProjectRevision3ValidationError::TooManyAssets {
                actual: self.asset_store.assets.len(),
                max: MAX_REVISION3_ASSETS,
            });
        }

        let mut asset_bytes = 0u64;
        for (asset, meta) in &self.asset_store.assets {
            if meta.byte_len == 0
                || meta.media_type.is_empty()
                || meta.media_type.len() > 256
                || meta.media_type.chars().any(char::is_control)
            {
                return Err(ProjectRevision3ValidationError::InvalidAssetMetadata {
                    asset: *asset,
                });
            }
            asset_bytes = asset_bytes.saturating_add(meta.byte_len);
            if asset_bytes > MAX_REVISION3_REFERENCED_ASSET_BYTES {
                return Err(ProjectRevision3ValidationError::AssetBytesTooLarge {
                    actual: asset_bytes,
                    max: MAX_REVISION3_REFERENCED_ASSET_BYTES,
                });
            }
        }

        for (key, entity) in &self.entities {
            if key != &entity.id {
                return Err(ProjectRevision3ValidationError::EntityKeyMismatch {
                    key: *key,
                    embedded: entity.id,
                });
            }
            let entity_bytes = serde_json::to_vec(entity).map_err(|source| {
                ProjectRevision3ValidationError::SerializeEntity {
                    entity: *key,
                    source,
                }
            })?;
            if entity_bytes.len() > MAX_REVISION3_ENTITY_JSON_BYTES {
                return Err(ProjectRevision3ValidationError::EntityTooLarge {
                    entity: *key,
                    actual: entity_bytes.len(),
                    max: MAX_REVISION3_ENTITY_JSON_BYTES,
                });
            }
        }
        // Finish the complete cheap per-entity size preflight before following any cross-entity
        // Story/module or asset reference.
        for (key, entity) in &self.entities {
            match &entity.payload {
                EntityPayload::NpcDraft(npc) => self.validate_npc(*key, entity, npc)?,
                EntityPayload::QuestDraft(quest) => self.validate_quest(*key, entity, quest)?,
                _ => {}
            }
        }
        // A generated NPC module must not survive without the exact reverse edge. Checking the
        // module side separately closes orphan, wrong-owner-kind, and generator-label drift that
        // cannot be discovered by walking only live NPC Draft references.
        for (module_id, entity) in &self.entities {
            let EntityPayload::ScriptModule(module) = &entity.payload else {
                continue;
            };
            let npc_generated = module.owner.expected_kind == EntityKind::NpcDraft
                || module.generator_id == LOGICAL_NPC_CLONE_GENERATOR_ID
                || matches!(
                    &entity.origin,
                    OriginRef::Generated { generator_id, owner, .. }
                        if generator_id == LOGICAL_NPC_CLONE_GENERATOR_ID
                            || owner.expected_kind == EntityKind::NpcDraft
                );
            if !npc_generated || self.has_exact_npc_module_owner(*module_id, module) {
                continue;
            }
            return Err(ProjectRevision3ValidationError::OrphanNpcScriptModule {
                module: *module_id,
            });
        }
        Ok(())
    }

    fn validate_npc(
        &self,
        npc_id: crate::EntityId,
        entity: &crate::model_revision3::Entity,
        npc: &crate::model_revision3::NpcDraft,
    ) -> Result<(), ProjectRevision3ValidationError> {
        let invalid = |reason: String| ProjectRevision3ValidationError::InvalidNpcDraft {
            npc: npc_id,
            reason,
        };
        if npc.generator_id != LOGICAL_NPC_CLONE_GENERATOR_ID
            || npc.generator_version != LOGICAL_NPC_CLONE_GENERATOR_VERSION
        {
            return Err(invalid(
                "generator contract is not the closed logical NPC clone version 1".to_owned(),
            ));
        }
        if npc.input.target != self.target {
            return Err(invalid(
                "NPC generator target does not match the project target".to_owned(),
            ));
        }
        if !matches!(
            &entity.origin,
            OriginRef::New { authored_runtime_id }
                if !authored_runtime_id.is_empty()
                    && authored_runtime_id == &npc.input.unique_name
        ) {
            return Err(invalid(
                "NPC origin does not match its authored unique name".to_owned(),
            ));
        }
        if npc.script_module.project_id != self.project_id
            || npc.script_module.expected_kind != EntityKind::ScriptModule
        {
            return Err(ProjectRevision3ValidationError::InvalidNpcScriptReference { npc: npc_id });
        }
        let Some(module_entity) = self.entities.get(&npc.script_module.id) else {
            return Err(ProjectRevision3ValidationError::MissingNpcScriptModule { npc: npc_id });
        };
        let EntityPayload::ScriptModule(module) = &module_entity.payload else {
            return Err(ProjectRevision3ValidationError::MissingNpcScriptModule { npc: npc_id });
        };
        let owner = Revision3TypedRef::new(self.project_id, npc_id, EntityKind::NpcDraft);
        let expected = npc
            .regenerate_script_module(owner)
            .map_err(|error| invalid(error.to_string()))?;
        if module != &expected
            || !matches!(
                &module_entity.origin,
                OriginRef::Generated {
                    generator_id,
                    generator_version,
                    owner,
                } if generator_id == LOGICAL_NPC_CLONE_GENERATOR_ID
                    && *generator_version == LOGICAL_NPC_CLONE_GENERATOR_VERSION
                    && owner == &module.owner
            )
        {
            return Err(ProjectRevision3ValidationError::InvalidNpcScriptReference { npc: npc_id });
        }
        Ok(())
    }

    fn has_exact_npc_module_owner(
        &self,
        module_id: crate::EntityId,
        module: &crate::model_revision3::ScriptModule,
    ) -> bool {
        if module.owner.project_id != self.project_id
            || module.owner.expected_kind != EntityKind::NpcDraft
        {
            return false;
        }
        let Some(owner) = self.entities.get(&module.owner.id) else {
            return false;
        };
        let EntityPayload::NpcDraft(npc) = &owner.payload else {
            return false;
        };
        npc.script_module
            == Revision3TypedRef::new(self.project_id, module_id, EntityKind::ScriptModule)
    }

    fn validate_quest(
        &self,
        quest_id: crate::EntityId,
        entity: &crate::model_revision3::Entity,
        quest: &crate::model_revision3::QuestDraft,
    ) -> Result<(), ProjectRevision3ValidationError> {
        let reference = &quest.input.collision_catalog;
        let invalid = |reason: &str| ProjectRevision3ValidationError::InvalidQuestArtifactRef {
            quest: quest_id,
            reason: reason.to_owned(),
        };
        if quest.generator_id != REVISION3_QUEST_GENERATOR_ID
            || quest.generator_version != REVISION3_QUEST_GENERATOR_VERSION
        {
            return Err(invalid(
                "generator contract is not revision-3 Quest version 2",
            ));
        }
        if quest.input.quest_id != quest_id {
            return Err(invalid("quest input id does not match its entity id"));
        }
        if quest.input.target != self.target
            || quest.input.parent_quest.generation != self.target
            || quest.input.giver.generation != self.target
            || reference.generation != self.target
        {
            return Err(invalid(
                "Quest provenance generation does not match project target",
            ));
        }
        if quest.input.parent_quest.source_seal.byte_len == 0
            || quest.input.giver.source_seal.byte_len == 0
        {
            return Err(invalid("Quest parent or giver source seal is empty"));
        }
        let Some(expected_media_type) =
            quest_collision_artifact_media_for_layer(&reference.catalog_layer)
        else {
            return Err(invalid(
                "collision catalog layer is not the closed revision-3 layer",
            ));
        };
        if reference.artifact.byte_len == 0
            || reference.artifact.byte_len > MAX_QUEST_COLLISION_ARTIFACT_BYTES
            || reference.source_seal.byte_len != reference.artifact.byte_len
        {
            return Err(invalid(
                "raw and semantic artifact seals must have one equal bounded non-zero length",
            ));
        }
        if reference.basis_snapshot.byte_len == 0
            || reference.basis_snapshot.byte_len > MAX_REVISION3_SNAPSHOT_BYTES
        {
            return Err(invalid("basis snapshot seal has an invalid byte length"));
        }

        let asset = reference.artifact.sha256;
        let Some(meta) = self.asset_store.assets.get(&asset) else {
            return Err(ProjectRevision3ValidationError::MissingQuestArtifact {
                quest: quest_id,
                artifact: asset,
            });
        };
        if meta.byte_len != reference.artifact.byte_len {
            return Err(
                ProjectRevision3ValidationError::QuestArtifactMetadataMismatch {
                    quest: quest_id,
                    artifact: asset,
                    reason: "AssetStore byte length differs from raw artifact seal".to_owned(),
                },
            );
        }
        if meta.media_type != expected_media_type {
            return Err(
                ProjectRevision3ValidationError::QuestArtifactMetadataMismatch {
                    quest: quest_id,
                    artifact: asset,
                    reason: "AssetStore media type is not the closed Quest artifact type"
                        .to_owned(),
                },
            );
        }

        if quest.script_module.project_id != self.project_id
            || quest.script_module.expected_kind != EntityKind::ScriptModule
        {
            return Err(
                ProjectRevision3ValidationError::InvalidQuestScriptReference { quest: quest_id },
            );
        }
        let Some(module_entity) = self.entities.get(&quest.script_module.id) else {
            return Err(ProjectRevision3ValidationError::MissingQuestScriptModule {
                quest: quest_id,
            });
        };
        let EntityPayload::ScriptModule(module) = &module_entity.payload else {
            return Err(ProjectRevision3ValidationError::MissingQuestScriptModule {
                quest: quest_id,
            });
        };
        if module.owner.project_id != self.project_id
            || module.owner.id != quest_id
            || module.owner.expected_kind != EntityKind::QuestDraft
            || module.generator_id != REVISION3_QUEST_GENERATOR_ID
            || module.generator_version != REVISION3_QUEST_GENERATOR_VERSION
            || module.status != ScriptModuleStatus::OFFLINE_DRAFT_RUNTIME_UNQUALIFIED
        {
            return Err(
                ProjectRevision3ValidationError::InvalidQuestScriptReference { quest: quest_id },
            );
        }
        if module.source_sha256
            != crate::Sha256Digest::from_bytes(Sha256::digest(module.source.as_bytes()).into())
            || !matches!(
                &module_entity.origin,
                OriginRef::Generated {
                    generator_id,
                    generator_version,
                    owner,
                } if generator_id == REVISION3_QUEST_GENERATOR_ID
                    && *generator_version == REVISION3_QUEST_GENERATOR_VERSION
                    && owner == &module.owner
            )
        {
            return Err(
                ProjectRevision3ValidationError::InvalidQuestScriptReference { quest: quest_id },
            );
        }
        if !matches!(
            &entity.origin,
            OriginRef::New {
                authored_runtime_id
            } if !authored_runtime_id.is_empty()
                && authored_runtime_id == &quest.input.technical_id
        ) {
            return Err(invalid("Quest origin does not match its technical id"));
        }
        Ok(())
    }
}
