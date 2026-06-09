#[cfg(windows)]
use goresave_g1r_codec_host::WindowsImportResolver;
use goresave_g1r_codec_host::{
    DerivedProfileCacheEntry, ErrorCode, HostError, ImportResolver, PeImage, PeImportSymbol,
    PrivateMappingReport, ProbeRequest, ResolutionMode, ResolvedRvaReport, RuntimeCodecRvas,
    RuntimeCompressSample, RuntimeSelftestOracleSample, RuntimeSelftestReport,
    RuntimeSelftestSaveChunkRequest, RuntimeSelftestWorkerRequest, SectionProtection,
    SelfTestRequest, SelfTestResponse, derived_profile_entry_from_verified_self_test,
    export_derived_profile_from_cache, handle_ipc_line, handle_ipc_line_with_runtime_worker,
    parse_profile_json, probe_exe, probe_exe_with_derived_cache,
    record_derived_profile_cache_after_self_test, run_runtime_selftest_worker,
    run_runtime_selftest_worker_with_request, runtime_selftest_sample_from_save_chunk,
    runtime_selftest_worker_report, self_test_exe, self_test_exe_with_import_resolver,
    write_derived_profile_cache_entry,
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[test]
fn profile_parser_accepts_valid_profile() {
    let profile = parse_profile_json(&profile_json(
        "00",
        4096,
        "0x23A85CE7",
        "0x140000000",
        "0x1010",
        "0x1020",
        "0x1030",
    ))
    .unwrap();

    assert_eq!(profile.name, "g1r-test");
    assert_eq!(profile.exe_sha256, "00");
    assert_eq!(profile.file_size, 4096);
    assert_eq!(profile.pe_timestamp, 0x23A85CE7);
    assert_eq!(profile.image_base, 0x140000000);
    assert_eq!(profile.rv_as.oodle_lz_compress, 0x1010);
    assert_eq!(profile.rv_as.oodle_lz_decompress, 0x1020);
    assert_eq!(profile.rv_as.compressor_dispatch, 0x1030);
}

#[test]
fn profile_parser_rejects_missing_required_fields() {
    let err = parse_profile_json(r#"{"name":"g1r-test"}"#).unwrap_err();

    assert_eq!(err.code(), ErrorCode::InvalidProfile);
}

#[test]
fn pe_parser_rejects_non_pe_files() {
    let err = PeImage::parse(b"not a pe").unwrap_err();

    assert_eq!(err.code(), ErrorCode::InvalidPe);
}

#[test]
fn pe_parser_resolves_executable_rva_ranges() {
    let pe = PeImage::parse(&minimal_pe64_with_text_section()).unwrap();

    assert_eq!(pe.image_base(), 0x140000000);
    assert_eq!(pe.timestamp(), 0x23A85CE7);
    assert!(pe.is_executable_rva(0x1000));
    assert!(pe.is_executable_rva(0x1fff));
    assert!(!pe.is_executable_rva(0x2000));
}

#[test]
fn pe_parser_reads_import_descriptors() {
    let exe = minimal_pe64_with_imports_and_relocations();
    let pe = PeImage::parse(&exe).unwrap();

    let imports = pe.imports(&exe).unwrap();

    assert_eq!(imports.len(), 1);
    assert_eq!(imports[0].dll_name, "KERNEL32.dll");
    assert_eq!(imports[0].symbols.len(), 1);
    assert_eq!(
        imports[0].symbols[0].name.as_deref(),
        Some("GetProcAddress")
    );
    assert_eq!(imports[0].symbols[0].hint, Some(7));
}

#[test]
fn pe_parser_reads_base_relocation_blocks() {
    let exe = minimal_pe64_with_imports_and_relocations();
    let pe = PeImage::parse(&exe).unwrap();

    let relocations = pe.base_relocations(&exe).unwrap();

    assert_eq!(relocations.len(), 1);
    assert_eq!(relocations[0].page_rva, 0x1000);
    assert_eq!(relocations[0].entries.len(), 2);
    assert_eq!(relocations[0].entries[0].kind, 10);
    assert_eq!(relocations[0].entries[0].offset, 0x18);
    assert_eq!(relocations[0].entries[1].kind, 0);
}

#[test]
fn private_image_copy_applies_dir64_relocations_without_running_entry_point() {
    let exe = minimal_pe64_with_imports_and_relocations();
    let pe = PeImage::parse(&exe).unwrap();
    let mut image = pe.copy_image(&exe).unwrap();

    assert_eq!(image.entry_point_rva(), 0x1000);
    assert!(!image.entry_point_was_run());
    assert_eq!(image.read_u64(0x1018).unwrap(), 0x140001234);

    let applied = image
        .apply_base_relocations(&pe, &exe, 0x150000000)
        .unwrap();

    assert_eq!(applied, 1);
    assert_eq!(image.read_u64(0x1018).unwrap(), 0x150001234);
    assert!(!image.entry_point_was_run());
}

#[test]
fn private_image_resolves_imports_into_iat_with_injected_resolver() {
    let exe = minimal_pe64_with_imports_and_relocations();
    let pe = PeImage::parse(&exe).unwrap();
    let mut image = pe.copy_image(&exe).unwrap();
    let mut resolver = RecordingImportResolver::default();

    let report = image.resolve_imports(&pe, &exe, &mut resolver).unwrap();

    assert_eq!(report.dll_count, 1);
    assert_eq!(report.symbol_count, 1);
    assert_eq!(report.fixed_thunk_count, 1);
    assert_eq!(
        resolver.calls,
        vec![("KERNEL32.dll".to_string(), "GetProcAddress".to_string())]
    );
    assert_eq!(image.read_u64(0x2070).unwrap(), 0x180012340);
}

#[test]
fn pe_section_protection_plan_tracks_execute_read_write_flags() {
    let exe = minimal_pe64_with_imports_and_relocations();
    let pe = PeImage::parse(&exe).unwrap();

    let protections = pe.section_protections();

    assert_eq!(protections.len(), 3);
    assert_eq!(protections[0].name, ".text");
    assert!(protections[0].read);
    assert!(protections[0].execute);
    assert!(!protections[0].write);
    assert_eq!(protections[0].memory_protection, "execute_read");
    assert_eq!(protections[1].name, ".idata");
    assert!(protections[1].read);
    assert!(!protections[1].execute);
    assert!(!protections[1].write);
    assert_eq!(protections[1].memory_protection, "read_only");
}

#[test]
fn private_image_resolves_executable_rvas_to_mapped_addresses() {
    let exe = minimal_pe64_with_imports_and_relocations();
    let pe = PeImage::parse(&exe).unwrap();
    let mut image = pe.copy_image(&exe).unwrap();
    image
        .apply_base_relocations(&pe, &exe, 0x150000000)
        .unwrap();

    let mapped = image
        .resolve_executable_rva(&pe, "oodleLzCompress", 0x1010)
        .unwrap();

    assert_eq!(mapped.name, "oodleLzCompress");
    assert_eq!(mapped.rva, 0x1010);
    assert_eq!(mapped.mapped_va, 0x150001010);
    assert!(mapped.executable);
}

#[cfg(windows)]
#[test]
fn pe_private_mapper_allocates_relocates_and_protects_without_running_entry_point() {
    let exe = minimal_pe64_with_imports_and_relocations();
    let pe = PeImage::parse(&exe).unwrap();

    let mapped = pe.map_private_image(&exe).unwrap();

    assert_ne!(mapped.base(), 0);
    assert_eq!(mapped.size(), pe.size_of_image() as usize);
    assert_eq!(mapped.read_u64(0x1018).unwrap(), mapped.base() + 0x1234);
    assert_eq!(mapped.protected_section_count(), 3);
    assert_eq!(mapped.entry_point_rva(), 0x1000);
    assert!(!mapped.entry_point_was_run());
}

#[test]
fn ipc_parser_rejects_unknown_commands() {
    let response = handle_ipc_line(
        r#"{"id":"req-1","command":"self_destruct","exePath":"C:\\missing.exe"}"#,
        &[],
    );
    let value: Value = serde_json::from_str(&response).unwrap();

    assert_eq!(value["id"], "req-1");
    assert_eq!(value["ok"], false);
    assert_eq!(value["error"]["code"], "invalid_command");
}

#[test]
fn ipc_parser_accepts_utf8_bom_from_powershell_pipe() {
    let response = handle_ipc_line(
        "\u{feff}{\"id\":\"req-1\",\"command\":\"self_destruct\"}",
        &[],
    );
    let value: Value = serde_json::from_str(&response).unwrap();

    assert_eq!(value["id"], "req-1");
    assert_eq!(value["ok"], false);
    assert_eq!(value["error"]["code"], "invalid_command");
}

#[test]
fn response_serialization_for_missing_exe_is_stable() {
    let response = handle_ipc_line(
        r#"{"id":"req-1","command":"probe","exePath":"C:\\missing\\G1R-Win64-Shipping.exe"}"#,
        &[],
    );
    let value: Value = serde_json::from_str(&response).unwrap();

    assert_eq!(value["id"], "req-1");
    assert_eq!(value["ok"], false);
    assert_eq!(value["error"]["code"], "missing_exe");
    assert!(
        value["error"]["message"]
            .as_str()
            .unwrap()
            .contains("not found")
    );
}

#[test]
fn self_test_reports_loader_analysis_but_keeps_codec_disabled() {
    let exe = minimal_pe64_with_imports_and_relocations();
    let profile = parse_profile_json(&profile_json(
        &sha256_hex(&exe),
        exe.len() as u64,
        "0x23A85CE7",
        "0x140000000",
        "0x1010",
        "0x1020",
        "0x1030",
    ))
    .unwrap();
    let temp = write_temp_exe(&exe);

    let response = self_test_exe(
        &SelfTestRequest {
            exe_path: temp.path().to_path_buf(),
            relocation_base: Some(0x150000000),
            resolve_imports: false,
            map_image: false,
            run_runtime_selftests: false,
            runtime_selftest_run_decompress: false,
            runtime_selftest_run_compress: false,
            runtime_selftest_decompress_repeat_count: 1,
            runtime_selftest_compress_repeat_count: 1,
            runtime_selftest_sample: None,
            runtime_selftest_compress_sample: None,
        },
        &[profile],
    )
    .unwrap();

    assert_eq!(response.profile.as_deref(), Some("g1r-test"));
    assert_eq!(response.resolution_mode, Some(ResolutionMode::KnownProfile));
    assert!(response.private_mapping.pe_parsed);
    assert_eq!(response.private_mapping.import_dll_count, 1);
    assert_eq!(response.private_mapping.base_relocation_count, 1);
    assert_eq!(response.private_mapping.applied_relocation_count, 1);
    assert_eq!(response.private_mapping.section_protection_count, 3);
    assert_eq!(response.private_mapping.import_resolution_status, "not_run");
    assert!(!response.runtime_selftests.requested);
    assert!(!response.private_mapping.entry_point_run);
    assert_eq!(response.runtime_selftests.decompress, "not_run");
    assert_eq!(response.runtime_selftests.compress, "not_run");
    assert!(!response.can_decompress);
    assert!(!response.can_compress);
}

#[cfg(windows)]
#[test]
fn self_test_can_map_image_when_explicitly_requested() {
    let exe = minimal_pe64_with_imports_and_relocations();
    let profile = parse_profile_json(&profile_json(
        &sha256_hex(&exe),
        exe.len() as u64,
        "0x23A85CE7",
        "0x140000000",
        "0x1010",
        "0x1020",
        "0x1030",
    ))
    .unwrap();
    let temp = write_temp_exe(&exe);

    let response = self_test_exe(
        &SelfTestRequest {
            exe_path: temp.path().to_path_buf(),
            relocation_base: None,
            resolve_imports: false,
            map_image: true,
            run_runtime_selftests: false,
            runtime_selftest_run_decompress: false,
            runtime_selftest_run_compress: false,
            runtime_selftest_decompress_repeat_count: 1,
            runtime_selftest_compress_repeat_count: 1,
            runtime_selftest_sample: None,
            runtime_selftest_compress_sample: None,
        },
        &[profile],
    )
    .unwrap();

    assert!(response.private_mapping.memory_mapped);
    assert!(response.private_mapping.memory_mapped_base.unwrap() != 0);
    assert_eq!(response.private_mapping.memory_mapped_size, pe_size(&exe));
    assert_eq!(response.private_mapping.memory_protection_applied_count, 3);
    assert!(!response.private_mapping.entry_point_run);
    assert!(!response.can_decompress);
    assert!(!response.can_compress);
}

#[test]
fn self_test_marks_runtime_selftests_requested_but_worker_not_configured() {
    let exe = minimal_pe64_with_imports_and_relocations();
    let profile = parse_profile_json(&profile_json(
        &sha256_hex(&exe),
        exe.len() as u64,
        "0x23A85CE7",
        "0x140000000",
        "0x1010",
        "0x1020",
        "0x1030",
    ))
    .unwrap();
    let temp = write_temp_exe(&exe);

    let response = self_test_exe(
        &SelfTestRequest {
            exe_path: temp.path().to_path_buf(),
            relocation_base: None,
            resolve_imports: false,
            map_image: false,
            run_runtime_selftests: true,
            runtime_selftest_run_decompress: false,
            runtime_selftest_run_compress: false,
            runtime_selftest_decompress_repeat_count: 1,
            runtime_selftest_compress_repeat_count: 1,
            runtime_selftest_sample: None,
            runtime_selftest_compress_sample: None,
        },
        &[profile],
    )
    .unwrap();

    assert!(response.runtime_selftests.requested);
    assert_eq!(response.runtime_selftests.decompress, "not_run");
    assert_eq!(response.runtime_selftests.compress, "not_run");
    assert_eq!(response.runtime_selftests.worker_status, "not_configured");
    assert!(
        response
            .runtime_selftests
            .reason
            .unwrap()
            .contains("not configured")
    );
    assert!(!response.private_mapping.entry_point_run);
    assert!(!response.can_decompress);
    assert!(!response.can_compress);
}

#[test]
fn self_test_resolves_imports_when_explicitly_enabled_with_injected_resolver() {
    let exe = minimal_pe64_with_imports_and_relocations();
    let profile = parse_profile_json(&profile_json(
        &sha256_hex(&exe),
        exe.len() as u64,
        "0x23A85CE7",
        "0x140000000",
        "0x1010",
        "0x1020",
        "0x1030",
    ))
    .unwrap();
    let temp = write_temp_exe(&exe);
    let mut resolver = RecordingImportResolver::default();

    let response = self_test_exe_with_import_resolver(
        &SelfTestRequest {
            exe_path: temp.path().to_path_buf(),
            relocation_base: Some(0x150000000),
            resolve_imports: true,
            map_image: false,
            run_runtime_selftests: false,
            runtime_selftest_run_decompress: false,
            runtime_selftest_run_compress: false,
            runtime_selftest_decompress_repeat_count: 1,
            runtime_selftest_compress_repeat_count: 1,
            runtime_selftest_sample: None,
            runtime_selftest_compress_sample: None,
        },
        &[profile],
        &mut resolver,
    )
    .unwrap();

    assert_eq!(
        response.private_mapping.import_resolution_status,
        "resolved"
    );
    assert_eq!(response.private_mapping.fixed_import_thunk_count, 1);
    assert_eq!(
        resolver.calls,
        vec![("KERNEL32.dll".to_string(), "GetProcAddress".to_string())]
    );
    assert!(!response.private_mapping.entry_point_run);
    assert!(!response.can_decompress);
}

#[test]
fn ipc_self_test_serializes_loader_analysis() {
    let exe = minimal_pe64_with_imports_and_relocations();
    let profile = parse_profile_json(&profile_json(
        &sha256_hex(&exe),
        exe.len() as u64,
        "0x23A85CE7",
        "0x140000000",
        "0x1010",
        "0x1020",
        "0x1030",
    ))
    .unwrap();
    let temp = write_temp_exe(&exe);

    let response = handle_ipc_line(
        &format!(
            r#"{{"id":"req-1","command":"self_test","exePath":"{}","relocationBase":"0x150000000"}}"#,
            json_escape_path(temp.path())
        ),
        &[profile],
    );
    let value: Value = serde_json::from_str(&response).unwrap();

    assert_eq!(value["id"], "req-1");
    assert_eq!(value["ok"], true);
    assert_eq!(value["data"]["privateMapping"]["peParsed"], true);
    assert_eq!(value["data"]["privateMapping"]["entryPointRun"], false);
    assert_eq!(value["data"]["privateMapping"]["memoryMapped"], false);
    assert_eq!(
        value["data"]["privateMapping"]["importResolutionStatus"],
        "not_run"
    );
    assert_eq!(value["data"]["privateMapping"]["sectionProtectionCount"], 3);
    assert_eq!(value["data"]["runtimeSelftests"]["decompress"], "not_run");
    assert_eq!(value["data"]["canDecompress"], false);
}

#[test]
fn ipc_self_test_reports_requested_runtime_selftests_when_worker_not_configured() {
    let exe = minimal_pe64_with_imports_and_relocations();
    let profile = parse_profile_json(&profile_json(
        &sha256_hex(&exe),
        exe.len() as u64,
        "0x23A85CE7",
        "0x140000000",
        "0x1010",
        "0x1020",
        "0x1030",
    ))
    .unwrap();
    let temp = write_temp_exe(&exe);

    let response = handle_ipc_line(
        &format!(
            r#"{{"id":"req-1","command":"self_test","exePath":"{}","runRuntimeSelftests":true}}"#,
            json_escape_path(temp.path())
        ),
        &[profile],
    );
    let value: Value = serde_json::from_str(&response).unwrap();

    assert_eq!(value["id"], "req-1");
    assert_eq!(value["ok"], true);
    assert_eq!(value["data"]["runtimeSelftests"]["requested"], true);
    assert_eq!(
        value["data"]["runtimeSelftests"]["workerStatus"],
        "not_configured"
    );
    assert_eq!(value["data"]["runtimeSelftests"]["decompress"], "not_run");
    assert_eq!(value["data"]["runtimeSelftests"]["compress"], "not_run");
    assert!(
        value["data"]["runtimeSelftests"]["reason"]
            .as_str()
            .unwrap()
            .contains("not configured")
    );
    assert_eq!(value["data"]["privateMapping"]["entryPointRun"], false);
    assert_eq!(value["data"]["canCompress"], false);
    assert_eq!(value["data"]["canDecompress"], false);
}

#[test]
fn runtime_selftest_worker_process_returns_structured_not_requested_report() {
    let report = run_runtime_selftest_worker(&helper_binary_path(), Duration::from_secs(5), &[]);

    assert!(report.requested);
    assert_eq!(report.worker_status, "completed");
    assert_ne!(report.worker_pid.unwrap(), std::process::id());
    assert_eq!(report.worker_exit_code, Some(0));
    assert_eq!(report.decompress, "not_run");
    assert_eq!(report.compress, "not_run");
    assert!(report.reason.unwrap().contains("not requested"));
}

#[test]
fn runtime_selftest_worker_request_carries_exe_and_decompress_oracle_metadata() {
    let request = sample_runtime_worker_request(PathBuf::from(r"C:\G1R.exe"));

    let report = runtime_selftest_worker_report(Some(request)).unwrap();

    assert!(report.requested);
    assert_eq!(report.worker_status, "worker");
    assert_eq!(report.exe_path.as_deref(), Some(r"C:\G1R.exe"));
    assert_eq!(report.decompress_sample_status, "accepted");
    assert_eq!(report.decompress_sample_expected_size, Some(3));
    assert_eq!(report.decompress_sample_compressed_size, Some(3));
    assert_eq!(
        report.decompress_sample_expected_sha1.as_deref(),
        Some("00112233445566778899aabbccddeeff00112233")
    );
    assert_eq!(report.decompress, "not_run");
    assert_eq!(report.compress, "not_run");
}

#[test]
fn runtime_selftest_worker_spawn_passes_request_to_child_over_stdin() {
    let report = run_runtime_selftest_worker_with_request(
        &helper_binary_path(),
        Duration::from_secs(5),
        &[],
        &sample_runtime_worker_request(PathBuf::from(r"C:\G1R.exe")),
    );

    assert_eq!(report.worker_status, "completed");
    assert_eq!(report.exe_path.as_deref(), Some(r"C:\G1R.exe"));
    assert_eq!(report.decompress_sample_status, "accepted");
    assert_eq!(report.decompress_sample_expected_size, Some(3));
    assert_eq!(report.decompress_sample_compressed_size, Some(3));
    assert_eq!(
        report.decompress_sample_expected_sha1.as_deref(),
        Some("00112233445566778899aabbccddeeff00112233")
    );
    assert_eq!(report.worker_exit_code, Some(0));
}

#[test]
fn runtime_selftest_worker_rejects_bad_sample_base64_without_ooodle_call() {
    let mut request = sample_runtime_worker_request(PathBuf::from(r"C:\G1R.exe"));
    request.decompress_sample.compressed_base64 = "%".to_string();

    let report = runtime_selftest_worker_report(Some(request)).unwrap();

    assert_eq!(report.decompress_sample_status, "invalid_request");
    assert_eq!(report.decompress, "failed");
    assert_eq!(report.compress, "failed");
    assert!(report.reason.unwrap().contains("compressedBase64"));
}

#[test]
fn runtime_selftest_save_chunk_reads_requested_compressed_payload() {
    let save = minimal_gsav_with_oodle_chunks(&[
        (&[0x8c, 0x06, 0x00, 0x07, 0x08, 0x88], 131_072),
        (&[0xaa, 0xbb], 7),
    ]);
    let temp = write_temp_sav(&save);

    let sample = runtime_selftest_sample_from_save_chunk(&RuntimeSelftestSaveChunkRequest {
        save_path: temp.path().to_path_buf(),
        chunk_index: 1,
        expected_compressed_sha1: Some("65b1e351a6cbfeb41c927222bc9ef53aad3396b0".to_string()),
        expected_decompressed_sha1: "00112233445566778899aabbccddeeff00112233".to_string(),
        expected_decompressed_head_hex: "010203".to_string(),
    })
    .unwrap();

    assert_eq!(sample.compressed_base64, "qrs=");
    assert_eq!(sample.expected_size, 7);
    assert_eq!(
        sample.expected_decompressed_sha1,
        "00112233445566778899aabbccddeeff00112233"
    );
    assert_eq!(sample.expected_decompressed_head_hex, "010203");
}

#[test]
fn runtime_selftest_save_chunk_rejects_compressed_sha1_mismatch() {
    let save = minimal_gsav_with_oodle_chunks(&[(&[0xaa, 0xbb], 7)]);
    let temp = write_temp_sav(&save);

    let err = runtime_selftest_sample_from_save_chunk(&RuntimeSelftestSaveChunkRequest {
        save_path: temp.path().to_path_buf(),
        chunk_index: 0,
        expected_compressed_sha1: Some("0000000000000000000000000000000000000000".to_string()),
        expected_decompressed_sha1: "00112233445566778899aabbccddeeff00112233".to_string(),
        expected_decompressed_head_hex: "010203".to_string(),
    })
    .unwrap_err();

    assert_eq!(err.code(), ErrorCode::InvalidRequest);
    assert!(err.to_string().contains("compressed SHA-1"));
}

#[test]
fn runtime_selftest_worker_rejects_non_executable_rvas_before_call() {
    let exe = minimal_pe64_with_text_section();
    let temp = write_temp_exe(&exe);
    let mut request = sample_runtime_worker_request(temp.path().to_path_buf());
    request.codec_rvas = Some(RuntimeCodecRvas {
        oodle_lz_compress: 0x1010,
        oodle_lz_decompress: 0x2000,
        compressor_dispatch: 0x1030,
    });

    let report = runtime_selftest_worker_report(Some(request)).unwrap();

    assert_eq!(report.decompress_sample_status, "accepted");
    assert_eq!(report.decompress, "failed");
    assert_eq!(report.compress, "failed");
    let preflight = report.runtime_preflight.as_ref().unwrap();
    assert_eq!(preflight.status, "failed");
    assert!(
        preflight
            .reason
            .as_deref()
            .unwrap()
            .contains("oodleLzDecompress")
    );
    assert!(
        preflight
            .reason
            .as_deref()
            .unwrap()
            .contains("outside executable sections")
    );
}

#[cfg(windows)]
#[test]
fn runtime_selftest_worker_maps_exe_and_resolves_rvas_before_call() {
    let exe = minimal_pe64_with_imports_and_relocations();
    let temp = write_temp_exe(&exe);
    let mut request = sample_runtime_worker_request(temp.path().to_path_buf());
    request.codec_rvas = Some(RuntimeCodecRvas {
        oodle_lz_compress: 0x1010,
        oodle_lz_decompress: 0x1020,
        compressor_dispatch: 0x1030,
    });

    let report = runtime_selftest_worker_report(Some(request)).unwrap();

    assert_eq!(report.decompress_sample_status, "accepted");
    assert_eq!(report.decompress, "not_run");
    assert_eq!(report.compress, "not_run");
    let preflight = report.runtime_preflight.as_ref().unwrap();
    assert_eq!(preflight.status, "ready_to_call");
    assert_eq!(preflight.import_resolution_status, "resolved");
    assert_eq!(preflight.fixed_import_thunk_count, 1);
    assert!(preflight.memory_mapped);
    assert_ne!(preflight.memory_mapped_base.unwrap(), 0);
    assert_eq!(preflight.memory_protection_applied_count, 3);
    assert_eq!(preflight.entry_point_run, false);
    assert_eq!(preflight.resolved_rvas.len(), 3);
    assert!(
        preflight
            .resolved_rvas
            .iter()
            .all(|rva| rva.executable && rva.mapped_va != 0)
    );
    assert!(report.reason.unwrap().contains("not requested"));
}

#[cfg(windows)]
#[test]
fn runtime_selftest_worker_opt_in_decompress_call_invokes_mapped_function_and_checks_sha1() {
    let exe = minimal_pe64_with_imports_and_relocations();
    let temp = write_temp_exe(&exe);
    let mut request = sample_runtime_worker_request(temp.path().to_path_buf());
    request.codec_rvas = Some(RuntimeCodecRvas {
        oodle_lz_compress: 0x1010,
        oodle_lz_decompress: 0x1020,
        compressor_dispatch: 0x1030,
    });
    request.run_decompress_call = true;

    let report = runtime_selftest_worker_report(Some(request)).unwrap();

    assert_eq!(report.decompress_sample_status, "accepted");
    assert_eq!(report.decompress, "failed");
    assert_eq!(report.decompress_call_return, Some(3));
    assert_eq!(report.decompress_output_size, Some(3));
    assert!(report.decompress_output_sha1.is_some());
    assert!(report.reason.unwrap().contains("SHA-1 mismatch"));
}

#[cfg(windows)]
#[test]
fn ipc_self_test_marks_can_decompress_when_runtime_decompress_passes() {
    let mut exe = minimal_pe64_with_imports_and_relocations();
    exe[0x220..0x232].copy_from_slice(&[
        0x41, 0xc6, 0x00, 0x01, // mov byte ptr [r8], 1
        0x41, 0xc6, 0x40, 0x01, 0x02, // mov byte ptr [r8 + 1], 2
        0x41, 0xc6, 0x40, 0x02, 0x03, // mov byte ptr [r8 + 2], 3
        0x4c, 0x89, 0xc8, // mov rax, r9
        0xc3, // ret
    ]);
    let profile = parse_profile_json(&profile_json(
        &sha256_hex(&exe),
        exe.len() as u64,
        "0x23A85CE7",
        "0x140000000",
        "0x1010",
        "0x1020",
        "0x1030",
    ))
    .unwrap();
    let temp = write_temp_exe(&exe);

    let response = handle_ipc_line_with_runtime_worker(
        &format!(
            r#"{{"id":"req-1","command":"self_test","exePath":"{}","runRuntimeSelftests":true,"runtimeSelftestRunDecompress":true,"runtimeSelftestSample":{{"compressedBase64":"AQID","expectedSize":3,"expectedDecompressedSha1":"7037807198c22a7d2b0807371d763779a84fdfcf","expectedDecompressedHeadHex":"010203"}}}}"#,
            json_escape_path(temp.path())
        ),
        &[profile],
        &helper_binary_path(),
    );
    let value: Value = serde_json::from_str(&response).unwrap();

    assert_eq!(value["id"], "req-1");
    assert_eq!(value["ok"], true);
    assert_eq!(value["data"]["runtimeSelftests"]["decompress"], "passed");
    assert_eq!(value["data"]["canDecompress"], true);
    assert_eq!(value["data"]["canCompress"], false);
}

#[cfg(windows)]
#[test]
fn runtime_selftest_worker_repeats_decompress_call_requested_count() {
    let mut exe = minimal_pe64_with_imports_and_relocations();
    exe[0x220..0x232].copy_from_slice(&[
        0x41, 0xc6, 0x00, 0x01, // mov byte ptr [r8], 1
        0x41, 0xc6, 0x40, 0x01, 0x02, // mov byte ptr [r8 + 1], 2
        0x41, 0xc6, 0x40, 0x02, 0x03, // mov byte ptr [r8 + 2], 3
        0x4c, 0x89, 0xc8, // mov rax, r9
        0xc3, // ret
    ]);
    let temp = write_temp_exe(&exe);
    let mut request = sample_runtime_worker_request(temp.path().to_path_buf());
    request.codec_rvas = Some(RuntimeCodecRvas {
        oodle_lz_compress: 0x1010,
        oodle_lz_decompress: 0x1020,
        compressor_dispatch: 0x1030,
    });
    request.run_decompress_call = true;
    request.decompress_repeat_count = 3;
    request.decompress_sample.expected_decompressed_sha1 =
        "7037807198c22a7d2b0807371d763779a84fdfcf".to_string();

    let report = runtime_selftest_worker_report(Some(request)).unwrap();

    assert_eq!(report.decompress, "passed");
    assert_eq!(report.decompress_call_count, Some(3));
    assert_eq!(
        report.decompress_output_sha1.as_deref(),
        Some("7037807198c22a7d2b0807371d763779a84fdfcf")
    );
}

#[cfg(windows)]
#[test]
fn ipc_self_test_forwards_decompress_repeat_count_to_runtime_worker() {
    let mut exe = minimal_pe64_with_imports_and_relocations();
    exe[0x220..0x232].copy_from_slice(&[
        0x41, 0xc6, 0x00, 0x01, // mov byte ptr [r8], 1
        0x41, 0xc6, 0x40, 0x01, 0x02, // mov byte ptr [r8 + 1], 2
        0x41, 0xc6, 0x40, 0x02, 0x03, // mov byte ptr [r8 + 2], 3
        0x4c, 0x89, 0xc8, // mov rax, r9
        0xc3, // ret
    ]);
    let profile = parse_profile_json(&profile_json(
        &sha256_hex(&exe),
        exe.len() as u64,
        "0x23A85CE7",
        "0x140000000",
        "0x1010",
        "0x1020",
        "0x1030",
    ))
    .unwrap();
    let temp = write_temp_exe(&exe);

    let response = handle_ipc_line_with_runtime_worker(
        &format!(
            r#"{{"id":"req-1","command":"self_test","exePath":"{}","runRuntimeSelftests":true,"runtimeSelftestRunDecompress":true,"runtimeSelftestDecompressRepeatCount":3,"runtimeSelftestSample":{{"compressedBase64":"AQID","expectedSize":3,"expectedDecompressedSha1":"7037807198c22a7d2b0807371d763779a84fdfcf","expectedDecompressedHeadHex":"010203"}}}}"#,
            json_escape_path(temp.path())
        ),
        &[profile],
        &helper_binary_path(),
    );
    let value: Value = serde_json::from_str(&response).unwrap();

    assert_eq!(value["id"], "req-1");
    assert_eq!(value["ok"], true);
    assert_eq!(value["data"]["runtimeSelftests"]["decompress"], "passed");
    assert_eq!(value["data"]["runtimeSelftests"]["decompressCallCount"], 3);
}

#[test]
fn runtime_selftest_worker_timeout_is_reported_without_running_in_parent() {
    let report = run_runtime_selftest_worker(
        &helper_binary_path(),
        Duration::from_millis(25),
        &["--delay-ms", "250"],
    );

    assert!(report.requested);
    assert_eq!(report.worker_status, "timeout");
    assert!(report.worker_pid.unwrap() != 0);
    assert_eq!(report.worker_exit_code, None);
    assert_eq!(report.decompress, "failed");
    assert_eq!(report.compress, "failed");
    assert!(report.reason.unwrap().contains("timed out"));
}

#[test]
fn ipc_self_test_uses_runtime_worker_when_configured() {
    let exe = minimal_pe64_with_imports_and_relocations();
    let profile = parse_profile_json(&profile_json(
        &sha256_hex(&exe),
        exe.len() as u64,
        "0x23A85CE7",
        "0x140000000",
        "0x1010",
        "0x1020",
        "0x1030",
    ))
    .unwrap();
    let temp = write_temp_exe(&exe);

    let response = handle_ipc_line_with_runtime_worker(
        &format!(
            r#"{{"id":"req-1","command":"self_test","exePath":"{}","runRuntimeSelftests":true}}"#,
            json_escape_path(temp.path())
        ),
        &[profile],
        &helper_binary_path(),
    );
    let value: Value = serde_json::from_str(&response).unwrap();

    assert_eq!(value["id"], "req-1");
    assert_eq!(value["ok"], true);
    assert_eq!(value["data"]["runtimeSelftests"]["requested"], true);
    assert_eq!(
        value["data"]["runtimeSelftests"]["workerStatus"],
        "completed"
    );
    assert!(
        value["data"]["runtimeSelftests"]["workerPid"]
            .as_u64()
            .unwrap()
            != 0
    );
    assert_eq!(value["data"]["runtimeSelftests"]["workerExitCode"], 0);
    assert_eq!(value["data"]["runtimeSelftests"]["decompress"], "not_run");
    assert_eq!(value["data"]["privateMapping"]["entryPointRun"], false);
    assert_eq!(value["data"]["canDecompress"], false);
}

#[test]
fn ipc_self_test_forwards_decompress_oracle_sample_to_runtime_worker() {
    let exe = minimal_pe64_with_imports_and_relocations();
    let profile = parse_profile_json(&profile_json(
        &sha256_hex(&exe),
        exe.len() as u64,
        "0x23A85CE7",
        "0x140000000",
        "0x1010",
        "0x1020",
        "0x1030",
    ))
    .unwrap();
    let temp = write_temp_exe(&exe);

    let response = handle_ipc_line_with_runtime_worker(
        &format!(
            r#"{{"id":"req-1","command":"self_test","exePath":"{}","runRuntimeSelftests":true,"runtimeSelftestSample":{{"compressedBase64":"AQID","expectedSize":3,"expectedDecompressedSha1":"00112233445566778899aabbccddeeff00112233","expectedDecompressedHeadHex":"010203"}}}}"#,
            json_escape_path(temp.path())
        ),
        &[profile],
        &helper_binary_path(),
    );
    let value: Value = serde_json::from_str(&response).unwrap();

    assert_eq!(value["id"], "req-1");
    assert_eq!(value["ok"], true);
    assert_eq!(
        value["data"]["runtimeSelftests"]["workerStatus"],
        "completed"
    );
    assert_eq!(
        value["data"]["runtimeSelftests"]["decompressSampleStatus"],
        "accepted"
    );
    assert_eq!(
        value["data"]["runtimeSelftests"]["decompressSampleExpectedSize"],
        3
    );
    assert_eq!(
        value["data"]["runtimeSelftests"]["decompressSampleCompressedSize"],
        3
    );
    #[cfg(windows)]
    {
        assert_eq!(
            value["data"]["runtimeSelftests"]["runtimePreflight"]["status"],
            "ready_to_call"
        );
        assert_eq!(
            value["data"]["runtimeSelftests"]["runtimePreflight"]["resolvedRvas"]
                .as_array()
                .unwrap()
                .len(),
            3
        );
    }
    assert_eq!(value["data"]["canDecompress"], false);
}

#[test]
fn ipc_self_test_loads_save_chunk_sample_for_runtime_worker() {
    let exe = minimal_pe64_with_imports_and_relocations();
    let profile = parse_profile_json(&profile_json(
        &sha256_hex(&exe),
        exe.len() as u64,
        "0x23A85CE7",
        "0x140000000",
        "0x1010",
        "0x1020",
        "0x1030",
    ))
    .unwrap();
    let exe_temp = write_temp_exe(&exe);
    let save = minimal_gsav_with_oodle_chunks(&[(&[0xaa, 0xbb], 7)]);
    let save_temp = write_temp_sav(&save);

    let response = handle_ipc_line_with_runtime_worker(
        &format!(
            r#"{{"id":"req-1","command":"self_test","exePath":"{}","runRuntimeSelftests":true,"runtimeSelftestSaveChunk":{{"savePath":"{}","chunkIndex":0,"expectedCompressedSha1":"65b1e351a6cbfeb41c927222bc9ef53aad3396b0","expectedDecompressedSha1":"00112233445566778899aabbccddeeff00112233","expectedDecompressedHeadHex":"010203"}}}}"#,
            json_escape_path(exe_temp.path()),
            json_escape_path(save_temp.path())
        ),
        &[profile],
        &helper_binary_path(),
    );
    let value: Value = serde_json::from_str(&response).unwrap();

    assert_eq!(value["id"], "req-1");
    assert_eq!(value["ok"], true);
    assert_eq!(
        value["data"]["runtimeSelftests"]["decompressSampleStatus"],
        "accepted"
    );
    assert_eq!(
        value["data"]["runtimeSelftests"]["decompressSampleCompressedSize"],
        2
    );
    assert_eq!(
        value["data"]["runtimeSelftests"]["decompressSampleExpectedSize"],
        7
    );
    assert_eq!(
        value["data"]["runtimeSelftests"]["decompressSampleExpectedSha1"],
        "00112233445566778899aabbccddeeff00112233"
    );
    assert_eq!(value["data"]["canDecompress"], false);
}

#[test]
fn ipc_self_test_resolves_imports_only_when_requested() {
    let exe = minimal_pe64_with_imports_and_relocations();
    let profile = parse_profile_json(&profile_json(
        &sha256_hex(&exe),
        exe.len() as u64,
        "0x23A85CE7",
        "0x140000000",
        "0x1010",
        "0x1020",
        "0x1030",
    ))
    .unwrap();
    let temp = write_temp_exe(&exe);

    let response = handle_ipc_line(
        &format!(
            r#"{{"id":"req-1","command":"self_test","exePath":"{}","relocationBase":"0x150000000","resolveImports":true}}"#,
            json_escape_path(temp.path())
        ),
        &[profile],
    );
    let value: Value = serde_json::from_str(&response).unwrap();

    assert_eq!(value["id"], "req-1");
    assert_eq!(value["ok"], true);
    assert_eq!(
        value["data"]["privateMapping"]["importResolutionStatus"],
        "resolved"
    );
    assert_eq!(value["data"]["privateMapping"]["fixedImportThunkCount"], 1);
    assert_eq!(value["data"]["privateMapping"]["entryPointRun"], false);
}

#[cfg(windows)]
#[test]
fn ipc_self_test_maps_image_only_when_requested() {
    let exe = minimal_pe64_with_imports_and_relocations();
    let profile = parse_profile_json(&profile_json(
        &sha256_hex(&exe),
        exe.len() as u64,
        "0x23A85CE7",
        "0x140000000",
        "0x1010",
        "0x1020",
        "0x1030",
    ))
    .unwrap();
    let temp = write_temp_exe(&exe);

    let response = handle_ipc_line(
        &format!(
            r#"{{"id":"req-1","command":"self_test","exePath":"{}","mapImage":true}}"#,
            json_escape_path(temp.path())
        ),
        &[profile],
    );
    let value: Value = serde_json::from_str(&response).unwrap();

    assert_eq!(value["id"], "req-1");
    assert_eq!(value["ok"], true);
    assert_eq!(value["data"]["privateMapping"]["memoryMapped"], true);
    assert!(
        value["data"]["privateMapping"]["memoryMappedBase"]
            .as_u64()
            .unwrap()
            != 0
    );
    assert_eq!(
        value["data"]["privateMapping"]["memoryProtectionAppliedCount"],
        3
    );
    assert_eq!(value["data"]["privateMapping"]["entryPointRun"], false);
    assert_eq!(value["data"]["canDecompress"], false);
}

#[test]
fn ipc_decompress_requires_base64_payload_before_codec_gate() {
    let response = handle_ipc_line(
        r#"{"id":"req-1","command":"decompress","exePath":"C:\\G1R.exe","expectedSize":16}"#,
        &[],
    );
    let value: Value = serde_json::from_str(&response).unwrap();

    assert_eq!(value["id"], "req-1");
    assert_eq!(value["ok"], false);
    assert_eq!(value["error"]["code"], "invalid_request");
    assert!(
        value["error"]["message"]
            .as_str()
            .unwrap()
            .contains("inputBase64")
    );
}

#[test]
fn ipc_decompress_rejects_expected_size_above_request_limit() {
    let response = handle_ipc_line(
        r#"{"id":"req-1","command":"decompress","exePath":"C:\\G1R.exe","inputBase64":"AQID","expectedSize":131073}"#,
        &[],
    );
    let value: Value = serde_json::from_str(&response).unwrap();

    assert_eq!(value["id"], "req-1");
    assert_eq!(value["ok"], false);
    assert_eq!(value["error"]["code"], "invalid_request");
    assert_eq!(value["error"]["details"]["maxUncompressedSize"], 0x20000);
}

#[test]
fn ipc_decompress_validates_payload_then_keeps_codec_disabled() {
    let response = handle_ipc_line(
        r#"{"id":"req-1","command":"decompress","exePath":"C:\\G1R.exe","inputBase64":"AQID","expectedSize":3}"#,
        &[],
    );
    let value: Value = serde_json::from_str(&response).unwrap();

    assert_eq!(value["id"], "req-1");
    assert_eq!(value["ok"], false);
    assert_eq!(value["error"]["code"], "unsupported_exe");
    assert!(
        value["error"]["message"]
            .as_str()
            .unwrap()
            .contains("runtime selftests")
    );
}

#[cfg(windows)]
#[test]
fn ipc_decompress_uses_runtime_worker_and_returns_output_base64() {
    let exe = minimal_pe64_with_imports_and_relocations();
    let profile = parse_profile_json(&profile_json(
        &sha256_hex(&exe),
        exe.len() as u64,
        "0x23A85CE7",
        "0x140000000",
        "0x1010",
        "0x1020",
        "0x1030",
    ))
    .unwrap();
    let temp = write_temp_exe(&exe);

    let response = handle_ipc_line_with_runtime_worker(
        &format!(
            r#"{{"id":"req-1","command":"decompress","exePath":"{}","inputBase64":"AQID","expectedSize":3}}"#,
            json_escape_path(temp.path())
        ),
        &[profile],
        &helper_binary_path(),
    );
    let value: Value = serde_json::from_str(&response).unwrap();

    assert_eq!(value["id"], "req-1");
    assert_eq!(value["ok"], true);
    assert_eq!(value["data"]["outputBase64"], "AAAA");
    assert_eq!(value["data"]["profile"], "g1r-test");
    assert_eq!(value["data"]["resolutionMode"], "known_profile");
}

#[cfg(windows)]
#[test]
fn ipc_decompress_many_uses_one_runtime_worker_and_returns_outputs_base64() {
    let exe = minimal_pe64_with_imports_and_relocations();
    let profile = parse_profile_json(&profile_json(
        &sha256_hex(&exe),
        exe.len() as u64,
        "0x23A85CE7",
        "0x140000000",
        "0x1010",
        "0x1020",
        "0x1030",
    ))
    .unwrap();
    let temp = write_temp_exe(&exe);

    let response = handle_ipc_line_with_runtime_worker(
        &format!(
            r#"{{
                "id":"req-1",
                "command":"decompress_many",
                "exePath":"{}",
                "chunks":[
                    {{"inputBase64":"AQID","expectedSize":3}},
                    {{"inputBase64":"BAUGBw==","expectedSize":4}}
                ]
            }}"#,
            json_escape_path(temp.path())
        ),
        &[profile],
        &helper_binary_path(),
    );
    let value: Value = serde_json::from_str(&response).unwrap();

    assert_eq!(value["id"], "req-1");
    assert_eq!(value["ok"], true);
    assert_eq!(
        value["data"]["outputsBase64"],
        serde_json::json!(["AAAA", "AAAAAA=="])
    );
    assert_eq!(value["data"]["profile"], "g1r-test");
    assert_eq!(value["data"]["resolutionMode"], "known_profile");
}

