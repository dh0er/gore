//! Exact-current managed revision-3 item-patch catalog and prepare-only authoring routes.
//!
//! The read route binds the native embedded item schema to one fully reopened Store head. The
//! prepare route accepts one canonical pure item-patch transaction request, independently resolves
//! its class, provenance, field names, and scalar types against that native schema, then prepares
//! and fully reopens an immutable candidate. Neither route publishes the fixed head, reads a game
//! or save, builds, deploys, or grants runtime authority.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::OnceLock;

use gore_authoring::model_revision3::{EntityPayload, OriginRef};
use gore_authoring::{
    apply_revision3_item_patch_transaction_v1, AssetVerification, ContentSeal,
    GameGenerationAnchor, ItemFiniteFloatV1, ItemScalarTypeV1, ItemScalarValueV1, ProjectRevision3,
    Revision3ItemPatchBuildStatusV1, Revision3ItemPatchChangeV1, Revision3ItemPatchConflictV1,
    Revision3ItemPatchErrorV1, Revision3ItemPatchEvaluationV1, Revision3ItemPatchMutationV1,
    Revision3ItemPatchOutcomeV1, Revision3ItemPatchPublicationStatusV1,
    Revision3ItemPatchRequestV1, Revision3ItemPatchRuntimeStatusV1, Sha256Digest, WorkingHead,
    WorkingProjectStore, WorkingStoreError, WorkingStoreLimits, MAX_PROJECT_JSON_BYTES,
    MAX_REVISION3_ITEM_PATCH_REQUEST_JSON_BYTES_V1,
};
use gore_generation::FileSeal;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::authoring_store_root_guard::{RetainedStoreRoot, RetainedStoreRootError};
use crate::err;

pub(super) const CATALOG_COMMAND: &str = "authoring_store_read_revision3_item_catalog_v1";
pub(super) const PREPARE_COMMAND: &str = "authoring_store_prepare_revision3_item_patch_v1";

const CATALOG_LAYER: &str = "base-game.items.g1r.bundled.v1";
const ITEM_CATALOG_JSON: &str = include_str!("../../../apps/mod-studio/assets/item_catalog.json");
const ITEM_MODEL_JSON: &str = include_str!("../../../apps/mod-studio/assets/model.json");

