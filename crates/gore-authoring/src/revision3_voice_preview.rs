//! Exact-current, filesystem-free binding for previewing one managed revision-3 Voice take.
//!
//! The binding closes the complete author-facing path from one `DialogLine` through its
//! `LocalizationEntry` and locale-specific `VoiceSlot` to one retained candidate `VoiceTake`.
//! It does not read media, materialize a preview, mutate a project, or grant build, deployment,
//! publication, game-write, save-write, or runtime authority.

use std::io::{self, Write};

use serde::{Deserialize, Serialize};

use crate::model_revision3::{EntityKind, EntityPayload, OggCodec, OggMetadata, VoiceTakeStatus};
use crate::strict_json::reject_duplicate_object_keys;
use crate::{AssetRef, EntityId, LocaleCode, ProjectId, ProjectRevision3, WorkingHead};

pub const MAX_REVISION3_VOICE_TAKE_PREVIEW_REQUEST_JSON_BYTES_V1: usize = 64 * 1024;

/// Complete stale-state binding for one take shown inside one exact line/language context.
///
/// Field order is part of the canonical JSON contract shared with Mod Studio. The full hidden
/// `AssetRef` is deliberately included so a UI catalog cannot silently redirect the operation to
/// a different blob or logical media identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Revision3VoiceTakePreviewRequestV1 {
    pub expected_head: WorkingHead,
    pub expected_project_id: ProjectId,
    pub expected_revision: u64,
    pub line_id: EntityId,
    pub expected_line_revision: u64,
    pub localization_id: EntityId,
    pub expected_localization_revision: u64,
    pub expected_loc_id: String,
    pub slot_id: EntityId,
    pub expected_slot_revision: u64,
    pub locale: LocaleCode,
    pub take_id: EntityId,
    pub expected_take_revision: u64,
    pub expected_asset: AssetRef,
}

impl Revision3VoiceTakePreviewRequestV1 {
    /// Parse only bounded, duplicate-free JSON in its exact canonical spelling.
    pub fn from_json(json: &str) -> Result<Self, Revision3VoiceTakePreviewRequestJsonErrorV1> {
        if json.len() > MAX_REVISION3_VOICE_TAKE_PREVIEW_REQUEST_JSON_BYTES_V1 {
            return Err(Revision3VoiceTakePreviewRequestJsonErrorV1::InputTooLarge {
                actual: json.len(),
                limit: MAX_REVISION3_VOICE_TAKE_PREVIEW_REQUEST_JSON_BYTES_V1,
            });
        }
        reject_duplicate_object_keys(json)
            .map_err(Revision3VoiceTakePreviewRequestJsonErrorV1::InvalidJson)?;
        let request: Self = serde_json::from_str(json)
            .map_err(Revision3VoiceTakePreviewRequestJsonErrorV1::InvalidJson)?;
        if request.to_canonical_json()?.as_bytes() != json.as_bytes() {
            return Err(Revision3VoiceTakePreviewRequestJsonErrorV1::NonCanonicalJson);
        }
        Ok(request)
    }

