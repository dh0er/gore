# Auto-Updater + GitHub Release Workflow Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** goresave updates itself from GitHub Releases via Velopack; a tag push builds and publishes the installer + update feed.

**Architecture:** The Rust core (`goresave_core`) gains a `velopack`-backed updater module exposed through the existing `goresave_execute(json)` FFI channel plus one new startup export. Flutter calls the startup hook first thing in `main()`, checks/downloads updates in the background via a `StateNotifier`, and shows a banner when an update is staged. CI packs the Flutter release folder with `vpk` and attaches the Velopack assets to the GitHub release; the app reads the feed from the stable `releases/latest/download/` URL (no token, repo is public).

**Tech Stack:** Rust (`velopack` crate, thiserror, serde_json), Flutter/Dart (riverpod `StateNotifier`, dart:ffi), GitHub Actions (`vpk` CLI via dotnet tool).

**Spec:** `docs/superpowers/specs/2026-06-11-auto-updater-design.md`

**Repo facts (verified):**
- FFI entry: `crates/goresave_core/src/lib.rs:4717` `goresave_execute`, command dispatch in `execute_json_inner` at `lib.rs:344-353`, response wrapper `execute_json` at `lib.rs:320` (`{"ok":true,"data":...}` / `{"ok":false,"error":{code,message}}`).
- `CoreError` enum at `lib.rs:22-35` (thiserror, string payloads), error-code mapping at `lib.rs:324-331`.
- Dart service: `apps/goresave/lib/features/editor/domain/core_service.dart` — `GoresaveCoreService.execute(command, {payload})`, DLL candidates in `_candidateLibraryPaths()`.
- Provider: `apps/goresave/lib/providers/data_providers.dart:8` `coreServiceProvider`.
- App shell: `apps/goresave/lib/features/app/ui/goresave_app.dart` — `MaterialApp.router` with `builder: UiScaleRoot(...)`.
- About dialog: `apps/goresave/lib/features/app/ui/about_dialog.dart:43` shows `Version X (Build N)`.
- `build.py` builds core DLL + codec host + Flutter release into `apps/goresave/build/windows/x64/runner/Release`, zips to `dist/`.
- Riverpod style: legacy `StateNotifier` (`import 'package:flutter_riverpod/legacy.dart'`), see `ui_settings.dart`.
- Icon exists: `apps/goresave/windows/runner/resources/app_icon.ico`.
- Test runner: `python test.py` (runs cargo + flutter tests). Flutter tests live flat in `apps/goresave/test/`.

---

### Task 1: Drop build number, show git short SHA in About dialog

**Files:**
- Modify: `apps/goresave/pubspec.yaml` (version line)
- Modify: `apps/goresave/lib/features/app/ui/about_dialog.dart`
- Test: `apps/goresave/test/about_dialog_test.dart` (create)

- [ ] **Step 1: Write the failing test**

Create `apps/goresave/test/about_dialog_test.dart`:

```dart
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/app/ui/about_dialog.dart';
import 'package:package_info_plus/package_info_plus.dart';

void main() {
  test('aboutVersionLabel shows version and git sha, no build number', () {
    final info = PackageInfo(
      appName: 'goresave',
      packageName: 'goresave',
      version: '1.2.3',
      buildNumber: '7',
    );
    // Default GIT_SHA dart-define is 'dev' in tests.
    expect(aboutVersionLabel(info), 'Version 1.2.3 (dev)');
  });

  test('aboutVersionLabel is empty while package info loads', () {
    expect(aboutVersionLabel(null), '');
  });
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `flutter test test/about_dialog_test.dart` (cwd `apps/goresave`; use `fvm flutter` / the 3.44.0 toolchain as `test.py` does)
Expected: FAIL — `aboutVersionLabel` is not defined.

- [ ] **Step 3: Implement**

In `about_dialog.dart`, add below the `_githubUrl` const:

```dart
const String gitSha = String.fromEnvironment('GIT_SHA', defaultValue: 'dev');

String aboutVersionLabel(PackageInfo? info) =>
    info == null ? '' : 'Version ${info.version} ($gitSha)';
```

Replace the version computation inside the `FutureBuilder` (lines 41-43):

```dart
                  final version = aboutVersionLabel(snapshot.data);