const MAX_PATH_BYTES: usize = 32 * 1024;
const MAX_HEAD_JSON_BYTES: usize = 64 * 1024;
const MAX_ERROR_MESSAGE_BYTES: usize = 4 * 1024;
const MAX_RESPONSE_BYTES: usize = 64 * 1024 * 1024;
const MAX_BASIS_REVISION: u64 = i64::MAX as u64 - 1;
const ITEM_INTEGER_32_MIN: i64 = i32::MIN as i64;
const ITEM_INTEGER_32_MAX: i64 = i32::MAX as i64;
const ITEM_FLOAT_32_MAX: f64 = f32::MAX as f64;
const ITEM_CATALOG_AUTHORITY: &str = "audited_binds_ancestry_manifest_v1";
// Offline audit output: every embedded item class was joined to the sealed Binds field owners
// through the reviewed native class-ancestry evidence for both registered generations. This seal
// commits to the complete sorted `(vanilla class, field, native scalar type)` projection, not just
// to a global field-name allow-list. Updating either bundled asset requires an explicit re-audit.
const AUDITED_ITEM_FIELD_MANIFEST_BYTES: u64 = 603_471;
const AUDITED_ITEM_FIELD_MANIFEST_SHA256: [u8; 32] = [
    0xe7, 0x34, 0xcb, 0x7e, 0x7b, 0x71, 0x62, 0x3a, 0x14, 0xb6, 0x68, 0x29, 0x8b, 0xe0, 0xc9, 0x14,
    0x3f, 0x30, 0xb9, 0xf5, 0xa1, 0xfa, 0x05, 0x97, 0x87, 0xd2, 0xbb, 0xf8, 0xd5, 0x07, 0xf4, 0xe8,
];
const MAX_WIRE_BYTES: usize = MAX_PROJECT_JSON_BYTES * 2
    + MAX_REVISION3_ITEM_PATCH_REQUEST_JSON_BYTES_V1 * 2
    + MAX_PATH_BYTES * 6
    + MAX_HEAD_JSON_BYTES * 2
    + 8 * 1024;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ExactWireRequest<P> {
    command: String,
    payload: P,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ReadCatalogWirePayload {
    expected_head_json: String,
    root: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct PrepareItemPatchWirePayload {
    current_project_json: String,
    item_patch_request_json: String,
    root: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCatalogEntry {
    category: String,
    id: String,
    path: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawModel {
    classes: BTreeMap<String, RawModelClass>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawModelClass {
    fields: Vec<RawModelField>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawModelField {
    name: String,
    #[serde(rename = "type")]
    scalar_type: String,
    #[serde(default)]
    default: Option<Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
enum NativeItemScalarType {
    Integer,
    Float,
    Boolean,
}

impl NativeItemScalarType {
    fn transaction_type(self) -> ItemScalarTypeV1 {
        match self {
            Self::Integer => ItemScalarTypeV1::Integer,
            Self::Float => ItemScalarTypeV1::Float,
            Self::Boolean => ItemScalarTypeV1::Boolean,
        }
    }

    fn numeric_contract(
        self,
    ) -> (
        Option<NativeItemNumericDomain>,
        Option<ItemScalarValueV1>,
        Option<ItemScalarValueV1>,
    ) {
        match self {
            // These domains come from the audited shipped Binds.Cache, whose sealed
            // `(owner, field, value type)` witnesses are checked in
            // `gore_as::cache::binds::tests::
            // validates_item_authoring_field_types_against_real_binds_cache`.
            // AngelScript `int` is signed 32-bit and every embedded model `float`
            // authoring field is witnessed as native `float32`. No gameplay-semantic
            // minima are inferred from defaults or field names.
            Self::Integer => (
                Some(NativeItemNumericDomain::SignedInteger32),
                Some(ItemScalarValueV1::Integer(ITEM_INTEGER_32_MIN)),
                Some(ItemScalarValueV1::Integer(ITEM_INTEGER_32_MAX)),
            ),
            Self::Float => (
                Some(NativeItemNumericDomain::FiniteFloat32),
                Some(ItemScalarValueV1::Float(
                    ItemFiniteFloatV1::new(-ITEM_FLOAT_32_MAX)
                        .expect("finite f32 minimum is a finite f64"),
                )),
                Some(ItemScalarValueV1::Float(
                    ItemFiniteFloatV1::new(ITEM_FLOAT_32_MAX)
                        .expect("finite f32 maximum is a finite f64"),
                )),
            ),
            Self::Boolean => (None, None, None),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum NativeItemNumericDomain {
    SignedInteger32,
    FiniteFloat32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct NativeItemField {
    name: String,
    scalar_type: NativeItemScalarType,
    #[serde(skip_serializing_if = "Option::is_none")]
    numeric_domain: Option<NativeItemNumericDomain>,
    #[serde(skip_serializing_if = "Option::is_none")]
    minimum_value: Option<ItemScalarValueV1>,
    #[serde(skip_serializing_if = "Option::is_none")]
    maximum_value: Option<ItemScalarValueV1>,
    #[serde(skip_serializing_if = "Option::is_none")]
    default_value: Option<ItemScalarValueV1>,
}

impl NativeItemField {
    fn accepts(&self, value: &ItemScalarValueV1) -> bool {
        if value.scalar_type() != self.scalar_type.transaction_type() {
            return false;
        }
        match (self.numeric_domain, value) {
            (Some(NativeItemNumericDomain::SignedInteger32), ItemScalarValueV1::Integer(value)) => {
                (ITEM_INTEGER_32_MIN..=ITEM_INTEGER_32_MAX).contains(value)
            }
            (Some(NativeItemNumericDomain::FiniteFloat32), ItemScalarValueV1::Float(value)) => {
                value.get().abs() <= ITEM_FLOAT_32_MAX
            }
            (None, ItemScalarValueV1::Boolean(_)) => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct NativeItemCatalogEntry {
    category: String,
    fields: Vec<NativeItemField>,
    runtime_path: String,
    source_seal: ContentSeal,
    vanilla_class: String,
}

#[derive(Debug, Serialize)]
struct NativeItemCatalogEntrySource<'a> {
    catalog_layer: &'static str,
    category: &'a str,
    fields: &'a [NativeItemField],
    runtime_path: &'a str,
    vanilla_class: &'a str,
}

#[derive(Debug, Serialize)]
struct NativeItemCatalogSealSource<'a> {
    authority: &'static str,
    audited_generation: &'static str,
    catalog_layer: &'static str,
    entries: &'a [NativeItemCatalogEntry],
    field_manifest_seal: &'a ContentSeal,
    schema_revision: u32,
    target: &'a GameGenerationAnchor,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
struct NativeItemFieldManifestRow {
    vanilla_class: String,
    field_name: String,
    scalar_type: NativeItemScalarType,
}

#[derive(Debug, Serialize)]
struct NativeItemFieldManifestSource<'a> {
    authority: &'static str,
    rows: &'a [NativeItemFieldManifestRow],
    schema_revision: u32,
}

#[derive(Debug, Serialize)]
struct NativeItemCatalogDocument<'a> {
    catalog_layer: &'static str,
    catalog_seal: &'a ContentSeal,
    entries: &'a [NativeItemCatalogEntry],
    schema_revision: u32,
    target: &'a GameGenerationAnchor,
}

#[derive(Debug)]
struct NativeItemCatalog {
    entries: Vec<NativeItemCatalogEntry>,
    by_class: BTreeMap<String, usize>,
    field_manifest_seal: ContentSeal,
}

impl NativeItemCatalog {
    fn entry(&self, vanilla_class: &str) -> Option<&NativeItemCatalogEntry> {
        self.by_class
            .get(vanilla_class)
            .and_then(|index| self.entries.get(*index))
    }

    fn bind(&self, target: &GameGenerationAnchor) -> Result<BoundNativeItemCatalog<'_>, Failure> {
        let audited_generation = audited_item_generation(target)?;
        let catalog_seal = seal_serializable(&NativeItemCatalogSealSource {
            authority: ITEM_CATALOG_AUTHORITY,
            audited_generation,
            catalog_layer: CATALOG_LAYER,
            entries: &self.entries,
            field_manifest_seal: &self.field_manifest_seal,
            schema_revision: 1,
            target,
        })
        .map_err(catalog_invariant)?;
        Ok(BoundNativeItemCatalog {
            catalog: self,
            catalog_seal,
        })
    }
}

#[derive(Debug)]
struct BoundNativeItemCatalog<'a> {
    catalog: &'a NativeItemCatalog,
    catalog_seal: ContentSeal,
}

impl<'a> BoundNativeItemCatalog<'a> {
    fn entry(&self, vanilla_class: &str) -> Option<&'a NativeItemCatalogEntry> {
        self.catalog.entry(vanilla_class)
    }

    fn canonical_document_json(&self, target: &GameGenerationAnchor) -> Result<String, Failure> {
        serde_json::to_string(&NativeItemCatalogDocument {
            catalog_layer: CATALOG_LAYER,
            catalog_seal: &self.catalog_seal,
            entries: &self.catalog.entries,
            schema_revision: 1,
            target,
        })
        .map_err(|_| catalog_invariant("the native item catalog could not be serialized"))
    }
}

#[derive(Debug)]
struct Failure {
    code: &'static str,
    message: String,
}

impl Failure {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: truncate_utf8(message.into(), MAX_ERROR_MESSAGE_BYTES),
        }
    }

    fn response(self) -> Value {
        err(self.code, self.message)
    }
}

static NATIVE_ITEM_CATALOG: OnceLock<Result<NativeItemCatalog, String>> = OnceLock::new();

pub(super) fn read_revision3_item_catalog_v1_raw(input: &str) -> Value {
    read_revision3_item_catalog_v1_inner(input).unwrap_or_else(Failure::response)
}

fn read_revision3_item_catalog_v1_inner(input: &str) -> Result<Value, Failure> {
    let payload: ReadCatalogWirePayload = parse_exact_wire(input, CATALOG_COMMAND)?;
    validate_path(&payload.root)?;
    let expected_head = parse_canonical_head(&payload.expected_head_json)?;
    let retained_root =
        RetainedStoreRoot::capture(Path::new(&payload.root)).map_err(map_initial_root_error)?;

    // Every post-capture result is deferred until the retained path chain passes its closing
    // audit. Root replacement therefore dominates stale-head, Store, catalog, and response
    // failures instead of accidentally returning evidence about another same-path directory.
    let operation = (|| {
        let store = retained_root
            .open_existing_store(ffi_store_limits())
            .map_err(map_store_error)?;
        let basis = store
            .open_current_revision3(AssetVerification::Full)
            .map_err(map_store_error)?;
        if basis.head != expected_head {
            return Err(head_conflict());
        }
        require_signed_serializable(&basis.project)?;
        let catalog = native_item_catalog()?.bind(&basis.project.target)?;
        validate_project_item_patches_against_catalog(&basis.project, &catalog)?;
        let catalog_json = catalog.canonical_document_json(&basis.project.target)?;

        // Close the read window after the catalog has been materialized. A same-call Store change
        // is not allowed to produce chooser evidence for an obsolete project checkpoint.
        require_fixed_basis(&store, &basis.head, &basis.project)?;
        let head_json = canonical_head_json(&basis.head)?;
        if head_json != payload.expected_head_json {
            return Err(Failure::new(
                "AUTHORING_REVISION3_ITEM_CATALOG_HEAD_INVALID",
                "expected_head_json is not in its exact canonical spelling",
            ));
        }

        let response = enforce_response_budget(json!({
            "ok": true,
            "head_json": head_json,
            "project_id": basis.project.project_id.to_string(),
            "project_revision": basis.project.revision,
            "catalog_json": catalog_json,
            "catalog_seal": catalog.catalog_seal,
            "catalog_authority": "native_embedded_schema_exact_current_project",
            "build_status": "not_evaluated",
            "runtime_status": "runtime_unqualified",
            "publication_status": "not_applicable",
        }))?;
        require_fixed_basis(&store, &basis.head, &basis.project)?;
        Ok(response)
    })();

    retained_root.revalidate().map_err(map_closing_root_error)?;
    operation
}

pub(super) fn prepare_revision3_item_patch_v1_raw(input: &str) -> Value {
    prepare_revision3_item_patch_v1_inner(input).unwrap_or_else(Failure::response)
}

fn prepare_revision3_item_patch_v1_inner(input: &str) -> Result<Value, Failure> {
    prepare_revision3_item_patch_v1_inner_with_test_seams(input, || {}, || {})
}

#[cfg(test)]
fn prepare_revision3_item_patch_v1_inner_with_post_prepare_guard<A>(
    input: &str,
    after_checkpoint: A,
) -> Result<Value, Failure>
where
    A: FnOnce(),
{
    prepare_revision3_item_patch_v1_inner_with_test_seams(input, after_checkpoint, || {})
}

fn prepare_revision3_item_patch_v1_inner_with_test_seams<A, F>(
    input: &str,
    after_checkpoint: A,
    final_guard: F,
) -> Result<Value, Failure>
where
    A: FnOnce(),
    F: FnOnce(),
{
    let payload: PrepareItemPatchWirePayload = parse_exact_wire(input, PREPARE_COMMAND)?;
    validate_prepare_payload(&payload)?;
    let request = Revision3ItemPatchRequestV1::from_json(&payload.item_patch_request_json)
        .map_err(map_request_error)?;
    require_signed_serializable(&request)?;
    let retained_root =
        RetainedStoreRoot::capture(Path::new(&payload.root)).map_err(map_initial_root_error)?;

    let operation = (|| {
        let store = retained_root
            .open_existing_store(ffi_store_limits())
            .map_err(map_store_error)?;
        let basis = store
            .open_current_revision3(AssetVerification::Full)
            .map_err(map_store_error)?;
        validate_basis_revision(basis.project.revision)?;
        require_signed_serializable(&basis.project)?;
        require_signed_serializable(&basis.head)?;

        let canonical_basis = basis.project.to_canonical_json().map_err(|_| {
            Failure::new(
                "AUTHORING_REVISION3_ITEM_PATCH_STORE_INVARIANT",
                "the exact current revision-3 project could not be serialized canonically",
            )
        })?;
        if canonical_basis.as_bytes() != payload.current_project_json.as_bytes() {
            return Err(Failure::new(
                "AUTHORING_REVISION3_ITEM_PATCH_PROJECT_CONFLICT",
                "current_project_json differs from the exact published revision-3 project",
            ));
        }
        bind_request_to_basis(&basis.head, &basis.project, &request)?;
        let catalog = native_item_catalog()?.bind(&basis.project.target)?;
        validate_project_item_patches_against_catalog(&basis.project, &catalog)?;
        let selected = validate_request_against_catalog(&catalog, &request)?;
        let (selected_catalog_layer, selected_vanilla_class, selected_source_seal) =
            request_provenance(&request);

        let outcome = match apply_revision3_item_patch_transaction_v1(
            &basis.head,
            &canonical_basis,
            &payload.item_patch_request_json,
        )
        .map_err(map_transaction_error)?
        {
            Revision3ItemPatchEvaluationV1::Applied(outcome) => *outcome,
            Revision3ItemPatchEvaluationV1::Rejected(rejection) => {
                return Err(map_transaction_conflict(rejection.conflict));
            }
        };
        require_signed_serializable(&outcome.project)?;
        validate_project_item_patches_against_catalog(&outcome.project, &catalog)?;
        verify_outcome_binding(&basis.head, &basis.project, &request, selected, &outcome)?;
        match outcome.build_status {
            Revision3ItemPatchBuildStatusV1::Blocked => {}
        }
        match outcome.runtime_status {
            Revision3ItemPatchRuntimeStatusV1::RuntimeUnqualified => {}
        }
        match outcome.publication_status {
            Revision3ItemPatchPublicationStatusV1::NotSupported => {}
        }

        let prepared = store
            .prepare_revision3_checkpoint(Some(&basis.head), &outcome.project)
            .map_err(map_store_error)?;
        let reopened = store
            .open_revision3_head_bytes(&prepared.head_bytes, AssetVerification::Full)
            .map_err(map_store_error)?;
        if reopened.head != prepared.head || reopened.project != outcome.project {
            return Err(Failure::new(
                "AUTHORING_REVISION3_ITEM_PATCH_STORE_INVARIANT",
                "the prepared item-patch checkpoint did not fully reopen exactly",
            ));
        }
        let reopened_json = reopened.project.to_canonical_json().map_err(|_| {
            Failure::new(
                "AUTHORING_REVISION3_ITEM_PATCH_STORE_INVARIANT",
                "the fully reopened item-patch candidate could not be serialized",
            )
        })?;
        if reopened_json != outcome.canonical_project_json {
            return Err(Failure::new(
                "AUTHORING_REVISION3_ITEM_PATCH_STORE_INVARIANT",
                "the fully reopened item-patch candidate changed canonical bytes",
            ));
        }

        after_checkpoint();
        require_fixed_basis(&store, &basis.head, &basis.project)?;

        let basis_head_json = canonical_head_json(&basis.head)?;
        let candidate_head_json = String::from_utf8(prepared.head_bytes).map_err(|_| {
            Failure::new(
                "AUTHORING_REVISION3_ITEM_PATCH_STORE_INVARIANT",
                "the prepared item-patch head is not UTF-8 JSON",
            )
        })?;
        if candidate_head_json.is_empty() || candidate_head_json.len() > MAX_HEAD_JSON_BYTES {
            return Err(Failure::new(
                "AUTHORING_REVISION3_ITEM_PATCH_RESPONSE_LIMIT",
                "the prepared item-patch head exceeds its bounded transport limit",
            ));
        }
        require_signed_serializable(&prepared.head)?;

        let change = match outcome.change {
            Revision3ItemPatchChangeV1::Created => "created",
            Revision3ItemPatchChangeV1::Updated => "updated",
            Revision3ItemPatchChangeV1::Removed => "removed",
        };
        let response = json!({
            "ok": true,
            "outcome": "prepared_unpublished",
            "basis_head_json": basis_head_json,
            "head_json": candidate_head_json,
            "project_json": outcome.canonical_project_json,
            "project_id": outcome.project.project_id.to_string(),
            "revision": outcome.project.revision,
            "entity_id": outcome.entity_id.to_string(),
            "entity_revision": outcome.entity_revision,
            "change": change,
            "catalog_layer": selected_catalog_layer,
            "vanilla_class": selected_vanilla_class,
            "source_seal": selected_source_seal,
            "catalog_seal": catalog.catalog_seal,
            "build_status": "blocked",
            "runtime_status": "runtime_unqualified",
            "publication_status": "not_supported",
        });
        let response = enforce_response_budget(response)?;

        final_guard();
        require_fixed_basis(&store, &basis.head, &basis.project)?;
        Ok(response)
    })();

    retained_root.revalidate().map_err(map_closing_root_error)?;
    operation
}

fn build_native_item_catalog() -> Result<NativeItemCatalog, String> {
    build_native_item_catalog_from_sources(ITEM_CATALOG_JSON, ITEM_MODEL_JSON)
}

fn build_native_item_catalog_from_sources(
    item_catalog_json: &str,
    item_model_json: &str,
) -> Result<NativeItemCatalog, String> {
    let raw_entries: Vec<RawCatalogEntry> = serde_json::from_str(item_catalog_json)
        .map_err(|error| format!("embedded item catalog is invalid: {error}"))?;
    let mut raw_model: RawModel = serde_json::from_str(item_model_json)
        .map_err(|error| format!("embedded item model is invalid: {error}"))?;
    if raw_entries.is_empty() {
        return Err("embedded item catalog is empty".to_owned());
    }

    let allowed_categories = BTreeSet::from([
        "ammunition",
        "amulet",
        "armor",
        "food",
        "key",
        "melee_weapon",
        "misc",
        "mission",
        "ranged_weapon",
        "ring",
        "rune",
        "scroll",
        "special",
        "trophy",
        "writing",
    ]);
    let mut seen = BTreeSet::new();
    let mut entries = Vec::with_capacity(raw_entries.len());
    let mut manifest_rows = Vec::new();
    for raw in raw_entries {
        if !valid_item_name(&raw.id) || !seen.insert(raw.id.clone()) {
            return Err(format!(
                "embedded item catalog has invalid or duplicate id {}",
                raw.id
            ));
        }
        if !allowed_categories.contains(raw.category.as_str()) {
            return Err(format!(
                "embedded item {} has an unsupported category",
                raw.id
            ));
        }
        if raw.path != format!("/Script/Angelscript.{}", raw.id) {
            return Err(format!(
                "embedded item {} has a mismatched runtime path",
                raw.id
            ));
        }
        let class = raw_model
            .classes
            .remove(&raw.id)
            .ok_or_else(|| format!("embedded item {} has no field schema", raw.id))?;
        if class.fields.is_empty() {
            return Err(format!("embedded item {} has no authorable fields", raw.id));
        }
        let mut field_names = BTreeSet::new();
        let mut fields = Vec::with_capacity(class.fields.len());
        for raw_field in class.fields {
            if !valid_field_name(&raw_field.name) || !field_names.insert(raw_field.name.clone()) {
                return Err(format!(
                    "embedded item {} has an invalid or duplicate field {}",
                    raw.id, raw_field.name
                ));
            }
            let (scalar_type, default_value) = parse_native_field(&raw.id, &raw_field)?;
            manifest_rows.push(NativeItemFieldManifestRow {
                vanilla_class: raw.id.clone(),
                field_name: raw_field.name.clone(),
                scalar_type,
            });
            let (numeric_domain, minimum_value, maximum_value) = scalar_type.numeric_contract();
            let field = NativeItemField {
                name: raw_field.name,
                scalar_type,
                numeric_domain,
                minimum_value,
                maximum_value,
                default_value,
            };
            if field
                .default_value
                .as_ref()
                .is_some_and(|value| !field.accepts(value))
            {
                return Err(format!(
                    "embedded item {} field {} has a default outside its native numeric domain",
                    raw.id, field.name
                ));
            }
            fields.push(field);
        }
        let source = NativeItemCatalogEntrySource {
            catalog_layer: CATALOG_LAYER,
            category: &raw.category,
            fields: &fields,
            runtime_path: &raw.path,
            vanilla_class: &raw.id,
        };
        let source_seal = seal_serializable(&source)?;
        entries.push(NativeItemCatalogEntry {
            category: raw.category,
            fields,
            runtime_path: raw.path,
            source_seal,
            vanilla_class: raw.id,
        });
    }
    if !raw_model.classes.is_empty() {
        return Err("embedded item model contains classes outside the item catalog".to_owned());
    }
    manifest_rows.sort();
    let field_manifest_seal = seal_serializable(&NativeItemFieldManifestSource {
        authority: ITEM_CATALOG_AUTHORITY,
        rows: &manifest_rows,
        schema_revision: 1,
    })?;
    if field_manifest_seal.byte_len != AUDITED_ITEM_FIELD_MANIFEST_BYTES
        || field_manifest_seal.sha256.as_bytes() != &AUDITED_ITEM_FIELD_MANIFEST_SHA256
    {
        return Err(format!(
            "embedded item model differs from the audited Binds/ancestry field manifest: got {} bytes / sha256 {}",
            field_manifest_seal.byte_len, field_manifest_seal.sha256
        ));
    }
    entries.sort_by(|left, right| left.vanilla_class.cmp(&right.vanilla_class));
    let by_class = entries
        .iter()
        .enumerate()
        .map(|(index, entry)| (entry.vanilla_class.clone(), index))
        .collect();
    Ok(NativeItemCatalog {
        entries,
        by_class,
        field_manifest_seal,
    })
}

fn parse_native_field(
    item: &str,
    raw: &RawModelField,
) -> Result<(NativeItemScalarType, Option<ItemScalarValueV1>), String> {
    let scalar_type = match raw.scalar_type.as_str() {
        "int" => NativeItemScalarType::Integer,
        "float" => NativeItemScalarType::Float,
        "bool" => NativeItemScalarType::Boolean,
        _ => {
            return Err(format!(
                "embedded item {item} field {} has an unsupported scalar type",
                raw.name
            ));
        }
    };
    let default_value = match (&scalar_type, &raw.default) {
        (_, None) => None,
        (NativeItemScalarType::Integer, Some(Value::Number(value))) => value
            .as_i64()
            .map(ItemScalarValueV1::Integer)
            .ok_or_else(|| {
                format!(
                    "embedded item {item} field {} has an invalid default",
                    raw.name
                )
            })
            .map(Some)?,
        (NativeItemScalarType::Float, Some(Value::Number(value))) => {
            let value = value.as_f64().ok_or_else(|| {
                format!(
                    "embedded item {item} field {} has an invalid default",
                    raw.name
                )
            })?;
            Some(ItemScalarValueV1::Float(
                ItemFiniteFloatV1::new(value).map_err(|_| {
                    format!(
                        "embedded item {item} field {} has a non-finite default",
                        raw.name
                    )
                })?,
            ))
        }
        (NativeItemScalarType::Boolean, Some(Value::Bool(value))) => {
            Some(ItemScalarValueV1::Boolean(*value))
        }
        _ => {
            return Err(format!(
                "embedded item {item} field {} has a default of the wrong type",
                raw.name
            ));
        }
    };
    Ok((scalar_type, default_value))
}

fn seal_serializable(value: &impl Serialize) -> Result<ContentSeal, String> {
    let bytes = serde_json::to_vec(value)
        .map_err(|error| format!("native item catalog serialization failed: {error}"))?;
    let digest: [u8; 32] = Sha256::digest(&bytes).into();
    Ok(ContentSeal {
        byte_len: bytes.len() as u64,
        sha256: Sha256Digest::from_bytes(digest),
    })
}

fn native_item_catalog() -> Result<&'static NativeItemCatalog, Failure> {
    match NATIVE_ITEM_CATALOG.get_or_init(build_native_item_catalog) {
        Ok(catalog) => Ok(catalog),
        Err(reason) => Err(catalog_invariant(reason)),
    }
}

/// A project anchor holds the executable and nothing else, so the executable is the only key
/// available here. `rows_are_pairwise_distinct` in `gore-generation` is what keeps that key
/// unambiguous; widening it to the full triple would be a separate behaviour change.
fn audited_item_generation(target: &GameGenerationAnchor) -> Result<&'static str, Failure> {
    let executable = FileSeal {
        byte_len: target.executable.byte_len,
        sha256: *target.executable.sha256.as_bytes(),
    };
    gore_generation::row_for_executable(&executable)
        .map(|row| row.audited_item_generation)
        .ok_or_else(|| {
            Failure::new(
                "AUTHORING_REVISION3_ITEM_PATCH_TARGET_UNSUPPORTED",
                "the project target has no audited embedded Item schema generation",
            )
        })
}

fn validate_project_item_patches_against_catalog(
    project: &ProjectRevision3,
    catalog: &BoundNativeItemCatalog<'_>,
) -> Result<(), Failure> {
    let mut targets = BTreeSet::new();
    for entity in project.entities.values() {
        let EntityPayload::ItemPatch(patch) = &entity.payload else {
            continue;
        };
        let entry = catalog.entry(&patch.vanilla_class).ok_or_else(|| {
            Failure::new(
                "AUTHORING_REVISION3_ITEM_PATCH_CATALOG_CONFLICT",
                "an existing ItemPatch class is absent from the current native item catalog",
            )
        })?;
        if !targets.insert(patch.vanilla_class.as_str()) {
            return Err(Failure::new(
                "AUTHORING_REVISION3_ITEM_PATCH_CATALOG_CONFLICT",
                "the project contains duplicate ItemPatch targets",
            ));
        }
        let expected_origin = OriginRef::Vanilla {
            generation: project.target.clone(),
            catalog_layer: CATALOG_LAYER.to_owned(),
            canonical_selector: entry.vanilla_class.clone(),
            source_seal: entry.source_seal.clone(),
        };
        if entity.origin != expected_origin || patch.vanilla_class != entry.vanilla_class {
            return Err(Failure::new(
                "AUTHORING_REVISION3_ITEM_PATCH_PROVENANCE_CONFLICT",
                "an existing ItemPatch does not match the current native catalog provenance",
            ));
        }
        let schema: BTreeMap<&str, &NativeItemField> = entry
            .fields
            .iter()
            .map(|field| (field.name.as_str(), field))
            .collect();
        for (name, value) in &patch.fields {
            let Some(expected) = schema.get(name.as_str()) else {
                return Err(Failure::new(
                    "AUTHORING_REVISION3_ITEM_PATCH_FIELD_CONFLICT",
                    format!(
                        "existing ItemPatch field {name} is absent from the current native item schema"
                    ),
                ));
            };
            if !expected.accepts(value) {
                return Err(Failure::new(
                    "AUTHORING_REVISION3_ITEM_PATCH_FIELD_CONFLICT",
                    format!(
                        "existing ItemPatch field {name} is outside the current native scalar type or numeric domain"
                    ),
                ));
            }
        }
    }
    Ok(())
}

fn validate_request_against_catalog<'a>(
    catalog: &BoundNativeItemCatalog<'a>,
    request: &Revision3ItemPatchRequestV1,
) -> Result<&'a NativeItemCatalogEntry, Failure> {
    let (catalog_layer, vanilla_class, source_seal) = request_provenance(request);
    let entry = catalog.entry(vanilla_class).ok_or_else(|| {
        Failure::new(
            "AUTHORING_REVISION3_ITEM_PATCH_CATALOG_CONFLICT",
            "the requested vanilla item class is absent from the native item catalog",
        )
    })?;
    if catalog_layer != CATALOG_LAYER || source_seal != &entry.source_seal {
        return Err(Failure::new(
            "AUTHORING_REVISION3_ITEM_PATCH_PROVENANCE_CONFLICT",
            "the requested item provenance differs from the current native sealed catalog",
        ));
    }
    if let Revision3ItemPatchMutationV1::Upsert { fields, .. } = &request.mutation {
        let schema: BTreeMap<&str, &NativeItemField> = entry
            .fields
            .iter()
            .map(|field| (field.name.as_str(), field))
            .collect();
        for (name, value) in fields {
            let Some(expected) = schema.get(name.as_str()) else {
                return Err(Failure::new(
                    "AUTHORING_REVISION3_ITEM_PATCH_FIELD_CONFLICT",
                    format!("field {name} is absent from the selected native item schema"),
                ));
            };
            if !expected.accepts(value) {
                return Err(Failure::new(
                    "AUTHORING_REVISION3_ITEM_PATCH_FIELD_CONFLICT",
                    format!(
                        "field {name} has a value outside the native item scalar type or numeric domain"
                    ),
                ));
            }
        }
    }
    Ok(entry)
}

fn request_provenance(request: &Revision3ItemPatchRequestV1) -> (&str, &str, &ContentSeal) {
    match &request.mutation {
        Revision3ItemPatchMutationV1::Upsert {
            catalog_layer,
            vanilla_class,
            source_seal,
            ..
        } => (catalog_layer, vanilla_class, source_seal),
        Revision3ItemPatchMutationV1::Remove {
            expected_catalog_layer,
            expected_vanilla_class,
            expected_source_seal,
            ..
        } => (
            expected_catalog_layer,
            expected_vanilla_class,
            expected_source_seal,
        ),
    }
}

fn parse_exact_wire<P>(input: &str, command: &str) -> Result<P, Failure>
where
    P: DeserializeOwned + Serialize,
{
    if input.len() > MAX_WIRE_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_ITEM_PATCH_INPUT_LIMIT",
            format!("revision-3 item request exceeds the {MAX_WIRE_BYTES}-byte wire limit"),
        ));
    }
    let request: ExactWireRequest<P> =
        serde_json::from_str(input).map_err(|_| invalid_request())?;
    if request.command != command {
        return Err(invalid_request());
    }
    let canonical = serde_json::to_string(&request).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_ITEM_PATCH_INVARIANT",
            "the item outer request could not be serialized",
        )
    })?;
    if canonical.as_bytes() != input.as_bytes() {
        return Err(invalid_request());
    }
    Ok(request.payload)
}

