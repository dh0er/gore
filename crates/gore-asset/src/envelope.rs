//! Version-gated export boundaries for retoc's split legacy package form.
//!
//! This layer parses only the `.uasset` header and validates the export ranges
//! it points at in `.uexp`. It never invents a property offset. For an
//! unversioned UObject export, a property codec is invoked at byte zero of the
//! exact export envelope; the byte count returned by that codec is then used to
//! retain any class-native suffix opaquely.

use std::any::Any;
use std::collections::HashSet;
use std::io::Cursor;
use std::panic::{catch_unwind, AssertUnwindSafe};

use retoc::legacy_asset::{EPackageFlags, FLegacyPackageFileSummary, FLegacyPackageHeader};
use retoc::version::EngineVersion;
use retoc::zen::FPackageIndex;
use thiserror::Error;

use crate::{
    PackageCarrier, PackageComponent, PrimitiveError, PrimitivePropertyBlock, PropertySlot,
    SchemaDb, SchemaError, SchemaId,
};

const PACKAGE_FOOTER_BYTES: usize = 4;
const MAX_CLASS_OUTER_DEPTH: usize = 128;

/// One export range resolved from the cooked legacy header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportBoundary {
    export_index: usize,
    object_name: String,
    class_path: String,
    local_generated_class: bool,
    component: PackageComponent,
    offset: usize,
    length: usize,
}

impl ExportBoundary {
    pub fn export_index(&self) -> usize {
        self.export_index
    }

    pub fn object_name(&self) -> &str {
        &self.object_name
    }

    /// Exact class path obtained from this export's package-map class index.
    pub fn class_path(&self) -> &str {
        &self.class_path
    }

    /// Bind the exact export class to a class schema.
    ///
    /// A locally exported generated class is never replaced by one of its
    /// native parents. If its `_C` schema is absent, callers receive the typed
    /// `LocalGeneratedClassSchemaMissing` error instead.
    pub fn resolve_class_schema(&self, schemas: &SchemaDb) -> Result<SchemaId, ExportSchemaError> {
        match schemas.resolve_class(&self.class_path) {
            Ok(schema_id) => Ok(schema_id),
            Err(SchemaError::SchemaNotFound { .. }) if self.local_generated_class => {
                Err(ExportSchemaError::LocalGeneratedClassSchemaMissing {
                    export_index: self.export_index,
                    class_path: self.class_path.clone(),
                })
            }
            Err(source) => Err(ExportSchemaError::Schema {
                export_index: self.export_index,
                class_path: self.class_path.clone(),
                source,
            }),
        }
    }

    pub fn component(&self) -> PackageComponent {
        self.component
    }

    pub fn offset(&self) -> usize {
        self.offset
    }

    pub fn length(&self) -> usize {
        self.length
    }

    pub fn end(&self) -> usize {
        self.offset
            .checked_add(self.length)
            .expect("export boundary was validated when constructed")
    }
}

/// Parsed UE5.4 G1R package header plus its validated `.uexp` export ranges.
///
/// The profile is intentionally fixed: G1R's packages are unversioned, so
/// retoc needs an explicit UE5.4 fallback to interpret the legacy header.
#[derive(Debug)]
pub struct LegacyPackageEnvelope<'a> {
    carrier: &'a PackageCarrier,
    cooked_header_size: usize,
    uexp_data_length: usize,
    exports: Vec<ExportBoundary>,
}

