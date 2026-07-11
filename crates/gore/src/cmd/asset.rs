//! `gore asset` -- schema-backed, copy-on-write fixed-leaf DataAsset tooling.

use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use clap::{Args, Subcommand};
use gore_asset::{
    describe_fixed_leaves, FixedLeafDescriptor, FixedLeafPatch, FixedLeafSelector,
    FixedLeafSelectorStep, LegacyPackageEnvelope, PackageCarrier, PackageComponent, PackageLimits,
    PackagePairSeal, PropertySpanWalker, SchemaDb, FIXED_LEAF_SELECTOR_FORMAT,
    FIXED_LEAF_SELECTOR_PROFILE,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};

const MAX_USMAP_BYTES: u64 = 128 * 1024 * 1024;
const MAX_SELECTOR_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Debug, Subcommand)]
pub enum AssetAction {
    /// List structurally editable fixed-width leaves in a legacy split package.
    Inspect(InspectArgs),
    /// Apply one snapshot-bound raw wire edit to a new package pair.
    PatchFixed(PatchFixedArgs),
}

#[derive(Debug, Args)]
pub struct InspectArgs {
    /// Input legacy `.uasset`; the sibling `.uexp` is required.
    #[arg(long, value_name = "INPUT.uasset")]
    pub uasset: PathBuf,
    /// Exact `.usmap` used to decode this package generation.
    #[arg(long, value_name = "MAPPINGS.usmap")]
    pub usmap: PathBuf,
    /// Inspect only this export; unsupported/missing selected exports are fatal.
    #[arg(long)]
    pub export_index: Option<usize>,
    /// Emit one machine-readable JSON document.
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct PatchFixedArgs {
    /// Input legacy `.uasset`; it is never modified.
    #[arg(long, value_name = "INPUT.uasset")]
    pub uasset: PathBuf,
    /// Exact `.usmap` named by the selector.
    #[arg(long, value_name = "MAPPINGS.usmap")]
    pub usmap: PathBuf,
    /// JSON containing a selector, descriptor, or one inspect leaf object.
    #[arg(long, value_name = "SELECTOR.json")]
    pub selector: PathBuf,
    /// Exact current raw little-endian wire bytes; must agree with the selector.
    #[arg(long, value_name = "HEX")]
    pub expected_hex: String,
    /// Exact replacement wire bytes; no gameplay/domain validation is implied.
    #[arg(long, value_name = "HEX")]
    pub replacement_hex: String,
    /// New `.uasset` output; its sibling `.uexp` is created without clobbering.
    #[arg(short = 'o', long, value_name = "OUTPUT.uasset")]
    pub out: PathBuf,
    /// Emit one machine-readable JSON document.
    #[arg(long)]
    pub json: bool,
}

pub fn run(action: AssetAction) -> Result<()> {
    match action {
        AssetAction::Inspect(args) => inspect(args),
        AssetAction::PatchFixed(args) => patch_fixed(args),
    }
}

#[derive(Debug)]
struct VerifiedInput {
    path: PathBuf,
    bytes: Vec<u8>,
    sha256: [u8; 32],
}

#[derive(Debug, Serialize)]
struct ExportReport {
    index: usize,
    object_name: String,
    class_path: String,
    component: PackageComponent,
    offset: usize,
    length: usize,
    status: &'static str,
    error: Option<String>,
    schema: Option<String>,
    property_bytes: Option<usize>,
    native_suffix_bytes: Option<usize>,
    leaves: Vec<LeafReport>,
}

#[derive(Debug, Serialize)]
struct LeafReport {
    index: usize,
    semantic_path: String,
    editable: bool,
    selector: FixedLeafSelector,
}

fn inspect(args: InspectArgs) -> Result<()> {
    let usmap = read_verified_bounded(&args.usmap, MAX_USMAP_BYTES, "ASSET_USMAP")?;
    let schemas = SchemaDb::from_usmap(&usmap.bytes).context("ASSET_USMAP")?;
    let carrier =
        PackageCarrier::load(&args.uasset, PackageLimits::default()).context("ASSET_INPUT")?;
    let package = LegacyPackageEnvelope::parse_g1r_ue5_4(&carrier).context("ASSET_ENVELOPE")?;

    let indices = match args.export_index {
        Some(index) => {
            if index >= package.exports().len() {
                bail!(
                    "ASSET_EXPORT: export {index} does not exist (exports={})",
                    package.exports().len()
                );
            }
            vec![index]
        }
        None => (0..package.exports().len()).collect(),
    };

    let mut reports = Vec::with_capacity(indices.len());
    for index in indices {
        let boundary = package
            .exports()
            .get(index)
            .expect("selected export index was bounded above");
        let mut report = ExportReport {
            index,
            object_name: boundary.object_name().to_owned(),
            class_path: boundary.class_path().to_owned(),
            component: boundary.component(),
            offset: boundary.offset(),
            length: boundary.length(),
            status: "unsupported",
            error: None,
            schema: None,
            property_bytes: None,
            native_suffix_bytes: None,
            leaves: Vec::new(),
        };

        let walked = (|| -> Result<_> {
            let export = package.export(index).context("ASSET_EXPORT")?;
            let schema_id = export
                .boundary()
                .resolve_class_schema(&schemas)
                .context("ASSET_SCHEMA")?;
            let block = PropertySpanWalker::g1r_ue5_4(&schemas)
                .walk(export.bytes(), schema_id)
                .context("ASSET_WALK")?;
            let descriptors =
                describe_fixed_leaves(&carrier, &export, &schemas).context("ASSET_SELECTOR")?;
            let property_bytes = block.consumed();
            let native_suffix_bytes = export
                .bytes()
                .len()
                .checked_sub(property_bytes)
                .context("ASSET_WALK: property range exceeds export")?;
            Ok((
                block.schema_name().to_owned(),
                property_bytes,
                native_suffix_bytes,
                descriptors,
            ))
        })();

        match walked {
            Ok((schema, property_bytes, native_suffix_bytes, descriptors)) => {
                report.status = "walked";
                report.schema = Some(schema);
                report.property_bytes = Some(property_bytes);
                report.native_suffix_bytes = Some(native_suffix_bytes);
                report.leaves = descriptors
                    .into_iter()
                    .enumerate()
                    .map(|(leaf_index, descriptor)| leaf_report(leaf_index, descriptor))
                    .collect();
            }
            Err(error) if args.export_index.is_none() => {
                report.error = Some(format!("{error:#}"));
            }
            Err(error) => return Err(error),
        }
        reports.push(report);
    }

    let walked_exports = reports
        .iter()
        .filter(|report| report.status == "walked")
        .count();
    let editable_leaves = reports
        .iter()
        .flat_map(|report| &report.leaves)
        .filter(|leaf| leaf.editable)
        .count();
    let status = if walked_exports == 0 {
        "unsupported"
    } else if walked_exports == reports.len() {
        "walked"
    } else {
        "partial"
    };

    let source = carrier
        .source_paths()
        .expect("a loaded package always retains canonical source paths");
    let seal = PackagePairSeal::capture(&carrier);
    let document = json!({
        "format": 1,
        "status": status,
        "summary": {
            "exports": reports.len(),
            "walked_exports": walked_exports,
            "editable_leaves": editable_leaves,
        },
        "selector_format": {
            "format": FIXED_LEAF_SELECTOR_FORMAT,
            "profile": FIXED_LEAF_SELECTOR_PROFILE,
        },
        "binding": {
            "package_seal": seal,
            "usmap_sha256": encode_hex(&usmap.sha256),
        },
        "input": {
            "uasset": source.uasset().display().to_string(),
            "uexp": source.uexp().display().to_string(),
            "uasset_length": carrier.len(PackageComponent::Uasset),
            "uexp_length": carrier.len(PackageComponent::Uexp),
        },
        "usmap": {
            "path": usmap.path.display().to_string(),
            "length": usmap.bytes.len(),
            "sha256": encode_hex(&usmap.sha256),
        },
        "exports": reports,
    });

    if args.json {
        println!("{}", serde_json::to_string_pretty(&document)?);
    } else {
        print_inspect_text(&document)?;
    }
    Ok(())
}

fn leaf_report(index: usize, descriptor: FixedLeafDescriptor) -> LeafReport {
    LeafReport {
        index,
        semantic_path: semantic_path(&descriptor.selector.path),
        editable: descriptor.editable,
        selector: descriptor.selector,
    }
}

fn print_inspect_text(document: &serde_json::Value) -> Result<()> {
    let binding = &document["binding"];
    println!(
        "SUMMARY\tstatus={}\texports={}\twalked_exports={}\teditable_leaves={}",
        document["status"].as_str().unwrap_or("unknown"),
        document["summary"]["exports"],
        document["summary"]["walked_exports"],
        document["summary"]["editable_leaves"],
    );
    println!(
        "BINDING\tprofile={}\tformat={}\tuasset_sha256={}\tuexp_sha256={}\tusmap_sha256={}",
        FIXED_LEAF_SELECTOR_PROFILE,
        FIXED_LEAF_SELECTOR_FORMAT,
        binding["package_seal"]["uasset_sha256"]
            .as_str()
            .context("serializing uasset seal")?,
        binding["package_seal"]["uexp_sha256"]
            .as_str()
            .context("serializing uexp seal")?,
        binding["usmap_sha256"]
            .as_str()
            .context("serializing USMAP seal")?,
    );
    for export in document["exports"]
        .as_array()
        .context("serializing exports")?
    {
        println!(
            "EXPORT\tindex={}\tobject={}\tclass={}\tcomponent={}\toffset={}\tlength={}\tstatus={}\terror={}",
            export["index"],
            serde_json::to_string(&export["object_name"] )?,
            serde_json::to_string(&export["class_path"] )?,
            export["component"].as_str().unwrap_or("unknown"),
            export["offset"],
            export["length"],
            export["status"].as_str().unwrap_or("unknown"),
            serde_json::to_string(&export["error"] )?,
        );
        if let Some(leaves) = export["leaves"].as_array() {
            for leaf in leaves {
                println!(
                    "LEAF\texport={}\tindex={}\tpath={}\tkind={}\texpected_hex={}\teditable={}\tselector={}",
                    export["index"],
                    leaf["index"],
                    serde_json::to_string(&leaf["semantic_path"] )?,
                    leaf["selector"]["kind"].as_str().unwrap_or("unknown"),
                    leaf["selector"]["expected_hex"].as_str().unwrap_or(""),
                    leaf["editable"],
                    serde_json::to_string(&leaf["selector"] )?,
                );
            }
        }
    }
    Ok(())
}

fn patch_fixed(args: PatchFixedArgs) -> Result<()> {
    let selector_input =
        read_verified_bounded(&args.selector, MAX_SELECTOR_BYTES, "ASSET_SELECTOR")?;
    let selector = parse_selector_document(&selector_input.bytes)?;
    let expected = decode_cli_hex(&args.expected_hex, selector.kind.width(), "ASSET_EXPECTED")?;
    let replacement = decode_cli_hex(
        &args.replacement_hex,
        selector.kind.width(),
        "ASSET_REPLACEMENT",
    )?;
    let expected_hex = encode_hex(&expected);
    if expected_hex != selector.expected_hex {
        bail!(
            "ASSET_EXPECTED: explicit expected bytes {expected_hex} do not match selector bytes {}",
            selector.expected_hex
        );
    }

    let usmap = read_verified_bounded(&args.usmap, MAX_USMAP_BYTES, "ASSET_USMAP")?;
    let schemas = SchemaDb::from_usmap(&usmap.bytes).context("ASSET_USMAP")?;
    let mut carrier =
        PackageCarrier::load(&args.uasset, PackageLimits::default()).context("ASSET_INPUT")?;

    let patch = {
        let package = LegacyPackageEnvelope::parse_g1r_ue5_4(&carrier).context("ASSET_ENVELOPE")?;
        let export = package
            .export(selector.export_index)
            .context("ASSET_EXPORT")?;
        let schema_id = export
            .boundary()
            .resolve_class_schema(&schemas)
            .context("ASSET_SCHEMA")?;
        let block = PropertySpanWalker::g1r_ue5_4(&schemas)
            .walk(export.bytes(), schema_id)
            .context("ASSET_WALK")?;
        let leaf = selector
            .resolve(&carrier, &export, &schemas)
            .context("ASSET_SELECTOR")?;
        FixedLeafPatch::plan(
            &carrier,
            &export,
            &schemas,
            &block,
            &leaf,
            &expected,
            &replacement,
        )
        .context("ASSET_REPLACEMENT")?
    };

    let patch_receipt = patch.apply(&mut carrier, &schemas).context("ASSET_DRIFT")?;
    let write_receipt = carrier.write_new(&args.out).context("ASSET_OUTPUT")?;

    let result = json!({
        "format": 1,
        "status": "patched",
        "input_selector": selector,
        "output_requires_reinspect": true,
        "expected_hex": expected_hex,
        "replacement_hex": encode_hex(&replacement),
        "patch": {
            "before": patch_receipt.before,
            "after": patch_receipt.after,
            "export_index": patch_receipt.export_index,
            "component": patch_receipt.component,
            "absolute_offset": patch_receipt.absolute_offset,
            "length": patch_receipt.length,
            "kind": patch_receipt.kind,
        },
        "output": {
            "uasset": digest_json(&write_receipt.uasset),
            "uexp": digest_json(&write_receipt.uexp),
        },
    });
    if args.json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!(
            "PATCHED\texport={}\tkind={}\texpected_hex={}\treplacement_hex={}\tcomponent={}\toffset={}\tlength={}",
            patch_receipt.export_index,
            kind_name(patch_receipt.kind),
            expected_hex,
            encode_hex(&replacement),
            patch_receipt.component,
            patch_receipt.absolute_offset,
            patch_receipt.length,
        );
        for component in [&write_receipt.uexp, &write_receipt.uasset] {
            println!(
                "WROTE\tpath={}\tlength={}\tsha256={}",
                serde_json::to_string(&component.path.display().to_string())?,
                component.length,
                encode_hex(&component.sha256),
            );
        }
        println!("NOTICE\toutput_requires_reinspect=true");
    }
    Ok(())
}

