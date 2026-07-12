//! Durable, deployment-independent authoring primitives for GORE Mod Studio.
//!
//! This first phase deliberately models only the concepts needed for a
//! line-centric voice workflow: localization entries, dialog lines, semantic
//! voice slots, and reusable voice takes. It does not imply runtime support for
//! quests, NPCs, or new cooked identities.

mod ids;
mod model;
mod validate;

pub use ids::{EntityId, FixedHexError, ProjectId, Sha256Digest};
pub use model::{
    ArchiveSeal, AssetMeta, AssetRef, AssetStoreIndex, ContentSeal, DialogLine, Entity, EntityKind,
    EntityPayload, FormatV2, GameGenerationAnchor, LocaleCode, LocaleCodeError, LocalizationEntry,
    OggCodec, OggMetadata, OriginRef, ProjectJsonError, ProjectMeta, ProjectV2, SchemaRevisionV1,
    TypedRef, VoiceMemberProof, VoiceOperation, VoiceSlot, VoiceTake, VoiceTakeStatus, VoiceTarget,
    VoiceTargetResolution, MAX_PROJECT_JSON_BYTES,
};
pub use validate::{Diagnostic, DiagnosticCode, DiagnosticSeverity, ValidationProfile};