impl<'a> LegacyPackageEnvelope<'a> {
    /// Parse the retoc Zen-to-Legacy representation used by the current G1R build.
    ///
    /// This rejects versioned packages, headers whose cooked size differs from
    /// `.uasset`, missing/wrong `.uexp` footer magic, invalid ranges, and
    /// overlapping exports. Gaps are retained: non-export package data may
    /// legally exist before the footer.
    pub fn parse_g1r_ue5_4(carrier: &'a PackageCarrier) -> Result<Self, EnvelopeError> {
        let uasset = carrier.bytes(PackageComponent::Uasset);
        let uexp = carrier.bytes(PackageComponent::Uexp);
        let fallback = EngineVersion::UE5_4.package_file_version();
        let parsed = catch_unwind(AssertUnwindSafe(|| {
            FLegacyPackageHeader::deserialize(&mut Cursor::new(uasset), Some(fallback))
        }))
        .map_err(|panic| EnvelopeError::ParserPanic(panic_message(panic)))?
        .map_err(|error| EnvelopeError::HeaderParse(error.to_string()))?;

        if !parsed.summary.versioning_info.is_unversioned
            || !parsed.summary.uses_unversioned_property_serialization()
        {
            return Err(EnvelopeError::NotUnversioned);
        }
        if !parsed.summary.has_package_flags(EPackageFlags::Cooked) {
            return Err(EnvelopeError::NotCooked);
        }

        let cooked_header_size = usize::try_from(parsed.summary.versioning_info.total_header_size)
            .map_err(|_| EnvelopeError::InvalidCookedHeaderSize {
                value: parsed.summary.versioning_info.total_header_size,
            })?;
        if cooked_header_size != uasset.len() {
            return Err(EnvelopeError::HeaderLengthMismatch {
                advertised: cooked_header_size,
                actual: uasset.len(),
            });
        }

        let advertised_exports = usize::try_from(parsed.summary.exports.count).map_err(|_| {
            EnvelopeError::InvalidExportCount {
                value: parsed.summary.exports.count,
            }
        })?;
        if advertised_exports != parsed.exports.len() {
            return Err(EnvelopeError::ExportCountMismatch {
                advertised: advertised_exports,
                parsed: parsed.exports.len(),
            });
        }

        if uexp.len() < PACKAGE_FOOTER_BYTES {
            return Err(EnvelopeError::MissingPackageFooter { length: uexp.len() });
        }
        let uexp_data_length = uexp.len() - PACKAGE_FOOTER_BYTES;
        let expected_footer = FLegacyPackageFileSummary::PACKAGE_FILE_TAG.to_le_bytes();
        let actual_footer: [u8; PACKAGE_FOOTER_BYTES] = uexp[uexp_data_length..]
            .try_into()
            .expect("four-byte suffix checked above");
        if actual_footer != expected_footer {
            return Err(EnvelopeError::PackageFooterMismatch {
                expected: expected_footer,
                actual: actual_footer,
            });
        }

        let cooked_header_i64 =
            i64::try_from(cooked_header_size).map_err(|_| EnvelopeError::RangeArithmetic)?;
        let mut exports = Vec::with_capacity(parsed.exports.len());
        for (export_index, export) in parsed.exports.iter().enumerate() {
            let relative = export
                .serial_offset
                .checked_sub(cooked_header_i64)
                .ok_or(EnvelopeError::RangeArithmetic)?;
            let offset =
                usize::try_from(relative).map_err(|_| EnvelopeError::InvalidExportRange {
                    export_index,
                    serial_offset: export.serial_offset,
                    serial_size: export.serial_size,
                    cooked_header_size,
                })?;
            let length = usize::try_from(export.serial_size).map_err(|_| {
                EnvelopeError::InvalidExportRange {
                    export_index,
                    serial_offset: export.serial_offset,
                    serial_size: export.serial_size,
                    cooked_header_size,
                }
            })?;
            let end = offset
                .checked_add(length)
                .ok_or(EnvelopeError::RangeArithmetic)?;
            if end > uexp_data_length {
                return Err(EnvelopeError::ExportOutOfBounds {
                    export_index,
                    offset,
                    end,
                    uexp_data_length,
                });
            }
            let object_name = parsed
                .name_map
                .get(export.object_name)
                .map_err(|error| EnvelopeError::ObjectName {
                    export_index,
                    message: error.to_string(),
                })?
                .into_owned();
            let resolved_class = resolve_export_class(&parsed, export_index)?;
            exports.push(ExportBoundary {
                export_index,
                object_name,
                class_path: resolved_class.path,
                local_generated_class: resolved_class.local_generated,
                component: PackageComponent::Uexp,
                offset,
                length,
            });
        }

        let mut ordered: Vec<_> = exports
            .iter()
            .map(|boundary| (boundary.offset, boundary.end(), boundary.export_index))
            .collect();
        ordered.sort_unstable();
        for pair in ordered.windows(2) {
            if pair[0].1 > pair[1].0 {
                return Err(EnvelopeError::OverlappingExports {
                    first: pair[0].2,
                    second: pair[1].2,
                });
            }
        }

        Ok(Self {
            carrier,
            cooked_header_size,
            uexp_data_length,
            exports,
        })
    }

    pub fn cooked_header_size(&self) -> usize {
        self.cooked_header_size
    }

    /// Number of `.uexp` bytes before the four-byte package footer.
    pub fn uexp_data_length(&self) -> usize {
        self.uexp_data_length
    }

    pub fn exports(&self) -> &[ExportBoundary] {
        &self.exports
    }

    pub fn export(&self, export_index: usize) -> Result<ExportEnvelope<'_>, EnvelopeError> {
        let boundary =
            self.exports
                .get(export_index)
                .ok_or(EnvelopeError::ExportIndexOutOfRange {
                    export_index,
                    export_count: self.exports.len(),
                })?;
        let bytes = self
            .carrier
            .slice(boundary.component, boundary.offset, boundary.length)
            .map_err(|error| EnvelopeError::Carrier(error.to_string()))?;
        Ok(ExportEnvelope { boundary, bytes })
    }
}

/// Borrowed bytes for exactly one header-proven export.
#[derive(Debug, Clone, Copy)]
pub struct ExportEnvelope<'a> {
    boundary: &'a ExportBoundary,
    bytes: &'a [u8],
}

impl<'a> ExportEnvelope<'a> {
    pub fn boundary(&self) -> &ExportBoundary {
        self.boundary
    }