#[cfg(windows)]
#[test]
fn ipc_decompress_uses_derived_profile_cache_rvas() {
    let exe = minimal_pe64_with_imports_and_relocations();
    let temp = write_temp_exe(&exe);
    let dir = tempfile::tempdir().unwrap();
    let cache_path = dir.path().join("derived_profiles.json");
    let entry = derived_profile_entry_for_exe(&exe, 0x1010, 0x1020, 0x1030, true, true);
    write_derived_profile_cache_entry(&cache_path, entry).unwrap();

    let response = handle_ipc_line_with_runtime_worker(
        &format!(
            r#"{{
                "id":"req-1",
                "command":"decompress",
                "exePath":"{}",
                "derivedProfileCachePath":"{}",
                "inputBase64":"AQID",
                "expectedSize":3
            }}"#,
            json_escape_path(temp.path()),
            json_escape_path(&cache_path)
        ),
        &[],
        &helper_binary_path(),
    );
    let value: Value = serde_json::from_str(&response).unwrap();

    assert_eq!(value["id"], "req-1");
    assert_eq!(value["ok"], true);
    assert_eq!(value["data"]["outputBase64"], "AAAA");
    assert_eq!(value["data"]["profile"], "g1r-derived-test");
    assert_eq!(value["data"]["resolutionMode"], "derived_profile_cache");
}