fn digest_json(digest: &gore_asset::ComponentDigest) -> serde_json::Value {
    json!({
        "path": digest.path.display().to_string(),
        "length": digest.length,
        "sha256": encode_hex(&digest.sha256),
    })
}

fn parse_selector_document(bytes: &[u8]) -> Result<FixedLeafSelector> {
    match serde_json::from_slice::<FixedLeafSelector>(bytes) {
        Ok(selector) => Ok(selector),
        Err(selector_error) => {
            match serde_json::from_slice::<FixedLeafDescriptor>(bytes) {
                Ok(descriptor) => Ok(descriptor.selector),
                Err(descriptor_error) => {
                    match serde_json::from_slice::<InspectLeafDocument>(bytes) {
                    Ok(leaf) => Ok(leaf.selector),
                    Err(leaf_error) => bail!(
                        "ASSET_SELECTOR: expected FixedLeafSelector, FixedLeafDescriptor, or inspect leaf JSON; selector error: {selector_error}; descriptor error: {descriptor_error}; inspect leaf error: {leaf_error}"
                    ),
                }
                }
            }
        }
    }
}

#[derive(Debug, Deserialize)]
struct InspectLeafDocument {
    selector: FixedLeafSelector,
}

fn decode_cli_hex(value: &str, expected_bytes: usize, code: &'static str) -> Result<Vec<u8>> {
    if value.len() != expected_bytes.saturating_mul(2) {
        bail!(
            "{code}: expected {} hex characters, got {}",
            expected_bytes.saturating_mul(2),
            value.len()
        );
    }
    if !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        bail!("{code}: value must be contiguous ASCII hex without prefixes or separators");
    }
    Ok(value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| (hex_nibble(pair[0]) << 4) | hex_nibble(pair[1]))
        .collect())
}