    pub fn to_canonical_json(&self) -> Result<String, Revision3VoiceTakePreviewRequestJsonErrorV1> {
        let mut writer =
            BoundedRequestWriter::new(MAX_REVISION3_VOICE_TAKE_PREVIEW_REQUEST_JSON_BYTES_V1);
        let serialized = serde_json::to_writer(&mut writer, self);
        if let Some(actual) = writer.first_exceeded_size {
            return Err(Revision3VoiceTakePreviewRequestJsonErrorV1::InputTooLarge {
                actual,
                limit: MAX_REVISION3_VOICE_TAKE_PREVIEW_REQUEST_JSON_BYTES_V1,
            });
        }
        serialized.map_err(Revision3VoiceTakePreviewRequestJsonErrorV1::Serialize)?;
        String::from_utf8(writer.bytes)
            .map_err(|_| Revision3VoiceTakePreviewRequestJsonErrorV1::NonUtf8Serialization)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Revision3VoiceTakePreviewRequestJsonErrorV1 {
    #[error(
        "revision-3 Voice take preview request exceeds the {limit}-byte limit: {actual} bytes"
    )]
    InputTooLarge { actual: usize, limit: usize },
    #[error("invalid revision-3 Voice take preview request JSON: {0}")]
    InvalidJson(#[source] serde_json::Error),
    #[error("could not serialize revision-3 Voice take preview request JSON: {0}")]
    Serialize(#[source] serde_json::Error),
    #[error("revision-3 Voice take preview request is not in its exact canonical spelling")]
    NonCanonicalJson,
    #[error("revision-3 Voice take preview request serializer emitted non-UTF-8 bytes")]
    NonUtf8Serialization,
}

/// Stable semantic stale-state or graph conflict. No variant carries filesystem paths.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Revision3VoiceTakePreviewConflictV1 {
    #[error("request basis head does not match the exact supplied head")]
    CurrentHeadMismatch,
    #[error("expected project {expected}, but exact basis is {actual}")]
    ProjectIdentityMismatch {
        expected: ProjectId,
        actual: ProjectId,
    },
    #[error("expected project revision {expected}, but exact basis is {actual}")]
    ProjectRevisionConflict { expected: u64, actual: u64 },
    #[error("line, localization, slot, and take IDs must be non-zero and pairwise distinct")]
    InvalidEntityIdentity,
    #[error("DialogLine entity {line} is missing or has the wrong kind")]
    InvalidDialogLine { line: EntityId },
    #[error("expected DialogLine revision {expected}, but exact basis is {actual}")]
    DialogLineRevisionConflict { expected: u64, actual: u64 },
    #[error("DialogLine {line} does not reference requested LocalizationEntry {localization}")]
    LocalizationReferenceMismatch {
        line: EntityId,
        localization: EntityId,
    },
    #[error("LocalizationEntry entity {localization} is missing or has the wrong kind")]
    InvalidLocalization { localization: EntityId },
    #[error("expected LocalizationEntry revision {expected}, but exact basis is {actual}")]
    LocalizationRevisionConflict { expected: u64, actual: u64 },
    #[error("expected localization identity differs from the exact LocalizationEntry")]
    LocalizationIdentityMismatch,
    #[error("DialogLine/locale does not reference requested VoiceSlot {slot}")]
    VoiceSlotReferenceMismatch { slot: EntityId },
    #[error("VoiceSlot entity {slot} is missing or has the wrong kind")]
    InvalidVoiceSlot { slot: EntityId },
    #[error("expected VoiceSlot revision {expected}, but exact basis is {actual}")]
    VoiceSlotRevisionConflict { expected: u64, actual: u64 },
    #[error("VoiceSlot {slot} locale differs from the requested locale")]
    VoiceSlotLocaleMismatch { slot: EntityId },
    #[error("VoiceTake {take} is not an exact candidate of requested VoiceSlot {slot}")]
    VoiceTakeNotCandidate { slot: EntityId, take: EntityId },
    #[error("VoiceTake entity {take} is missing or has the wrong kind")]
    InvalidVoiceTake { take: EntityId },
    #[error("expected VoiceTake revision {expected}, but exact basis is {actual}")]
    VoiceTakeRevisionConflict { expected: u64, actual: u64 },
    #[error("VoiceTake {take} locale differs from the requested locale")]
    VoiceTakeLocaleMismatch { take: EntityId },
    #[error("VoiceTake {take} asset differs from the exact expected AssetRef")]
    VoiceTakeAssetMismatch { take: EntityId },
    #[error("VoiceTake {take} asset is absent from the exact project AssetStore")]
    MissingVoiceAsset { take: EntityId },
    #[error("VoiceTake {take} asset metadata is not exact canonical audio/ogg")]
    VoiceAssetMetadataMismatch { take: EntityId },
}

/// Immutable graph facts required by a media-owning adapter after semantic binding succeeds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Revision3VoiceTakePreviewBindingV1 {
    pub basis_head: WorkingHead,
    pub project_id: ProjectId,
    pub project_revision: u64,
    pub line_id: EntityId,
    pub line_revision: u64,
    pub localization_id: EntityId,
    pub localization_revision: u64,
    pub loc_id: String,
    pub slot_id: EntityId,
    pub slot_revision: u64,
    pub locale: LocaleCode,
    pub take_id: EntityId,
    pub take_revision: u64,
    pub asset: AssetRef,
    pub ogg: OggMetadata,
    pub status: VoiceTakeStatus,
}