#[cfg(windows)]
#[test]
fn ipc_decompress_rejects_uncached_pattern_profile_before_selftest() {
    let exe = minimal_pe64_with_shifted_g1r_patterns();
    let profile = parse_profile_json(&profile_json(
        "0000000000000000000000000000000000000000000000000000000000000000",
        exe.len() as u64,
        "0x23A85CE7",
        "0x140000000",
        "0x1010",
        "0x1020",
        "0x1030",
    ))
    .unwrap();
    let temp = write_temp_exe(&exe);

    let response = handle_ipc_line_with_runtime_worker(
        &format!(
            r#"{{
                "id":"req-1",
                "command":"decompress",
                "exePath":"{}",
                "inputBase64":"AQID",
                "expectedSize":3
            }}"#,
            json_escape_path(temp.path())
        ),
        &[profile],
        &helper_binary_path(),
    );
    let value: Value = serde_json::from_str(&response).unwrap();

    assert_eq!(value["id"], "req-1");
    assert_eq!(value["ok"], false);
    assert_eq!(value["error"]["code"], "unsupported_exe");
    assert!(
        value["error"]["message"]
            .as_str()
            .unwrap()
            .contains("requires self_test")
    );
}

#[cfg(windows)]
#[test]
fn ipc_decompress_large_output_does_not_timeout_on_worker_stdout_pipe() {
    let exe = minimal_pe64_with_imports_and_relocations();
    let profile = parse_profile_json(&profile_json(
        &sha256_hex(&exe),
        exe.len() as u64,
        "0x23A85CE7",
        "0x140000000",
        "0x1010",
        "0x1020",
        "0x1030",
    ))
    .unwrap();
    let temp = write_temp_exe(&exe);

    let response = handle_ipc_line_with_runtime_worker(
        &format!(
            r#"{{"id":"req-1","command":"decompress","exePath":"{}","inputBase64":"AQID","expectedSize":131072}}"#,
            json_escape_path(temp.path())
        ),
        &[profile],
        &helper_binary_path(),
    );
    let value: Value = serde_json::from_str(&response).unwrap();

    assert_eq!(value["id"], "req-1");
    assert_eq!(value["ok"], true);
    assert_eq!(
        value["data"]["outputBase64"].as_str().unwrap().len(),
        174_764
    );
}