```

(The `final info = snapshot.data;` line and the old ternary are removed.)

- [ ] **Step 4: Run test to verify it passes**

Run: `flutter test test/about_dialog_test.dart`
Expected: PASS (2 tests).

- [ ] **Step 5: Remove build number from pubspec**

In `apps/goresave/pubspec.yaml` change line 4:

```yaml
version: 0.1.0
```

(was `version: 0.1.0+1`)

- [ ] **Step 6: Full test suite**

Run: `python test.py` (repo root)
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add apps/goresave/pubspec.yaml apps/goresave/lib/features/app/ui/about_dialog.dart apps/goresave/test/about_dialog_test.dart
git commit -m "feat(app): replace build number with git short sha in about dialog"
```

---

### Task 2: Plumb GIT_SHA through build.py

**Files:**
- Modify: `build.py`

No automated test (build script); verified by CI and local dist builds.

- [ ] **Step 1: Add SHA resolution and dart-define**

In `build.py`, add after `read_version()` (line 76):

```python
def resolve_git_sha(override: str | None) -> str:
    """Short commit SHA for the About dialog: flag > CI env > git > 'dev'."""
    if override:
        return override
    env_sha = os.environ.get("GITHUB_SHA", "")
    if env_sha:
        return env_sha[:7]
    probe = subprocess.run(
        ["git", "rev-parse", "--short=7", "HEAD"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    if probe.returncode == 0 and probe.stdout.strip():
        return probe.stdout.strip()
    return "dev"
```

Change `dist()` signature and the Flutter build line:

```python
def dist(version: str, git_sha: str) -> Path:
```

```python
    run(
        "Flutter Windows release",
        [FLUTTER, "build", "windows", "--release", f"--dart-define=GIT_SHA={git_sha}"],
        cwd=APP,
    )
```

In `main()` add the argument and pass it through:

```python
    parser.add_argument("--git-sha", help="Short commit SHA (default: env/git).")
    args = parser.parse_args()
    dist(args.version or read_version(), resolve_git_sha(args.git_sha))
```

- [ ] **Step 2: Smoke-check argument parsing**

Run: `python build.py --help`
Expected: shows `--git-sha` option, exits 0. (Do not run a full dist build here.)

- [ ] **Step 3: Commit**

```bash
git add build.py
git commit -m "feat(build): inject GIT_SHA dart-define into Windows release build"
```

---

### Task 3: Rust updater module (velopack)

**Files:**
- Modify: `crates/goresave_core/Cargo.toml`
- Create: `crates/goresave_core/src/updater.rs`
- Modify: `crates/goresave_core/src/lib.rs` (module decl, `CoreError` variant, error code, dispatch arms, FFI export)

- [ ] **Step 1: Add dependency**

Run: `cargo add velopack -p goresave_core`
Expected: latest `velopack` added to `crates/goresave_core/Cargo.toml`.
(As built: velopack 1.2.0, whose API uses PascalCase fields like
`info.TargetFullRelease.Version` instead of the snake_case shown in the
snippets below — the snippets predate the 1.x release.)

- [ ] **Step 2: Add CoreError variant**

In `lib.rs`, extend the enum (after the `Validation` variant, line 34):

```rust
    #[error("update error: {0}")]
    Update(String),
```

And the code mapping in `execute_json` (after the `Validation` arm, line 330):

```rust
                CoreError::Update(_) => "UPDATE_ERROR",
```

- [ ] **Step 3: Write failing tests**

Create `crates/goresave_core/src/updater.rs` with tests only for now:

```rust
#[cfg(test)]
mod tests {
    // cargo test binaries are never Velopack-installed, so the updater must
    // report itself disabled instead of erroring.
    #[test]
    fn update_check_reports_disabled_outside_velopack_install() {
        let response = crate::execute_json(r#"{"command":"update_check","payload":{}}"#);
        let value: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(value["ok"], true, "response: {response}");
        assert_eq!(value["data"]["status"], "disabled");
    }

    #[test]
    fn update_download_without_pending_update_fails() {
        let response = crate::execute_json(r#"{"command":"update_download","payload":{}}"#);
        let value: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(value["ok"], false);
        assert_eq!(value["error"]["code"], "UPDATE_ERROR");
    }

    #[test]
    fn update_apply_restart_without_pending_update_fails() {
        let response =
            crate::execute_json(r#"{"command":"update_apply_restart","payload":{}}"#);
        let value: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(value["ok"], false);
        assert_eq!(value["error"]["code"], "UPDATE_ERROR");
    }
}
```

