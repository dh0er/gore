# Profile Savegame Screenshots Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Show profile information and savegame screenshots in the existing GoReSave editor.

**Architecture:** Extend the Rust core JSON contract with optional profile and screenshot summaries, parsed from `PersistentDataList.sav` and `Profile_<id>_Screenshots.sav`. Mirror those fields in Flutter domain models, then render a profile header, screenshot save cards, and an Overview screenshot hero while preserving existing editor workflows.

**Tech Stack:** Rust core with serde/base64, Flutter/Dart with Riverpod and Material 3, existing `python test.py` repository test runner.

---

### Task 1: Core Profile Metadata

**Files:**
- Modify: `crates/goresave_core/src/lib.rs`

- [ ] **Step 1: Write failing Rust tests**

Add tests near the existing persistent metadata tests:

```rust
#[test]
fn scan_save_dir_reports_profile_summaries_from_persistent_data_list() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("PersistentDataList.sav"), persistent_data_list(&[
        ("G1R-001", "Auto", 1, "MainMap", 60.0, false, true),
        ("G1R-002", "Quick", 1, "MainMap", 120.0, true, false),
    ])).unwrap();
    fs::write(
        dir.path().join("G1R-001.sav"),
        build_gsav(2, &public_payload("Auto"), &compressed_stream_with_one_chunk(b"seed", 4), &[0, 0, 0, 0]),
    ).unwrap();

    let value = execute_json_inner(&json!({
        "command": "scan_save_dir",
        "payload": { "path": dir.path() }
    }).to_string()).unwrap();

    assert_eq!(value["activeProfileId"], 0);
    assert_eq!(value["profiles"][0]["profileId"], 0);
    assert_eq!(value["profiles"][0]["profileName"], "0");
    assert_eq!(value["profiles"][0]["quickSaveSlots"], json!(["G1R-001", "G1R-002", "G1R-003"]));
    assert_eq!(value["profiles"][0]["autoSaveSlots"], json!(["G1R-001", "G1R-002"]));
    assert_eq!(value["profiles"][0]["savedSlots"], json!(["G1R-001", "G1R-002"]));
}
```

- [ ] **Step 2: Run test to verify RED**

Run: `cargo test -p goresave_core scan_save_dir_reports_profile_summaries_from_persistent_data_list`

Expected: compile or assertion failure because `profiles` and `activeProfileId` are not implemented.

- [ ] **Step 3: Implement profile structs and parsing**

Add a serializable `ProfileSummary` and parse profile data from the existing FString scan and property readers. Reuse `value_after_property_in_range`, `read_i32_property_in_range`, and simple array parsing helpers for quick, auto, and saved slot arrays.

- [ ] **Step 4: Return profile data from scan**

Change `scan_save_dir` callers through `execute_json_inner("scan_save_dir")` so the response contains `saves`, `profiles`, and `activeProfileId`. Keep `scan_save_dir(&Path) -> Result<Vec<SaveListItem>, CoreError>` available for existing tests by adding a higher-level `scan_save_dir_summary`.

- [ ] **Step 5: Run GREEN**

Run: `cargo test -p goresave_core scan_save_dir_reports_profile_summaries_from_persistent_data_list`

Expected: pass.

### Task 2: Core Screenshot Extraction

**Files:**
- Modify: `crates/goresave_core/src/lib.rs`

- [ ] **Step 1: Write failing Rust tests**

Add tests for extracting JPEG payloads from a synthetic screenshot private blob and for scan attaching them to slots:

```rust
#[test]
fn parse_screenshot_payload_extracts_jpeg_by_slot() {
    let payload = screenshot_private_payload(&[
        ("G1R-001", &[0xff, 0xd8, 0x01, 0x02, 0xff, 0xd9]),
    ]);

    let screenshots = parse_screenshot_payload(&payload);

    assert_eq!(screenshots["G1R-001"].mime_type, "image/jpeg");
    assert_eq!(screenshots["G1R-001"].byte_length, 6);
    assert_eq!(screenshots["G1R-001"].bytes_base64, "/9gBAv/Z");
}

#[test]
fn scan_save_dir_attaches_screenshot_to_matching_slot() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("PersistentDataList.sav"), persistent_data_list(&[
        ("G1R-001", "Auto", 1, "MainMap", 60.0, false, true),
    ])).unwrap();
    fs::write(
        dir.path().join("G1R-001.sav"),
        build_gsav(2, &public_payload("Auto"), &compressed_stream_with_one_chunk(b"seed", 4), &[0, 0, 0, 0]),
    ).unwrap();
    fs::write(
        dir.path().join("Profile_0_Screenshots.sav"),
        screenshot_gsav_for_tests(&[("G1R-001", &[0xff, 0xd8, 0x01, 0x02, 0xff, 0xd9])]),
    ).unwrap();

    let value = execute_json_inner(&json!({
        "command": "scan_save_dir",
        "payload": { "path": dir.path() }
    }).to_string()).unwrap();

    assert_eq!(value["saves"][0]["screenshot"]["mimeType"], "image/jpeg");
    assert_eq!(value["saves"][0]["screenshot"]["byteLength"], 6);
}
```