#[test]
fn ipc_compress_rejects_uncompressed_payload_above_request_limit() {
    let oversized_base64 = "A".repeat(174_764);
    let response = handle_ipc_line(
        &format!(
            r#"{{"id":"req-1","command":"compress","exePath":"C:\\G1R.exe","inputBase64":"{oversized_base64}","level":6}}"#
        ),
        &[],
    );
    let value: Value = serde_json::from_str(&response).unwrap();

    assert_eq!(value["id"], "req-1");
    assert_eq!(value["ok"], false);
    assert_eq!(value["error"]["code"], "invalid_request");
    assert_eq!(value["error"]["details"]["maxUncompressedSize"], 0x20000);
}

#[test]
fn ipc_compress_requires_numeric_level_before_codec_gate() {
    let response = handle_ipc_line(
        r#"{"id":"req-1","command":"compress","exePath":"C:\\G1R.exe","inputBase64":"AQID","level":"6"}"#,
        &[],
    );
    let value: Value = serde_json::from_str(&response).unwrap();

    assert_eq!(value["id"], "req-1");
    assert_eq!(value["ok"], false);
    assert_eq!(value["error"]["code"], "invalid_request");
    assert!(
        value["error"]["message"]
            .as_str()
            .unwrap()
            .contains("level")
    );
}

#[cfg(windows)]
#[test]
fn ipc_compress_uses_runtime_worker_and_returns_roundtripped_output_base64() {
    let mut exe = minimal_pe64_with_imports_and_relocations();
    exe[0x220..0x237].copy_from_slice(&[
        0x8a, 0x01, // mov al, byte ptr [rcx]
        0x41, 0x88, 0x00, // mov byte ptr [r8], al
        0x8a, 0x41, 0x01, // mov al, byte ptr [rcx + 1]
        0x41, 0x88, 0x40, 0x01, // mov byte ptr [r8 + 1], al
        0x8a, 0x41, 0x02, // mov al, byte ptr [rcx + 2]
        0x41, 0x88, 0x40, 0x02, // mov byte ptr [r8 + 2], al
        0x4c, 0x89, 0xc8, // mov rax, r9
        0xc3, // ret
    ]);
    exe[0x250..0x267].copy_from_slice(&[
        0x8a, 0x02, // mov al, byte ptr [rdx]
        0x41, 0x88, 0x01, // mov byte ptr [r9], al
        0x8a, 0x42, 0x01, // mov al, byte ptr [rdx + 1]
        0x41, 0x88, 0x41, 0x01, // mov byte ptr [r9 + 1], al
        0x8a, 0x42, 0x02, // mov al, byte ptr [rdx + 2]
        0x41, 0x88, 0x41, 0x02, // mov byte ptr [r9 + 2], al
        0x4c, 0x89, 0xc0, // mov rax, r8
        0xc3, // ret
    ]);
    let profile = parse_profile_json(&profile_json(
        &sha256_hex(&exe),
        exe.len() as u64,
        "0x23A85CE7",
        "0x140000000",
        "0x1050",
        "0x1020",
        "0x1030",
    ))
    .unwrap();
    let temp = write_temp_exe(&exe);

    let response = handle_ipc_line_with_runtime_worker(
        &format!(
            r#"{{"id":"req-1","command":"compress","exePath":"{}","inputBase64":"AQID","level":6}}"#,
            json_escape_path(temp.path())
        ),
        &[profile],
        &helper_binary_path(),
    );
    let value: Value = serde_json::from_str(&response).unwrap();

    assert_eq!(value["id"], "req-1");
    assert_eq!(value["ok"], true);
    assert_eq!(value["data"]["outputBase64"], "AQID");
    assert_eq!(value["data"]["profile"], "g1r-test");
    assert_eq!(value["data"]["resolutionMode"], "known_profile");
}

