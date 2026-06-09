#!/usr/bin/env python3
"""goresave test runner."""

from __future__ import annotations

import argparse
import os
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).parent
APP = ROOT / "apps" / "goresave"


def _resolve_tool(env_var: str, names: list[str], fallback: Path) -> Path:
    """Resolve a tool from an env override, then PATH, then a local fallback.

    Keeps the runner working on machines where the tool is on PATH or under a
    different FVM version, instead of failing on a single hard-coded path.
    """
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
CARGO = Path.home() / ".cargo" / "bin" / "cargo.exe"
VS_CMAKE_BIN = Path(
    r"C:\Program Files\Microsoft Visual Studio\2022\Community\Common7\IDE\CommonExtensions\Microsoft\CMake\CMake\bin"
)


def env() -> dict[str, str]:
    out = dict(os.environ)
    path_parts = [
        str(FLUTTER.parent),
        str(Path.home() / ".cargo" / "bin"),
        str(VS_CMAKE_BIN),
        out.get("PATH", ""),
    ]
    out["PATH"] = os.pathsep.join(path_parts)
    return out


def run(label: str, cmd: list[str], cwd: Path = ROOT) -> int:
    print(f"\n{'=' * 72}\n{label}\n-> {' '.join(cmd)}\n{'=' * 72}")
    completed = subprocess.run(cmd, cwd=cwd, env=env())
    return completed.returncode


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "suite",
        nargs="?",
        choices=["all", "rust", "tools", "flutter", "analyze", "build-native"],
        default="all",
    )
    args = parser.parse_args()

    suites = (
        [args.suite]
        if args.suite != "all"
        else ["rust", "tools", "analyze", "flutter"]
    )
    codes: list[int] = []

    if "rust" in suites:
        codes.append(run("Rust tests", [str(CARGO), "test"]))

    if "tools" in suites:
        codes.append(
            run(
                "Python tool tests",
                [sys.executable, "-m", "unittest", "discover", "-s", "tools"],
            )
        )

    if "build-native" in suites:
        codes.append(run("Rust native build", [str(CARGO), "build"]))

    if "analyze" in suites:
        codes.append(run("Flutter pub get", [str(FLUTTER), "pub", "get"], APP))
        codes.append(run("Flutter analyze", [str(FLUTTER), "analyze"], APP))

    if "flutter" in suites:
        codes.append(run("Flutter tests", [str(FLUTTER), "test"], APP))

    return max(codes) if codes else 0


if __name__ == "__main__":
    sys.exit(main())