/// Bind one request to one already-open exact revision-3 project without touching a filesystem.
///
/// Any retained candidate status is accepted, and selection is intentionally irrelevant. This
/// lets an author audition Draft/Recorded/Reviewed/Approved alternatives without granting any
/// approval, build, or runtime qualification. The take's provenance is likewise not restricted,
/// so future recording/transcode origins can use the same exact-current contract.
pub fn bind_revision3_voice_take_preview_v1(
    exact_basis_head: &WorkingHead,
    project: &ProjectRevision3,
    request: &Revision3VoiceTakePreviewRequestV1,
) -> Result<Revision3VoiceTakePreviewBindingV1, Revision3VoiceTakePreviewConflictV1> {
    use Revision3VoiceTakePreviewConflictV1 as Conflict;

    if &request.expected_head != exact_basis_head {
        return Err(Conflict::CurrentHeadMismatch);
    }
    if request.expected_project_id != project.project_id {
        return Err(Conflict::ProjectIdentityMismatch {
            expected: request.expected_project_id,
            actual: project.project_id,
        });
    }
    if request.expected_revision != project.revision {
        return Err(Conflict::ProjectRevisionConflict {
            expected: request.expected_revision,
            actual: project.revision,
        });
    }
    let ids = [
        request.line_id,
        request.localization_id,
        request.slot_id,
        request.take_id,
    ];
    if ids.iter().any(is_zero_entity_id)
        || ids
            .iter()
            .enumerate()
            .any(|(index, id)| ids[index + 1..].contains(id))
    {
        return Err(Conflict::InvalidEntityIdentity);
    }

    let line_entity =
        project
            .entities
            .get(&request.line_id)
            .ok_or(Conflict::InvalidDialogLine {
                line: request.line_id,
            })?;
    let EntityPayload::DialogLine(line) = &line_entity.payload else {
        return Err(Conflict::InvalidDialogLine {
            line: request.line_id,
        });
    };
    if line_entity.revision != request.expected_line_revision {
        return Err(Conflict::DialogLineRevisionConflict {
            expected: request.expected_line_revision,
            actual: line_entity.revision,
        });
    }
    if line.localization.project_id != project.project_id
        || line.localization.id != request.localization_id
        || line.localization.expected_kind != EntityKind::LocalizationEntry
    {
        return Err(Conflict::LocalizationReferenceMismatch {
            line: request.line_id,
            localization: request.localization_id,
        });
    }

    let localization_entity =
        project
            .entities
            .get(&request.localization_id)
            .ok_or(Conflict::InvalidLocalization {
                localization: request.localization_id,
            })?;
    let EntityPayload::LocalizationEntry(localization) = &localization_entity.payload else {
        return Err(Conflict::InvalidLocalization {
            localization: request.localization_id,
        });
    };
    if localization_entity.revision != request.expected_localization_revision {
        return Err(Conflict::LocalizationRevisionConflict {
            expected: request.expected_localization_revision,
            actual: localization_entity.revision,
        });
    }
    if localization.loc_id != request.expected_loc_id {
        return Err(Conflict::LocalizationIdentityMismatch);
    }

    let Some(slot_ref) = line.voice_slots.get(&request.locale) else {
        return Err(Conflict::VoiceSlotReferenceMismatch {
            slot: request.slot_id,
        });
    };
    if slot_ref.project_id != project.project_id
        || slot_ref.id != request.slot_id
        || slot_ref.expected_kind != EntityKind::VoiceSlot
    {
        return Err(Conflict::VoiceSlotReferenceMismatch {
            slot: request.slot_id,
        });
    }
    let slot_entity = project
        .entities
        .get(&request.slot_id)
        .ok_or(Conflict::InvalidVoiceSlot {
            slot: request.slot_id,
        })?;
    let EntityPayload::VoiceSlot(slot) = &slot_entity.payload else {
        return Err(Conflict::InvalidVoiceSlot {
            slot: request.slot_id,
        });
    };
    if slot_entity.revision != request.expected_slot_revision {
        return Err(Conflict::VoiceSlotRevisionConflict {
            expected: request.expected_slot_revision,
            actual: slot_entity.revision,
        });
    }
    if slot.locale != request.locale {
        return Err(Conflict::VoiceSlotLocaleMismatch {
            slot: request.slot_id,
        });
    }
    let is_candidate = slot.candidates.iter().any(|candidate| {
        candidate.project_id == project.project_id
            && candidate.id == request.take_id
            && candidate.expected_kind == EntityKind::VoiceTake
    });
    if !is_candidate {
        return Err(Conflict::VoiceTakeNotCandidate {
            slot: request.slot_id,
            take: request.take_id,
        });
    }

    let take_entity = project
        .entities
        .get(&request.take_id)
        .ok_or(Conflict::InvalidVoiceTake {
            take: request.take_id,
        })?;
    let EntityPayload::VoiceTake(take) = &take_entity.payload else {
        return Err(Conflict::InvalidVoiceTake {
            take: request.take_id,
        });
    };
    if take_entity.revision != request.expected_take_revision {
        return Err(Conflict::VoiceTakeRevisionConflict {
            expected: request.expected_take_revision,
            actual: take_entity.revision,
        });
    }
    if take.locale != request.locale {
        return Err(Conflict::VoiceTakeLocaleMismatch {
            take: request.take_id,
        });
    }
    if take.asset != request.expected_asset {
        return Err(Conflict::VoiceTakeAssetMismatch {
            take: request.take_id,
        });
    }
    let asset_meta =
        project
            .asset_store
            .assets
            .get(&take.asset.sha256)
            .ok_or(Conflict::MissingVoiceAsset {
                take: request.take_id,
            })?;
    if asset_meta.byte_len != take.asset.byte_len || asset_meta.media_type != "audio/ogg" {
        return Err(Conflict::VoiceAssetMetadataMismatch {
            take: request.take_id,
        });
    }

    Ok(Revision3VoiceTakePreviewBindingV1 {
        basis_head: exact_basis_head.clone(),
        project_id: project.project_id,
        project_revision: project.revision,
        line_id: request.line_id,
        line_revision: line_entity.revision,
        localization_id: request.localization_id,
        localization_revision: localization_entity.revision,
        loc_id: localization.loc_id.clone(),
        slot_id: request.slot_id,
        slot_revision: slot_entity.revision,
        locale: request.locale.clone(),
        take_id: request.take_id,
        take_revision: take_entity.revision,
        asset: take.asset.clone(),
        ogg: take.ogg.clone(),
        status: take.status,
    })
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Revision3VoiceTakePreviewOggErrorV1 {
    #[error("managed Voice take is not a bounded valid Vorbis or Opus Ogg stream: {0}")]
    Invalid(String),
    #[error("managed Voice take Ogg metadata exceeds the revision-3 wire range")]
    MetadataLimit,
}

/// Honest strength of the codec validation that produced one exact media-QA result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Revision3VoiceTakeMediaAssuranceV1 {
    /// Every Vorbis packet was decoded through the end of the logical stream and yielded PCM.
    VorbisFullPcmDecode,
    /// Opus packet framing, durations, pre-skip, granule origin, and EOS trim were validated, but
    /// the compressed SILK/CELT payload was not decoded.
    OpusPacketAndTimingStructureOnly,
}