#[cfg(windows)]
#[test]
fn ipc_compress_many_uses_one_runtime_worker_and_returns_outputs_base64() {
    let mut exe = minimal_pe64_with_imports_and_relocations();
    exe[0x220..0x237].copy_from_slice(&[
        0x8a, 0x01, 0x41, 0x88, 0x00, 0x8a, 0x41, 0x01, 0x41, 0x88, 0x40, 0x01, 0x8a, 0x41, 0x02,
        0x41, 0x88, 0x40, 0x02, 0x4c, 0x89, 0xc8, 0xc3,
    ]);
    exe[0x250..0x267].copy_from_slice(&[
        0x8a, 0x02, 0x41, 0x88, 0x01, 0x8a, 0x42, 0x01, 0x41, 0x88, 0x41, 0x01, 0x8a, 0x42, 0x02,
        0x41, 0x88, 0x41, 0x02, 0x4c, 0x89, 0xc0, 0xc3,
    ]);
    let profile = parse_profile_json(&profile_json(
        &sha256_hex(&exe),
        exe.len() as u64,
        "0x23A85CE7",
        "0x140000000",
        "0x1050",
        "0x1020",
        "0x1030",
    ))
    .unwrap();
    let temp = write_temp_exe(&exe);

    let response = handle_ipc_line_with_runtime_worker(
        &format!(
            r#"{{
                "id":"req-1",
                "command":"compress_many",
                "exePath":"{}",
                "chunks":[
                    {{"inputBase64":"AQID","level":6}},
                    {{"inputBase64":"BAUG","level":6}}
                ]
            }}"#,
            json_escape_path(temp.path())
        ),
        &[profile],
        &helper_binary_path(),
    );
    let value: Value = serde_json::from_str(&response).unwrap();

    assert_eq!(value["id"], "req-1");
    assert_eq!(value["ok"], true);
    assert_eq!(
        value["data"]["outputsBase64"],
        serde_json::json!(["AQID", "BAUG"])
    );
    assert_eq!(value["data"]["profile"], "g1r-test");
    assert_eq!(value["data"]["resolutionMode"], "known_profile");
}

#[cfg(windows)]
#[test]
fn ipc_compress_many_reports_worker_failure_before_missing_outputs_base64() {
    let mut exe = minimal_pe64_with_imports_and_relocations();
    exe[0x250..0x253].copy_from_slice(&[
        0x31, 0xc0, // xor eax, eax
        0xc3, // ret
    ]);
    let profile = parse_profile_json(&profile_json(
        &sha256_hex(&exe),
        exe.len() as u64,
        "0x23A85CE7",
        "0x140000000",
        "0x1050",
        "0x1020",
        "0x1030",
    ))
    .unwrap();
    let temp = write_temp_exe(&exe);

    let response = handle_ipc_line_with_runtime_worker(
        &format!(
            r#"{{
                "id":"req-1",
                "command":"compress_many",
                "exePath":"{}",
                "chunks":[
                    {{"inputBase64":"AQID","level":6}},
                    {{"inputBase64":"BAUG","level":6}}
                ]
            }}"#,
            json_escape_path(temp.path())
        ),
        &[profile],
        &helper_binary_path(),
    );
    let value: Value = serde_json::from_str(&response).unwrap();

    assert_eq!(value["id"], "req-1");
    assert_eq!(value["ok"], false);
    assert_eq!(value["error"]["code"], "unsupported_exe");
    assert!(
        value["error"]["message"]
            .as_str()
            .unwrap()
            .contains("compress_many worker failed runtime verification")
    );
    assert!(
        value["error"]["message"]
            .as_str()
            .unwrap()
            .contains("Oodle compress returned non-positive result 0")
    );
    assert!(
        value["error"]["details"]["runtimeSelftests"]["reason"]
            .as_str()
            .unwrap()
            .contains("Oodle compress returned non-positive result 0")
    );
}

#[cfg(windows)]
#[test]
fn ipc_compress_uses_derived_profile_cache_rvas() {
    let mut exe = minimal_pe64_with_imports_and_relocations();
    exe[0x220..0x237].copy_from_slice(&[
        0x8a, 0x01, // mov al, byte ptr [rcx]
        0x41, 0x88, 0x00, // mov byte ptr [r8], al
        0x8a, 0x41, 0x01, // mov al, byte ptr [rcx + 1]
        0x41, 0x88, 0x40, 0x01, // mov byte ptr [r8 + 1], al
        0x8a, 0x41, 0x02, // mov al, byte ptr [rcx + 2]
        0x41, 0x88, 0x40, 0x02, // mov byte ptr [r8 + 2], al
        0x4c, 0x89, 0xc8, // mov rax, r9
        0xc3, // ret
    ]);
    exe[0x250..0x267].copy_from_slice(&[
        0x8a, 0x02, // mov al, byte ptr [rdx]
        0x41, 0x88, 0x01, // mov byte ptr [r9], al
        0x8a, 0x42, 0x01, // mov al, byte ptr [rdx + 1]
        0x41, 0x88, 0x41, 0x01, // mov byte ptr [r9 + 1]
        0x8a, 0x42, 0x02, // mov al, byte ptr [rdx + 2]
        0x41, 0x88, 0x41, 0x02, // mov byte ptr [r9 + 2]
        0x4c, 0x89, 0xc0, // mov rax, r8
        0xc3, // ret
    ]);
    let temp = write_temp_exe(&exe);
    let dir = tempfile::tempdir().unwrap();
    let cache_path = dir.path().join("derived_profiles.json");
    let entry = derived_profile_entry_for_exe(&exe, 0x1050, 0x1020, 0x1030, true, true);
    write_derived_profile_cache_entry(&cache_path, entry).unwrap();

    let response = handle_ipc_line_with_runtime_worker(
        &format!(
            r#"{{
                "id":"req-1",
                "command":"compress",
                "exePath":"{}",
                "derivedProfileCachePath":"{}",
                "inputBase64":"AQID",
                "level":6
            }}"#,
            json_escape_path(temp.path()),
            json_escape_path(&cache_path)
        ),
        &[],
        &helper_binary_path(),
    );
    let value: Value = serde_json::from_str(&response).unwrap();

    assert_eq!(value["id"], "req-1");
    assert_eq!(value["ok"], true);
    assert_eq!(value["data"]["outputBase64"], "AQID");
    assert_eq!(value["data"]["profile"], "g1r-derived-test");
    assert_eq!(value["data"]["resolutionMode"], "derived_profile_cache");
}

#[cfg(windows)]
#[test]
fn ipc_compress_rejects_uncached_pattern_profile_before_selftest() {
    let exe = minimal_pe64_with_shifted_g1r_patterns();
    let profile = parse_profile_json(&profile_json(
        "0000000000000000000000000000000000000000000000000000000000000000",
        exe.len() as u64,
        "0x23A85CE7",
        "0x140000000",
        "0x1010",
        "0x1020",
        "0x1030",
    ))
    .unwrap();
    let temp = write_temp_exe(&exe);

    let response = handle_ipc_line_with_runtime_worker(
        &format!(
            r#"{{
                "id":"req-1",
                "command":"compress",
                "exePath":"{}",
                "inputBase64":"AQID",
                "level":6
            }}"#,
            json_escape_path(temp.path())
        ),
        &[profile],
        &helper_binary_path(),
    );
    let value: Value = serde_json::from_str(&response).unwrap();

    assert_eq!(value["id"], "req-1");
    assert_eq!(value["ok"], false);
    assert_eq!(value["error"]["code"], "unsupported_exe");
    assert!(
        value["error"]["message"]
            .as_str()
            .unwrap()
            .contains("requires self_test")
    );
}

#[cfg(windows)]
#[test]
fn runtime_selftest_worker_repeats_compress_roundtrip_requested_count() {
    let mut exe = minimal_pe64_with_imports_and_relocations();
    exe[0x220..0x237].copy_from_slice(&[
        0x8a, 0x01, // mov al, byte ptr [rcx]
        0x41, 0x88, 0x00, // mov byte ptr [r8], al
        0x8a, 0x41, 0x01, // mov al, byte ptr [rcx + 1]
        0x41, 0x88, 0x40, 0x01, // mov byte ptr [r8 + 1], al
        0x8a, 0x41, 0x02, // mov al, byte ptr [rcx + 2]
        0x41, 0x88, 0x40, 0x02, // mov byte ptr [r8 + 2], al
        0x4c, 0x89, 0xc8, // mov rax, r9
        0xc3, // ret
    ]);
    exe[0x250..0x267].copy_from_slice(&[
        0x8a, 0x02, // mov al, byte ptr [rdx]
        0x41, 0x88, 0x01, // mov byte ptr [r9], al
        0x8a, 0x42, 0x01, // mov al, byte ptr [rdx + 1]
        0x41, 0x88, 0x41, 0x01, // mov byte ptr [r9 + 1], al
        0x8a, 0x42, 0x02, // mov al, byte ptr [rdx + 2]
        0x41, 0x88, 0x41, 0x02, // mov byte ptr [r9 + 2], al
        0x4c, 0x89, 0xc0, // mov rax, r8
        0xc3, // ret
    ]);
    let temp = write_temp_exe(&exe);
    let mut request = sample_runtime_worker_request(temp.path().to_path_buf());
    request.codec_rvas = Some(RuntimeCodecRvas {
        oodle_lz_compress: 0x1050,
        oodle_lz_decompress: 0x1020,
        compressor_dispatch: 0x1030,
    });
    request.run_compress_call = true;
    request.compress_repeat_count = 3;
    request.compress_sample = Some(RuntimeCompressSample {
        input_base64: "AQID".to_string(),
        level: 6,
    });

    let report = runtime_selftest_worker_report(Some(request)).unwrap();

    assert_eq!(report.compress, "passed");
    assert_eq!(report.compress_call_count, Some(3));
    assert_eq!(
        report.compress_roundtrip_sha1.as_deref(),
        Some("7037807198c22a7d2b0807371d763779a84fdfcf")
    );
}

#[cfg(windows)]
#[test]
fn ipc_self_test_forwards_compress_repeat_count_to_runtime_worker() {
    let mut exe = minimal_pe64_with_imports_and_relocations();
    exe[0x220..0x237].copy_from_slice(&[
        0x8a, 0x01, // mov al, byte ptr [rcx]
        0x41, 0x88, 0x00, // mov byte ptr [r8], al
        0x8a, 0x41, 0x01, // mov al, byte ptr [rcx + 1]
        0x41, 0x88, 0x40, 0x01, // mov byte ptr [r8 + 1], al
        0x8a, 0x41, 0x02, // mov al, byte ptr [rcx + 2]
        0x41, 0x88, 0x40, 0x02, // mov byte ptr [r8 + 2], al
        0x4c, 0x89, 0xc8, // mov rax, r9
        0xc3, // ret
    ]);
    exe[0x250..0x267].copy_from_slice(&[
        0x8a, 0x02, // mov al, byte ptr [rdx]
        0x41, 0x88, 0x01, // mov byte ptr [r9], al
        0x8a, 0x42, 0x01, // mov al, byte ptr [rdx + 1]
        0x41, 0x88, 0x41, 0x01, // mov byte ptr [r9 + 1], al
        0x8a, 0x42, 0x02, // mov al, byte ptr [rdx + 2]
        0x41, 0x88, 0x41, 0x02, // mov byte ptr [r9 + 2], al
        0x4c, 0x89, 0xc0, // mov rax, r8
        0xc3, // ret
    ]);
    let profile = parse_profile_json(&profile_json(
        &sha256_hex(&exe),
        exe.len() as u64,
        "0x23A85CE7",
        "0x140000000",
        "0x1050",
        "0x1020",
        "0x1030",
    ))
    .unwrap();
    let temp = write_temp_exe(&exe);

    let response = handle_ipc_line_with_runtime_worker(
        &format!(
            r#"{{"id":"req-1","command":"self_test","exePath":"{}","runRuntimeSelftests":true,"runtimeSelftestRunCompress":true,"runtimeSelftestCompressRepeatCount":3,"runtimeSelftestCompressSample":{{"inputBase64":"AQID","level":6}}}}"#,
            json_escape_path(temp.path())
        ),
        &[profile],
        &helper_binary_path(),
    );
    let value: Value = serde_json::from_str(&response).unwrap();

    assert_eq!(value["id"], "req-1");
    assert_eq!(value["ok"], true);
    assert_eq!(value["data"]["runtimeSelftests"]["compress"], "passed");
    assert_eq!(value["data"]["runtimeSelftests"]["compressCallCount"], 3);
}

#[cfg(windows)]
#[test]
fn ipc_self_test_marks_can_compress_when_decompress_and_compress_pass() {
    let mut exe = minimal_pe64_with_imports_and_relocations();
    exe[0x220..0x237].copy_from_slice(&[
        0x8a, 0x01, // mov al, byte ptr [rcx]
        0x41, 0x88, 0x00, // mov byte ptr [r8], al
        0x8a, 0x41, 0x01, // mov al, byte ptr [rcx + 1]
        0x41, 0x88, 0x40, 0x01, // mov byte ptr [r8 + 1], al
        0x8a, 0x41, 0x02, // mov al, byte ptr [rcx + 2]
        0x41, 0x88, 0x40, 0x02, // mov byte ptr [r8 + 2], al
        0x4c, 0x89, 0xc8, // mov rax, r9
        0xc3, // ret
    ]);
    exe[0x250..0x267].copy_from_slice(&[
        0x8a, 0x02, // mov al, byte ptr [rdx]
        0x41, 0x88, 0x01, // mov byte ptr [r9], al
        0x8a, 0x42, 0x01, // mov al, byte ptr [rdx + 1]
        0x41, 0x88, 0x41, 0x01, // mov byte ptr [r9 + 1], al
        0x8a, 0x42, 0x02, // mov al, byte ptr [rdx + 2]
        0x41, 0x88, 0x41, 0x02, // mov byte ptr [r9 + 2], al
        0x4c, 0x89, 0xc0, // mov rax, r8
        0xc3, // ret
    ]);
    let profile = parse_profile_json(&profile_json(
        &sha256_hex(&exe),
        exe.len() as u64,
        "0x23A85CE7",
        "0x140000000",
        "0x1050",
        "0x1020",
        "0x1030",
    ))
    .unwrap();
    let temp = write_temp_exe(&exe);

    let response = handle_ipc_line_with_runtime_worker(
        &format!(
            r#"{{"id":"req-1","command":"self_test","exePath":"{}","runRuntimeSelftests":true,"runtimeSelftestRunDecompress":true,"runtimeSelftestRunCompress":true,"runtimeSelftestSample":{{"compressedBase64":"AQID","expectedSize":3,"expectedDecompressedSha1":"7037807198c22a7d2b0807371d763779a84fdfcf","expectedDecompressedHeadHex":"010203"}},"runtimeSelftestCompressSample":{{"inputBase64":"AQID","level":6}}}}"#,
            json_escape_path(temp.path())
        ),
        &[profile],
        &helper_binary_path(),
    );
    let value: Value = serde_json::from_str(&response).unwrap();

    assert_eq!(value["id"], "req-1");
    assert_eq!(value["ok"], true);
    assert_eq!(value["data"]["runtimeSelftests"]["decompress"], "passed");
    assert_eq!(value["data"]["runtimeSelftests"]["compress"], "passed");
    assert_eq!(value["data"]["canDecompress"], true);
    assert_eq!(value["data"]["canCompress"], true);
}

