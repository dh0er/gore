#!/usr/bin/env python3
"""Fail closed when Mod Manager release artifacts violate their Windows contract."""

from __future__ import annotations

import argparse
import ctypes
import hashlib
import os
from pathlib import Path
import re
import stat
import struct
import sys
import tempfile
from typing import Callable, Iterable, Mapping
import zipfile


ROOT = Path(__file__).resolve().parent.parent
VERSION_RE = re.compile(r"^[0-9]+\.[0-9]+\.[0-9]+$")
AMD64 = 0x8664

RUNTIME_FILES = {
    "msvcp140.dll",
    "vcruntime140.dll",
    "vcruntime140_1.dll",
}
PORTABLE_PLUGIN_FILES = {
    "file_selector_windows_plugin.dll",
    "screen_retriever_windows_plugin.dll",
    "url_launcher_windows_plugin.dll",
    "window_manager_plugin.dll",
}
UPDATER_FILES = {
    "auto_updater_windows_plugin.dll",
    "WinSparkle.dll",
}
BASE_PE_FILES = {
    "gore_manager.exe",
    "gore_ffi.dll",
    "flutter_windows.dll",
    *PORTABLE_PLUGIN_FILES,
    *RUNTIME_FILES,
}
PORTABLE_ROOT_FILES = {
    *BASE_PE_FILES,
    "LICENSE",
    "THIRD_PARTY_LICENSES.md",
}
INSTALLER_SOURCE_ROOT_FILES = {*BASE_PE_FILES, *UPDATER_FILES}
REQUIRED_DATA_FILES = {
    "data/app.so",
    "data/icudtl.dat",
    "data/flutter_assets/AssetManifest.bin",
    "data/flutter_assets/FontManifest.json",
    "data/flutter_assets/NativeAssetsManifest.json",
    "data/flutter_assets/NOTICES.Z",
}

APP_METADATA = {
    "CompanyName": "dh0er",
    "FileDescription": "GORE Mod Manager",
    "InternalName": "gore_manager",
    "LegalCopyright": "Copyright (C) 2026 dh0er. All rights reserved.",
    "OriginalFilename": "gore_manager.exe",
    "ProductName": "GORE Mod Manager",
}
INSTALLER_METADATA = {
    "CompanyName": "dh0er",
    "FileDescription": "GORE Mod Manager Setup",
    "LegalCopyright": "Copyright (C) 2026 dh0er. All rights reserved.",
    "ProductName": "GORE Mod Manager",
}
VERSION_FIELDS = (
    "CompanyName",
    "FileDescription",
    "FileVersion",
    "InternalName",
    "LegalCopyright",
    "OriginalFilename",
    "ProductName",
    "ProductVersion",
)

EXPECTED_INSTALLER_FILES = (
    'Source: "{#SourceDir}\\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs',
    'Source: "..\\..\\..\\LICENSE"; DestDir: "{app}"; Flags: ignoreversion',
    'Source: "..\\..\\..\\THIRD_PARTY_LICENSES.md"; DestDir: "{app}"; Flags: ignoreversion',
)

