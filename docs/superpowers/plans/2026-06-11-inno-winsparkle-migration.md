# Inno Setup + WinSparkle Migration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the Velopack-based Windows distribution with the taptell approach: an Inno Setup installer (wizard with free install-directory choice) plus WinSparkle auto-updates via the `auto_updater` Flutter plugin, with the appcast XML and installer hosted as GitHub release assets.

**Architecture:** The Rust core loses all updater commands; update checking moves entirely into the Flutter layer via the `auto_updater` plugin (dh0er fork, wraps WinSparkle 0.8.1 on Windows). WinSparkle polls `releases/latest/download/appcast-windows.xml` (stable URL on the latest GitHub release), verifies the DSA signature against a public key embedded as a Windows resource, downloads the versioned `GoresaveSetup-<ver>.exe`, and runs it — Inno Setup updates in place. CI builds the installer with `iscc` (preinstalled on `windows-latest`) and generates/signs the appcast with a Python script.

**Tech Stack:** Inno Setup 6, WinSparkle 0.8.1 (via `auto_updater` git dependency), OpenSSL DSA-SHA1 signing, GitHub Actions, GitHub Releases as feed host.

**Reference implementation:** taptell (`C:/sbx/taptell`) — `release_tools/windows.py` (Inno script generation), `release_tools/appcast.py` (appcast + DSA signing), `lib/features/app/domain/desktop_auto_updater_notifier.dart` (Dart integration), `windows/runner/Runner.rc:114` (DSA pubkey resource).

**Key facts discovered during research (do not re-derive):**

