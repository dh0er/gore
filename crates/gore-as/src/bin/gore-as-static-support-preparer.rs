use std::fs::{self, OpenOptions};
use std::io::Write as _;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context as _, Result};
use gore_as::compiler_profile::binds::BindsDatabase;
use gore_as::compiler_profile::capture::{
    decode_capture_v1, PinnedSupportBlobV1, StaticProfileSupportManifestV1,
    StaticSupportPayloadSealsV1, STATIC_SUPPORT_MANIFEST_SCHEMA_V1,
    STATIC_SUPPORT_MANIFEST_SCHEMA_VERSION_V1,
};
use gore_as::compiler_profile::manifest::{
    BindsProfileV1, CompilerArchitectureV1, CompilerBuildConfigurationV1, CompilerOracleV1,
    CompilerPlatformV1, CompilerTargetV1, FileSealV1, PeCodeViewV1, Sha1Digest, Sha256Digest,
};
use gore_as::compiler_profile::qualification_suite::{
    full_qualification_corpus_v1, FULL_QUALIFICATION_SUITE_ID_V1,
};
use sha1::Sha1;
use sha2::{Digest as _, Sha256};

const USAGE: &str = "usage: gore-as-static-support-preparer \
<sealed.capture> <G1R-Win64-Shipping.exe> <Binds.Cache> \
<PrecompiledScript_Shipping.Cache> <depot.manifest> <probe.PrecompiledScript.Cache> \
<new-output-root>";