fn validate_prepare_payload(payload: &PrepareItemPatchWirePayload) -> Result<(), Failure> {
    validate_path(&payload.root)?;
    if payload.current_project_json.is_empty() || payload.item_patch_request_json.is_empty() {
        return Err(invalid_request());
    }
    if payload.current_project_json.len() > MAX_PROJECT_JSON_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_ITEM_PATCH_PROJECT_LIMIT",
            format!("current_project_json exceeds the {MAX_PROJECT_JSON_BYTES}-byte limit"),
        ));
    }
    if payload.item_patch_request_json.len() > MAX_REVISION3_ITEM_PATCH_REQUEST_JSON_BYTES_V1 {
        return Err(Failure::new(
            "AUTHORING_REVISION3_ITEM_PATCH_REQUEST_LIMIT",
            format!(
                "item_patch_request_json exceeds the {MAX_REVISION3_ITEM_PATCH_REQUEST_JSON_BYTES_V1}-byte limit"
            ),
        ));
    }
    Ok(())
}

fn validate_path(path: &str) -> Result<(), Failure> {
    if path.is_empty() || path.len() > MAX_PATH_BYTES || path.contains('\0') {
        return Err(invalid_request());
    }
    Ok(())
}

fn parse_canonical_head(input: &str) -> Result<WorkingHead, Failure> {
    if input.is_empty() || input.len() > MAX_HEAD_JSON_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_ITEM_CATALOG_HEAD_INVALID",
            "expected_head_json is empty or exceeds its bounded transport limit",
        ));
    }
    let head: WorkingHead = serde_json::from_str(input).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_ITEM_CATALOG_HEAD_INVALID",
            "expected_head_json is not one closed revision-3 working head",
        )
    })?;
    let canonical = serde_json::to_string(&head).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_ITEM_PATCH_INVARIANT",
            "the item head could not be serialized",
        )
    })?;
    if canonical != input {
        return Err(Failure::new(
            "AUTHORING_REVISION3_ITEM_CATALOG_HEAD_INVALID",
            "expected_head_json is not duplicate-free canonical JSON",
        ));
    }
    Ok(head)
}

