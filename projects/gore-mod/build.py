#!/usr/bin/env python3
"""gore-mod release build.

Builds the gore_core cdylib (release), builds the Flutter Windows release, and
copies gore_core.dll next to gore_mod.exe so the dart:ffi bridge finds it.

Usage:
    python build.py            # full release build + DLL bundle
"""

from __future__ import annotations

import os
import shutil
import subprocess
from pathlib import Path

# build.py lives at projects/gore-mod/; the cargo workspace is at the repo root.
ROOT = Path(__file__).parent
REPO_ROOT = ROOT.parent.parent
APP = ROOT / "app"
RELEASE_DIR = APP / "build" / "windows" / "x64" / "runner" / "Release"
TARGET_RELEASE = REPO_ROOT / "target" / "release"
CORE_DLL = "gore_core.dll"


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


def main() -> int:
    run("Build gore_core (release cdylib)", [CARGO, "build", "--release", "-p", "gore_core"])
    run("Flutter Windows release", [FLUTTER, "build", "windows", "--release"], cwd=APP)

    if not RELEASE_DIR.exists():
        raise SystemExit(f"missing Flutter release output: {RELEASE_DIR}")
    src = TARGET_RELEASE / CORE_DLL
    if not src.exists():
        raise SystemExit(f"missing native artifact: {src}")
    shutil.copy2(src, RELEASE_DIR / CORE_DLL)
    print(f"copied {CORE_DLL} -> {RELEASE_DIR / CORE_DLL}")
    print("\ngore-mod release bundle ready.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