#[cfg(windows)]
#[test]
fn windows_import_resolver_resolves_kernel32_by_name() {
    let mut resolver = WindowsImportResolver::default();
    let symbol = PeImportSymbol {
        name: Some("GetProcAddress".to_string()),
        hint: None,
        ordinal: None,
    };

    let address = resolver.resolve_import("KERNEL32.dll", &symbol).unwrap();

    assert_ne!(address, 0);
}

#[cfg(windows)]
#[test]
fn windows_import_resolver_prefers_app_local_search_dir_candidate() {
    let temp_dir = tempfile::tempdir().unwrap();
    let dll_path = temp_dir.path().join("Example.dll");
    std::fs::write(&dll_path, b"not a real dll").unwrap();
    let resolver = WindowsImportResolver::with_search_dirs(vec![temp_dir.path().to_path_buf()]);

    let candidate = resolver.candidate_dll_path("Example.dll");

    assert_eq!(candidate.as_deref(), Some(dll_path.as_path()));
}

#[test]
fn probe_rejects_missing_exe() {
    let err = probe_exe(
        &ProbeRequest {
            exe_path: Path::new(r"C:\missing\G1R-Win64-Shipping.exe").to_path_buf(),
        },
        &[],
    )
    .unwrap_err();

    assert_eq!(err.code(), ErrorCode::MissingExe);
}

#[test]
fn probe_accepts_known_profile_when_hash_and_pe_match() {
    let exe = minimal_pe64_with_text_section();
    let profile = parse_profile_json(&profile_json(
        &sha256_hex(&exe),
        exe.len() as u64,
        "0x23A85CE7",
        "0x140000000",
        "0x1010",
        "0x1020",
        "0x1030",
    ))
    .unwrap();
    let temp = write_temp_exe(&exe);

    let response = probe_exe(
        &ProbeRequest {
            exe_path: temp.path().to_path_buf(),
        },
        &[profile],
    )
    .unwrap();

    assert!(response.supported);
    assert_eq!(response.profile.as_deref(), Some("g1r-test"));
    assert_eq!(response.resolution_mode, Some(ResolutionMode::KnownProfile));
    assert!(!response.can_compress);
    assert!(!response.can_decompress);
}

#[test]
fn probe_rejects_known_hash_when_rva_is_not_executable() {
    let exe = minimal_pe64_with_text_section();
    let profile = parse_profile_json(&profile_json(
        &sha256_hex(&exe),
        exe.len() as u64,
        "0x23A85CE7",
        "0x140000000",
        "0x9000",
        "0x1020",
        "0x1030",
    ))
    .unwrap();
    let temp = write_temp_exe(&exe);

    let err = probe_exe(
        &ProbeRequest {
            exe_path: temp.path().to_path_buf(),
        },
        &[profile],
    )
    .unwrap_err();

    assert_eq!(err.code(), ErrorCode::UnsupportedExe);
    assert!(err.to_string().contains("outside executable sections"));
}

#[test]
fn probe_accepts_unknown_hash_when_patterns_resolve_shifted_rvas() {
    let exe = minimal_pe64_with_shifted_g1r_patterns();
    let profile = parse_profile_json(&profile_json(
        "0000000000000000000000000000000000000000000000000000000000000000",
        999999,
        "0x11111111",
        "0x150000000",
        "0x1010",
        "0x1020",
        "0x1030",
    ))
    .unwrap();
    let temp = write_temp_exe(&exe);

    let response = probe_exe(
        &ProbeRequest {
            exe_path: temp.path().to_path_buf(),
        },
        &[profile],
    )
    .unwrap();

    assert!(!response.supported);
    assert_eq!(response.profile.as_deref(), Some("g1r-test"));
    assert_eq!(
        response.resolution_mode,
        Some(ResolutionMode::PatternProfile)
    );
    assert!(!response.can_compress);
    assert!(!response.can_decompress);
    assert_eq!(
        response.resolved_rvas,
        Some(RuntimeCodecRvas {
            oodle_lz_compress: 0x1060,
            oodle_lz_decompress: 0x10d0,
            compressor_dispatch: 0x1140,
        })
    );
    assert_eq!(
        response.resolver_attempts,
        vec!["known_profile", "pattern_profile"]
    );
}

#[test]
fn probe_rejects_unknown_hash_when_patterns_are_missing_even_if_old_rvas_are_executable() {
    let exe = minimal_pe64_with_text_section();
    let profile = parse_profile_json(&profile_json(
        "0000000000000000000000000000000000000000000000000000000000000000",
        exe.len() as u64,
        "0x23A85CE7",
        "0x140000000",
        "0x1010",
        "0x1020",
        "0x1030",
    ))
    .unwrap();
    let temp = write_temp_exe(&exe);

    let err = probe_exe(
        &ProbeRequest {
            exe_path: temp.path().to_path_buf(),
        },
        &[profile],
    )
    .unwrap_err();

    assert_eq!(err.code(), ErrorCode::UnsupportedExe);
    assert!(
        err.to_string()
            .contains("could not be resolved to verified codec functions")
    );
}

#[test]
fn probe_rejects_unknown_hash_when_pattern_match_is_ambiguous() {
    let mut exe = minimal_pe64_with_shifted_g1r_patterns();
    exe[0x220..0x220 + G1R_COMPRESS_PATTERN.len()].copy_from_slice(&G1R_COMPRESS_PATTERN);
    let profile = parse_profile_json(&profile_json(
        "0000000000000000000000000000000000000000000000000000000000000000",
        exe.len() as u64,
        "0x23A85CE7",
        "0x140000000",
        "0x1010",
        "0x1020",
        "0x1030",
    ))
    .unwrap();
    let temp = write_temp_exe(&exe);

    let err = probe_exe(
        &ProbeRequest {
            exe_path: temp.path().to_path_buf(),
        },
        &[profile],
    )
    .unwrap_err();

    assert_eq!(err.code(), ErrorCode::UnsupportedExe);
    assert!(
        err.to_string()
            .contains("could not be resolved to verified codec functions")
    );
}

#[test]
fn probe_rejects_pattern_profile_when_declared_near_strings_are_missing() {
    let exe = minimal_pe64_with_shifted_g1r_patterns();
    let profile = parse_profile_json(&profile_json_with_patterns(
        "0000000000000000000000000000000000000000000000000000000000000000",
        exe.len() as u64,
        r#"{
            "compressAnchors": [
                {
                    "name": "compress_missing_anchor",
                    "nearStrings": ["definitely-not-in-this-exe"]
                }
            ]
        }"#,
    ))
    .unwrap();
    let temp = write_temp_exe(&exe);

    let err = probe_exe(
        &ProbeRequest {
            exe_path: temp.path().to_path_buf(),
        },
        &[profile],
    )
    .unwrap_err();

    assert_eq!(err.code(), ErrorCode::UnsupportedExe);
    assert!(
        err.to_string()
            .contains("could not be resolved to verified codec functions")
    );
}

#[test]
fn probe_rejects_pattern_profile_when_compress_does_not_call_dispatch() {
    let mut exe = minimal_pe64_with_shifted_g1r_patterns();
    exe[0x2a8] = 0x90;
    let profile = parse_profile_json(&profile_json(
        "0000000000000000000000000000000000000000000000000000000000000000",
        exe.len() as u64,
        "0x23A85CE7",
        "0x140000000",
        "0x1010",
        "0x1020",
        "0x1030",
    ))
    .unwrap();
    let temp = write_temp_exe(&exe);

    let err = probe_exe(
        &ProbeRequest {
            exe_path: temp.path().to_path_buf(),
        },
        &[profile],
    )
    .unwrap_err();

    assert_eq!(err.code(), ErrorCode::UnsupportedExe);
    assert!(
        err.to_string()
            .contains("could not be resolved to verified codec functions")
    );
}

#[test]
fn probe_uses_verified_derived_profile_cache_for_same_exe_hash() {
    let exe = minimal_pe64_with_text_section();
    let temp = write_temp_exe(&exe);
    let dir = tempfile::tempdir().unwrap();
    let cache_path = dir.path().join("derived_profiles.json");
    let entry = derived_profile_entry_for_exe(&exe, 0x1010, 0x1020, 0x1030, true, true);
    write_derived_profile_cache_entry(&cache_path, entry).unwrap();

    assert!(
        probe_exe(
            &ProbeRequest {
                exe_path: temp.path().to_path_buf(),
            },
            &[],
        )
        .is_err()
    );

    let response = probe_exe_with_derived_cache(
        &ProbeRequest {
            exe_path: temp.path().to_path_buf(),
        },
        &[],
        Some(&cache_path),
    )
    .unwrap();

    assert!(response.supported);
    assert_eq!(
        response.resolution_mode,
        Some(ResolutionMode::DerivedProfileCache)
    );
    assert!(response.can_decompress);
    assert!(response.can_compress);
    assert_eq!(
        response.resolver_attempts,
        vec!["known_profile", "derived_profile_cache"]
    );
}

#[cfg(windows)]
#[test]
fn ipc_self_test_forwards_pattern_resolved_rvas_to_runtime_worker_preflight() {
    let exe = minimal_pe64_with_shifted_g1r_patterns();
    let profile = parse_profile_json(&profile_json(
        "0000000000000000000000000000000000000000000000000000000000000000",
        999999,
        "0x11111111",
        "0x150000000",
        "0x1010",
        "0x1020",
        "0x1030",
    ))
    .unwrap();
    let temp = write_temp_exe(&exe);

    let response = handle_ipc_line_with_runtime_worker(
        &format!(
            r#"{{
                "id":"req-1",
                "command":"self_test",
                "exePath":"{}",
                "runRuntimeSelftests":true,
                "runtimeSelftestSample":{{
                    "compressedBase64":"AQID",
                    "expectedSize":3,
                    "expectedDecompressedSha1":"00112233445566778899aabbccddeeff00112233",
                    "expectedDecompressedHeadHex":"010203"
                }}
            }}"#,
            json_escape_path(temp.path())
        ),
        &[profile],
        &helper_binary_path(),
    );
    let value: Value = serde_json::from_str(&response).unwrap();

    assert_eq!(value["id"], "req-1");
    assert_eq!(value["ok"], true);
    assert_eq!(value["data"]["resolutionMode"], "pattern_profile");
    assert_eq!(
        value["data"]["runtimeSelftests"]["runtimePreflight"]["resolvedRvas"][0]["rva"],
        0x1060
    );
    assert_eq!(
        value["data"]["runtimeSelftests"]["runtimePreflight"]["resolvedRvas"][1]["rva"],
        0x10d0
    );
    assert_eq!(
        value["data"]["runtimeSelftests"]["runtimePreflight"]["resolvedRvas"][2]["rva"],
        0x1140
    );
}

#[test]
fn derived_profile_entry_is_created_only_after_pattern_runtime_selftest_passes() {
    let exe = minimal_pe64_with_shifted_g1r_patterns();
    let temp = write_temp_exe(&exe);

    let entry = derived_profile_entry_from_verified_self_test(
        temp.path(),
        &pattern_self_test_response(true, false),
    )
    .unwrap()
    .unwrap();

    assert_eq!(entry.source_profile.as_deref(), Some("g1r-test"));
    assert_eq!(entry.exe_sha256, sha256_hex(&exe));
    assert_eq!(entry.file_size, exe.len() as u64);
    assert_eq!(entry.pe_timestamp, "0x23A85CE7");
    assert_eq!(entry.image_base, "0x140000000");
    assert_eq!(
        entry.resolved_rvas,
        RuntimeCodecRvas {
            oodle_lz_compress: 0x1060,
            oodle_lz_decompress: 0x10d0,
            compressor_dispatch: 0x1140,
        }
    );
    assert_eq!(
        entry.confidence,
        "pattern_resolved_decompress_selftest_passed"
    );

    assert!(
        derived_profile_entry_from_verified_self_test(
            temp.path(),
            &known_self_test_response(true, true),
        )
        .unwrap()
        .is_none()
    );
    assert!(
        derived_profile_entry_from_verified_self_test(
            temp.path(),
            &pattern_self_test_response(false, false),
        )
        .unwrap()
        .is_none()
    );
}