Add to `lib.rs` next to the existing module declarations (top of file, near `mod codec_backend;`):

```rust
mod updater;
```

- [ ] **Step 4: Run tests to verify they fail**

Run: `cargo test -p goresave_core updater`
Expected: compile error — `update_check` command unknown / functions missing.

- [ ] **Step 5: Implement updater.rs**

Prepend to `updater.rs` (above the test module):

```rust
//! Velopack-backed self-update commands.
//!
//! The update feed is the latest GitHub release: `releases/latest/download/`
//! redirects to the newest release's assets, where CI uploads
//! `releases.win.json` and the Velopack packages. No API calls, no token.

use std::sync::Mutex;

use serde_json::{json, Value};
use velopack::sources::HttpSource;
use velopack::{UpdateCheck, UpdateInfo, UpdateManager, VelopackApp};

use crate::CoreError;

const UPDATE_FEED_URL: &str = "https://github.com/dh0er/goresave/releases/latest/download/";

/// Update found by `update_check`, consumed by download/apply.
static PENDING_UPDATE: Mutex<Option<UpdateInfo>> = Mutex::new(None);

/// Runs Velopack startup hooks (install/update/uninstall callbacks). May exit
/// the process, so the app must call this before any other work.
pub fn velopack_startup() {
    VelopackApp::build().run();
}

/// None when the app is not Velopack-installed (dev run, portable zip).
fn update_manager() -> Option<UpdateManager> {
    let source = HttpSource::new(UPDATE_FEED_URL);
    UpdateManager::new(source, None, None).ok()
}

pub fn update_check(_payload: &Value) -> Result<Value, CoreError> {
    let Some(manager) = update_manager() else {
        return Ok(json!({ "status": "disabled" }));
    };
    match manager.check_for_updates() {
        Ok(UpdateCheck::UpdateAvailable(info)) => {
            let version = info.target_full_release.version.clone();
            *PENDING_UPDATE.lock().unwrap() = Some(info);
            Ok(json!({ "status": "updateAvailable", "version": version }))
        }
        Ok(_) => Ok(json!({ "status": "upToDate" })),
        Err(err) => Err(CoreError::Update(err.to_string())),
    }
}

pub fn update_download(_payload: &Value) -> Result<Value, CoreError> {
    let manager = update_manager()
        .ok_or_else(|| CoreError::Update("updater is disabled".to_string()))?;
    let pending = PENDING_UPDATE.lock().unwrap().clone();
    let info = pending
        .ok_or_else(|| CoreError::Update("no update pending; run update_check".to_string()))?;
    manager
        .download_updates(&info, None)
        .map_err(|err| CoreError::Update(err.to_string()))?;
    Ok(json!({
        "downloaded": true,
        "version": info.target_full_release.version,
    }))
}

pub fn update_apply_restart(_payload: &Value) -> Result<Value, CoreError> {
    let manager = update_manager()
        .ok_or_else(|| CoreError::Update("updater is disabled".to_string()))?;
    let pending = PENDING_UPDATE.lock().unwrap().clone();
    let info = pending
        .ok_or_else(|| CoreError::Update("no update pending; run update_check".to_string()))?;
    manager
        .apply_updates_and_restart(&info)
        .map_err(|err| CoreError::Update(err.to_string()))?;
    Ok(json!({ "applied": true }))
}
```

API-drift note: signatures above follow the velopack crate README (`UpdateManager::new(source, options, locator)`, `UpdateCheck::UpdateAvailable`, `download_updates(&info, None)`, `apply_updates_and_restart(&info)`). If the resolved crate version differs (e.g. `UpdateInfo` not `Clone` — then use `.take()` instead of `.clone()` on the mutex guard; or `apply_updates_and_restart` wants `&info.target_full_release`), adapt to the compiler against https://docs.rs/velopack. Behavior and JSON shapes must stay exactly as specified.

Add dispatch arms in `execute_json_inner` (`lib.rs`, inside `match command`, before the unknown-command fallthrough):

```rust
        "update_check" => updater::update_check(&payload),
        "update_download" => updater::update_download(&payload),
        "update_apply_restart" => updater::update_apply_restart(&payload),
```

Add the FFI export next to `goresave_execute`/`goresave_free` (`lib.rs:4716+`):

