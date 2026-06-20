#!/usr/bin/env python3
"""gore-tools monorepo build orchestrator.

One entry point for every project under projects/. Wraps the per-project
build logic (Flutter apps, Rust crates) and adds debug builds, tests, and a
flag-driven release pipeline.

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

Tags are per-project prefixed: gore-save-vX.Y.Z, gore-mod-vX.Y.Z,
gore-cli-vX.Y.Z. The Release workflow matches the prefix and builds only
that project.

Examples:
    python build.py all test
    python build.py gore-save dist
    python build.py gore-cli release 0.2.0 --all
    python build.py gore-mod release 0.1.0 --bump --build --installer  # local only
"""

from __future__ import annotations

import argparse
import os
import re
import shutil
import subprocess
import sys
from pathlib import Path

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
# Each project declares its kind plus the paths the recipes need. Internal
# libraries (gore-core) and generated mods (gore-dump) carry releasable=False
# so `all` skips them for release/dist and `release <project>` rejects them.
PROJECTS: dict[str, dict] = {
    "gore-save": {
        "kind": "flutter",
        "dir": "projects/gore-save",
        "pubspec": "app/pubspec.yaml",
        "tag_prefix": "gore-save",
        "changelog": "CHANGELOG.md",
        "installer": "installer/setup.iss",
        "installer_name": "GoresaveSetup",  # -> GoresaveSetup-X.Y.Z.exe
        "exe": "goresave.exe",  # CMake BINARY_NAME
        "core_dll": "goresave_core",  # cargo crate + dll basename
        "dist_zip": "goresave-{version}-windows-x64",
        "releasable": True,
    },
    "gore-mod": {
        "kind": "flutter",
        "dir": "projects/gore-mod",
        "pubspec": "app/pubspec.yaml",
        "tag_prefix": "gore-mod",
        "changelog": "CHANGELOG.md",
        "installer": "installer/setup.iss",
        "installer_name": "GoreModSetup",
        "exe": "gore_mod.exe",  # CMake BINARY_NAME
        "core_dll": "gore_core",
        "dist_zip": "gore-mod-{version}-windows-x64",
        "releasable": True,
    },
    "gore-cli": {
        "kind": "rust-bin",
        "dir": "projects/gore-cli",
        "manifest": "crates/gore_cli/Cargo.toml",
        "crate": "gore_cli",
        "bin": "gore-cli",  # produced exe basename
        "tag_prefix": "gore-cli",
        "changelog": "CHANGELOG.md",
        "dist_zip": "gore-cli-{version}-windows-x64",
        "releasable": True,
    },
    "gore-core": {
        "kind": "rust-lib",
        "dir": "projects/gore-core",
        "manifest": "crates/gore_core/Cargo.toml",
        "crate": "gore_core",
        "releasable": False,
    },
}

RELEASE_ORDER = ["gore-cli", "gore-save", "gore-mod"]  # for `all`


def env() -> dict[str, str]:
    out = dict(os.environ)
    out["PATH"] = os.pathsep.join(
        [str(FLUTTER.parent), str(CARGO.parent), out.get("PATH", "")]
    )
    return out


def run(label: str, cmd: list[object], cwd: Path = ROOT, dry: bool = False) -> None:
    printable = " ".join(str(part) for part in cmd)
    print(f"\n{'=' * 72}\n{label}\n-> {printable}  (cwd={cwd})\n{'=' * 72}")
    if dry:
        print("[dry-run] skipped")
        return
    completed = subprocess.run([str(part) for part in cmd], cwd=cwd, env=env())
    if completed.returncode != 0:
        raise SystemExit(f"{label} failed (exit {completed.returncode})")


def pdir(project: str) -> Path:
    return ROOT / PROJECTS[project]["dir"]


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
    # tag built with `cargo --locked` resolves the old version. `--workspace`
    # updates only the bumped member's lock entry, leaving registry deps pinned.
    if cfg["kind"] in ("rust-bin", "rust-lib"):
        run(f"sync Cargo.lock {project}", [CARGO, "update", "--workspace"])


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


# --------------------------------------------------------------------------- #
# Build recipes                                                               #
# --------------------------------------------------------------------------- #
def flutter_build_dir(project: str, release: bool = True) -> Path:
    mode = "Release" if release else "Debug"
    return pdir(project) / "app" / "build" / "windows" / "x64" / "runner" / mode