    pub fn bytes(&self) -> &'a [u8] {
        self.bytes
    }

    /// Split a property prefix only after a schema-aware decoder has returned
    /// its exact consumed byte count. The remainder is retained as opaque
    /// class-native data; this method never assumes a fixed suffix size.
    pub fn split_decoded_prefix(
        &self,
        consumed: usize,
    ) -> Result<ExportSegments<'a>, EnvelopeError> {
        if consumed > self.bytes.len() {
            return Err(EnvelopeError::DecodedPrefixOutOfBounds {
                consumed,
                export_length: self.bytes.len(),
            });
        }
        Ok(ExportSegments {
            decoded_prefix: &self.bytes[..consumed],
            native_suffix: &self.bytes[consumed..],
        })
    }

    /// Decode a primitive-only unversioned property block at the exact export
    /// start and retain the class-native suffix byte-for-byte.
    ///
    /// Callers must select a UObject export whose serialization starts with its
    /// script-property stream. Complex non-zero properties remain a hard error;
    /// no payload length is guessed.
    pub fn decode_primitive_properties(
        &self,
        slots: &[PropertySlot],
    ) -> Result<PrimitiveExportEnvelope<'a>, EnvelopeError> {
        let (properties, consumed) = PrimitivePropertyBlock::decode(self.bytes, slots)?;
        let segments = self.split_decoded_prefix(consumed)?;
        Ok(PrimitiveExportEnvelope {
            properties,
            encoded_properties: segments.decoded_prefix,
            native_suffix: segments.native_suffix,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExportSegments<'a> {
    pub decoded_prefix: &'a [u8],
    pub native_suffix: &'a [u8],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrimitiveExportEnvelope<'a> {
    pub properties: PrimitivePropertyBlock,
    pub encoded_properties: &'a [u8],
    pub native_suffix: &'a [u8],
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ExportSchemaError {
    #[error(
        "export {export_index} uses local generated class {class_path:?}, but that exact class schema is missing; refusing a parent-class fallback"
    )]
    LocalGeneratedClassSchemaMissing {
        export_index: usize,
        class_path: String,
    },
    #[error("export {export_index} class {class_path:?} cannot be bound to USMAP: {source}")]
    Schema {
        export_index: usize,
        class_path: String,
        #[source]
        source: SchemaError,
    },
}

#[derive(Debug)]
struct ResolvedExportClass {
    path: String,
    local_generated: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum PackageObjectRef {
    Import(usize),
    Export(usize),
}

impl PackageObjectRef {
    fn kind(self) -> &'static str {
        match self {
            Self::Import(_) => "import",
            Self::Export(_) => "export",
        }
    }

    fn index(self) -> usize {
        match self {
            Self::Import(index) | Self::Export(index) => index,
        }
    }
}

#[derive(Debug)]
struct ResolvedObjectNode {
    object_ref: PackageObjectRef,
    name: String,
}

fn resolve_export_class(
    package: &FLegacyPackageHeader,
    export_index: usize,
) -> Result<ResolvedExportClass, EnvelopeError> {
    let export = package
        .exports
        .get(export_index)
        .ok_or(EnvelopeError::ExportIndexOutOfRange {
            export_index,
            export_count: package.exports.len(),
        })?;
    let Some(mut current) = checked_package_object_ref(package, export_index, export.class_index)?
    else {
        return Err(EnvelopeError::NullExportClass { export_index });
    };
    let class_is_local_export = matches!(current, PackageObjectRef::Export(_));
    let mut seen = HashSet::new();
    let mut leaf_to_root = Vec::new();

    for _ in 0..MAX_CLASS_OUTER_DEPTH {
        if !seen.insert(current) {
            return Err(EnvelopeError::ClassOuterCycle {
                export_index,
                referenced_kind: current.kind(),
                referenced_index: current.index(),
            });
        }

        let (name_index, outer_index) = match current {
            PackageObjectRef::Import(index) => {
                let object = &package.imports[index];
                (object.object_name, object.outer_index)
            }
            PackageObjectRef::Export(index) => {
                let object = &package.exports[index];
                (object.object_name, object.outer_index)
            }
        };
        let name = package
            .name_map
            .get(name_index)
            .map_err(|error| EnvelopeError::ClassObjectName {
                export_index,
                referenced_kind: current.kind(),
                referenced_index: current.index(),
                message: error.to_string(),
            })?
            .into_owned();
        if name.is_empty() {
            return Err(EnvelopeError::InvalidClassPath {
                export_index,
                reason: format!(
                    "{} {} has an empty object name",
                    current.kind(),
                    current.index()
                ),
            });
        }
        leaf_to_root.push(ResolvedObjectNode {
            object_ref: current,
            name,
        });

        let Some(outer) = checked_package_object_ref(package, export_index, outer_index)? else {
            let path = build_class_path(package, export_index, &leaf_to_root)?;
            let local_generated = class_is_local_export
                && leaf_to_root
                    .first()
                    .is_some_and(|node| node.name.ends_with("_C"));
            return Ok(ResolvedExportClass {
                path,
                local_generated,
            });
        };
        current = outer;
    }

    Err(EnvelopeError::ClassOuterDepthExceeded {
        export_index,
        limit: MAX_CLASS_OUTER_DEPTH,
    })
}

fn checked_package_object_ref(
    package: &FLegacyPackageHeader,
    export_index: usize,
    package_index: FPackageIndex,
) -> Result<Option<PackageObjectRef>, EnvelopeError> {
    let raw = i64::from(package_index.index);
    let (kind, index, count) = if raw > 0 {
        (
            "export",
            usize::try_from(raw - 1).map_err(|_| EnvelopeError::RangeArithmetic)?,
            package.exports.len(),
        )
    } else if raw < 0 {
        // Work in i64 so even an attacker-controlled i32::MIN is ordinary
        // bounded input. retoc's `to_import_index` asserts and negates i32.
        (
            "import",
            usize::try_from(-raw - 1).map_err(|_| EnvelopeError::RangeArithmetic)?,
            package.imports.len(),
        )
    } else {
        return Ok(None);
    };

    if index >= count {
        return Err(EnvelopeError::ClassObjectOutOfBounds {
            export_index,
            referenced_kind: kind,
            referenced_index: index,
            object_count: count,
        });
    }
    Ok(Some(if raw > 0 {
        PackageObjectRef::Export(index)
    } else {
        PackageObjectRef::Import(index)
    }))
}

fn build_class_path(
    package: &FLegacyPackageHeader,
    export_index: usize,
    leaf_to_root: &[ResolvedObjectNode],
) -> Result<String, EnvelopeError> {
    let root = leaf_to_root
        .last()
        .ok_or_else(|| EnvelopeError::InvalidClassPath {
            export_index,
            reason: "empty outer chain".to_owned(),
        })?;
    let mut object_names: Vec<&str> = leaf_to_root
        .iter()
        .rev()
        .map(|node| node.name.as_str())
        .collect();

    let module = match root.object_ref {
        PackageObjectRef::Import(_) => {
            if object_names.len() < 2 || !root.name.starts_with('/') {
                return Err(EnvelopeError::InvalidClassPath {
                    export_index,
                    reason: format!(
                        "import outer root {:?} does not identify a package plus class",
                        root.name
                    ),
                });
            }
            object_names.remove(0)
        }
        PackageObjectRef::Export(_) => {
            if package.summary.package_name.is_empty() {
                return Err(EnvelopeError::InvalidClassPath {
                    export_index,
                    reason: "local class has an empty package name".to_owned(),
                });
            }
            package.summary.package_name.as_str()
        }
    };

    Ok(format!("{module}.{}", object_names.join(".")))
}

#[derive(Debug, Error)]
pub enum EnvelopeError {
    #[error("retoc panicked while parsing the legacy package header: {0}")]
    ParserPanic(String),
    #[error("could not parse the UE5.4 legacy package header: {0}")]
    HeaderParse(String),
    #[error("package is not an unversioned cooked-property package")]
    NotUnversioned,
    #[error("package does not carry the cooked-package flag")]
    NotCooked,
    #[error("invalid negative cooked header size {value}")]
    InvalidCookedHeaderSize { value: i32 },
    #[error("cooked header advertises {advertised} bytes, but .uasset has {actual} bytes")]
    HeaderLengthMismatch { advertised: usize, actual: usize },
    #[error("invalid negative export count {value}")]
    InvalidExportCount { value: i32 },
    #[error("header advertises {advertised} exports, but retoc parsed {parsed}")]
    ExportCountMismatch { advertised: usize, parsed: usize },
    #[error(".uexp has only {length} bytes and cannot contain the package footer")]
    MissingPackageFooter { length: usize },
    #[error(".uexp package footer mismatch: expected {expected:02x?}, got {actual:02x?}")]
    PackageFooterMismatch { expected: [u8; 4], actual: [u8; 4] },
    #[error(
        "export {export_index} has invalid serial range offset={serial_offset}, size={serial_size}, cooked_header={cooked_header_size}"
    )]
    InvalidExportRange {
        export_index: usize,
        serial_offset: i64,
        serial_size: i64,
        cooked_header_size: usize,
    },
    #[error(
        "export {export_index} range {offset}..{end} exceeds the {uexp_data_length}-byte .uexp data region"
    )]
    ExportOutOfBounds {
        export_index: usize,
        offset: usize,
        end: usize,
        uexp_data_length: usize,
    },
    #[error("exports {first} and {second} overlap")]
    OverlappingExports { first: usize, second: usize },
    #[error("export {export_index} has an invalid object name: {message}")]
    ObjectName {
        export_index: usize,
        message: String,
    },
    #[error("export index {export_index} is outside the {export_count}-export package")]
    ExportIndexOutOfRange {
        export_index: usize,
        export_count: usize,
    },
    #[error("export {export_index} has a null class reference")]
    NullExportClass { export_index: usize },
    #[error(
        "export {export_index} class outer chain references {referenced_kind} {referenced_index}, but that map has only {object_count} entries"
    )]
    ClassObjectOutOfBounds {
        export_index: usize,
        referenced_kind: &'static str,
        referenced_index: usize,
        object_count: usize,
    },
    #[error(
        "export {export_index} class outer chain cycles at {referenced_kind} {referenced_index}"
    )]
    ClassOuterCycle {
        export_index: usize,
        referenced_kind: &'static str,
        referenced_index: usize,
    },
    #[error("export {export_index} class outer chain exceeds the {limit}-object safety limit")]
    ClassOuterDepthExceeded { export_index: usize, limit: usize },
    #[error(
        "export {export_index} class outer chain {referenced_kind} {referenced_index} has an invalid object name: {message}"
    )]
    ClassObjectName {
        export_index: usize,
        referenced_kind: &'static str,
        referenced_index: usize,
        message: String,
    },
    #[error("export {export_index} class path is invalid: {reason}")]
    InvalidClassPath { export_index: usize, reason: String },
    #[error("decoded prefix length {consumed} exceeds export length {export_length}")]
    DecodedPrefixOutOfBounds {
        consumed: usize,
        export_length: usize,
    },
    #[error("package range arithmetic overflowed")]
    RangeArithmetic,
    #[error("package carrier rejected a validated export range: {0}")]
    Carrier(String),
    #[error(transparent)]
    Primitive(#[from] PrimitiveError),
}

