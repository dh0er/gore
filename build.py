#!/usr/bin/env python3
"""gore monorepo build orchestrator.

One entry point for every releasable product in the flat layout (Flutter apps
under apps/, the gore CLI under crates/gore). Wraps the per-project build logic
(Flutter apps, Rust crates) and adds debug builds, tests, and a flag-driven
release pipeline.

Usage:
    python build.py <project|all> build      [--debug|--release] [--run]
    python build.py <project>     run        [--debug|--release]  # build if missing, launch
    python build.py <project|all> dist                  # build + package
    python build.py <project|all> installer             # dist + setup.exe
    python build.py <project|all> test
    python build.py <project>     release X.Y.Z <steps> [--dry-run]

Release steps (additive; pass at least one, or --all):
    --bump        write X.Y.Z into the project manifest (pubspec/Cargo.toml)
    --changelog   require a non-empty CHANGELOG.md section for X.Y.Z
    --build       compile locally (release mode)
    --pack        package locally (zip / bundle)
    --installer   build the Windows installer locally
    --tag         create git tag <prefix>-vX.Y.Z
    --push        push the tag (this is what triggers the CI release)
    --all         = --bump --changelog --tag --push (CI builds remotely)
    --dry-run     print every action, change nothing

Project names are the whole naming scheme. A project is its tag prefix and its
artifact name: gore-cli, gore-save-editor, gore-mod-studio, gore-mod-manager
produce tags <project>-vX.Y.Z, zips <project>-X.Y.Z-windows-x64.zip and
installers <project>-X.Y.Z-setup.exe. The Release workflow matches the tag
prefix and builds only that project.

Examples:
    python build.py all test
    python build.py gore-save-editor dist
    python build.py gore-cli release 0.2.0 --all
    python build.py gore-mod-studio release 0.1.0 --bump --build --installer

Code signing (Azure Trusted / Artifact Signing):
    Off by default -- local dist/installer builds ship unsigned. Signing is
    opt-in via GORE_SIGN=1 (CI sets it), which also requires these env vars:
        TRUSTED_SIGNING_ENDPOINT  TRUSTED_SIGNING_ACCOUNT  TRUSTED_SIGNING_PROFILE
        AZURE_TENANT_ID  AZURE_CLIENT_ID  AZURE_CLIENT_SECRET   (service principal)
    With GORE_SIGN=1 a missing var hard-fails rather than silently shipping
    unsigned. GORE_SIGN_NO_PROXY=1 makes only the signing subprocess tree
    (direct signtool, or ISCC and its signtool child) bypass the system proxy,
    for machines whose PAC routes AAD login through a tunnel that may be down.
"""

from __future__ import annotations

import argparse
from contextlib import ExitStack, contextmanager
from dataclasses import dataclass
import hashlib
import os
import posixpath
import re
import shutil
import stat
import struct
import subprocess
import sys
import tempfile
from pathlib import Path
import zipfile

from scripts import standalone_compiler_bundle

ROOT = Path(__file__).parent


def _resolve_tool(env_var: str, names: list[str], fallback: Path) -> Path:
    override = os.environ.get(env_var)
    if override:
        return Path(override)
    for name in names:
        found = shutil.which(name)
        if found:
            return Path(found)
    return fallback


FLUTTER = _resolve_tool(
    "FLUTTER",
    ["flutter.bat", "flutter"],
    Path.home() / "fvm" / "versions" / "3.44.0" / "bin" / "flutter.bat",
)
CARGO = _resolve_tool(
    "CARGO", ["cargo.exe", "cargo"], Path.home() / ".cargo" / "bin" / "cargo.exe"
)
ISCC = _resolve_tool(
    "ISCC",
    ["iscc", "iscc.exe"],
    Path(r"C:\Program Files (x86)\Inno Setup 6\iscc.exe"),
)


# --------------------------------------------------------------------------- #
# Project registry                                                            #
# --------------------------------------------------------------------------- #
# Each project declares its kind plus the paths the recipes need. Only the
# shippable products live here; internal libraries (gore-reflect,
# gore-save, gore-ffi, gore-as, ...) are plain workspace crates with no release
# entry. A project may carry releasable=False so `all` skips it for
# release/dist and `release <project>` rejects it.
PROJECTS: dict[str, dict] = {
    "gore-save-editor": {  # save editor (Flutter, WinSparkle)
        "kind": "flutter",
        "dir": "apps/save-editor",
        "pubspec": "pubspec.yaml",
        "tag_prefix": "gore-save-editor",
        "changelog": "CHANGELOG.md",
        "installer": "installer/setup.iss",
        "exe": "goresave.exe",  # keep: CMake BINARY_NAME (Inno AppId-tied upgrade)
        "core_crate": "gore-save",  # cargo -p selector (cargo wants the hyphenated package id)
        "core_dll": "gore_save",  # was goresave_core; dll now gore_save.dll (cargo underscores it)
        "releasable": True,
    },
    "gore-mod-studio": {  # mod studio (Flutter, WinSparkle)
        "kind": "flutter",
        "dir": "apps/mod-studio",
        "pubspec": "pubspec.yaml",
        "tag_prefix": "gore-mod-studio",
        "changelog": "CHANGELOG.md",
        "installer": "installer/setup.iss",
        "exe": "gore_mod.exe",  # CMake BINARY_NAME
        "core_crate": "gore-ffi",  # cargo -p selector (cargo wants the hyphenated package id)
        "core_dll": "gore_ffi",  # was gore_core; dll now gore_ffi.dll (cargo underscores it)
        # Bundle the standalone `gore` CLI (gore.exe + its lua/shared SDK) beside
        # the app, so GUI users get the power tools Studio does not expose (gore
        # as disasm/decompile, catalog/dump/stubs, mgr) without a second download.
        # Staged into the Flutter Release dir, so both the installer
        # (SourceDir=Release) and the portable zip (copied from Release) ship it.
        "companions": ["gore-cli"],
        "standalone_compiler_bundle": True,
        "releasable": True,
    },
    "gore-mod-manager": {  # mod manager (Flutter, WinSparkle)
        "kind": "flutter",
        "dir": "apps/mod-manager",
        "pubspec": "pubspec.yaml",
        "tag_prefix": "gore-mod-manager",
        "changelog": "CHANGELOG.md",
        "installer": "installer/setup.iss",
        # Let Inno sign both Setup and its embedded uninstaller in signed builds.
        # This globally unique name is deliberately not a generic `byparam`
        # alias: Inno's /S command can otherwise be reused by injected script
        # content with attacker-controlled parameters.
        "inno_sign_tool": "gore_mod_manager_ats_b7e4d2c95a184f6b",
        "exe": "gore_manager.exe",  # CMake BINARY_NAME
        "core_crate": "gore-ffi",  # shares the mod-studio FFI crate
        "core_dll": "gore_ffi",  # dll gore_ffi.dll (cargo underscores it)
        # The runner, plugins, and native core use MSVC's dynamic release CRT.
        # Keep this Manager-only until the other products receive their own
        # release qualification. Packaging resolves these files from the exact
        # Visual Studio instance that Flutter/CMake selected.
        "app_local_msvc_runtime": (
            "msvcp140.dll",
            "vcruntime140.dll",
            "vcruntime140_1.dll",
        ),
        "releasable": True,
    },
    "gore-cli": {  # the unified CLI
        "kind": "rust-bin",
        "dir": "crates/gore",
        "manifest": "Cargo.toml",
        "crate": "gore",
        "bin": "gore",  # produces gore.exe
        "tag_prefix": "gore-cli",
        "changelog": "CHANGELOG.md",
        "releasable": True,
        "standalone_compiler_bundle": True,
        # extra dirs staged beside the exe in the release zip: (src relative to ROOT, dest name).
        # `gore deploy-shared` resolves the SDK from `shared/` next to the binary.
        "bundle_dirs": [("lua/shared", "shared")],
        # Markdown docs staged beside the exe. Links that point out of the guide
        # tree are rewritten to absolute GitHub URLs (see stage_docs).
        "doc_dirs": [("docs/guide", "docs")],
        # The same guide, rendered by the freshly built binary into one browsable,
        # self-contained HTML file. The Markdown copies are what `grep` wants (the
        # MCP server has its own, compiled into the exe); this is what a human
        # double-clicks, because Windows has no handler for .md and the guide is
        # far too table-heavy for Notepad.
        "guide_html": "docs/guide.html",
    },
}

RELEASE_ORDER = [  # for `all`
    "gore-cli",
    "gore-save-editor",
    "gore-mod-studio",
    "gore-mod-manager",
]


# Every shipped artifact is named after its project, which is also its tag
# prefix: `<project>-vX.Y.Z` tags produce `<project>-X.Y.Z-windows-x64.zip` and
# `<project>-X.Y.Z-setup.exe`. Keep these three in lockstep.
def zip_basename(project: str, version: str) -> str:
    return f"{PROJECTS[project]['tag_prefix']}-{version}-windows-x64"


def installer_basename(project: str, version: str) -> str:
    return f"{PROJECTS[project]['tag_prefix']}-{version}-setup"


def env() -> dict[str, str]:
    out = dict(os.environ)
    out["PATH"] = os.pathsep.join(
        [str(FLUTTER.parent), str(CARGO.parent), out.get("PATH", "")]
    )
    return out


def run(
    label: str,
    cmd: list[object],
    cwd: Path = ROOT,
    dry: bool = False,
    extra_env: dict[str, str] | None = None,
) -> None:
    printable = " ".join(str(part) for part in cmd)
    print(f"\n{'=' * 72}\n{label}\n-> {printable}  (cwd={cwd})\n{'=' * 72}")
    if dry:
        print("[dry-run] skipped")
        return
    child_env = env()
    if extra_env:
        child_env.update(extra_env)
    completed = subprocess.run([str(part) for part in cmd], cwd=cwd, env=child_env)
    if completed.returncode != 0:
        raise SystemExit(f"{label} failed (exit {completed.returncode})")