THIRD_PARTY_NOTICE_FILES = ("about.hbs", "THIRD_PARTY_LICENSES.md")
WINSPARKLE_NOTICE_START = "### WinSparkle 0.8.1 updater"
WINSPARKLE_NOTICE_END = "### BSD 2-Clause - embedded diagnostics helper"
# These hashes come from the exact local dependency package at the pinned
# resolved ref below. Its original files are COPYING (SHA-256
# d236b139330d1456b8f392f8d10453e86c9a9375ec939566b234b9363a56535e) and
# COPYING.expat (SHA-256
# 7e043c766b1772d1ccc47751c7334db4feb1f2f9a7fd5055df60a6fde420a2f5);
# rendered blocks use LF and no edge blanks.
WINSPARKLE_NOTICE_SHA256 = (
    "d5221ccb51b03d603fcd7d92b4b284fdf1e4609956b65600eb59048312a6be5d"
)
WINSPARKLE_UPSTREAM_BLOCK_SHA256 = {
    "WinSparkle license": (
        "06f49858020bdc0ac014b0591dd9e8b4ff6e3d547576777b0a5e73f27a4e9c97"
    ),
    "OpenSSL attribution from WinSparkle COPYING": (
        "0be450ce884e718e17497338a2f39f7efa3096cc513d7c7683c77efe8cf2f516"
    ),
    "Expat license": (
        "4c76c8c281d86ab08dadc05f47b15b827ef18fd3035090f167ffab81cccd7069"
    ),
}
AUTO_UPDATER_WINDOWS_LOCK_STANZA = "\n".join(
    (
        "  auto_updater_windows:",
        '    dependency: "direct overridden"',
        "    description:",
        '      path: "packages/auto_updater_windows"',
        "      ref: swiftpm-support",
        '      resolved-ref: "56dc406f6e0f6ccf01d70d2fbc88f7ca1c3ebf9a"',
        '      url: "https://github.com/dh0er/auto_updater.git"',
        "    source: git",
        '    version: "1.0.1"',
    )
)
THIRD_PARTY_NOTICE_MARKERS = {
    "WinSparkle 0.8.1 heading": "### WinSparkle 0.8.1 updater",
    "WinSparkle COPYING source": (
        "`packages/auto_updater_windows/windows/WinSparkle-0.8.1/COPYING`"
    ),
    "WinSparkle copyright": "Copyright (c) 2009-2023 Vaclav Slavik",
    "WinSparkle OpenSSL attribution": (
        "This product includes software developed by the OpenSSL Project\n"
        "for use in the OpenSSL Toolkit (http://www.openssl.org/)."
    ),
    "WinSparkle COPYING.expat source": (
        "`packages/auto_updater_windows/windows/WinSparkle-0.8.1/COPYING.expat`"
    ),
    "WinSparkle Expat copyright": (
        "Copyright (c) 1998-2000 Thai Open Source Software Center Ltd and Clark Cooper\n"
        "Copyright (c) 2001-2017 Expat maintainers"
    ),
}

_WINDOWS_FORBIDDEN_CHARS = set('<>:"\\|?*')
_WINDOWS_RESERVED = re.compile(r"^(?:con|prn|aux|nul|com[1-9]|lpt[1-9])(?:\..*)?$", re.I)
_FILE_ATTRIBUTE_REPARSE_POINT = 0x0400


class ContractError(ValueError):
    pass


def _normalise_newlines(text: str) -> str:
    return text.replace("\r\n", "\n").replace("\r", "\n")


def _winsparkle_notice_section(text: str) -> str | None:
    text = _normalise_newlines(text)
    if text.count(WINSPARKLE_NOTICE_START) != 1:
        return None
    start = text.index(WINSPARKLE_NOTICE_START)
    end = text.find(WINSPARKLE_NOTICE_END, start)
    if end < 0:
        return None
    return text[start:end].rstrip("\n")


def _markdown_code_block(section: str, heading: str) -> str | None:
    match = re.search(
        rf"(?ms)^#### {re.escape(heading)}\n.*?^```\n(.*?)\n^```$", section
    )
    return None if match is None else match.group(1)


def _auto_updater_windows_lock_contract(root: Path) -> list[str]:
    relative = "apps/mod-manager/pubspec.lock"
    path = root / relative
    if path.is_symlink() or not path.is_file():
        return [f"release notices: missing or non-regular file: {relative}"]
    try:
        text = _normalise_newlines(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError) as error:
        return [f"release notices: cannot read {relative}: {error}"]
    stanzas = re.findall(
        r"(?ms)^  auto_updater_windows:\n.*?(?=^  [A-Za-z0-9_-]+:\n|\Z)", text
    )
    if len(stanzas) != 1 or stanzas[0].rstrip("\n") != AUTO_UPDATER_WINDOWS_LOCK_STANZA:
        return [
            "release notices: auto_updater_windows dependency pin changed; "
            "reverify WinSparkle notices against the new resolved source"
        ]
    return []


