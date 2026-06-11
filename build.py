#!/usr/bin/env python3
"""goresave distribution build.

Builds the native core DLL and codec-host EXE in release mode, builds the
Flutter Windows release, places the native binaries next to goresave.exe, and
packages the Release folder into dist/goresave-<version>-windows-x64.zip.

Usage:
    python build.py            # full distribution build
    python build.py dist       # same
    python build.py --version 1.2.3
"""

from __future__ import annotations

import argparse
import os
import re
import shutil
import subprocess
from pathlib import Path

ROOT = Path(__file__).parent
APP = ROOT / "apps" / "goresave"
RELEASE_DIR = APP / "build" / "windows" / "x64" / "runner" / "Release"
TARGET_RELEASE = ROOT / "target" / "release"
DIST = ROOT / "dist"

CORE_DLL = "goresave_core.dll"
CODEC_HOST_EXE = "goresave_g1r_codec_host.exe"


def _resolve_tool(env_var: str, names: list[str], fallback: Path) -> Path:
    """Resolve a tool from an env override, then PATH, then a local fallback."""
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
    "CARGO",
    ["cargo.exe", "cargo"],
    Path.home() / ".cargo" / "bin" / "cargo.exe",
)


def env() -> dict[str, str]:
    out = dict(os.environ)
    out["PATH"] = os.pathsep.join(
        [str(FLUTTER.parent), str(CARGO.parent), out.get("PATH", "")]
    )
    return out


def run(label: str, cmd: list[object], cwd: Path = ROOT) -> None:
    printable = " ".join(str(part) for part in cmd)
    print(f"\n{'=' * 72}\n{label}\n-> {printable}\n{'=' * 72}")
    completed = subprocess.run([str(part) for part in cmd], cwd=cwd, env=env())
    if completed.returncode != 0:
        raise SystemExit(f"{label} failed (exit {completed.returncode})")


def read_version() -> str:
    text = (APP / "pubspec.yaml").read_text(encoding="utf-8")
    match = re.search(r"^version:\s*([0-9]+\.[0-9]+\.[0-9]+)", text, re.MULTILINE)
    return match.group(1) if match else "0.0.0"


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


def dist(version: str, git_sha: str) -> Path:
    run("Build core (release)", [CARGO, "build", "--release", "-p", "goresave_core"])
    run(
        "Build codec host (release)",
        [CARGO, "build", "--release", "-p", "goresave_g1r_codec_host"],
    )
    run(
        "Flutter Windows release",
        [FLUTTER, "build", "windows", "--release", f"--dart-define=GIT_SHA={git_sha}"],
        cwd=APP,
    )

    if not RELEASE_DIR.exists():
        raise SystemExit(f"missing Flutter release output: {RELEASE_DIR}")
    for name in (CORE_DLL, CODEC_HOST_EXE):
        src = TARGET_RELEASE / name
        if not src.exists():
            raise SystemExit(f"missing native artifact: {src}")
        shutil.copy2(src, RELEASE_DIR / name)
        print(f"copied {name} -> {RELEASE_DIR / name}")

    DIST.mkdir(exist_ok=True)
    base = DIST / f"goresave-{version}-windows-x64"
    zip_path = base.with_suffix(".zip")
    if zip_path.exists():
        zip_path.unlink()
    archive = shutil.make_archive(str(base), "zip", root_dir=RELEASE_DIR)
    print(f"\npackaged: {archive}")
    return Path(archive)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", nargs="?", choices=["dist"], default="dist")
    parser.add_argument("--version", help="Override version (default: from pubspec).")
    parser.add_argument("--git-sha", help="Short commit SHA (default: env/git).")
    args = parser.parse_args()
    dist(args.version or read_version(), resolve_git_sha(args.git_sha))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