# --------------------------------------------------------------------------- #
# Code signing (Azure Trusted / Artifact Signing)                             #
# --------------------------------------------------------------------------- #
# Ship Authenticode-signed PE files so AV ML engines (SecureAge et al.) stop
# false-flagging the unsigned Flutter runner — the flag NexusMods quarantines
# our portable zip on.
#
# Signing is OPT-IN: off by default (local builds ship unsigned), on only when
# GORE_SIGN=1 — CI sets it. When opted in, these env vars must all be present
# (a missing one hard-fails rather than silently shipping unsigned):
#
#   TRUSTED_SIGNING_ENDPOINT   Account URI, e.g. https://weu.codesigning.azure.net/
#   TRUSTED_SIGNING_ACCOUNT    Artifact Signing account name
#   TRUSTED_SIGNING_PROFILE    certificate profile name
#   AZURE_TENANT_ID            \
#   AZURE_CLIENT_ID             > service-principal credential (the dlib reads
#   AZURE_CLIENT_SECRET        /  these via Azure.Identity EnvironmentCredential)
#
# Nothing secret lives in the repo; CI injects the above from repo secrets.
TS_DLIB_VERSION = "1.0.95"
TS_DLIB_DIR = ROOT / "tools" / "trusted-signing"
TS_DLIB_PACKAGE_URL = (
    "https://api.nuget.org/v3-flatcontainer/microsoft.trusted.signing.client/"
    f"{TS_DLIB_VERSION}/microsoft.trusted.signing.client.{TS_DLIB_VERSION}.nupkg"
)
# Measured 2026-08-24 from the exact package above on the official NuGet v3
# flat-container endpoint. The package is checked before it is opened as a ZIP,
# and every x64 runtime file is checked again before signtool may load the dlib.
TS_DLIB_PACKAGE_BYTES = 14_651_525
TS_DLIB_PACKAGE_SHA256 = (
    "3bfcf1e0a3cb42af1692f0a8ed45c15de070c2de86f28a59b2795d904d8a920f"
)
TS_DLIB_RUNTIME_FILES: dict[str, tuple[int, str]] = {
    "Azure.CodeSigning.Dlib.Core.dll": (
        34_848,
        "c186269dc83ab4be53d4453c5033a8363e4dc300473d54fdcea2e852c522aa4b",
    ),
    "Azure.CodeSigning.Dlib.dll": (
        121_400,
        "a359b420f676bc0223a379a84ca8369588ae7f265fd4f3e761e3425cba376916",
    ),
    "Azure.CodeSigning.Dlib.runtimeconfig.json": (
        416,
        "c5c339dd6be63f3030b74251503f874cbf56ca1bce24280341aa3331a0345dee",
    ),
    "Azure.CodeSigning.dll": (
        126_520,
        "d202f1dac409ba3c76d8780269d2f3e858f2c4f26cb0b03a04f1625cfc47eab1",
    ),
    "Azure.Core.dll": (
        428_088,
        "d17327c51909936e91bbc9fb8f42bf0291a5b01cfba74e8a6304186aa90d95ff",
    ),
    "Azure.Identity.dll": (
        355_912,
        "abf121ecd9236e6b689d312446e97dc84ef4babc41d3bf846e4099ee5868b272",
    ),
    "concrt140.dll": (
        312_376,
        "c0d3caea36c51af58789ea31d7331e5ac78892f85a003c211191e822cd14baf3",
    ),
    "Ijwhost.dll": (
        129_608,
        "bf9b1c4c601fa355a53b4dccfaff99e1697482c3968e3fb3b3d3be9da36dd41e",
    ),
    "mfc140.dll": (
        5_612_616,
        "0acb52f8ca94c6b0cc183ba38c5ea23d98ef7dce1f821ec9af240ee97fc03bcc",
    ),
    "mfc140chs.dll": (
        41_016,
        "567d7d7dd70714913be99f6d20bf2ac20258ba758cfbf91bee8e141a122f95a5",
    ),
    "mfc140cht.dll": (
        40_992,
        "bd56fc0dbd135f451e9dd3de652c6955d67844d60766e862ac4c7f34c1bec3cf",
    ),
    "mfc140deu.dll": (
        69_704,
        "f201ae3a63803cafade53bfdd636efba4da23cb7d1d5ebdcbf2cce18d93b21af",
    ),
    "mfc140enu.dll": (
        59_960,
        "11dba7fc477e1f89694e5625d884e0be1ff669f3fdcc06aa6af5f0d90d15b02e",
    ),
    "mfc140esn.dll": (
        68_680,
        "0dbbae157283aafdc741f4a3f33a747a9dbd5d94cac043c990d8babe66d3b468",
    ),
    "mfc140fra.dll": (
        69_688,
        "1e02482ccb116ed0144921bca425b90ab75dc7a37d50657c9d293985786660c9",
    ),
    "mfc140ita.dll": (
        67_656,
        "1c9851d44e098ee12126e72cf91f381409ff5b407651bc53a9f759fccd926df1",
    ),
    "mfc140jpn.dll": (
        48_672,
        "a5c246439c2483e8350a186024049aaca6e4ce0d2beffb3aab2fb4cf5b463917",
    ),
    "mfc140kor.dll": (
        48_200,
        "cca89545803e84c68fc434098ea7c0fde8433fb89282d5f53915c6bac88504bd",
    ),
    "mfc140rus.dll": (
        65_608,
        "26ea1f9163079b9f77223a865352b63a60957c4fc397bcb96cee1285e0cf035a",
    ),
    "mfc140u.dll": (
        5_647_904,
        "216a282ce4a8d6700bce8a6f24902614dab1848eced38426077c23dbf6b299e9",
    ),
    "mfcm140.dll": (
        86_048,
        "96f701928126ec679098b6d802992e9b322766cddc69ed04cd9b0de9d3085a1b",
    ),
    "mfcm140u.dll": (
        86_088,
        "a9f034b3f48760aa92b7da8ed31d9a6ac7bd4e8f1ca9ca3623a92d6151c12e0c",
    ),
    "Microsoft.Extensions.DependencyInjection.Abstractions.dll": (
        63_544,
        "33e3bc263badfc8060dacb91dd84ce3ba482537ca93fc09ae3c10b1465984307",
    ),
    "Microsoft.Extensions.Logging.Abstractions.dll": (
        65_080,
        "814bc115e46d86608c182b6730bafa7ac85cdf701117310e528e47d04ef80d5d",
    ),
    "Microsoft.Identity.Client.dll": (
        1_013_304,
        "ab7a6aa4df70caeed8411ccc3a67d16ed7ca676bf65a5965ce8a564c33631c3d",
    ),
    "Microsoft.Identity.Client.Extensions.Msal.dll": (
        66_592,
        "e95844ccca5dc36a4ea7e8caf5022bcfd2577a1d238d7c03e95062b90f91e326",
    ),
    "Microsoft.IdentityModel.Abstractions.dll": (
        19_000,
        "dadb93b1c46ff805da565ade87db63b6b9cf9ef6b26a499a2debd045e75d1d05",
    ),
    "msvcp140.dll": (
        565_280,
        "020a4a42e57a798c1f3d8fc5d3e0c34a579392a78f2a17d19f1cf6834db22c36",
    ),
    "msvcp140_1.dll": (
        25_656,
        "56b6711cd9824d2922b9d6640c0a03a941949f23cc1f0141b10909df3060bd6d",
    ),
    "msvcp140_2.dll": (
        257_608,
        "4e27fcdb0c68e86273d3881ae8068903f993e0fe1bf3618a5ff2442c5c4ec801",
    ),
    "msvcp140_atomic_wait.dll": (
        40_008,
        "afee47f8c81eaa3863b422291948dae76a2f7e853acad57de63a27d5ea56da55",
    ),
    "msvcp140_codecvt_ids.dll": (
        21_560,
        "99e426f790566241e8954dbe9bdafa944cc9eedba9ed3b597244bff579baeedc",
    ),
    "System.ClientModel.dll": (
        168_504,
        "853f600e1e5361907ede16434052c7a5533c980e1cf68bc530d5047590667849",
    ),
    "System.Memory.Data.dll": (
        29_256,
        "601e46fa4237d625737225e5c550f570ca0d8fe47bc65328c40798b7b267b994",
    ),
    "System.Security.Cryptography.ProtectedData.dll": (
        27_192,
        "2eefe57706b4797b9c120135581d349d1297f59ee01ceab487247ed7f41b3898",
    ),
    "vcamp140.dll": (
        398_392,
        "4a0be69037198efa308fb366147ea5b6ed8bfe1d0461ce9d3d2caef5fdb3b29e",
    ),
    "vccorlib140.dll": (
        341_560,
        "49ec7ce4e0813ea8f591260a73bc95141e0a348fbc2f989c34504fe2c8a3b2bb",
    ),
    "vcomp140.dll": (
        181_832,
        "cccfa4b5cbd9db3037ead853198bf11d96a0b65e9c0b29a8cb77d8d109f4ff17",
    ),
    "vcruntime140.dll": (
        110_152,
        "4b99b5feefad94835ceb785523601363e94d152d1ccecf320469646ebee3ca0d",
    ),
    "vcruntime140_1.dll": (
        39_480,
        "d36143a132a3b6feeb261a2a655c31d18b9a0d9caa2d79cdf81139cbd5840af8",
    ),
    "vcruntime140_threads.dll": (
        28_216,
        "f99f30dcc43b1bbade02d5681054ac5a64369f25e4f52ea313beb8fe5e0ef1d7",
    ),
}
SIGNTOOL_PACKAGE_VERSION = "10.0.28000.2526"
SIGNTOOL_SDK_VERSION = "10.0.28000.0"
SIGNTOOL_DIR = ROOT / "tools" / "windows-sdk-signing"
SIGNTOOL_PACKAGE_URL = (
    "https://api.nuget.org/v3-flatcontainer/microsoft.windows.sdk.buildtools/"
    f"{SIGNTOOL_PACKAGE_VERSION}/"
    f"microsoft.windows.sdk.buildtools.{SIGNTOOL_PACKAGE_VERSION}.nupkg"
)
# Measured 2026-08-24 from the exact official NuGet package above. Signtool and
# its complete private x64 side-by-side closure are extracted into a directory
# containing no unmeasured siblings, checked again before every signature, and
# pinned until the process exits. The host SDK, PATH and SIGNTOOL are never used.
SIGNTOOL_PACKAGE_BYTES = 22_238_762
SIGNTOOL_PACKAGE_SHA256 = (
    "a09a4c9d68160ced4765137a9a7444ea560ea86c45d6a77093dea58c2f7563a0"
)
SIGNTOOL_RUNTIME_FILES: dict[str, tuple[int, str]] = {
    "appxpackaging.dll": (
        2_439_624,
        "9ec6c6da3b3f56e7e9280b6b7f6ea40a2b69c7d35d772517446997e72ddb5f31",
    ),
    "appxsip.dll": (
        461_256,
        "d5eed9fac1072f0473a782f0a4623953447c769ee5c3e3468e41f85348fe9a1e",
    ),
    "Microsoft.Windows.Build.Appx.AppxPackaging.dll.manifest": (
        1_924,
        "b50d228857d9cd6509854feacdc153a78804be4312bf699298dbf6a5bd846ddd",
    ),
    "Microsoft.Windows.Build.Appx.AppxSip.dll.manifest": (
        491,
        "bf9dadc2a05e46f479f8bdc4ebda515682449cfe04794afb9b8a9cf12edf7f4e",
    ),
    "Microsoft.Windows.Build.Appx.OpcServices.dll.manifest": (
        450,
        "7babce6a576b434c71b93797ac6968963a9c23feea56d8d45faeeaa8175907ec",
    ),
    "Microsoft.Windows.Build.Signing.mssign32.dll.manifest": (
        238,
        "e0c65f5b5f5ae116abbb566dc7ddf95b666fc9a7f327afa67502100954d8d713",
    ),
    "Microsoft.Windows.Build.Signing.wintrust.dll.manifest": (
        238,
        "9a6e1a316f4148eaace69326f5f99a6a7ac4e9b58b5c22082f6603dd964bfad9",
    ),
    "mssign32.dll": (
        150_000,
        "c80184629a6c7c7bd991c42916d2c207992cedfa97492ac8eeae527923fe1e5d",
    ),
    "opcservices.dll": (
        1_743_304,
        "07d6b667a02c44431d7eb3b64ac246db6d00e28a13edeca47a69e9c48eeba412",
    ),
    "signtool.exe": (
        551_408,
        "80972965e7fc311d293222b1a0e2c1bfb60f363239173964dbe2a71638314b9f",
    ),
    "signtool.exe.manifest": (
        968,
        "c1a768e47b3d054eee0d8ab9027eba122a52bf6a058ae1c02e4ddcb96cf4b09f",
    ),
    "wintrust.dll": (
        461_256,
        "4504dc9fde99e27ff7e9cad4150608e88298319305a2f376247506a43001ad44",
    ),
    "wintrust.dll.ini": (
        2_081,
        "ca458c3ff25d27a7c61674ee9547f13fc70c7208583cbfba75ca39b4098fa21c",
    ),
}
TS_TIMESTAMP = "http://timestamp.acs.microsoft.com"
_TS_ENV_KEYS = (
    "TRUSTED_SIGNING_ENDPOINT",
    "TRUSTED_SIGNING_ACCOUNT",
    "TRUSTED_SIGNING_PROFILE",
    "AZURE_TENANT_ID",
    "AZURE_CLIENT_ID",
    "AZURE_CLIENT_SECRET",
)


def _signing_config() -> dict[str, str] | None:
    # Opt-in: signing is off unless GORE_SIGN=1 (CI sets it). When opted in every
    # credential must be present — a missing one hard-fails rather than silently
    # shipping an unsigned build.
    if os.environ.get("GORE_SIGN") != "1":
        return None
    vals = {k: os.environ.get(k, "") for k in _TS_ENV_KEYS}
    missing = [k for k, v in vals.items() if not v]
    if missing:
        raise SystemExit(f"GORE_SIGN=1 but missing signing env: {', '.join(missing)}")
    return vals


# Opt-in proxy bypass for the signing call only (GORE_SIGN_NO_PROXY=1).
#
# A corporate PAC may route login.microsoftonline.com through a local tunnel; when
# that tunnel is down, token acquisition dies with "No connection could be made
# because the target machine actively refused it (localhost:9000)" and signing
# fails even though the network is fine. These overrides are handed only to the
# process that owns signing (direct signtool, or ISCC and its signtool child), so
# the rest of the build (Flutter, cargo, pub) keeps the system proxy untouched.
#
# .NET only builds its proxy from the environment when a proxy is actually set
# there, and only then honours NO_PROXY -- hence the deliberately dead dummy
# proxy paired with a bypass list covering every host signing talks to (AAD
# login, the codesigning endpoint, and the RFC3161 timestamp server). Bypass
# entries need the leading-dot form to match subdomains.
_SIGN_NO_PROXY = (
    ".microsoftonline.com,login.microsoftonline.com,.microsoft.com,"
    ".azure.net,.windows.net"
)


def _sign_proxy_overrides() -> dict[str, str]:
    if os.environ.get("GORE_SIGN_NO_PROXY") != "1":
        return {}
    print("signing: bypassing the system proxy (GORE_SIGN_NO_PROXY=1)")
    return {
        "HTTP_PROXY": "http://127.0.0.1:9",
        "HTTPS_PROXY": "http://127.0.0.1:9",
        "NO_PROXY": _SIGN_NO_PROXY,
    }


def _find_signtool() -> Path:
    return _ensure_signtool()