def _third_party_notice_contract(root: Path) -> list[str]:
    problems: list[str] = []
    sections: dict[str, str] = {}
    for relative in THIRD_PARTY_NOTICE_FILES:
        path = root / relative
        if path.is_symlink() or not path.is_file():
            problems.append(f"release notices: missing or non-regular file: {relative}")
            continue
        try:
            text = path.read_text(encoding="utf-8")
        except (OSError, UnicodeError) as error:
            problems.append(f"release notices: cannot read {relative}: {error}")
            continue
        for label, marker in THIRD_PARTY_NOTICE_MARKERS.items():
            if marker not in text:
                problems.append(f"release notices: {relative} is missing {label}")
        section = _winsparkle_notice_section(text)
        if section is None:
            problems.append(
                f"release notices: {relative} has no unique bounded WinSparkle section"
            )
            continue
        sections[relative] = section
        digest = hashlib.sha256(section.encode("utf-8")).hexdigest()
        if digest != WINSPARKLE_NOTICE_SHA256:
            problems.append(
                f"release notices: {relative} exact WinSparkle 0.8.1 notice "
                "section changed"
            )
        for heading, expected_digest in WINSPARKLE_UPSTREAM_BLOCK_SHA256.items():
            block = _markdown_code_block(section, heading)
            block_digest = (
                None
                if block is None
                else hashlib.sha256(block.encode("utf-8")).hexdigest()
            )
            if block_digest != expected_digest:
                problems.append(
                    f"release notices: {relative} exact upstream {heading} "
                    "block changed"
                )
    if len(sections) == len(THIRD_PARTY_NOTICE_FILES) and len(set(sections.values())) != 1:
        problems.append(
            "release notices: about.hbs and THIRD_PARTY_LICENSES.md WinSparkle "
            "sections differ"
        )
    problems.extend(_auto_updater_windows_lock_contract(root))
    return problems


def _windows_path(name: str, *, directory: bool) -> str:
    if not name or "\x00" in name or "\\" in name or name.startswith("/"):
        raise ContractError("empty, absolute, NUL, or backslash path")
    if directory:
        if not name.endswith("/"):
            raise ContractError("directory entry lacks trailing slash")
        name = name[:-1]
    elif name.endswith("/"):
        raise ContractError("file entry has trailing slash")
    parts = name.split("/")
    if not parts or any(part in ("", ".", "..") for part in parts):
        raise ContractError("empty, dot, or parent path component")
    for part in parts:
        if part[-1] in (" ", "."):
            raise ContractError("component ends in a Windows-ignored character")
        if any(ord(char) < 32 or char in _WINDOWS_FORBIDDEN_CHARS for char in part):
            raise ContractError("component contains a Windows-forbidden character")
        if _WINDOWS_RESERVED.fullmatch(part):
            raise ContractError("component is a reserved Windows device name")
    return "/".join(part.casefold() for part in parts)


def _validate_names(
    entries: Iterable[tuple[str, bool]], label: str
) -> tuple[dict[str, str], list[str]]:
    files: dict[str, str] = {}
    kinds: dict[str, bool] = {}
    problems: list[str] = []
    for raw_name, directory in entries:
        try:
            canonical = _windows_path(raw_name, directory=directory)
        except ContractError as error:
            problems.append(f"{label}: unsafe path {raw_name!r}: {error}")
            continue
        if canonical in kinds:
            problems.append(
                f"{label}: duplicate Windows path {raw_name!r} collides with "
                f"{files.get(canonical, canonical)!r}"
            )
            continue
        kinds[canonical] = directory
        if not directory:
            files[canonical] = raw_name

    for canonical, directory in kinds.items():
        if directory:
            continue
        parts = canonical.split("/")
        for index in range(1, len(parts)):
            parent = "/".join(parts[:index])
            if parent in kinds and not kinds[parent]:
                problems.append(
                    f"{label}: file path {canonical!r} descends through file {parent!r}"
                )
    return files, problems


def _pe_machine(payload: bytes, label: str) -> int:
    try:
        if len(payload) < 0x40 or payload[:2] != b"MZ":
            raise ContractError("missing DOS header")
        pe_offset = struct.unpack_from("<I", payload, 0x3C)[0]
        if pe_offset > len(payload) - 6 or payload[pe_offset : pe_offset + 4] != b"PE\0\0":
            raise ContractError("invalid PE header")
        return struct.unpack_from("<H", payload, pe_offset + 4)[0]
    except struct.error as error:
        raise ContractError(f"truncated PE header: {error}") from error