- [ ] **Step 2: Run tests to verify RED**

Run: `cargo test -p goresave_core screenshot`

Expected: compile failure because screenshot helpers/types do not exist.

- [ ] **Step 3: Implement screenshot summary and parser**

Add `ScreenshotSummary` with `mime_type`, `byte_length`, and `bytes_base64`. Implement a conservative parser that scans the decoded screenshot payload for `G1R-###` slot keys followed by JPEG SOI/EOI bytes and stores valid JPEGs only.

- [ ] **Step 4: Attach screenshots without failing scans**

Read `Profile_<profileId>_Screenshots.sav` when present. For tests and uncompressed-like synthetic fixtures, support extracting from a GSAV private stream with stored chunks. If extraction fails, return profile and saves without screenshots.

- [ ] **Step 5: Run GREEN**

Run: `cargo test -p goresave_core screenshot`

Expected: pass.

### Task 3: Flutter Domain Models

**Files:**
- Modify: `apps/goresave/lib/features/editor/domain/editor_models.dart`
- Modify: `apps/goresave/lib/features/editor/domain/editor_notifier.dart`
- Modify: `apps/goresave/test/features/editor/domain/editor_models_test.dart`
- Modify: `apps/goresave/test/editor_notifier_test.dart`

- [ ] **Step 1: Write failing Dart model tests**

Add tests that parse `ProfileSummary`, `ScreenshotSummary`, `SaveSlot.screenshot`, and `SaveInspection.screenshot`.

- [ ] **Step 2: Run test to verify RED**

Run: `cd apps\goresave; flutter test test\features\editor\domain\editor_models_test.dart`

Expected: compile failure because models do not exist.

- [ ] **Step 3: Implement Dart models**

Add immutable `ScreenshotSummary` and `ProfileSummary`. Extend `SaveSlot`, `SaveInspection`, and `EditorState` with optional screenshot/profile lists and active profile lookup.

- [ ] **Step 4: Parse scan response profiles**

Update `EditorNotifier.refresh` to read `profiles` and `activeProfileId` from scan data while preserving existing selection behavior.

- [ ] **Step 5: Run GREEN**

Run: `cd apps\goresave; flutter test test\features\editor\domain\editor_models_test.dart test\editor_notifier_test.dart`

Expected: pass.

### Task 4: Flutter UI

**Files:**
- Modify: `apps/goresave/lib/features/editor/ui/editor_page.dart`
- Modify: `apps/goresave/test/widget_test.dart`

- [ ] **Step 1: Write failing widget tests**

Update the fake core response to include profile and screenshot fields. Add expectations for profile header text, screenshot image widgets, selected save card facts, and Overview screenshot hero.

- [ ] **Step 2: Run widget tests to verify RED**

Run: `cd apps\goresave; flutter test test\widget_test.dart`

Expected: failures because UI does not render the new profile/screenshot surfaces.

- [ ] **Step 3: Implement profile header and save cards**

Replace the plain sidebar list tiles with fixed-aspect screenshot cards, a profile header, and fallback placeholders.

- [ ] **Step 4: Implement Overview screenshot hero**

Replace the current header card with a screenshot hero that shows image or placeholder plus selected save metadata.

- [ ] **Step 5: Run GREEN**

Run: `cd apps\goresave; flutter test test\widget_test.dart`

Expected: pass.

### Task 5: Full Verification

**Files:**
- Modify as needed from prior tasks only.

- [ ] **Step 1: Format code**

Run: `cargo fmt` and `cd apps\goresave; dart format lib test`.

- [ ] **Step 2: Run full repository tests**

Run: `python test.py`

Expected: all configured Rust and Flutter tests pass.

- [ ] **Step 3: Build or run targeted app smoke**

Run: `cd apps\goresave; flutter test`.

Expected: all Flutter tests pass.

- [ ] **Step 4: Review diff**

Run: `git diff --stat` and `git diff --check`.

Expected: no whitespace errors; diff only contains plan, tests, core, models, and UI files for this feature.