def _verify_trusted_signing_file(
    path: Path,
    expected_size: int,
    expected_sha256: str,
    label: str = "Trusted Signing runtime",
) -> None:
    """Hash one signing file without following a link or accepting hardlinks."""

    try:
        before = path.lstat()
        if (
            not stat.S_ISREG(before.st_mode)
            or path.is_symlink()
            or getattr(before, "st_file_attributes", 0)
            & getattr(stat, "FILE_ATTRIBUTE_REPARSE_POINT", 0x400)
        ):
            raise SystemExit(f"{label} entry is not a regular file: {path}")
        if before.st_nlink != 1:
            raise SystemExit(f"{label} entry has multiple links: {path}")
        if before.st_size != expected_size:
            raise SystemExit(
                f"{label} size mismatch for {path.name}: "
                f"expected {expected_size}, got {before.st_size}"
            )
        digest = hashlib.sha256()
        byte_len = 0
        with path.open("rb") as source:
            opened = os.fstat(source.fileno())
            if (
                not stat.S_ISREG(opened.st_mode)
                or getattr(opened, "st_file_attributes", 0)
                & getattr(stat, "FILE_ATTRIBUTE_REPARSE_POINT", 0x400)
                or opened.st_nlink != 1
                or opened.st_size != expected_size
                or (opened.st_dev, opened.st_ino) != (before.st_dev, before.st_ino)
            ):
                raise SystemExit(f"{label} entry changed while opening: {path}")
            while chunk := source.read(1024 * 1024):
                byte_len += len(chunk)
                if byte_len > expected_size:
                    raise SystemExit(f"{label} entry grew while reading: {path}")
                digest.update(chunk)
            closed = os.fstat(source.fileno())
        after = path.lstat()
    except (FileNotFoundError, OSError) as error:
        raise SystemExit(f"cannot verify {label} entry {path}: {error}") from error
    identity = (before.st_dev, before.st_ino, before.st_size, before.st_nlink)
    if (
        byte_len != expected_size
        or (opened.st_dev, opened.st_ino, opened.st_size, opened.st_nlink) != identity
        or (closed.st_dev, closed.st_ino, closed.st_size, closed.st_nlink) != identity
        or (after.st_dev, after.st_ino, after.st_size, after.st_nlink) != identity
        or not stat.S_ISREG(after.st_mode)
        or getattr(after, "st_file_attributes", 0)
        & getattr(stat, "FILE_ATTRIBUTE_REPARSE_POINT", 0x400)
        or path.is_symlink()
    ):
        raise SystemExit(f"{label} entry changed while reading: {path}")
    actual_sha256 = digest.hexdigest()
    if actual_sha256 != expected_sha256:
        raise SystemExit(
            f"{label} SHA-256 mismatch for {path.name}: "
            f"expected {expected_sha256}, got {actual_sha256}"
        )


def _verify_trusted_signing_runtime(root: Path) -> Path:
    """Verify the complete pinned x64 runtime and return its checked dlib."""

    try:
        root_stat = root.lstat()
        if (
            not stat.S_ISDIR(root_stat.st_mode)
            or root.is_symlink()
            or getattr(root_stat, "st_file_attributes", 0)
            & getattr(stat, "FILE_ATTRIBUTE_REPARSE_POINT", 0x400)
        ):
            raise SystemExit(f"Trusted Signing runtime is not a real directory: {root}")
        entries = list(root.iterdir())
    except (FileNotFoundError, OSError) as error:
        raise SystemExit(
            f"cannot inspect Trusted Signing runtime {root}: {error}"
        ) from error
    actual_names = [entry.name for entry in entries]
    if len(actual_names) != len(set(actual_names)) or set(actual_names) != set(
        TS_DLIB_RUNTIME_FILES
    ):
        missing = sorted(set(TS_DLIB_RUNTIME_FILES) - set(actual_names))
        unexpected = sorted(set(actual_names) - set(TS_DLIB_RUNTIME_FILES))
        raise SystemExit(
            "Trusted Signing runtime file set mismatch "
            f"(missing={missing}, unexpected={unexpected})"
        )
    for name, (expected_size, expected_sha256) in TS_DLIB_RUNTIME_FILES.items():
        _verify_trusted_signing_file(root / name, expected_size, expected_sha256)
    return root / "Azure.CodeSigning.Dlib.dll"


def _download_trusted_signing_package() -> bytes:
    """Fetch exactly the pinned package, bounded by its known byte length."""

    import urllib.request

    print(f"fetching pinned Trusted Signing dlib {TS_DLIB_VERSION} from nuget.org ...")
    try:
        with urllib.request.urlopen(TS_DLIB_PACKAGE_URL, timeout=60) as response:
            data = response.read(TS_DLIB_PACKAGE_BYTES + 1)
    except OSError as error:
        raise SystemExit(f"Trusted Signing package download failed: {error}") from error
    if len(data) != TS_DLIB_PACKAGE_BYTES:
        raise SystemExit(
            "Trusted Signing package size mismatch: "
            f"expected {TS_DLIB_PACKAGE_BYTES}, got {len(data)}"
        )
    actual_sha256 = hashlib.sha256(data).hexdigest()
    if actual_sha256 != TS_DLIB_PACKAGE_SHA256:
        raise SystemExit(
            "Trusted Signing package SHA-256 mismatch: "
            f"expected {TS_DLIB_PACKAGE_SHA256}, got {actual_sha256}"
        )
    return data


def _ensure_dlib() -> Path:
    """Return only the exact pinned Microsoft Trusted Signing x64 runtime.

    A pre-existing gitignored runtime is never trusted by location. It must
    match every pinned file exactly. A missing runtime is staged from package
    bytes whose size and SHA-256 are verified before ZIP parsing, then published
    without replacing anything that appeared concurrently.
    """

    if os.path.lexists(TS_DLIB_DIR):
        return _verify_trusted_signing_runtime(TS_DLIB_DIR)

    import io

    data = _download_trusted_signing_package()
    prefix = "bin/x64/"
    TS_DLIB_DIR.parent.mkdir(parents=True, exist_ok=True)
    try:
        with zipfile.ZipFile(io.BytesIO(data)) as archive:
            selected = [
                entry
                for entry in archive.infolist()
                if entry.filename.startswith(prefix) and not entry.is_dir()
            ]
            archive_names = [entry.filename.removeprefix(prefix) for entry in selected]
            if len(archive_names) != len(set(archive_names)) or set(
                archive_names
            ) != set(TS_DLIB_RUNTIME_FILES):
                raise SystemExit(
                    "pinned Trusted Signing package has an unexpected x64 file set"
                )
            by_name = {entry.filename.removeprefix(prefix): entry for entry in selected}
            with tempfile.TemporaryDirectory(
                prefix="trusted-signing-", dir=TS_DLIB_DIR.parent
            ) as temporary:
                staged = Path(temporary)
                for name, (
                    expected_size,
                    expected_sha256,
                ) in TS_DLIB_RUNTIME_FILES.items():
                    entry = by_name[name]
                    if entry.file_size != expected_size:
                        raise SystemExit(
                            f"pinned Trusted Signing package size mismatch for {name}"
                        )
                    destination = staged / name
                    with (
                        archive.open(entry) as source,
                        destination.open("xb") as output,
                    ):
                        shutil.copyfileobj(source, output, length=1024 * 1024)
                        output.flush()
                        os.fsync(output.fileno())
                    _verify_trusted_signing_file(
                        destination, expected_size, expected_sha256
                    )
                _verify_trusted_signing_runtime(staged)
                if os.path.lexists(TS_DLIB_DIR):
                    raise SystemExit(
                        "Trusted Signing runtime appeared during download; refusing to replace it"
                    )
                staged.rename(TS_DLIB_DIR)
    except zipfile.BadZipFile as error:
        raise SystemExit(
            f"pinned Trusted Signing package is not a valid ZIP: {error}"
        ) from error
    except OSError as error:
        raise SystemExit(
            f"cannot publish pinned Trusted Signing runtime: {error}"
        ) from error
    return _verify_trusted_signing_runtime(TS_DLIB_DIR)


def _verify_signtool_runtime(root: Path) -> Path:
    """Verify the complete private Signtool x64 runtime and return its executable."""

    try:
        root_stat = root.lstat()
        if (
            not stat.S_ISDIR(root_stat.st_mode)
            or root.is_symlink()
            or getattr(root_stat, "st_file_attributes", 0)
            & getattr(stat, "FILE_ATTRIBUTE_REPARSE_POINT", 0x400)
        ):
            raise SystemExit(f"Signtool runtime is not a real directory: {root}")
        entries = list(root.iterdir())
    except (FileNotFoundError, OSError) as error:
        raise SystemExit(f"cannot inspect Signtool runtime {root}: {error}") from error
    actual_names = [entry.name for entry in entries]
    if len(actual_names) != len(set(actual_names)) or set(actual_names) != set(
        SIGNTOOL_RUNTIME_FILES
    ):
        missing = sorted(set(SIGNTOOL_RUNTIME_FILES) - set(actual_names))
        unexpected = sorted(set(actual_names) - set(SIGNTOOL_RUNTIME_FILES))
        raise SystemExit(
            "Signtool runtime file set mismatch "
            f"(missing={missing}, unexpected={unexpected})"
        )
    for name, (expected_size, expected_sha256) in SIGNTOOL_RUNTIME_FILES.items():
        _verify_trusted_signing_file(
            root / name,
            expected_size,
            expected_sha256,
            "Signtool runtime",
        )
    return root / "signtool.exe"


def _download_signtool_package() -> bytes:
    """Fetch exactly the pinned Windows SDK Build Tools NuGet package."""

    import urllib.request

    print(
        "fetching pinned Windows SDK Build Tools "
        f"{SIGNTOOL_PACKAGE_VERSION} from nuget.org ..."
    )
    try:
        with urllib.request.urlopen(SIGNTOOL_PACKAGE_URL, timeout=60) as response:
            data = response.read(SIGNTOOL_PACKAGE_BYTES + 1)
    except OSError as error:
        raise SystemExit(f"pinned Signtool package download failed: {error}") from error
    if len(data) != SIGNTOOL_PACKAGE_BYTES:
        raise SystemExit(
            "Signtool package size mismatch: "
            f"expected {SIGNTOOL_PACKAGE_BYTES}, got {len(data)}"
        )
    actual_sha256 = hashlib.sha256(data).hexdigest()
    if actual_sha256 != SIGNTOOL_PACKAGE_SHA256:
        raise SystemExit(
            "Signtool package SHA-256 mismatch: "
            f"expected {SIGNTOOL_PACKAGE_SHA256}, got {actual_sha256}"
        )
    return data


def _ensure_signtool() -> Path:
    """Return the exact package-pinned Signtool and its private dependency closure."""

    if os.path.lexists(SIGNTOOL_DIR):
        return _verify_signtool_runtime(SIGNTOOL_DIR)

    import io

    data = _download_signtool_package()
    prefix = f"bin/{SIGNTOOL_SDK_VERSION}/x64/"
    SIGNTOOL_DIR.parent.mkdir(parents=True, exist_ok=True)
    try:
        with zipfile.ZipFile(io.BytesIO(data)) as archive:
            selected = [
                entry
                for entry in archive.infolist()
                if entry.filename.removeprefix(prefix) in SIGNTOOL_RUNTIME_FILES
                and entry.filename.startswith(prefix)
                and not entry.is_dir()
            ]
            archive_names = [entry.filename.removeprefix(prefix) for entry in selected]
            if len(archive_names) != len(set(archive_names)) or set(
                archive_names
            ) != set(SIGNTOOL_RUNTIME_FILES):
                raise SystemExit(
                    "pinned Windows SDK package does not contain the exact Signtool closure"
                )
            by_name = {entry.filename.removeprefix(prefix): entry for entry in selected}
            with tempfile.TemporaryDirectory(
                prefix="windows-sdk-signing-", dir=SIGNTOOL_DIR.parent
            ) as temporary:
                staged = Path(temporary)
                for name, (
                    expected_size,
                    expected_sha256,
                ) in SIGNTOOL_RUNTIME_FILES.items():
                    entry = by_name[name]
                    if entry.file_size != expected_size:
                        raise SystemExit(
                            f"pinned Signtool package size mismatch for {name}"
                        )
                    destination = staged / name
                    with (
                        archive.open(entry) as source,
                        destination.open("xb") as output,
                    ):
                        shutil.copyfileobj(source, output, length=1024 * 1024)
                        output.flush()
                        os.fsync(output.fileno())
                    _verify_trusted_signing_file(
                        destination,
                        expected_size,
                        expected_sha256,
                        "Signtool runtime",
                    )
                _verify_signtool_runtime(staged)
                if os.path.lexists(SIGNTOOL_DIR):
                    raise SystemExit(
                        "Signtool runtime appeared during download; refusing to replace it"
                    )
                staged.rename(SIGNTOOL_DIR)
    except zipfile.BadZipFile as error:
        raise SystemExit(
            f"pinned Signtool package is not a valid ZIP: {error}"
        ) from error
    except OSError as error:
        raise SystemExit(f"cannot publish pinned Signtool runtime: {error}") from error
    return _verify_signtool_runtime(SIGNTOOL_DIR)


def _write_metadata(cfg: dict[str, str]) -> Path:
    import json
    import tempfile

    meta = {
        "Endpoint": cfg["TRUSTED_SIGNING_ENDPOINT"],
        "CodeSigningAccountName": cfg["TRUSTED_SIGNING_ACCOUNT"],
        "CertificateProfileName": cfg["TRUSTED_SIGNING_PROFILE"],
    }
    fd, path = tempfile.mkstemp(prefix="ts-meta-", suffix=".json")
    with os.fdopen(fd, "w", encoding="utf-8") as f:
        json.dump(meta, f)
    return Path(path)


def sign_paths(paths: list[Path], dry: bool) -> None:
    """Authenticode-sign the given PE files (exe/dll) via Trusted Signing. No-op
    when signing env is unset (see _signing_config) or no PE files are given."""
    pe = [p for p in paths if p.suffix.lower() in (".exe", ".dll") and p.exists()]
    cfg = _signing_config()
    if cfg is None:
        if pe:
            print(
                f"signing: off (default) for {len(pe)} file(s) — set GORE_SIGN=1 to enable"
            )
        return
    if not pe:
        return
    if dry:
        print(f"[dry-run] would code-sign {len(pe)} file(s)")
        return
    dlib = _ensure_dlib()
    signtool = _find_signtool()
    meta = _write_metadata(cfg)
    try:
        _run_trusted_signing_once(signtool, dlib, meta, pe)
    finally:
        meta.unlink(missing_ok=True)