def _check_required_paths(
    files: Mapping[str, str], required: Iterable[str], label: str
) -> list[str]:
    problems: list[str] = []
    for expected in sorted(required, key=str.casefold):
        canonical = expected.casefold()
        actual = files.get(canonical)
        if actual is None:
            problems.append(f"{label}: missing {expected}")
        elif actual != expected:
            problems.append(f"{label}: wrong path casing {actual!r}; expected {expected!r}")
    return problems


def _check_root_files(
    files: Mapping[str, str], allowed: Iterable[str], label: str
) -> list[str]:
    allowed_names = {name.casefold() for name in allowed}
    problems: list[str] = []
    for canonical, raw_name in sorted(files.items()):
        if "/" not in canonical and canonical not in allowed_names:
            problems.append(f"{label}: unexpected root file {raw_name}")
        elif "/" in canonical and canonical.endswith((".exe", ".dll")):
            problems.append(f"{label}: nested PE payload is forbidden: {raw_name}")
    return problems


def _read_version_info(path: Path) -> dict[str, str]:
    if sys.platform != "win32":
        raise ContractError("Windows version-resource APIs are required")
    version = ctypes.WinDLL("version", use_last_error=True)
    version.GetFileVersionInfoSizeW.argtypes = [ctypes.c_wchar_p, ctypes.POINTER(ctypes.c_ulong)]
    version.GetFileVersionInfoSizeW.restype = ctypes.c_ulong
    version.GetFileVersionInfoW.argtypes = [
        ctypes.c_wchar_p,
        ctypes.c_ulong,
        ctypes.c_ulong,
        ctypes.c_void_p,
    ]
    version.GetFileVersionInfoW.restype = ctypes.c_int
    version.VerQueryValueW.argtypes = [
        ctypes.c_void_p,
        ctypes.c_wchar_p,
        ctypes.POINTER(ctypes.c_void_p),
        ctypes.POINTER(ctypes.c_uint),
    ]
    version.VerQueryValueW.restype = ctypes.c_int

    ignored = ctypes.c_ulong()
    size = version.GetFileVersionInfoSizeW(str(path), ctypes.byref(ignored))
    if not size:
        raise ContractError(f"no Windows version resource: {path}")
    buffer = ctypes.create_string_buffer(size)
    if not version.GetFileVersionInfoW(str(path), 0, size, buffer):
        raise ContractError(f"cannot read Windows version resource: {path}")

    pointer = ctypes.c_void_p()
    length = ctypes.c_uint()
    translations: list[tuple[int, int]] = []
    if version.VerQueryValueW(
        buffer, r"\VarFileInfo\Translation", ctypes.byref(pointer), ctypes.byref(length)
    ):
        raw = ctypes.string_at(pointer, length.value)
        translations.extend(
            struct.unpack_from("<HH", raw, offset)
            for offset in range(0, len(raw) - 3, 4)
        )
    for fallback in ((0x0409, 0x04E4), (0x0409, 0x04B0)):
        if fallback not in translations:
            translations.append(fallback)

    result: dict[str, str] = {}
    for field in VERSION_FIELDS:
        for language, codepage in translations:
            query = rf"\StringFileInfo\{language:04x}{codepage:04x}\{field}"
            if version.VerQueryValueW(
                buffer, query, ctypes.byref(pointer), ctypes.byref(length)
            ):
                result[field] = ctypes.wstring_at(pointer, length.value).rstrip("\x00")
                break
    return result


def _check_metadata(
    info: Mapping[str, str], expected: Mapping[str, str], label: str
) -> list[str]:
    return [
        f"{label}: {field}={info.get(field)!r}; expected {value!r}"
        for field, value in expected.items()
        if info.get(field) != value
    ]


