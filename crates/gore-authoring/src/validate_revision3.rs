//! Filesystem-free closed-model validation for schema revision 3.

use std::collections::{BTreeMap, BTreeSet};

use sha2::{Digest as _, Sha256};

use crate::model_revision3::{
    quest_collision_artifact_media_for_layer, revision3_voice_target_key_v1, EntityKind,
    EntityPayload, OriginRef, ProjectRevision3, ProjectRevision3ValidationError,
    ScriptModuleStatus, VoiceTargetResolution, MAX_CATALOG_LAYER_BYTES,
    MAX_QUEST_COLLISION_ARTIFACT_BYTES, MAX_REVISION3_ASSETS, MAX_REVISION3_ENTITIES,
    MAX_REVISION3_ENTITY_JSON_BYTES, MAX_REVISION3_ITEM_CLASS_BYTES_V1,
    MAX_REVISION3_ITEM_ENUM_TYPE_BYTES_V1, MAX_REVISION3_ITEM_FIELD_NAME_BYTES_V1,
    MAX_REVISION3_ITEM_PATCH_FIELDS_V1, MAX_REVISION3_ITEM_STRING_BYTES_V1,
    MAX_REVISION3_ITEM_STRING_TOTAL_BYTES_V1, MAX_REVISION3_NPC_GREETING_BINDINGS_V1,
    MAX_REVISION3_QUEST_TRANSCRIPT_BINDINGS_V1, MAX_REVISION3_REFERENCED_ASSET_BYTES,
    MAX_REVISION3_SNAPSHOT_BYTES, REVISION3_QUEST_GENERATOR_ID, REVISION3_QUEST_GENERATOR_VERSION,
};
use crate::story_transaction_revision3_voice::{
    MAX_REVISION3_VOICE_LOGICAL_NAME_BYTES_V1, MAX_REVISION3_VOICE_SLOT_CANDIDATES_V1,
};
use crate::story_transaction_revision3_voice_target::{
    validate_revision3_voice_loc_id_basename_stem_v1, validate_revision3_voice_target_resolution_v1,
};
use crate::{
    validate_draft_quest_objective_titles, validate_draft_quest_transition_plan_v1, EntityId,
    LocaleCode, Revision3TypedRef, LOGICAL_NPC_CLONE_GENERATOR_ID,
    LOGICAL_NPC_CLONE_GENERATOR_VERSION,
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
        self.validate_voice_entities()?;
        self.validate_item_patches()?;
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

    fn validate_item_patches(&self) -> Result<(), ProjectRevision3ValidationError> {
        let invalid = |item_patch: EntityId, reason: String| {
            ProjectRevision3ValidationError::InvalidItemPatch { item_patch, reason }
        };
        let mut targets = BTreeMap::<String, EntityId>::new();

        for (id, entity) in &self.entities {
            let EntityPayload::ItemPatch(patch) = &entity.payload else {
                continue;
            };
            if entity.display_name.trim().is_empty()
                || entity.display_name.len() > MAX_REVISION3_ITEM_CLASS_BYTES_V1
                || entity.display_name.chars().any(char::is_control)
            {
                return Err(invalid(
                    *id,
                    "display name is not bounded printable text".to_owned(),
                ));
            }
            let OriginRef::Vanilla {
                generation,
                catalog_layer,
                canonical_selector,
                source_seal,
            } = &entity.origin
            else {
                return Err(invalid(
                    *id,
                    "origin must be exact sealed vanilla catalog provenance".to_owned(),
                ));
            };
            if generation != &self.target {
                return Err(invalid(
                    *id,
                    "origin generation does not match the project target".to_owned(),
                ));
            }
            if source_seal.byte_len == 0 {
                return Err(invalid(*id, "origin source seal has zero bytes".to_owned()));
            }
            if !valid_catalog_component(catalog_layer, MAX_CATALOG_LAYER_BYTES) {
                return Err(invalid(
                    *id,
                    "catalog layer is not bounded canonical text".to_owned(),
                ));
            }
            if !valid_item_identifier(&patch.vanilla_class, MAX_REVISION3_ITEM_CLASS_BYTES_V1) {
                return Err(invalid(
                    *id,
                    "vanilla class is not one canonical AngelScript identifier".to_owned(),
                ));
            }
            if canonical_selector != &patch.vanilla_class {
                return Err(invalid(
                    *id,
                    "origin canonical selector does not equal the patched vanilla class".to_owned(),
                ));
            }
            if patch.fields.is_empty() || patch.fields.len() > MAX_REVISION3_ITEM_PATCH_FIELDS_V1 {
                return Err(invalid(
                    *id,
                    format!(
                        "field map must contain 1..={} entries",
                        MAX_REVISION3_ITEM_PATCH_FIELDS_V1
                    ),
                ));
            }

            let mut string_bytes = 0usize;
            for (name, value) in &patch.fields {
                if !valid_item_identifier(name, MAX_REVISION3_ITEM_FIELD_NAME_BYTES_V1) {
                    return Err(invalid(
                        *id,
                        format!("field {name:?} is not one bounded canonical identifier"),
                    ));
                }
                match value {
                    crate::model_revision3::ItemScalarValueV1::String(value) => {
                        if value.len() > MAX_REVISION3_ITEM_STRING_BYTES_V1
                            || value.chars().any(|character| character == '\0')
                        {
                            return Err(invalid(
                                *id,
                                format!("string field {name} exceeds its closed value budget"),
                            ));
                        }
                        string_bytes = string_bytes.saturating_add(value.len());
                    }
                    crate::model_revision3::ItemScalarValueV1::Enum { enum_type, .. } => {
                        if !valid_qualified_item_identifier(
                            enum_type,
                            MAX_REVISION3_ITEM_ENUM_TYPE_BYTES_V1,
                        ) {
                            return Err(invalid(
                                *id,
                                format!("enum field {name} has an invalid declared enum type"),
                            ));
                        }
                    }
                    _ => {}
                }
                if string_bytes > MAX_REVISION3_ITEM_STRING_TOTAL_BYTES_V1 {
                    return Err(invalid(
                        *id,
                        "item string fields exceed their aggregate byte budget".to_owned(),
                    ));
                }
            }

            // Catalog layer and source seal prove which observation informed the edit, but
            // neither creates another runtime target. One project generation/class has one patch.
            let target_key = canonical_selector.clone();
            if let Some(existing_patch) = targets.insert(target_key, *id) {
                return Err(ProjectRevision3ValidationError::DuplicateItemPatchTarget {
                    item_patch: *id,
                    existing_patch,
                });
            }
        }
        Ok(())
    }

    fn validate_voice_entities(&self) -> Result<(), ProjectRevision3ValidationError> {
        let invalid_graph = |entity: EntityId, reason: String| {
            ProjectRevision3ValidationError::InvalidVoiceGraph { entity, reason }
        };
        let mut slot_owners = BTreeMap::<EntityId, (EntityId, LocaleCode)>::new();

        // Close every forward DialogLine edge first. This also constructs the one-and-only owner
        // table used by the VoiceSlot pass below.
        for (line_id, entity) in &self.entities {
            let EntityPayload::DialogLine(line) = &entity.payload else {
                continue;
            };
            if line.localization.project_id != self.project_id
                || line.localization.expected_kind != EntityKind::LocalizationEntry
                || line.localization.id == *line_id
            {
                return Err(invalid_graph(
                    *line_id,
                    "LocalizationEntry reference is not exact-project and kind-bound".to_owned(),
                ));
            }
            let Some(localization_entity) = self.entities.get(&line.localization.id) else {
                return Err(invalid_graph(
                    *line_id,
                    "referenced LocalizationEntry is missing".to_owned(),
                ));
            };
            let EntityPayload::LocalizationEntry(localization) = &localization_entity.payload
            else {
                return Err(invalid_graph(
                    *line_id,
                    "referenced entity is not a LocalizationEntry".to_owned(),
                ));
            };
            // A LocalizationEntry is a general story identity until this line actually owns
            // Voice content. Non-Voice dialog projects remain valid; the Voice-take and
            // target transactions apply the same portable stem predicate before creating or
            // resolving the first slot.
            if !line.voice_slots.is_empty()
                && validate_revision3_voice_loc_id_basename_stem_v1(&localization.loc_id).is_err()
            {
                return Err(invalid_graph(
                    line.localization.id,
                    "LocID is not one canonical portable ASCII Voice basename stem".to_owned(),
                ));
            }

            for (locale, reference) in &line.voice_slots {
                if !self.authoring_locales.contains(locale) {
                    return Err(invalid_graph(
                        *line_id,
                        format!("voice locale {locale} is absent from authoring_locales"),
                    ));
                }
                if reference.project_id != self.project_id
                    || reference.expected_kind != EntityKind::VoiceSlot
                {
                    return Err(invalid_graph(
                        *line_id,
                        format!("voice locale {locale} is not an exact-project VoiceSlot ref"),
                    ));
                }
                let Some(slot_entity) = self.entities.get(&reference.id) else {
                    return Err(invalid_graph(
                        *line_id,
                        format!("voice locale {locale} references a missing VoiceSlot"),
                    ));
                };
                let EntityPayload::VoiceSlot(slot) = &slot_entity.payload else {
                    return Err(invalid_graph(
                        *line_id,
                        format!("voice locale {locale} reference has the wrong entity kind"),
                    ));
                };
                if &slot.locale != locale {
                    return Err(invalid_graph(
                        *line_id,
                        format!(
                            "line locale {locale} references a VoiceSlot for {}",
                            slot.locale
                        ),
                    ));
                }
                if let Some((first_line, first_locale)) =
                    slot_owners.insert(reference.id, (*line_id, locale.clone()))
                {
                    return Err(invalid_graph(
                        reference.id,
                        format!(
                            "VoiceSlot is shared by {first_line}/{first_locale} and {line_id}/{locale}"
                        ),
                    ));
                }
            }
        }

        let mut resolved_targets = BTreeMap::<(String, String), EntityId>::new();
        for (entity_id, entity) in &self.entities {
            match &entity.payload {
                EntityPayload::VoiceSlot(slot) => {
                    if !self.authoring_locales.contains(&slot.locale) {
                        return Err(invalid_graph(
                            *entity_id,
                            format!(
                                "VoiceSlot locale {} is absent from authoring_locales",
                                slot.locale
                            ),
                        ));
                    }
                    if !slot_owners.contains_key(entity_id) {
                        return Err(invalid_graph(
                            *entity_id,
                            "VoiceSlot has no unique DialogLine/locale owner".to_owned(),
                        ));
                    }
                    if slot.candidates.len() > MAX_REVISION3_VOICE_SLOT_CANDIDATES_V1 {
                        return Err(invalid_graph(
                            *entity_id,
                            format!(
                                "VoiceSlot has {} take candidates; maximum is {}",
                                slot.candidates.len(),
                                MAX_REVISION3_VOICE_SLOT_CANDIDATES_V1
                            ),
                        ));
                    }
                    let mut candidates = BTreeSet::new();
                    for candidate in &slot.candidates {
                        if candidate.project_id != self.project_id
                            || candidate.expected_kind != EntityKind::VoiceTake
                            || !candidates.insert(candidate.id)
                        {
                            return Err(invalid_graph(
                                *entity_id,
                                "VoiceTake candidates are not unique exact-project refs".to_owned(),
                            ));
                        }
                        let Some(candidate_entity) = self.entities.get(&candidate.id) else {
                            return Err(invalid_graph(
                                *entity_id,
                                format!("VoiceTake candidate {} is missing", candidate.id),
                            ));
                        };
                        let EntityPayload::VoiceTake(take) = &candidate_entity.payload else {
                            return Err(invalid_graph(
                                *entity_id,
                                format!("candidate {} is not a VoiceTake", candidate.id),
                            ));
                        };
                        if take.locale != slot.locale {
                            return Err(invalid_graph(
                                *entity_id,
                                format!(
                                    "candidate {} locale {} differs from slot locale {}",
                                    candidate.id, take.locale, slot.locale
                                ),
                            ));
                        }
                    }
                    if let Some(selected) = &slot.selected {
                        if selected.project_id != self.project_id
                            || selected.expected_kind != EntityKind::VoiceTake
                            || !candidates.contains(&selected.id)
                        {
                            return Err(invalid_graph(
                                *entity_id,
                                "selected VoiceTake is not an exact candidate".to_owned(),
                            ));
                        }
                        let Some(selected_entity) = self.entities.get(&selected.id) else {
                            return Err(invalid_graph(
                                *entity_id,
                                "selected VoiceTake is missing".to_owned(),
                            ));
                        };
                        if !matches!(&selected_entity.payload, EntityPayload::VoiceTake(_)) {
                            return Err(invalid_graph(
                                *entity_id,
                                "selected entity is not a VoiceTake".to_owned(),
                            ));
                        }
                    }
                    validate_revision3_voice_target_resolution_v1(&slot.target_resolution)
                        .map_err(
                            |reason| ProjectRevision3ValidationError::InvalidVoiceTarget {
                                slot: *entity_id,
                                reason,
                            },
                        )?;
                    if let VoiceTargetResolution::Resolved { target } = &slot.target_resolution {
                        let key = revision3_voice_target_key_v1(target);
                        if let Some(existing_slot) = resolved_targets.insert(key, *entity_id) {
                            return Err(ProjectRevision3ValidationError::DuplicateVoiceTarget {
                                slot: *entity_id,
                                existing_slot,
                            });
                        }
                    }
                }
                EntityPayload::VoiceTake(take) => {
                    let invalid_take =
                        |reason: &str| ProjectRevision3ValidationError::InvalidVoiceTake {
                            take: *entity_id,
                            reason: reason.to_owned(),
                        };
                    if !self.authoring_locales.contains(&take.locale) {
                        return Err(invalid_take("locale is absent from authoring_locales"));
                    }
                    if take.asset.byte_len == 0
                        || take.asset.sha256.as_bytes().iter().all(|byte| *byte == 0)
                        || !valid_voice_logical_name(&take.asset.logical_name)
                    {
                        return Err(invalid_take("asset reference is not closed and bounded"));
                    }
                    let Some(meta) = self.asset_store.assets.get(&take.asset.sha256) else {
                        return Err(invalid_take("asset is absent from asset_store"));
                    };
                    if meta.byte_len != take.asset.byte_len || meta.media_type != "audio/ogg" {
                        return Err(invalid_take(
                            "asset_store metadata is not the exact audio/ogg reference",
                        ));
                    }
                    if take.ogg.channels == 0
                        || take.ogg.sample_rate == 0
                        || take.ogg.pages == 0
                        || take.ogg.logical_streams == 0
                    {
                        return Err(invalid_take("Ogg metadata contains a zero dimension"));
                    }
                }
                _ => {}
            }
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
        self.validate_npc_greetings(npc_id, npc)?;
        Ok(())
    }

    fn validate_npc_greetings(
        &self,
        npc_id: crate::EntityId,
        npc: &crate::model_revision3::NpcDraft,
    ) -> Result<(), ProjectRevision3ValidationError> {
        let invalid = |reason: String| ProjectRevision3ValidationError::InvalidNpcGreetings {
            npc: npc_id,
            reason,
        };
        if npc.greetings.len() > MAX_REVISION3_NPC_GREETING_BINDINGS_V1 {
            return Err(invalid(format!(
                "binding count {} exceeds {}",
                npc.greetings.len(),
                MAX_REVISION3_NPC_GREETING_BINDINGS_V1
            )));
        }
        let mut line_ids = BTreeSet::new();
        for (index, binding) in npc.greetings.iter().enumerate() {
            if binding.line.project_id != self.project_id {
                return Err(invalid(format!(
                    "binding {index} references another project"
                )));
            }
            if binding.line.expected_kind != EntityKind::DialogLine {
                return Err(invalid(format!(
                    "binding {index} does not expect a DialogLine"
                )));
            }
            if !line_ids.insert(binding.line.id) {
                return Err(invalid(format!(
                    "binding {index} duplicates DialogLine {}",
                    binding.line.id
                )));
            }
            let Some(line) = self.entities.get(&binding.line.id) else {
                return Err(invalid(format!(
                    "binding {index} references missing DialogLine {}",
                    binding.line.id
                )));
            };
            if !matches!(line.payload, EntityPayload::DialogLine(_)) {
                return Err(invalid(format!(
                    "binding {index} target {} is not a DialogLine",
                    binding.line.id
                )));
            }
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
                "generator contract is not the revision-3 Quest version 4 contract",
            ));
        }
        validate_draft_quest_objective_titles(
            &quest.input.objective_title,
            &quest.input.additional_objective_titles,
        )
        .map_err(|_| invalid("Quest objective list is not closed and bounded"))?;
        validate_draft_quest_transition_plan_v1(
            &quest.input.transition_plan,
            1 + quest.input.additional_objective_titles.len(),
        )
        .map_err(|_| invalid("Quest transition plan is not closed and bounded"))?;
        self.validate_quest_transcript(quest_id, quest)?;
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
            || module.generator_version != quest.generator_version
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
                    && *generator_version == quest.generator_version
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

    fn validate_quest_transcript(
        &self,
        quest_id: crate::EntityId,
        quest: &crate::model_revision3::QuestDraft,
    ) -> Result<(), ProjectRevision3ValidationError> {
        let invalid = |reason: String| ProjectRevision3ValidationError::InvalidQuestTranscript {
            quest: quest_id,
            reason,
        };
        if quest.transcript.len() > MAX_REVISION3_QUEST_TRANSCRIPT_BINDINGS_V1 {
            return Err(invalid(format!(
                "binding count {} exceeds {}",
                quest.transcript.len(),
                MAX_REVISION3_QUEST_TRANSCRIPT_BINDINGS_V1
            )));
        }
        let active_slots = quest
            .input
            .transition_plan
            .objective_slots
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        let mut line_ids = BTreeSet::new();
        for (index, binding) in quest.transcript.iter().enumerate() {
            if binding.line.project_id != self.project_id {
                return Err(invalid(format!(
                    "binding {index} references another project"
                )));
            }
            if binding.line.expected_kind != EntityKind::DialogLine {
                return Err(invalid(format!(
                    "binding {index} does not expect a DialogLine"
                )));
            }
            if !line_ids.insert(binding.line.id) {
                return Err(invalid(format!(
                    "binding {index} duplicates DialogLine {}",
                    binding.line.id
                )));
            }
            let Some(line) = self.entities.get(&binding.line.id) else {
                return Err(invalid(format!(
                    "binding {index} references missing DialogLine {}",
                    binding.line.id
                )));
            };
            if !matches!(line.payload, EntityPayload::DialogLine(_)) {
                return Err(invalid(format!(
                    "binding {index} target {} is not a DialogLine",
                    binding.line.id
                )));
            }
            match binding.objective_slot {
                Some(slot) if !active_slots.contains(&slot) => {
                    return Err(invalid(format!(
                        "binding {index} targets inactive objective slot {slot}"
                    )));
                }
                _ => {}
            }
        }
        Ok(())
    }
}