def _trusted_signing_args(
    signtool: Path, dlib: Path, metadata: Path, paths: list[Path | str]
) -> list[Path | str]:
    """Build the one Azure Trusted Signing command used by direct and Inno signing."""
    return [
        signtool,
        "sign",
        "/v",
        "/fd",
        "SHA256",
        "/tr",
        TS_TIMESTAMP,
        "/td",
        "SHA256",
        "/dlib",
        dlib,
        "/dmdf",
        metadata,
        *paths,
    ]


def _same_path(left: Path, right: Path) -> bool:
    return os.path.normcase(os.path.abspath(left)) == os.path.normcase(
        os.path.abspath(right)
    )


@contextmanager
def _pin_trusted_signing_runtime(root: Path):
    """Keep every measured runtime path immutable until signtool exits."""

    try:
        with ExitStack() as pins:
            for name in sorted(TS_DLIB_RUNTIME_FILES):
                pins.enter_context(
                    standalone_compiler_bundle._pin_windows_file_path(
                        root / name,
                        f"Trusted Signing runtime file {name}",
                    )
                )
            yield
    except standalone_compiler_bundle.BundleError as error:
        raise SystemExit(f"cannot pin Trusted Signing runtime: {error}") from error


@contextmanager
def _pin_signtool_runtime(root: Path):
    """Keep the complete measured Signtool path and private closure immutable."""

    try:
        with ExitStack() as pins:
            for name in sorted(SIGNTOOL_RUNTIME_FILES):
                pins.enter_context(
                    standalone_compiler_bundle._pin_windows_file_path(
                        root / name,
                        f"Signtool runtime file {name}",
                    )
                )
            yield
    except standalone_compiler_bundle.BundleError as error:
        raise SystemExit(f"cannot pin Signtool runtime: {error}") from error


def _run_trusted_signing_once(
    signtool: Path,
    dlib: Path,
    metadata: Path,
    paths: list[Path | str],
) -> None:
    """Pin and re-check the exact tool, runtime, metadata and target identities."""

    targets = [Path(path) for path in paths]
    for path in (signtool, dlib, metadata, *targets):
        if not path.is_absolute() or Path(os.path.normpath(path)) != path:
            raise SystemExit(f"signing path must be absolute and normalized: {path}")
    try:
        with ExitStack() as pins:
            pins.enter_context(_pin_signtool_runtime(signtool.parent))
            pins.enter_context(_pin_trusted_signing_runtime(dlib.parent))
            pins.enter_context(
                standalone_compiler_bundle._pin_windows_file_path(
                    metadata,
                    "Trusted Signing metadata",
                )
            )
            for target in targets:
                pins.enter_context(
                    standalone_compiler_bundle._pin_windows_mutable_file_path(
                        target,
                        "Authenticode signing target",
                    )
                )

            checked_signtool = _verify_signtool_runtime(signtool.parent)
            checked_dlib = _verify_trusted_signing_runtime(dlib.parent)
            if not _same_path(signtool, checked_signtool):
                raise SystemExit(
                    f"Signtool path is outside the pinned runtime: {signtool}"
                )
            if not _same_path(dlib, checked_dlib):
                raise SystemExit(
                    f"Trusted Signing dlib path is outside the pinned runtime: {dlib}"
                )
            run(
                f"code-sign {len(targets)} file(s)",
                _trusted_signing_args(
                    checked_signtool,
                    checked_dlib,
                    metadata,
                    targets,
                ),
                extra_env=_sign_proxy_overrides(),
            )
    except standalone_compiler_bundle.BundleError as error:
        raise SystemExit(f"cannot pin signing input: {error}") from error


def _inno_quote_sign_arg(value: Path | str) -> str:
    """Quote a fixed SignTool argument using Inno's command-line placeholders."""
    raw = str(value)
    if '"' in raw or "\r" in raw or "\n" in raw:
        raise SystemExit(f"invalid character in Inno SignTool argument: {raw!r}")
    return f"$q{raw.replace('$', '$$')}$q"


def _inno_trusted_signing_command(signtool: Path, dlib: Path, metadata: Path) -> str:
    """Route each Inno signature through a fresh runtime verification."""

    args: list[Path | str] = [
        Path(sys.executable),
        Path(__file__).resolve(),
        "__trusted_signing__",
        "trusted-sign-one",
        "--signtool",
        signtool,
        "--dlib",
        dlib,
        "--metadata",
        metadata,
        "--path",
        "$f",
    ]
    return " ".join(
        _inno_quote_sign_arg(arg) if isinstance(arg, Path) else str(arg) for arg in args
    )


def sign_dir(directory: Path, dry: bool, exclude_names: tuple[str, ...] = ()) -> None:
    """Sign every owned PE under `directory`, except named redistributables."""
    excluded = {name.casefold() for name in exclude_names}
    sign_paths(
        sorted(
            path
            for path in directory.rglob("*")
            if path.name.casefold() not in excluded
        ),
        dry,
    )


def pdir(project: str) -> Path:
    return ROOT / PROJECTS[project]["dir"]


# --------------------------------------------------------------------------- #
# Docs staging                                                                #
# --------------------------------------------------------------------------- #
# Base for links the shipped guide inherits from the repo. A guide page links to
# component READMEs and crates with `../…`; inside a release zip those targets do
# not exist, so they are rewritten to absolute GitHub URLs.
# GitHub serves files under /blob/ and directories under /tree/ — it redirects
# between the two, but emit the right one so the packaged links resolve in one
# hop and do not depend on that redirect.
REPO_WEB_BASE = "https://github.com/dh0er/gore"


def repo_ref() -> str:
    """The commit the docs are staged from, for pinning outbound links.

    A branch name would rot: a link is only valid for the tree the guide was
    built from, and a release zip outlives whatever `main` looks like later.
    Falls back to `main` outside a git checkout.
    """
    try:
        out = subprocess.run(
            ["git", "rev-parse", "HEAD"],
            cwd=ROOT,
            capture_output=True,
            text=True,
            check=True,
        )
    except (OSError, subprocess.CalledProcessError):
        return "main"
    return out.stdout.strip() or "main"


# Any Markdown link whose target starts with `../` — one or more levels up.
_OUTBOUND_LINK_RE = re.compile(r"\]\((\.\./[^)#]*?)(#[^)]*)?\)")


def rewrite_outbound_links(
    text: str, page_dir: str, ref: str, base: str = REPO_WEB_BASE
) -> str:
    """Point Markdown links that escape the doc tree at GitHub.

    `page_dir` is the containing directory of the page, relative to the repo
    root (e.g. `docs/guide`), and is what the `../` hops are resolved against.
    Links inside the tree (`items.md`, `items.md#flags`) are left untouched —
    they still resolve next to the shipped file.
    """

    def repoint(m: re.Match) -> str:
        target = posixpath.normpath(posixpath.join(page_dir, m.group(1)))
        route = "tree" if (ROOT / target).is_dir() else "blob"
        return f"]({base}/{route}/{ref}/{target}{m.group(2) or ''})"

    return _OUTBOUND_LINK_RE.sub(repoint, text)


def stage_docs(src_dir: Path, dest_dir: Path) -> int:
    """Copy the Markdown docs in `src_dir` to `dest_dir`, rewriting links.

    Returns the number of files written. Only `.md` files are shipped; anything
    else in the doc tree stays in the repo.
    """
    dest_dir.mkdir(parents=True, exist_ok=True)
    ref = repo_ref()
    src_from_root = Path(os.path.relpath(src_dir.resolve(), ROOT)).as_posix()
    count = 0
    for md in sorted(src_dir.rglob("*.md")):
        rel = md.relative_to(src_dir)
        target = dest_dir / rel
        target.parent.mkdir(parents=True, exist_ok=True)
        page_dir = posixpath.normpath(
            posixpath.join(src_from_root, rel.parent.as_posix())
        )
        target.write_text(
            rewrite_outbound_links(md.read_text(encoding="utf-8"), page_dir, ref),
            encoding="utf-8",
            newline="\n",
        )
        count += 1
    return count


# --------------------------------------------------------------------------- #
# Version helpers                                                             #
# --------------------------------------------------------------------------- #
VERSION_RE = re.compile(r"^[0-9]+\.[0-9]+\.[0-9]+$")


def read_version(project: str) -> str:
    cfg = PROJECTS[project]
    if cfg["kind"] == "flutter":
        text = (pdir(project) / cfg["pubspec"]).read_text(encoding="utf-8")
        m = re.search(r"^version:\s*([0-9]+\.[0-9]+\.[0-9]+)", text, re.MULTILINE)
        return m.group(1) if m else "0.0.0"
    # rust
    text = (pdir(project) / cfg["manifest"]).read_text(encoding="utf-8")
    m = re.search(r'^version\s*=\s*"([0-9]+\.[0-9]+\.[0-9]+)"', text, re.MULTILINE)
    return m.group(1) if m else "0.0.0"


def write_version(project: str, version: str, dry: bool) -> None:
    cfg = PROJECTS[project]
    if cfg["kind"] == "flutter":
        path = pdir(project) / cfg["pubspec"]
        pattern = r"^version:\s*[0-9]+\.[0-9]+\.[0-9]+.*$"
        replacement = f"version: {version}"
    else:
        path = pdir(project) / cfg["manifest"]
        pattern = r'^version\s*=\s*"[0-9]+\.[0-9]+\.[0-9]+"'
        replacement = f'version = "{version}"'
    text = path.read_text(encoding="utf-8")
    new, n = re.subn(pattern, replacement, text, count=1, flags=re.MULTILINE)
    if n == 0:
        raise SystemExit(f"no version line found to bump in {path}")
    print(f"bump {project} -> {version} ({path})")
    if dry:
        return
    path.write_text(new, encoding="utf-8")
    # A Rust version bump must be mirrored into the workspace Cargo.lock, or a
    # tag built with `cargo --locked` resolves the old version. Update only the
    # bumped crate to exactly this version (`-p <crate> --precise`), so no
    # unrelated dependency resolutions sneak into the release commit.
    if cfg["kind"] in ("rust-bin", "rust-lib"):
        run(
            f"sync Cargo.lock {project}",
            [CARGO, "update", "-p", cfg["crate"], "--precise", version],
        )


def changelog_notes(project: str, version: str) -> str:
    cfg = PROJECTS[project]
    name = cfg.get("changelog")
    if not name:
        raise SystemExit(f"{project} has no CHANGELOG configured")
    path = pdir(project) / name
    if not path.exists():
        raise SystemExit(f"missing {path}")
    text = path.read_text(encoding="utf-8")
    pattern = re.compile(
        rf"^## \[{re.escape(version)}\][^\r\n]*\r?\n(.*?)(?=^## \[|\Z)",
        re.MULTILINE | re.DOTALL,
    )
    m = pattern.search(text)
    notes = m.group(1).strip() if m else ""
    if not notes:
        raise SystemExit(f"CHANGELOG.md has no non-empty section for {version}")
    return notes


def resolve_git_sha() -> str:
    env_sha = os.environ.get("GITHUB_SHA", "")
    if env_sha:
        return env_sha[:7]
    try:
        probe = subprocess.run(
            ["git", "rev-parse", "--short=7", "HEAD"],
            cwd=ROOT,
            capture_output=True,
            text=True,
        )
    except OSError:
        return "dev"
    return probe.stdout.strip() or "dev" if probe.returncode == 0 else "dev"


def _git(args: list[str]) -> str | None:
    """Run a git command in the repository root; None if git or the repo is absent."""
    try:
        probe = subprocess.run(
            ["git", *args], cwd=ROOT, capture_output=True, text=True, check=False
        )
    except OSError:
        return None
    return probe.stdout if probe.returncode == 0 else None


def discard_line_ending_only_churn(project: str) -> None:
    """Restore tracked files a build rewrote with different line endings only.

    `flutter build` regenerates the tracked l10n sources, and on Windows writes them
    with CRLF where the repository stores LF. `git diff` reports nothing, because
    `.gitattributes` marks them `text eol=lf` and Git normalises the difference
    away — but `git status` still calls them modified and never stops, so a build
    left behind a worktree that looked dirty and aborted any tooling that insists on
    a clean branch.

    A file is restored only when `git diff` reports nothing for it — the same
    question, asked the way Git itself answers it. Normalisation applies, so a
    line-ending-only difference shows as no diff, while anything Git would carry
    into a commit shows up. That covers a changed file mode as well: an executable
    bit is not part of a blob's contents, so comparing blob hashes would have missed
    a `chmod +x` and thrown it away. A genuinely regenerated file — a real
    translation change — is left exactly where it is, for the developer to review
    and commit.
    """
    project_dir = pdir(project).relative_to(ROOT).as_posix()
    status = _git(["status", "--porcelain=v1", "-z", "--", project_dir])
    if not status:
        return
    for entry in status.split("\0"):
        # Worktree-modified, unstaged: "_M<path>". Anything staged or untracked is
        # the developer's business, not ours.
        if not entry.startswith(" M "):
            continue
        path = entry[3:]
        real_diff = _git(["diff", "--raw", "--", path])
        if real_diff is None or real_diff.strip():
            continue  # a difference Git would keep: contents, or the file mode
        _git(["checkout", "--", path])


# --------------------------------------------------------------------------- #
# Build recipes                                                               #
# --------------------------------------------------------------------------- #
def flutter_build_dir(project: str, release: bool = True) -> Path:
    mode = "Release" if release else "Debug"
    return pdir(project) / "build" / "windows" / "x64" / "runner" / mode