fn bind_request_to_basis(
    head: &WorkingHead,
    project: &ProjectRevision3,
    request: &Revision3ItemPatchRequestV1,
) -> Result<(), Failure> {
    if request.expected_head != *head {
        return Err(head_conflict());
    }
    if request.expected_project_id != project.project_id
        || request.expected_revision != project.revision
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_ITEM_PATCH_PROJECT_CONFLICT",
            "the item-patch request project differs from the exact published project",
        ));
    }
    if request.expected_target != project.target {
        return Err(Failure::new(
            "AUTHORING_REVISION3_ITEM_PATCH_TARGET_CONFLICT",
            "the item-patch request target differs from the exact published project target",
        ));
    }
    Ok(())
}

fn verify_outcome_binding(
    basis_head: &WorkingHead,
    basis: &ProjectRevision3,
    request: &Revision3ItemPatchRequestV1,
    selected: &NativeItemCatalogEntry,
    outcome: &Revision3ItemPatchOutcomeV1,
) -> Result<(), Failure> {
    let expected_revision = basis
        .revision
        .checked_add(1)
        .ok_or_else(|| revision_limit("the project revision cannot be incremented"))?;
    let (expected_id, expected_change) = match &request.mutation {
        Revision3ItemPatchMutationV1::Upsert {
            entity_id,
            expected_entity_revision: None,
            ..
        } => (*entity_id, Revision3ItemPatchChangeV1::Created),
        Revision3ItemPatchMutationV1::Upsert { entity_id, .. } => {
            (*entity_id, Revision3ItemPatchChangeV1::Updated)
        }
        Revision3ItemPatchMutationV1::Remove { entity_id, .. } => {
            (*entity_id, Revision3ItemPatchChangeV1::Removed)
        }
    };
    if outcome.basis_head != *basis_head
        || outcome.project.project_id != basis.project_id
        || outcome.project.target != basis.target
        || outcome.project.revision != expected_revision
        || outcome.entity_id != expected_id
        || outcome.change != expected_change
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_ITEM_PATCH_INVARIANT",
            "the item-patch transaction outcome escaped its exact request basis",
        ));
    }
    match outcome.change {
        Revision3ItemPatchChangeV1::Created | Revision3ItemPatchChangeV1::Updated => {
            let entity = outcome.project.entities.get(&expected_id).ok_or_else(|| {
                Failure::new(
                    "AUTHORING_REVISION3_ITEM_PATCH_INVARIANT",
                    "the item-patch transaction omitted its retained entity",
                )
            })?;
            let EntityPayload::ItemPatch(patch) = &entity.payload else {
                return Err(Failure::new(
                    "AUTHORING_REVISION3_ITEM_PATCH_INVARIANT",
                    "the item-patch transaction returned the wrong entity kind",
                ));
            };
            let expected_origin = OriginRef::Vanilla {
                generation: basis.target.clone(),
                catalog_layer: CATALOG_LAYER.to_owned(),
                canonical_selector: selected.vanilla_class.clone(),
                source_seal: selected.source_seal.clone(),
            };
            if entity.origin != expected_origin || patch.vanilla_class != selected.vanilla_class {
                return Err(Failure::new(
                    "AUTHORING_REVISION3_ITEM_PATCH_INVARIANT",
                    "the item-patch transaction changed native catalog provenance",
                ));
            }
            if outcome.entity_revision != Some(entity.revision) {
                return Err(Failure::new(
                    "AUTHORING_REVISION3_ITEM_PATCH_INVARIANT",
                    "the item-patch transaction returned a mismatched entity revision",
                ));
            }
        }
        Revision3ItemPatchChangeV1::Removed => {
            if outcome.entity_revision.is_some()
                || outcome.project.entities.contains_key(&expected_id)
            {
                return Err(Failure::new(
                    "AUTHORING_REVISION3_ITEM_PATCH_INVARIANT",
                    "the removed item patch remains in the transaction outcome",
                ));
            }
        }
    }
    let canonical = outcome.project.to_canonical_json().map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_ITEM_PATCH_INVARIANT",
            "the item-patch transaction outcome could not be serialized canonically",
        )
    })?;
    if canonical != outcome.canonical_project_json {
        return Err(Failure::new(
            "AUTHORING_REVISION3_ITEM_PATCH_INVARIANT",
            "the item-patch transaction outcome carries inconsistent canonical bytes",
        ));
    }
    Ok(())
}