fn main() {
    if let Err(error) = run() {
        eprintln!("static support preparation failed: {error:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let mut args = std::env::args_os().skip(1).map(PathBuf::from);
    let capture_path = args.next().context(USAGE)?;
    let executable_path = args.next().context(USAGE)?;
    let binds_path = args.next().context(USAGE)?;
    let shipping_path = args.next().context(USAGE)?;
    let depot_manifest_path = args.next().context(USAGE)?;
    let probe_cache_path = args.next().context(USAGE)?;
    let output_root = args.next().context(USAGE)?;
    if args.next().is_some() {
        bail!(USAGE);
    }
    for (label, path) in [
        ("capture", &capture_path),
        ("executable", &executable_path),
        ("binds", &binds_path),
        ("shipping cache", &shipping_path),
        ("depot manifest", &depot_manifest_path),
        ("probe cache", &probe_cache_path),
        ("output root", &output_root),
    ] {
        require_absolute_normalized(path, label)?;
    }

    let capture_bytes = fs::read(&capture_path).context("reading sealed capture")?;
    let decoded = decode_capture_v1(&capture_bytes).context("decoding sealed capture")?;
    let binds_bytes = fs::read(&binds_path).context("reading Binds.Cache")?;
    let binds_database = BindsDatabase::parse(&binds_bytes).context("parsing Binds.Cache")?;
    let probe_cache = fs::read(&probe_cache_path).context("reading game-oracle probe cache")?;
    if probe_cache.is_empty() {
        bail!("game-oracle probe cache is empty");
    }

    fs::create_dir(&output_root).context("creating new static-support root")?;
    let payload_root = output_root.join("payloads");
    fs::create_dir(&payload_root).context("creating static-support payload root")?;

    let native_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("native/standalone-compiler");
    let corpus = full_qualification_corpus_v1().context("building canonical probe corpus")?;
    let corpus_json = corpus
        .to_json()
        .context("serializing canonical probe corpus")?;
    let pending = |kind: &str| {
        serde_json::to_vec_pretty(&serde_json::json!({
            "schema": "gore.as.unqualified-static-support-pending-parity",
            "schema_version": 1,
            "kind": kind,
            "qualified": false,
            "state": "pending_differential_oracle",
            "capture_stream_sha256": decoded.sealed_stream_sha256,
            "corpus_sha256": corpus.canonical_sha256,
        }))
        .expect("serializing fixed pending-parity marker cannot fail")
    };

    let payloads = [
        ("reflected-type-graph.bin", binds_bytes.clone()),
        (
            "opcode-table.bin",
            fs::read(native_root.join("vendor/unreangel/source/as_bytecode.cpp"))
                .context("reading pinned donor opcode table")?,
        ),
        (
            "operand-schema.bin",
            fs::read(native_root.join("vendor/unreangel/include/angelscript.h"))
                .context("reading pinned donor operand schema")?,
        ),
        ("codegen-probe-corpus.json", corpus_json),
        (
            "expected-probe-results.json",
            pending("expected_probe_results"),
        ),
        (
            "serializer-schema.bin",
            fs::read(native_root.join("src/precompiled_data.cpp"))
                .context("reading pinned cache serializer implementation")?,
        ),
        (
            "reference-table-order.bin",
            fs::read(native_root.join("src/precompiled_metadata.cpp"))
                .context("reading pinned reference-table implementation")?,
        ),
        ("normalized-oracle-corpus.bin", probe_cache),
        ("diagnostic-parity.json", pending("diagnostic_parity")),
        ("semantic-parity.json", pending("semantic_parity")),
    ];

    let mut seals = Vec::with_capacity(payloads.len());
    for (name, bytes) in payloads {
        write_new(&payload_root.join(name), &bytes).with_context(|| format!("writing {name}"))?;
        seals.push(PinnedSupportBlobV1 {
            byte_len: bytes.len() as u64,
            sha256: sha256(&bytes),
        });
    }
    let payloads = StaticSupportPayloadSealsV1 {
        reflected_type_graph: seals[0],
        opcode_table: seals[1],
        operand_schema: seals[2],
        codegen_probe_corpus: seals[3],
        expected_probe_results: seals[4],
        serializer_schema: seals[5],
        reference_table_order: seals[6],
        normalized_oracle_corpus: seals[7],
        diagnostic_parity: seals[8],
        semantic_parity: seals[9],
    };

    let manifest = StaticProfileSupportManifestV1 {
        schema: STATIC_SUPPORT_MANIFEST_SCHEMA_V1.to_owned(),
        schema_version: STATIC_SUPPORT_MANIFEST_SCHEMA_VERSION_V1,
        target: CompilerTargetV1 {
            steam_app_id: 1_297_900,
            steam_build_id: 24_539_464,
            depot_id: 1_297_901,
            depot_manifest_gid: 1_585_071_322_101_748_861,
            platform: CompilerPlatformV1::Windows,
            architecture: CompilerArchitectureV1::X86_64,
            build_configuration: CompilerBuildConfigurationV1::Shipping,
        },
        oracle: CompilerOracleV1 {
            executable: file_seal(&executable_path, true)?,
            binds_cache: file_seal(&binds_path, true)?,
            shipping_cache: file_seal(&shipping_path, true)?,
            depot_manifest: file_seal(&depot_manifest_path, false)?,
            pe_codeview: PeCodeViewV1 {
                guid: "cf0b83bd-e023-061b-2100-0f0fccf871d2".to_owned(),
                age: 1,
            },
        },
        binds: BindsProfileV1::from_database(&binds_database),
        unreal_metadata_schema_version: 1,
        opcode_table_version: "unreangel-g1r-build24539464-asbc-212-v1".to_owned(),
        cache_format_version: 1,
        required_probe_suite_version: FULL_QUALIFICATION_SUITE_ID_V1.to_owned(),
        payloads,
    };
    let manifest_json = manifest
        .to_json_pretty()
        .context("validating static-support manifest")?;
    write_new(&output_root.join("static-support.json"), &manifest_json)
        .context("writing static-support manifest")?;
    println!(
        "{{\"output_root\":{},\"capture_stream_sha256\":{},\"corpus_sha256\":{},\"bind_structs\":{},\"bind_classes\":{},\"bind_methods\":{}}}",
        serde_json::to_string(&output_root)?,
        serde_json::to_string(&decoded.sealed_stream_sha256)?,
        serde_json::to_string(&corpus.canonical_sha256)?,
        binds_database.structs.len(),
        binds_database.classes.len(),
        binds_database.method_count(),
    );
    Ok(())
}

fn file_seal(path: &Path, steam_content: bool) -> Result<FileSealV1> {
    let bytes = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    Ok(FileSealV1 {
        byte_len: bytes.len() as u64,
        sha256: sha256(&bytes),
        steam_content_sha1: steam_content.then(|| {
            let digest: [u8; 20] = Sha1::digest(&bytes).into();
            Sha1Digest::from_bytes(digest)
        }),
    })
}

fn sha256(bytes: &[u8]) -> Sha256Digest {
    Sha256Digest::from_bytes(Sha256::digest(bytes).into())
}

fn write_new(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

fn require_absolute_normalized(path: &Path, label: &str) -> Result<()> {
    if !path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                std::path::Component::CurDir | std::path::Component::ParentDir
            )
        })
    {
        bail!(
            "{label} path must be absolute and normalized: {}",
            path.display()
        );
    }
    Ok(())
}