def dist_dir(project: str) -> Path:
    """Top-level dist output dir for a project (dist/<project>/)."""
    return ROOT / "dist" / project


def flutter_release_dir(project: str) -> Path:
    return flutter_build_dir(project, release=True)


def target_dir(release: bool) -> Path:
    return ROOT / "target" / ("release" if release else "debug")


_PREPARED_STANDALONE_BUNDLES: dict[
    tuple[str | None, bool], standalone_compiler_bundle.PreparedBundle
] = {}
_QUALIFIED_PROFILE_VERIFIER: tuple[Path, standalone_compiler_bundle.Seal] | None = None
_PROMOTION_ATTESTATION_VERIFIER: tuple[Path, standalone_compiler_bundle.Seal] | None = (
    None
)
_INTERNAL_STANDALONE_COMPILER_ASSET_ROOT = ROOT / "crates" / "gore-as" / "assets"
_INTERNAL_STANDALONE_COMPILER_ARCHIVE = (
    _INTERNAL_STANDALONE_COMPILER_ASSET_ROOT
    / standalone_compiler_bundle.INTERNAL_PACKAGE_ARCHIVE_FILE
)
_INTERNAL_STANDALONE_COMPILER_DESCRIPTOR = (
    _INTERNAL_STANDALONE_COMPILER_ASSET_ROOT
    / standalone_compiler_bundle.INTERNAL_PACKAGE_DESCRIPTOR_FILE
)


def _configured_standalone_compiler_release_input() -> Path | None:
    raw = os.environ.get("GORE_STANDALONE_COMPILER_RELEASE_INPUT", "").strip()
    if not raw:
        return None
    path = Path(raw)
    if not path.is_absolute() or any(part in (".", "..") for part in path.parts):
        raise SystemExit(
            "GORE_STANDALONE_COMPILER_RELEASE_INPUT must be an absolute normalized path"
        )
    return path


def _qualified_profile_verifier(*, dry: bool):
    """Build once and return the pinned Rust typed-profile verifier callback."""

    global _QUALIFIED_PROFILE_VERIFIER
    if dry:
        return None
    if _QUALIFIED_PROFILE_VERIFIER is None:
        target_parent = ROOT / "target" / "standalone-profile-verifier-trusted"
        target_parent.mkdir(parents=True, exist_ok=True)
        target_root = Path(
            tempfile.mkdtemp(prefix="build-", dir=target_parent)
        ).resolve()
        run(
            "cargo build standalone compiler qualified-profile verifier (release)",
            [
                CARGO,
                "build",
                "-p",
                "gore-as",
                "--release",
                "--bin",
                "gore-as-qualified-profile-verifier",
                "--locked",
            ],
            dry=False,
            extra_env={"CARGO_TARGET_DIR": str(target_root)},
        )
        candidate = target_root / "release" / "gore-as-qualified-profile-verifier.exe"
        if not candidate.is_file():
            raise SystemExit(
                f"qualified-profile verifier build did not produce {candidate}"
            )
        _QUALIFIED_PROFILE_VERIFIER = _promote_qualified_profile_verifier_authority(
            candidate, target_root
        )
    verifier, verifier_seal = _QUALIFIED_PROFILE_VERIFIER

    def verify(
        root: Path, profile_sha256: str
    ) -> standalone_compiler_bundle.QualifiedProfileTreeAuthority:
        return standalone_compiler_bundle.verify_qualified_profile_with_executable(
            verifier, verifier_seal, root, profile_sha256
        )

    return verify


def _promote_qualified_profile_verifier_authority(
    candidate: Path, target_root: Path
) -> tuple[Path, standalone_compiler_bundle.Seal]:
    """Copy Cargo's pinned hardlink into the sole single-link executable authority."""

    candidate_bytes = standalone_compiler_bundle._read_pinned_windows_regular(
        candidate,
        standalone_compiler_bundle.MAX_SIDECAR_BYTES,
        "freshly linked qualified-profile verifier",
        require_single_link=False,
    )
    authority_dir = target_root / "authority"
    try:
        authority_dir.mkdir()
        verifier = authority_dir / "gore-as-qualified-profile-verifier.exe"
        with verifier.open("xb") as stream:
            stream.write(candidate_bytes)
            stream.flush()
            os.fsync(stream.fileno())
    except OSError as error:
        raise SystemExit(f"cannot publish verifier authority copy: {error}") from error
    verifier_bytes = standalone_compiler_bundle._read_regular_no_follow(
        verifier,
        standalone_compiler_bundle.MAX_SIDECAR_BYTES,
        "single-link qualified-profile verifier authority",
    )
    if verifier_bytes != candidate_bytes:
        raise SystemExit("qualified-profile verifier authority copy changed")
    return (
        verifier,
        standalone_compiler_bundle.Seal(
            len(verifier_bytes), hashlib.sha256(verifier_bytes).hexdigest()
        ),
    )


def _promotion_attestation_verifier(*, dry: bool):
    """Return a single-link, measured GitHub CLI attestation verifier."""

    global _PROMOTION_ATTESTATION_VERIFIER
    if dry:
        return None
    if _PROMOTION_ATTESTATION_VERIFIER is None:
        candidate_name = shutil.which("gh.exe") or shutil.which("gh")
        if candidate_name is None:
            raise SystemExit(
                "GitHub CLI is required to verify standalone compiler provenance"
            )
        candidate = Path(candidate_name).resolve()
        target_parent = ROOT / "target" / "github-attestation-verifier-trusted"
        target_parent.mkdir(parents=True, exist_ok=True)
        target_root = Path(
            tempfile.mkdtemp(prefix="build-", dir=target_parent)
        ).resolve()
        candidate_bytes = standalone_compiler_bundle._read_pinned_windows_regular(
            candidate,
            standalone_compiler_bundle.MAX_SIDECAR_BYTES,
            "installed GitHub attestation verifier",
            require_single_link=False,
        )
        verifier = target_root / "gh.exe"
        try:
            with verifier.open("xb") as stream:
                stream.write(candidate_bytes)
                stream.flush()
                os.fsync(stream.fileno())
        except OSError as error:
            raise SystemExit(
                f"cannot publish GitHub attestation verifier authority: {error}"
            ) from error
        verifier_bytes = standalone_compiler_bundle._read_regular_no_follow(
            verifier,
            standalone_compiler_bundle.MAX_SIDECAR_BYTES,
            "single-link GitHub attestation verifier authority",
        )
        if verifier_bytes != candidate_bytes:
            raise SystemExit("GitHub attestation verifier authority copy changed")
        _PROMOTION_ATTESTATION_VERIFIER = (
            verifier,
            standalone_compiler_bundle.Seal(
                len(verifier_bytes), hashlib.sha256(verifier_bytes).hexdigest()
            ),
        )
    verifier, verifier_seal = _PROMOTION_ATTESTATION_VERIFIER

    def verify(
        bundle_path: Path,
        subjects: dict[str, Path],
        authority: standalone_compiler_bundle.PromotionAuthority,
    ) -> None:
        standalone_compiler_bundle.verify_github_attestation_with_executable(
            verifier,
            verifier_seal,
            bundle_path,
            subjects,
            authority,
        )

    return verify


def _product_promotion_attestation_verifier(*, dry: bool):
    """Use external Sigstore verification only for an explicit development input."""

    if _configured_standalone_compiler_release_input() is None:
        return standalone_compiler_bundle.trust_pinned_internal_package_attestation
    return _promotion_attestation_verifier(dry=dry)


def _prepare_standalone_compiler_bundle(
    project: str, *, dry: bool
) -> standalone_compiler_bundle.PreparedBundle | None:
    if not PROJECTS[project].get("standalone_compiler_bundle"):
        return None
    configured_release_input = _configured_standalone_compiler_release_input()
    source_key = (
        f"development:{configured_release_input}"
        if configured_release_input is not None
        else f"internal:{_INTERNAL_STANDALONE_COMPILER_DESCRIPTOR}"
    )
    key = (source_key, dry)
    cached = _PREPARED_STANDALONE_BUNDLES.get(key)
    if cached is not None:
        return cached
    work_root = ROOT / "target" / "standalone-compiler-product-bundle"
    if dry:
        state = (
            "development release-input override"
            if configured_release_input is not None
            else "GORE-internal compressed package"
        )
        print(f"[dry-run] would prepare standalone compiler bundle: {state}")
        prepared = standalone_compiler_bundle.PreparedBundle(
            present=True,
            work_root=work_root,
            catalog_path=work_root / standalone_compiler_bundle.EMBEDDED_CATALOG_FILE,
            bundle_root=work_root / "compiler",
            sidecar_name=standalone_compiler_bundle.SIDECAR_FILE,
            catalog_sha256=None,
        )
    else:
        try:
            qualified_profile_verifier = _qualified_profile_verifier(dry=False)
            if configured_release_input is None:
                descriptor = standalone_compiler_bundle.read_internal_package_descriptor(
                    _INTERNAL_STANDALONE_COMPILER_DESCRIPTOR
                )
                extracted_parent = (
                    ROOT / "target" / "standalone-compiler-internal-input"
                )
                extracted_parent.mkdir(parents=True, exist_ok=True)
                release_input = standalone_compiler_bundle.materialize_internal_package(
                    _INTERNAL_STANDALONE_COMPILER_ARCHIVE,
                    _INTERNAL_STANDALONE_COMPILER_DESCRIPTOR,
                    extracted_parent / descriptor.archive.sha256,
                    qualified_profile_verifier=qualified_profile_verifier,
                )
                promotion_attestation_verifier = (
                    standalone_compiler_bundle.trust_pinned_internal_package_attestation
                )
            else:
                release_input = configured_release_input
                promotion_attestation_verifier = _promotion_attestation_verifier(
                    dry=False
                )
            prepared = standalone_compiler_bundle.prepare_product_bundle(
                release_input,
                work_root,
                qualified_profile_verifier=qualified_profile_verifier,
                promotion_attestation_verifier=promotion_attestation_verifier,
            )
        except standalone_compiler_bundle.BundleError as error:
            raise SystemExit(
                f"standalone compiler package failed: {error}"
            ) from error
        state = f"qualified catalog {prepared.catalog_sha256}"
        print(f"prepared standalone compiler bundle: {state}")
    _PREPARED_STANDALONE_BUNDLES[key] = prepared
    return prepared


def _standalone_compiler_build_env(project: str, *, dry: bool) -> dict[str, str]:
    prepared = _prepare_standalone_compiler_bundle(project, dry=dry)
    if prepared is None:
        return {}
    catalog_sha256 = prepared.catalog_sha256
    if catalog_sha256 is None:
        if not dry:
            raise SystemExit(
                "prepared standalone compiler catalog has no SHA-256 authority"
            )
        catalog_sha256 = "<prepared-catalog-sha256>"
    return {
        "GORE_STANDALONE_COMPILER_CATALOG_PATH": str(prepared.catalog_path),
        "GORE_STANDALONE_COMPILER_CATALOG_SHA256": catalog_sha256,
    }


def _verify_host_embedded_standalone_compiler_catalog(
    project: str, host_artifact: Path, *, dry: bool
) -> None:
    """Prove the linked host reports exactly the catalog prepared for this build."""

    prepared = _prepare_standalone_compiler_bundle(project, dry=dry)
    if prepared is None or dry:
        return
    if prepared.catalog_sha256 is None:
        raise SystemExit(
            "prepared standalone compiler catalog has no SHA-256 authority"
        )
    try:
        host_bytes = standalone_compiler_bundle._read_pinned_windows_regular(
            host_artifact,
            1024 * 1024 * 1024,
            "standalone compiler product host",
            require_single_link=False,
        )
    except (OSError, standalone_compiler_bundle.BundleError) as error:
        raise SystemExit(
            f"cannot inspect linked standalone compiler host: {error}"
        ) from error
    prefix = b"GORE_AS_EMBEDDED_COMPILER_CATALOG_SHA256="
    reported = {
        match.group(1).decode("ascii").casefold()
        for match in re.finditer(prefix + rb"([0-9a-fA-F]{64})", host_bytes)
    }
    if reported != {prepared.catalog_sha256.casefold()}:
        raise SystemExit(
            "built host does not report exactly the prepared standalone compiler catalog "
            f"SHA-256 (expected {prepared.catalog_sha256}, found {sorted(reported)})"
        )


def _verify_staged_product_host_catalogs(
    project: str, staging_root: Path, *, dry: bool
) -> None:
    if dry or not PROJECTS[project].get("standalone_compiler_bundle"):
        return
    cfg = PROJECTS[project]
    if cfg["kind"] == "rust-bin":
        names = [f"{cfg['bin']}.exe"]
    else:
        names = [f"{cfg['core_dll']}.dll"]
        names.extend(
            f"{PROJECTS[companion]['bin']}.exe"
            for companion in cfg.get("companions", ())
        )
    for name in names:
        _verify_host_embedded_standalone_compiler_catalog(
            project, staging_root / name, dry=False
        )


