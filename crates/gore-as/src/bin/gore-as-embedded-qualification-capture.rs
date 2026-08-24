use std::fs::{self, OpenOptions};
use std::io::Write as _;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context as _, Result};
use gore_as::cache::semantic_observer::observe_whole_cache_semantics_v1;
use gore_as::compiler_profile::embedded_qualification::{
    EmbeddedQualificationHarnessConfigV1, EmbeddedQualificationHarnessV1,
};
use gore_as::compiler_profile::manifest::Sha256Digest;
use gore_as::compiler_profile::qualification::QualifiedSidecarIdentityV1;
use gore_as::compiler_profile::qualification_runner::CompilerProbeBackendKindV1;
use gore_as::compiler_profile::qualification_suite::{
    capture_and_seal_offline_qualification_artifacts_v1, full_qualification_corpus_v1,
    OfflineQualificationCaptureBackendV1,
};
use gore_as::compiler_profile::standalone_qualification::unqualified_profile_manifest_path_v1;
use gore_as::diagnostics::DiagnosticsOptions;
use gore_as::standalone_sidecar::{
    SidecarExecutableSealV1, StandaloneSidecarConfigV1, SIDECAR_REQUEST_VERSION_V2,
    SIDECAR_RESPONSE_VERSION_V1,
};
use sha2::{Digest as _, Sha256};

const USAGE: &str = "usage: gore-as-embedded-qualification-capture \
<unqualified-profile-root> <game-dir> <G1R-Win64-Shipping.exe> \
<PrecompiledScript_Shipping.Cache> <Binds.Cache> <authority.capture> \
<capture-controller.exe> <capture-bridge.dll> <diagnostics-hook.dll> \
<invoke-observer-sidecar.exe> <new-output-root>";

