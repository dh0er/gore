#!/usr/bin/env python3
"""Build the Rust native core used by the Flutter app."""

from __future__ import annotations

import argparse
import os
import shutil
import subprocess
import sys
from pathlib import Path

# This script lives at projects/gore-save/tools/; ROOT is projects/gore-save,
# but the cargo workspace target/ is at the monorepo root (two levels up).
ROOT = Path(__file__).resolve().parents[1]
REPO_ROOT = ROOT.parents[1]
APP = ROOT / "app"
CARGO = Path.home() / ".cargo" / "bin" / "cargo.exe"
VS_CMAKE_BIN = Path(
    r"C:\Program Files\Microsoft Visual Studio\2022\Community\Common7\IDE\CommonExtensions\Microsoft\CMake\CMake\bin"
)
NATIVE_ARTIFACTS = ("goresave_core.dll",)
WINDOWS_BUNDLE_CONFIGS = {
    "debug": "Debug",
    "profile": "Profile",
    "release": "Release",
}


def env() -> dict[str, str]:
    out = dict(os.environ)
    out["PATH"] = os.pathsep.join(
        [
            str(Path.home() / ".cargo" / "bin"),
            str(VS_CMAKE_BIN),
            out.get("PATH", ""),
        ]
    )
    return out


def windows_bundle_dir(app: Path = APP, profile: str = "debug") -> Path:
    try:
        config = WINDOWS_BUNDLE_CONFIGS[profile]
    except KeyError as err:
        raise ValueError(f"Unsupported Windows bundle profile: {profile}") from err
    return app / "build" / "windows" / "x64" / "runner" / config


def copy_native_artifacts_to_windows_bundle(
    *,
    root: Path = REPO_ROOT,
    app: Path = APP,
    profile: str = "debug",
) -> list[Path]:
    bundle_dir = windows_bundle_dir(app=app, profile=profile)
    app_exe = bundle_dir / "goresave.exe"
    if not app_exe.exists():
        raise FileNotFoundError(
            f"Windows app bundle not found: {app_exe}. "
            "Run `flutter build windows` before bundling native artifacts."
        )

    copied: list[Path] = []
    for artifact in NATIVE_ARTIFACTS:
        source = root / "target" / profile / artifact
        if not source.exists():
            raise FileNotFoundError(f"Native artifact not found: {source}")
        destination = bundle_dir / artifact
        shutil.copy2(source, destination)
        copied.append(destination)
    return copied


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--release", action="store_true")
    parser.add_argument("--copy-debug", action="store_true")
    parser.add_argument(
        "--bundle-windows",
        action="store_true",
        help="Copy native artifacts next to the built Flutter Windows app.",
    )
    args = parser.parse_args()

    cmd = [str(CARGO), "build"]
    if args.release:
        cmd.append("--release")
    result = subprocess.run(cmd, cwd=ROOT, env=env())
    if result.returncode != 0:
        return result.returncode

    profile = "release" if args.release else "debug"
    for artifact in NATIVE_ARTIFACTS:
        source = REPO_ROOT / "target" / profile / artifact
        if not source.exists():
            print(f"Native artifact not found: {source}")
            return 1

    if args.copy_debug:
        for artifact in NATIVE_ARTIFACTS:
            source = REPO_ROOT / "target" / profile / artifact
            out = APP / artifact
            shutil.copy2(source, out)
            print(f"Copied {source} -> {out}")

    if args.bundle_windows:
        try:
            copied = copy_native_artifacts_to_windows_bundle(profile=profile)
        except (FileNotFoundError, ValueError) as err:
            print(err)
            return 1
        for destination in copied:
            print(f"Bundled {destination}")

    return 0


if __name__ == "__main__":
    sys.exit(main())