def _stage_standalone_compiler_bundle(
    project: str, destination_parent: Path, *, dry: bool
) -> standalone_compiler_bundle.PreparedBundle | None:
    prepared = _prepare_standalone_compiler_bundle(project, dry=dry)
    if prepared is None:
        return None
    if dry:
        action = (
            "stage qualified bytes" if prepared.present else "remove stale compiler/"
        )
        print(f"[dry-run] would {action} at {destination_parent}")
        return prepared
    try:
        standalone_compiler_bundle.stage_product_bundle(
            prepared,
            destination_parent,
            qualified_profile_verifier=(
                _qualified_profile_verifier(dry=False) if prepared.present else None
            ),
            promotion_attestation_verifier=(
                _product_promotion_attestation_verifier(dry=False)
                if prepared.present
                else None
            ),
        )
    except standalone_compiler_bundle.BundleError as error:
        raise SystemExit(f"standalone compiler staging failed: {error}") from error
    return prepared


def _standalone_compiler_signing_exclusions(
    project: str, *, dry: bool
) -> tuple[str, ...]:
    prepared = _prepare_standalone_compiler_bundle(project, dry=dry)
    return prepared.signing_exclusions if prepared is not None else ()


def _verify_staged_standalone_compiler_bundle(
    project: str, destination_parent: Path, *, dry: bool
) -> None:
    prepared = _prepare_standalone_compiler_bundle(project, dry=dry)
    if prepared is None or dry:
        return
    bundle_root = destination_parent / "compiler"
    if not prepared.present:
        if bundle_root.exists() or bundle_root.is_symlink():
            raise SystemExit(
                "BundleAbsent packaging retained a stale compiler directory"
            )
        return
    try:
        verified = standalone_compiler_bundle.verify_staged_bundle(
            bundle_root,
            qualified_profile_verifier=_qualified_profile_verifier(dry=False),
            promotion_attestation_verifier=_product_promotion_attestation_verifier(
                dry=False
            ),
        )
    except standalone_compiler_bundle.BundleError as error:
        raise SystemExit(
            f"staged standalone compiler verification failed: {error}"
        ) from error
    if hashlib.sha256(verified.catalog_bytes).hexdigest() != prepared.catalog_sha256:
        raise SystemExit(
            "staged standalone compiler catalog differs from the host build"
        )


_PE_MACHINE_AMD64 = 0x8664
_MSVC_RUNTIME_NAME_RE = re.compile(
    r"^(?:msvcp|msvcr|vcruntime|concrt|vcomp)[a-z0-9_]*\.dll$", re.IGNORECASE
)


@dataclass(frozen=True)
class _AppLocalRuntimeFile:
    name: str
    source: Path
    sha256: str


@dataclass(frozen=True)
class _AppLocalRuntimePlan:
    files: tuple[_AppLocalRuntimeFile, ...]

    @property
    def names(self) -> tuple[str, ...]:
        return tuple(item.name for item in self.files)


def _sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _pe_machine(path: Path) -> int:
    """Read a PE machine id without loading or executing the binary."""
    try:
        size = path.stat().st_size
        with path.open("rb") as stream:
            if size < 0x40 or stream.read(2) != b"MZ":
                raise ValueError("missing DOS header")
            stream.seek(0x3C)
            pe_offset = struct.unpack("<I", stream.read(4))[0]
            if pe_offset > size - 6:
                raise ValueError("PE header is outside the file")
            stream.seek(pe_offset)
            if stream.read(4) != b"PE\0\0":
                raise ValueError("missing PE signature")
            return struct.unpack("<H", stream.read(2))[0]
    except (OSError, struct.error, ValueError) as error:
        raise SystemExit(f"invalid PE runtime file {path}: {error}") from error


def _cmake_cache_value(project: str, key: str) -> str:
    cache = pdir(project) / "build" / "windows" / "x64" / "CMakeCache.txt"
    try:
        lines = cache.read_text(encoding="utf-8").splitlines()
    except OSError as error:
        raise SystemExit(f"cannot read Flutter CMake cache {cache}: {error}") from error
    prefix = f"{key}:"
    values = [
        line.split("=", 1)[1]
        for line in lines
        if line.startswith(prefix) and "=" in line
    ]
    if len(values) != 1 or not values[0].strip():
        raise SystemExit(f"Flutter CMake cache must contain one {key}: {cache}")
    return values[0].strip()


def _msvc_crt_dir(vc_root: Path, redist_version: str) -> Path:
    x64_dir = vc_root / "Redist" / "MSVC" / redist_version / "x64"
    try:
        entries = tuple(x64_dir.iterdir())
    except OSError as error:
        raise SystemExit(
            f"cannot inspect matching x64 MSVC redist {x64_dir}: {error}"
        ) from error

    candidates: list[Path] = []
    for entry in entries:
        if re.fullmatch(r"Microsoft\.VC[0-9]+\.CRT", entry.name, re.IGNORECASE) is None:
            continue
        if entry.is_symlink() or (
            hasattr(entry, "is_junction") and entry.is_junction()
        ):
            continue
        if entry.is_dir():
            candidates.append(entry)
    if len(candidates) != 1:
        names = sorted(entry.name for entry in candidates)
        raise SystemExit(
            "matching x64 MSVC redist must contain exactly one CRT family; "
            f"found {names} under {x64_dir}"
        )
    return candidates[0]


def _msvc_runtime_sources(
    project: str, runtime_names: tuple[str, ...]
) -> tuple[Path, tuple[_AppLocalRuntimeFile, ...]]:
    platform = _cmake_cache_value(project, "CMAKE_GENERATOR_PLATFORM")
    if platform.casefold() != "x64":
        raise SystemExit(
            f"{project} release runtime requires x64 CMake output, got {platform!r}"
        )

    linker = Path(_cmake_cache_value(project, "CMAKE_LINKER"))
    if linker.name.casefold() != "link.exe" or not linker.is_file():
        raise SystemExit(
            f"CMake linker does not identify an installed MSVC toolchain: {linker}"
        )
    dumpbin = linker.with_name("dumpbin.exe")
    if not dumpbin.is_file():
        raise SystemExit(
            f"dumpbin.exe missing beside the selected CMake linker: {dumpbin}"
        )

    vc_root: Path | None = None
    for parent in linker.parents:
        if parent.name.casefold() != "vc":
            continue
        try:
            linker.relative_to(parent / "Tools" / "MSVC")
        except ValueError:
            continue
        vc_root = parent
        break
    if vc_root is None:
        raise SystemExit(f"CMake linker is not under VC/Tools/MSVC: {linker}")

    version_file = (
        vc_root / "Auxiliary" / "Build" / "Microsoft.VCRedistVersion.default.txt"
    )
    try:
        redist_version = version_file.read_text(encoding="utf-8-sig").strip()
    except OSError as error:
        raise SystemExit(
            f"cannot read matching MSVC redist version {version_file}: {error}"
        ) from error
    if re.fullmatch(r"[0-9]+(?:\.[0-9]+){2,3}", redist_version) is None:
        raise SystemExit(
            f"invalid MSVC redist version in {version_file}: {redist_version!r}"
        )

    crt_dir = _msvc_crt_dir(vc_root, redist_version)
    try:
        resolved_x64_dir = crt_dir.parent.resolve(strict=True)
        resolved_crt_dir = crt_dir.resolve(strict=True)
    except OSError as error:
        raise SystemExit(
            f"cannot resolve x64 MSVC redist directory: {crt_dir}"
        ) from error
    if resolved_crt_dir.parent != resolved_x64_dir:
        raise SystemExit(
            f"x64 MSVC runtime escapes its matching Redist directory: {crt_dir}"
        )

    files: list[_AppLocalRuntimeFile] = []
    for name in runtime_names:
        source = crt_dir / name
        if source.is_symlink() or not source.is_file():
            raise SystemExit(
                f"required app-local MSVC runtime missing or not a regular file: {source}"
            )
        try:
            resolved_source = source.resolve(strict=True)
        except OSError as error:
            raise SystemExit(
                f"cannot resolve app-local MSVC runtime {source}: {error}"
            ) from error
        if resolved_source.parent != resolved_crt_dir:
            raise SystemExit(
                f"app-local MSVC runtime escapes its Redist directory: {source}"
            )
        machine = _pe_machine(resolved_source)
        if machine != _PE_MACHINE_AMD64:
            raise SystemExit(
                f"app-local MSVC runtime must be x64 PE (0x8664), got 0x{machine:04x}: {source}"
            )
        files.append(
            _AppLocalRuntimeFile(
                name=name,
                source=resolved_source,
                sha256=_sha256(resolved_source),
            )
        )
    return dumpbin, tuple(files)


def _dumpbin_msvc_imports(dumpbin: Path, paths: list[Path]) -> set[str]:
    imports: set[str] = set()
    for path in paths:
        try:
            completed = subprocess.run(
                [str(dumpbin), "/nologo", "/dependents", str(path)],
                capture_output=True,
                text=True,
                errors="replace",
                check=False,
            )
        except OSError as error:
            raise SystemExit(
                f"cannot inspect PE dependencies for {path}: {error}"
            ) from error
        if completed.returncode != 0:
            detail = (completed.stderr or completed.stdout).strip()
            raise SystemExit(
                f"dumpbin failed for {path}: {detail or completed.returncode}"
            )
        for line in (completed.stdout + "\n" + completed.stderr).splitlines():
            candidate = line.strip()
            if _MSVC_RUNTIME_NAME_RE.fullmatch(candidate):
                imports.add(candidate.casefold())
    return imports


def _prepare_app_local_runtime(
    project: str, bundle_dir: Path
) -> _AppLocalRuntimePlan | None:
    configured = PROJECTS[project].get("app_local_msvc_runtime")
    if configured is None:
        return None
    runtime_names = tuple(str(name).casefold() for name in configured)
    expected = set(runtime_names)
    if len(expected) != len(runtime_names) or any(
        _MSVC_RUNTIME_NAME_RE.fullmatch(name) is None for name in runtime_names
    ):
        raise SystemExit(f"invalid app-local MSVC runtime contract for {project}")
    if not bundle_dir.is_dir():
        raise SystemExit(f"Windows release bundle missing: {bundle_dir}")

    dumpbin, files = _msvc_runtime_sources(project, runtime_names)
    pe_paths: list[Path] = []
    for path in sorted(bundle_dir.rglob("*")):
        name = path.name.casefold()
        if _MSVC_RUNTIME_NAME_RE.fullmatch(name):
            if name not in expected or path.parent != bundle_dir:
                raise SystemExit(f"unexpected app-local MSVC runtime in bundle: {path}")
            if path.is_symlink() or not path.is_file():
                raise SystemExit(
                    f"existing app-local MSVC runtime is not a regular file: {path}"
                )
            continue
        if path.is_file() and path.suffix.casefold() in (".exe", ".dll"):
            pe_paths.append(path)

    imports = _dumpbin_msvc_imports(dumpbin, pe_paths + [item.source for item in files])
    if imports != expected:
        missing = sorted(expected - imports)
        unexpected = sorted(imports - expected)
        raise SystemExit(
            "app-local MSVC runtime closure changed; "
            f"missing imports={missing}, unexpected imports={unexpected}"
        )
    return _AppLocalRuntimePlan(files=files)


def _verify_staged_runtime(bundle_dir: Path, plan: _AppLocalRuntimePlan) -> None:
    expected = {item.name.casefold(): item for item in plan.files}
    seen: dict[str, Path] = {}
    for path in bundle_dir.rglob("*"):
        name = path.name.casefold()
        if _MSVC_RUNTIME_NAME_RE.fullmatch(name) is None:
            continue
        if name not in expected or path.parent != bundle_dir or name in seen:
            raise SystemExit(
                f"unexpected app-local MSVC runtime in finalized bundle: {path}"
            )
        if path.is_symlink() or not path.is_file():
            raise SystemExit(
                f"finalized app-local MSVC runtime is not a regular file: {path}"
            )
        if _pe_machine(path) != _PE_MACHINE_AMD64:
            raise SystemExit(f"finalized app-local MSVC runtime is not x64: {path}")
        if _sha256(path) != expected[name].sha256:
            raise SystemExit(f"finalized app-local MSVC runtime was modified: {path}")
        seen[name] = path
    missing = sorted(set(expected) - set(seen))
    if missing:
        raise SystemExit(
            f"finalized bundle is missing app-local MSVC runtime: {missing}"
        )


def _stage_runtime_atomically(bundle_dir: Path, plan: _AppLocalRuntimePlan) -> None:
    with tempfile.TemporaryDirectory(
        prefix=".gore-runtime-", dir=bundle_dir.parent
    ) as scratch_name:
        scratch = Path(scratch_name)
        prepared = scratch / "prepared"
        backups = scratch / "backups"
        prepared.mkdir()
        backups.mkdir()

        # Copy and verify every source outside the packaged tree before replacing
        # any existing bundle file.
        for item in plan.files:
            temp_path = prepared / item.name
            shutil.copy2(item.source, temp_path)
            if (
                _pe_machine(temp_path) != _PE_MACHINE_AMD64
                or _sha256(temp_path) != item.sha256
            ):
                raise SystemExit(
                    f"copied app-local MSVC runtime did not verify: {item.source}"
                )

        moved_backups: list[tuple[Path, Path]] = []
        installed: list[Path] = []
        try:
            for item in plan.files:
                target = bundle_dir / item.name
                if target.exists():
                    backup = backups / item.name
                    os.replace(target, backup)
                    moved_backups.append((target, backup))
            for item in plan.files:
                target = bundle_dir / item.name
                os.replace(prepared / item.name, target)
                installed.append(target)
            _verify_staged_runtime(bundle_dir, plan)
        except BaseException as error:
            rollback_errors: list[str] = []
            for target in installed:
                try:
                    target.unlink(missing_ok=True)
                except OSError as rollback_error:
                    rollback_errors.append(f"remove {target}: {rollback_error}")
            for target, backup in moved_backups:
                if not backup.exists():
                    continue
                try:
                    os.replace(backup, target)
                except OSError as rollback_error:
                    rollback_errors.append(f"restore {target}: {rollback_error}")
            if rollback_errors:
                raise RuntimeError(
                    "app-local MSVC runtime rollback failed: "
                    + "; ".join(rollback_errors)
                ) from error
            raise
        print(f"bundled app-local MSVC runtime -> {bundle_dir}")