fn semantic_path(path: &[FixedLeafSelectorStep]) -> String {
    let mut result = String::new();
    for step in path {
        match step {
            FixedLeafSelectorStep::Property {
                property_name,
                array_index,
                array_dimension,
                ..
            } => {
                result.push('/');
                result.push_str(property_name);
                if *array_dimension > 1 {
                    result.push('[');
                    result.push_str(&array_index.to_string());
                    result.push(']');
                }
            }
            FixedLeafSelectorStep::Struct { name, .. } => {
                result.push_str("/struct:");
                result.push_str(name);
            }
            FixedLeafSelectorStep::Map { .. } => result.push_str("/map"),
            FixedLeafSelectorStep::MapEntryValue { key } => {
                result.push_str("/value:key=");
                result.push_str(&key.sha256[..key.sha256.len().min(12)]);
            }
            FixedLeafSelectorStep::MapEntryKey { key } => {
                result.push_str("/key=");
                result.push_str(&key.sha256[..key.sha256.len().min(12)]);
            }
            FixedLeafSelectorStep::RemovedMapKey { key } => {
                result.push_str("/removed-key=");
                result.push_str(&key.sha256[..key.sha256.len().min(12)]);
            }
        }
    }
    if result.is_empty() {
        "/".to_owned()
    } else {
        result
    }
}