fn panic_message(panic: Box<dyn Any + Send>) -> String {
    if let Some(message) = panic.downcast_ref::<&str>() {
        (*message).to_owned()
    } else if let Some(message) = panic.downcast_ref::<String>() {
        message.clone()
    } else {
        "non-string panic payload".to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use retoc::legacy_asset::{FMinimalName, FObjectExport, FObjectImport};
    use retoc::logging::Log;

    fn bool_slot(schema_index: usize, name: &str) -> PropertySlot {
        PropertySlot {
            schema_index,
            property_name: name.to_owned(),
            array_index: 0,
            array_dimension: 1,
            inner: usmap::PropertyInner::Bool,
            declaring_schema_id: 0,
            declaring_schema_name: "Fixture".to_owned(),
            declaring_module_path: Some("/Script/Test".to_owned()),
        }
    }

    fn package_with_exports(exports: &[(&str, &[u8], i64)]) -> PackageCarrier {
        package_with_exports_and_flags(
            exports,
            EPackageFlags::Cooked as u32
                | EPackageFlags::FilterEditorOnly as u32
                | EPackageFlags::UsesUnversionedProperties as u32,
        )
    }

    fn package_with_exports_and_flags(
        exports: &[(&str, &[u8], i64)],
        package_flags: u32,
    ) -> PackageCarrier {
        package_with_exports_configured(exports, package_flags, |_| {})
    }

    fn package_with_exports_configured(
        exports: &[(&str, &[u8], i64)],
        package_flags: u32,
        configure: impl FnOnce(&mut FLegacyPackageHeader),
    ) -> PackageCarrier {
        let mut package = FLegacyPackageHeader::default();
        package.summary.versioning_info.package_file_version =
            EngineVersion::UE5_4.package_file_version();
        package.summary.versioning_info.is_unversioned = true;
        package.summary.package_name = "/Game/EnvelopeFixture".to_owned();
        package.summary.package_flags = package_flags;

        let class_index = add_imported_class(&mut package, "/Script/Test", "Fixture");

        let mut uexp_data = Vec::new();
        for (name, bytes, relative_offset) in exports {
            let object_name = package.name_map.store(name);
            package.exports.push(FObjectExport {
                class_index,
                object_name,
                serial_offset: *relative_offset,
                serial_size: bytes.len() as i64,
                ..FObjectExport::default()
            });
            let offset = usize::try_from(*relative_offset).unwrap();
            if uexp_data.len() < offset + bytes.len() {
                uexp_data.resize(offset + bytes.len(), 0);
            }
            uexp_data[offset..offset + bytes.len()].copy_from_slice(bytes);
        }
        configure(&mut package);

        let mut uasset = Cursor::new(Vec::new());
        package
            .serialize(&mut uasset, None, &Log::no_log())
            .unwrap();
        uexp_data.extend_from_slice(&FLegacyPackageFileSummary::PACKAGE_FILE_TAG.to_le_bytes());
        PackageCarrier::from_bytes(
            uasset.into_inner(),
            uexp_data,
            crate::PackageLimits::default(),
        )
        .unwrap()
    }

    fn add_imported_class(
        package: &mut FLegacyPackageHeader,
        module: &str,
        class: &str,
    ) -> FPackageIndex {
        let core_uobject = package.name_map.store("/Script/CoreUObject");
        let package_class = package.name_map.store("Package");
        let class_class = package.name_map.store("Class");
        let module_name = package.name_map.store(module);
        let class_name = package.name_map.store(class);

        let module_index = package.imports.len();
        package.imports.push(FObjectImport {
            class_package: core_uobject,
            class_name: package_class,
            object_name: module_name,
            ..FObjectImport::default()
        });
        let class_index = package.imports.len();
        package.imports.push(FObjectImport {
            class_package: core_uobject,
            class_name: class_class,
            outer_index: FPackageIndex::create_import(module_index as u32),
            object_name: class_name,
            ..FObjectImport::default()
        });
        FPackageIndex::create_import(class_index as u32)
    }

    fn schema_db(entries: &[(&str, &str, usmap::FlagsType)]) -> SchemaDb {
        let schemas = entries
            .iter()
            .map(|(name, _, _)| usmap::Struct {
                name: (*name).to_owned(),
                super_struct: None,
                properties: Vec::new(),
            })
            .collect();
        let modules = entries
            .iter()
            .map(|(_, module, _)| (*module).to_owned())
            .collect();
        let flags = entries
            .iter()
            .map(|(_, _, kind)| usmap::StructFlags {
                type_: *kind,
                value: 0,
                prop_flags: Vec::new(),
            })
            .collect();
        SchemaDb::from_parsed(usmap::Usmap {
            enums: Vec::new(),
            structs: schemas,
            cext: None,
            ppth: Some(usmap::ExtPpth {
                version: 0,
                enums: Vec::new(),
                structs: modules,
            }),
            eatr: Some(usmap::ExtEatr {
                version: 0,
                enum_flags: Vec::new(),
                struct_flags: flags,
            }),
            envp: None,
        })
        .unwrap()
    }

    fn cooked_flags() -> u32 {
        EPackageFlags::Cooked as u32
            | EPackageFlags::FilterEditorOnly as u32
            | EPackageFlags::UsesUnversionedProperties as u32
    }

    #[test]
    fn parses_boundary_and_preserves_empty_fragments_and_native_suffix() {
        // Two legal empty fragments, a final two-value fragment, two bool
        // payload bytes, then class-native bytes outside the property block.
        let export = [0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x01, 0x00, 0xaa, 0xbb];
        let carrier = package_with_exports(&[("Asset", &export, 0)]);
        let package = LegacyPackageEnvelope::parse_g1r_ue5_4(&carrier).unwrap();

        assert_eq!(
            package.cooked_header_size(),
            carrier.len(PackageComponent::Uasset)
        );
        assert_eq!(package.uexp_data_length(), export.len());
        assert_eq!(
            package.exports(),
            [ExportBoundary {
                export_index: 0,
                object_name: "Asset".to_owned(),
                class_path: "/Script/Test.Fixture".to_owned(),
                local_generated_class: false,
                component: PackageComponent::Uexp,
                offset: 0,
                length: export.len(),
            }]
        );

        let envelope = package.export(0).unwrap();
        assert_eq!(envelope.bytes(), export);
        let decoded = envelope
            .decode_primitive_properties(&[bool_slot(0, "A"), bool_slot(1, "B")])
            .unwrap();
        assert_eq!(decoded.encoded_properties, &export[..8]);
        assert_eq!(decoded.native_suffix, &[0xaa, 0xbb]);
        assert_eq!(
            decoded
                .properties
                .encode(&[bool_slot(0, "A"), bool_slot(1, "B")])
                .unwrap(),
            export[..8]
        );
    }

    #[test]
    fn binds_native_export_class_to_its_exact_qualified_schema() {
        let bytes = [0x00, 0x01];
        let carrier =
            package_with_exports_configured(&[("Asset", &bytes, 0)], cooked_flags(), |package| {
                package.imports.clear();
                package.exports[0].class_index =
                    add_imported_class(package, "/Script/G1R", "PhysicMaterialsColor");
            });
        let package = LegacyPackageEnvelope::parse_g1r_ue5_4(&carrier).unwrap();
        let boundary = &package.exports()[0];
        assert_eq!(boundary.class_path(), "/Script/G1R.PhysicMaterialsColor");

        // A same-named foreign class proves the package map supplied the
        // module qualification instead of relying on ambiguous short names.
        let schemas = schema_db(&[
            (
                "PhysicMaterialsColor",
                "/Script/Other",
                usmap::FlagsType::Class,
            ),
            (
                "PhysicMaterialsColor",
                "/Script/G1R",
                usmap::FlagsType::Class,
            ),
        ]);
        let schema_id = boundary.resolve_class_schema(&schemas).unwrap();
        assert_eq!(
            schemas.schema(schema_id).unwrap().qualified_name(),
            "/Script/G1R.PhysicMaterialsColor"
        );
    }

    #[test]
    fn local_generated_class_requires_its_exact_c_schema_without_parent_fallback() {
        let asset = [0x00, 0x01];
        let generated_class = [0x00, 0x01];
        let carrier = package_with_exports_configured(
            &[("Asset", &asset, 0), ("BP_Fixture_C", &generated_class, 2)],
            cooked_flags(),
            |package| {
                package.exports[0].class_index = FPackageIndex::create_export(1);
            },
        );
        let package = LegacyPackageEnvelope::parse_g1r_ue5_4(&carrier).unwrap();
        let boundary = &package.exports()[0];
        assert_eq!(boundary.class_path(), "/Game/EnvelopeFixture.BP_Fixture_C");

        let parent_only = schema_db(&[(
            "PrimaryDataAsset",
            "/Script/Engine",
            usmap::FlagsType::Class,
        )]);
        assert_eq!(
            boundary.resolve_class_schema(&parent_only),
            Err(ExportSchemaError::LocalGeneratedClassSchemaMissing {
                export_index: 0,
                class_path: "/Game/EnvelopeFixture.BP_Fixture_C".to_owned(),
            })
        );

        let exact = schema_db(&[(
            "BP_Fixture_C",
            "/Game/EnvelopeFixture",
            usmap::FlagsType::Class,
        )]);
        assert_eq!(boundary.resolve_class_schema(&exact), Ok(0));
    }

    #[test]
    fn reports_missing_ambiguous_and_non_class_usmap_bindings() {
        let bytes = [0x00, 0x01];
        let carrier = package_with_exports(&[("Asset", &bytes, 0)]);
        let package = LegacyPackageEnvelope::parse_g1r_ue5_4(&carrier).unwrap();
        let boundary = &package.exports()[0];

        let missing = schema_db(&[]);
        assert!(matches!(
            boundary.resolve_class_schema(&missing),
            Err(ExportSchemaError::Schema {
                source: SchemaError::SchemaNotFound { .. },
                ..
            })
        ));

        let ambiguous = schema_db(&[
            ("Fixture", "/Script/Test", usmap::FlagsType::Class),
            ("Fixture", "/Script/Test", usmap::FlagsType::Class),
        ]);
        assert!(matches!(
            boundary.resolve_class_schema(&ambiguous),
            Err(ExportSchemaError::Schema {
                source: SchemaError::SchemaAmbiguous { .. },
                ..
            })
        ));

        let not_a_class = schema_db(&[("Fixture", "/Script/Test", usmap::FlagsType::Struct)]);
        assert!(matches!(
            boundary.resolve_class_schema(&not_a_class),
            Err(ExportSchemaError::Schema {
                source: SchemaError::NotAClass(_),
                ..
            })
        ));
    }

    #[test]
    fn imported_generated_class_also_refuses_a_primary_dataasset_fallback() {
        let bytes = [0x00, 0x01];
        let carrier =
            package_with_exports_configured(&[("Asset", &bytes, 0)], cooked_flags(), |package| {
                package.imports.clear();
                package.exports[0].class_index = add_imported_class(
                    package,
                    "/Game/Blueprints/BP_ImportedFixture",
                    "BP_ImportedFixture_C",
                );
            });
        let package = LegacyPackageEnvelope::parse_g1r_ue5_4(&carrier).unwrap();
        let boundary = &package.exports()[0];
        assert_eq!(
            boundary.class_path(),
            "/Game/Blueprints/BP_ImportedFixture.BP_ImportedFixture_C"
        );

        let parent_only = schema_db(&[(
            "PrimaryDataAsset",
            "/Script/Engine",
            usmap::FlagsType::Class,
        )]);
        assert!(matches!(
            boundary.resolve_class_schema(&parent_only),
            Err(ExportSchemaError::Schema {
                source: SchemaError::SchemaNotFound { .. },
                ..
            })
        ));
    }

    #[test]
    fn malformed_class_names_and_import_roots_are_typed_errors() {
        let bytes = [0x00, 0x01];
        let bad_name =
            package_with_exports_configured(&[("Asset", &bytes, 0)], cooked_flags(), |package| {
                let class_import = package.exports[0].class_index;
                let class_index = usize::try_from(-i64::from(class_import.index) - 1).unwrap();
                package.imports[class_index].object_name = FMinimalName {
                    index: i32::MAX,
                    number: 0,
                };
            });
        assert!(matches!(
            LegacyPackageEnvelope::parse_g1r_ue5_4(&bad_name),
            Err(EnvelopeError::ClassObjectName {
                export_index: 0,
                referenced_kind: "import",
                ..
            })
        ));

        let bad_root =
            package_with_exports_configured(&[("Asset", &bytes, 0)], cooked_flags(), |package| {
                let invalid_root = package.name_map.store("Not/A/PackagePath");
                package.imports[0].object_name = invalid_root;
            });
        assert!(matches!(
            LegacyPackageEnvelope::parse_g1r_ue5_4(&bad_root),
            Err(EnvelopeError::InvalidClassPath {
                export_index: 0,
                ..
            })
        ));
    }

    #[test]
    fn rejects_null_and_both_kinds_of_out_of_bounds_class_references() {
        let bytes = [0x00, 0x01];
        let null =
            package_with_exports_configured(&[("Asset", &bytes, 0)], cooked_flags(), |package| {
                package.exports[0].class_index = FPackageIndex::create_null()
            });
        assert!(matches!(
            LegacyPackageEnvelope::parse_g1r_ue5_4(&null),
            Err(EnvelopeError::NullExportClass { export_index: 0 })
        ));

        let import_oob =
            package_with_exports_configured(&[("Asset", &bytes, 0)], cooked_flags(), |package| {
                package.exports[0].class_index = FPackageIndex { index: i32::MIN }
            });
        assert!(matches!(
            LegacyPackageEnvelope::parse_g1r_ue5_4(&import_oob),
            Err(EnvelopeError::ClassObjectOutOfBounds {
                export_index: 0,
                referenced_kind: "import",
                ..
            })
        ));

        let export_oob =
            package_with_exports_configured(&[("Asset", &bytes, 0)], cooked_flags(), |package| {
                package.exports[0].class_index = FPackageIndex { index: i32::MAX }
            });
        assert!(matches!(
            LegacyPackageEnvelope::parse_g1r_ue5_4(&export_oob),
            Err(EnvelopeError::ClassObjectOutOfBounds {
                export_index: 0,
                referenced_kind: "export",
                ..
            })
        ));
    }

    #[test]
    fn rejects_import_and_export_outer_cycles() {
        let bytes = [0x00, 0x01];
        let import_cycle =
            package_with_exports_configured(&[("Asset", &bytes, 0)], cooked_flags(), |package| {
                let first_name = package.name_map.store("First");
                let second_name = package.name_map.store("Second");
                package.imports = vec![
                    FObjectImport {
                        outer_index: FPackageIndex::create_import(1),
                        object_name: first_name,
                        ..FObjectImport::default()
                    },
                    FObjectImport {
                        outer_index: FPackageIndex::create_import(0),
                        object_name: second_name,
                        ..FObjectImport::default()
                    },
                ];
                package.exports[0].class_index = FPackageIndex::create_import(0);
            });
        assert!(matches!(
            LegacyPackageEnvelope::parse_g1r_ue5_4(&import_cycle),
            Err(EnvelopeError::ClassOuterCycle {
                export_index: 0,
                referenced_kind: "import",
                ..
            })
        ));

        let local_class = [0x00, 0x01];
        let export_cycle = package_with_exports_configured(
            &[("Asset", &bytes, 0), ("BP_Fixture_C", &local_class, 2)],
            cooked_flags(),
            |package| {
                package.exports[0].class_index = FPackageIndex::create_export(1);
                package.exports[1].outer_index = FPackageIndex::create_export(1);
            },
        );
        assert!(matches!(
            LegacyPackageEnvelope::parse_g1r_ue5_4(&export_cycle),
            Err(EnvelopeError::ClassOuterCycle {
                export_index: 0,
                referenced_kind: "export",
                referenced_index: 1,
            })
        ));
    }

    #[test]
    fn rejects_class_outer_chains_beyond_the_fixed_depth_limit() {
        let bytes = [0x00, 0x01];
        let carrier =
            package_with_exports_configured(&[("Asset", &bytes, 0)], cooked_flags(), |package| {
                package.imports.clear();
                for index in 0..=MAX_CLASS_OUTER_DEPTH {
                    let object_name = package.name_map.store(&format!("Node{index}"));
                    let outer_index = if index < MAX_CLASS_OUTER_DEPTH {
                        FPackageIndex::create_import((index + 1) as u32)
                    } else {
                        FPackageIndex::create_null()
                    };
                    package.imports.push(FObjectImport {
                        outer_index,
                        object_name,
                        ..FObjectImport::default()
                    });
                }
                package.exports[0].class_index = FPackageIndex::create_import(0);
            });
        assert!(matches!(
            LegacyPackageEnvelope::parse_g1r_ue5_4(&carrier),
            Err(EnvelopeError::ClassOuterDepthExceeded {
                export_index: 0,
                limit: MAX_CLASS_OUTER_DEPTH,
            })
        ));
    }

    #[test]
    fn rejects_bad_footer_and_out_of_range_prefix_split() {
        let export = [0x00, 0x01]; // valid final empty header
        let carrier = package_with_exports(&[("Asset", &export, 0)]);
        let package = LegacyPackageEnvelope::parse_g1r_ue5_4(&carrier).unwrap();
        assert!(matches!(
            package.export(0).unwrap().split_decoded_prefix(3),
            Err(EnvelopeError::DecodedPrefixOutOfBounds { .. })
        ));

        let mut corrupt = PackageCarrier::from_bytes(
            carrier.bytes(PackageComponent::Uasset).to_vec(),
            carrier.bytes(PackageComponent::Uexp).to_vec(),
            crate::PackageLimits::default(),
        )
        .unwrap();
        let end = corrupt.len(PackageComponent::Uexp);
        corrupt
            .replace_range(PackageComponent::Uexp, end - 1, 1, &[0])
            .unwrap();
        assert!(matches!(
            LegacyPackageEnvelope::parse_g1r_ue5_4(&corrupt),
            Err(EnvelopeError::PackageFooterMismatch { .. })
        ));
    }

    #[test]
    fn rejects_overlapping_export_ranges() {
        let first = [0x00, 0x01, 1, 2];
        let second = [0x00, 0x01, 3, 4];
        let carrier = package_with_exports(&[("First", &first, 0), ("Second", &second, 2)]);
        assert!(matches!(
            LegacyPackageEnvelope::parse_g1r_ue5_4(&carrier),
            Err(EnvelopeError::OverlappingExports {
                first: 0,
                second: 1
            })
        ));
    }

    #[test]
    fn rejects_non_cooked_packages_even_with_unversioned_properties() {
        let export = [0x00, 0x01];
        let carrier = package_with_exports_and_flags(
            &[("Asset", &export, 0)],
            EPackageFlags::UsesUnversionedProperties as u32,
        );
        assert!(matches!(
            LegacyPackageEnvelope::parse_g1r_ue5_4(&carrier),
            Err(EnvelopeError::NotCooked)
        ));
    }
}