```rust
/// Velopack startup hook. Call before any other FFI function; may exit the
/// process when invoked as an install/update hook.
#[unsafe(no_mangle)]
pub extern "C" fn goresave_velopack_startup() {
    updater::velopack_startup();
}
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test -p goresave_core`
Expected: PASS, including the three new updater tests. (Test order safety: all three tests rely on `PENDING_UPDATE` staying `None`; `update_check` returns `disabled` before touching it, so parallel test order cannot break them.)

- [ ] **Step 7: Commit**

```bash
git add crates/goresave_core/Cargo.toml Cargo.lock crates/goresave_core/src/updater.rs crates/goresave_core/src/lib.rs
git commit -m "feat(core): velopack updater commands and startup hook"
```

---

### Task 4: Dart — Velopack startup call in main()

**Files:**
- Modify: `apps/goresave/lib/features/editor/domain/core_service.dart`
- Modify: `apps/goresave/lib/main.dart`

No unit test: the function is a thin FFI passthrough whose only observable behavior (process exit during hooks) cannot run under `flutter test`. Covered by manual E2E (Task 8).

- [ ] **Step 1: Add startup binding in core_service.dart**

Add typedefs next to the existing ones (after line 12):

```dart
typedef _StartupNative = Void Function();
typedef _StartupDart = void Function();
```

Add a top-level function (above `class GoresaveCoreService`):

```dart
/// Runs Velopack install/update hooks in the native core. Must be the very
/// first call in main(): Velopack may exit the process during hook
/// invocations. A missing DLL is fine (dev run without a built core).
void velopackStartup() {
  for (final candidate in _candidateLibraryPaths()) {
    try {
      final library = DynamicLibrary.open(candidate);
      final startup = library.lookupFunction<_StartupNative, _StartupDart>(
        'goresave_velopack_startup',
      );
      startup();
      return;
    } catch (_) {
      continue;
    }
  }
}
```

- [ ] **Step 2: Call it first in main()**

In `apps/goresave/lib/main.dart`, add import and call as the first statement of `main()`:

```dart
import 'package:goresave/features/editor/domain/core_service.dart';
```

```dart
Future<void> main() async {
  // Must run before anything else: Velopack hooks may exit the process.
  velopackStartup();
  WidgetsFlutterBinding.ensureInitialized();
```

- [ ] **Step 3: Analyze + tests**

Run: `flutter analyze` and `flutter test` (cwd `apps/goresave`)
Expected: no new issues, all tests pass.

- [ ] **Step 4: Commit**

```bash
git add apps/goresave/lib/features/editor/domain/core_service.dart apps/goresave/lib/main.dart
git commit -m "feat(app): run velopack startup hook before app init"
```

---

### Task 5: Dart — UpdateNotifier (check, silent download, state)

**Files:**
- Create: `apps/goresave/lib/features/app/domain/update_notifier.dart`
- Test: `apps/goresave/test/update_notifier_test.dart` (create)

- [ ] **Step 1: Write the failing tests**

Create `apps/goresave/test/update_notifier_test.dart`:

```dart
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/app/domain/update_notifier.dart';
import 'package:goresave/features/editor/domain/core_service.dart';

class _FakeCoreService implements GoresaveCoreService {
  _FakeCoreService(this.responses);

  final Map<String, Object> responses;
  final List<String> commands = [];

  @override
  bool get isAvailable => true;

  @override
  String get description => 'fake';

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
    commands.add(command);
    final response = responses[command];
    if (response is Exception) {
      throw response;
    }
    return (response as Map<String, Object?>?) ?? {'ok': false};
  }
}

void main() {
  test('downloads silently and becomes ready when update available', () async {
    final fake = _FakeCoreService({
      'update_check': {
        'ok': true,
        'data': {'status': 'updateAvailable', 'version': '0.2.0'},
      },
      'update_download': {
        'ok': true,
        'data': {'downloaded': true, 'version': '0.2.0'},
      },
    });
    final notifier = UpdateNotifier(fake);
    await pumpEventQueue();
    expect(fake.commands, ['update_check', 'update_download']);
    expect(notifier.state, isA<UpdateReady>());
    expect((notifier.state as UpdateReady).version, '0.2.0');
  });

  test('stays idle when updater disabled', () async {
    final fake = _FakeCoreService({
      'update_check': {
        'ok': true,
        'data': {'status': 'disabled'},
      },
    });
    final notifier = UpdateNotifier(fake);
    await pumpEventQueue();
    expect(fake.commands, ['update_check']);
    expect(notifier.state, isA<UpdateIdle>());
  });

  test('stays idle when up to date', () async {
    final fake = _FakeCoreService({
      'update_check': {
        'ok': true,
        'data': {'status': 'upToDate'},
      },
    });
    final notifier = UpdateNotifier(fake);
    await pumpEventQueue();
    expect(notifier.state, isA<UpdateIdle>());
  });

  test('stays idle and does not throw on check failure', () async {
    final fake = _FakeCoreService({'update_check': Exception('offline')});
    final notifier = UpdateNotifier(fake);
    await pumpEventQueue();
    expect(notifier.state, isA<UpdateIdle>());
  });

  test('stays idle when download fails', () async {
    final fake = _FakeCoreService({
      'update_check': {
        'ok': true,
        'data': {'status': 'updateAvailable', 'version': '0.2.0'},
      },
      'update_download': {
        'ok': false,
        'error': {'code': 'UPDATE_ERROR', 'message': 'network'},
      },
    });
    final notifier = UpdateNotifier(fake);
    await pumpEventQueue();
    expect(notifier.state, isA<UpdateIdle>());
  });

  test('applyAndRestart sends update_apply_restart', () async {
    final fake = _FakeCoreService({
      'update_check': {
        'ok': true,
        'data': {'status': 'disabled'},
      },
      'update_apply_restart': {
        'ok': true,
        'data': {'applied': true},
      },
    });
    final notifier = UpdateNotifier(fake);
    await pumpEventQueue();
    await notifier.applyAndRestart();
    expect(fake.commands.last, 'update_apply_restart');
  });
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `flutter test test/update_notifier_test.dart`
Expected: FAIL — `update_notifier.dart` does not exist.

- [ ] **Step 3: Implement**

Create `apps/goresave/lib/features/app/domain/update_notifier.dart`:

```dart
import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/legacy.dart';
import 'package:goresave/features/editor/domain/core_service.dart';
import 'package:goresave/providers/data_providers.dart';

sealed class UpdateState {
  const UpdateState();
}

/// No update staged: none available, updater disabled, or a check/download
/// failed (updates are best-effort and never block the app).
class UpdateIdle extends UpdateState {
  const UpdateIdle();
}

/// An update is downloaded and staged; restarting applies [version].
class UpdateReady extends UpdateState {
  const UpdateReady(this.version);

  final String version;
}

class UpdateNotifier extends StateNotifier<UpdateState> {
  UpdateNotifier(this._core) : super(const UpdateIdle()) {
    _checkAndDownload();
  }

  final GoresaveCoreService _core;

  Future<void> _checkAndDownload() async {
    try {
      final check = await _core.execute('update_check');
      final data = check['data'];
      if (check['ok'] != true ||
          data is! Map ||
          data['status'] != 'updateAvailable') {
        return;
      }
      final version = data['version'];
      final download = await _core.execute('update_download');
      if (download['ok'] == true && version is String && mounted) {
        state = UpdateReady(version);
      }
    } catch (error) {
      debugPrint('goresave update check failed: $error');
    }
  }

  Future<void> applyAndRestart() async {
    try {
      await _core.execute('update_apply_restart');
    } catch (error) {
      debugPrint('goresave update apply failed: $error');
    }
  }
}

final updateProvider = StateNotifierProvider<UpdateNotifier, UpdateState>(
  (ref) => UpdateNotifier(ref.watch(coreServiceProvider)),
);
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `flutter test test/update_notifier_test.dart`
Expected: PASS (6 tests).

- [ ] **Step 5: Commit**

```bash
git add apps/goresave/lib/features/app/domain/update_notifier.dart apps/goresave/test/update_notifier_test.dart
git commit -m "feat(app): update notifier with silent background download"
```

---

### Task 6: Dart — update banner in app shell

**Files:**
- Create: `apps/goresave/lib/features/app/ui/update_banner.dart`
- Modify: `apps/goresave/lib/features/app/ui/goresave_app.dart`
- Test: `apps/goresave/test/update_banner_test.dart` (create)

- [ ] **Step 1: Write the failing tests**

Create `apps/goresave/test/update_banner_test.dart`:

