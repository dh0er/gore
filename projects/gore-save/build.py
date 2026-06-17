#!/usr/bin/env python3
"""goresave distribution build.

Builds the native core DLL in release mode (the Oodle codec is linked in-process
via goresave_oodle), builds the Flutter Windows release, places the native DLL
next to goresave.exe, and packages the Release folder into
dist/goresave-<version>-windows-x64.zip.

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

# build.py lives at projects/gore-save/; the cargo workspace + LICENSE are at
# the monorepo root (two levels up).
ROOT = Path(__file__).parent
REPO_ROOT = ROOT.parent.parent
APP = ROOT / "app"
RELEASE_DIR = APP / "build" / "windows" / "x64" / "runner" / "Release"
TARGET_RELEASE = REPO_ROOT / "target" / "release"
DIST = ROOT / "dist"

CORE_DLL = "goresave_core.dll"
LICENSE_FILE = REPO_ROOT / "LICENSE"


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
    try:
        probe = subprocess.run(
            ["git", "rev-parse", "--short=7", "HEAD"],
            cwd=ROOT,
            capture_output=True,
            text=True,
        )
    except OSError:
        return "dev"
    if probe.returncode == 0 and probe.stdout.strip():
        return probe.stdout.strip()
    return "dev"


def copy_license_to_bundle(release_dir: Path) -> Path:
    if not LICENSE_FILE.exists():
        raise SystemExit(f"missing license file: {LICENSE_FILE}")
    destination = release_dir / "LICENSE"
    shutil.copy2(LICENSE_FILE, destination)
    print(f"copied LICENSE -> {destination}")
    return destination


def dist(version: str, git_sha: str) -> Path:
    run("Build core (release)", [CARGO, "build", "--release", "-p", "goresave_core"])
    run(
        "Flutter Windows release",
        [FLUTTER, "build", "windows", "--release", f"--dart-define=GIT_SHA={git_sha}"],
        cwd=APP,
    )

    if not RELEASE_DIR.exists():
        raise SystemExit(f"missing Flutter release output: {RELEASE_DIR}")
    src = TARGET_RELEASE / CORE_DLL
    if not src.exists():
        raise SystemExit(f"missing native artifact: {src}")
    shutil.copy2(src, RELEASE_DIR / CORE_DLL)
    print(f"copied {CORE_DLL} -> {RELEASE_DIR / CORE_DLL}")

    copy_license_to_bundle(RELEASE_DIR)

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