#[test]
fn derived_profile_cache_write_and_export_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let cache_path = dir.path().join("derived_profiles.json");
    let entry = sample_derived_profile_entry();

    write_derived_profile_cache_entry(&cache_path, entry.clone()).unwrap();

    let exported = export_derived_profile_from_cache(&cache_path, &entry.exe_sha256).unwrap();
    assert_eq!(exported.entry, entry);

    let response = handle_ipc_line(
        &format!(
            r#"{{
                "id":"req-1",
                "command":"export_derived_profile",
                "cachePath":"{}",
                "exeSha256":"{}"
            }}"#,
            json_escape_path(&cache_path),
            entry.exe_sha256
        ),
        &[],
    );
    let value: Value = serde_json::from_str(&response).unwrap();

    assert_eq!(value["id"], "req-1");
    assert_eq!(value["ok"], true);
    assert_eq!(value["data"]["entry"]["name"], "g1r-derived-test");
    assert_eq!(
        value["data"]["entry"]["resolvedRvas"]["oodleLzCompress"],
        0x1060
    );
}

#[test]
fn record_derived_profile_cache_after_self_test_writes_only_verified_pattern_profiles() {
    let exe = minimal_pe64_with_shifted_g1r_patterns();
    let temp = write_temp_exe(&exe);
    let dir = tempfile::tempdir().unwrap();
    let cache_path = dir.path().join("derived_profiles.json");

    let report = record_derived_profile_cache_after_self_test(
        temp.path(),
        &pattern_self_test_response(true, true),
        Some(&cache_path),
    )
    .unwrap();

    let expected_sha256 = sha256_hex(&exe);
    assert!(report.written);
    assert_eq!(report.sha256.as_deref(), Some(expected_sha256.as_str()));
    let exported =
        export_derived_profile_from_cache(&cache_path, report.sha256.as_deref().unwrap()).unwrap();
    assert!(exported.entry.can_compress);
    assert_eq!(
        exported.entry.confidence,
        "pattern_resolved_compress_roundtrip_passed"
    );

    let known_report = record_derived_profile_cache_after_self_test(
        temp.path(),
        &known_self_test_response(true, true),
        Some(&cache_path),
    )
    .unwrap();
    assert!(!known_report.written);
}

#[test]
fn ipc_self_test_reports_derived_cache_not_written_without_verified_pattern_selftests() {
    let exe = minimal_pe64_with_shifted_g1r_patterns();
    let profile = parse_profile_json(&profile_json(
        "0000000000000000000000000000000000000000000000000000000000000000",
        999999,
        "0x11111111",
        "0x150000000",
        "0x1010",
        "0x1020",
        "0x1030",
    ))
    .unwrap();
    let temp = write_temp_exe(&exe);
    let dir = tempfile::tempdir().unwrap();
    let cache_path = dir.path().join("derived_profiles.json");

    let response = handle_ipc_line(
        &format!(
            r#"{{
                "id":"req-1",
                "command":"self_test",
                "exePath":"{}",
                "derivedProfileCachePath":"{}"
            }}"#,
            json_escape_path(temp.path()),
            json_escape_path(&cache_path)
        ),
        &[profile],
    );
    let value: Value = serde_json::from_str(&response).unwrap();

    assert_eq!(value["id"], "req-1");
    assert_eq!(value["ok"], true);
    assert_eq!(value["data"]["resolutionMode"], "pattern_profile");
    assert_eq!(value["data"]["derivedProfileCache"]["written"], false);
    assert!(
        value["data"]["derivedProfileCache"]["reason"]
            .as_str()
            .unwrap()
            .contains("runtime selftests")
    );
    assert!(!cache_path.exists());
}

fn profile_json(
    exe_sha256: &str,
    file_size: u64,
    pe_timestamp: &str,
    image_base: &str,
    compress_rva: &str,
    decompress_rva: &str,
    dispatch_rva: &str,
) -> String {
    format!(
        r#"{{
            "name": "g1r-test",
            "exeSha256": "{exe_sha256}",
            "fileSize": {file_size},
            "peTimestamp": "{pe_timestamp}",
            "imageBase": "{image_base}",
            "rvAs": {{
                "oodleLzCompress": "{compress_rva}",
                "oodleLzDecompress": "{decompress_rva}",
                "compressorDispatch": "{dispatch_rva}"
            }},
            "fingerprints": {{}},
            "patterns": {{}}
        }}"#
    )
}

fn profile_json_with_patterns(exe_sha256: &str, file_size: u64, patterns: &str) -> String {
    format!(
        r#"{{
            "name": "g1r-test",
            "exeSha256": "{exe_sha256}",
            "fileSize": {file_size},
            "peTimestamp": "0x23A85CE7",
            "imageBase": "0x140000000",
            "rvAs": {{
                "oodleLzCompress": "0x1010",
                "oodleLzDecompress": "0x1020",
                "compressorDispatch": "0x1030"
            }},
            "fingerprints": {{}},
            "patterns": {patterns}
        }}"#
    )
}

fn write_temp_exe(bytes: &[u8]) -> tempfile::NamedTempFile {
    let mut file = tempfile::Builder::new().suffix(".exe").tempfile().unwrap();
    file.write_all(bytes).unwrap();
    file
}

fn write_temp_sav(bytes: &[u8]) -> tempfile::NamedTempFile {
    let mut file = tempfile::Builder::new().suffix(".sav").tempfile().unwrap();
    file.write_all(bytes).unwrap();
    file
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn helper_binary_path() -> PathBuf {
    option_env!("CARGO_BIN_EXE_goresave_g1r_codec_host")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::current_exe()
                .unwrap()
                .with_file_name("goresave_g1r_codec_host.exe")
        })
}

fn sample_runtime_worker_request(exe_path: PathBuf) -> RuntimeSelftestWorkerRequest {
    RuntimeSelftestWorkerRequest {
        exe_path,
        codec_rvas: None,
        run_decompress_call: false,
        decompress_repeat_count: 1,
        return_decompressed_output: false,
        verify_decompressed_output: true,
        run_compress_call: false,
        compress_repeat_count: 1,
        return_compressed_output: false,
        compress_sample: None,
        compress_samples: Vec::new(),
        decompress_sample: RuntimeSelftestOracleSample {
            compressed_base64: "AQID".to_string(),
            expected_size: 3,
            expected_decompressed_sha1: "00112233445566778899aabbccddeeff00112233".to_string(),
            expected_decompressed_head_hex: "010203".to_string(),
        },
        decompress_samples: Vec::new(),
    }
}

fn pattern_self_test_response(can_decompress: bool, can_compress: bool) -> SelfTestResponse {
    self_test_response(
        Some(ResolutionMode::PatternProfile),
        can_decompress,
        can_compress,
    )
}

fn known_self_test_response(can_decompress: bool, can_compress: bool) -> SelfTestResponse {
    self_test_response(
        Some(ResolutionMode::KnownProfile),
        can_decompress,
        can_compress,
    )
}

fn self_test_response(
    resolution_mode: Option<ResolutionMode>,
    can_decompress: bool,
    can_compress: bool,
) -> SelfTestResponse {
    SelfTestResponse {
        profile: Some("g1r-test".to_string()),
        resolution_mode,
        private_mapping: PrivateMappingReport {
            pe_parsed: true,
            image_base: 0x140000000,
            size_of_image: 0x3000,
            copied_section_count: 1,
            import_dll_count: 0,
            import_symbol_count: 0,
            import_resolution_status: "not_run",
            fixed_import_thunk_count: 0,
            base_relocation_count: 0,
            applied_relocation_count: 0,
            section_protection_count: 0,
            memory_mapped: false,
            memory_mapped_base: None,
            memory_mapped_size: 0,
            memory_protection_applied_count: 0,
            entry_point_rva: 0x1000,
            entry_point_run: false,
            resolved_rvas: vec![
                ResolvedRvaReport {
                    name: "oodleLzCompress",
                    rva: 0x1060,
                    executable: true,
                    preferred_va: 0x140001060,
                    mapped_va: 0x140001060,
                },
                ResolvedRvaReport {
                    name: "oodleLzDecompress",
                    rva: 0x10d0,
                    executable: true,
                    preferred_va: 0x1400010d0,
                    mapped_va: 0x1400010d0,
                },
                ResolvedRvaReport {
                    name: "compressorDispatch",
                    rva: 0x1140,
                    executable: true,
                    preferred_va: 0x140001140,
                    mapped_va: 0x140001140,
                },
            ],
            section_protections: Vec::<SectionProtection>::new(),
        },
        runtime_selftests: runtime_selftest_report_for_cache(can_decompress, can_compress),
        can_compress,
        can_decompress,
    }
}

fn runtime_selftest_report_for_cache(
    can_decompress: bool,
    can_compress: bool,
) -> RuntimeSelftestReport {
    RuntimeSelftestReport {
        requested: true,
        worker_status: "completed".to_string(),
        worker_pid: None,
        worker_exit_code: Some(0),
        exe_path: None,
        decompress_sample_status: "valid".to_string(),
        decompress_sample_expected_size: Some(3),
        decompress_sample_compressed_size: Some(3),
        decompress_sample_expected_sha1: Some(
            "00112233445566778899aabbccddeeff00112233".to_string(),
        ),
        decompress_sample_expected_head_hex: Some("010203".to_string()),
        runtime_preflight: None,
        decompress_call_return: Some(0),
        decompress_call_count: Some(1),
        decompress_output_size: Some(3),
        decompress_output_sha1: Some("00112233445566778899aabbccddeeff00112233".to_string()),
        decompress_output_head_hex: Some("010203".to_string()),
        decompress_output_base64: None,
        decompress_outputs_base64: None,
        compress_call_return: can_compress.then_some(10),
        compress_call_count: can_compress.then_some(1),
        compress_output_size: can_compress.then_some(10),
        compress_output_sha1: can_compress
            .then_some("abcdefabcdefabcdefabcdefabcdefabcdefabcd".to_string()),
        compress_output_base64: None,
        compress_outputs_base64: None,
        compress_roundtrip_sha1: can_compress
            .then_some("00112233445566778899aabbccddeeff00112233".to_string()),
        decompress: if can_decompress { "passed" } else { "failed" }.to_string(),
        compress: if can_compress { "passed" } else { "not_run" }.to_string(),
        reason: Some("test report".to_string()),
    }
}

fn sample_derived_profile_entry() -> DerivedProfileCacheEntry {
    derived_profile_entry_for_exe(&[0], 0x1060, 0x10d0, 0x1140, true, false)
}

fn derived_profile_entry_for_exe(
    exe: &[u8],
    compress_rva: u32,
    decompress_rva: u32,
    dispatch_rva: u32,
    can_decompress: bool,
    can_compress: bool,
) -> DerivedProfileCacheEntry {
    let (exe_sha256, file_size, pe_timestamp, image_base) = match PeImage::parse(exe) {
        Ok(pe) => (
            sha256_hex(exe),
            exe.len() as u64,
            format!("0x{:08X}", pe.timestamp()),
            format!("0x{:X}", pe.image_base()),
        ),
        Err(_) => (
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
            1234,
            "0x23A85CE7".to_string(),
            "0x140000000".to_string(),
        ),
    };

    DerivedProfileCacheEntry {
        name: "g1r-derived-test".to_string(),
        source_profile: Some("g1r-test".to_string()),
        exe_sha256,
        file_size,
        pe_timestamp,
        image_base,
        resolution_mode: ResolutionMode::PatternProfile,
        resolved_rvas: RuntimeCodecRvas {
            oodle_lz_compress: compress_rva,
            oodle_lz_decompress: decompress_rva,
            compressor_dispatch: dispatch_rva,
        },
        can_decompress,
        can_compress,
        runtime_selftest_decompress: "passed".to_string(),
        runtime_selftest_compress: if can_compress { "passed" } else { "not_run" }.to_string(),
        confidence: if can_compress {
            "pattern_resolved_compress_roundtrip_passed"
        } else {
            "pattern_resolved_decompress_selftest_passed"
        }
        .to_string(),
        matched_anchors: vec![
            "oodle_lz_compress_wrapper_prologue".to_string(),
            "oodle_lz_decompress_wrapper_prologue".to_string(),
            "compressor_dispatch_prologue".to_string(),
        ],
        cached_at_unix_seconds: 123,
    }
}

fn pe_size(bytes: &[u8]) -> u32 {
    PeImage::parse(bytes).unwrap().size_of_image()
}