/// Exact rational duration of one Voice take without floating-point rounding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Revision3VoiceTakeMediaDurationV1 {
    sample_frames: u64,
    timebase_hz: u32,
}

impl Revision3VoiceTakeMediaDurationV1 {
    /// Playable frames per channel after codec start/end trimming.
    pub const fn sample_frames(self) -> u64 {
        self.sample_frames
    }

    /// Frames per second for interpreting [`Self::sample_frames`].
    pub const fn timebase_hz(self) -> u32 {
        self.timebase_hz
    }
}

/// Exact media facts derived from one bounded managed Ogg object.
///
/// This is media-input evidence only. It does not assess loudness, clipping, acting, subtitle
/// fit, desktop audibility, build/deployment readiness, or in-game behavior.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Revision3VoiceTakeMediaQaV1 {
    ogg: OggMetadata,
    duration: Revision3VoiceTakeMediaDurationV1,
    assurance: Revision3VoiceTakeMediaAssuranceV1,
}

impl Revision3VoiceTakeMediaQaV1 {
    pub fn ogg(&self) -> &OggMetadata {
        &self.ogg
    }

    pub const fn duration(&self) -> Revision3VoiceTakeMediaDurationV1 {
        self.duration
    }

    pub const fn assurance(&self) -> Revision3VoiceTakeMediaAssuranceV1 {
        self.assurance
    }

    fn into_ogg(self) -> OggMetadata {
        self.ogg
    }
}

/// Backward-compatible error alias for the broader media-QA inspector.
pub type Revision3VoiceTakeMediaQaErrorV1 = Revision3VoiceTakePreviewOggErrorV1;