```dart
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/app/domain/update_notifier.dart';
import 'package:goresave/features/app/ui/update_banner.dart';
import 'package:goresave/features/editor/domain/core_service.dart';

class _FakeCoreService implements GoresaveCoreService {
  _FakeCoreService(this.checkData);

  final Map<String, Object?> checkData;
  final List<String> commands = [];

  @override
  bool get isAvailable => true;

  @override
  String get description => 'fake';

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
    commands.add(command);
    return switch (command) {
      'update_check' => {'ok': true, 'data': checkData},
      _ => {'ok': true, 'data': const <String, Object?>{}},
    };
  }
}

Widget _host(GoresaveCoreService core) {
  return ProviderScope(
    overrides: [
      updateProvider.overrideWith((ref) => UpdateNotifier(core)),
    ],
    child: const MaterialApp(
      home: UpdateBannerHost(child: Text('content')),
    ),
  );
}

void main() {
  testWidgets('no banner when idle', (tester) async {
    final core = _FakeCoreService({'status': 'upToDate'});
    await tester.pumpWidget(_host(core));
    await tester.pumpAndSettle();
    expect(find.text('content'), findsOneWidget);
    expect(find.textContaining('Update'), findsNothing);
  });

  testWidgets('banner shown when ready; restart triggers apply', (tester) async {
    final core = _FakeCoreService({
      'status': 'updateAvailable',
      'version': '0.2.0',
    });
    await tester.pumpWidget(_host(core));
    await tester.pumpAndSettle();
    expect(find.text('Update 0.2.0 ready'), findsOneWidget);
    expect(find.text('content'), findsOneWidget);

    await tester.tap(find.text('Restart to update'));
    await tester.pumpAndSettle();
    expect(core.commands, contains('update_apply_restart'));
  });
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `flutter test test/update_banner_test.dart`
Expected: FAIL — `update_banner.dart` does not exist.

- [ ] **Step 3: Implement the banner**

Create `apps/goresave/lib/features/app/ui/update_banner.dart`:

```dart
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:goresave/features/app/domain/update_notifier.dart';

/// Wraps the app content and shows a slim banner above it once an update has
/// been downloaded and staged.
class UpdateBannerHost extends ConsumerWidget {
  const UpdateBannerHost({required this.child, super.key});

  final Widget child;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final state = ref.watch(updateProvider);
    if (state is! UpdateReady) {
      return child;
    }
    final theme = Theme.of(context);
    final onContainer = theme.colorScheme.onPrimaryContainer;
    return Column(
      children: [
        Material(
          color: theme.colorScheme.primaryContainer,
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 4),
            child: Row(
              children: [
                Icon(Icons.system_update_alt, size: 16, color: onContainer),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(
                    'Update ${state.version} ready',
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: onContainer,
                    ),
                  ),
                ),
                TextButton(
                  onPressed: () =>
                      ref.read(updateProvider.notifier).applyAndRestart(),
                  child: const Text('Restart to update'),
                ),
              ],
            ),
          ),
        ),
        Expanded(child: child),
      ],
    );
  }
}
```

- [ ] **Step 4: Mount in GoresaveApp**

In `goresave_app.dart` add the import and wrap the builder child:

```dart
import 'package:goresave/features/app/ui/update_banner.dart';
```

```dart
      builder: (context, child) => UiScaleRoot(
        child: UpdateBannerHost(child: child ?? const SizedBox.shrink()),
      ),
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `flutter test test/update_banner_test.dart`, then `flutter test` (full)
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add apps/goresave/lib/features/app/ui/update_banner.dart apps/goresave/lib/features/app/ui/goresave_app.dart apps/goresave/test/update_banner_test.dart
git commit -m "feat(ui): update-ready banner with restart action"
```

---

### Task 7: Release workflow — version check + vpk pack

**Files:**
- Modify: `.github/workflows/release.yml` (full replacement below)
- Modify: `README.md`

- [ ] **Step 1: Replace release.yml**

```yaml
name: Release

on:
  push:
    tags: ["v*"]
  workflow_dispatch:

permissions:
  contents: write

