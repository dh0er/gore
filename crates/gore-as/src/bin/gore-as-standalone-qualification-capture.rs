use std::fs::{self, OpenOptions};
use std::io::Write as _;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context as _, Result};
use gore_as::compiler_profile::manifest::Sha256Digest;
use gore_as::compiler_profile::qualification::QualifiedSidecarIdentityV1;
use gore_as::compiler_profile::qualification_runner::CompilerProbeBackendKindV1;
use gore_as::compiler_profile::qualification_suite::{
    capture_and_seal_offline_qualification_artifacts_v1, full_qualification_corpus_v1,
    OfflineQualificationCaptureBackendV1,
};
use gore_as::compiler_profile::standalone_qualification::{
    unqualified_profile_manifest_path_v1, StandaloneQualificationHarnessConfigV1,
    StandaloneQualificationHarnessV1,
};
use gore_as::standalone_sidecar::{
    SidecarExecutableSealV1, StandaloneSidecarConfigV1, SIDECAR_REQUEST_VERSION_V2,
    SIDECAR_RESPONSE_VERSION_V1,
};
use sha2::{Digest as _, Sha256};

const USAGE: &str = "usage: gore-as-standalone-qualification-capture \
<sidecar.exe> <unqualified-profile-root> <PrecompiledScript_Shipping.Cache> \
<Binds.Cache> <new-output-root>";

fn main() {
    if let Err(error) = run() {
        eprintln!("standalone qualification capture failed: {error:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let mut args = std::env::args_os().skip(1).map(PathBuf::from);
    let sidecar_path = args.next().context(USAGE)?;
    let profile_root = args.next().context(USAGE)?;
    let base_cache_path = args.next().context(USAGE)?;
    let binds_cache_path = args.next().context(USAGE)?;
    let output_root = args.next().context(USAGE)?;
    if args.next().is_some() {
        bail!(USAGE);
    }
    for (label, path) in [
        ("sidecar", &sidecar_path),
        ("profile root", &profile_root),
        ("base cache", &base_cache_path),
        ("binds cache", &binds_cache_path),
        ("output root", &output_root),
    ] {
        require_absolute_normalized(path, label)?;
    }

    let sidecar_bytes = fs::read(&sidecar_path).context("reading sidecar")?;
    let sidecar_sha256 = Sha256Digest::from_bytes(Sha256::digest(&sidecar_bytes).into());
    let seal = SidecarExecutableSealV1 {
        byte_len: sidecar_bytes.len() as u64,
        sha256: sidecar_sha256,
    };
    let authority = QualifiedSidecarIdentityV1 {
        byte_len: seal.byte_len,
        sha256: seal.sha256,
        request_version: SIDECAR_REQUEST_VERSION_V2,
        response_version: SIDECAR_RESPONSE_VERSION_V1,
    };
    fs::create_dir(&output_root).context("creating qualification output root")?;
    let scratch_root = output_root.join("scratch");
    fs::create_dir(&scratch_root).context("creating sidecar scratch root")?;
    let sidecar = StandaloneSidecarConfigV1::new(
        sidecar_path,
        seal,
        unqualified_profile_manifest_path_v1(&profile_root),
        profile_root,
        scratch_root,
    );
    let mut harness =
        StandaloneQualificationHarnessV1::new(StandaloneQualificationHarnessConfigV1 {
            sidecar,
            sidecar_authority: authority,
            base_cache_path,
            binds_cache_path,
        })
        .context("opening standalone qualification harness")?;
    let corpus = full_qualification_corpus_v1().context("building canonical corpus")?;
    if let Ok(case_id) = std::env::var("GORE_AS_QUALIFICATION_DEBUG_CASE") {
        let case = corpus
            .cases
            .iter()
            .find(|case| case.case_id == case_id)
            .with_context(|| format!("unknown debug qualification case {case_id:?}"))?;
        harness
            .capture_probe(case)
            .map_err(anyhow::Error::msg)
            .with_context(|| format!("capturing debug qualification case {case_id:?}"))?;
        println!(
            "{{\"debug_case\":{},\"accepted\":true}}",
            serde_json::to_string(&case_id)?
        );
        return Ok(());
    }
    let artifacts = capture_and_seal_offline_qualification_artifacts_v1(
        &corpus,
        CompilerProbeBackendKindV1::Standalone,
        &mut harness,
    )
    .context("capturing standalone corpus")?;

    write_new(
        &output_root.join("standalone-qualification-artifacts.json"),
        artifacts.manifest_json(),
    )?;
    for (name, bytes) in artifacts.cache_blobs() {
        write_new(&output_root.join(name), bytes)
            .with_context(|| format!("writing cache artifact {name}"))?;
    }
    let authority_summary = artifacts.authority_summary()?;
    println!(
        "{{\"output_root\":{},\"sidecar_sha256\":{},\"manifest_sha256\":{},\"cache_artifacts\":{}}}",
        serde_json::to_string(&output_root)?,
        serde_json::to_string(&sidecar_sha256)?,
        serde_json::to_string(&authority_summary.manifest_canonical_sha256)?,
        artifacts.cache_blobs().len(),
    );
    Ok(())
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