def _zip_contract(
    root: Path,
    archive: Path,
    version: str,
    version_info_reader: Callable[[Path], Mapping[str, str]],
) -> list[str]:
    label = "portable zip"
    problems: list[str] = []
    try:
        with zipfile.ZipFile(archive) as package:
            infos = package.infolist()
            files, name_problems = _validate_names(
                ((info.filename, info.is_dir()) for info in infos), label
            )
            problems.extend(name_problems)
            problems.extend(_check_required_paths(files, PORTABLE_ROOT_FILES, label))
            problems.extend(_check_required_paths(files, REQUIRED_DATA_FILES, label))
            problems.extend(_check_root_files(files, PORTABLE_ROOT_FILES, label))

            by_name: dict[str, zipfile.ZipInfo] = {}
            for info in infos:
                mode = info.external_attr >> 16
                if stat.S_IFMT(mode) == stat.S_IFLNK:
                    problems.append(f"{label}: symbolic link is forbidden: {info.filename}")
                if info.flag_bits & 0x1:
                    problems.append(f"{label}: encrypted entry is forbidden: {info.filename}")
                if info.is_dir():
                    continue
                try:
                    canonical = _windows_path(info.filename, directory=False)
                except ContractError:
                    continue
                by_name.setdefault(canonical, info)

            forbidden = {name.casefold() for name in UPDATER_FILES}
            for canonical, raw_name in files.items():
                if canonical.rsplit("/", 1)[-1] in forbidden:
                    problems.append(f"{label}: updater payload is forbidden: {raw_name}")

            for required in sorted(PORTABLE_ROOT_FILES | REQUIRED_DATA_FILES):
                info = by_name.get(required.casefold())
                if info is not None and info.file_size == 0:
                    problems.append(f"{label}: required file is empty: {required}")

            for license_name in ("LICENSE", "THIRD_PARTY_LICENSES.md"):
                info = by_name.get(license_name.casefold())
                source = root / license_name
                if info is None or not source.is_file():
                    continue
                try:
                    if package.read(info) != source.read_bytes():
                        problems.append(
                            f"{label}: {license_name} does not match repository source"
                        )
                except OSError as error:
                    problems.append(f"{label}: cannot compare {license_name}: {error}")

            for pe_name in sorted(BASE_PE_FILES):
                info = by_name.get(pe_name.casefold())
                if info is None:
                    continue
                try:
                    machine = _pe_machine(package.read(info), pe_name)
                    if machine != AMD64:
                        problems.append(
                            f"{label}: {pe_name} is PE machine 0x{machine:04x}, expected x64"
                        )
                except (ContractError, OSError, RuntimeError) as error:
                    problems.append(f"{label}: invalid PE {pe_name}: {error}")

            app = by_name.get("gore_manager.exe")
            if app is not None:
                with tempfile.TemporaryDirectory(prefix="gore-manager-artifact-") as temp_name:
                    extracted = Path(temp_name) / "gore_manager.exe"
                    extracted.write_bytes(package.read(app))
                    try:
                        info = version_info_reader(extracted)
                    except (ContractError, OSError) as error:
                        problems.append(f"{label}: cannot inspect app metadata: {error}")
                    else:
                        expected = {
                            **APP_METADATA,
                            "FileVersion": version,
                            "ProductVersion": version,
                        }
                        problems.extend(_check_metadata(info, expected, f"{label} app"))
    except (OSError, zipfile.BadZipFile) as error:
        problems.append(f"{label}: cannot open {archive}: {error}")
    return problems


def _is_reparse_entry(entry: os.DirEntry[str], status: os.stat_result) -> bool:
    return entry.is_symlink() or bool(
        getattr(status, "st_file_attributes", 0) & _FILE_ATTRIBUTE_REPARSE_POINT
    )


def _filesystem_entries(directory: Path, label: str) -> tuple[dict[str, str], list[str]]:
    problems: list[str] = []
    entries: list[tuple[str, bool]] = []
    try:
        root_status = directory.lstat()
    except OSError as error:
        return {}, [f"{label}: cannot inspect {directory}: {error}"]
    if directory.is_symlink() or (
        getattr(root_status, "st_file_attributes", 0) & _FILE_ATTRIBUTE_REPARSE_POINT
    ):
        return {}, [f"{label}: release root is a forbidden reparse point: {directory}"]
    if not stat.S_ISDIR(root_status.st_mode):
        return {}, [f"{label}: release root is not a directory: {directory}"]

    pending: list[tuple[Path, str]] = [(directory, "")]
    while pending:
        current, prefix = pending.pop()
        try:
            with os.scandir(current) as iterator:
                children = sorted(iterator, key=lambda child: child.name.casefold())
        except OSError as error:
            problems.append(f"{label}: cannot enumerate {current}: {error}")
            continue
        for entry in children:
            relative = f"{prefix}/{entry.name}" if prefix else entry.name
            try:
                status = entry.stat(follow_symlinks=False)
            except OSError as error:
                problems.append(f"{label}: cannot inspect {relative}: {error}")
                continue
            if _is_reparse_entry(entry, status):
                problems.append(f"{label}: reparse point is forbidden: {relative}")
                continue
            if stat.S_ISDIR(status.st_mode):
                entries.append((relative + "/", True))
                pending.append((Path(entry.path), relative))
            elif stat.S_ISREG(status.st_mode):
                entries.append((relative, False))
            else:
                problems.append(f"{label}: non-regular entry is forbidden: {relative}")
    files, name_problems = _validate_names(entries, label)
    return files, [*problems, *name_problems]


