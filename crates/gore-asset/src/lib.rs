//! Schema-aware, byte-preserving editing primitives for cooked Unreal assets.
//!
//! Container extraction and repacking stay in `gore-tex`; this crate owns the
//! USMAP, cooked-package carrier, and unversioned-property layers shared by
//! generic DataAsset editors.

pub mod envelope;
pub mod package;
pub mod patch;
pub mod primitive;
pub mod schema;
pub mod span;
pub mod unversioned;

pub use envelope::{
    EnvelopeError, ExportBoundary, ExportEnvelope, ExportSchemaError, ExportSegments,
    LegacyPackageEnvelope, PrimitiveExportEnvelope,
};
pub use package::{
    ComponentDigest, PackageCarrier, PackageComponent, PackageError, PackageLimits, PackagePaths,
    PackageWriteReceipt,
};
pub use patch::{FixedLeafPatch, FixedLeafPatchError, FixedLeafPatchReceipt, PackagePairSeal};
pub use primitive::{
    PrimitiveError, PrimitiveKind, PrimitiveProperty, PrimitivePropertyBlock, PrimitiveValue,
    PropertyPayload,
};
pub use schema::{PropertySlot, SchemaDb, SchemaError, SchemaId, SchemaKind, SchemaRecord};
pub use span::{
    FixedValueSpan, FixedWireKind, MapEntrySpan, MapSection, MapValueSpan, PropertyBlockSpans,
    PropertySpan, PropertySpanWalker, SliceSpan, SpanError, SpanLimits, StructValueSpan, ValueSpan,
};
pub use unversioned::{HeaderEntry, ResolvedHeaderEntry, UnversionedError, UnversionedHeader};
