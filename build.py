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
    unsigned. GORE_SIGN_NO_PROXY=1 makes the signtool call alone bypass the
    system proxy, for machines whose PAC routes AAD login through a tunnel that
    may be down (the rest of the build keeps the system proxy).
"""

from __future__ import annotations

import argparse
import os
import posixpath
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
        "releasable": True,
    },
    "gore-mod-manager": {  # mod manager (Flutter, WinSparkle)
        "kind": "flutter",
        "dir": "apps/mod-manager",
        "pubspec": "pubspec.yaml",
        "tag_prefix": "gore-mod-manager",
        "changelog": "CHANGELOG.md",
        "installer": "installer/setup.iss",
        "exe": "gore_manager.exe",  # CMake BINARY_NAME
        "core_crate": "gore-ffi",  # shares the mod-studio FFI crate
        "core_dll": "gore_ffi",  # dll gore_ffi.dll (cargo underscores it)
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
# fails even though the network is fine. These overrides are handed to the signtool
# child process alone, so the rest of the build (Flutter, cargo, pub) keeps using
# the system proxy untouched.
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
    print("signing: bypassing the system proxy for signtool (GORE_SIGN_NO_PROXY=1)")
    return {
        "HTTP_PROXY": "http://127.0.0.1:9",
        "HTTPS_PROXY": "http://127.0.0.1:9",
        "NO_PROXY": _SIGN_NO_PROXY,
    }


def _find_signtool() -> Path:
    override = os.environ.get("SIGNTOOL")
    if override:
        return Path(override)
    found = shutil.which("signtool") or shutil.which("signtool.exe")
    if found:
        return Path(found)
    base = Path(r"C:\Program Files (x86)\Windows Kits\10\bin")
    cands = sorted(base.glob("*/x64/signtool.exe"), reverse=True)
    if cands:
        return cands[0]
    raise SystemExit("signtool.exe not found (install the Windows 10/11 SDK)")


def _ensure_dlib() -> Path:
    """Return the Trusted Signing dlib, fetching the official Microsoft NuGet
    package on first use so neither local checkouts nor CI need a separate
    install step (the payload is gitignored under tools/)."""
    dlib = TS_DLIB_DIR / "Azure.CodeSigning.Dlib.dll"
    if dlib.exists():
        return dlib
    import io
    import urllib.request
    import zipfile

    ver = TS_DLIB_VERSION
    url = (
        "https://api.nuget.org/v3-flatcontainer/microsoft.trusted.signing.client/"
        f"{ver}/microsoft.trusted.signing.client.{ver}.nupkg"
    )
    print(f"fetching Trusted Signing dlib {ver} from nuget.org ...")
    TS_DLIB_DIR.mkdir(parents=True, exist_ok=True)
    data = urllib.request.urlopen(url).read()  # official MS package, official registry
    with zipfile.ZipFile(io.BytesIO(data)) as z:
        for e in z.infolist():
            if e.filename.startswith("bin/x64/") and not e.is_dir():
                with z.open(e) as src, open(TS_DLIB_DIR / Path(e.filename).name, "wb") as dst:
                    shutil.copyfileobj(src, dst)
    if not dlib.exists():
        raise SystemExit("Trusted Signing dlib fetch failed")
    return dlib


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
            print(f"signing: off (default) for {len(pe)} file(s) — set GORE_SIGN=1 to enable")
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
        run(
            f"code-sign {len(pe)} file(s)",
            [
                signtool, "sign", "/v", "/fd", "SHA256",
                "/tr", TS_TIMESTAMP, "/td", "SHA256",
                "/dlib", dlib, "/dmdf", meta,
                *pe,
            ],
            extra_env=_sign_proxy_overrides(),
        )
    finally:
        meta.unlink(missing_ok=True)


def sign_dir(directory: Path, dry: bool) -> None:
    """Sign every PE file directly under `directory` (recursively)."""
    sign_paths(sorted(directory.rglob("*")), dry)


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

    A file is restored only when Git's own filtered hash of the working copy already
    equals what the index holds, which means the two differ in nothing but line
    endings. A genuinely regenerated file — a real translation change — hashes
    differently and is left exactly where it is, for the developer to review and
    commit.
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
        indexed = _git(["rev-parse", f":{path}"])
        working = _git(["hash-object", "--path", path, "--", path])
        if indexed is None or working is None:
            continue
        if indexed.strip() == working.strip():
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
        )
        if dry:
            print(f"[dry-run] would bundle {dep} into {rel}")
            continue
        exe = target_dir(True) / f"{dcfg['bin']}.exe"
        if not exe.exists():
            raise SystemExit(f"missing companion binary: {exe}")
        shutil.copy2(exe, rel / exe.name)
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
        run(f"cargo build {project} ({mode})", cmd, dry=dry)
        return
    # flutter app: build native cdylib first, then the app, then bundle the dll.
    # The cargo package id (hyphenated) and the produced dll basename
    # (underscored) differ, so they are tracked as separate fields.
    crate = cfg["core_crate"]
    cargo_cmd = [CARGO, "build", "-p", crate]
    if release:
        cargo_cmd.append("--release")
    run(f"cargo build {crate} ({mode})", cargo_cmd, dry=dry)

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
        sign_dir(staging, dry=dry)
        if base.with_suffix(".zip").exists():
            base.with_suffix(".zip").unlink()
        archive = shutil.make_archive(str(base), "zip", root_dir=staging)
        shutil.rmtree(staging)
        print(f"\npackaged: {archive}")
        return Path(archive)

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
    if (ROOT / "THIRD_PARTY_LICENSES.md").exists():
        shutil.copy2(ROOT / "THIRD_PARTY_LICENSES.md", staging / "THIRD_PARTY_LICENSES.md")
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
            [staging / exe.name, "guide", "html", "-o", rendered, "--repo-ref", repo_ref()],
        )
        if not rendered.is_file():
            raise SystemExit(f"guide render produced nothing at {rendered}")
    sign_dir(staging, dry=dry)
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
    sign_dir(rel, dry=dry)
    run(
        f"installer {project}",
        [
            ISCC,
            "/Qp",
            f"/DAppVersion={version}",
            f"/DSourceDir={rel}",
            f"/DOutputDir={dist}",
            # Passed in rather than hardcoded per .iss, so the produced file name
            # and the path this function returns cannot drift apart.
            f"/DOutputBaseName={installer_basename(project, version)}",
            iss,
        ],
        dry=dry,
    )
    out = dist / f"{installer_basename(project, version)}.exe"
    # Sign the installer itself. Must happen before the CI appcast step computes
    # its DSA signature over the final shipped bytes.
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
    run(f"cargo test {cfg['core_crate']}", [CARGO, "test", "-p", cfg["core_crate"]], dry=dry)
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