fn require_fixed_basis(
    store: &WorkingProjectStore,
    expected_head: &WorkingHead,
    expected_project: &ProjectRevision3,
) -> Result<(), Failure> {
    let current = store
        .open_current_revision3(AssetVerification::Full)
        .map_err(map_store_error)?;
    if current.head != *expected_head || current.project != *expected_project {
        return Err(head_conflict());
    }
    Ok(())
}

fn validate_basis_revision(revision: u64) -> Result<(), Failure> {
    if revision > MAX_BASIS_REVISION {
        return Err(revision_limit(
            "the published project revision cannot be incremented on the signed wire",
        ));
    }
    Ok(())
}

fn require_signed_serializable(value: &impl Serialize) -> Result<(), Failure> {
    let value = serde_json::to_value(value).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_ITEM_PATCH_INVARIANT",
            "an item-patch value could not be represented on the JSON wire",
        )
    })?;
    require_signed_json_value(&value)
}

fn require_signed_json_value(value: &Value) -> Result<(), Failure> {
    match value {
        Value::Number(number) if number.as_u64().is_some_and(|value| value > i64::MAX as u64) => {
            Err(Failure::new(
                "AUTHORING_REVISION3_ITEM_PATCH_SIGNED_WIRE_LIMIT",
                "an item-patch wire integer exceeds the signed 64-bit transport range",
            ))
        }
        Value::Array(values) => values.iter().try_for_each(require_signed_json_value),
        Value::Object(values) => values.values().try_for_each(require_signed_json_value),
        _ => Ok(()),
    }
}

fn canonical_head_json(head: &WorkingHead) -> Result<String, Failure> {
    require_signed_serializable(head)?;
    let value = serde_json::to_string(head).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_ITEM_PATCH_INVARIANT",
            "the item-patch head could not be serialized",
        )
    })?;
    if value.is_empty() || value.len() > MAX_HEAD_JSON_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_ITEM_PATCH_RESPONSE_LIMIT",
            "the item-patch head exceeds its bounded transport limit",
        ));
    }
    Ok(value)
}

fn enforce_response_budget(response: Value) -> Result<Value, Failure> {
    let encoded = serde_json::to_vec(&response).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_ITEM_PATCH_INVARIANT",
            "the item response could not be serialized",
        )
    })?;
    if encoded.len() > MAX_RESPONSE_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_ITEM_PATCH_RESPONSE_LIMIT",
            "the item response exceeds its bounded transport budget",
        ));
    }
    Ok(response)
}

fn ffi_store_limits() -> WorkingStoreLimits {
    WorkingStoreLimits {
        max_referenced_entity_bytes: MAX_PROJECT_JSON_BYTES as u64,
        ..WorkingStoreLimits::default()
    }
}

fn valid_item_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 512
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn valid_field_name(value: &str) -> bool {
    valid_item_name(value)
}

fn invalid_request() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_ITEM_PATCH_REQUEST_INVALID",
        "request must be exact canonical JSON for one native item catalog read or item-patch prepare payload",
    )
}

fn head_conflict() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_ITEM_PATCH_HEAD_CONFLICT",
        "the published revision-3 head changed or differs from the item request",
    )
}

fn revision_limit(message: &'static str) -> Failure {
    Failure::new("AUTHORING_REVISION3_ITEM_PATCH_REVISION_LIMIT", message)
}

fn catalog_invariant(message: impl Into<String>) -> Failure {
    Failure::new("AUTHORING_REVISION3_ITEM_CATALOG_INVARIANT", message)
}

fn map_initial_root_error(_error: RetainedStoreRootError) -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_ITEM_PATCH_STORE_ROOT_UNAVAILABLE",
        "managed Store root identity could not be captured safely",
    )
}

fn map_closing_root_error(_error: RetainedStoreRootError) -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_ITEM_PATCH_STORE_ROOT_CHANGED",
        "the managed Store root changed identity during the item operation",
    )
}

fn map_request_error(error: impl std::fmt::Display) -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_ITEM_PATCH_REQUEST_INVALID",
        format!("the exact item-patch request is invalid: {error}"),
    )
}

fn map_transaction_error(error: Revision3ItemPatchErrorV1) -> Failure {
    match error {
        Revision3ItemPatchErrorV1::InvalidProject(_) => Failure::new(
            "AUTHORING_REVISION3_ITEM_PATCH_PROJECT_INVALID",
            "the exact current project is not a valid item-patch basis",
        ),
        Revision3ItemPatchErrorV1::InvalidRequest(error) => map_request_error(error),
        Revision3ItemPatchErrorV1::ReopenCandidate(_)
        | Revision3ItemPatchErrorV1::CanonicalReopenMismatch => Failure::new(
            "AUTHORING_REVISION3_ITEM_PATCH_INVARIANT",
            "the item-patch transaction candidate failed exact canonical reopen",
        ),
    }
}

fn map_transaction_conflict(error: Revision3ItemPatchConflictV1) -> Failure {
    let code = match &error {
        Revision3ItemPatchConflictV1::CurrentHeadMismatch => {
            "AUTHORING_REVISION3_ITEM_PATCH_HEAD_CONFLICT"
        }
        Revision3ItemPatchConflictV1::ProjectIdentityMismatch { .. }
        | Revision3ItemPatchConflictV1::ProjectRevisionConflict { .. } => {
            "AUTHORING_REVISION3_ITEM_PATCH_PROJECT_CONFLICT"
        }
        Revision3ItemPatchConflictV1::ProjectTargetMismatch => {
            "AUTHORING_REVISION3_ITEM_PATCH_TARGET_CONFLICT"
        }
        Revision3ItemPatchConflictV1::ZeroEntityId
        | Revision3ItemPatchConflictV1::EntityAlreadyExists { .. }
        | Revision3ItemPatchConflictV1::EntityMissingOrWrongKind { .. }
        | Revision3ItemPatchConflictV1::EntityRevisionConflict { .. }
        | Revision3ItemPatchConflictV1::DuplicateVanillaTarget => {
            "AUTHORING_REVISION3_ITEM_PATCH_ENTITY_CONFLICT"
        }
        Revision3ItemPatchConflictV1::ProvenanceConflict { .. } => {
            "AUTHORING_REVISION3_ITEM_PATCH_PROVENANCE_CONFLICT"
        }
        Revision3ItemPatchConflictV1::NoChanges => "AUTHORING_REVISION3_ITEM_PATCH_NO_CHANGES",
        Revision3ItemPatchConflictV1::ProjectRevisionOverflow
        | Revision3ItemPatchConflictV1::EntityRevisionOverflow { .. } => {
            "AUTHORING_REVISION3_ITEM_PATCH_REVISION_LIMIT"
        }
        Revision3ItemPatchConflictV1::EntityCapacityExceeded
        | Revision3ItemPatchConflictV1::CandidateTooLarge { .. } => {
            "AUTHORING_REVISION3_ITEM_PATCH_PROJECT_LIMIT"
        }
        Revision3ItemPatchConflictV1::CandidateNotPersistable { .. } => {
            "AUTHORING_REVISION3_ITEM_PATCH_PROJECT_INVALID"
        }
    };
    Failure::new(code, error.to_string())
}

fn map_store_error(error: WorkingStoreError) -> Failure {
    if error.is_read_mount_changed() {
        return map_closing_root_error(RetainedStoreRootError::Changed);
    }
    let code = match error {
        WorkingStoreError::InvalidLimits(_) => {
            "AUTHORING_REVISION3_ITEM_PATCH_STORE_LIMITS_INVALID"
        }
        WorkingStoreError::MissingRoot(_) => "AUTHORING_REVISION3_ITEM_PATCH_STORE_ROOT_MISSING",
        WorkingStoreError::UnsafePath { .. } => "AUTHORING_REVISION3_ITEM_PATCH_STORE_PATH_UNSAFE",
        WorkingStoreError::LimitExceeded { .. } => "AUTHORING_REVISION3_ITEM_PATCH_STORE_LIMIT",
        WorkingStoreError::HeadConflict { .. } => "AUTHORING_REVISION3_ITEM_PATCH_HEAD_CONFLICT",
        WorkingStoreError::MissingHead(_) => "AUTHORING_REVISION3_ITEM_PATCH_HEAD_MISSING",
        WorkingStoreError::MissingObject(_) => {
            "AUTHORING_REVISION3_ITEM_PATCH_STORE_OBJECT_MISSING"
        }
        WorkingStoreError::SealMismatch { .. } => {
            "AUTHORING_REVISION3_ITEM_PATCH_STORE_SEAL_MISMATCH"
        }
        WorkingStoreError::Collision { .. } => "AUTHORING_REVISION3_ITEM_PATCH_STORE_COLLISION",
        WorkingStoreError::InvalidJson { .. } | WorkingStoreError::NonCanonicalJson { .. } => {
            "AUTHORING_REVISION3_ITEM_PATCH_STORE_JSON_INVALID"
        }
        WorkingStoreError::Invariant(_)
        | WorkingStoreError::InvalidOgg(_)
        | WorkingStoreError::OggMetadataMismatch { .. } => {
            "AUTHORING_REVISION3_ITEM_PATCH_STORE_INVARIANT"
        }
        WorkingStoreError::StagingCleanup { .. } | WorkingStoreError::Io(_) => {
            "AUTHORING_REVISION3_ITEM_PATCH_STORE_IO"
        }
    };
    Failure::new(code, "the revision-3 item Store operation failed")
}