jobs:
  windows:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4

      - name: Resolve and verify version
        id: version
        shell: pwsh
        run: |
          $pubspec = Get-Content apps/goresave/pubspec.yaml -Raw
          if ($pubspec -notmatch '(?m)^version:\s*([0-9]+\.[0-9]+\.[0-9]+)\s*$') {
            Write-Error 'pubspec.yaml version must be plain X.Y.Z (no build number)'
            exit 1
          }
          $version = $Matches[1]
          if ('${{ github.ref_type }}' -eq 'tag') {
            $tag = '${{ github.ref_name }}'
            if ($tag -ne "v$version") {
              Write-Error "tag $tag does not match pubspec version $version"
              exit 1
            }
          }
          "version=$version" >> $env:GITHUB_OUTPUT

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Cache cargo
        uses: Swatinem/rust-cache@v2

      - name: Install Flutter
        uses: subosito/flutter-action@v2
        with:
          flutter-version: 3.44.0
          channel: stable
          cache: true

      # GIT_SHA comes from the GITHUB_SHA env via build.py.
      - name: Build distribution
        run: python build.py dist

      - name: Install Velopack CLI
        run: dotnet tool install -g vpk

      # Pulls the previous release so vpk can build delta packages.
      # Fails harmlessly on the very first release.
      - name: Download previous Velopack release
        continue-on-error: true
        run: >-
          vpk download github
          --repoUrl https://github.com/${{ github.repository }}
          --outputDir dist/velopack

      - name: Velopack pack
        run: >-
          vpk pack
          --packId Goresave
          --packVersion ${{ steps.version.outputs.version }}
          --packDir apps/goresave/build/windows/x64/runner/Release
          --mainExe goresave.exe
          --packTitle goresave
          --packAuthors dh0er
          --icon apps/goresave/windows/runner/resources/app_icon.ico
          --outputDir dist/velopack

      - name: Upload build artifact
        uses: actions/upload-artifact@v4
        with:
          name: goresave-windows-x64
          path: |
            dist/*.zip
            dist/velopack/*

      # releases.win.json + packages must be attached so that
      # releases/latest/download/ serves the update feed.
      - name: Publish GitHub release
        if: startsWith(github.ref, 'refs/tags/')
        uses: softprops/action-gh-release@v2
        with:
          files: |
            dist/*.zip
            dist/velopack/*
          generate_release_notes: true
```

- [ ] **Step 2: Validate workflow syntax**

Run: `gh workflow list` is not enough; instead lint locally:
`python -c "import yaml,sys; yaml.safe_load(open('.github/workflows/release.yml', encoding='utf-8'))"`
Expected: exits 0.

- [ ] **Step 3: Document install/update in README**

In `README.md`, add after the Features section:

```markdown
## Installation & Updates

Download `Goresave-win-Setup.exe` from the
[latest release](https://github.com/dh0er/goresave/releases/latest) and run it.
The app checks GitHub Releases on startup, downloads updates in the background,
and applies them when you click "Restart to update".

A portable zip is also attached to each release; the portable build does not
auto-update.

### Releasing (maintainers)

1. Set `version:` in `apps/goresave/pubspec.yaml` to the new `X.Y.Z`.
2. Commit, then `git tag vX.Y.Z && git push origin vX.Y.Z`.
3. The Release workflow builds the zip + Velopack packages and publishes the
   GitHub release. The tag must match the pubspec version or the build fails.
```

- [ ] **Step 4: Commit**

```bash
git add .github/workflows/release.yml README.md
git commit -m "feat(ci): velopack packaging and update feed in release workflow"
```

---

### Task 8: Verification + manual E2E (after merge)

Automated, before merge:

- [ ] **Step 1: Full suite**

Run: `python test.py` (repo root)
Expected: cargo + flutter tests all pass.

- [ ] **Step 2: Local dist smoke test**

Run: `python build.py dist`
Expected: zip produced; launch `apps/goresave/build/windows/x64/runner/Release/goresave.exe` — app starts normally, no banner (updater disabled, not Velopack-installed), About dialog shows `Version 0.1.0 (<sha>)`.

Manual, after merge to main (requires repo public — `gh repo edit dh0er/goresave --visibility public`):

- [ ] **Step 3: Release v0.1.0** — tag and push; verify the workflow publishes Setup.exe, nupkg, `releases.win.json`, zip.
- [ ] **Step 4: Install** via Setup.exe; verify app launches and About shows version + sha.
- [ ] **Step 5: Release v0.1.1** (bump pubspec, tag); relaunch installed app; verify banner "Update 0.1.1 ready" appears, click "Restart to update", app restarts on 0.1.1.
- [ ] **Step 6: Portable check** — unzip the 0.1.1 zip, run it, verify no banner appears.