def _section_lines(text: str, section: str) -> list[str]:
    active = False
    lines: list[str] = []
    for raw_line in text.splitlines():
        stripped = raw_line.strip()
        if stripped.startswith("[") and stripped.endswith("]"):
            active = stripped.casefold() == f"[{section}]".casefold()
            continue
        if active and stripped and not stripped.startswith(";"):
            lines.append(stripped)
    return lines


def _installer_recipe_contract(setup: Path, version: str, installer_name: str) -> list[str]:
    label = "installer recipe"
    try:
        text = setup.read_text(encoding="utf-8")
    except OSError as error:
        return [f"{label}: cannot read {setup}: {error}"]

    problems: list[str] = []
    files = tuple(_section_lines(text, "Files"))
    if files != EXPECTED_INSTALLER_FILES:
        problems.append(f"{label}: [Files] contract changed: {files!r}")

    settings: dict[str, str] = {}
    for line in _section_lines(text, "Setup"):
        if "=" not in line:
            problems.append(f"{label}: malformed [Setup] directive: {line}")
            continue
        key, value = line.split("=", 1)
        if key in settings:
            problems.append(f"{label}: duplicate [Setup] directive: {key}")
        settings[key] = value
    expected_settings = {
        "AppVersion": "{#AppVersion}",
        "OutputBaseFilename": "{#OutputBaseName}",
        "ArchitecturesAllowed": "x64compatible",
        "ArchitecturesInstallIn64BitMode": "x64compatible",
        "LicenseFile": r"..\..\..\LICENSE",
        "VersionInfoCompany": "dh0er",
        "VersionInfoCopyright": "Copyright (C) 2026 dh0er. All rights reserved.",
        "VersionInfoDescription": "GORE Mod Manager Setup",
        "VersionInfoOriginalFileName": "{#OutputBaseName}.exe",
        "VersionInfoProductName": "GORE Mod Manager",
        "VersionInfoProductTextVersion": "{#AppVersion}",
        "VersionInfoProductVersion": "{#AppVersion}.0",
        "VersionInfoTextVersion": "{#AppVersion}",
        "VersionInfoVersion": "{#AppVersion}.0",
    }
    for key, value in expected_settings.items():
        if settings.get(key) != value:
            problems.append(f"{label}: {key}={settings.get(key)!r}; expected {value!r}")

    expected_installer = f"gore-mod-manager-{version}-setup.exe"
    if installer_name != expected_installer:
        problems.append(
            f"{label}: installer name {installer_name!r}; expected {expected_installer!r}"
        )
    return problems


