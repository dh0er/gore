//! Bounded, read-only fixed-width DataAsset inspection for Mod Studio.
//!
//! This is deliberately an offline evidence surface, not a patch or runtime-qualification API.
//! It reuses `gore-asset`'s version-gated package envelope, USMAP schema walker, and offset-free
//! selector receipts. Native paths and parser diagnostics never cross the FFI boundary.
//!
//! Individual files are bound to stable no-follow handles for their complete read and reopen.
//! A split vanilla package has no shared semantic generation identifier, so the combined seal is
//! evidence for the exact two byte vectors inspected, not a claim that a live install used that
//! pair atomically. The command never reads a game installation and never writes any file.

use std::io;
use std::path::{Path, PathBuf};

use gore_asset::{
    FixedLeafDescriptor, FixedLeafInspectionError, FixedLeafInspectionLimits,
    FixedLeafInspectionSession, FixedLeafSelector, FixedLeafSelectorError, FixedLeafWorkBudget,
    FixedLeafWorkLimits, LegacyHeaderLimits, LegacyPackageEnvelope, PackageCarrier,
    PackageComponent, PackageLimits, PackagePairSeal, SchemaDb, SpanLimits, UsmapLimits,
    FIXED_LEAF_SELECTOR_FORMAT, FIXED_LEAF_SELECTOR_PROFILE,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::err;
use super::voice::{read_bounded_single_link_source, SecureSourceReadError};

const MAX_FILESYSTEM_PATH_BYTES: usize = 32 * 1024;
const MAX_FFI_REQUEST_BYTES: usize = MAX_FILESYSTEM_PATH_BYTES * 12 + 512;
const MAX_UASSET_BYTES: u64 = 64 * 1024 * 1024;
const MAX_UEXP_BYTES: u64 = 256 * 1024 * 1024;
const MAX_PACKAGE_BYTES: u64 = 320 * 1024 * 1024;
const MAX_USMAP_BYTES: u64 = 128 * 1024 * 1024;
const MAX_FFI_EXPORTS: usize = 4_096;
const MAX_FFI_LEAVES_PER_EXPORT: usize = 10_000;
const MAX_FFI_TOTAL_LEAVES: usize = 20_000;
const MAX_FFI_RESPONSE_BYTES: usize = 8 * 1024 * 1024;
const MAX_FFI_SPAN_NODES: usize = 500_000;
const MAX_FFI_COLLECTION_ELEMENTS: usize = 250_000;
const MAX_FFI_WORK: usize = 1_000_000;
const MAX_FFI_SELECTOR_STEPS: usize = 500_000;
const MAX_FFI_SELECTOR_DEPTH: usize = 128;
const MAX_FFI_SCHEMA_STRING_BYTES: usize = 32 * 1024 * 1024;
const MAX_FFI_ALLOCATION_BYTES: usize = 64 * 1024 * 1024;
const MAX_FFI_BYTE_WORK: usize = 384 * 1024 * 1024;
const MAX_FFI_HASH_BYTES: usize = 512 * 1024 * 1024;
const RESPONSE_BASE_BYTES: usize = 4 * 1024;
const RESPONSE_EXPORT_BASE_BYTES: usize = 768;
const RESPONSE_LEAF_BASE_BYTES: usize = 128;

#[derive(Debug)]
struct Failure {
    code: &'static str,
    message: &'static str,
}

impl Failure {
    const fn new(code: &'static str, message: &'static str) -> Self {
        Self { code, message }
    }

    fn response(self) -> Value {
        err(self.code, self.message)
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireRequest {
    command: String,
    payload: WirePayload,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WirePayload {
    uasset_path: String,
    usmap_path: String,
    #[serde(default)]
    export_index: Option<usize>,
}

#[derive(Debug, Serialize)]
struct InspectResponse {
    ok: bool,
    format: &'static str,
    status: InspectStatus,
    summary: Summary,
    selector_format: SelectorFormat,
    binding: Binding,
    input: InputFacts,
    selection: Selection,
    exports: Vec<ExportReport>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
enum InspectStatus {
    Walked,
    Partial,
    Unsupported,
}

#[derive(Debug, Serialize)]
struct Summary {
    package_exports: usize,
    reported_exports: usize,
    walked_exports: usize,
    editable_leaves: usize,
}

#[derive(Debug, Serialize)]
struct SelectorFormat {
    format: u32,
    profile: &'static str,
}

#[derive(Debug, Serialize)]
struct Binding {
    package_seal: PackagePairSeal,
    usmap_sha256: String,
}

#[derive(Debug, Serialize)]
struct InputFacts {
    uasset_length: u64,
    uexp_length: u64,
    usmap_length: u64,
}

#[derive(Debug, Serialize)]
struct Selection {
    export_index: Option<usize>,
}

#[derive(Debug, Serialize)]
struct ExportReport {
    index: usize,
    object_name: String,
    class_path: String,
    component: PackageComponent,
    length: usize,
    status: InspectStatus,
    failure: Option<ExportFailure>,
    schema: Option<String>,
    property_bytes: Option<usize>,
    native_suffix_bytes: Option<usize>,
    leaves: Vec<LeafReport>,
}

#[derive(Debug, Serialize)]
struct ExportFailure {
    stage: &'static str,
    code: &'static str,
}

#[derive(Debug, Serialize)]
struct LeafReport {
    index: usize,
    editable: bool,
    selector: FixedLeafSelector,
}

/// `payload: {uasset_path, usmap_path, export_index?}` -> bounded offline fixed-leaf evidence.
pub(super) fn fixed_inspect_v1_raw(input: &str) -> Value {
    if input.len() > MAX_FFI_REQUEST_BYTES {
        return bad_request().response();
    }
    let request: WireRequest = match serde_json::from_str(input) {
        Ok(request) => request,
        Err(_) => return bad_request().response(),
    };
    if request.command != "dataasset_fixed_inspect_v1" {
        return bad_request().response();
    }
    match fixed_inspect_v1_inner(request.payload) {
        Ok(response) => response,
        Err(error) => error.response(),
    }
}

fn fixed_inspect_v1_inner(payload: WirePayload) -> Result<Value, Failure> {
    validate_wire_path(&payload.uasset_path, "uasset")?;
    validate_wire_path(&payload.usmap_path, "usmap")?;

    let uasset_path = PathBuf::from(&payload.uasset_path);
    let usmap_path = PathBuf::from(&payload.usmap_path);
    let uexp_path = uasset_path.with_extension("uexp");

    let uasset = read_input(&uasset_path, MAX_UASSET_BYTES, "uasset")?;
    let uexp = read_input(&uexp_path, MAX_UEXP_BYTES, "uexp")?;
    let usmap = read_input(&usmap_path, MAX_USMAP_BYTES, "usmap")?;
    let usmap_length = to_u64(usmap.len())?;

    let limits = package_limits();
    let carrier = PackageCarrier::from_bytes(uasset, uexp, limits).map_err(|_| input_limit())?;
    let schemas = SchemaDb::from_usmap_bounded(
        &usmap,
        UsmapLimits {
            max_file_bytes: usize::try_from(MAX_USMAP_BYTES).unwrap_or(usize::MAX),
            max_decompressed_bytes: usize::try_from(MAX_USMAP_BYTES).unwrap_or(usize::MAX),
            ..UsmapLimits::default()
        },
    )
    .map_err(|_| {
        Failure::new(
            "DATAASSET_USMAP_INVALID",
            "the USMAP input is not a supported valid schema map",
        )
    })?;
    drop(usmap);
    let package = LegacyPackageEnvelope::parse_g1r_ue5_4_with_limits(
        &carrier,
        LegacyHeaderLimits {
            max_names: 250_000,
            max_imports: 250_000,
            max_exports: MAX_FFI_EXPORTS,
            max_cell_imports: 0,
            max_cell_exports: 0,
            max_preload_dependencies: 1_000_000,
            max_data_resources: 100_000,
            max_summary_array_elements: 250_000,
            max_string_bytes: 64 * 1024,
            max_total_string_bytes: 16 * 1024 * 1024,
            max_derived_metadata_bytes: 16 * 1024 * 1024,
            max_class_path_bytes: 64 * 1024,
            max_derived_work: 1_000_000,
        },
    )
    .map_err(|_| {
        Failure::new(
            "DATAASSET_PACKAGE_INVALID",
            "the package pair is not a supported valid G1R UE5.4 cooked package",
        )
    })?;

    let package_export_count = package.exports().len();
    let indices = selected_indices(package_export_count, payload.export_index)?;
    let session = FixedLeafInspectionSession::new(&carrier, &schemas).map_err(|_| {
        Failure::new(
            "DATAASSET_INSPECT_FAILED",
            "the exact package and USMAP identities could not be retained",
        )
    })?;
    let mut response_budget = ResponseBuildBudget::new(MAX_FFI_RESPONSE_BYTES);
    response_budget.charge(RESPONSE_BASE_BYTES)?;
    response_budget.charge_string(session.usmap_sha256())?;
    response_budget.charge_product(indices.len(), RESPONSE_EXPORT_BASE_BYTES)?;
    let mut work_budget = FixedLeafWorkBudget::new(FixedLeafWorkLimits {
        max_work: MAX_FFI_WORK,
        max_nodes: MAX_FFI_SPAN_NODES,
        max_collection_elements: MAX_FFI_COLLECTION_ELEMENTS,
        max_leaves: MAX_FFI_TOTAL_LEAVES,
        max_selector_steps: MAX_FFI_SELECTOR_STEPS,
        max_selector_bytes: response_budget.remaining(),
        max_schema_string_bytes: MAX_FFI_SCHEMA_STRING_BYTES,
        max_allocation_bytes: MAX_FFI_ALLOCATION_BYTES,
        max_byte_work: MAX_FFI_BYTE_WORK,
        max_hash_bytes: MAX_FFI_HASH_BYTES,
    });
    let mut reports = Vec::new();
    reports
        .try_reserve_exact(indices.len())
        .map_err(|_| response_limit())?;
    let mut total_leaves = 0usize;
    for index in indices {
        let report = inspect_export(
            &package,
            &session,
            index,
            &mut response_budget,
            &mut work_budget,
        )?;
        total_leaves = total_leaves
            .checked_add(report.leaves.len())
            .ok_or_else(response_limit)?;
        if total_leaves > MAX_FFI_TOTAL_LEAVES {
            return Err(response_limit());
        }
        reports.push(report);
    }

    let walked_exports = reports
        .iter()
        .filter(|report| report.status == InspectStatus::Walked)
        .count();
    let editable_leaves = reports
        .iter()
        .flat_map(|report| &report.leaves)
        .filter(|leaf| leaf.editable)
        .count();
    let status = if walked_exports == 0 {
        InspectStatus::Unsupported
    } else if walked_exports == reports.len() {
        InspectStatus::Walked
    } else {
        InspectStatus::Partial
    };

    let response = InspectResponse {
        ok: true,
        format: "gore.dataasset.fixed-inspect.v1",
        status,
        summary: Summary {
            package_exports: package_export_count,
            reported_exports: reports.len(),
            walked_exports,
            editable_leaves,
        },
        selector_format: SelectorFormat {
            format: FIXED_LEAF_SELECTOR_FORMAT,
            profile: FIXED_LEAF_SELECTOR_PROFILE,
        },
        binding: Binding {
            package_seal: session.package_seal().clone(),
            usmap_sha256: session.usmap_sha256().to_owned(),
        },
        input: InputFacts {
            uasset_length: to_u64(carrier.len(PackageComponent::Uasset))?,
            uexp_length: to_u64(carrier.len(PackageComponent::Uexp))?,
            usmap_length,
        },
        selection: Selection {
            export_index: payload.export_index,
        },
        exports: reports,
    };
    bounded_to_value(response, MAX_FFI_RESPONSE_BYTES)
}

fn inspect_export(
    package: &LegacyPackageEnvelope<'_>,
    session: &FixedLeafInspectionSession<'_>,
    index: usize,
    response_budget: &mut ResponseBuildBudget,
    work_budget: &mut FixedLeafWorkBudget,
) -> Result<ExportReport, Failure> {
    let boundary = &package.exports()[index];
    response_budget.charge_string(boundary.object_name())?;
    response_budget.charge_string(boundary.class_path())?;
    let mut report = ExportReport {
        index,
        object_name: boundary.object_name().to_owned(),
        class_path: boundary.class_path().to_owned(),
        component: boundary.component(),
        length: boundary.length(),
        status: InspectStatus::Unsupported,
        failure: None,
        schema: None,
        property_bytes: None,
        native_suffix_bytes: None,
        leaves: Vec::new(),
    };
    let export = package.export(index).map_err(|_| {
        Failure::new(
            "DATAASSET_INSPECT_FAILED",
            "a validated package export could not be reopened",
        )
    })?;
    work_budget.cap_selector_bytes(response_budget.remaining());
    let selector_before = work_budget.remaining_selector_bytes();
    let inspection = match session.inspect_export_bounded(
        &export,
        FixedLeafInspectionLimits {
            span_limits: SpanLimits {
                max_depth: 64,
                max_collection_elements: MAX_FFI_COLLECTION_ELEMENTS,
                max_total_nodes: MAX_FFI_SPAN_NODES,
            },
            max_descriptors_per_export: MAX_FFI_LEAVES_PER_EXPORT,
            max_selector_steps_per_leaf: MAX_FFI_SELECTOR_DEPTH,
        },
        work_budget,
    ) {
        Ok(inspection) => inspection,
        Err(error) if error.is_resource_limit() => return Err(response_limit()),
        Err(FixedLeafInspectionError::Selector(FixedLeafSelectorError::ExportSchema(_))) => {
            report.failure = Some(ExportFailure {
                stage: "schema",
                code: "schema_unsupported",
            });
            return Ok(report);
        }
        Err(FixedLeafInspectionError::SchemaUnsupported { .. }) => {
            report.failure = Some(ExportFailure {
                stage: "schema",
                code: "schema_unsupported",
            });
            return Ok(report);
        }
        Err(FixedLeafInspectionError::Selector(FixedLeafSelectorError::Span(_))) => {
            report.failure = Some(ExportFailure {
                stage: "walk",
                code: "property_stream_unsupported",
            });
            return Ok(report);
        }
        Err(_) => {
            report.failure = Some(ExportFailure {
                stage: "selector",
                code: "selector_receipt_unsupported",
            });
            return Ok(report);
        }
    };
    let selector_after = work_budget.remaining_selector_bytes();
    response_budget.charge(selector_before.saturating_sub(selector_after))?;
    let (schema_name, property_bytes, native_suffix_bytes, descriptors) = inspection.into_parts();
    response_budget.charge_product(descriptors.len(), RESPONSE_LEAF_BASE_BYTES)?;
    report.status = InspectStatus::Walked;
    report.schema = Some(schema_name);
    report.property_bytes = Some(property_bytes);
    report.native_suffix_bytes = Some(native_suffix_bytes);
    report
        .leaves
        .try_reserve_exact(descriptors.len())
        .map_err(|_| response_limit())?;
    report.leaves.extend(
        descriptors
            .into_iter()
            .enumerate()
            .map(|(leaf_index, descriptor)| leaf_report(leaf_index, descriptor)),
    );
    Ok(report)
}

fn leaf_report(index: usize, descriptor: FixedLeafDescriptor) -> LeafReport {
    LeafReport {
        index,
        editable: descriptor.editable,
        selector: descriptor.selector,
    }
}

fn selected_indices(
    package_export_count: usize,
    export_index: Option<usize>,
) -> Result<Vec<usize>, Failure> {
    match export_index {
        Some(index) if index >= package_export_count => Err(Failure::new(
            "DATAASSET_EXPORT_INVALID",
            "the selected export index does not exist in the package",
        )),
        Some(index) => Ok(vec![index]),
        None if package_export_count > MAX_FFI_EXPORTS => Err(response_limit()),
        None => Ok((0..package_export_count).collect()),
    }
}

fn validate_wire_path(value: &str, expected_extension: &'static str) -> Result<(), Failure> {
    if value.is_empty() || value.len() > MAX_FILESYSTEM_PATH_BYTES || value.contains('\0') {
        return Err(bad_request());
    }
    let path = Path::new(value);
    let expected_hidden_name = format!(".{expected_extension}");
    let valid_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| !name.is_empty() && !name.eq_ignore_ascii_case(&expected_hidden_name));
    let valid_extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case(expected_extension));
    if !valid_name || !valid_extension {
        return Err(bad_request());
    }
    Ok(())
}

fn read_input(path: &Path, limit: u64, kind: &'static str) -> Result<Vec<u8>, Failure> {
    read_bounded_single_link_source(path, limit).map_err(|error| match error {
        SecureSourceReadError::Unavailable => match kind {
            "uasset" => Failure::new(
                "DATAASSET_INPUT_UNAVAILABLE",
                "the uasset input could not be opened or read",
            ),
            "uexp" => Failure::new(
                "DATAASSET_INPUT_UNAVAILABLE",
                "the derived uexp input could not be opened or read",
            ),
            _ => Failure::new(
                "DATAASSET_INPUT_UNAVAILABLE",
                "the USMAP input could not be opened or read",
            ),
        },
        SecureSourceReadError::Unsafe => Failure::new(
            "DATAASSET_INPUT_UNSAFE",
            "an input is not a safe single-link regular file with safe ancestors",
        ),
        SecureSourceReadError::Limit => input_limit(),
        SecureSourceReadError::Changed => Failure::new(
            "DATAASSET_INPUT_CHANGED",
            "an input changed while its exact bytes were being read",
        ),
    })
}

const fn package_limits() -> PackageLimits {
    PackageLimits {
        max_uasset_bytes: MAX_UASSET_BYTES,
        max_uexp_bytes: MAX_UEXP_BYTES,
        max_total_bytes: MAX_PACKAGE_BYTES,
    }
}

fn to_u64(value: usize) -> Result<u64, Failure> {
    u64::try_from(value).map_err(|_| input_limit())
}

struct ResponseBuildBudget {
    remaining: usize,
}

impl ResponseBuildBudget {
    const fn new(limit: usize) -> Self {
        Self { remaining: limit }
    }

    const fn remaining(&self) -> usize {
        self.remaining
    }

    fn charge(&mut self, bytes: usize) -> Result<(), Failure> {
        if bytes > self.remaining {
            return Err(response_limit());
        }
        self.remaining -= bytes;
        Ok(())
    }

    fn charge_product(&mut self, count: usize, width: usize) -> Result<(), Failure> {
        let bytes = count.checked_mul(width).ok_or_else(response_limit)?;
        self.charge(bytes)
    }

    fn charge_string(&mut self, value: &str) -> Result<(), Failure> {
        // A JSON string byte can expand to at most one six-byte `\u00XX` escape.
        let bytes = value
            .len()
            .checked_mul(6)
            .and_then(|bytes| bytes.checked_add(2))
            .ok_or_else(response_limit)?;
        self.charge(bytes)
    }
}

struct BoundedCounter {
    remaining: usize,
}

impl io::Write for BoundedCounter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > self.remaining {
            return Err(io::Error::other("bounded response exceeded"));
        }
        self.remaining -= bytes.len();
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn serialized_within_limit<T: Serialize>(value: &T, limit: usize) -> bool {
    let mut counter = BoundedCounter { remaining: limit };
    serde_json::to_writer(&mut counter, value).is_ok()
}

fn bounded_to_value<T: Serialize>(value: T, limit: usize) -> Result<Value, Failure> {
    bounded_to_value_with(value, limit, |value| serde_json::to_value(value))
}

fn bounded_to_value_with<T, F>(value: T, limit: usize, convert: F) -> Result<Value, Failure>
where
    T: Serialize,
    F: FnOnce(T) -> Result<Value, serde_json::Error>,
{
    if !serialized_within_limit(&value, limit) {
        return Err(response_limit());
    }
    convert(value).map_err(|_| response_limit())
}

const fn bad_request() -> Failure {
    Failure::new(
        "DATAASSET_REQUEST_INVALID",
        "payload must contain only valid 'uasset_path' and 'usmap_path' strings plus optional 'export_index'",
    )
}

const fn input_limit() -> Failure {
    Failure::new(
        "DATAASSET_INPUT_LIMIT",
        "a DataAsset inspection input exceeds its supported resource limit",
    )
}

const fn response_limit() -> Failure {
    Failure::new(
        "DATAASSET_RESPONSE_LIMIT",
        "the complete fixed-leaf inspection response exceeds its supported resource limit",
    )
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::Cursor;

    use retoc::legacy_asset::{
        EPackageFlags, FLegacyPackageFileSummary, FLegacyPackageHeader, FObjectExport,
        FObjectImport,
    };
    use retoc::logging::Log;
    use retoc::version::EngineVersion;
    use retoc::zen::FPackageIndex;
    use serde_json::json;
    use tempfile::TempDir;

    use super::*;

    const WALKED_EXPORT: &[u8] = &[0x00, 0x03, 0x01];

    struct Fixture {
        uasset: PathBuf,
        usmap: PathBuf,
    }

    fn call(uasset: &Path, usmap: &Path, export_index: Option<usize>) -> Value {
        let mut payload = json!({
            "uasset_path": uasset.display().to_string(),
            "usmap_path": usmap.display().to_string(),
        });
        if let Some(index) = export_index {
            payload["export_index"] = json!(index);
        }
        serde_json::from_str(&crate::execute_json(
            &json!({
                "command": "dataasset_fixed_inspect_v1",
                "payload": payload,
            })
            .to_string(),
        ))
        .unwrap()
    }

    #[test]
    fn request_schema_rejects_unknown_duplicate_and_wrong_typed_fields() {
        let requests = [
            r#"{"command":"dataasset_fixed_inspect_v1","payload":{"uasset_path":"a.uasset","usmap_path":"b.usmap","extra":true}}"#,
            r#"{"command":"dataasset_fixed_inspect_v1","extra":true,"payload":{"uasset_path":"a.uasset","usmap_path":"b.usmap"}}"#,
            r#"{"command":"dataasset_fixed_inspect_v1","payload":{"uasset_path":"a.uasset","uasset_path":"b.uasset","usmap_path":"b.usmap"}}"#,
            r#"{"command":"dataasset_fixed_inspect_v1","payload":{"uasset_path":"a.uasset","usmap_path":"b.usmap","export_index":"0"}}"#,
            r#"{"command":"wrong","payload":{"uasset_path":"a.uasset","usmap_path":"b.usmap"}}"#,
            r#"{"command":"dataasset_fixed_inspect_v1","payload":{"uasset_path":"a.txt","usmap_path":"b.usmap"}}"#,
        ];
        for request in requests {
            let response = fixed_inspect_v1_raw(request);
            assert_eq!(response["error"]["code"], "DATAASSET_REQUEST_INVALID");
        }

        let oversized_path = format!("{}.uasset", "x".repeat(MAX_FILESYSTEM_PATH_BYTES));
        let oversized = fixed_inspect_v1_raw(
            &json!({
                "command": "dataasset_fixed_inspect_v1",
                "payload": {"uasset_path": oversized_path, "usmap_path": "b.usmap"},
            })
            .to_string(),
        );
        assert_eq!(oversized["error"]["code"], "DATAASSET_REQUEST_INVALID");
        assert_eq!(
            fixed_inspect_v1_raw(&" ".repeat(MAX_FFI_REQUEST_BYTES + 1))["error"]["code"],
            "DATAASSET_REQUEST_INVALID"
        );
    }

    #[test]
    fn missing_input_error_is_sanitized() {
        let temp = TempDir::new().unwrap();
        let uasset = temp.path().join("private-missing.uasset");
        let usmap = temp.path().join("private-missing.usmap");
        let response = call(&uasset, &usmap, None);
        assert_eq!(response["error"]["code"], "DATAASSET_INPUT_UNAVAILABLE");
        let serialized = response.to_string();
        assert!(!serialized.contains(uasset.to_string_lossy().as_ref()));
        assert!(!serialized.contains(temp.path().to_string_lossy().as_ref()));

        let small = temp.path().join("small.usmap");
        fs::write(&small, b"1234").unwrap();
        let limited = read_input(&small, 3, "usmap").unwrap_err();
        assert_eq!(limited.code, "DATAASSET_INPUT_LIMIT");
    }

    #[test]
    fn walked_response_contains_only_offline_sealed_offset_free_evidence() {
        let temp = TempDir::new().unwrap();
        let fixture = write_fixture(temp.path(), &[WALKED_EXPORT]);
        let response = call(&fixture.uasset, &fixture.usmap, None);

        assert_eq!(response["ok"], true, "response: {response}");
        assert_eq!(response["format"], "gore.dataasset.fixed-inspect.v1");
        assert_eq!(response["status"], "walked");
        assert_eq!(response["summary"]["package_exports"], 1);
        assert_eq!(response["summary"]["reported_exports"], 1);
        assert_eq!(response["summary"]["walked_exports"], 1);
        assert_eq!(response["summary"]["editable_leaves"], 1);
        assert_eq!(response["selector_format"]["format"], 1);
        assert_eq!(response["selector_format"]["profile"], "g1r_ue5_4");
        assert_eq!(response["exports"][0]["status"], "walked");
        assert_eq!(response["exports"][0]["component"], "uexp");
        assert_eq!(
            response["exports"][0]["leaves"].as_array().unwrap().len(),
            1
        );
        assert_eq!(response["exports"][0]["leaves"][0]["editable"], true);
        assert_eq!(
            response["exports"][0]["leaves"][0]["selector"]["expected_hex"],
            "01"
        );
        assert_eq!(
            response["exports"][0]["leaves"][0]["selector"]["package_seal"],
            response["binding"]["package_seal"]
        );
        assert_eq!(
            response["exports"][0]["leaves"][0]["selector"]["usmap_sha256"],
            response["binding"]["usmap_sha256"]
        );
        assert!(response["exports"][0].get("offset").is_none());
        assert!(response["input"].get("uasset_path").is_none());
        assert!(response.get("deployed").is_none());
        assert!(response.get("runtime_qualification").is_none());
        assert!(!response
            .to_string()
            .contains(temp.path().to_string_lossy().as_ref()));
        assert!(serialized_within_limit(&response, MAX_FFI_RESPONSE_BYTES));
    }

    #[test]
    fn mixed_exports_are_partial_and_selected_unsupported_is_not_overclaimed() {
        let temp = TempDir::new().unwrap();
        let fixture = write_fixture(temp.path(), &[WALKED_EXPORT, &[0xff]]);

        let all = call(&fixture.uasset, &fixture.usmap, None);
        assert_eq!(all["ok"], true, "response: {all}");
        assert_eq!(all["status"], "partial");
        assert_eq!(all["summary"]["walked_exports"], 1);
        assert_eq!(all["exports"][0]["status"], "walked");
        assert_eq!(all["exports"][1]["status"], "unsupported");
        assert_eq!(all["exports"][1]["failure"]["stage"], "walk");

        let selected = call(&fixture.uasset, &fixture.usmap, Some(1));
        assert_eq!(selected["ok"], true, "response: {selected}");
        assert_eq!(selected["status"], "unsupported");
        assert_eq!(selected["selection"]["export_index"], 1);
        assert_eq!(selected["summary"]["reported_exports"], 1);

        let missing = call(&fixture.uasset, &fixture.usmap, Some(2));
        assert_eq!(missing["error"]["code"], "DATAASSET_EXPORT_INVALID");
    }

    #[test]
    fn invalid_usmap_and_unsafe_multilink_errors_are_sanitized() {
        let temp = TempDir::new().unwrap();
        let fixture = write_fixture(temp.path(), &[WALKED_EXPORT]);
        fs::write(&fixture.usmap, b"not a USMAP").unwrap();
        let invalid = call(&fixture.uasset, &fixture.usmap, None);
        assert_eq!(invalid["error"]["code"], "DATAASSET_USMAP_INVALID");
        assert!(!invalid
            .to_string()
            .contains(temp.path().to_string_lossy().as_ref()));

        let fixture = write_fixture(temp.path(), &[WALKED_EXPORT]);
        fs::write(&fixture.uasset, b"not a cooked package").unwrap();
        let invalid = call(&fixture.uasset, &fixture.usmap, None);
        assert_eq!(invalid["error"]["code"], "DATAASSET_PACKAGE_INVALID");
        assert!(!invalid
            .to_string()
            .contains(temp.path().to_string_lossy().as_ref()));

        let fixture = write_fixture(temp.path(), &[WALKED_EXPORT]);
        let linked = temp.path().join("Linked.uasset");
        fs::hard_link(&fixture.uasset, &linked).unwrap();
        let unsafe_response = call(&linked, &fixture.usmap, None);
        assert_eq!(unsafe_response["error"]["code"], "DATAASSET_INPUT_UNSAFE");
        assert!(!unsafe_response
            .to_string()
            .contains(linked.to_string_lossy().as_ref()));
    }

    #[test]
    fn response_counter_fails_closed_at_the_exact_limit() {
        use std::cell::Cell;

        let small = json!({"value": "abc"});
        let exact = serde_json::to_vec(&small).unwrap().len();
        assert!(serialized_within_limit(&small, exact));
        assert!(!serialized_within_limit(&small, exact - 1));

        let calls = Cell::new(0usize);
        let rejected = bounded_to_value_with(small.clone(), exact - 1, |value| {
            calls.set(calls.get() + 1);
            Ok(value)
        });
        assert_eq!(rejected.unwrap_err().code, "DATAASSET_RESPONSE_LIMIT");
        assert_eq!(calls.get(), 0, "Value conversion must follow preflight");
        let accepted = bounded_to_value_with(small.clone(), exact, |value| {
            calls.set(calls.get() + 1);
            Ok(value)
        })
        .unwrap();
        assert_eq!(accepted, small);
        assert_eq!(calls.get(), 1);

        let mut budget = ResponseBuildBudget::new(8);
        budget.charge(8).unwrap();
        assert_eq!(budget.remaining(), 0);
        assert_eq!(
            budget.charge(1).unwrap_err().code,
            "DATAASSET_RESPONSE_LIMIT"
        );
    }

    fn write_fixture(directory: &Path, exports: &[&[u8]]) -> Fixture {
        let mut package = FLegacyPackageHeader::default();
        package.summary.versioning_info.package_file_version =
            EngineVersion::UE5_4.package_file_version();
        package.summary.versioning_info.is_unversioned = true;
        package.summary.package_name = "/Game/DataAssetFfiFixture".to_owned();
        package.summary.package_flags = EPackageFlags::Cooked as u32
            | EPackageFlags::FilterEditorOnly as u32
            | EPackageFlags::UsesUnversionedProperties as u32;

        let class_index = add_imported_class(&mut package, "/Script/Test", "Fixture");
        let mut uexp_bytes = Vec::new();
        for (index, export_bytes) in exports.iter().enumerate() {
            let object_name = package.name_map.store(&format!("Fixture{index}"));
            package.exports.push(FObjectExport {
                class_index,
                object_name,
                serial_offset: uexp_bytes.len() as i64,
                serial_size: export_bytes.len() as i64,
                ..FObjectExport::default()
            });
            uexp_bytes.extend_from_slice(export_bytes);
        }

        let mut serialized_header = Cursor::new(Vec::new());
        package
            .serialize(&mut serialized_header, None, &Log::no_log())
            .unwrap();
        uexp_bytes.extend_from_slice(&FLegacyPackageFileSummary::PACKAGE_FILE_TAG.to_le_bytes());

        let uasset = directory.join("Fixture.uasset");
        fs::write(&uasset, serialized_header.into_inner()).unwrap();
        fs::write(uasset.with_extension("uexp"), uexp_bytes).unwrap();

        let mapping = usmap::Usmap {
            enums: Vec::new(),
            structs: vec![usmap::Struct {
                name: "Fixture".to_owned(),
                super_struct: None,
                properties: vec![usmap::Property {
                    name: "Enabled".to_owned(),
                    array_dim: 1,
                    index: 0,
                    inner: usmap::PropertyInner::Bool,
                }],
            }],
            cext: None,
            ppth: Some(usmap::ExtPpth {
                version: 0,
                enums: Vec::new(),
                structs: vec!["/Script/Test".to_owned()],
            }),
            eatr: Some(usmap::ExtEatr {
                version: 0,
                enum_flags: Vec::new(),
                struct_flags: vec![usmap::StructFlags {
                    type_: usmap::FlagsType::Class,
                    value: 0,
                    prop_flags: Vec::new(),
                }],
            }),
            envp: None,
        };
        let mut raw_usmap = Vec::new();
        mapping.write(&mut raw_usmap).unwrap();
        let usmap = directory.join("Fixture.usmap");
        fs::write(&usmap, raw_usmap).unwrap();

        Fixture { uasset, usmap }
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
}