- `auto_updater` API: `autoUpdater.setFeedURL(url)` (calls `win_sparkle_set_appcast_url` + `win_sparkle_init`), `autoUpdater.setScheduledCheckInterval(seconds)` (min 3600), `autoUpdater.checkForUpdates(inBackground: true)` (= `win_sparkle_check_update_without_ui`, shows UI only when an update exists).
- WinSparkle reads the DSA public key from a Windows resource named `DSAPub` of type `DSAPEM` (see `winsparkle.h:178` in the plugin). Taptell embeds it via `Runner.rc`: `DSAPub DSAPEM "../../dsa_pub.pem"`.
- WinSparkle reads app name/version/company from the exe `VERSIONINFO`. goresave's [Runner.rc](apps/goresave/windows/runner/Runner.rc) already has `CompanyName`, `ProductName`, `ProductVersion` — nothing to change there besides the resource line.
- Appcast signing: DSA-SHA1 via `openssl dgst -sha1 -sign` with the private key from env `WINSPARKLE_DSA_PRIV_KEY_B64` (base64-encoded PEM). Unsigned appcasts are rejected once a pubkey resource is embedded.
- GitHub `releases/latest/download/<asset>` is a stable redirect to the newest release's asset — already used by the old Velopack feed, reuse it for `appcast-windows.xml`.
- Inno Setup 6 is preinstalled on `windows-latest` runners at `C:\Program Files (x86)\Inno Setup 6\iscc.exe` (not on PATH). OpenSSL and Python are on PATH.
- The working tree currently has TWO uncommitted edits from an abandoned Velopack-MSI attempt: `--msi` flag in `.github/workflows/release.yml` and an `## [Unreleased]` MSI bullet in `CHANGELOG.md`. Both are superseded by this plan and get overwritten in Tasks 7 and 8 — do not commit them as-is.
- **Migration caveat (document, don't fix):** existing v0.1.0 installs were installed via Velopack `Setup.exe` and poll `releases.win.json`. After the next release that asset no longer exists, so v0.1.0 users will NOT auto-update; they download the new installer once by hand. The stale Velopack install dir (`%LocalAppData%\Goresave`) is not cleaned up.

---

### Task 1: Swap the Dart updater to the auto_updater plugin

**Files:**
- Modify: `apps/goresave/pubspec.yaml`
- Delete: `apps/goresave/lib/features/app/domain/update_notifier.dart`
- Delete: `apps/goresave/lib/features/app/ui/update_banner.dart`
- Create: `apps/goresave/lib/features/app/domain/desktop_updater.dart`
- Modify: `apps/goresave/lib/features/app/ui/goresave_app.dart`
- Modify: `apps/goresave/lib/main.dart`

- [ ] **Step 1: Check for test references to the old updater**

Run: `grep -rn "update_notifier\|update_banner\|UpdateBanner\|updateProvider\|UpdateReady\|UpdateIdle" C:/sbx/goresave/apps/goresave/test C:/sbx/goresave/apps/goresave/integration_test 2>/dev/null`

Expected: no matches (verified during planning for `lib/`; if tests DO match, delete or rewrite those tests in this task — they test the Velopack flow that no longer exists).

- [ ] **Step 2: Add auto_updater dependencies to pubspec.yaml**

In `apps/goresave/pubspec.yaml`, add to `dependencies:` (after `window_manager`):

```yaml
  auto_updater:
    git:
      url: https://github.com/dh0er/auto_updater.git
      path: packages/auto_updater
      ref: swiftpm-support
```

And add a top-level `dependency_overrides:` section (after `dependencies:`):

```yaml
dependency_overrides:
  auto_updater_windows:
    git:
      url: https://github.com/dh0er/auto_updater.git
      ref: swiftpm-support
      path: packages/auto_updater_windows
  auto_updater_macos:
    git:
      url: https://github.com/dh0er/auto_updater.git
      ref: swiftpm-support
      path: packages/auto_updater_macos
  auto_updater_platform_interface:
    git:
      url: https://github.com/dh0er/auto_updater.git
      ref: swiftpm-support
      path: packages/auto_updater_platform_interface
```

(The overrides pin all federated sub-packages to the same fork/ref — same scheme as taptell's pubspec.)

- [ ] **Step 3: Run pub get**

Run: `flutter pub get` in `apps/goresave`
Expected: resolves, `pubspec.lock` updated, no errors.

- [ ] **Step 4: Delete the Velopack-flow Dart files**

```bash
git rm apps/goresave/lib/features/app/domain/update_notifier.dart apps/goresave/lib/features/app/ui/update_banner.dart
```

- [ ] **Step 5: Create desktop_updater.dart**

Create `apps/goresave/lib/features/app/domain/desktop_updater.dart`:

```dart
import 'dart:io';

import 'package:auto_updater/auto_updater.dart';
import 'package:flutter/foundation.dart';

/// Stable URL: releases/latest/download/ redirects to the newest GitHub
/// release's assets, where CI uploads the signed appcast.
const _appcastUrl =
    'https://github.com/dh0er/goresave/releases/latest/download/appcast-windows.xml';

const _checkIntervalSeconds = 3600;

/// Initializes WinSparkle-based auto-updates. Best-effort: failures are
/// logged and never block startup. No-op outside Windows release builds
/// (dev runs are not installed, so an update prompt would be wrong).
Future<void> initDesktopUpdater() async {
  if (!kReleaseMode || !Platform.isWindows) {
    return;
  }
  try {
    await autoUpdater.setFeedURL(_appcastUrl);
    await autoUpdater.setScheduledCheckInterval(_checkIntervalSeconds);
    // Silent check on startup: WinSparkle shows its own dialog only when
    // an update actually exists.
    await autoUpdater.checkForUpdates(inBackground: true);
  } catch (error) {
    debugPrint('goresave updater init failed: $error');
  }
}
```

- [ ] **Step 6: Unwrap UpdateBannerHost in goresave_app.dart**

In `apps/goresave/lib/features/app/ui/goresave_app.dart`:
- Remove the import line `import 'package:goresave/features/app/ui/update_banner.dart';`
- Replace `child: UpdateBannerHost(child: child ?? const SizedBox.shrink()),` with `child: child ?? const SizedBox.shrink(),`

(WinSparkle brings its own update dialog; the in-app banner is obsolete.)

- [ ] **Step 7: Wire initDesktopUpdater into main.dart**

In `apps/goresave/lib/main.dart`:
- Remove the stale comment lines `// Velopack startup hooks run in the native Windows runner (main.cpp)` and `// before the Flutter engine starts.`
- Add import: `import 'package:goresave/features/app/domain/desktop_updater.dart';`
- After `WidgetsFlutterBinding.ensureInitialized();` add:

```dart
  await initDesktopUpdater();
```

- [ ] **Step 8: Analyze and test**

Run: `flutter analyze` in `apps/goresave`
Expected: No issues found.

Run: `flutter test` in `apps/goresave`
Expected: all tests pass.

- [ ] **Step 9: Commit**

```bash
git add apps/goresave/pubspec.yaml apps/goresave/pubspec.lock apps/goresave/lib
git commit -m "feat(app): replace Velopack update flow with WinSparkle auto_updater"
```

---

### Task 2: Remove the Velopack updater from the Rust core

**Files:**
- Delete: `crates/goresave_core/src/updater.rs`
- Modify: `crates/goresave_core/src/lib.rs` (lines 3, 494-496, 5502-5507)
- Modify: `crates/goresave_core/Cargo.toml` (line 17)

- [ ] **Step 1: Delete updater.rs**

```bash
git rm crates/goresave_core/src/updater.rs
```

- [ ] **Step 2: Remove module, dispatch arms, and FFI export from lib.rs**

In `crates/goresave_core/src/lib.rs`:
- Remove line `mod updater;` (line 3)
- Remove the three dispatch arms (lines 494-496):

```rust
        "update_check" => updater::update_check(&payload),
        "update_download" => updater::update_download(&payload),
        "update_apply_restart" => updater::update_apply_restart(&payload),
```

- Remove the FFI export and its doc comment (around line 5502):

```rust
/// Velopack startup hook. Call before any other FFI function; may exit the
/// process when invoked as an install/update hook.
#[unsafe(no_mangle)]
pub extern "C" fn goresave_velopack_startup() {
    updater::velopack_startup();
}
```

- [ ] **Step 3: Remove the velopack dependency**

In `crates/goresave_core/Cargo.toml`, remove line 17: `velopack = "1.2.0"`

- [ ] **Step 4: Build and test**

Run: `cargo test -p goresave_core`
Expected: compiles without warnings about unused imports, all tests pass (the four updater tests were inside the deleted `updater.rs`, so nothing references the removed commands).

Also check nothing else references the module: `grep -rn "updater\|velopack" C:/sbx/goresave/crates --include=*.rs --include=*.toml | grep -v target`
Expected: no matches.

- [ ] **Step 5: Commit**

```bash
git add crates/goresave_core
git commit -m "refactor(core): remove Velopack updater commands"
```

---

### Task 3: Remove the Velopack startup hook from the Windows runner

**Files:**
- Modify: `apps/goresave/windows/runner/main.cpp`

- [ ] **Step 1: Remove the hook**

In `apps/goresave/windows/runner/main.cpp`:
- Delete the entire anonymous namespace block containing `RunVelopackStartupHooks()` (the comment block starting `// Runs Velopack install/update hooks...` through the closing `}  // namespace`). The `#include <string>` line was only needed for `std::wstring` in that function — remove it too.
- In `wWinMain`, delete the two lines:

```cpp
  // Must run before anything else: Velopack hooks may exit the process.
  RunVelopackStartupHooks();
```

- [ ] **Step 2: Verify the app still builds**

Run: `flutter build windows --debug` in `apps/goresave`
Expected: build succeeds. (Debug is enough to compile main.cpp; the full release build happens in CI via build.py.)

- [ ] **Step 3: Commit**

```bash
git add apps/goresave/windows/runner/main.cpp
git commit -m "refactor(windows): drop Velopack startup hooks from runner"
```

---

### Task 4: Generate DSA keys and embed the public key

**Files:**
- Create: `apps/goresave/dsa_pub.pem`
- Modify: `apps/goresave/windows/runner/Runner.rc`
- GitHub secret: `WINSPARKLE_DSA_PRIV_KEY_B64` (repo dh0er/goresave)

- [ ] **Step 1: Generate the DSA keypair OUTSIDE the repo**

```bash
mkdir -p ~/goresave-keys
openssl dsaparam -out ~/goresave-keys/dsa_param.pem 3072
openssl gendsa -out ~/goresave-keys/dsa_priv.pem ~/goresave-keys/dsa_param.pem
openssl dsa -in ~/goresave-keys/dsa_priv.pem -pubout -out C:/sbx/goresave/apps/goresave/dsa_pub.pem
```

Expected: `apps/goresave/dsa_pub.pem` exists and starts with `-----BEGIN PUBLIC KEY-----`. The private key lives only in `~/goresave-keys/` — NEVER add it to the repo.

- [ ] **Step 2: Upload the private key as a GitHub Actions secret**

```bash
base64 -w0 ~/goresave-keys/dsa_priv.pem | gh secret set WINSPARKLE_DSA_PRIV_KEY_B64 --repo dh0er/goresave
```

Expected: `✓ Set Actions secret WINSPARKLE_DSA_PRIV_KEY_B64 for dh0er/goresave`

- [ ] **Step 3: Tell the user to back up the private key**

The user must store `~/goresave-keys/dsa_priv.pem` somewhere durable (password manager). Losing it means future releases can't be verified by already-shipped builds. Surface this in the final summary — a lost key strands every installed copy, same class of problem as the v0.1.0 migration note.

- [ ] **Step 4: Embed the public key as a Windows resource**

In `apps/goresave/windows/runner/Runner.rc`, add directly under the `// Flutter icon` / `IDI_APP_ICON ... "resources\\app_icon.ico"` block (mirroring taptell's `Runner.rc:114`):

```
/////////////////////////////////////////////////////////////////////////////
//
// WinSparkle DSA public key (resource name/type fixed by WinSparkle)
//

DSAPub                  DSAPEM                  "../../dsa_pub.pem"
```

(Path is relative to `windows/runner/`, so `../../dsa_pub.pem` resolves to `apps/goresave/dsa_pub.pem`.)

- [ ] **Step 5: Verify the resource compiles**

Run: `flutter build windows --debug` in `apps/goresave`
Expected: build succeeds (rc.exe fails the build loudly if the resource path is wrong).

- [ ] **Step 6: Commit**

```bash
git add apps/goresave/dsa_pub.pem apps/goresave/windows/runner/Runner.rc
git commit -m "feat(windows): embed WinSparkle DSA public key"
```

---

### Task 5: Add the Inno Setup script

**Files:**
- Create: `installer/setup.iss`

- [ ] **Step 1: Create installer/setup.iss**

```ini
; goresave Windows installer.
; Compiled by CI:
;   iscc /DAppVersion=<x.y.z> /DSourceDir=<abs path to Release dir> ^
;        /DOutputDir=<abs path for the exe> installer\setup.iss
; The wizard shows a directory page, so users pick any install location.
; WinSparkle updates download this same installer and re-run it; Inno
; remembers the previous location and updates in place.

#ifndef AppVersion
  #error "Pass /DAppVersion=x.y.z"
#endif
#ifndef SourceDir
  #error "Pass /DSourceDir=<path to flutter Release dir>"
#endif
#ifndef OutputDir
  #error "Pass /DOutputDir=<path for installer exe>"
#endif

[Setup]
; Fixed GUID identifies the app across versions for in-place updates.
AppId={{C7A35D8E-4B61-4E0D-9C0A-2F8B5D1E6A43}
AppName=goresave
AppVersion={#AppVersion}
AppVerName=goresave {#AppVersion}
AppPublisher=dh0er
DefaultDirName={autopf}\goresave
DefaultGroupName=goresave
; Per-user installs work without elevation; the dialog lets the user pick
; all-users (Program Files, admin) or current-user (no UAC prompt).
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
OutputDir={#OutputDir}
OutputBaseFilename=GoresaveSetup-{#AppVersion}
Compression=lzma
SolidCompression=yes
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
UninstallDisplayIcon={app}\goresave.exe
WizardStyle=modern
SetupIconFile=..\apps\goresave\windows\runner\resources\app_icon.ico
LicenseFile=..\LICENSE

[Files]
Source: "{#SourceDir}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{group}\goresave"; Filename: "{app}\goresave.exe"
Name: "{autodesktop}\goresave"; Filename: "{app}\goresave.exe"; Tasks: desktopicon

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; GroupDescription: "Additional icons:"

[Run]
Filename: "{app}\goresave.exe"; Description: "Launch goresave"; Flags: nowait postinstall skipifsilent
```

Note: `SetupIconFile`/`LicenseFile` are resolved relative to the .iss file (`installer/`), hence the `..\` prefixes. Verify `LICENSE` exists at the repo root (`ls C:/sbx/goresave/LICENSE`); if the file has a different name (e.g. `LICENSE.md`), adjust the `LicenseFile` line.

- [ ] **Step 2: Validate locally if iscc is installed, otherwise rely on CI**

Run: `& "C:\Program Files (x86)\Inno Setup 6\iscc.exe" /? 2>$null`
If iscc exists locally AND a Release build is present (`apps/goresave/build/windows/x64/runner/Release/goresave.exe`), do a trial compile:

```powershell
& "C:\Program Files (x86)\Inno Setup 6\iscc.exe" /Qp "/DAppVersion=0.0.0" "/DSourceDir=C:\sbx\goresave\apps\goresave\build\windows\x64\runner\Release" "/DOutputDir=C:\sbx\goresave\dist" installer\setup.iss
```

Expected: `dist/GoresaveSetup-0.0.0.exe` produced. If iscc is not installed locally, skip — Task 7's workflow is the verification point.

- [ ] **Step 3: Commit**

```bash
git add installer/setup.iss
git commit -m "feat(installer): add Inno Setup script with directory choice"
```

---

### Task 6: Add the appcast generator script

**Files:**
- Create: `scripts/appcast.py`
- Test: manual invocation with a throwaway key (steps below)

- [ ] **Step 1: Create scripts/appcast.py**

Adapted from taptell's `release_tools/appcast.py`, trimmed to Windows-only and GitHub-Releases-hosted:

```python
"""Generate and DSA-sign the WinSparkle appcast for a goresave release.

The appcast is uploaded as a GitHub release asset; the app polls
releases/latest/download/appcast-windows.xml. The enclosure URL points at
the versioned installer asset of the same release.

Usage:
    python scripts/appcast.py --version 0.1.1 \
        --installer dist/GoresaveSetup-0.1.1.exe \
        --notes dist/RELEASE_NOTES.md \
        --output dist/appcast-windows.xml

Environment:
    WINSPARKLE_DSA_PRIV_KEY_B64   base64-encoded DSA private key PEM.
                                  Required: WinSparkle rejects unsigned
                                  updates once a public key is embedded.
"""

from __future__ import annotations

import argparse
import base64
import os
import subprocess
import sys
import tempfile
import xml.etree.ElementTree as ET
from datetime import datetime, timezone
from email.utils import format_datetime
from pathlib import Path

REPO_DOWNLOAD_BASE = "https://github.com/dh0er/goresave/releases/download"


def sign_dsa(installer: Path) -> str:
    """Return the base64 DSA-SHA1 signature of *installer*."""
    key_b64 = os.environ.get("WINSPARKLE_DSA_PRIV_KEY_B64")
    if not key_b64:
        sys.exit("WINSPARKLE_DSA_PRIV_KEY_B64 is not set; refusing to "
                 "produce an unsigned appcast the app would reject")
    key_pem = base64.b64decode(key_b64).decode()
    with tempfile.TemporaryDirectory() as tmp:
        key_path = Path(tmp) / "dsa_priv.pem"
        key_path.write_text(key_pem, encoding="utf-8")
        result = subprocess.run(
            ["openssl", "dgst", "-sha1", "-sign", str(key_path),
             str(installer)],
            capture_output=True,
            check=True,
        )
    return base64.b64encode(result.stdout).decode()


def notes_to_html(notes_path: Path | None) -> str:
    if notes_path is None or not notes_path.exists():
        return ""
    lines = notes_path.read_text(encoding="utf-8").strip().split("\n")
    return "<br/>".join(line.strip() for line in lines if line.strip())


def to_cdata(text: str) -> str:
    return "<![CDATA[" + text.replace("]]>", "]]]]><![CDATA[>") + "]]>"


def build_appcast(*, version: str, installer: Path, notes_html: str,
                  signature: str) -> str:
    rss = ET.Element("rss", {
        "version": "2.0",
        "xmlns:sparkle": "http://www.andymatuschak.org/xml-namespaces/sparkle",
        "xmlns:dc": "http://purl.org/dc/elements/1.1/",
    })
    channel = ET.SubElement(rss, "channel")
    ET.SubElement(channel, "title").text = "goresave"

    item = ET.SubElement(channel, "item")
    ET.SubElement(item, "title").text = f"Version {version}"
    ET.SubElement(item, "sparkle:version").text = version
    ET.SubElement(item, "sparkle:shortVersionString").text = version
    ET.SubElement(item, "pubDate").text = format_datetime(
        datetime.now(timezone.utc))

    placeholder = None
    if notes_html:
        placeholder = "__APPCAST_DESCRIPTION_HTML__"
        ET.SubElement(item, "description").text = placeholder

    ET.SubElement(item, "enclosure", {
        "url": f"{REPO_DOWNLOAD_BASE}/v{version}/{installer.name}",
        "length": str(installer.stat().st_size),
        "type": "application/octet-stream",
        "sparkle:dsaSignature": signature,
    })

    ET.indent(rss, space="  ")
    xml = ET.tostring(rss, encoding="unicode", xml_declaration=True)
    if placeholder:
        xml = xml.replace(
            f"<description>{placeholder}</description>",
            f"<description>{to_cdata(notes_html)}</description>",
            1,
        )
    return xml + "\n"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--version", required=True)
    parser.add_argument("--installer", required=True, type=Path)
    parser.add_argument("--notes", type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()

    if not args.installer.exists():
        sys.exit(f"installer not found: {args.installer}")

    signature = sign_dsa(args.installer)
    xml = build_appcast(
        version=args.version,
        installer=args.installer,
        notes_html=notes_to_html(args.notes),
        signature=signature,
    )
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(xml, encoding="utf-8")
    print(f"appcast written: {args.output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
```

- [ ] **Step 2: Test the script end-to-end with a throwaway key**

```bash
cd /c/sbx/goresave
openssl dsaparam -out /tmp/test_param.pem 1024
openssl gendsa -out /tmp/test_priv.pem /tmp/test_param.pem
echo "dummy installer bytes" > /tmp/GoresaveSetup-9.9.9.exe
printf "Test note line 1\nTest note line 2\n" > /tmp/notes.md
WINSPARKLE_DSA_PRIV_KEY_B64=$(base64 -w0 /tmp/test_priv.pem) \
  python scripts/appcast.py --version 9.9.9 \
    --installer /tmp/GoresaveSetup-9.9.9.exe \
    --notes /tmp/notes.md --output /tmp/appcast-windows.xml
cat /tmp/appcast-windows.xml
```

Expected: XML with `<enclosure url="https://github.com/dh0er/goresave/releases/download/v9.9.9/GoresaveSetup-9.9.9.exe" ... sparkle:dsaSignature="..."/>`, `<sparkle:version>9.9.9</sparkle:version>`, and a CDATA description with `<br/>` between the two note lines.

Then verify the signature round-trips:

```bash
openssl dsa -in /tmp/test_priv.pem -pubout -out /tmp/test_pub.pem
python - <<'EOF'
import re, base64, subprocess
xml = open('/tmp/appcast-windows.xml', encoding='utf-8').read()
sig = base64.b64decode(re.search(r'dsaSignature="([^"]+)"', xml).group(1))
open('/tmp/sig.bin','wb').write(sig)
r = subprocess.run(['openssl','dgst','-sha1','-verify','/tmp/test_pub.pem',
                    '-signature','/tmp/sig.bin','/tmp/GoresaveSetup-9.9.9.exe'])
raise SystemExit(r.returncode)
EOF
```

Expected: `Verified OK`, exit 0.

- [ ] **Step 3: Also run the no-key failure path**

```bash
env -u WINSPARKLE_DSA_PRIV_KEY_B64 python scripts/appcast.py --version 9.9.9 \
  --installer /tmp/GoresaveSetup-9.9.9.exe --output /tmp/x.xml; echo "exit=$?"
```

Expected: error message about `WINSPARKLE_DSA_PRIV_KEY_B64`, exit=1 (hard failure so CI cannot silently ship an unsigned appcast).

- [ ] **Step 4: Commit**

```bash
git add scripts/appcast.py
git commit -m "feat(release): add WinSparkle appcast generator with DSA signing"
```

---

### Task 7: Rewrite the release workflow

**Files:**
- Modify: `.github/workflows/release.yml`

The working tree already contains an uncommitted `--msi` edit in this file — this task replaces that whole section, superseding it.

- [ ] **Step 1: Replace the Velopack steps**

In `.github/workflows/release.yml`, delete these steps entirely: `Install Velopack CLI`, `Download previous Velopack release`, `Velopack pack` (including the `--msi` line and the comment blocks above them, lines 53-78 of the current file).

Move the `Extract release notes from CHANGELOG.md` step BEFORE the new packaging steps and drop its `if: startsWith(github.ref, 'refs/tags/')` condition, but make the missing-section check non-fatal on non-tag runs: replace its two `Write-Error ... exit 1` blocks so they only fail for tags. Full replacement step:

```yaml
      # Release notes come from CHANGELOG.md only; a missing or empty
      # section for the released version fails a tag build. Non-tag runs
      # (workflow_dispatch smoke builds) fall back to a placeholder so the
      # appcast step still has notes input.
      - name: Extract release notes from CHANGELOG.md
        shell: pwsh
        run: |
          $version = '${{ steps.version.outputs.version }}'
          $isTag = '${{ github.ref_type }}' -eq 'tag'
          $changelog = Get-Content CHANGELOG.md -Raw
          $pattern = "(?ms)^## \[$([regex]::Escape($version))\][^`r`n]*`r?`n(.*?)(?=^## \[|\z)"
          $notes = ''
          if ($changelog -match $pattern) { $notes = $Matches[1].Trim() }
          if (-not $notes) {
            if ($isTag) {
              Write-Error "CHANGELOG.md has no non-empty section for $version"
              exit 1
            }
            $notes = "Development build $version"
          }
          New-Item -ItemType Directory -Force dist | Out-Null
          [IO.File]::WriteAllText((Join-Path $PWD 'dist\RELEASE_NOTES.md'), $notes)
```

In place of the deleted Velopack steps, insert:

```yaml
      # Inno Setup 6 is preinstalled on windows-latest but not on PATH.
      - name: Build installer (Inno Setup)
        shell: pwsh
        run: |
          & "C:\Program Files (x86)\Inno Setup 6\iscc.exe" /Qp `
            "/DAppVersion=${{ steps.version.outputs.version }}" `
            "/DSourceDir=$PWD\apps\goresave\build\windows\x64\runner\Release" `
            "/DOutputDir=$PWD\dist" `
            installer\setup.iss
          if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

      # Signs the installer and writes the appcast the app polls via
      # releases/latest/download/appcast-windows.xml.
      - name: Generate signed appcast
        env:
          WINSPARKLE_DSA_PRIV_KEY_B64: ${{ secrets.WINSPARKLE_DSA_PRIV_KEY_B64 }}
        run: >-
          python scripts/appcast.py
          --version ${{ steps.version.outputs.version }}
          --installer dist/GoresaveSetup-${{ steps.version.outputs.version }}.exe
          --notes dist/RELEASE_NOTES.md
          --output dist/appcast-windows.xml
```

- [ ] **Step 2: Update the artifact and release file lists**

`Upload build artifact` step — replace the `path:` block with:

```yaml
          path: |
            dist/*.zip
            dist/*.exe
            dist/appcast-windows.xml
```

`Publish GitHub release` step — replace the `files:` block with:

```yaml
          files: |
            dist/*.zip
            dist/*.exe
            dist/appcast-windows.xml
```

(The `dist/velopack/*` patterns disappear with Velopack; `releases.win.json` is no longer published — see migration caveat in the header.)

- [ ] **Step 3: Validate YAML**

Run: `python -c "import yaml; yaml.safe_load(open('.github/workflows/release.yml', encoding='utf-8')); print('yaml ok')"`
Expected: `yaml ok`

- [ ] **Step 4: Commit**

```bash
git add .github/workflows/release.yml
git commit -m "feat(release): build Inno Setup installer and signed appcast in CI"
```

---

### Task 8: Update CHANGELOG and README, clean up

**Files:**
- Modify: `CHANGELOG.md` (replace the uncommitted MSI bullet)
- Modify: `README.md` (if it documents the installer/update flow)

- [ ] **Step 1: Replace the Unreleased section in CHANGELOG.md**

Replace the existing uncommitted `## [Unreleased]` block (the MSI bullet) with:

```markdown
## [Unreleased]

### Changed

- New Windows installer (Inno Setup): the setup wizard now lets you choose
  the install directory, including per-user installs without admin rights.
- Auto-updates now use WinSparkle with signed update feeds. **Users of
  v0.1.0 must download and run the new installer manually once** — the old
  updater's feed is no longer published.
```

- [ ] **Step 2: Check README for stale installer/update docs**

Run: `grep -n -i "velopack\|setup.exe\|install" C:/sbx/goresave/README.md`
If the README describes the old Setup.exe one-click install or Velopack updates, rewrite those sentences to describe the Inno installer (directory choice, per-user or all-users) and WinSparkle update prompts. If no matches, skip.

- [ ] **Step 3: Remove the temporary auto_updater clone**

```bash
rm -rf /c/sbx/_tmp_auto_updater
```

- [ ] **Step 4: Commit**

```bash
git add CHANGELOG.md README.md
git commit -m "docs: document Inno Setup installer and WinSparkle update migration"
```

---

### Task 9: End-to-end verification

- [ ] **Step 1: Full local build**

Run: `python build.py dist` at the repo root.
Expected: exits 0, `dist/goresave-0.1.0-windows-x64.zip` produced, Release folder contains `goresave.exe` + `goresave_core.dll`.

- [ ] **Step 2: Confirm the exe carries the DSA resource**

```powershell
$bytes = [IO.File]::ReadAllBytes('apps\goresave\build\windows\x64\runner\Release\goresave.exe')
$text = [Text.Encoding]::ASCII.GetString($bytes)
if ($text.Contains('BEGIN PUBLIC KEY')) { 'DSA resource embedded' } else { Write-Error 'DSA resource missing' }
```

Expected: `DSA resource embedded`

- [ ] **Step 3: Repo-wide leftover scan**

Run: `grep -rn -i "velopack" C:/sbx/goresave --include=*.rs --include=*.dart --include=*.cpp --include=*.toml --include=*.yml --include=*.yaml --include=*.lock | grep -v target | grep -v docs/superpowers`
Expected: no matches (historical plan/spec docs are allowed to keep their mentions).

Run: `cargo test -p goresave_core` and `flutter analyze` + `flutter test` in `apps/goresave` once more.
Expected: all green.

- [ ] **Step 4: CI smoke run (optional but recommended)**

Trigger `workflow_dispatch` on the release workflow: `gh workflow run release.yml --repo dh0er/goresave`. Non-tag runs build the installer and appcast and upload them as a build artifact without publishing a release.
Expected: green run; artifact contains `GoresaveSetup-0.1.0.exe` and `appcast-windows.xml`.