/// Derive exact revision-3 Ogg metadata, rational duration, and validation assurance.
///
/// Vorbis duration is the validated decoded timeline after PCM origin and EOS trimming. Opus
/// duration is the validated 48 kHz granule duration after origin, pre-skip, and EOS trim. The
/// result remains media-input evidence and never grants audible runtime or deployment
/// qualification.
pub fn inspect_revision3_voice_take_media_qa_v1(
    bytes: &[u8],
) -> Result<Revision3VoiceTakeMediaQaV1, Revision3VoiceTakeMediaQaErrorV1> {
    let validation = gore_vo::validate_ogg_with_timing(bytes, &gore_vo::Limits::default())
        .map_err(|error| Revision3VoiceTakePreviewOggErrorV1::Invalid(error.to_string()))?;
    let info = validation.info;
    let timing = validation.timing;
    let pages = u32::try_from(info.pages)
        .map_err(|_| Revision3VoiceTakePreviewOggErrorV1::MetadataLimit)?;
    let logical_streams = u32::try_from(info.logical_streams)
        .map_err(|_| Revision3VoiceTakePreviewOggErrorV1::MetadataLimit)?;
    if timing.duration_sample_frames == 0 || i64::try_from(timing.duration_sample_frames).is_err() {
        return Err(Revision3VoiceTakePreviewOggErrorV1::MetadataLimit);
    }

    let inconsistent = || {
        Revision3VoiceTakePreviewOggErrorV1::Invalid(
            "validated Ogg duration or decode assurance is internally inconsistent".to_owned(),
        )
    };
    let (codec, channels, sample_rate, assurance) = match info.codec {
        gore_vo::OggCodec::Vorbis {
            channels,
            sample_rate,
        } => {
            if !timing.pcm_decode_complete || timing.duration_timebase_hz != sample_rate {
                return Err(inconsistent());
            }
            (
                OggCodec::Vorbis,
                channels,
                sample_rate,
                Revision3VoiceTakeMediaAssuranceV1::VorbisFullPcmDecode,
            )
        }
        gore_vo::OggCodec::Opus { channels, .. } => {
            if timing.pcm_decode_complete || timing.duration_timebase_hz != 48_000 {
                return Err(inconsistent());
            }
            (
                OggCodec::Opus,
                channels,
                48_000,
                Revision3VoiceTakeMediaAssuranceV1::OpusPacketAndTimingStructureOnly,
            )
        }
        gore_vo::OggCodec::Unknown => {
            return Err(Revision3VoiceTakePreviewOggErrorV1::Invalid(
                "Ogg codec is not Vorbis or Opus".to_owned(),
            ));
        }
    };
    Ok(Revision3VoiceTakeMediaQaV1 {
        ogg: OggMetadata {
            codec,
            channels,
            sample_rate,
            pages,
            logical_streams,
        },
        duration: Revision3VoiceTakeMediaDurationV1 {
            sample_frames: timing.duration_sample_frames,
            timebase_hz: timing.duration_timebase_hz,
        },
        assurance,
    })
}

/// Derive the exact revision-3 metadata expected for preview bytes.
///
/// This compatibility API intentionally retains its original return type and serialized shape.
/// New media-QA callers should use [`inspect_revision3_voice_take_media_qa_v1`].
pub fn inspect_revision3_voice_take_preview_ogg_v1(
    bytes: &[u8],
) -> Result<OggMetadata, Revision3VoiceTakePreviewOggErrorV1> {
    inspect_revision3_voice_take_media_qa_v1(bytes).map(Revision3VoiceTakeMediaQaV1::into_ogg)
}

fn is_zero_entity_id(value: &EntityId) -> bool {
    value.as_bytes().iter().all(|byte| *byte == 0)
}

struct BoundedRequestWriter {
    bytes: Vec<u8>,
    limit: usize,
    first_exceeded_size: Option<usize>,
}

impl BoundedRequestWriter {
    fn new(limit: usize) -> Self {
        Self {
            bytes: Vec::new(),
            limit,
            first_exceeded_size: None,
        }
    }
}