fn minimal_gsav_with_oodle_chunks(chunks: &[(&[u8], u64)]) -> Vec<u8> {
    const PACKAGE_FILE_TAG: u32 = 0x9E2A83C1;
    const COMPRESSED_HEADER_V2: u32 = 0x22222222;

    let public_payload = fstring("None");
    let summary_compressed_size = chunks
        .iter()
        .map(|(compressed, _)| compressed.len() as u64)
        .sum::<u64>();
    let summary_uncompressed_size = chunks
        .iter()
        .map(|(_, uncompressed_size)| *uncompressed_size)
        .sum::<u64>();
    let mut stream = Vec::new();
    stream.extend_from_slice(&summary_uncompressed_size.to_le_bytes());
    stream.extend_from_slice(&fstring("Oodle"));
    stream.extend_from_slice(&PACKAGE_FILE_TAG.to_le_bytes());
    stream.extend_from_slice(&COMPRESSED_HEADER_V2.to_le_bytes());
    stream.extend_from_slice(&131_072u64.to_le_bytes());
    stream.push(2);
    stream.extend_from_slice(&summary_compressed_size.to_le_bytes());
    stream.extend_from_slice(&summary_uncompressed_size.to_le_bytes());
    for (compressed, uncompressed_size) in chunks {
        stream.extend_from_slice(&(compressed.len() as u64).to_le_bytes());
        stream.extend_from_slice(&uncompressed_size.to_le_bytes());
    }
    for (compressed, _) in chunks {
        stream.extend_from_slice(compressed);
    }

    let body_size = 13 + public_payload.len() + stream.len();
    let mut out = Vec::new();
    out.extend_from_slice(b"GSAV");
    out.push(2);
    out.extend_from_slice(&(body_size as u32).to_le_bytes());
    out.extend_from_slice(&(public_payload.len() as u32).to_le_bytes());
    out.extend_from_slice(&public_payload);
    out.extend_from_slice(&stream);
    out.extend_from_slice(&[0, 0, 0, 0]);
    out
}

fn fstring(value: &str) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&((value.len() + 1) as i32).to_le_bytes());
    out.extend_from_slice(value.as_bytes());
    out.push(0);
    out
}

fn minimal_pe64_with_text_section() -> Vec<u8> {
    let mut bytes = vec![0u8; 0x400];
    bytes[0..2].copy_from_slice(b"MZ");
    write_u32(&mut bytes, 0x3c, 0x80);

    bytes[0x80..0x84].copy_from_slice(b"PE\0\0");
    write_u16(&mut bytes, 0x84, 0x8664);
    write_u16(&mut bytes, 0x86, 1);
    write_u32(&mut bytes, 0x88, 0x23A85CE7);
    write_u16(&mut bytes, 0x94, 0x00f0);
    write_u16(&mut bytes, 0x96, 0x0022);

    let optional = 0x98;
    write_u16(&mut bytes, optional, 0x020b);
    write_u32(&mut bytes, optional + 4, 0x200);
    write_u32(&mut bytes, optional + 16, 0x1000);
    write_u32(&mut bytes, optional + 20, 0x1000);
    write_u64(&mut bytes, optional + 24, 0x140000000);
    write_u32(&mut bytes, optional + 32, 0x1000);
    write_u32(&mut bytes, optional + 36, 0x200);
    write_u32(&mut bytes, optional + 56, 0x3000);
    write_u32(&mut bytes, optional + 60, 0x200);
    write_u16(&mut bytes, optional + 68, 3);
    write_u32(&mut bytes, optional + 108, 16);

    let section = optional + 0xf0;
    bytes[section..section + 5].copy_from_slice(b".text");
    write_u32(&mut bytes, section + 8, 0x1000);
    write_u32(&mut bytes, section + 12, 0x1000);
    write_u32(&mut bytes, section + 16, 0x200);
    write_u32(&mut bytes, section + 20, 0x200);
    write_u32(&mut bytes, section + 36, 0x60000020);

    bytes[0x200..0x215].copy_from_slice(b"oo2::OodleLZ_Compress");
    bytes[0x230..0x247].copy_from_slice(b"oo2::OodleLZ_Decompress");
    bytes
}

fn minimal_pe64_with_shifted_g1r_patterns() -> Vec<u8> {
    let mut bytes = minimal_pe64_with_text_section();
    bytes[0x260..0x260 + G1R_COMPRESS_PATTERN.len()].copy_from_slice(&G1R_COMPRESS_PATTERN);
    bytes[0x2d0..0x2d0 + G1R_DECOMPRESS_PATTERN.len()].copy_from_slice(&G1R_DECOMPRESS_PATTERN);
    bytes[0x340..0x340 + G1R_DISPATCH_PATTERN.len()].copy_from_slice(&G1R_DISPATCH_PATTERN);
    write_rel32_call(&mut bytes, 0x2a8, 0x10a8, 0x1140);
    write_rel32_jmp(&mut bytes, 0x2b0, 0x10b0, 0x10c0);
    write_rel32_call(&mut bytes, 0x318, 0x1118, 0x1140);
    write_rel32_jmp(&mut bytes, 0x320, 0x1120, 0x1130);
    bytes
}

const G1R_COMPRESS_PATTERN: [u8; 64] = [
    0x40, 0x55, 0x53, 0x56, 0x57, 0x41, 0x54, 0x41, 0x55, 0x41, 0x56, 0x41, 0x57, 0x48, 0x8d, 0x6c,
    0x24, 0xc8, 0x48, 0x81, 0xec, 0x38, 0x01, 0x00, 0x00, 0x48, 0xc7, 0x45, 0xb0, 0xfe, 0xff, 0xff,
    0xff, 0x48, 0x8b, 0x05, 0x00, 0x4e, 0xab, 0x02, 0x48, 0x33, 0xc4, 0x48, 0x89, 0x45, 0x20, 0x4d,
    0x8b, 0xe9, 0x4c, 0x89, 0x4c, 0x24, 0x60, 0x49, 0x8b, 0xf0, 0x48, 0x8b, 0xfa, 0x4c, 0x63, 0xf1,
];

const G1R_DECOMPRESS_PATTERN: [u8; 64] = [
    0x40, 0x55, 0x53, 0x56, 0x57, 0x41, 0x54, 0x41, 0x55, 0x41, 0x56, 0x41, 0x57, 0x48, 0x81, 0xec,
    0xf8, 0x00, 0x00, 0x00, 0x48, 0x8d, 0x6c, 0x24, 0x60, 0x48, 0xc7, 0x45, 0x48, 0xfe, 0xff, 0xff,
    0xff, 0x48, 0x8b, 0x05, 0x20, 0x45, 0xab, 0x02, 0x48, 0x33, 0xc5, 0x48, 0x89, 0x85, 0x88, 0x00,
    0x00, 0x00, 0x4c, 0x89, 0x4d, 0x18, 0x4d, 0x8b, 0xe8, 0x4c, 0x89, 0x45, 0x38, 0x4c, 0x8b, 0xf2,
];

const G1R_DISPATCH_PATTERN: [u8; 60] = [
    0x48, 0x89, 0x5c, 0x24, 0x08, 0x48, 0x89, 0x74, 0x24, 0x10, 0x57, 0x48, 0x83, 0xec, 0x50, 0x4d,
    0x8b, 0xd1, 0x48, 0x63, 0xf9, 0x48, 0x8b, 0xda, 0xb8, 0x01, 0x00, 0x00, 0x00, 0x8b, 0x94, 0x24,
    0x80, 0x00, 0x00, 0x00, 0x4d, 0x8b, 0xd8, 0x85, 0xd2, 0x44, 0x8b, 0xca, 0x44, 0x0f, 0x48, 0xc8,
    0x83, 0xff, 0x0d, 0x0f, 0x87, 0xc1, 0x01, 0x00, 0x00, 0x48, 0x8d, 0x0d,
];

fn minimal_pe64_with_imports_and_relocations() -> Vec<u8> {
    let mut bytes = vec![0u8; 0x800];
    bytes[0..2].copy_from_slice(b"MZ");
    write_u32(&mut bytes, 0x3c, 0x80);

    bytes[0x80..0x84].copy_from_slice(b"PE\0\0");
    write_u16(&mut bytes, 0x84, 0x8664);
    write_u16(&mut bytes, 0x86, 3);
    write_u32(&mut bytes, 0x88, 0x23A85CE7);
    write_u16(&mut bytes, 0x94, 0x00f0);
    write_u16(&mut bytes, 0x96, 0x0022);

    let optional = 0x98;
    write_u16(&mut bytes, optional, 0x020b);
    write_u32(&mut bytes, optional + 4, 0x200);
    write_u32(&mut bytes, optional + 16, 0x1000);
    write_u32(&mut bytes, optional + 20, 0x1000);
    write_u64(&mut bytes, optional + 24, 0x140000000);
    write_u32(&mut bytes, optional + 32, 0x1000);
    write_u32(&mut bytes, optional + 36, 0x200);
    write_u32(&mut bytes, optional + 56, 0x4000);
    write_u32(&mut bytes, optional + 60, 0x200);
    write_u16(&mut bytes, optional + 68, 3);
    write_u32(&mut bytes, optional + 108, 16);
    write_u32(&mut bytes, optional + 120, 0x2000);
    write_u32(&mut bytes, optional + 124, 0x100);
    write_u32(&mut bytes, optional + 152, 0x3000);
    write_u32(&mut bytes, optional + 156, 0x0c);

    let text = optional + 0xf0;
    bytes[text..text + 5].copy_from_slice(b".text");
    write_u32(&mut bytes, text + 8, 0x1000);
    write_u32(&mut bytes, text + 12, 0x1000);
    write_u32(&mut bytes, text + 16, 0x200);
    write_u32(&mut bytes, text + 20, 0x200);
    write_u32(&mut bytes, text + 36, 0x60000020);

    let idata = text + 40;
    bytes[idata..idata + 6].copy_from_slice(b".idata");
    write_u32(&mut bytes, idata + 8, 0x1000);
    write_u32(&mut bytes, idata + 12, 0x2000);
    write_u32(&mut bytes, idata + 16, 0x200);
    write_u32(&mut bytes, idata + 20, 0x400);
    write_u32(&mut bytes, idata + 36, 0x40000040);

    let reloc = idata + 40;
    bytes[reloc..reloc + 6].copy_from_slice(b".reloc");
    write_u32(&mut bytes, reloc + 8, 0x1000);
    write_u32(&mut bytes, reloc + 12, 0x3000);
    write_u32(&mut bytes, reloc + 16, 0x200);
    write_u32(&mut bytes, reloc + 20, 0x600);
    write_u32(&mut bytes, reloc + 36, 0x42000040);

    bytes[0x200..0x215].copy_from_slice(b"oo2::OodleLZ_Compress");
    bytes[0x230..0x247].copy_from_slice(b"oo2::OodleLZ_Decompress");
    bytes[0x220..0x246].copy_from_slice(&[
        0x48, 0x8b, 0x44, 0x24, 0x50, // mov rax, qword ptr [rsp + 0x50]
        0x48, 0x85, 0xc0, // test rax, rax
        0x75, 0x15, // jne bad_callback
        0x8b, 0x44, 0x24, 0x70, // mov eax, dword ptr [rsp + 0x70]
        0x85, 0xc0, // test eax, eax
        0x75, 0x0c, // jne bad_thread_phase
        0x4c, 0x89, 0xc8, // mov rax, r9
        0xc3, // ret
        0x48, 0xc7, 0xc0, 0x64, 0x00, 0x00, 0x00, // bad_callback: mov rax, 100
        0xc3, // ret
        0x48, 0xc7, 0xc0, 0x65, 0x00, 0x00, 0x00, // bad_thread_phase: mov rax, 101
        0xc3, // ret
    ]);
    write_u64(&mut bytes, 0x218, 0x140001234);

    write_u32(&mut bytes, 0x400, 0x2050);
    write_u32(&mut bytes, 0x404, 0);
    write_u32(&mut bytes, 0x408, 0);
    write_u32(&mut bytes, 0x40c, 0x2030);
    write_u32(&mut bytes, 0x410, 0x2070);
    bytes[0x430..0x43d].copy_from_slice(b"KERNEL32.dll\0");
    write_u16(&mut bytes, 0x480, 7);
    bytes[0x482..0x491].copy_from_slice(b"GetProcAddress\0");
    write_u64(&mut bytes, 0x450, 0x2080);
    write_u64(&mut bytes, 0x458, 0);
    write_u64(&mut bytes, 0x470, 0x2080);
    write_u64(&mut bytes, 0x478, 0);

    write_u32(&mut bytes, 0x600, 0x1000);
    write_u32(&mut bytes, 0x604, 0x0c);
    write_u16(&mut bytes, 0x608, (10 << 12) | 0x18);
    write_u16(&mut bytes, 0x60a, 0);

    bytes
}

fn json_escape_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "\\\\")
}

#[derive(Default)]
struct RecordingImportResolver {
    calls: Vec<(String, String)>,
}

impl ImportResolver for RecordingImportResolver {
    fn resolve_import(
        &mut self,
        dll_name: &str,
        symbol: &PeImportSymbol,
    ) -> Result<u64, HostError> {
        let name = symbol
            .name
            .clone()
            .unwrap_or_else(|| format!("#{}", symbol.ordinal.unwrap()));
        self.calls.push((dll_name.to_string(), name));
        Ok(0x180012340)
    }
}

fn write_u16(bytes: &mut [u8], offset: usize, value: u16) {
    bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

fn write_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn write_u64(bytes: &mut [u8], offset: usize, value: u64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

fn write_rel32_call(bytes: &mut [u8], offset: usize, source_rva: u32, target_rva: u32) {
    bytes[offset] = 0xE8;
    write_i32(bytes, offset + 1, rel32(source_rva, target_rva));
}

fn write_rel32_jmp(bytes: &mut [u8], offset: usize, source_rva: u32, target_rva: u32) {
    bytes[offset] = 0xE9;
    write_i32(bytes, offset + 1, rel32(source_rva, target_rva));
}

fn rel32(source_rva: u32, target_rva: u32) -> i32 {
    (target_rva as i64 - source_rva as i64 - 5) as i32
}

fn write_i32(bytes: &mut [u8], offset: usize, value: i32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}