fn truncate_utf8(mut value: String, max_bytes: usize) -> String {
    if value.len() <= max_bytes {
        return value;
    }
    let suffix = "...";
    let mut end = max_bytes - suffix.len();
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    value.truncate(end);
    value.push_str(suffix);
    value
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};
    use std::fs;

    use gore_authoring::{
        AssetStoreIndex, EntityId, FormatV2, ItemPatchV1, ProjectId, ProjectMeta, Revision3Entity,
        SchemaRevisionV3, WorkingStoreFormat,
    };
    use gore_story_catalog::{
        known_generation_v1, known_generation_v2, ContentSeal as StoryContentSeal,
    };
    use tempfile::TempDir;

    use super::*;

    struct PublishedStore {
        temp: TempDir,
        project: ProjectRevision3,
        project_json: String,
        head: WorkingHead,
        head_json: String,
    }

    fn entity_id(value: u8) -> EntityId {
        EntityId::from_bytes([value; 16])
    }

    fn authoring_target(executable: &StoryContentSeal) -> GameGenerationAnchor {
        GameGenerationAnchor {
            executable: ContentSeal {
                byte_len: executable.byte_len,
                sha256: Sha256Digest::from_bytes(*executable.sha256.as_bytes()),
            },
        }
    }

    fn project(revision: u64) -> ProjectRevision3 {
        let generation = known_generation_v1();
        ProjectRevision3 {
            format: FormatV2,
            schema_revision: SchemaRevisionV3,
            project_id: ProjectId::from_bytes([0x21; 16]),
            revision,
            meta: ProjectMeta {
                name: "Managed item fixture".to_owned(),
                version: "1.0.0".to_owned(),
                author: "tests".to_owned(),
            },
            target: authoring_target(&generation.executable),
            authoring_locales: BTreeSet::new(),
            entities: BTreeMap::new(),
            asset_store: AssetStoreIndex::default(),
        }
    }

    fn published_store(revision: u64) -> PublishedStore {
        published_project(project(revision))
    }

    fn published_project(project: ProjectRevision3) -> PublishedStore {
        let temp = TempDir::new().unwrap();
        let store = WorkingProjectStore::at(temp.path(), ffi_store_limits()).unwrap();
        let project_json = project.to_canonical_json().unwrap();
        let prepared = store.prepare_revision3_checkpoint(None, &project).unwrap();
        let head_json = String::from_utf8(prepared.head_bytes.clone()).unwrap();
        fs::write(temp.path().join("gore-project.json"), &prepared.head_bytes).unwrap();
        PublishedStore {
            temp,
            project,
            project_json,
            head: prepared.head,
            head_json,
        }
    }

    fn apple() -> &'static NativeItemCatalogEntry {
        native_item_catalog().unwrap().entry("ItFo_Apple").unwrap()
    }

    fn arrow() -> &'static NativeItemCatalogEntry {
        native_item_catalog().unwrap().entry("ItAm_Arrow").unwrap()
    }

    fn upsert_request(fixture: &PublishedStore) -> Revision3ItemPatchRequestV1 {
        Revision3ItemPatchRequestV1 {
            expected_head: fixture.head.clone(),
            expected_project_id: fixture.project.project_id,
            expected_revision: fixture.project.revision,
            expected_target: fixture.project.target.clone(),
            mutation: Revision3ItemPatchMutationV1::Upsert {
                entity_id: entity_id(0x41),
                expected_entity_revision: None,
                display_name: "Apple balance".to_owned(),
                catalog_layer: CATALOG_LAYER.to_owned(),
                vanilla_class: apple().vanilla_class.clone(),
                source_seal: apple().source_seal.clone(),
                fields: BTreeMap::from([("m_Value".to_owned(), ItemScalarValueV1::Integer(25))]),
            },
        }
    }

    fn prepare_wire(fixture: &PublishedStore, request: &Revision3ItemPatchRequestV1) -> String {
        serde_json::to_string(&ExactWireRequest {
            command: PREPARE_COMMAND.to_owned(),
            payload: PrepareItemPatchWirePayload {
                current_project_json: fixture.project_json.clone(),
                item_patch_request_json: request.to_canonical_json().unwrap(),
                root: fixture.temp.path().to_string_lossy().into_owned(),
            },
        })
        .unwrap()
    }

    fn catalog_wire(fixture: &PublishedStore) -> String {
        serde_json::to_string(&ExactWireRequest {
            command: CATALOG_COMMAND.to_owned(),
            payload: ReadCatalogWirePayload {
                expected_head_json: fixture.head_json.clone(),
                root: fixture.temp.path().to_string_lossy().into_owned(),
            },
        })
        .unwrap()
    }

    #[test]
    fn native_catalog_is_exact_current_sealed_and_field_typed() {
        let fixture = published_store(7);
        let response = read_revision3_item_catalog_v1_raw(&catalog_wire(&fixture));
        assert_eq!(response["ok"], true, "{response:#}");
        assert_eq!(response["head_json"], fixture.head_json);
        assert_eq!(
            response["project_id"],
            fixture.project.project_id.to_string()
        );
        assert_eq!(response["project_revision"], 7);
        assert_eq!(
            response["catalog_authority"],
            "native_embedded_schema_exact_current_project"
        );
        assert_eq!(response["runtime_status"], "runtime_unqualified");
        let document: Value =
            serde_json::from_str(response["catalog_json"].as_str().unwrap()).unwrap();
        assert_eq!(document["schema_revision"], 1);
        assert_eq!(document["catalog_layer"], CATALOG_LAYER);
        assert_eq!(document["target"], json!(fixture.project.target));
        assert_eq!(document["entries"].as_array().unwrap().len(), 798);
        let apple_document = document["entries"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["vanilla_class"] == "ItFo_Apple")
            .unwrap();
        let value = apple_document["fields"]
            .as_array()
            .unwrap()
            .iter()
            .find(|field| field["name"] == "m_Value")
            .unwrap();
        assert_eq!(value["scalar_type"], "integer");
        assert_eq!(value["numeric_domain"], "signed_integer32");
        assert_eq!(
            value["minimum_value"],
            json!({"type":"integer","data":i32::MIN})
        );
        assert_eq!(
            value["maximum_value"],
            json!({"type":"integer","data":i32::MAX})
        );
        assert_eq!(value["default_value"], json!({"type":"integer","data":4}));
        let weight = apple_document["fields"]
            .as_array()
            .unwrap()
            .iter()
            .find(|field| field["name"] == "m_Weight")
            .unwrap();
        assert_eq!(weight["numeric_domain"], "finite_float32");
        assert_eq!(
            weight["minimum_value"]["data"].as_f64().unwrap(),
            -(f32::MAX as f64)
        );
        assert_eq!(
            weight["maximum_value"]["data"].as_f64().unwrap(),
            f32::MAX as f64
        );
        let auto_target = apple_document["fields"]
            .as_array()
            .unwrap()
            .iter()
            .find(|field| field["name"] == "m_AutoTarget")
            .unwrap();
        assert!(auto_target.get("numeric_domain").is_none());
        assert!(auto_target.get("minimum_value").is_none());
        assert!(auto_target.get("maximum_value").is_none());

        let ore = document["entries"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["vanilla_class"] == "ItMi_Orenugget")
            .unwrap();
        let ore_stack = ore["fields"]
            .as_array()
            .unwrap()
            .iter()
            .find(|field| field["name"] == "m_MaxStack")
            .unwrap();
        assert_eq!(
            ore_stack["default_value"],
            json!({"type":"integer","data":0})
        );
        assert_eq!(
            ore_stack["minimum_value"],
            json!({"type":"integer","data":i32::MIN})
        );
        assert_eq!(apple_document["source_seal"], json!(apple().source_seal));
    }

    #[test]
    fn catalog_authority_is_target_bound_and_unknown_targets_fail_closed() {
        let v1 = published_store(2);
        let v1_response = read_revision3_item_catalog_v1_raw(&catalog_wire(&v1));
        assert_eq!(v1_response["ok"], true, "{v1_response:#}");

        let mut v2_project = project(2);
        v2_project.target = authoring_target(&known_generation_v2().executable);
        let v2 = published_project(v2_project);
        let v2_response = read_revision3_item_catalog_v1_raw(&catalog_wire(&v2));
        assert_eq!(v2_response["ok"], true, "{v2_response:#}");
        assert_ne!(
            v1_response["catalog_seal"], v2_response["catalog_seal"],
            "the authority seal must commit to the selected audited target"
        );

        let mut unknown_project = project(2);
        unknown_project.target.executable.sha256 = Sha256Digest::from_bytes([0x31; 32]);
        let unknown = published_project(unknown_project);
        assert_eq!(
            read_revision3_item_catalog_v1_raw(&catalog_wire(&unknown))["error"]["code"],
            "AUTHORING_REVISION3_ITEM_PATCH_TARGET_UNSUPPORTED"
        );
        let request = upsert_request(&unknown);
        assert_eq!(
            prepare_revision3_item_patch_v1_raw(&prepare_wire(&unknown, &request))["error"]["code"],
            "AUTHORING_REVISION3_ITEM_PATCH_TARGET_UNSUPPORTED"
        );
    }

    #[test]
    fn embedded_model_must_equal_the_audited_class_field_type_manifest() {
        let exact =
            build_native_item_catalog_from_sources(ITEM_CATALOG_JSON, ITEM_MODEL_JSON).unwrap();
        assert_eq!(
            exact.field_manifest_seal.byte_len,
            AUDITED_ITEM_FIELD_MANIFEST_BYTES
        );
        assert_eq!(
            exact.field_manifest_seal.sha256.as_bytes(),
            &AUDITED_ITEM_FIELD_MANIFEST_SHA256
        );

        fn rejected(mutator: impl FnOnce(&mut Value)) {
            let mut model: Value = serde_json::from_str(ITEM_MODEL_JSON).unwrap();
            mutator(&mut model);
            let error = build_native_item_catalog_from_sources(
                ITEM_CATALOG_JSON,
                &serde_json::to_string(&model).unwrap(),
            )
            .unwrap_err();
            assert!(
                error.contains("audited Binds/ancestry field manifest"),
                "{error}"
            );
        }

        rejected(|model| {
            model["classes"]["ItFo_Apple"]["fields"]
                .as_array_mut()
                .unwrap()
                .push(json!({"name":"m_Unproven","type":"bool","default":false}));
        });
        rejected(|model| {
            let value = model["classes"]["ItFo_Apple"]["fields"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|field| field["name"] == "m_Value")
                .unwrap();
            value["type"] = json!("float");
        });
        rejected(|model| {
            let radius = model["classes"]["ItAm_Arrow"]["fields"]
                .as_array()
                .unwrap()
                .iter()
                .find(|field| field["name"] == "m_Radius")
                .unwrap()
                .clone();
            model["classes"]["ItFo_Apple"]["fields"]
                .as_array_mut()
                .unwrap()
                .push(radius);
        });
    }

    #[test]
    fn exact_current_prepare_returns_fully_reopened_unpublished_candidate() {
        let fixture = published_store(7);
        let original_fixed_head = fs::read(fixture.temp.path().join("gore-project.json")).unwrap();
        let request = upsert_request(&fixture);
        let response = prepare_revision3_item_patch_v1_raw(&prepare_wire(&fixture, &request));
        assert_eq!(response["ok"], true, "{response:#}");
        assert_eq!(response["outcome"], "prepared_unpublished");
        assert_eq!(response["basis_head_json"], fixture.head_json);
        assert_eq!(response["revision"], 8);
        assert_eq!(response["change"], "created");
        assert_eq!(response["entity_revision"], 0);
        assert_eq!(response["catalog_layer"], CATALOG_LAYER);
        assert_eq!(response["vanilla_class"], "ItFo_Apple");
        assert_eq!(response["build_status"], "blocked");
        assert_eq!(response["runtime_status"], "runtime_unqualified");
        assert_eq!(response["publication_status"], "not_supported");
        assert_eq!(
            fs::read(fixture.temp.path().join("gore-project.json")).unwrap(),
            original_fixed_head
        );

        let store =
            WorkingProjectStore::open_existing(fixture.temp.path(), ffi_store_limits()).unwrap();
        let candidate = store
            .open_revision3_head_bytes(
                response["head_json"].as_str().unwrap().as_bytes(),
                AssetVerification::Full,
            )
            .unwrap();
        assert_eq!(
            candidate.project.to_canonical_json().unwrap(),
            response["project_json"]
        );
        let entity = &candidate.project.entities[&entity_id(0x41)];
        let EntityPayload::ItemPatch(patch) = &entity.payload else {
            panic!("expected item patch")
        };
        assert_eq!(patch.fields["m_Value"], ItemScalarValueV1::Integer(25));
        assert_eq!(
            entity.origin,
            OriginRef::Vanilla {
                generation: fixture.project.target,
                catalog_layer: CATALOG_LAYER.to_owned(),
                canonical_selector: "ItFo_Apple".to_owned(),
                source_seal: apple().source_seal.clone(),
            }
        );
    }

    #[test]
    fn dart_golden_float_request_and_project_reopen_and_prepare_end_to_end() {
        const BASIS: &str = include_str!(
            "../../../apps/mod-studio/test/fixtures/revision3_item_patch_float_basis_v1.json"
        );
        const REQUEST: &str = include_str!(
            "../../../apps/mod-studio/test/fixtures/revision3_item_patch_float_request_v1.json"
        );
        const CANDIDATE: &str = include_str!(
            "../../../apps/mod-studio/test/fixtures/revision3_item_patch_float_candidate_v1.json"
        );
        let basis = BASIS.trim_end();
        let request_json = REQUEST.trim_end();
        let candidate = CANDIDATE.trim_end();

        let parsed = Revision3ItemPatchRequestV1::from_json(request_json).unwrap();
        assert!(Revision3ItemPatchRequestV1::from_json(
            &request_json.replace("\"data\":1e-6", "\"data\":0.000001")
        )
        .is_err());
        assert!(Revision3ItemPatchRequestV1::from_json(
            &request_json.replace("\"data\":1e+20", "\"data\":100000000000000000000.0")
        )
        .is_err());
        let direct =
            apply_revision3_item_patch_transaction_v1(&parsed.expected_head, basis, request_json)
                .unwrap();
        let Revision3ItemPatchEvaluationV1::Applied(direct) = direct else {
            panic!("Dart golden request was rejected")
        };
        assert_eq!(direct.canonical_project_json, candidate);
        assert_eq!(
            ProjectRevision3::from_json(candidate).unwrap(),
            direct.project
        );
        assert!(ProjectRevision3::from_json(
            &candidate.replace("\"data\":1e-6", "\"data\":0.000001")
        )
        .is_err());

        // Rebind the exact Dart-produced scalar map to the real native Arrow
        // catalog provenance and exercise the complete prepare-only FFI route.
        let Revision3ItemPatchMutationV1::Upsert { fields, .. } = parsed.mutation else {
            panic!("Dart golden request is not an upsert")
        };
        let fixture = published_store(7);
        let request = Revision3ItemPatchRequestV1 {
            expected_head: fixture.head.clone(),
            expected_project_id: fixture.project.project_id,
            expected_revision: fixture.project.revision,
            expected_target: fixture.project.target.clone(),
            mutation: Revision3ItemPatchMutationV1::Upsert {
                entity_id: entity_id(0x42),
                expected_entity_revision: None,
                display_name: "Arrow physics".to_owned(),
                catalog_layer: CATALOG_LAYER.to_owned(),
                vanilla_class: arrow().vanilla_class.clone(),
                source_seal: arrow().source_seal.clone(),
                fields,
            },
        };
        let canonical_request = request.to_canonical_json().unwrap();
        assert!(canonical_request.contains("\"data\":1e-6"));
        assert!(canonical_request.contains("\"data\":1e+20"));
        assert!(canonical_request.contains("\"data\":0.0"));
        assert!(canonical_request.contains("\"data\":0.25"));
        let response = prepare_revision3_item_patch_v1_raw(&prepare_wire(&fixture, &request));
        assert_eq!(response["ok"], true, "{response:#}");
        let prepared =
            ProjectRevision3::from_json(response["project_json"].as_str().unwrap()).unwrap();
        let EntityPayload::ItemPatch(patch) = &prepared.entities[&entity_id(0x42)].payload else {
            panic!("expected Arrow item patch")
        };
        assert_eq!(
            patch.fields["m_ArcParam"],
            ItemScalarValueV1::Float(ItemFiniteFloatV1::new(1e-6).unwrap())
        );
        assert_eq!(
            patch.fields["m_Buoyancy"],
            ItemScalarValueV1::Float(ItemFiniteFloatV1::new(1e20).unwrap())
        );
        assert_eq!(
            patch.fields["m_Mass"],
            ItemScalarValueV1::Float(ItemFiniteFloatV1::new(0.0).unwrap())
        );
        assert_eq!(
            patch.fields["m_Weight"],
            ItemScalarValueV1::Float(ItemFiniteFloatV1::new(0.25).unwrap())
        );
    }

    #[test]
    fn prepare_rejects_unsealed_provenance_unknown_fields_and_wrong_types() {
        let fixture = published_store(3);
        let mut forged = upsert_request(&fixture);
        let Revision3ItemPatchMutationV1::Upsert { source_seal, .. } = &mut forged.mutation else {
            unreachable!()
        };
        source_seal.byte_len += 1;
        assert_eq!(
            prepare_revision3_item_patch_v1_raw(&prepare_wire(&fixture, &forged))["error"]["code"],
            "AUTHORING_REVISION3_ITEM_PATCH_PROVENANCE_CONFLICT"
        );

        let mut unknown = upsert_request(&fixture);
        let Revision3ItemPatchMutationV1::Upsert { fields, .. } = &mut unknown.mutation else {
            unreachable!()
        };
        *fields = BTreeMap::from([("m_NotARealField".to_owned(), ItemScalarValueV1::Integer(1))]);
        assert_eq!(
            prepare_revision3_item_patch_v1_raw(&prepare_wire(&fixture, &unknown))["error"]["code"],
            "AUTHORING_REVISION3_ITEM_PATCH_FIELD_CONFLICT"
        );

        let mut wrong_type = upsert_request(&fixture);
        let Revision3ItemPatchMutationV1::Upsert { fields, .. } = &mut wrong_type.mutation else {
            unreachable!()
        };
        *fields = BTreeMap::from([("m_Value".to_owned(), ItemScalarValueV1::Boolean(true))]);
        assert_eq!(
            prepare_revision3_item_patch_v1_raw(&prepare_wire(&fixture, &wrong_type))["error"]
                ["code"],
            "AUTHORING_REVISION3_ITEM_PATCH_FIELD_CONFLICT"
        );
    }

    #[test]
    fn prepare_enforces_native_i32_and_finite_f32_domains() {
        let fixture = published_store(3);
        let mut boundary = upsert_request(&fixture);
        let Revision3ItemPatchMutationV1::Upsert { fields, .. } = &mut boundary.mutation else {
            unreachable!()
        };
        *fields = BTreeMap::from([
            (
                "m_Mass".to_owned(),
                ItemScalarValueV1::Float(ItemFiniteFloatV1::new(f32::MAX as f64).unwrap()),
            ),
            (
                "m_MaxStack".to_owned(),
                ItemScalarValueV1::Integer(i32::MAX as i64),
            ),
            (
                "m_Value".to_owned(),
                ItemScalarValueV1::Integer(i32::MIN as i64),
            ),
            (
                "m_Weight".to_owned(),
                ItemScalarValueV1::Float(ItemFiniteFloatV1::new(-(f32::MAX as f64)).unwrap()),
            ),
        ]);
        let boundary_response =
            prepare_revision3_item_patch_v1_raw(&prepare_wire(&fixture, &boundary));
        assert_eq!(boundary_response["ok"], true, "{boundary_response:#}");

        for (name, value) in [
            ("m_Value", ItemScalarValueV1::Integer(i32::MIN as i64 - 1)),
            (
                "m_MaxStack",
                ItemScalarValueV1::Integer(i32::MAX as i64 + 1),
            ),
            (
                "m_Weight",
                ItemScalarValueV1::Float(ItemFiniteFloatV1::new(f64::MAX).unwrap()),
            ),
            (
                "m_Mass",
                ItemScalarValueV1::Float(ItemFiniteFloatV1::new(-f64::MAX).unwrap()),
            ),
        ] {
            let mut outside = upsert_request(&fixture);
            let Revision3ItemPatchMutationV1::Upsert { fields, .. } = &mut outside.mutation else {
                unreachable!()
            };
            *fields = BTreeMap::from([(name.to_owned(), value)]);
            assert_eq!(
                prepare_revision3_item_patch_v1_raw(&prepare_wire(&fixture, &outside))["error"]
                    ["code"],
                "AUTHORING_REVISION3_ITEM_PATCH_FIELD_CONFLICT",
                "{name}"
            );
        }
    }

    #[test]
    fn catalog_and_remove_reject_retired_project_provenance() {
        let mut basis = project(9);
        let retired_seal = ContentSeal {
            byte_len: 1234,
            sha256: Sha256Digest::from_bytes([0x77; 32]),
        };
        basis.entities.insert(
            entity_id(0x41),
            Revision3Entity {
                id: entity_id(0x41),
                display_name: "Older Apple patch".to_owned(),
                origin: OriginRef::Vanilla {
                    generation: basis.target.clone(),
                    catalog_layer: "base-game.items.g1r.retired.v0".to_owned(),
                    canonical_selector: "ItFo_Apple".to_owned(),
                    source_seal: retired_seal.clone(),
                },
                revision: 4,
                payload: EntityPayload::ItemPatch(ItemPatchV1 {
                    vanilla_class: "ItFo_Apple".to_owned(),
                    fields: BTreeMap::from([(
                        "m_Value".to_owned(),
                        ItemScalarValueV1::Integer(15),
                    )]),
                }),
            },
        );
        let fixture = published_project(basis);
        let catalog_response = read_revision3_item_catalog_v1_raw(&catalog_wire(&fixture));
        assert_eq!(
            catalog_response["error"]["code"], "AUTHORING_REVISION3_ITEM_PATCH_PROVENANCE_CONFLICT",
            "{catalog_response:#}"
        );
        assert!(catalog_response.get("catalog_json").is_none());
        let request = Revision3ItemPatchRequestV1 {
            expected_head: fixture.head.clone(),
            expected_project_id: fixture.project.project_id,
            expected_revision: fixture.project.revision,
            expected_target: fixture.project.target.clone(),
            mutation: Revision3ItemPatchMutationV1::Remove {
                entity_id: entity_id(0x41),
                expected_entity_revision: 4,
                expected_catalog_layer: "base-game.items.g1r.retired.v0".to_owned(),
                expected_vanilla_class: "ItFo_Apple".to_owned(),
                expected_source_seal: retired_seal.clone(),
            },
        };
        let response = prepare_revision3_item_patch_v1_raw(&prepare_wire(&fixture, &request));
        assert_eq!(
            response["error"]["code"], "AUTHORING_REVISION3_ITEM_PATCH_PROVENANCE_CONFLICT",
            "{response:#}"
        );
        assert!(!response.to_string().contains("project_json"));

        let current = apple();
        let mut current_basis = project(10);
        current_basis.entities.insert(
            entity_id(0x42),
            Revision3Entity {
                id: entity_id(0x42),
                display_name: "Current Apple patch".to_owned(),
                origin: OriginRef::Vanilla {
                    generation: current_basis.target.clone(),
                    catalog_layer: CATALOG_LAYER.to_owned(),
                    canonical_selector: current.vanilla_class.clone(),
                    source_seal: current.source_seal.clone(),
                },
                revision: 2,
                payload: EntityPayload::ItemPatch(ItemPatchV1 {
                    vanilla_class: current.vanilla_class.clone(),
                    fields: BTreeMap::from([(
                        "m_Value".to_owned(),
                        ItemScalarValueV1::Integer(15),
                    )]),
                }),
            },
        );
        let current_fixture = published_project(current_basis);
        let current_request = Revision3ItemPatchRequestV1 {
            expected_head: current_fixture.head.clone(),
            expected_project_id: current_fixture.project.project_id,
            expected_revision: current_fixture.project.revision,
            expected_target: current_fixture.project.target.clone(),
            mutation: Revision3ItemPatchMutationV1::Remove {
                entity_id: entity_id(0x42),
                expected_entity_revision: 2,
                expected_catalog_layer: CATALOG_LAYER.to_owned(),
                expected_vanilla_class: current.vanilla_class.clone(),
                expected_source_seal: current.source_seal.clone(),
            },
        };
        let current_response =
            prepare_revision3_item_patch_v1_raw(&prepare_wire(&current_fixture, &current_request));
        assert_eq!(current_response["ok"], true, "{current_response:#}");
        assert_eq!(current_response["change"], "removed");
        let candidate =
            ProjectRevision3::from_json(current_response["project_json"].as_str().unwrap())
                .unwrap();
        assert!(!candidate.entities.contains_key(&entity_id(0x42)));
    }

    #[test]
    fn catalog_and_prepare_reject_unsupported_existing_item_fields() {
        for (field_name, value) in [
            ("m_NotARealField", ItemScalarValueV1::Integer(1)),
            ("m_Value", ItemScalarValueV1::Boolean(true)),
            ("m_Value", ItemScalarValueV1::Integer(i32::MAX as i64 + 1)),
        ] {
            let current = apple();
            let mut basis = project(6);
            basis.entities.insert(
                entity_id(0x42),
                Revision3Entity {
                    id: entity_id(0x42),
                    display_name: "Unsupported Apple patch".to_owned(),
                    origin: OriginRef::Vanilla {
                        generation: basis.target.clone(),
                        catalog_layer: CATALOG_LAYER.to_owned(),
                        canonical_selector: current.vanilla_class.clone(),
                        source_seal: current.source_seal.clone(),
                    },
                    revision: 1,
                    payload: EntityPayload::ItemPatch(ItemPatchV1 {
                        vanilla_class: current.vanilla_class.clone(),
                        fields: BTreeMap::from([(field_name.to_owned(), value)]),
                    }),
                },
            );
            let fixture = published_project(basis);
            let catalog_response = read_revision3_item_catalog_v1_raw(&catalog_wire(&fixture));
            assert_eq!(
                catalog_response["error"]["code"], "AUTHORING_REVISION3_ITEM_PATCH_FIELD_CONFLICT",
                "{field_name}: {catalog_response:#}"
            );
            let request = upsert_request(&fixture);
            let prepare_response =
                prepare_revision3_item_patch_v1_raw(&prepare_wire(&fixture, &request));
            assert_eq!(
                prepare_response["error"]["code"], "AUTHORING_REVISION3_ITEM_PATCH_FIELD_CONFLICT",
                "{field_name}: {prepare_response:#}"
            );
            assert!(prepare_response.get("project_json").is_none());
        }
    }

    #[test]
    fn stale_head_and_same_call_publication_race_fail_closed() {
        let fixture = published_store(4);
        let mut stale = upsert_request(&fixture);
        stale.expected_head = WorkingHead {
            store_format: WorkingStoreFormat,
            snapshot: ContentSeal {
                byte_len: 1,
                sha256: Sha256Digest::from_bytes([0x99; 32]),
            },
        };
        assert_eq!(
            prepare_revision3_item_patch_v1_raw(&prepare_wire(&fixture, &stale))["error"]["code"],
            "AUTHORING_REVISION3_ITEM_PATCH_HEAD_CONFLICT"
        );

        let request = upsert_request(&fixture);
        let raw = prepare_wire(&fixture, &request);
        let root = fixture.temp.path().to_owned();
        let basis_head = fixture.head.clone();
        let result = prepare_revision3_item_patch_v1_inner_with_post_prepare_guard(&raw, || {
            let store = WorkingProjectStore::open_existing(&root, ffi_store_limits()).unwrap();
            let rival = project(5);
            let prepared = store
                .prepare_revision3_checkpoint(Some(&basis_head), &rival)
                .unwrap();
            fs::write(root.join("gore-project.json"), prepared.head_bytes).unwrap();
        });
        assert_eq!(
            result.unwrap_err().code,
            "AUTHORING_REVISION3_ITEM_PATCH_HEAD_CONFLICT"
        );
    }

    #[test]
    fn exact_outer_wires_reject_duplicates_unknowns_and_noncanonical_spelling() {
        let fixture = published_store(1);
        let canonical = catalog_wire(&fixture);
        assert_eq!(
            read_revision3_item_catalog_v1_raw(&format!(" {canonical}"))["error"]["code"],
            "AUTHORING_REVISION3_ITEM_PATCH_REQUEST_INVALID"
        );
        let duplicate = canonical.replacen(
            &format!("\"command\":\"{CATALOG_COMMAND}\""),
            &format!("\"command\":\"{CATALOG_COMMAND}\",\"command\":\"{CATALOG_COMMAND}\""),
            1,
        );
        assert_eq!(
            read_revision3_item_catalog_v1_raw(&duplicate)["error"]["code"],
            "AUTHORING_REVISION3_ITEM_PATCH_REQUEST_INVALID"
        );
        let unknown = canonical.replacen("\"root\":", "\"extra\":false,\"root\":", 1);
        assert_eq!(
            read_revision3_item_catalog_v1_raw(&unknown)["error"]["code"],
            "AUTHORING_REVISION3_ITEM_PATCH_REQUEST_INVALID"
        );
    }

    #[test]
    fn public_dispatch_routes_both_closed_item_commands() {
        let catalog_fixture = published_store(2);
        let catalog_response: Value =
            serde_json::from_str(&crate::execute_json(&catalog_wire(&catalog_fixture))).unwrap();
        assert_eq!(catalog_response["ok"], true, "{catalog_response:#}");
        assert_eq!(catalog_response["project_revision"], 2);

        let prepare_fixture = published_store(2);
        let request = upsert_request(&prepare_fixture);
        let prepare_response: Value = serde_json::from_str(&crate::execute_json(&prepare_wire(
            &prepare_fixture,
            &request,
        )))
        .unwrap();
        assert_eq!(prepare_response["ok"], true, "{prepare_response:#}");
        assert_eq!(prepare_response["outcome"], "prepared_unpublished");
    }
}