def _installer_contract(
    root: Path,
    installer: Path,
    version: str,
    version_info_reader: Callable[[Path], Mapping[str, str]],
) -> list[str]:
    # Inno consumes this Release tree through the exact [Files] recipe checked
    # below. Verify that static input instead of executing the generated setup.
    label = "installer source"
    release = (
        root
        / "apps"
        / "mod-manager"
        / "build"
        / "windows"
        / "x64"
        / "runner"
        / "Release"
    )
    if not release.is_dir():
        return [f"{label}: missing release directory: {release}"]

    files, problems = _filesystem_entries(release, label)
    problems.extend(_check_required_paths(files, INSTALLER_SOURCE_ROOT_FILES, label))
    problems.extend(_check_required_paths(files, REQUIRED_DATA_FILES, label))
    problems.extend(_check_root_files(files, INSTALLER_SOURCE_ROOT_FILES, label))
    for required in sorted(INSTALLER_SOURCE_ROOT_FILES | REQUIRED_DATA_FILES):
        raw_name = files.get(required.casefold())
        if raw_name is not None:
            try:
                if (release / Path(raw_name)).stat().st_size == 0:
                    problems.append(f"{label}: required file is empty: {required}")
            except OSError as error:
                problems.append(f"{label}: cannot stat {required}: {error}")

    for pe_name in sorted(INSTALLER_SOURCE_ROOT_FILES):
        raw_name = files.get(pe_name.casefold())
        if raw_name is None:
            continue
        path = release / Path(raw_name)
        try:
            machine = _pe_machine(path.read_bytes(), pe_name)
            if machine != AMD64:
                problems.append(
                    f"{label}: {pe_name} is PE machine 0x{machine:04x}, expected x64"
                )
        except (ContractError, OSError) as error:
            problems.append(f"{label}: invalid PE {pe_name}: {error}")

    app = release / "gore_manager.exe"
    if app.is_file():
        try:
            info = version_info_reader(app)
        except (ContractError, OSError) as error:
            problems.append(f"{label}: cannot inspect app metadata: {error}")
        else:
            expected = {
                **APP_METADATA,
                "FileVersion": version,
                "ProductVersion": version,
            }
            problems.extend(_check_metadata(info, expected, f"{label} app"))

    setup = root / "apps" / "mod-manager" / "installer" / "setup.iss"
    problems.extend(_installer_recipe_contract(setup, version, installer.name))

    if installer.is_symlink() or not installer.is_file():
        problems.append(f"installer: missing or non-regular file: {installer}")
    else:
        try:
            if installer.stat().st_size == 0:
                problems.append(f"installer: empty file: {installer}")
            _pe_machine(installer.read_bytes(), installer.name)
            info = version_info_reader(installer)
        except (ContractError, OSError) as error:
            problems.append(f"installer: cannot inspect metadata: {error}")
        else:
            expected = {
                **INSTALLER_METADATA,
                "FileVersion": version,
                "OriginalFilename": installer.name,
                "ProductVersion": version,
            }
            problems.extend(_check_metadata(info, expected, "installer"))

    for license_name in ("LICENSE", "THIRD_PARTY_LICENSES.md"):
        license_path = root / license_name
        if license_path.is_symlink() or not license_path.is_file():
            problems.append(f"installer source: missing license file: {license_name}")
        elif license_path.stat().st_size == 0:
            problems.append(f"installer source: empty license file: {license_name}")
    return problems


def verify_release(
    root: Path,
    version: str,
    *,
    version_info_reader: Callable[[Path], Mapping[str, str]] = _read_version_info,
) -> list[str]:
    if VERSION_RE.fullmatch(version) is None:
        return [f"version must be plain X.Y.Z, got {version!r}"]
    notice_problems = _third_party_notice_contract(root)
    dist = root / "dist" / "gore-mod-manager"
    portable_name = f"gore-mod-manager-{version}-windows-x64.zip"
    installer_name = f"gore-mod-manager-{version}-setup.exe"
    portable = dist / portable_name
    installer = dist / installer_name

    problems: list[str] = []
    problems.extend(notice_problems)
    if not dist.is_dir():
        problems.append(f"release output directory missing: {dist}")
        return problems
    package_entries = [
        (path.name, path.is_dir())
        for path in dist.iterdir()
        if path.suffix.casefold() in (".zip", ".exe")
    ]
    package_files, name_problems = _validate_names(package_entries, "release output")
    problems.extend(name_problems)
    problems.extend(
        _check_required_paths(package_files, (portable_name, installer_name), "release output")
    )
    allowed_packages = {portable_name.casefold(), installer_name.casefold()}
    for canonical, raw_name in package_files.items():
        if canonical not in allowed_packages:
            problems.append(f"release output: unexpected package artifact {raw_name}")

    if portable.is_symlink() or not portable.is_file():
        problems.append(f"portable zip: missing or non-regular file: {portable}")
    else:
        problems.extend(_zip_contract(root, portable, version, version_info_reader))
    problems.extend(_installer_contract(root, installer, version, version_info_reader))
    return problems


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--version", required=True)
    parser.add_argument("--root", type=Path, default=ROOT)
    args = parser.parse_args()
    problems = verify_release(args.root.resolve(), args.version)
    if problems:
        for problem in problems:
            print(f"ERROR: {problem}", file=sys.stderr)
        return 1
    print(f"OK: GORE Mod Manager {args.version} release artifact contract verified.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