impl Write for BoundedRequestWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        let next = self.bytes.len().saturating_add(buffer.len());
        if next > self.limit {
            self.first_exceeded_size.get_or_insert(next);
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "bounded Voice preview request writer limit exceeded",
            ));
        }
        self.bytes.extend_from_slice(buffer);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use crate::model_revision3::{
        DialogLine, Entity, LocalizationEntry, OriginRef, SchemaRevisionV3, TypedRef, VoiceSlot,
        VoiceTake, VoiceTargetResolution,
    };
    use crate::{
        AssetMeta, AssetStoreIndex, ContentSeal, FormatV2, GameGenerationAnchor, ProjectMeta,
        Sha256Digest, WorkingStoreFormat,
    };

    use super::*;

    const LOC_ID: &str = "GRD_263_ASGHAN_OPEN_INFO_06_02";

    fn id(tag: u8) -> EntityId {
        EntityId::from_bytes([tag; 16])
    }

    fn project_id() -> ProjectId {
        ProjectId::from_bytes([0x31; 16])
    }

    fn locale() -> LocaleCode {
        "de".parse().unwrap()
    }

    fn head() -> WorkingHead {
        WorkingHead {
            store_format: WorkingStoreFormat,
            snapshot: ContentSeal {
                byte_len: 100,
                sha256: Sha256Digest::from_bytes([0x41; 32]),
            },
        }
    }

    fn origin(tag: u8) -> OriginRef {
        OriginRef::Imported {
            importer: "voice-preview-tests".to_owned(),
            source_seal: ContentSeal {
                byte_len: 10,
                sha256: Sha256Digest::from_bytes([tag; 32]),
            },
            external_identity: None,
        }
    }

    fn entity(tag: u8, revision: u64, payload: EntityPayload) -> Entity {
        Entity {
            id: id(tag),
            display_name: format!("entity-{tag}"),
            origin: origin(tag),
            revision,
            payload,
        }
    }

    fn fixture(status: VoiceTakeStatus) -> (ProjectRevision3, Revision3VoiceTakePreviewRequestV1) {
        let localization_id = id(1);
        let line_id = id(2);
        let slot_id = id(3);
        let take_id = id(4);
        let asset = AssetRef {
            sha256: Sha256Digest::from_bytes([0x55; 32]),
            byte_len: 1234,
            logical_name: "asghan_take.ogg".to_owned(),
        };
        let ogg = OggMetadata {
            codec: OggCodec::Vorbis,
            channels: 1,
            sample_rate: 48_000,
            pages: 2,
            logical_streams: 1,
        };
        let project = ProjectRevision3 {
            format: FormatV2,
            schema_revision: SchemaRevisionV3,
            project_id: project_id(),
            revision: 9,
            meta: ProjectMeta {
                name: "preview fixture".to_owned(),
                version: "1.0.0".to_owned(),
                author: "tests".to_owned(),
            },
            target: GameGenerationAnchor {
                executable: ContentSeal {
                    byte_len: 1,
                    sha256: Sha256Digest::from_bytes([0x42; 32]),
                },
            },
            authoring_locales: BTreeSet::from([locale()]),
            entities: BTreeMap::from([
                (
                    localization_id,
                    entity(
                        1,
                        5,
                        EntityPayload::LocalizationEntry(LocalizationEntry {
                            loc_id: LOC_ID.to_owned(),
                            texts: BTreeMap::new(),
                        }),
                    ),
                ),
                (
                    line_id,
                    entity(
                        2,
                        6,
                        EntityPayload::DialogLine(DialogLine {
                            localization: TypedRef::new(
                                project_id(),
                                localization_id,
                                EntityKind::LocalizationEntry,
                            ),
                            speaker_hint: Some("Asghan".to_owned()),
                            voice_slots: BTreeMap::from([(
                                locale(),
                                TypedRef::new(project_id(), slot_id, EntityKind::VoiceSlot),
                            )]),
                        }),
                    ),
                ),
                (
                    slot_id,
                    entity(
                        3,
                        7,
                        EntityPayload::VoiceSlot(VoiceSlot {
                            locale: locale(),
                            target_resolution: VoiceTargetResolution::Unresolved,
                            candidates: vec![TypedRef::new(
                                project_id(),
                                take_id,
                                EntityKind::VoiceTake,
                            )],
                            selected: None,
                        }),
                    ),
                ),
                (
                    take_id,
                    entity(
                        4,
                        8,
                        EntityPayload::VoiceTake(VoiceTake {
                            locale: locale(),
                            asset: asset.clone(),
                            ogg,
                            status,
                        }),
                    ),
                ),
            ]),
            asset_store: AssetStoreIndex {
                assets: BTreeMap::from([(
                    asset.sha256,
                    AssetMeta {
                        byte_len: asset.byte_len,
                        media_type: "audio/ogg".to_owned(),
                    },
                )]),
            },
        };
        let request = Revision3VoiceTakePreviewRequestV1 {
            expected_head: head(),
            expected_project_id: project_id(),
            expected_revision: 9,
            line_id,
            expected_line_revision: 6,
            localization_id,
            expected_localization_revision: 5,
            expected_loc_id: LOC_ID.to_owned(),
            slot_id,
            expected_slot_revision: 7,
            locale: locale(),
            take_id,
            expected_take_revision: 8,
            expected_asset: asset,
        };
        (project, request)
    }

    #[test]
    fn request_json_is_bounded_duplicate_free_and_exact_canonical() {
        let (_, request) = fixture(VoiceTakeStatus::Draft);
        let canonical = request.to_canonical_json().unwrap();
        assert_eq!(
            Revision3VoiceTakePreviewRequestV1::from_json(&canonical).unwrap(),
            request
        );
        assert!(matches!(
            Revision3VoiceTakePreviewRequestV1::from_json(&format!(" {canonical}")),
            Err(Revision3VoiceTakePreviewRequestJsonErrorV1::NonCanonicalJson)
        ));
        let duplicate = canonical.replacen(
            "\"expected_revision\":9",
            "\"expected_revision\":9,\"expected_revision\":9",
            1,
        );
        assert!(matches!(
            Revision3VoiceTakePreviewRequestV1::from_json(&duplicate),
            Err(Revision3VoiceTakePreviewRequestJsonErrorV1::InvalidJson(_))
        ));
        assert!(matches!(
            Revision3VoiceTakePreviewRequestV1::from_json(
                &" ".repeat(MAX_REVISION3_VOICE_TAKE_PREVIEW_REQUEST_JSON_BYTES_V1 + 1)
            ),
            Err(Revision3VoiceTakePreviewRequestJsonErrorV1::InputTooLarge { .. })
        ));
    }

    #[test]
    fn binding_closes_full_graph_and_accepts_every_status_without_selection() {
        for status in [
            VoiceTakeStatus::Draft,
            VoiceTakeStatus::Recorded,
            VoiceTakeStatus::Reviewed,
            VoiceTakeStatus::Approved,
        ] {
            let (project, request) = fixture(status);
            let bound = bind_revision3_voice_take_preview_v1(&head(), &project, &request).unwrap();
            assert_eq!(bound.take_id, request.take_id);
            assert_eq!(bound.asset, request.expected_asset);
            assert_eq!(bound.status, status);
        }
    }

    #[test]
    fn binding_does_not_require_unique_take_ownership() {
        let (mut project, request) = fixture(VoiceTakeStatus::Reviewed);
        let second_line_id = id(5);
        let second_slot_id = id(6);
        project.entities.insert(
            second_line_id,
            entity(
                5,
                1,
                EntityPayload::DialogLine(DialogLine {
                    localization: TypedRef::new(
                        project_id(),
                        request.localization_id,
                        EntityKind::LocalizationEntry,
                    ),
                    speaker_hint: Some("Other speaker".to_owned()),
                    voice_slots: BTreeMap::from([(
                        locale(),
                        TypedRef::new(project_id(), second_slot_id, EntityKind::VoiceSlot),
                    )]),
                }),
            ),
        );
        project.entities.insert(
            second_slot_id,
            entity(
                6,
                1,
                EntityPayload::VoiceSlot(VoiceSlot {
                    locale: locale(),
                    target_resolution: VoiceTargetResolution::Unresolved,
                    candidates: vec![TypedRef::new(
                        project_id(),
                        request.take_id,
                        EntityKind::VoiceTake,
                    )],
                    selected: None,
                }),
            ),
        );
        let canonical = project.to_canonical_json().unwrap();
        let project = ProjectRevision3::from_json(&canonical).unwrap();
        assert!(bind_revision3_voice_take_preview_v1(&head(), &project, &request).is_ok());
    }

    #[test]
    fn binding_rejects_every_stale_graph_layer_and_asset_drift() {
        let (project, request) = fixture(VoiceTakeStatus::Recorded);

        let mut stale = request.clone();
        stale.expected_head.snapshot.byte_len += 1;
        assert!(matches!(
            bind_revision3_voice_take_preview_v1(&head(), &project, &stale),
            Err(Revision3VoiceTakePreviewConflictV1::CurrentHeadMismatch)
        ));
        let mut stale = request.clone();
        stale.expected_project_id = ProjectId::from_bytes([0x99; 16]);
        assert!(matches!(
            bind_revision3_voice_take_preview_v1(&head(), &project, &stale),
            Err(Revision3VoiceTakePreviewConflictV1::ProjectIdentityMismatch { .. })
        ));
        let mut stale = request.clone();
        stale.expected_line_revision -= 1;
        assert!(matches!(
            bind_revision3_voice_take_preview_v1(&head(), &project, &stale),
            Err(Revision3VoiceTakePreviewConflictV1::DialogLineRevisionConflict { .. })
        ));
        let mut stale = request.clone();
        stale.expected_localization_revision -= 1;
        assert!(matches!(
            bind_revision3_voice_take_preview_v1(&head(), &project, &stale),
            Err(Revision3VoiceTakePreviewConflictV1::LocalizationRevisionConflict { .. })
        ));
        let mut stale = request.clone();
        stale.expected_slot_revision -= 1;
        assert!(matches!(
            bind_revision3_voice_take_preview_v1(&head(), &project, &stale),
            Err(Revision3VoiceTakePreviewConflictV1::VoiceSlotRevisionConflict { .. })
        ));
        let mut stale = request.clone();
        stale.expected_take_revision -= 1;
        assert!(matches!(
            bind_revision3_voice_take_preview_v1(&head(), &project, &stale),
            Err(Revision3VoiceTakePreviewConflictV1::VoiceTakeRevisionConflict { .. })
        ));
        let mut stale = request;
        stale.expected_asset.logical_name = "different.ogg".to_owned();
        assert!(matches!(
            bind_revision3_voice_take_preview_v1(&head(), &project, &stale),
            Err(Revision3VoiceTakePreviewConflictV1::VoiceTakeAssetMismatch { .. })
        ));
    }

    #[test]
    fn media_qa_reports_exact_real_vorbis_and_opus_timing_and_assurance() {
        let vorbis = inspect_revision3_voice_take_media_qa_v1(include_bytes!(
            "../../gore-vo/testdata/tiny-vorbis.ogg"
        ))
        .unwrap();
        assert_eq!(vorbis.ogg().codec, OggCodec::Vorbis);
        assert_eq!(vorbis.ogg().sample_rate, 48_000);
        assert_eq!(vorbis.duration().sample_frames(), 3_840);
        assert_eq!(vorbis.duration().timebase_hz(), 48_000);
        assert_eq!(
            vorbis.assurance(),
            Revision3VoiceTakeMediaAssuranceV1::VorbisFullPcmDecode
        );

        let opus = inspect_revision3_voice_take_media_qa_v1(include_bytes!(
            "../../gore-vo/testdata/tiny-opus.ogg"
        ))
        .unwrap();
        assert_eq!(opus.ogg().codec, OggCodec::Opus);
        assert_eq!(opus.ogg().sample_rate, 48_000);
        assert_eq!(opus.duration().sample_frames(), 3_840);
        assert_eq!(opus.duration().timebase_hz(), 48_000);
        assert_eq!(
            opus.assurance(),
            Revision3VoiceTakeMediaAssuranceV1::OpusPacketAndTimingStructureOnly
        );
    }

    #[test]
    fn preview_ogg_inspector_remains_a_metadata_only_compatibility_wrapper() {
        let bytes = include_bytes!("../../gore-vo/testdata/tiny-vorbis.ogg");
        let media = inspect_revision3_voice_take_media_qa_v1(bytes).unwrap();
        let metadata = inspect_revision3_voice_take_preview_ogg_v1(bytes).unwrap();
        assert_eq!(&metadata, media.ogg());
        assert_eq!(
            serde_json::to_value(&metadata).unwrap(),
            serde_json::json!({
                "codec": "vorbis",
                "channels": 1,
                "sample_rate": 48_000,
                "pages": 3,
                "logical_streams": 1,
            })
        );
        assert_eq!(
            serde_json::to_value(&media).unwrap(),
            serde_json::json!({
                "ogg": {
                    "codec": "vorbis",
                    "channels": 1,
                    "sample_rate": 48_000,
                    "pages": 3,
                    "logical_streams": 1,
                },
                "duration": {
                    "sample_frames": 3_840,
                    "timebase_hz": 48_000,
                },
                "assurance": "vorbis_full_pcm_decode",
            })
        );
        assert!(matches!(
            inspect_revision3_voice_take_media_qa_v1(b"not ogg"),
            Err(Revision3VoiceTakePreviewOggErrorV1::Invalid(_))
        ));
        assert!(matches!(
            inspect_revision3_voice_take_preview_ogg_v1(b"not ogg"),
            Err(Revision3VoiceTakePreviewOggErrorV1::Invalid(_))
        ));
    }
}
