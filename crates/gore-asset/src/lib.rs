//! Schema-aware, byte-preserving editing primitives for cooked Unreal assets.
//!
//! Container extraction and repacking stay in `gore-tex`; this crate owns the
//! USMAP, cooked-package carrier, and unversioned-property layers shared by
//! generic DataAsset editors.

pub mod dataasset_workflow;
pub mod envelope;
#[path = "offline_pack.rs"]
pub mod legacy_offline_pack;
mod legacy_preflight;
pub mod package;
pub mod patch;
pub mod primitive;
pub mod reviewed_dataasset;
pub mod schema;
pub mod span;
#[cfg(any(test, feature = "test-fixtures"))]
#[doc(hidden)]
pub mod test_fixture;
pub mod unversioned;
mod usmap_preflight;

pub use envelope::{
    EnvelopeError, ExportBoundary, ExportEnvelope, ExportSchemaError, ExportSegments,
    LegacyPackageEnvelope, PrimitiveExportEnvelope,
};
pub use legacy_offline_pack::managed_offline_pack::{
    prepare_managed_reviewed_dataasset_pack_v1, stage_prepared_managed_reviewed_dataasset_pack_v1,
    ManagedReviewedDataAssetPackPublicationUncertainV1, ManagedReviewedDataAssetPackPublicationV1,
    ManagedReviewedDataAssetPackPublishedWithCleanupWarningV1, ManagedReviewedTripletFileSealV1,
    PreparedManagedReviewedDataAssetPackV1, PublishedManagedReviewedDataAssetPackV1,
    PublishedManagedReviewedReceiptSealV1, StagedManagedReviewedDataAssetPackV1,
    VerifiedManagedReviewedTripletPostPackV1,
};
pub use legacy_preflight::{LegacyHeaderLimits, LegacyHeaderPreflightError};
pub use package::{
    ComponentDigest, PackageCarrier, PackageComponent, PackageError, PackageLimits, PackagePaths,
    PackageWriteReceipt,
};
pub use patch::{
    describe_fixed_leaves, FixedLeafDescriptor, FixedLeafInspection, FixedLeafInspectionCounters,
    FixedLeafInspectionError, FixedLeafInspectionLimits, FixedLeafInspectionSession,
    FixedLeafMapKeyIdentity, FixedLeafPatch, FixedLeafPatchError, FixedLeafPatchReceipt,
    FixedLeafRole, FixedLeafSelector, FixedLeafSelectorError, FixedLeafSelectorStep,
    FixedLeafWireType, FixedLeafWorkBudget, FixedLeafWorkLimits, PackagePairSeal,
    FIXED_LEAF_SELECTOR_FORMAT, FIXED_LEAF_SELECTOR_PROFILE,
};
pub use primitive::{
    PrimitiveError, PrimitiveKind, PrimitiveProperty, PrimitivePropertyBlock, PrimitiveValue,
    PropertyPayload,
};
pub use reviewed_dataasset::{
    evaluate_reviewed_dataasset_stage_v1, prepare_reviewed_footstep_preset_size_v1,
    reviewed_footstep_preset_target_from_ids_v1, verify_reviewed_footstep_preset_post_pack_v1,
    ReviewedDataAssetErrorV1, ReviewedDataAssetStageBlockReasonV1,
    ReviewedDataAssetStageEligibilityV1, ReviewedFootstepPresetPostPackErrorV1,
    ReviewedFootstepPresetReplacementV1, ReviewedFootstepPresetSizeV1,
    ReviewedFootstepPresetTargetV1, VerifiedReviewedFootstepPresetPostPackV1,
    REVIEWED_DATAASSET_FORMAT_V1, REVIEWED_FEET_TEXTURE_SIZE_FIELD_ID,
    REVIEWED_FOOTSTEP_PRESET_SCHEMA_ID, REVIEWED_FOOTSTEP_PRESET_SCHEMA_REVISION,
};
pub use schema::{PropertySlot, SchemaDb, SchemaError, SchemaId, SchemaKind, SchemaRecord};
pub use span::{
    FixedValueSpan, FixedWireKind, MapEntrySpan, MapSection, MapValueSpan, PropertyBlockSpans,
    PropertySpan, PropertySpanWalker, SliceSpan, SpanError, SpanLimits, SpanWalkFailure,
    SpanWalkResourceLimits, SpanWalkUsage, StructValueSpan, ValueSpan,
};
pub use unversioned::{
    HeaderDecodeBudget, HeaderDecodeLimits, HeaderDecodeUsage, HeaderEntry, ResolvedHeaderEntry,
    UnversionedError, UnversionedHeader,
};
pub use usmap_preflight::{UsmapLimits, UsmapPreflightError};