fn kind_name(kind: gore_asset::FixedWireKind) -> &'static str {
    match kind {
        gore_asset::FixedWireKind::Byte => "byte",
        gore_asset::FixedWireKind::Bool => "bool",
        gore_asset::FixedWireKind::Int32 => "int32",
        gore_asset::FixedWireKind::Float32 => "float32",
        gore_asset::FixedWireKind::PackageIndex => "package_index",
        gore_asset::FixedWireKind::FName => "fname",
        gore_asset::FixedWireKind::Float64 => "float64",
        gore_asset::FixedWireKind::UInt64 => "uint64",
        gore_asset::FixedWireKind::UInt32 => "uint32",
        gore_asset::FixedWireKind::UInt16 => "uint16",
        gore_asset::FixedWireKind::Int64 => "int64",
        gore_asset::FixedWireKind::Int16 => "int16",
        gore_asset::FixedWireKind::Int8 => "int8",
        gore_asset::FixedWireKind::LinearColorF32x4 => "linear_color_f32x4",
        gore_asset::FixedWireKind::Vector4F64x4 => "vector4_f64x4",
    }
}

fn read_verified_bounded(path: &Path, limit: u64, code: &'static str) -> Result<VerifiedInput> {
    let link_metadata = fs::symlink_metadata(path)
        .with_context(|| format!("{code}: inspecting '{}'", path.display()))?;
    if link_metadata.file_type().is_symlink() || !link_metadata.is_file() {
        bail!(
            "{code}: input is not a regular non-symlink file: {}",
            path.display()
        );
    }
    let canonical = fs::canonicalize(path)
        .with_context(|| format!("{code}: canonicalizing '{}'", path.display()))?;
    let mut file = File::open(&canonical)
        .with_context(|| format!("{code}: opening '{}'", canonical.display()))?;
    let advertised = file
        .metadata()
        .with_context(|| format!("{code}: reading metadata for '{}'", canonical.display()))?
        .len();
    if advertised > limit {
        bail!(
            "{code}: '{}' is {advertised} bytes; limit is {limit}",
            canonical.display()
        );
    }
    let allocation = usize::try_from(advertised)
        .with_context(|| format!("{code}: input length does not fit memory"))?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(allocation)
        .with_context(|| format!("{code}: reserving {allocation} bytes"))?;
    (&mut file)
        .take(limit.saturating_add(1))
        .read_to_end(&mut bytes)
        .with_context(|| format!("{code}: reading '{}'", canonical.display()))?;
    if u64::try_from(bytes.len())? != advertised {
        bail!(
            "{code}: input changed length while being read: {}",
            canonical.display()
        );
    }
    let sha256: [u8; 32] = Sha256::digest(&bytes).into();
    verify_file_hash(&canonical, advertised, sha256, limit, code)?;
    Ok(VerifiedInput {
        path: canonical,
        bytes,
        sha256,
    })
}