def _sign_and_stage_app_local_runtime(
    project: str,
    bundle_dir: Path,
    dry: bool,
    exclude_names: tuple[str, ...] = (),
) -> _AppLocalRuntimePlan | None:
    runtime_names = tuple(PROJECTS[project].get("app_local_msvc_runtime", ()))
    if not runtime_names:
        sign_dir(bundle_dir, dry=dry, exclude_names=exclude_names)
        return None
    if dry:
        sign_dir(
            bundle_dir,
            dry=True,
            exclude_names=tuple(dict.fromkeys((*runtime_names, *exclude_names))),
        )
        print(f"[dry-run] would stage app-local MSVC runtime into {bundle_dir}")
        return None

    # Validate the entire app + CRT import closure before signing anything.
    plan = _prepare_app_local_runtime(project, bundle_dir)
    if plan is None:
        raise SystemExit(f"missing app-local MSVC runtime plan for {project}")
    # Microsoft ships these Redist DLLs already signed. Preserve their exact
    # bytes by signing GORE-owned PEs first, then atomically replacing the DLLs.
    sign_dir(
        bundle_dir,
        dry=False,
        exclude_names=tuple(dict.fromkeys((*plan.names, *exclude_names))),
    )
    _stage_runtime_atomically(bundle_dir, plan)
    return plan


def _verify_runtime_zip(archive: Path, plan: _AppLocalRuntimePlan) -> None:
    expected = {item.name.casefold(): item for item in plan.files}
    seen: set[str] = set()
    try:
        with zipfile.ZipFile(archive) as package:
            for info in package.infolist():
                basename = Path(info.filename).name.casefold()
                if _MSVC_RUNTIME_NAME_RE.fullmatch(basename) is None:
                    continue
                if (
                    basename not in expected
                    or info.filename.replace("\\", "/") != basename
                    or basename in seen
                ):
                    raise SystemExit(
                        f"unexpected app-local MSVC runtime in zip: {info.filename}"
                    )
                digest = hashlib.sha256()
                with package.open(info) as stream:
                    for chunk in iter(lambda: stream.read(1024 * 1024), b""):
                        digest.update(chunk)
                if digest.hexdigest() != expected[basename].sha256:
                    raise SystemExit(
                        f"modified app-local MSVC runtime in zip: {info.filename}"
                    )
                seen.add(basename)
    except (OSError, zipfile.BadZipFile) as error:
        raise SystemExit(
            f"cannot verify packaged app-local MSVC runtime {archive}: {error}"
        ) from error
    missing = sorted(set(expected) - seen)
    if missing:
        raise SystemExit(f"portable zip is missing app-local MSVC runtime: {missing}")


def stage_companions(project: str, dry: bool) -> None:
    """Build sibling CLI binaries and drop them into the Flutter Release dir.

    A Flutter project may declare `companions: [<project>, ...]` to ship another
    project's release binary (and its bundled data dirs) beside the app. Injecting
    into the Release dir means both dist paths pick it up for free: the installer
    packages from SourceDir=Release, and the portable zip is copied from Release.
    Used to bundle the `gore` CLI with mod-studio.
    """
    cfg = PROJECTS[project]
    companions = cfg.get("companions", [])
    if not companions:
        return
    rel = flutter_release_dir(project)
    for dep in companions:
        dcfg = PROJECTS[dep]
        run(
            f"cargo build companion {dep}",
            [CARGO, "build", "-p", dcfg["crate"], "--release"],
            dry=dry,
            extra_env=_standalone_compiler_build_env(project, dry=dry),
        )
        if dry:
            print(f"[dry-run] would bundle {dep} into {rel}")
            continue
        exe = target_dir(True) / f"{dcfg['bin']}.exe"
        if not exe.exists():
            raise SystemExit(f"missing companion binary: {exe}")
        _verify_host_embedded_standalone_compiler_catalog(project, exe, dry=False)
        shutil.copy2(exe, rel / exe.name)
        _verify_host_embedded_standalone_compiler_catalog(
            project, rel / exe.name, dry=False
        )
        print(f"bundled companion {exe.name} -> {rel / exe.name}")
        # Mirror the companion's own bundled data dirs (e.g. gore's lua/shared
        # SDK), so the bundled CLI behaves identically to the standalone one.
        for src_rel, dest_name in dcfg.get("bundle_dirs", []):
            src_dir = ROOT / src_rel
            if not src_dir.is_dir():
                raise SystemExit(f"missing companion bundle dir: {src_dir}")
            dest = rel / dest_name
            if dest.exists():
                shutil.rmtree(dest)
            shutil.copytree(src_dir, dest)
            print(f"bundled companion dir {src_rel} -> {dest}")


def build_project(project: str, release: bool, dry: bool) -> None:
    cfg = PROJECTS[project]
    mode = "release" if release else "debug"
    if cfg["kind"] in ("rust-bin", "rust-lib"):
        cmd = [CARGO, "build", "-p", cfg["crate"]]
        if release:
            cmd.append("--release")
        run(
            f"cargo build {project} ({mode})",
            cmd,
            dry=dry,
            extra_env=_standalone_compiler_build_env(project, dry=dry),
        )
        if not dry:
            _verify_host_embedded_standalone_compiler_catalog(
                project,
                target_dir(release) / f"{cfg['bin']}.exe",
                dry=False,
            )
        return
    # flutter app: build native cdylib first, then the app, then bundle the dll.
    # The cargo package id (hyphenated) and the produced dll basename
    # (underscored) differ, so they are tracked as separate fields.
    crate = cfg["core_crate"]
    cargo_cmd = [CARGO, "build", "-p", crate]
    if release:
        cargo_cmd.append("--release")
    run(
        f"cargo build {crate} ({mode})",
        cargo_cmd,
        dry=dry,
        extra_env=_standalone_compiler_build_env(project, dry=dry),
    )
    if not dry:
        _verify_host_embedded_standalone_compiler_catalog(
            project,
            target_dir(release) / f"{cfg['core_dll']}.dll",
            dry=False,
        )

    flutter_cmd = [FLUTTER, "build", "windows", f"--{mode}"]
    # Every Flutter app's About dialog reads GIT_SHA (String.fromEnvironment, defaultValue 'dev').
    # Only rust projects returned early above, so every project reaching here is a Flutter app —
    # supply the SHA to all of them, or released mod-studio / mod-manager binaries show 'dev'.
    flutter_cmd.append(f"--dart-define=GIT_SHA={resolve_git_sha()}")
    run(f"flutter build {project} ({mode})", flutter_cmd, cwd=pdir(project), dry=dry)

    if dry:
        return
    # The build rewrote the generated l10n sources; put back the ones that differ in
    # line endings alone, so a build does not leave a worktree that reads as dirty
    # (see discard_line_ending_only_churn).
    discard_line_ending_only_churn(project)
    rel = flutter_build_dir(project, release)
    if not rel.exists():
        raise SystemExit(f"missing flutter output: {rel}")
    dll = f"{cfg['core_dll']}.dll"
    src = target_dir(release) / dll
    if not src.exists():
        raise SystemExit(f"missing native artifact: {src}")
    shutil.copy2(src, rel / dll)
    _verify_host_embedded_standalone_compiler_catalog(project, rel / dll, dry=False)
    print(f"copied {dll} -> {rel / dll}")


def runnable_exe(project: str, release: bool) -> Path:
    cfg = PROJECTS[project]
    if cfg["kind"] == "flutter":
        return flutter_build_dir(project, release) / cfg["exe"]
    if cfg["kind"] == "rust-bin":
        return target_dir(release) / f"{cfg['bin']}.exe"
    raise SystemExit(f"{project} is not runnable")


def run_project(project: str, release: bool) -> None:
    """Launch the built program, building it first if it is missing."""
    exe = runnable_exe(project, release)
    if not exe.exists():
        mode = "release" if release else "debug"
        print(f"{exe.name} ({mode}) not built yet; building first...")
        build_project(project, release=release, dry=False)
    if not exe.exists():
        raise SystemExit(f"build did not produce {exe}")
    # Run from the exe's own directory so a Flutter app finds its bundled DLLs.
    if PROJECTS[project]["kind"] == "flutter":
        # A GUI app would otherwise block the build script until the window is
        # closed. Launch it detached and return immediately.
        print(f"\nlaunching {exe} (background)")
        subprocess.Popen([str(exe)], cwd=exe.parent, env=env())
        return
    # CLI: wait and propagate the program's own exit code rather than framing a
    # nonzero exit as a build failure (a CLI may exit nonzero, e.g. usage with
    # no args).
    print(f"\nlaunching {exe}")
    completed = subprocess.run([str(exe)], cwd=exe.parent, env=env())
    raise SystemExit(completed.returncode)


def dist_project(project: str, dry: bool) -> Path | None:
    """Release-build and package into dist/. Returns the zip path."""
    cfg = PROJECTS[project]
    if not cfg.get("releasable"):
        raise SystemExit(f"{project} is not releasable")
    build_project(project, release=True, dry=dry)
    # Drop any declared companion binaries (e.g. the `gore` CLI for mod-studio)
    # into the Release dir before it is packaged / handed to the installer.
    stage_companions(project, dry=dry)
    if cfg["kind"] == "flutter":
        # The catalog was generated and embedded before the Rust host DLL/CLI build. Stage the
        # exact same immutable bytes beside the finished host only after compilation.
        _stage_standalone_compiler_bundle(
            project, flutter_release_dir(project), dry=dry
        )
    version = read_version(project)
    dist = dist_dir(project)
    if not dry:
        dist.mkdir(parents=True, exist_ok=True)
    base = dist / zip_basename(project, version)

    if cfg["kind"] == "flutter":
        rel = flutter_release_dir(project)
        license_file = ROOT / "LICENSE"
        if dry:
            print(f"[dry-run] would zip {rel} -> {base}.zip (minus WinSparkle.dll)")
            return None
        # Stage a copy so the portable zip can omit files without touching the
        # shared Release dir that the Inno installer packages from.
        staging = dist / "_stage"
        if staging.exists():
            shutil.rmtree(staging)
        try:
            shutil.copytree(rel, staging)
            if license_file.exists():
                shutil.copy2(license_file, staging / "LICENSE")
            third_party = ROOT / "THIRD_PARTY_LICENSES.md"
            if third_party.exists():
                shutil.copy2(third_party, staging / "THIRD_PARTY_LICENSES.md")
            # The auto-updater DLLs are false-positive virus magnets that NexusMods
            # quarantines. The portable build never calls the updater (it is gated
            # to Inno-installed copies); the runner delay-loads the plugin and stubs
            # out its registration when absent (see windows/runner/updater_delayload
            # .cpp), so both DLLs can be dropped without a load-time crash. Installer
            # builds still bundle them straight from `rel`.
            for dll_name in ("auto_updater_windows_plugin.dll", "WinSparkle.dll"):
                dll = staging / dll_name
                if dll.exists():
                    dll.unlink()
                    print(f"dropped {dll_name} from portable zip")
            # Sign the staged PE files before zipping so the portable archive ships
            # signed binaries (this is the build NexusMods scans on upload).
            runtime_plan = _sign_and_stage_app_local_runtime(
                project,
                staging,
                dry=dry,
                exclude_names=_standalone_compiler_signing_exclusions(project, dry=dry),
            )
            _verify_staged_product_host_catalogs(project, staging, dry=dry)
            _verify_staged_standalone_compiler_bundle(project, staging, dry=dry)
            final_archive = base.parent / f"{base.name}.zip"
            if runtime_plan is None:
                if final_archive.exists():
                    final_archive.unlink()
                archive = Path(shutil.make_archive(str(base), "zip", root_dir=staging))
            else:
                # Keep any previous release artifact intact until the candidate
                # archive has passed the Manager runtime contract.
                with tempfile.TemporaryDirectory(
                    prefix=".gore-package-", dir=dist
                ) as scratch_name:
                    candidate = Path(
                        shutil.make_archive(
                            str(Path(scratch_name) / base.name),
                            "zip",
                            root_dir=staging,
                        )
                    )
                    _verify_runtime_zip(candidate, runtime_plan)
                    os.replace(candidate, final_archive)
                archive = final_archive
            print(f"\npackaged: {archive}")
            return archive
        finally:
            if staging.exists():
                shutil.rmtree(staging)

    # rust-bin: zip the exe + LICENSE
    exe = target_dir(True) / f"{cfg['bin']}.exe"
    if not dry and not exe.exists():
        raise SystemExit(f"missing binary: {exe}")
    if dry:
        print(f"[dry-run] would zip {exe} -> {base}.zip")
        return None
    staging = dist / "_stage"
    if staging.exists():
        shutil.rmtree(staging)
    staging.mkdir(parents=True)
    shutil.copy2(exe, staging / exe.name)
    _stage_standalone_compiler_bundle(project, staging, dry=dry)
    if (ROOT / "LICENSE").exists():
        shutil.copy2(ROOT / "LICENSE", staging / "LICENSE")
    if (ROOT / "THIRD_PARTY_LICENSES.md").exists():
        shutil.copy2(
            ROOT / "THIRD_PARTY_LICENSES.md", staging / "THIRD_PARTY_LICENSES.md"
        )
    # stage any bundled data dirs beside the exe (e.g. gore's lua/shared SDK)
    for src_rel, dest_name in cfg.get("bundle_dirs", []):
        src_dir = ROOT / src_rel
        if not src_dir.is_dir():
            raise SystemExit(f"missing bundle dir: {src_dir}")
        shutil.copytree(src_dir, staging / dest_name)
    # stage the Markdown guide beside the exe, with out-of-tree links absolutized
    for src_rel, dest_name in cfg.get("doc_dirs", []):
        src_dir = ROOT / src_rel
        if not src_dir.is_dir():
            raise SystemExit(f"missing doc dir: {src_dir}")
        written = stage_docs(src_dir, staging / dest_name)
        if not written:
            raise SystemExit(f"no docs found in {src_dir}")
        print(f"staged {written} doc file(s) from {src_rel} -> {dest_name}/")
    # Render the browsable guide with the binary we just built, so it can never disagree
    # with the pages compiled into it. Pinned to the same commit as the Markdown copies.
    guide_html = cfg.get("guide_html")
    if guide_html:
        rendered = staging / guide_html
        rendered.parent.mkdir(parents=True, exist_ok=True)
        run(
            "render browsable guide",
            [
                staging / exe.name,
                "guide",
                "html",
                "-o",
                rendered,
                "--repo-ref",
                repo_ref(),
            ],
        )
        if not rendered.is_file():
            raise SystemExit(f"guide render produced nothing at {rendered}")
    sign_dir(
        staging,
        dry=dry,
        exclude_names=_standalone_compiler_signing_exclusions(project, dry=dry),
    )
    _verify_staged_product_host_catalogs(project, staging, dry=dry)
    _verify_staged_standalone_compiler_bundle(project, staging, dry=dry)
    if base.with_suffix(".zip").exists():
        base.with_suffix(".zip").unlink()
    archive = shutil.make_archive(str(base), "zip", root_dir=staging)
    shutil.rmtree(staging)
    print(f"\npackaged: {archive}")
    return Path(archive)