def flutter_release_dir(project: str) -> Path:
    return flutter_build_dir(project, release=True)


def target_dir(release: bool) -> Path:
    return ROOT / "target" / ("release" if release else "debug")


def build_project(project: str, release: bool, dry: bool) -> None:
    cfg = PROJECTS[project]
    mode = "release" if release else "debug"
    if cfg["kind"] in ("rust-bin", "rust-lib"):
        cmd = [CARGO, "build", "-p", cfg["crate"]]
        if release:
            cmd.append("--release")
        run(f"cargo build {project} ({mode})", cmd, dry=dry)
        return
    # flutter app: build native cdylib first, then the app, then bundle the dll.
    crate = cfg["core_dll"]
    cargo_cmd = [CARGO, "build", "-p", crate]
    if release:
        cargo_cmd.append("--release")
    run(f"cargo build {crate} ({mode})", cargo_cmd, dry=dry)

    flutter_cmd = [FLUTTER, "build", "windows", f"--{mode}"]
    if project == "gore-save":
        flutter_cmd.append(f"--dart-define=GIT_SHA={resolve_git_sha()}")
    run(f"flutter build {project} ({mode})", flutter_cmd, cwd=pdir(project) / "app", dry=dry)

    if dry:
        return
    rel = flutter_build_dir(project, release)
    if not rel.exists():
        raise SystemExit(f"missing flutter output: {rel}")
    dll = f"{crate}.dll"
    src = target_dir(release) / dll
    if not src.exists():
        raise SystemExit(f"missing native artifact: {src}")
    shutil.copy2(src, rel / dll)
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
    # Propagate the program's own exit code rather than framing a nonzero exit
    # as a build failure (a CLI may exit nonzero, e.g. usage with no args).
    print(f"\nlaunching {exe}")
    completed = subprocess.run([str(exe)], cwd=exe.parent, env=env())
    raise SystemExit(completed.returncode)


def dist_project(project: str, dry: bool) -> Path | None:
    """Release-build and package into dist/. Returns the zip path."""
    cfg = PROJECTS[project]
    if not cfg.get("releasable"):
        raise SystemExit(f"{project} is not releasable")
    build_project(project, release=True, dry=dry)
    version = read_version(project)
    dist = pdir(project) / "dist"
    if not dry:
        dist.mkdir(exist_ok=True)
    base = dist / cfg["dist_zip"].format(version=version)

    if cfg["kind"] == "flutter":
        rel = flutter_release_dir(project)
        license_file = ROOT / "LICENSE"
        if not dry:
            if license_file.exists():
                shutil.copy2(license_file, rel / "LICENSE")
            if base.with_suffix(".zip").exists():
                base.with_suffix(".zip").unlink()
            archive = shutil.make_archive(str(base), "zip", root_dir=rel)
            print(f"\npackaged: {archive}")
            return Path(archive)
        print(f"[dry-run] would zip {rel} -> {base}.zip")
        return None

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
    if (ROOT / "LICENSE").exists():
        shutil.copy2(ROOT / "LICENSE", staging / "LICENSE")
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
    dist = pdir(project) / "dist"
    iss = pdir(project) / cfg["installer"]
    run(
        f"installer {project}",
        [
            ISCC,
            "/Qp",
            f"/DAppVersion={version}",
            f"/DSourceDir={rel}",
            f"/DOutputDir={dist}",
            iss,
        ],
        dry=dry,
    )
    out = dist / f"{cfg['installer_name']}-{version}.exe"
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
    run(f"cargo test {cfg['core_dll']}", [CARGO, "test", "-p", cfg["core_dll"]], dry=dry)
    app = pdir(project) / "app"
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
            changed = subprocess.run(
                ["git", "diff", "--quiet", "HEAD", "--", *rel_paths], cwd=ROOT
            ).returncode != 0
        if changed:
            git(["commit", "-m", f"release({project}): {version}", "--", *rel_paths], dry)
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
        return [n for n in names if PROJECTS[n].get("releasable", True) or not for_release]
    if target not in PROJECTS:
        raise SystemExit(f"unknown project {target!r}; choices: {', '.join(PROJECTS)}, all")
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

    targets = expand_targets(args.target, for_release=args.command in ("dist", "installer"))
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