fn main() {
    if let Err(error) = run() {
        eprintln!("embedded qualification capture failed: {error:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let mut args = std::env::args_os().skip(1).map(PathBuf::from);
    let profile_root = args.next().context(USAGE)?;
    let game_dir = args.next().context(USAGE)?;
    let executable_path = args.next().context(USAGE)?;
    let shipping_cache_path = args.next().context(USAGE)?;
    let binds_cache_path = args.next().context(USAGE)?;
    let authority_capture_path = args.next().context(USAGE)?;
    let capture_controller_path = args.next().context(USAGE)?;
    let capture_bridge_path = args.next().context(USAGE)?;
    let diagnostics_hook_path = args.next().context(USAGE)?;
    let sidecar_path = args.next().context(USAGE)?;
    let output_root = args.next().context(USAGE)?;
    if args.next().is_some() {
        bail!(USAGE);
    }
    for (label, path) in [
        ("profile root", &profile_root),
        ("game directory", &game_dir),
        ("executable", &executable_path),
        ("Shipping cache", &shipping_cache_path),
        ("Binds cache", &binds_cache_path),
        ("authority capture", &authority_capture_path),
        ("capture controller", &capture_controller_path),
        ("capture bridge", &capture_bridge_path),
        ("diagnostics hook", &diagnostics_hook_path),
        ("invoke observer", &sidecar_path),
        ("output root", &output_root),
    ] {
        require_absolute_normalized(path, label)?;
    }
    let sidecar_bytes = fs::read(&sidecar_path).context("reading invoke observer sidecar")?;
    let sidecar_sha256 = Sha256Digest::from_bytes(Sha256::digest(&sidecar_bytes).into());
    let sidecar_seal = SidecarExecutableSealV1 {
        byte_len: sidecar_bytes.len() as u64,
        sha256: sidecar_sha256,
    };
    let sidecar_authority = QualifiedSidecarIdentityV1 {
        byte_len: sidecar_seal.byte_len,
        sha256: sidecar_seal.sha256,
        request_version: SIDECAR_REQUEST_VERSION_V2,
        response_version: SIDECAR_RESPONSE_VERSION_V1,
    };
    fs::create_dir(&output_root).context("creating embedded qualification output root")?;
    let scratch_root = output_root.join("scratch");
    let sidecar_scratch_root = output_root.join("invoke-observer-scratch");
    fs::create_dir(&scratch_root).context("creating embedded qualification scratch")?;
    fs::create_dir(&sidecar_scratch_root).context("creating invoke observer scratch")?;
    let invoke_observer = StandaloneSidecarConfigV1::new(
        sidecar_path,
        sidecar_seal,
        unqualified_profile_manifest_path_v1(&profile_root),
        profile_root.clone(),
        sidecar_scratch_root,
    );
    let mut harness = EmbeddedQualificationHarnessV1::new(EmbeddedQualificationHarnessConfigV1 {
        profile_root,
        game_dir,
        executable_path,
        shipping_cache_path,
        binds_cache_path,
        authority_capture_path,
        capture_controller_path,
        capture_bridge_path,
        diagnostics: DiagnosticsOptions {
            hook_dll: Some(diagnostics_hook_path),
            ..DiagnosticsOptions::default()
        },
        scratch_root,
        invoke_observer,
        invoke_observer_authority: sidecar_authority,
    })
    .context("opening embedded qualification harness")?;
    let corpus = full_qualification_corpus_v1().context("building canonical corpus")?;
    if let Ok(case_id) = std::env::var("GORE_AS_QUALIFICATION_DEBUG_CASE") {
        let case = corpus
            .cases
            .iter()
            .find(|case| case.case_id == case_id)
            .with_context(|| format!("unknown debug qualification case {case_id:?}"))?;
        let captured = harness
            .capture_probe(case)
            .map_err(anyhow::Error::msg)
            .with_context(|| format!("capturing embedded debug case {case_id:?}"))?;
        let artifact = captured
            .observation()
            .accepted_artifact()
            .context("embedded debug case did not return an accepted cache")?;
        let semantics = observe_whole_cache_semantics_v1(artifact.cache_bytes(), None)
            .context("observing embedded debug cache")?;
        let opcode_counts = [
            "FinConstruct",
            "CopyScript",
            "FreeNullV8",
            "CpyVtoR1",
            "CmpPtrNull",
            "ThrowException",
            "DestructScript",
            "TrackRef",
            "UntrackRef",
            "ValidateRef",
            "SaveReturnValue",
            "ResolveObjectPtr",
        ]
        .into_iter()
        .map(|opcode| {
            (
                opcode.to_owned(),
                serde_json::json!(semantics.opcode_count_named(opcode)),
            )
        })
        .collect::<serde_json::Map<_, _>>();
        println!(
            "{}",
            serde_json::to_string(&serde_json::json!({
                "debug_case": case_id,
                "accepted": true,
                "cache_byte_len": artifact.cache_bytes().len(),
                "opcode_counts": opcode_counts,
                "tail_table_counts": semantics.tail_table_counts(),
                "model_counts": {
                    "classes": semantics.class_count(),
                    "behaviour_functions": semantics.behaviour_function_count(),
                    "properties": semantics.property_count(),
                    "globals": semantics.global_count(),
                    "initializer_functions": semantics.initializer_function_count(),
                },
                "compiler_build_flags": captured.compiler_build_flags(),
            }))?
        );
        return Ok(());
    }
    let artifacts = capture_and_seal_offline_qualification_artifacts_v1(
        &corpus,
        CompilerProbeBackendKindV1::EmbeddedGame,
        &mut harness,
    )
    .context("capturing embedded corpus")?;
    write_new(
        &output_root.join("embedded-qualification-artifacts.json"),
        artifacts.manifest_json(),
    )?;
    for (name, bytes) in artifacts.cache_blobs() {
        write_new(&output_root.join(name), bytes)
            .with_context(|| format!("writing cache artifact {name}"))?;
    }
    let authority = artifacts.authority_summary()?;
    println!(
        "{{\"output_root\":{},\"manifest_sha256\":{},\"cache_artifacts\":{}}}",
        serde_json::to_string(&output_root)?,
        serde_json::to_string(&authority.manifest_canonical_sha256)?,
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