def installer_project(project: str, dry: bool) -> Path | None:
    cfg = PROJECTS[project]
    if "installer" not in cfg:
        raise SystemExit(f"{project} has no installer")
    dist_project(project, dry=dry)
    version = read_version(project)
    rel = flutter_release_dir(project)
    dist = dist_dir(project)
    iss = pdir(project) / cfg["installer"]
    # dist_project signed the staging copy for the zip; the installer packages
    # from the Release dir, so sign those PE files too before Inno bundles them.
    _sign_and_stage_app_local_runtime(
        project,
        rel,
        dry=dry,
        exclude_names=_standalone_compiler_signing_exclusions(project, dry=dry),
    )
    _verify_staged_product_host_catalogs(project, rel, dry=dry)
    _verify_staged_standalone_compiler_bundle(project, rel, dry=dry)
    base_args: list[Path | str] = [
        ISCC,
        "/Qp",
        f"/DAppVersion={version}",
        f"/DSourceDir={rel}",
        f"/DOutputDir={dist}",
        # Passed in rather than hardcoded per .iss, so the produced file name
        # and the path this function returns cannot drift apart.
        f"/DOutputBaseName={installer_basename(project, version)}",
    ]

    inno_tool = cfg.get("inno_sign_tool")
    signing_cfg = _signing_config() if inno_tool is not None else None
    inno_signs_installer = inno_tool is not None and signing_cfg is not None
    if inno_signs_installer and dry:
        print("[dry-run] would configure Inno to sign Setup and Uninstall")
        run(f"installer {project}", [*base_args, iss], dry=True)
    elif inno_signs_installer:
        assert signing_cfg is not None
        dlib = _ensure_dlib()
        signtool = _find_signtool()
        meta = _write_metadata(signing_cfg)
        try:
            with tempfile.TemporaryDirectory(prefix="gore-inno-uninstaller-") as cache:
                # Reject drift before starting Inno. Its configured wrapper
                # repeats this check immediately before every signtool load.
                _verify_trusted_signing_runtime(dlib.parent)
                sign_command = _inno_trusted_signing_command(signtool, dlib, meta)
                run(
                    f"installer {project}",
                    [
                        *base_args,
                        f"/S{inno_tool}={sign_command}",
                        "/DGORE_SIGNED_INSTALLER=1",
                        f"/DGORE_SIGNED_UNINSTALLER_DIR={cache}",
                        iss,
                    ],
                    dry=False,
                    extra_env=_sign_proxy_overrides(),
                )
        finally:
            meta.unlink(missing_ok=True)
    else:
        run(f"installer {project}", [*base_args, iss], dry=dry)
    out = dist / f"{installer_basename(project, version)}.exe"
    if inno_tool is None:
        # Products without integrated Inno signing retain the existing outer
        # Setup signing path. Manager's signed path was already signed by Inno;
        # signing it again here would append a redundant Authenticode signature.
        sign_paths([out], dry=dry)
    print(f"installer: {out}")
    return out


def test_project(project: str, dry: bool) -> None:
    cfg = PROJECTS[project]
    if cfg["kind"] in ("rust-bin", "rust-lib"):
        run(f"cargo test {project}", [CARGO, "test", "-p", cfg["crate"]], dry=dry)
        return
    # A Flutter app is backed by a native Rust cdylib (core_dll crate); its
    # unit tests live there, so cover them too — analyze/test alone would skip
    # all the native logic the app depends on.
    run(
        f"cargo test {cfg['core_crate']}",
        [CARGO, "test", "-p", cfg["core_crate"]],
        dry=dry,
    )
    app = pdir(project)
    run(f"flutter pub get {project}", [FLUTTER, "pub", "get"], cwd=app, dry=dry)
    run(f"flutter analyze {project}", [FLUTTER, "analyze"], cwd=app, dry=dry)
    run(f"flutter test {project}", [FLUTTER, "test"], cwd=app, dry=dry)


# --------------------------------------------------------------------------- #
# Release pipeline                                                            #
# --------------------------------------------------------------------------- #
def git(args: list[str], dry: bool) -> None:
    run(f"git {args[0]}", ["git", *args], dry=dry)


def release_project(project: str, version: str, steps: dict, dry: bool) -> None:
    cfg = PROJECTS[project]
    if not cfg.get("releasable"):
        raise SystemExit(f"{project} is not releasable")
    if not VERSION_RE.match(version):
        raise SystemExit(f"version must be X.Y.Z, got {version!r}")
    prefix = cfg["tag_prefix"]
    tag = f"{prefix}-v{version}"

    if steps["bump"]:
        write_version(project, version, dry)
    if steps["changelog"]:
        notes = changelog_notes(project, version)
        print(f"\nCHANGELOG ok for {version}:\n{notes[:200]}...")
    if steps["build"]:
        build_project(project, release=True, dry=dry)
    if steps["pack"]:
        dist_project(project, dry=dry)
    if steps["installer"]:
        installer_project(project, dry=dry)
    if steps["tag"]:
        # The tag must match the manifest version, or CI's version-verify step
        # rejects the build and leaves a stray remote tag. --bump already wrote
        # it; without --bump, confirm the manifest is already at this version
        # so a mistyped version argument fails before anything is tagged.
        if not steps["bump"]:
            actual = read_version(project)
            if actual != version:
                raise SystemExit(
                    f"manifest version {actual} != requested {version}; "
                    "re-run with --bump to set it, or fix the version argument"
                )
        manifest = cfg["pubspec"] if cfg["kind"] == "flutter" else cfg["manifest"]
        rel_paths = [f"{cfg['dir']}/{manifest}"]
        if cfg.get("changelog"):
            rel_paths.append(f"{cfg['dir']}/{cfg['changelog']}")
        # The bump also rewrites the root workspace lockfile (see write_version).
        if cfg["kind"] in ("rust-bin", "rust-lib"):
            rel_paths.append("Cargo.lock")
        # Commit only this project's files via pathspec, so unrelated staged
        # work never rides along on the release tag. Skip the commit entirely
        # when those files are already at the target version (nothing to
        # commit would otherwise abort and the tag would never be created).
        if dry:
            changed = True
        else:
            changed = (
                subprocess.run(
                    ["git", "diff", "--quiet", "HEAD", "--", *rel_paths], cwd=ROOT
                ).returncode
                != 0
            )
        if changed:
            git(
                ["commit", "-m", f"release({project}): {version}", "--", *rel_paths],
                dry,
            )
        else:
            print(f"no changes in {rel_paths}; skipping release commit")
        git(["tag", tag], dry)
    if steps["push"]:
        git(["push"], dry)
        git(["push", "origin", tag], dry)
        print(f"\npushed {tag} -> CI Release workflow will build {project}")


# --------------------------------------------------------------------------- #
# CLI                                                                         #
# --------------------------------------------------------------------------- #
def expand_targets(target: str, for_release: bool) -> list[str]:
    if target == "all":
        names = RELEASE_ORDER if for_release else list(PROJECTS)
        return [
            n for n in names if PROJECTS[n].get("releasable", True) or not for_release
        ]
    if target not in PROJECTS:
        raise SystemExit(
            f"unknown project {target!r}; choices: {', '.join(PROJECTS)}, all"
        )
    return [target]


def main() -> int:
    p = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    p.add_argument("target", help="project name or 'all'")
    sub = p.add_subparsers(dest="command", required=True)

    b = sub.add_parser("build", help="compile")
    b.add_argument("--release", action="store_true")
    b.add_argument("--debug", action="store_true")
    b.add_argument("--run", action="store_true", help="launch right after building")

    rn = sub.add_parser("run", help="launch the built program (builds if missing)")
    rn.add_argument("--release", action="store_true")
    rn.add_argument("--debug", action="store_true")

    sub.add_parser("dist", help="build + package")
    sub.add_parser("installer", help="dist + installer exe")
    sub.add_parser("test", help="run tests")

    trusted_sign_one = sub.add_parser("trusted-sign-one", help=argparse.SUPPRESS)
    trusted_sign_one.add_argument("--signtool", required=True, type=Path)
    trusted_sign_one.add_argument("--dlib", required=True, type=Path)
    trusted_sign_one.add_argument("--metadata", required=True, type=Path)
    trusted_sign_one.add_argument("--path", required=True, type=Path)

    r = sub.add_parser("release", help="flag-driven release pipeline")
    r.add_argument("version", help="X.Y.Z")
    r.add_argument("--bump", action="store_true")
    r.add_argument("--changelog", action="store_true")
    r.add_argument("--build", action="store_true")
    r.add_argument("--pack", action="store_true")
    r.add_argument("--installer", action="store_true")
    r.add_argument("--tag", action="store_true")
    r.add_argument("--push", action="store_true")
    r.add_argument("--all", action="store_true", help="bump+changelog+tag+push")
    r.add_argument("--dry-run", action="store_true")

    args = p.parse_args()

    if args.command == "trusted-sign-one":
        if args.target != "__trusted_signing__":
            raise SystemExit("trusted-sign-one is an internal signing command")
        if _signing_config() is None:
            raise SystemExit("trusted-sign-one requires GORE_SIGN=1")
        _run_trusted_signing_once(
            args.signtool,
            args.dlib,
            args.metadata,
            [args.path],
        )
        return 0

    if args.command == "release":
        targets = expand_targets(args.target, for_release=True)
        if len(targets) != 1:
            raise SystemExit("release takes exactly one project (not 'all')")
        steps = {
            "bump": args.bump or args.all,
            "changelog": args.changelog or args.all,
            "build": args.build,
            "pack": args.pack,
            "installer": args.installer,
            "tag": args.tag or args.all,
            "push": args.push or args.all,
        }
        if not any(steps.values()):
            raise SystemExit(
                "release needs at least one step flag (--bump/--build/--pack/"
                "--installer/--changelog/--tag/--push) or --all"
            )
        release_project(targets[0], args.version, steps, dry=args.dry_run)
        return 0

    if args.command == "run":
        targets = expand_targets(args.target, for_release=False)
        if len(targets) != 1:
            raise SystemExit("run takes exactly one project (not 'all')")
        run_project(targets[0], release=args.release or not args.debug)
        return 0

    if args.command == "build" and args.run:
        targets = expand_targets(args.target, for_release=False)
        if len(targets) != 1:
            raise SystemExit("build --run takes exactly one project (not 'all')")
        t = targets[0]
        release = args.release or not args.debug
        runnable_exe(t, release)  # rejects non-runnable before the rebuild
        build_project(t, release=release, dry=False)
        run_project(t, release=release)  # exe just built, so this only launches
        return 0

    targets = expand_targets(
        args.target, for_release=args.command in ("dist", "installer")
    )
    for t in targets:
        if args.command == "build":
            build_project(t, release=args.release or not args.debug, dry=False)
        elif args.command == "dist":
            dist_project(t, dry=False)
        elif args.command == "installer":
            if "installer" not in PROJECTS[t]:
                print(f"skip {t}: no installer")
                continue
            installer_project(t, dry=False)
        elif args.command == "test":
            test_project(t, dry=False)
    return 0


if __name__ == "__main__":
    sys.exit(main())