fn valid_catalog_component(value: &str, max_bytes: usize) -> bool {
    !value.is_empty()
        && value.len() <= max_bytes
        && value.trim() == value
        && !value.chars().any(char::is_control)
}

fn valid_item_identifier(value: &str, max_bytes: usize) -> bool {
    if value.is_empty() || value.len() > max_bytes || !value.is_ascii() {
        return false;
    }
    let mut bytes = value.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };
    (first == b'_' || first.is_ascii_alphabetic())
        && bytes.all(|byte| byte == b'_' || byte.is_ascii_alphanumeric())
}

fn valid_qualified_item_identifier(value: &str, max_bytes: usize) -> bool {
    !value.is_empty()
        && value.len() <= max_bytes
        && value
            .split("::")
            .all(|segment| valid_item_identifier(segment, max_bytes))
}

fn valid_voice_logical_name(value: &str) -> bool {
    let folded = value.to_ascii_lowercase();
    if value.trim() != value
        || value.len() <= 4
        || value.len() > MAX_REVISION3_VOICE_LOGICAL_NAME_BYTES_V1
        || !folded.ends_with(".ogg")
        || value.chars().any(|character| {
            character.is_control()
                || matches!(
                    character,
                    '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'
                )
        })
    {
        return false;
    }
    let stem = &value[..value.len() - 4];
    if stem.is_empty() || stem == "." || stem == ".." {
        return false;
    }
    let device_stem = stem
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    !matches!(device_stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        && !(device_stem.len() == 4
            && (device_stem.starts_with("COM") || device_stem.starts_with("LPT"))
            && matches!(device_stem.as_bytes()[3], b'1'..=b'9'))
}