fn verify_file_hash(
    path: &Path,
    expected_length: u64,
    expected_sha256: [u8; 32],
    limit: u64,
    code: &'static str,
) -> Result<()> {
    let metadata = fs::symlink_metadata(path)
        .with_context(|| format!("{code}: re-inspecting '{}'", path.display()))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        bail!(
            "{code}: input changed to a non-regular file: {}",
            path.display()
        );
    }
    let mut file =
        File::open(path).with_context(|| format!("{code}: reopening '{}'", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    let mut actual_length = 0u64;
    loop {
        let read = file
            .read(&mut buffer)
            .with_context(|| format!("{code}: reverifying '{}'", path.display()))?;
        if read == 0 {
            break;
        }
        actual_length = actual_length
            .checked_add(u64::try_from(read)?)
            .context("verified input length overflowed")?;
        if actual_length > limit {
            bail!("{code}: input grew beyond {limit} bytes while being reverified");
        }
        hasher.update(&buffer[..read]);
    }
    let actual_sha256: [u8; 32] = hasher.finalize().into();
    if actual_length != expected_length || actual_sha256 != expected_sha256 {
        bail!("{code}: input changed while being read: {}", path.display());
    }
    Ok(())
}

fn hex_nibble(byte: u8) -> u8 {
    match byte {
        b'0'..=b'9' => byte - b'0',
        b'a'..=b'f' => byte - b'a' + 10,
        b'A'..=b'F' => byte - b'A' + 10,
        _ => unreachable!("hex input was validated before decoding"),
    }
}

fn encode_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        write!(&mut out, "{byte:02x}").expect("writing to String cannot fail");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_hex_is_full_width_and_canonicalized() {
        assert_eq!(decode_cli_hex("00AaFf", 3, "HEX").unwrap(), [0, 0xaa, 0xff]);
        assert!(decode_cli_hex("0x00", 2, "HEX").is_err());
        assert!(decode_cli_hex("0 00", 2, "HEX").is_err());
        assert!(decode_cli_hex("000", 2, "HEX").is_err());
    }

    #[test]
    fn bounded_reader_rejects_symlinks_or_oversize_and_detects_stable_input() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("selector.json");
        fs::write(&path, b"{}").unwrap();
        let input = read_verified_bounded(&path, 2, "TEST").unwrap();
        assert_eq!(input.bytes, b"{}");
        let expected_sha256: [u8; 32] = Sha256::digest(b"{}").into();
        assert_eq!(input.sha256, expected_sha256);
        assert!(read_verified_bounded(&path, 1, "TEST").is_err());
    }
}
