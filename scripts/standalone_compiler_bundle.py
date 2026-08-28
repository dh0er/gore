#!/usr/bin/env python3
"""Offline signing, internal staging, and verification for the standalone compiler bundle.

Regular GORE product builds never build, sign, or re-qualify a sidecar. They accept either no
internal compiler input (the honest BundleAbsent state) or one immutable internal input directory
whose descriptor pins the already signed and qualified bytes. This module deliberately has no
GitHub release or tag operation and is independent of the Rust product resolver.
"""

from __future__ import annotations

import argparse
from contextlib import contextmanager, ExitStack
import copy
import ctypes
from dataclasses import dataclass
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import shutil
import stat
import struct
import subprocess
import sys
import tempfile
from typing import Callable, Iterable
import zipfile


ROOT = Path(__file__).resolve().parent.parent

INTERNAL_INPUT_SCHEMA = "gore.as.standalone-compiler-internal-input"
INTERNAL_INPUT_SCHEMA_VERSION = 2
INTERNAL_INPUT_DESCRIPTOR_FILE = "internal-input.json"
CATALOG_SCHEMA = "gore.as.product-standalone-compiler-catalog"
CATALOG_SCHEMA_VERSION = 1
BUNDLE_DESCRIPTOR = "compiler-bundle-manifest.json"
CATALOG_FILE = "catalog.json"
EMBEDDED_CATALOG_FILE = "embedded-catalog.json"
SIDECAR_FILE = "gore-as-standalone-compiler.exe"
PROMOTION_IDENTITY_FILE = "promotion/signed-sidecar-identity.json"
PROMOTION_PROVENANCE_FILE = "promotion/source-provenance.json"
PROMOTION_ATTESTATION_FILE = "promotion/github-attestation.sigstore.json"
SIGNED_SIDECAR_IDENTITY_SCHEMA = "gore.as.signed-standalone-compiler-identity"
SIGNED_SIDECAR_IDENTITY_SCHEMA_VERSION = 1
PROMOTION_PROVENANCE_SCHEMA = (
    "gore.as.internal-standalone-compiler-signing-provenance"
)
PROMOTION_PROVENANCE_SCHEMA_VERSION = 1
QUALIFIED_PROMOTION_RECEIPT_FILE = "qualification-promotion-receipt.json"
EMBEDDED_QUALIFICATION_ARTIFACT_MANIFEST_FILE = "embedded-qualification-artifacts.json"
STANDALONE_QUALIFICATION_ARTIFACT_MANIFEST_FILE = (
    "standalone-qualification-artifacts.json"
)
QUALIFIED_PROMOTION_RECEIPT_SCHEMA = "gore.as.qualified-profile-promotion"
QUALIFIED_PROMOTION_RECEIPT_SCHEMA_VERSION = 1
QUALIFIED_PROFILE_TREE_HASH_DOMAIN = b"gore-as-qualified-profile-tree-v1\0"
LEGACY_SMOKE_REQUEST_VERSION = 1
PRODUCTION_REQUEST_VERSION = 2
PROTOCOL_RESPONSE_VERSION = 1
MAX_DESCRIPTOR_BYTES = 1024 * 1024
MAX_ATTESTATION_BYTES = 16 * 1024 * 1024
MAX_CATALOG_BYTES = 256 * 1024
MAX_SIDECAR_BYTES = 256 * 1024 * 1024
MAX_PROFILE_MANIFEST_BYTES = 4 * 1024 * 1024
MAX_PROFILE_BLOB_BYTES = 512 * 1024 * 1024
MAX_PROFILE_AGGREGATE_BYTES = 1024 * 1024 * 1024
MAX_FULL_TREE_RECEIPT_BYTES = 4 * 1024 * 1024
FULL_TREE_RECEIPT_SCHEMA = "gore.as.internal-full-tree-verification"
FULL_TREE_RECEIPT_VERSION = 1
FULL_TREE_VERIFICATION_DIRECTORY = "verification/full-tree"
# Python's classic-ZIP writer switches to ZIP64 above this boundary. The
# internal package deliberately forbids ZIP64, so the bound is explicit here.
MAX_ARCHIVE_UNCOMPRESSED_BYTES = (1 << 31) - 1
MAX_INTERNAL_INPUT_FILES = 4096
STALE_SIDECAR_BYTE_LENGTHS = frozenset({273_408})
INTERNAL_PACKAGE_DESCRIPTOR_SCHEMA = "gore.as.internal-standalone-compiler-package"
INTERNAL_PACKAGE_DESCRIPTOR_SCHEMA_VERSION = 1
INTERNAL_PACKAGE_ARCHIVE_FILE = "standalone-compiler-internal-package.zip"
INTERNAL_PACKAGE_DESCRIPTOR_FILE = "standalone-compiler-internal-package.json"
# The repository carries the qualified input compressed.  Its current 152 MiB
# internal-input tree is roughly 7 MiB with deterministic Deflate; leave ample
# room for additional qualified game APIs without accepting an unbounded asset.
MAX_INTERNAL_PACKAGE_BYTES = 128 * 1024 * 1024
REQUIRED_NOTICES = (
    "UNREANGEL-LICENSE.md",
    "SOURCE_INVENTORY.tsv",
    "PROVENANCE.toml",
)
# Publishing policy, deliberately separate from the runtime catalog's structural/API-based
# target selection. Product artifacts for the August 2026 generation must carry both the last
# qualified generation and its successor; runtime admission still does not pin complete binaries.
REQUIRED_PRODUCT_COMPILER_TARGETS_V1 = (
    {
        "target": {
            "steam_app_id": 1_297_900,
            "steam_build_id": 24_539_464,
            "depot_id": 1_297_901,
            "depot_manifest_gid": 1_585_071_322_101_748_861,
            "platform": "windows",
            "architecture": "x86_64",
            "build_configuration": "shipping",
        },
        "pe_codeview": {
            "guid": "cf0b83bd-e023-061b-2100-0f0fccf871d2",
            "age": 1,
        },
    },
    {
        "target": {
            "steam_app_id": 1_297_900,
            "steam_build_id": 24_878_692,
            "depot_id": 1_297_901,
            "depot_manifest_gid": 382_135_126_159_906_494,
            "platform": "windows",
            "architecture": "x86_64",
            "build_configuration": "shipping",
        },
        "pe_codeview": {
            "guid": "c2ca4ada-4878-d963-e567-717dc2c483a2",
            "age": 1,
        },
    },
)
PROFILE_BLOB_FIELDS = (
    ("engine", "ordered_engine_properties"),
    ("engine", "registration_trace"),
    ("engine", "post_bind_snapshot"),
    ("unreal_semantics", "reflected_type_graph"),
    ("frontend", "preprocessor_config"),
    ("frontend", "class_generator_config"),
    ("frontend", "compiler_options"),
    ("bytecode", "opcode_table"),
    ("bytecode", "operand_schema"),
    ("bytecode", "codegen_probe_corpus"),
    ("bytecode", "expected_probe_results"),
    ("cache_writer", "serializer_schema"),
    ("cache_writer", "reference_table_order"),
    ("cache_writer", "normalized_oracle_corpus"),
    ("qualification", "diagnostic_parity"),
    ("qualification", "semantic_parity"),
)
_FILE_ATTRIBUTE_REPARSE_POINT = 0x400
_PE_MACHINE_AMD64 = 0x8664
_PE32_PLUS_MAGIC = 0x20B
_IMAGE_DIRECTORY_ENTRY_IMPORT = 1
_IMAGE_DIRECTORY_ENTRY_SECURITY = 4
_IMAGE_DIRECTORY_ENTRY_DELAY_IMPORT = 13
_WIN_CERT_REVISION_2_0 = 0x0200
_WIN_CERT_TYPE_PKCS_SIGNED_DATA = 0x0002
_FIXED_SYSTEM_DLLS = frozenset(
    name.casefold()
    for name in (
        "advapi32.dll",
        "bcrypt.dll",
        "crypt32.dll",
        "kernel32.dll",
        "ntdll.dll",
        "ole32.dll",
        "oleaut32.dll",
        "rpcrt4.dll",
        "secur32.dll",
        "shell32.dll",
        "shlwapi.dll",
        "user32.dll",
        "version.dll",
        "ws2_32.dll",
    )
)


class BundleError(RuntimeError):
    """An internal input or staged bundle failed closed."""


@dataclass(frozen=True)
class Seal:
    byte_len: int
    sha256: str


@dataclass(frozen=True)
class PreparedBundle:
    present: bool
    work_root: Path
    catalog_path: Path
    bundle_root: Path | None
    sidecar_name: str | None
    catalog_sha256: str | None
    legacy_internal_input_v1: bool = False

    @property
    def signing_exclusions(self) -> tuple[str, ...]:
        return (self.sidecar_name,) if self.sidecar_name is not None else ()


@dataclass(frozen=True)
class VerifiedBundle:
    descriptor_bytes: bytes
    catalog_bytes: bytes
    expected_files: dict[str, Seal]
    sidecar_name: str
    internal_input_schema_version: int


@dataclass(frozen=True)
class QualifiedProfileTreeAuthority:
    profile_sha256: str
    manifest_sha256: str
    promotion_receipt_sha256: str
    tree_sha256: str
    file_count: int


@dataclass(frozen=True)
class FullTreeVerificationAuthority:
    profile_sha256: str
    sidecar: Seal
    shipping: Seal
    binds: Seal
    frozen_source: Seal
    embedded_reference: Seal
    standalone_candidate: Seal
    module_count: int


@dataclass(frozen=True)
class RecordedProfilePlan:
    key: tuple[object, ...]
    source_root: Path
    manifest: bytes
    blobs: list[tuple[str, Seal, str]]
    promotion_audits: dict[str, Seal]
    catalog_entry: dict[str, object]
    full_tree_relative: str
    full_tree_bytes: bytes
    full_tree_entry: dict[str, object]


@dataclass(frozen=True)
class InternalPackageDescriptor:
    asset: str
    archive: Seal
    compression: str
    catalog_sha256: str
    file_count: int


@dataclass(frozen=True)
class PromotionAuthority:
    repository: str
    commit: str
    workflow_sha: str
    workflow_run_id: int
    workflow_run_attempt: int


SidecarVerifier = Callable[[Path, bytes], None]
QualifiedProfileVerifier = Callable[[Path, str], QualifiedProfileTreeAuthority]
PromotionAttestationVerifier = Callable[
    [Path, dict[str, Path], PromotionAuthority], None
]


def _qualified_profile_tree_seal_sha256(files: list[tuple[str, Seal]]) -> str:
    digest = hashlib.sha256()
    digest.update(QUALIFIED_PROFILE_TREE_HASH_DOMAIN)
    digest.update(struct.pack("<Q", len(files)))
    for relative, seal in sorted(files):
        encoded = relative.encode("utf-8")
        digest.update(struct.pack("<Q", len(encoded)))
        digest.update(encoded)
        digest.update(struct.pack("<Q", seal.byte_len))
        digest.update(bytes.fromhex(seal.sha256))
    return digest.hexdigest()


def _qualified_profile_tree_sha256(files: list[tuple[str, bytes]]) -> str:
    return _qualified_profile_tree_seal_sha256(
        [(relative, Seal(len(bytes_), _sha256(bytes_))) for relative, bytes_ in files]
    )


def _qualified_profile_tree_summary(
    profile_root: Path,
) -> QualifiedProfileTreeAuthority:
    """Hash the exact fixed qualified-profile tree using the Rust verifier's V1 domain."""

    manifest = _read_regular_no_follow(
        profile_root / "compiler-profile.json",
        MAX_PROFILE_MANIFEST_BYTES,
        "qualified compiler profile manifest",
    )
    profile = _parse_json(
        manifest, "qualified compiler profile manifest", MAX_PROFILE_MANIFEST_BYTES
    )
    names = {"compiler-profile.json"}
    names.update(relative for relative, _, _ in _profile_blob_seals(profile))
    names.update(
        {
            EMBEDDED_QUALIFICATION_ARTIFACT_MANIFEST_FILE,
            STANDALONE_QUALIFICATION_ARTIFACT_MANIFEST_FILE,
            QUALIFIED_PROMOTION_RECEIPT_FILE,
        }
    )
    actual = _enumerate_regular_files(profile_root, "qualified compiler profile tree")
    if actual != names:
        raise BundleError(
            "qualified compiler profile tree file set differs: "
            f"missing={sorted(names - actual)}, unknown={sorted(actual - names)}"
        )
    files: list[tuple[str, bytes]] = []
    for relative in sorted(names):
        bytes_ = _read_regular_no_follow(
            profile_root.joinpath(*PurePosixPath(relative).parts),
            MAX_PROFILE_BLOB_BYTES,
            f"qualified compiler profile tree file {relative}",
        )
        files.append((relative, bytes_))
    receipt = next(
        bytes_
        for relative, bytes_ in files
        if relative == QUALIFIED_PROMOTION_RECEIPT_FILE
    )
    return QualifiedProfileTreeAuthority(
        profile_sha256=_require_hex(
            profile.get("profile_sha256"), 64, "qualified compiler profile SHA-256"
        ),
        manifest_sha256=_sha256(manifest),
        promotion_receipt_sha256=_sha256(receipt),
        tree_sha256=_qualified_profile_tree_sha256(files),
        file_count=len(files),
    )


@contextmanager
def _pin_windows_file_path(
    path: Path,
    label: str,
    *,
    require_single_link: bool = True,
    allow_file_write: bool = False,
) -> Iterable[None]:
    """Prevent delete/ancestor replacement while another process resolves `path`.

    The signed compiler artifacts are Windows-only. Reopening an already measured file by pathname
    without holding the complete path chain would reopen a TOCTOU window, so non-Windows hosts
    fail closed instead of claiming an equivalent guarantee. Mutable signing targets may allow
    in-place writes while still withholding delete sharing; all ordinary callers deny writes too.
    """

    if os.name != "nt":
        raise BundleError(f"{label} can only be pinned for execution on Windows")

    from ctypes import wintypes

    file_read_attributes = 0x0080
    generic_read = 0x80000000
    file_share_read = 0x00000001
    file_share_write = 0x00000002
    open_existing = 3
    file_flag_open_reparse_point = 0x00200000
    file_flag_backup_semantics = 0x02000000
    file_attribute_directory = 0x00000010
    invalid_handle_value = ctypes.c_void_p(-1).value

    class ByHandleFileInformation(ctypes.Structure):
        _fields_ = [
            ("file_attributes", wintypes.DWORD),
            ("creation_time", wintypes.FILETIME),
            ("last_access_time", wintypes.FILETIME),
            ("last_write_time", wintypes.FILETIME),
            ("volume_serial_number", wintypes.DWORD),
            ("file_size_high", wintypes.DWORD),
            ("file_size_low", wintypes.DWORD),
            ("number_of_links", wintypes.DWORD),
            ("file_index_high", wintypes.DWORD),
            ("file_index_low", wintypes.DWORD),
        ]

    kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
    create_file = kernel32.CreateFileW
    create_file.argtypes = [
        wintypes.LPCWSTR,
        wintypes.DWORD,
        wintypes.DWORD,
        wintypes.LPVOID,
        wintypes.DWORD,
        wintypes.DWORD,
        wintypes.HANDLE,
    ]
    create_file.restype = wintypes.HANDLE
    get_information = kernel32.GetFileInformationByHandle
    get_information.argtypes = [
        wintypes.HANDLE,
        ctypes.POINTER(ByHandleFileInformation),
    ]
    get_information.restype = wintypes.BOOL
    close_handle = kernel32.CloseHandle
    close_handle.argtypes = [wintypes.HANDLE]
    close_handle.restype = wintypes.BOOL

    handles: list[int] = []

    def pin(component: Path, *, directory: bool) -> None:
        handle = create_file(
            str(component),
            file_read_attributes if directory or allow_file_write else generic_read,
            file_share_read | file_share_write
            if directory or allow_file_write
            else file_share_read,
            None,
            open_existing,
            file_flag_open_reparse_point
            | (file_flag_backup_semantics if directory else 0),
            None,
        )
        if handle == invalid_handle_value:
            error = ctypes.get_last_error()
            raise BundleError(
                f"cannot pin {label} component {component}: WinError {error}"
            )
        information = ByHandleFileInformation()
        if not get_information(handle, ctypes.byref(information)):
            error = ctypes.get_last_error()
            close_handle(handle)
            raise BundleError(
                f"cannot inspect pinned {label} component {component}: WinError {error}"
            )
        is_directory = bool(information.file_attributes & file_attribute_directory)
        if (
            bool(information.file_attributes & _FILE_ATTRIBUTE_REPARSE_POINT)
            or is_directory != directory
            or (
                not directory
                and require_single_link
                and information.number_of_links != 1
            )
        ):
            close_handle(handle)
            raise BundleError(f"pinned {label} component is unsafe: {component}")
        handles.append(handle)

    try:
        parents: list[Path] = []
        current = path.parent
        while True:
            parents.append(current)
            if current.parent == current:
                break
            current = current.parent
        for parent in reversed(parents):
            pin(parent, directory=True)
        pin(path, directory=False)
        yield
    finally:
        for handle in reversed(handles):
            close_handle(handle)


@contextmanager
def _pin_windows_mutable_file_path(path: Path, label: str) -> Iterable[None]:
    """Hold one single-link file identity while an approved tool updates it in place."""

    try:
        before = path.lstat()
    except OSError as error:
        raise BundleError(f"cannot inspect mutable {label} {path}: {error}") from error
    if (
        not stat.S_ISREG(before.st_mode)
        or before.st_nlink != 1
        or path.is_symlink()
        or getattr(before, "st_file_attributes", 0) & _FILE_ATTRIBUTE_REPARSE_POINT
    ):
        raise BundleError(f"mutable {label} is not a safe single-link file: {path}")
    identity = (before.st_dev, before.st_ino)
    with _pin_windows_file_path(path, label, allow_file_write=True):
        yield
        try:
            after = path.lstat()
        except OSError as error:
            raise BundleError(
                f"mutable {label} disappeared during the operation: {path}: {error}"
            ) from error
        if (
            (after.st_dev, after.st_ino) != identity
            or not stat.S_ISREG(after.st_mode)
            or after.st_nlink != 1
            or path.is_symlink()
            or getattr(after, "st_file_attributes", 0) & _FILE_ATTRIBUTE_REPARSE_POINT
        ):
            raise BundleError(
                f"mutable {label} identity changed during the operation: {path}"
            )


@contextmanager
def _pin_windows_executable_path(
    path: Path, label: str, *, require_single_link: bool = True
) -> Iterable[None]:
    """Pin one already measured executable through its complete Windows path."""

    with _pin_windows_file_path(path, label, require_single_link=require_single_link):
        yield


@contextmanager
def _pin_windows_directories(directories: Iterable[Path], label: str) -> Iterable[None]:
    """Hold real directory handles without delete sharing during extraction."""

    if os.name != "nt":
        for directory in directories:
            _check_no_follow_chain(directory, label)
        yield
        return

    from ctypes import wintypes

    file_read_attributes = 0x0080
    file_share_read = 0x00000001
    file_share_write = 0x00000002
    open_existing = 3
    file_flag_open_reparse_point = 0x00200000
    file_flag_backup_semantics = 0x02000000
    file_attribute_directory = 0x00000010
    invalid_handle_value = ctypes.c_void_p(-1).value

    class ByHandleFileInformation(ctypes.Structure):
        _fields_ = [
            ("file_attributes", wintypes.DWORD),
            ("creation_time", wintypes.FILETIME),
            ("last_access_time", wintypes.FILETIME),
            ("last_write_time", wintypes.FILETIME),
            ("volume_serial_number", wintypes.DWORD),
            ("file_size_high", wintypes.DWORD),
            ("file_size_low", wintypes.DWORD),
            ("number_of_links", wintypes.DWORD),
            ("file_index_high", wintypes.DWORD),
            ("file_index_low", wintypes.DWORD),
        ]

    kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
    create_file = kernel32.CreateFileW
    create_file.argtypes = [
        wintypes.LPCWSTR,
        wintypes.DWORD,
        wintypes.DWORD,
        wintypes.LPVOID,
        wintypes.DWORD,
        wintypes.DWORD,
        wintypes.HANDLE,
    ]
    create_file.restype = wintypes.HANDLE
    get_information = kernel32.GetFileInformationByHandle
    get_information.argtypes = [
        wintypes.HANDLE,
        ctypes.POINTER(ByHandleFileInformation),
    ]
    get_information.restype = wintypes.BOOL
    close_handle = kernel32.CloseHandle
    close_handle.argtypes = [wintypes.HANDLE]
    close_handle.restype = wintypes.BOOL

    all_directories: dict[str, Path] = {}
    for directory in directories:
        current = _require_absolute_normalized(directory, label)
        while True:
            all_directories.setdefault(str(current).casefold(), current)
            if current.parent == current:
                break
            current = current.parent
    handles: list[int] = []
    try:
        for directory in sorted(
            all_directories.values(),
            key=lambda path: (len(path.parts), str(path).casefold()),
        ):
            handle = create_file(
                str(directory),
                file_read_attributes,
                file_share_read | file_share_write,
                None,
                open_existing,
                file_flag_open_reparse_point | file_flag_backup_semantics,
                None,
            )
            if handle == invalid_handle_value:
                error = ctypes.get_last_error()
                raise BundleError(
                    f"cannot pin {label} directory {directory}: WinError {error}"
                )
            information = ByHandleFileInformation()
            if not get_information(handle, ctypes.byref(information)):
                error = ctypes.get_last_error()
                close_handle(handle)
                raise BundleError(
                    f"cannot inspect pinned {label} directory {directory}: WinError {error}"
                )
            if (
                not information.file_attributes & file_attribute_directory
                or information.file_attributes & _FILE_ATTRIBUTE_REPARSE_POINT
            ):
                close_handle(handle)
                raise BundleError(f"pinned {label} directory is unsafe: {directory}")
            handles.append(handle)
        yield
    finally:
        for handle in reversed(handles):
            close_handle(handle)


def _read_pinned_windows_regular(
    path: Path,
    maximum: int,
    label: str,
    *,
    require_single_link: bool,
) -> bytes:
    """Read one Windows file while its complete path and file object cannot drift."""

    path = _require_absolute_normalized(path, label)
    with _pin_windows_executable_path(
        path, label, require_single_link=require_single_link
    ):
        try:
            with path.open("rb") as stream:
                bytes_ = stream.read(maximum + 1)
        except OSError as error:
            raise BundleError(f"cannot read pinned {label}: {error}") from error
    if not bytes_ or len(bytes_) > maximum:
        raise BundleError(f"{label} size is outside 1..{maximum} bytes")
    return bytes_


def verify_qualified_profile_with_executable(
    verifier: Path,
    expected_verifier_seal: Seal,
    profile_root: Path,
    expected_profile_sha256: str,
) -> QualifiedProfileTreeAuthority:
    verifier = _require_absolute_normalized(verifier, "qualified-profile verifier")
    profile_root = _require_absolute_normalized(profile_root, "qualified profile root")
    with _pin_windows_executable_path(
        verifier, "qualified-profile verifier executable"
    ):
        verifier_bytes = _read_regular_no_follow(
            verifier, MAX_SIDECAR_BYTES, "qualified-profile verifier executable"
        )
        _check_sealed_bytes(
            verifier_bytes,
            expected_verifier_seal,
            "qualified-profile verifier executable",
        )
        try:
            completed = subprocess.run(
                [str(verifier), str(profile_root)],
                cwd=ROOT,
                stdin=subprocess.DEVNULL,
                capture_output=True,
                check=False,
                timeout=120,
            )
        except (OSError, subprocess.SubprocessError) as error:
            raise BundleError(
                f"qualified-profile verifier could not run: {error}"
            ) from error
        if (
            _read_regular_no_follow(
                verifier, MAX_SIDECAR_BYTES, "qualified-profile verifier executable"
            )
            != verifier_bytes
        ):
            raise BundleError("qualified-profile verifier changed while executing")
    if completed.returncode != 0:
        stderr = completed.stderr.decode("utf-8", errors="replace")[:4096].strip()
        raise BundleError(
            "Rust typed qualified-profile reload failed"
            + (f": {stderr}" if stderr else "")
        )
    result = _parse_json(
        completed.stdout,
        "qualified-profile verifier response",
        MAX_DESCRIPTOR_BYTES,
    )
    _require_exact_fields(
        result,
        (
            "schema",
            "schema_version",
            "qualified",
            "profile_sha256",
            "manifest_sha256",
            "promotion_receipt_sha256",
            "tree_sha256",
            "file_count",
        ),
        "qualified-profile verifier response",
    )
    observed_tree = _qualified_profile_tree_summary(profile_root)
    if (
        result["schema"] != "gore.as.qualified-profile-verification"
        or result["schema_version"] != 1
        or result["qualified"] is not True
        or _require_hex(
            result["profile_sha256"],
            64,
            "qualified-profile verifier profile SHA-256",
        )
        != expected_profile_sha256
        or observed_tree.profile_sha256 != expected_profile_sha256
        or _require_hex(
            result["manifest_sha256"],
            64,
            "qualified-profile verifier manifest SHA-256",
        )
        != observed_tree.manifest_sha256
        or _require_hex(
            result["promotion_receipt_sha256"],
            64,
            "qualified-profile verifier promotion-receipt SHA-256",
        )
        != observed_tree.promotion_receipt_sha256
        or _require_hex(
            result["tree_sha256"], 64, "qualified-profile verifier tree SHA-256"
        )
        != observed_tree.tree_sha256
        or _require_uint(result["file_count"], "qualified-profile verifier file count")
        != observed_tree.file_count
    ):
        raise BundleError(
            "qualified-profile verifier returned a different profile-tree authority"
        )
    return observed_tree


def qualified_profile_verifier_from_path(verifier: Path) -> QualifiedProfileVerifier:
    """Measure one operator-authorized verifier before any later pinned execution."""

    verifier = _require_absolute_normalized(verifier, "qualified-profile verifier")
    verifier_bytes = _read_regular_no_follow(
        verifier, MAX_SIDECAR_BYTES, "qualified-profile verifier executable"
    )
    verifier_seal = Seal(len(verifier_bytes), _sha256(verifier_bytes))

    def verify(
        profile_root: Path, profile_sha256: str
    ) -> QualifiedProfileTreeAuthority:
        return verify_qualified_profile_with_executable(
            verifier, verifier_seal, profile_root, profile_sha256
        )

    return verify


def verify_github_attestation_with_executable(
    verifier: Path,
    expected_verifier_seal: Seal,
    bundle_path: Path,
    subjects: dict[str, Path],
    authority: PromotionAuthority,
) -> None:
    """Cryptographically bind promotion files to GitHub's workflow certificate."""

    verifier = _require_absolute_normalized(
        verifier, "GitHub attestation verifier executable"
    )
    bundle_path = _require_absolute_normalized(
        bundle_path, "GitHub promotion attestation bundle"
    )
    expected_names = {
        SIDECAR_FILE,
        PurePosixPath(PROMOTION_IDENTITY_FILE).name,
        PurePosixPath(PROMOTION_PROVENANCE_FILE).name,
    }
    if set(subjects) != expected_names:
        raise BundleError(
            "GitHub promotion attestation subjects are incomplete or unexpected"
        )
    subject_seals: dict[str, Seal] = {}
    normalized_subjects: dict[str, Path] = {}
    for name, path in subjects.items():
        if Path(name).name != name or not name.isascii():
            raise BundleError("GitHub promotion attestation subject name is unsafe")
        normalized = _require_absolute_normalized(
            path, f"GitHub promotion attestation subject {name}"
        )
        bytes_ = _read_regular_no_follow(
            normalized,
            MAX_SIDECAR_BYTES if name == SIDECAR_FILE else MAX_DESCRIPTOR_BYTES,
            f"GitHub promotion attestation subject {name}",
        )
        subject_seals[name] = Seal(len(bytes_), _sha256(bytes_))
        normalized_subjects[name] = normalized
    bundle_bytes = _read_regular_no_follow(
        bundle_path,
        MAX_ATTESTATION_BYTES,
        "GitHub promotion attestation bundle",
    )
    signer_workflow = (
        f"{authority.repository}/.github/workflows/standalone-compiler-promotion.yml"
    )
    environment = os.environ.copy()
    environment.pop("GH_TOKEN", None)
    environment.pop("GITHUB_TOKEN", None)
    with ExitStack() as pins:
        pins.enter_context(
            _pin_windows_executable_path(
                verifier, "GitHub attestation verifier executable"
            )
        )
        pins.enter_context(
            _pin_windows_file_path(bundle_path, "GitHub promotion attestation bundle")
        )
        for name, path in sorted(normalized_subjects.items()):
            pins.enter_context(
                _pin_windows_file_path(
                    path, f"GitHub promotion attestation subject {name}"
                )
            )
        verifier_bytes = _read_regular_no_follow(
            verifier,
            MAX_SIDECAR_BYTES,
            "GitHub attestation verifier executable",
        )
        _check_sealed_bytes(
            verifier_bytes,
            expected_verifier_seal,
            "GitHub attestation verifier executable",
        )
        for name, subject in sorted(normalized_subjects.items()):
            command = [
                str(verifier),
                "attestation",
                "verify",
                str(subject),
                "--repo",
                authority.repository,
                "--bundle",
                str(bundle_path),
                "--signer-workflow",
                signer_workflow,
                "--signer-digest",
                authority.workflow_sha,
                "--source-digest",
                authority.commit,
                "--deny-self-hosted-runners",
                "--format",
                "json",
            ]
            try:
                completed = subprocess.run(
                    command,
                    cwd=ROOT,
                    stdin=subprocess.DEVNULL,
                    capture_output=True,
                    check=False,
                    timeout=120,
                    env=environment,
                )
            except (OSError, subprocess.SubprocessError) as error:
                raise BundleError(
                    f"GitHub attestation verifier could not run for {name}: {error}"
                ) from error
            if completed.returncode != 0:
                detail = completed.stderr.decode("utf-8", errors="replace")[
                    :4096
                ].strip()
                raise BundleError(
                    f"GitHub promotion attestation failed for {name}"
                    + (f": {detail}" if detail else "")
                )
            try:
                result = json.loads(
                    completed.stdout.decode("utf-8"),
                    object_pairs_hook=_json_no_duplicates,
                )
            except (UnicodeDecodeError, json.JSONDecodeError) as error:
                raise BundleError(
                    "GitHub attestation verifier returned invalid JSON"
                ) from error
            if not isinstance(result, list) or len(result) != 1:
                raise BundleError(
                    "GitHub attestation verifier must return exactly one attestation"
                )
            entry = result[0]
            verification = (
                entry.get("verificationResult") if isinstance(entry, dict) else None
            )
            statement = (
                verification.get("statement")
                if isinstance(verification, dict)
                else None
            )
            attested_subjects = (
                statement.get("subject") if isinstance(statement, dict) else None
            )
            if not isinstance(attested_subjects, list):
                raise BundleError(
                    "GitHub attestation verification result has no subject set"
                )
            observed: dict[str, str] = {}
            for subject_entry in attested_subjects:
                if not isinstance(subject_entry, dict):
                    raise BundleError("GitHub attestation contains an invalid subject")
                subject_name = subject_entry.get("name")
                digest = subject_entry.get("digest")
                sha256 = digest.get("sha256") if isinstance(digest, dict) else None
                if (
                    not isinstance(subject_name, str)
                    or subject_name in observed
                    or not isinstance(sha256, str)
                ):
                    raise BundleError(
                        "GitHub attestation contains an invalid or duplicate subject"
                    )
                observed[subject_name] = sha256.casefold()
            if observed != {
                subject_name: seal.sha256
                for subject_name, seal in subject_seals.items()
            }:
                raise BundleError(
                    "GitHub attestation subject set differs from the promotion files"
                )
        if (
            _read_regular_no_follow(
                verifier,
                MAX_SIDECAR_BYTES,
                "GitHub attestation verifier executable",
            )
            != verifier_bytes
            or _read_regular_no_follow(
                bundle_path,
                MAX_ATTESTATION_BYTES,
                "GitHub promotion attestation bundle",
            )
            != bundle_bytes
        ):
            raise BundleError(
                "GitHub attestation verifier or bundle changed while executing"
            )
        for name, path in normalized_subjects.items():
            bytes_ = _read_regular_no_follow(
                path,
                MAX_SIDECAR_BYTES if name == SIDECAR_FILE else MAX_DESCRIPTOR_BYTES,
                f"GitHub promotion attestation subject {name}",
            )
            _check_sealed_bytes(
                bytes_,
                subject_seals[name],
                f"GitHub promotion attestation subject {name}",
            )


def promotion_attestation_verifier_from_path(
    verifier: Path,
) -> PromotionAttestationVerifier:
    """Measure one operator-authorized GitHub CLI before pinned verification."""

    verifier = _require_absolute_normalized(
        verifier, "GitHub attestation verifier executable"
    )
    verifier_bytes = _read_regular_no_follow(
        verifier, MAX_SIDECAR_BYTES, "GitHub attestation verifier executable"
    )
    verifier_seal = Seal(len(verifier_bytes), _sha256(verifier_bytes))

    def verify(
        bundle_path: Path,
        subjects: dict[str, Path],
        authority: PromotionAuthority,
    ) -> None:
        verify_github_attestation_with_executable(
            verifier, verifier_seal, bundle_path, subjects, authority
        )

    return verify


def _json_no_duplicates(pairs: list[tuple[str, object]]) -> dict[str, object]:
    result: dict[str, object] = {}
    for key, value in pairs:
        if key in result:
            raise BundleError(f"duplicate JSON field {key!r}")
        result[key] = value
    return result


def _parse_json(bytes_: bytes, label: str, maximum: int) -> dict[str, object]:
    if not bytes_ or len(bytes_) > maximum:
        raise BundleError(f"{label} size is outside 1..{maximum} bytes")
    try:
        value = json.loads(
            bytes_.decode("utf-8"), object_pairs_hook=_json_no_duplicates
        )
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise BundleError(f"{label} is not strict UTF-8 JSON: {error}") from error
    if not isinstance(value, dict):
        raise BundleError(f"{label} must be one JSON object")
    return value


def _canonical_pretty(value: object) -> bytes:
    return (json.dumps(value, ensure_ascii=False, indent=2) + "\n").encode("utf-8")


def _serde_compact(value: object) -> bytes:
    """Match serde_json::to_vec for the integer/string/list/object payloads used here."""

    return json.dumps(value, ensure_ascii=False, separators=(",", ":")).encode("utf-8")


def _sha256(bytes_: bytes) -> str:
    return hashlib.sha256(bytes_).hexdigest()


def _domain_json_sha256(domain: bytes, value: object, *, include_length: bool) -> str:
    encoded = _serde_compact(value)
    digest = hashlib.sha256()
    digest.update(domain)
    if include_length:
        digest.update(struct.pack("<Q", len(encoded)))
    digest.update(encoded)
    return digest.hexdigest()


def _require_exact_fields(
    value: dict[str, object], fields: Iterable[str], label: str
) -> None:
    expected = set(fields)
    actual = set(value)
    if actual != expected:
        raise BundleError(
            f"{label} fields differ: missing={sorted(expected - actual)}, "
            f"unknown={sorted(actual - expected)}"
        )


def _require_bool(value: object, expected: bool, label: str) -> None:
    if value is not expected:
        raise BundleError(f"{label} must be {str(expected).lower()}")


def _require_uint(value: object, label: str, *, maximum: int = (1 << 64) - 1) -> int:
    if (
        isinstance(value, bool)
        or not isinstance(value, int)
        or not 0 < value <= maximum
    ):
        raise BundleError(f"{label} must be an integer in 1..{maximum}")
    return value


def _require_nonnegative_uint(
    value: object, label: str, *, maximum: int = (1 << 64) - 1
) -> int:
    if (
        isinstance(value, bool)
        or not isinstance(value, int)
        or not 0 <= value <= maximum
    ):
        raise BundleError(f"{label} must be an integer in 0..{maximum}")
    return value


def _require_boolean(value: object, label: str) -> bool:
    if not isinstance(value, bool):
        raise BundleError(f"{label} must be a boolean")
    return value


def _require_hex(value: object, digits: int, label: str) -> str:
    if not isinstance(value, str) or len(value) != digits:
        raise BundleError(
            f"{label} must contain exactly {digits} hexadecimal characters"
        )
    try:
        int(value, 16)
    except ValueError as error:
        raise BundleError(f"{label} is not hexadecimal") from error
    if set(value) <= {"0"}:
        raise BundleError(f"{label} must not be the zero digest")
    return value.casefold()


def _safe_relative(value: object, label: str) -> str:
    if not isinstance(value, str) or not value or len(value.encode("utf-8")) > 512:
        raise BundleError(f"{label} is empty or too long")
    if "\\" in value or ":" in value or "\0" in value:
        raise BundleError(f"{label} is not a slash-separated relative path")
    path = PurePosixPath(value)
    if path.is_absolute() or any(part in ("", ".", "..") for part in path.parts):
        raise BundleError(f"{label} is not a normalized relative path")
    for part in path.parts:
        stem = part.split(".", 1)[0].casefold()
        if (
            len(part.encode("utf-8")) > 128
            or part[-1:] in (".", " ")
            or stem in {"con", "prn", "aux", "nul"}
            or stem.startswith("com")
            and stem[3:].isdigit()
            or stem.startswith("lpt")
            and stem[3:].isdigit()
            or not all(
                character.isascii() and (character.isalnum() or character in "-_.")
                for character in part
            )
        ):
            raise BundleError(f"{label} contains an unsafe component {part!r}")
    return value


def _require_absolute_normalized(path: Path, label: str) -> Path:
    if not path.is_absolute() or any(part in (".", "..") for part in path.parts):
        raise BundleError(f"{label} must be absolute and lexically normalized: {path}")
    return path


def _is_reparse(status: os.stat_result) -> bool:
    return bool(
        getattr(status, "st_file_attributes", 0) & _FILE_ATTRIBUTE_REPARSE_POINT
    )


def _check_no_follow_chain(path: Path, label: str) -> None:
    _require_absolute_normalized(path, label)
    existing: list[Path] = []
    current = path
    while True:
        if current.exists() or current.is_symlink():
            existing.append(current)
        parent = current.parent
        if parent == current:
            break
        current = parent
    for component in reversed(existing):
        try:
            status = component.lstat()
        except OSError as error:
            raise BundleError(
                f"cannot inspect {label} component {component}: {error}"
            ) from error
        if component.is_symlink() or _is_reparse(status):
            raise BundleError(
                f"{label} traverses a forbidden reparse point: {component}"
            )


def _read_regular_no_follow(path: Path, maximum: int, label: str) -> bytes:
    _check_no_follow_chain(path, label)
    flags = os.O_RDONLY | getattr(os, "O_BINARY", 0) | getattr(os, "O_NOFOLLOW", 0)
    try:
        descriptor = os.open(path, flags)
    except OSError as error:
        raise BundleError(
            f"cannot open {label} without following links: {path}: {error}"
        ) from error
    try:
        before = os.fstat(descriptor)
        if (
            not stat.S_ISREG(before.st_mode)
            or before.st_nlink != 1
            or _is_reparse(before)
        ):
            raise BundleError(
                f"{label} must be a non-reparse, single-link regular file: {path}"
            )
        if not 0 < before.st_size <= maximum:
            raise BundleError(f"{label} size is outside 1..{maximum} bytes: {path}")
        chunks: list[bytes] = []
        total = 0
        while True:
            chunk = os.read(descriptor, min(1024 * 1024, maximum + 1 - total))
            if not chunk:
                break
            chunks.append(chunk)
            total += len(chunk)
            if total > maximum:
                raise BundleError(f"{label} exceeds {maximum} bytes: {path}")
        after = os.fstat(descriptor)
        if (
            total != before.st_size
            or after.st_size != before.st_size
            or after.st_nlink != 1
            or (before.st_ino and after.st_ino != before.st_ino)
            or (before.st_dev and after.st_dev != before.st_dev)
        ):
            raise BundleError(f"{label} changed while held open: {path}")
        return b"".join(chunks)
    finally:
        os.close(descriptor)


def _write_new(path: Path, bytes_: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    flags = (
        os.O_WRONLY
        | os.O_CREAT
        | os.O_EXCL
        | getattr(os, "O_BINARY", 0)
        | getattr(os, "O_NOFOLLOW", 0)
    )
    descriptor = os.open(path, flags, 0o644)
    try:
        view = memoryview(bytes_)
        while view:
            written = os.write(descriptor, view)
            if written <= 0:
                raise BundleError(f"short write while creating {path}")
            view = view[written:]
        os.fsync(descriptor)
        status = os.fstat(descriptor)
        if (
            not stat.S_ISREG(status.st_mode)
            or status.st_nlink != 1
            or _is_reparse(status)
        ):
            raise BundleError(f"new output is not a single-link regular file: {path}")
    finally:
        os.close(descriptor)


def _seal(value: object, label: str, maximum: int) -> Seal:
    if not isinstance(value, dict):
        raise BundleError(f"{label} must be a seal object")
    _require_exact_fields(value, ("byte_len", "sha256"), label)
    return Seal(
        _require_uint(value["byte_len"], f"{label}.byte_len", maximum=maximum),
        _require_hex(value["sha256"], 64, f"{label}.sha256"),
    )


def _check_sealed_bytes(bytes_: bytes, expected: Seal, label: str) -> None:
    if len(bytes_) != expected.byte_len or _sha256(bytes_) != expected.sha256:
        raise BundleError(f"{label} does not match its pinned length/SHA-256")


def _parse_signed_sidecar_identity(
    bytes_: bytes,
    expected_signed_sidecar: Seal,
) -> dict[str, object]:
    identity = _parse_json(
        bytes_, "signed standalone compiler identity", MAX_DESCRIPTOR_BYTES
    )
    _require_exact_fields(
        identity,
        (
            "schema",
            "schema_version",
            "unsigned",
            "signed",
            "request_version",
            "response_version",
        ),
        "signed standalone compiler identity",
    )
    if (
        identity["schema"] != SIGNED_SIDECAR_IDENTITY_SCHEMA
        or identity["schema_version"] != SIGNED_SIDECAR_IDENTITY_SCHEMA_VERSION
    ):
        raise BundleError(
            "signed standalone compiler identity schema/version is unsupported"
        )
    unsigned = _seal(
        identity["unsigned"],
        "unsigned standalone compiler identity",
        MAX_SIDECAR_BYTES,
    )
    signed = _seal(
        identity["signed"],
        "signed standalone compiler identity",
        MAX_SIDECAR_BYTES,
    )
    if signed != expected_signed_sidecar:
        raise BundleError(
            "signed standalone compiler identity names different sidecar bytes"
        )
    if unsigned.sha256 == signed.sha256 or unsigned.byte_len >= signed.byte_len:
        raise BundleError(
            "signed standalone compiler identity does not prove a distinct unsigned input"
        )
    if (
        _require_uint(
            identity["request_version"],
            "signed standalone compiler identity request version",
            maximum=(1 << 32) - 1,
        )
        != PRODUCTION_REQUEST_VERSION
        or _require_uint(
            identity["response_version"],
            "signed standalone compiler identity response version",
            maximum=(1 << 32) - 1,
        )
        != PROTOCOL_RESPONSE_VERSION
    ):
        raise BundleError("signed standalone compiler identity protocol is unsupported")
    return identity


def _parse_promotion_provenance(
    bytes_: bytes,
    identity_seal: Seal,
    *,
    expected_repository: str | None = None,
    expected_commit: str | None = None,
) -> PromotionAuthority:
    provenance = _parse_json(
        bytes_, "compiler promotion provenance", MAX_DESCRIPTOR_BYTES
    )
    if (
        provenance.get("schema") != PROMOTION_PROVENANCE_SCHEMA
        or provenance.get("schema_version") != PROMOTION_PROVENANCE_SCHEMA_VERSION
    ):
        raise BundleError("compiler promotion provenance schema/version is unsupported")
    _require_exact_fields(
        provenance,
        (
            "schema",
            "schema_version",
            "repository",
            "commit",
            "workflow_sha",
            "workflow_run_id",
            "workflow_run_attempt",
            "signed_identity",
        ),
        "compiler promotion provenance",
    )
    repository = _require_repository(
        provenance["repository"], "compiler promotion repository"
    )
    commit = _require_hex(provenance["commit"], 40, "compiler promotion commit")
    workflow_sha = _require_hex(
        provenance["workflow_sha"], 40, "compiler promotion workflow SHA"
    )
    if expected_repository is not None and repository != expected_repository:
        raise BundleError(
            "compiler promotion repository differs from the authorized repository"
        )
    if expected_commit is not None and commit != expected_commit:
        raise BundleError(
            "compiler promotion commit differs from the authorized commit"
        )
    if (
        _seal(
            provenance["signed_identity"],
            "compiler promotion signed-identity seal",
            MAX_DESCRIPTOR_BYTES,
        )
        != identity_seal
    ):
        raise BundleError(
            "compiler promotion provenance names a different signed identity"
        )
    return PromotionAuthority(
        repository=repository,
        commit=commit,
        workflow_sha=workflow_sha,
        workflow_run_id=_require_uint(
            provenance["workflow_run_id"], "compiler promotion workflow run ID"
        ),
        workflow_run_attempt=_require_uint(
            provenance["workflow_run_attempt"],
            "compiler promotion workflow run attempt",
            maximum=(1 << 32) - 1,
        ),
    )


def _target_key(value: dict[str, object]) -> tuple[object, ...]:
    target = value.get("target")
    codeview = value.get("pe_codeview")
    if not isinstance(target, dict) or not isinstance(codeview, dict):
        raise BundleError("catalog target must contain target and pe_codeview objects")
    _require_exact_fields(
        target,
        (
            "steam_app_id",
            "steam_build_id",
            "depot_id",
            "depot_manifest_gid",
            "platform",
            "architecture",
            "build_configuration",
        ),
        "catalog target.target",
    )
    _require_exact_fields(codeview, ("guid", "age"), "catalog target.pe_codeview")
    numbers = (
        _require_uint(
            target["steam_app_id"], "catalog target.steam_app_id", maximum=(1 << 32) - 1
        ),
        _require_uint(target["steam_build_id"], "catalog target.steam_build_id"),
        _require_uint(
            target["depot_id"], "catalog target.depot_id", maximum=(1 << 32) - 1
        ),
        _require_uint(
            target["depot_manifest_gid"], "catalog target.depot_manifest_gid"
        ),
    )
    if (
        target["platform"] != "windows"
        or target["architecture"] != "x86_64"
        or target["build_configuration"] != "shipping"
    ):
        raise BundleError("catalog target must be windows/x86_64/shipping")
    guid = codeview["guid"]
    if (
        not isinstance(guid, str)
        or len(guid) != 36
        or any(guid[index] != "-" for index in (8, 13, 18, 23))
        or any(
            character not in "0123456789abcdefABCDEF"
            for index, character in enumerate(guid)
            if index not in (8, 13, 18, 23)
        )
    ):
        raise BundleError("catalog CodeView GUID is invalid")
    age = _require_uint(codeview["age"], "catalog CodeView age", maximum=(1 << 32) - 1)
    return (*numbers, guid.casefold(), age)


def _target_key_label(key: tuple[object, ...]) -> str:
    return f"BuildID {key[1]} / CodeView {key[4]} age {key[5]}"


def _require_product_profile_targets(catalog: dict[str, object]) -> None:
    profiles = catalog["profiles"]
    assert isinstance(profiles, list)
    actual = tuple(
        _target_key(profile["target"])
        for profile in profiles
        if isinstance(profile, dict)
    )
    required = tuple(
        sorted(
            _target_key(copy.deepcopy(target))
            for target in REQUIRED_PRODUCT_COMPILER_TARGETS_V1
        )
    )
    if actual != required:
        actual_set = set(actual)
        required_set = set(required)
        missing = [_target_key_label(key) for key in required if key not in actual_set]
        extra = [_target_key_label(key) for key in actual if key not in required_set]
        raise BundleError(
            "product compiler target set differs: "
            f"missing={missing}, extra={extra}; exactly BuildIDs 24539464 and 24878692 "
            "are required for this product generation"
        )


def _catalog(value: object) -> dict[str, object]:
    if not isinstance(value, dict):
        raise BundleError("internal input catalog must be an object")
    _require_exact_fields(
        value, ("schema", "schema_version", "sidecar", "profiles"), "catalog"
    )
    if (
        value["schema"] != CATALOG_SCHEMA
        or value["schema_version"] != CATALOG_SCHEMA_VERSION
    ):
        raise BundleError("catalog schema/version is unsupported")
    sidecar = value["sidecar"]
    if not isinstance(sidecar, dict):
        raise BundleError("catalog.sidecar must be an object")
    _require_exact_fields(
        sidecar,
        ("relative_path", "byte_len", "sha256", "protocol", "static_system_only"),
        "catalog.sidecar",
    )
    if (
        _safe_relative(sidecar["relative_path"], "catalog.sidecar.relative_path")
        != SIDECAR_FILE
    ):
        raise BundleError(f"catalog sidecar path must be exactly {SIDECAR_FILE!r}")
    _require_uint(
        sidecar["byte_len"], "catalog.sidecar.byte_len", maximum=MAX_SIDECAR_BYTES
    )
    if sidecar["byte_len"] in STALE_SIDECAR_BYTE_LENGTHS:
        raise BundleError(
            "catalog identifies a specifically retired stale sidecar byte length"
        )
    _require_hex(sidecar["sha256"], 64, "catalog.sidecar.sha256")
    protocol = sidecar["protocol"]
    if not isinstance(protocol, dict):
        raise BundleError("catalog.sidecar.protocol must be an object")
    _require_exact_fields(
        protocol, ("request_version", "response_version"), "catalog.sidecar.protocol"
    )
    if protocol != {
        "request_version": PRODUCTION_REQUEST_VERSION,
        "response_version": PROTOCOL_RESPONSE_VERSION,
    }:
        raise BundleError(
            "product catalog must bind the qualified FullGraph request/response protocol 2/1; "
            "request v1 is legacy-smoke-only"
        )
    _require_bool(
        sidecar["static_system_only"], True, "catalog.sidecar.static_system_only"
    )
    profiles = value["profiles"]
    if not isinstance(profiles, list) or not 0 < len(profiles) <= 64:
        raise BundleError("catalog profiles must contain 1..64 entries")
    previous: tuple[object, ...] | None = None
    paths = {SIDECAR_FILE.casefold()}
    for index, profile in enumerate(profiles):
        if not isinstance(profile, dict):
            raise BundleError(f"catalog profile {index} must be an object")
        _require_exact_fields(
            profile,
            (
                "manifest_relative_path",
                "manifest_byte_len",
                "manifest_sha256",
                "profile_sha256",
                "target",
            ),
            f"catalog profile {index}",
        )
        relative = _safe_relative(
            profile["manifest_relative_path"], f"catalog profile {index} manifest path"
        )
        if not relative.startswith("profiles/") or not relative.endswith(
            "/compiler-profile.json"
        ):
            raise BundleError(
                "catalog profile manifests must use profiles/*/compiler-profile.json"
            )
        if relative.casefold() in paths:
            raise BundleError("catalog paths are not unique")
        paths.add(relative.casefold())
        _require_uint(
            profile["manifest_byte_len"],
            f"catalog profile {index} manifest length",
            maximum=MAX_PROFILE_MANIFEST_BYTES,
        )
        _require_hex(
            profile["manifest_sha256"], 64, f"catalog profile {index} manifest SHA-256"
        )
        _require_hex(
            profile["profile_sha256"], 64, f"catalog profile {index} profile SHA-256"
        )
        key = _target_key(profile["target"])
        if previous is not None and key <= previous:
            raise BundleError("catalog profiles are duplicate or not target-sorted")
        previous = key
    return value


def _read_c_string(bytes_: bytes, offset: int, label: str) -> str:
    if not 0 <= offset < len(bytes_):
        raise BundleError(f"{label} offset is outside the PE")
    end = bytes_.find(b"\0", offset, min(len(bytes_), offset + 512))
    if end < 0:
        raise BundleError(f"{label} is not NUL terminated")
    try:
        return bytes_[offset:end].decode("ascii")
    except UnicodeDecodeError as error:
        raise BundleError(f"{label} is not ASCII") from error


def _pe_layout(bytes_: bytes) -> tuple[int, list[tuple[int, int, int, int]], int, int]:
    if len(bytes_) < 0x100 or bytes_[:2] != b"MZ":
        raise BundleError("sidecar is not a PE image")
    pe = struct.unpack_from("<I", bytes_, 0x3C)[0]
    if pe > min(len(bytes_) - 24, 1024 * 1024) or bytes_[pe : pe + 4] != b"PE\0\0":
        raise BundleError("sidecar has an invalid PE header")
    machine, sections, _, _, _, optional_size, _ = struct.unpack_from(
        "<HHIIIHH", bytes_, pe + 4
    )
    optional = pe + 24
    if (
        machine != _PE_MACHINE_AMD64
        or optional + optional_size > len(bytes_)
        or optional_size < 112
    ):
        raise BundleError("sidecar is not a bounded x64 PE32+ image")
    if struct.unpack_from("<H", bytes_, optional)[0] != _PE32_PLUS_MAGIC:
        raise BundleError("sidecar optional header is not PE32+")
    directories = struct.unpack_from("<I", bytes_, optional + 108)[0]
    section_table = optional + optional_size
    if sections == 0 or sections > 96 or section_table + sections * 40 > len(bytes_):
        raise BundleError("sidecar has an invalid PE section table")
    parsed_sections: list[tuple[int, int, int, int]] = []
    for index in range(sections):
        at = section_table + index * 40
        virtual_size, virtual_address, raw_size, raw_offset = struct.unpack_from(
            "<IIII", bytes_, at + 8
        )
        if raw_offset + raw_size > len(bytes_):
            raise BundleError("sidecar PE section exceeds the file")
        parsed_sections.append(
            (virtual_address, max(virtual_size, raw_size), raw_offset, raw_size)
        )
    return optional, parsed_sections, directories, optional_size


def _pe_directory(
    bytes_: bytes, optional: int, directories: int, optional_size: int, index: int
) -> tuple[int, int]:
    if directories <= index or 112 + (index + 1) * 8 > optional_size:
        return (0, 0)
    return struct.unpack_from("<II", bytes_, optional + 112 + index * 8)


def _rva_offset(rva: int, sections: list[tuple[int, int, int, int]], label: str) -> int:
    for virtual_address, span, raw_offset, raw_size in sections:
        if virtual_address <= rva < virtual_address + span:
            delta = rva - virtual_address
            if delta >= raw_size:
                break
            return raw_offset + delta
    raise BundleError(f"{label} RVA is not backed by PE file bytes")


def _authenticode_entry_count(bytes_: bytes) -> int:
    optional, _, directories, optional_size = _pe_layout(bytes_)
    offset, size = _pe_directory(
        bytes_, optional, directories, optional_size, _IMAGE_DIRECTORY_ENTRY_SECURITY
    )
    if offset == 0 or size == 0 or offset + size > len(bytes_):
        return 0
    count = 0
    cursor = offset
    end = offset + size
    while cursor < end:
        if cursor + 8 > end:
            raise BundleError("sidecar Authenticode table is truncated")
        length, revision, certificate_type = struct.unpack_from("<IHH", bytes_, cursor)
        if (
            length < 8
            or cursor + length > end
            or revision != _WIN_CERT_REVISION_2_0
            or certificate_type != _WIN_CERT_TYPE_PKCS_SIGNED_DATA
        ):
            raise BundleError("sidecar has a malformed Authenticode entry")
        count += 1
        cursor += (length + 7) & ~7
    if cursor != end:
        raise BundleError("sidecar Authenticode table alignment is invalid")
    return count


def _verify_static_imports(bytes_: bytes) -> None:
    optional, sections, directories, optional_size = _pe_layout(bytes_)
    delay_rva, delay_size = _pe_directory(
        bytes_,
        optional,
        directories,
        optional_size,
        _IMAGE_DIRECTORY_ENTRY_DELAY_IMPORT,
    )
    if delay_rva or delay_size:
        raise BundleError("sidecar delay-import table is forbidden")
    import_rva, import_size = _pe_directory(
        bytes_, optional, directories, optional_size, _IMAGE_DIRECTORY_ENTRY_IMPORT
    )
    if import_rva == 0 or import_size < 20:
        raise BundleError("sidecar has no bounded static import table")
    table = _rva_offset(import_rva, sections, "import directory")
    end = min(len(bytes_), table + import_size)
    terminated = False
    for descriptor in range(table, end, 20):
        if descriptor + 20 > end:
            break
        original_thunk, _, _, name_rva, first_thunk = struct.unpack_from(
            "<IIIII", bytes_, descriptor
        )
        if not any((original_thunk, name_rva, first_thunk)):
            terminated = True
            break
        dll = _read_c_string(
            bytes_, _rva_offset(name_rva, sections, "import DLL"), "import DLL"
        )
        dll_key = dll.casefold()
        if not (
            dll_key in _FIXED_SYSTEM_DLLS
            or dll_key.startswith("api-ms-win-")
            or dll_key.startswith("ext-ms-win-")
        ):
            raise BundleError(f"sidecar imports non-system DLL {dll!r}")
        thunk_rva = original_thunk or first_thunk
        thunk = _rva_offset(thunk_rva, sections, "import thunk")
        for index in range(65536):
            at = thunk + index * 8
            if at + 8 > len(bytes_):
                raise BundleError("sidecar import thunk is truncated")
            value = struct.unpack_from("<Q", bytes_, at)[0]
            if value == 0:
                break
            if value & (1 << 63):
                continue
            _read_c_string(
                bytes_, _rva_offset(value, sections, "import name") + 2, "import name"
            )
        else:
            raise BundleError("sidecar import thunk is unterminated")
    if not terminated:
        raise BundleError("sidecar import descriptor table is unterminated")


def _verify_authenticode_windows(path: Path) -> None:
    if os.name != "nt":
        raise BundleError(
            "Authenticode verification of a qualified sidecar requires Windows"
        )
    shell = shutil.which("pwsh.exe") or shutil.which("powershell.exe")
    if shell is None:
        raise BundleError("Authenticode verification requires PowerShell on Windows")
    path_variable = "GORE_AUTHENTICODE_VERIFY_LITERAL_PATH"
    command = (
        f"$signature = Get-AuthenticodeSignature -LiteralPath $env:{path_variable}; "
        "if ($signature.Status -ne 'Valid') { "
        "Write-Error ('Authenticode status: ' + $signature.Status + ' ' + $signature.StatusMessage); exit 1 }"
    )
    environment = os.environ.copy()
    environment[path_variable] = str(path)
    completed = subprocess.run(
        [shell, "-NoProfile", "-NonInteractive", "-Command", command],
        capture_output=True,
        text=True,
        env=environment,
    )
    if completed.returncode != 0:
        detail = (completed.stderr or completed.stdout).strip()
        raise BundleError(f"sidecar Authenticode verification failed: {detail}")


def verify_sidecar(path: Path, bytes_: bytes) -> None:
    path = _require_absolute_normalized(path, "signed sidecar")
    with _pin_windows_file_path(path, "signed sidecar"):
        observed = _read_regular_no_follow(path, MAX_SIDECAR_BYTES, "signed sidecar")
        if observed != bytes_:
            raise BundleError(
                "signed sidecar path differs from the bytes selected for verification"
            )
        if len(observed) in STALE_SIDECAR_BYTE_LENGTHS:
            raise BundleError("retired 273408-byte standalone sidecar is forbidden")
        _verify_static_imports(observed)
        if _authenticode_entry_count(observed) != 1:
            raise BundleError(
                "distributable sidecar must contain exactly one Authenticode entry"
            )
        _verify_authenticode_windows(path)
        if (
            _read_regular_no_follow(path, MAX_SIDECAR_BYTES, "signed sidecar")
            != observed
        ):
            raise BundleError("signed sidecar changed during Authenticode verification")


def _verify_production_capabilities(path: Path) -> None:
    try:
        completed = subprocess.run(
            [str(path), "--capabilities"],
            capture_output=True,
            timeout=10,
        )
    except (OSError, subprocess.SubprocessError) as error:
        raise BundleError(f"cannot query sidecar capabilities: {error}") from error
    if completed.returncode != 0 or completed.stderr:
        detail = (completed.stderr or completed.stdout)[:4096].decode(
            "utf-8", errors="replace"
        )
        raise BundleError(f"sidecar capabilities query failed: {detail.strip()}")
    capabilities = _parse_json(
        completed.stdout, "sidecar capabilities", MAX_CATALOG_BYTES
    )
    if (
        capabilities.get("backend") != "gore-as-standalone-compiler"
        or capabilities.get("request_version") != PRODUCTION_REQUEST_VERSION
        or capabilities.get("request_versions")
        != [LEGACY_SMOKE_REQUEST_VERSION, PRODUCTION_REQUEST_VERSION]
        or capabilities.get("response_version") != PROTOCOL_RESPONSE_VERSION
    ):
        raise BundleError(
            "sidecar does not advertise the required FullGraph 2/1 production protocol"
        )
    compile_capabilities = capabilities.get("compile")
    if not isinstance(compile_capabilities, dict) or any(
        compile_capabilities.get(field) is not expected
        for field, expected in (
            ("available", True),
            ("requires_qualified_profile", True),
            ("requires_unreal_runtime", False),
            ("requires_game_dll", False),
        )
    ):
        raise BundleError(
            "sidecar capabilities do not prove the standalone compile boundary"
        )


def _verify_pinned_production_capabilities(path: Path, expected: bytes) -> None:
    """Execute capabilities only while the complete measured path is locked."""

    with _pin_windows_file_path(path, "standalone compiler capability executable"):
        if (
            _read_regular_no_follow(
                path, MAX_SIDECAR_BYTES, "standalone compiler capability executable"
            )
            != expected
        ):
            raise BundleError(
                "standalone compiler capability path differs from the measured bytes"
            )
        _verify_production_capabilities(path)
        if (
            _read_regular_no_follow(
                path, MAX_SIDECAR_BYTES, "standalone compiler capability executable"
            )
            != expected
        ):
            raise BundleError(
                "standalone compiler changed while its capabilities were queried"
            )


def _profile_blob_seals(profile: dict[str, object]) -> list[tuple[str, Seal, str]]:
    blobs: list[tuple[str, Seal, str]] = []
    seen: dict[str, Seal] = {}
    aggregate = 0
    for group_name, field_name in PROFILE_BLOB_FIELDS:
        group = profile.get(group_name)
        if not isinstance(group, dict):
            raise BundleError(f"compiler profile {group_name} must be an object")
        blob = group.get(field_name)
        label = f"compiler profile {group_name}.{field_name}"
        if not isinstance(blob, dict):
            raise BundleError(f"{label} must be a blob seal")
        _require_exact_fields(blob, ("path", "byte_len", "sha256"), label)
        relative = _safe_relative(blob["path"], f"{label}.path")
        seal = Seal(
            _require_uint(
                blob["byte_len"], f"{label}.byte_len", maximum=MAX_PROFILE_BLOB_BYTES
            ),
            _require_hex(blob["sha256"], 64, f"{label}.sha256"),
        )
        previous = seen.get(relative.casefold())
        if previous is not None and previous != seal:
            raise BundleError(
                f"compiler profile aliases {relative!r} with conflicting seals"
            )
        seen[relative.casefold()] = seal
        aggregate += seal.byte_len
        if aggregate > MAX_PROFILE_AGGREGATE_BYTES:
            raise BundleError(
                "compiler profile payloads exceed the aggregate byte limit"
            )
        blobs.append((relative, seal, label))
    return blobs


def _sidecar_identity(sidecar: dict[str, object]) -> dict[str, object]:
    protocol = sidecar["protocol"]
    assert isinstance(protocol, dict)
    return {
        "byte_len": sidecar["byte_len"],
        "sha256": str(sidecar["sha256"]).casefold(),
        "request_version": protocol["request_version"],
        "response_version": protocol["response_version"],
    }


def _profile_oracle_seal(profile: dict[str, object], field: str) -> Seal:
    oracle = profile.get("oracle")
    if not isinstance(oracle, dict):
        raise BundleError("qualified compiler profile oracle must be an object")
    value = oracle.get(field)
    if not isinstance(value, dict):
        raise BundleError(f"qualified compiler profile oracle.{field} must be a file seal")
    return Seal(
        _require_uint(
            value.get("byte_len"),
            f"qualified compiler profile oracle.{field}.byte_len",
            maximum=MAX_PROFILE_BLOB_BYTES,
        ),
        _require_hex(
            value.get("sha256"),
            64,
            f"qualified compiler profile oracle.{field}.sha256",
        ),
    )


def _full_tree_receipt_relative(catalog_entry: dict[str, object]) -> str:
    target = catalog_entry["target"]
    assert isinstance(target, dict)
    target_tuple = target["target"]
    codeview = target["pe_codeview"]
    assert isinstance(target_tuple, dict) and isinstance(codeview, dict)
    relative = (
        f"{FULL_TREE_VERIFICATION_DIRECTORY}/build-"
        f"{target_tuple['steam_build_id']}-{str(codeview['guid']).casefold()}.json"
    )
    return _safe_relative(relative, "full-tree verification receipt path")


def _parse_full_tree_receipt(
    bytes_: bytes,
    catalog_entry: dict[str, object],
    profile: dict[str, object],
    sidecar: dict[str, object],
) -> FullTreeVerificationAuthority:
    receipt = _parse_json(
        bytes_, "full-tree verification receipt", MAX_FULL_TREE_RECEIPT_BYTES
    )
    if bytes_ != _canonical_pretty(receipt):
        raise BundleError("full-tree verification receipt is not canonical pretty JSON")
    _require_exact_fields(
        receipt,
        (
            "schema",
            "version",
            "passed",
            "execution",
            "authority",
            "frozen_source",
            "embedded_reference",
            "standalone_candidate",
            "bytediff",
            "whole_cache_semantics_v1",
        ),
        "full-tree verification receipt",
    )
    if (
        receipt["schema"] != FULL_TREE_RECEIPT_SCHEMA
        or _require_uint(
            receipt["version"],
            "full-tree verification receipt version",
            maximum=(1 << 32) - 1,
        )
        != FULL_TREE_RECEIPT_VERSION
    ):
        raise BundleError("full-tree verification receipt schema/version is unsupported")
    _require_bool(receipt["passed"], True, "full-tree verification receipt passed")

    execution = receipt["execution"]
    if not isinstance(execution, dict):
        raise BundleError("full-tree verification execution must be an object")
    _require_exact_fields(
        execution,
        (
            "backend",
            "runner_invocations",
            "standalone_attempted",
            "game_attempted",
            "install_restore",
            "closing_audit",
            "publication",
            "recovery_required",
            "fallback_present",
            "backend_diagnostic_count",
        ),
        "full-tree verification execution",
    )
    if (
        execution["backend"] != "standalone"
        or _require_uint(
            execution["runner_invocations"],
            "full-tree verification runner invocations",
            maximum=(1 << 32) - 1,
        )
        != 1
        or execution["install_restore"] != "not_started"
        or execution["closing_audit"] != "passed"
        or execution["publication"] != "published"
    ):
        raise BundleError(
            "full-tree verification did not use exactly one strict standalone publication"
        )
    _require_bool(
        execution["standalone_attempted"],
        True,
        "full-tree verification standalone_attempted",
    )
    _require_bool(
        execution["game_attempted"], False, "full-tree verification game_attempted"
    )
    _require_bool(
        execution["recovery_required"],
        False,
        "full-tree verification recovery_required",
    )
    _require_bool(
        execution["fallback_present"],
        False,
        "full-tree verification fallback_present",
    )
    _require_nonnegative_uint(
        execution["backend_diagnostic_count"],
        "full-tree verification backend diagnostic count",
        maximum=(1 << 32) - 1,
    )

    authority = receipt["authority"]
    if not isinstance(authority, dict):
        raise BundleError("full-tree verification authority must be an object")
    _require_exact_fields(
        authority,
        ("qualified_profile_sha256", "sidecar", "shipping", "binds"),
        "full-tree verification authority",
    )
    expected_profile_sha256 = _require_hex(
        catalog_entry["profile_sha256"], 64, "catalog profile SHA-256"
    )
    profile_sha256 = _require_hex(
        authority["qualified_profile_sha256"],
        64,
        "full-tree verification qualified profile SHA-256",
    )
    if profile_sha256 != expected_profile_sha256:
        raise BundleError("full-tree verification names a different qualified profile")

    receipt_sidecar = authority["sidecar"]
    if not isinstance(receipt_sidecar, dict):
        raise BundleError("full-tree verification sidecar authority must be an object")
    _require_exact_fields(
        receipt_sidecar,
        ("byte_len", "sha256", "request_version", "response_version"),
        "full-tree verification sidecar authority",
    )
    parsed_sidecar_identity = {
        "byte_len": _require_uint(
            receipt_sidecar["byte_len"],
            "full-tree verification sidecar byte length",
            maximum=MAX_SIDECAR_BYTES,
        ),
        "sha256": _require_hex(
            receipt_sidecar["sha256"],
            64,
            "full-tree verification sidecar SHA-256",
        ),
        "request_version": _require_uint(
            receipt_sidecar["request_version"],
            "full-tree verification sidecar request version",
            maximum=(1 << 32) - 1,
        ),
        "response_version": _require_uint(
            receipt_sidecar["response_version"],
            "full-tree verification sidecar response version",
            maximum=(1 << 32) - 1,
        ),
    }
    if parsed_sidecar_identity != _sidecar_identity(sidecar):
        raise BundleError(
            "full-tree verification names different final sidecar bytes or protocol"
        )
    sidecar_seal = Seal(
        int(parsed_sidecar_identity["byte_len"]),
        str(parsed_sidecar_identity["sha256"]),
    )

    shipping = _seal(
        authority["shipping"],
        "full-tree verification Shipping authority",
        MAX_PROFILE_BLOB_BYTES,
    )
    binds = _seal(
        authority["binds"],
        "full-tree verification Binds authority",
        MAX_PROFILE_BLOB_BYTES,
    )
    if shipping != _profile_oracle_seal(profile, "shipping_cache"):
        raise BundleError(
            "full-tree verification Shipping authority differs from the qualified profile"
        )
    if binds != _profile_oracle_seal(profile, "binds_cache"):
        raise BundleError(
            "full-tree verification Binds authority differs from the qualified profile"
        )

    frozen = receipt["frozen_source"]
    if not isinstance(frozen, dict):
        raise BundleError("full-tree verification frozen source must be an object")
    _require_exact_fields(
        frozen,
        ("module_count", "byte_len", "aggregate_sha256", "operations"),
        "full-tree verification frozen source",
    )
    module_count = _require_uint(
        frozen["module_count"],
        "full-tree verification frozen source module count",
        maximum=(1 << 32) - 1,
    )
    frozen_source = Seal(
        _require_uint(
            frozen["byte_len"],
            "full-tree verification frozen source byte length",
            maximum=MAX_PROFILE_AGGREGATE_BYTES,
        ),
        _require_hex(
            frozen["aggregate_sha256"],
            64,
            "full-tree verification frozen source aggregate SHA-256",
        ),
    )
    operations = frozen["operations"]
    if not isinstance(operations, dict):
        raise BundleError("full-tree verification operations must be an object")
    _require_exact_fields(
        operations,
        ("add", "edit", "delete"),
        "full-tree verification operations",
    )
    adds = _require_nonnegative_uint(
        operations["add"], "full-tree verification add count", maximum=(1 << 32) - 1
    )
    edits = _require_nonnegative_uint(
        operations["edit"],
        "full-tree verification edit count",
        maximum=(1 << 32) - 1,
    )
    deletes = _require_nonnegative_uint(
        operations["delete"],
        "full-tree verification delete count",
        maximum=(1 << 32) - 1,
    )
    if (adds, edits, deletes) != (0, module_count, 0):
        raise BundleError(
            "full-tree verification frozen source must be the exact edit-only base module universe"
        )

    embedded_reference = _seal(
        receipt["embedded_reference"],
        "full-tree verification embedded reference",
        MAX_PROFILE_BLOB_BYTES,
    )
    candidate_value = receipt["standalone_candidate"]
    if not isinstance(candidate_value, dict):
        raise BundleError("full-tree verification standalone candidate must be an object")
    _require_exact_fields(
        candidate_value,
        ("byte_len", "sha256", "module_count"),
        "full-tree verification standalone candidate",
    )
    standalone_candidate = Seal(
        _require_uint(
            candidate_value["byte_len"],
            "full-tree verification standalone candidate byte length",
            maximum=MAX_PROFILE_BLOB_BYTES,
        ),
        _require_hex(
            candidate_value["sha256"],
            64,
            "full-tree verification standalone candidate SHA-256",
        ),
    )
    if (
        _require_uint(
            candidate_value["module_count"],
            "full-tree verification standalone candidate module count",
            maximum=(1 << 32) - 1,
        )
        != module_count
    ):
        raise BundleError(
            "full-tree verification candidate module count differs from frozen source"
        )

    bytediff = receipt["bytediff"]
    if not isinstance(bytediff, dict):
        raise BundleError("full-tree verification bytediff must be an object")
    _require_exact_fields(
        bytediff,
        (
            "equivalent_to_fail_on_semantic",
            "context",
            "normalization",
            "aligned_functions",
            "identical",
            "benign",
            "semantic",
            "alignment_loss",
        ),
        "full-tree verification bytediff",
    )
    _require_bool(
        bytediff["equivalent_to_fail_on_semantic"],
        True,
        "full-tree verification fail-on-semantic equivalence",
    )
    if (
        _require_uint(
            bytediff["context"],
            "full-tree verification bytediff context",
            maximum=(1 << 32) - 1,
        )
        != 6
    ):
        raise BundleError("full-tree verification bytediff context must be 6")
    normalization = bytediff["normalization"]
    expected_normalization = {
        "n1_refs": True,
        "n2_slots": False,
        "n3_jumps": True,
        "n4_consts": True,
        "n5_scope": True,
        "n6_reguard": True,
    }
    if not isinstance(normalization, dict):
        raise BundleError("full-tree verification normalization must be an object")
    _require_exact_fields(
        normalization,
        expected_normalization,
        "full-tree verification normalization",
    )
    for field, expected in expected_normalization.items():
        _require_bool(
            normalization[field], expected, f"full-tree verification normalization {field}"
        )
    aligned = _require_nonnegative_uint(
        bytediff["aligned_functions"],
        "full-tree verification aligned function count",
    )
    identical = _require_nonnegative_uint(
        bytediff["identical"], "full-tree verification identical function count"
    )
    benign = _require_nonnegative_uint(
        bytediff["benign"], "full-tree verification benign function count"
    )
    semantic = _require_nonnegative_uint(
        bytediff["semantic"], "full-tree verification semantic function count"
    )
    alignment_loss = _require_nonnegative_uint(
        bytediff["alignment_loss"], "full-tree verification alignment-loss count"
    )
    if aligned != identical + benign + semantic:
        raise BundleError("full-tree verification bytediff counts are inconsistent")
    if semantic != 0 or alignment_loss != 0:
        raise BundleError(
            "full-tree verification must have zero semantic and alignment-loss diffs"
        )

    whole = receipt["whole_cache_semantics_v1"]
    if not isinstance(whole, dict):
        raise BundleError("full-tree WholeCache semantics must be an object")
    count_fields = (
        "function_count",
        "class_count",
        "behaviour_function_count",
        "property_count",
        "global_count",
        "initializer_function_count",
        "string_global_reference_count",
    )
    _require_exact_fields(
        whole,
        (
            "exact_struct_equality",
            "semantic_sha256",
            "module_count",
            "function_count",
            "opcode_counts",
            "class_count",
            "behaviour_function_count",
            "property_count",
            "global_count",
            "initializer_function_count",
            "string_global_reference_count",
            "tail_table_counts",
            "invoke_return_included",
        ),
        "full-tree WholeCache semantics",
    )
    _require_bool(
        whole["exact_struct_equality"],
        True,
        "full-tree WholeCache exact structural equality",
    )
    _require_hex(
        whole["semantic_sha256"], 64, "full-tree WholeCache semantic SHA-256"
    )
    if (
        _require_uint(
            whole["module_count"],
            "full-tree WholeCache module count",
            maximum=(1 << 32) - 1,
        )
        != module_count
    ):
        raise BundleError("full-tree WholeCache module count differs from frozen source")
    for field in count_fields:
        _require_nonnegative_uint(
            whole[field], f"full-tree WholeCache {field.replace('_', ' ')}"
        )
    opcode_counts = whole["opcode_counts"]
    if not isinstance(opcode_counts, list) or len(opcode_counts) != 213:
        raise BundleError("full-tree WholeCache opcode counts must contain exactly 213 rows")
    for index, count in enumerate(opcode_counts):
        _require_nonnegative_uint(
            count, f"full-tree WholeCache opcode count {index}"
        )
    tail_counts = whole["tail_table_counts"]
    if not isinstance(tail_counts, list) or len(tail_counts) != 7:
        raise BundleError("full-tree WholeCache tail table counts must contain 7 rows")
    for index, count in enumerate(tail_counts):
        _require_nonnegative_uint(
            count,
            f"full-tree WholeCache tail table count {index}",
            maximum=(1 << 32) - 1,
        )
    _require_boolean(
        whole["invoke_return_included"],
        "full-tree WholeCache invoke-return flag",
    )
    return FullTreeVerificationAuthority(
        profile_sha256=profile_sha256,
        sidecar=sidecar_seal,
        shipping=shipping,
        binds=binds,
        frozen_source=frozen_source,
        embedded_reference=embedded_reference,
        standalone_candidate=standalone_candidate,
        module_count=module_count,
    )


def _verify_qualification_identity(
    root: Path,
    profile: dict[str, object],
    blobs: list[tuple[str, Seal, str]],
    expected_sidecar: dict[str, object],
) -> None:
    qualification = profile.get("qualification")
    if (
        not isinstance(qualification, dict)
        or qualification.get("qualified") is not True
    ):
        raise BundleError("compiler profile is not qualified")
    blob_by_path = {relative: seal for relative, seal, _ in blobs}
    for field in ("diagnostic_parity", "semantic_parity"):
        blob = qualification.get(field)
        if not isinstance(blob, dict) or not isinstance(blob.get("path"), str):
            raise BundleError(f"compiler qualification {field} is invalid")
        relative = str(blob["path"])
        bytes_ = _read_regular_no_follow(
            root.joinpath(*PurePosixPath(relative).parts),
            MAX_PROFILE_BLOB_BYTES,
            f"compiler qualification {field}",
        )
        _check_sealed_bytes(
            bytes_, blob_by_path[relative], f"compiler qualification {field}"
        )
        report = _parse_json(
            bytes_, f"compiler qualification {field}", MAX_PROFILE_BLOB_BYTES
        )
        if report.get("standalone_compiler") != expected_sidecar:
            raise BundleError(
                f"compiler qualification {field} identifies a different signed sidecar/protocol"
            )


def _offline_artifact_authority_summary(
    manifest_bytes: bytes, expected_backend: str
) -> dict[str, object]:
    manifest = _parse_json(
        manifest_bytes,
        f"{expected_backend} qualification artifacts",
        MAX_PROFILE_BLOB_BYTES,
    )
    _require_exact_fields(
        manifest,
        (
            "schema",
            "schema_version",
            "semantic_observer",
            "suite_id",
            "corpus_sha256",
            "backend",
            "source_profile_sha256",
            "source_target",
            "standalone_compiler",
            "entries",
            "canonical_sha256",
        ),
        f"{expected_backend} qualification artifacts",
    )
    if (
        manifest["schema"] != "gore.as.offline-probe-artifacts"
        or manifest["schema_version"] != 1
        or manifest["semantic_observer"] != "gore.as.whole-cache-semantic-observer/v1"
        or manifest["backend"] != expected_backend
        or not isinstance(manifest["entries"], list)
        or not manifest["entries"]
    ):
        raise BundleError(
            f"{expected_backend} qualification artifact manifest is invalid"
        )
    source_profile_sha256 = _require_hex(
        manifest["source_profile_sha256"],
        64,
        f"{expected_backend} qualification source profile SHA-256",
    )
    if source_profile_sha256 == "0" * 64:
        raise BundleError(
            f"{expected_backend} qualification source profile SHA-256 is zero"
        )
    source_target = manifest["source_target"]
    if not isinstance(source_target, dict):
        raise BundleError(f"{expected_backend} qualification source target is invalid")
    _require_exact_fields(
        source_target,
        (
            "steam_app_id",
            "steam_build_id",
            "depot_id",
            "depot_manifest_gid",
            "platform",
            "architecture",
            "build_configuration",
        ),
        f"{expected_backend} qualification source target",
    )
    for field, maximum in (
        ("steam_app_id", (1 << 32) - 1),
        ("steam_build_id", (1 << 64) - 1),
        ("depot_id", (1 << 32) - 1),
        ("depot_manifest_gid", (1 << 64) - 1),
    ):
        if (
            _require_uint(
                source_target[field],
                f"{expected_backend} qualification source target {field}",
                maximum=maximum,
            )
            == 0
        ):
            raise BundleError(
                f"{expected_backend} qualification source target {field} is zero"
            )
    if (
        source_target["platform"] != "windows"
        or source_target["architecture"] != "x86_64"
        or source_target["build_configuration"] != "shipping"
    ):
        raise BundleError(
            f"{expected_backend} qualification source target platform is invalid"
        )
    standalone_compiler = manifest["standalone_compiler"]
    if expected_backend == "embedded_game":
        if standalone_compiler is not None:
            raise BundleError(
                "embedded qualification cannot identify a standalone compiler"
            )
    else:
        if not isinstance(standalone_compiler, dict):
            raise BundleError("standalone qualification omits its compiler identity")
        _require_exact_fields(
            standalone_compiler,
            ("byte_len", "sha256", "request_version", "response_version"),
            "standalone qualification compiler identity",
        )
        if (
            _require_uint(
                standalone_compiler["byte_len"],
                "standalone qualification compiler byte length",
            )
            == 0
        ):
            raise BundleError("standalone qualification compiler byte length is zero")
        _require_hex(
            standalone_compiler["sha256"],
            64,
            "standalone qualification compiler SHA-256",
        )
        for field in ("request_version", "response_version"):
            if (
                _require_uint(
                    standalone_compiler[field],
                    f"standalone qualification compiler {field}",
                    maximum=(1 << 32) - 1,
                )
                == 0
            ):
                raise BundleError(f"standalone qualification compiler {field} is zero")
    canonical = _require_hex(
        manifest["canonical_sha256"],
        64,
        f"{expected_backend} qualification artifact canonical SHA-256",
    )
    canonical_payload = copy.deepcopy(manifest)
    canonical_payload["canonical_sha256"] = "0" * 64
    if canonical != _domain_json_sha256(
        b"gore-as-offline-probe-artifacts-v1\0",
        canonical_payload,
        include_length=True,
    ):
        raise BundleError(
            f"{expected_backend} qualification artifact manifest canonical seal differs"
        )

    cache_seals: list[dict[str, object]] = []
    supplemental: list[dict[str, object]] = []
    for ordinal, entry in enumerate(manifest["entries"]):
        if not isinstance(entry, dict) or entry.get("ordinal") != ordinal:
            raise BundleError(
                f"{expected_backend} qualification artifact entry order is invalid"
            )
        case_id = entry.get("case_id")
        if not isinstance(case_id, str) or not case_id:
            raise BundleError(
                f"{expected_backend} qualification artifact case identity is invalid"
            )
        outcome = entry.get("outcome")
        if outcome not in ("accepted", "rejected"):
            raise BundleError(
                f"{expected_backend} qualification artifact outcome is invalid"
            )
        cache = entry.get("cache")
        if (outcome == "accepted") != (cache is not None):
            raise BundleError(
                f"{expected_backend} qualification accepted/cache shape differs"
            )
        if cache is not None:
            if not isinstance(cache, dict):
                raise BundleError(
                    f"{expected_backend} qualification artifact cache seal is invalid"
                )
            _require_exact_fields(
                cache,
                ("blob_id", "byte_len", "sha256"),
                f"{expected_backend} qualification cache seal",
            )
            if not isinstance(cache["blob_id"], str) or not cache["blob_id"]:
                raise BundleError(
                    f"{expected_backend} qualification cache blob id is invalid"
                )
            _require_uint(
                cache["byte_len"], f"{expected_backend} qualification cache byte length"
            )
            _require_hex(
                cache["sha256"], 64, f"{expected_backend} qualification cache SHA-256"
            )
            cache_seals.append(
                {
                    "case_id": case_id,
                    "artifact_role": "accepted_final",
                    "cache": cache,
                }
            )
        graph_transition = entry.get("graph_transition")
        if graph_transition is not None:
            if not isinstance(graph_transition, dict):
                raise BundleError(
                    f"{expected_backend} qualification graph transition is invalid"
                )
            baseline_cache = graph_transition.get("baseline_cache")
            if not isinstance(baseline_cache, dict):
                raise BundleError(
                    f"{expected_backend} qualification graph baseline cache is invalid"
                )
            _require_exact_fields(
                baseline_cache,
                ("blob_id", "byte_len", "sha256"),
                f"{expected_backend} qualification graph baseline cache",
            )
            if (
                not isinstance(baseline_cache["blob_id"], str)
                or not baseline_cache["blob_id"]
            ):
                raise BundleError(
                    f"{expected_backend} qualification graph baseline blob id is invalid"
                )
            _require_uint(
                baseline_cache["byte_len"],
                f"{expected_backend} qualification graph baseline byte length",
            )
            _require_hex(
                baseline_cache["sha256"],
                64,
                f"{expected_backend} qualification graph baseline SHA-256",
            )
            cache_seals.append(
                {
                    "case_id": case_id,
                    "artifact_role": "graph_baseline",
                    "cache": baseline_cache,
                }
            )
        supplemental.append(
            {
                "case_id": case_id,
                "frontend_coverage": entry.get("frontend_coverage"),
                "graph_transition": graph_transition,
                "compiler_build_flags": entry.get("compiler_build_flags"),
            }
        )
    if not cache_seals:
        raise BundleError(
            f"{expected_backend} qualification artifacts contain no accepted cache seal"
        )
    return {
        "backend": expected_backend,
        "suite_id": manifest["suite_id"],
        "corpus_sha256": _require_hex(
            manifest["corpus_sha256"],
            64,
            f"{expected_backend} qualification artifact corpus SHA-256",
        ),
        "source_profile_sha256": source_profile_sha256,
        "source_target": source_target,
        "standalone_compiler": standalone_compiler,
        "manifest_canonical_sha256": canonical,
        "manifest_json_sha256": _sha256(manifest_bytes),
        "cache_seals": cache_seals,
        "cache_seals_sha256": _domain_json_sha256(
            b"gore-as-offline-cache-seal-authority-v1\0",
            cache_seals,
            include_length=True,
        ),
        "supplemental_witnesses_sha256": _domain_json_sha256(
            b"gore-as-offline-supplemental-authority-v1\0",
            supplemental,
            include_length=True,
        ),
    }


def _profile_payload_canonical_sha256(
    root: Path,
    profile: dict[str, object],
    group: str,
    field: str,
) -> str:
    group_value = profile.get(group)
    if not isinstance(group_value, dict):
        raise BundleError(f"compiler profile {group} is invalid")
    blob = group_value.get(field)
    if not isinstance(blob, dict) or not isinstance(blob.get("path"), str):
        raise BundleError(f"compiler profile {group}.{field} is invalid")
    path = _safe_relative(blob["path"], f"compiler profile {group}.{field} path")
    payload = _read_regular_no_follow(
        root.joinpath(*PurePosixPath(path).parts),
        MAX_PROFILE_BLOB_BYTES,
        f"compiler profile {group}.{field}",
    )
    document = _parse_json(
        payload, f"compiler profile {group}.{field}", MAX_PROFILE_BLOB_BYTES
    )
    return _require_hex(
        document.get("canonical_sha256"),
        64,
        f"compiler profile {group}.{field} canonical SHA-256",
    )


def _verify_profile_promotion(
    root: Path,
    profile: dict[str, object],
    manifest_bytes: bytes,
    blobs: list[tuple[str, Seal, str]],
    expected_sidecar: dict[str, object],
) -> dict[str, Seal]:
    artifact_bytes: dict[str, bytes] = {}
    for name in (
        EMBEDDED_QUALIFICATION_ARTIFACT_MANIFEST_FILE,
        STANDALONE_QUALIFICATION_ARTIFACT_MANIFEST_FILE,
    ):
        artifact_bytes[name] = _read_regular_no_follow(
            root / name, MAX_PROFILE_BLOB_BYTES, f"qualified profile audit {name}"
        )
    embedded_summary = _offline_artifact_authority_summary(
        artifact_bytes[EMBEDDED_QUALIFICATION_ARTIFACT_MANIFEST_FILE], "embedded_game"
    )
    standalone_summary = _offline_artifact_authority_summary(
        artifact_bytes[STANDALONE_QUALIFICATION_ARTIFACT_MANIFEST_FILE], "standalone"
    )

    receipt_bytes = _read_regular_no_follow(
        root / QUALIFIED_PROMOTION_RECEIPT_FILE,
        MAX_DESCRIPTOR_BYTES,
        "qualified profile promotion receipt",
    )
    receipt = _parse_json(
        receipt_bytes, "qualified profile promotion receipt", MAX_DESCRIPTOR_BYTES
    )
    receipt_fields = (
        "schema",
        "schema_version",
        "qualified",
        "source_profile_sha256",
        "source_target",
        "source_materialization_receipt_sha256",
        "capture_stream_sha256",
        "static_support_manifest_sha256",
        "standalone_compiler",
        "embedded_artifacts",
        "standalone_artifacts",
        "corpus_sha256",
        "expected_results_sha256",
        "diagnostic_parity_sha256",
        "semantic_parity_sha256",
        "profile_sha256",
        "files",
        "canonical_sha256",
    )
    _require_exact_fields(
        receipt, receipt_fields, "qualified profile promotion receipt"
    )
    if (
        receipt["schema"] != QUALIFIED_PROMOTION_RECEIPT_SCHEMA
        or receipt["schema_version"] != QUALIFIED_PROMOTION_RECEIPT_SCHEMA_VERSION
        or receipt["qualified"] is not True
        or receipt["standalone_compiler"] != expected_sidecar
        or receipt["embedded_artifacts"] != embedded_summary
        or receipt["standalone_artifacts"] != standalone_summary
        or receipt["profile_sha256"] != profile.get("profile_sha256")
    ):
        raise BundleError("qualified profile promotion authority differs")
    qualification = profile.get("qualification")
    if (
        embedded_summary["suite_id"] != standalone_summary["suite_id"]
        or embedded_summary["corpus_sha256"] != standalone_summary["corpus_sha256"]
        or embedded_summary["source_profile_sha256"] != receipt["source_profile_sha256"]
        or standalone_summary["source_profile_sha256"]
        != receipt["source_profile_sha256"]
        or embedded_summary["source_target"] != receipt["source_target"]
        or standalone_summary["source_target"] != receipt["source_target"]
        or receipt["source_target"] != profile.get("target")
        or embedded_summary["standalone_compiler"] is not None
        or standalone_summary["standalone_compiler"] != expected_sidecar
        or not isinstance(qualification, dict)
        or qualification.get("required_probe_suite_version")
        != embedded_summary["suite_id"]
    ):
        raise BundleError("qualified profile promotion suite/corpus authority differs")
    for field in (
        "source_profile_sha256",
        "source_materialization_receipt_sha256",
        "capture_stream_sha256",
        "static_support_manifest_sha256",
        "corpus_sha256",
        "expected_results_sha256",
        "diagnostic_parity_sha256",
        "semantic_parity_sha256",
        "profile_sha256",
        "canonical_sha256",
    ):
        value = _require_hex(receipt[field], 64, f"qualified profile promotion {field}")
        if value == "0" * 64:
            raise BundleError(f"qualified profile promotion {field} is zero")

    expected_payload_digests = {
        "corpus_sha256": _profile_payload_canonical_sha256(
            root, profile, "bytecode", "codegen_probe_corpus"
        ),
        "expected_results_sha256": _profile_payload_canonical_sha256(
            root, profile, "bytecode", "expected_probe_results"
        ),
        "diagnostic_parity_sha256": _profile_payload_canonical_sha256(
            root, profile, "qualification", "diagnostic_parity"
        ),
        "semantic_parity_sha256": _profile_payload_canonical_sha256(
            root, profile, "qualification", "semantic_parity"
        ),
    }
    if any(
        receipt[field] != digest for field, digest in expected_payload_digests.items()
    ):
        raise BundleError("qualified profile promotion payload authority differs")
    if (
        embedded_summary["corpus_sha256"] != receipt["corpus_sha256"]
        or standalone_summary["corpus_sha256"] != receipt["corpus_sha256"]
    ):
        raise BundleError("qualified profile promotion corpus authority differs")

    observed_files: list[dict[str, object]] = [
        {
            "path": "compiler-profile.json",
            "byte_len": len(manifest_bytes),
            "sha256": _sha256(manifest_bytes),
        }
    ]
    seen: set[str] = set()
    for relative, seal, _ in blobs:
        if relative.casefold() in seen:
            continue
        seen.add(relative.casefold())
        observed_files.append(
            {"path": relative, "byte_len": seal.byte_len, "sha256": seal.sha256}
        )
    for name, bytes_ in artifact_bytes.items():
        observed_files.append(
            {"path": name, "byte_len": len(bytes_), "sha256": _sha256(bytes_)}
        )
    observed_files.sort(key=lambda value: str(value["path"]))
    if receipt["files"] != observed_files:
        raise BundleError("qualified profile promotion file seals differ")

    canonical_payload = {field: receipt[field] for field in receipt_fields[:-1]}
    if receipt["canonical_sha256"] != _domain_json_sha256(
        b"gore-as-qualified-profile-promotion-v1\0",
        canonical_payload,
        include_length=False,
    ):
        raise BundleError("qualified profile promotion receipt canonical seal differs")
    return {
        EMBEDDED_QUALIFICATION_ARTIFACT_MANIFEST_FILE: Seal(
            len(artifact_bytes[EMBEDDED_QUALIFICATION_ARTIFACT_MANIFEST_FILE]),
            _sha256(artifact_bytes[EMBEDDED_QUALIFICATION_ARTIFACT_MANIFEST_FILE]),
        ),
        STANDALONE_QUALIFICATION_ARTIFACT_MANIFEST_FILE: Seal(
            len(artifact_bytes[STANDALONE_QUALIFICATION_ARTIFACT_MANIFEST_FILE]),
            _sha256(artifact_bytes[STANDALONE_QUALIFICATION_ARTIFACT_MANIFEST_FILE]),
        ),
        QUALIFIED_PROMOTION_RECEIPT_FILE: Seal(
            len(receipt_bytes), _sha256(receipt_bytes)
        ),
    }


def _verify_profile(
    bundle_root: Path,
    catalog_entry: dict[str, object],
    sidecar: dict[str, object],
    expected_files: dict[str, Seal],
    qualified_profile_verifier: QualifiedProfileVerifier,
) -> dict[str, object]:
    relative = str(catalog_entry["manifest_relative_path"])
    manifest_path = bundle_root.joinpath(*PurePosixPath(relative).parts)
    manifest = _read_regular_no_follow(
        manifest_path, MAX_PROFILE_MANIFEST_BYTES, "qualified compiler profile manifest"
    )
    manifest_seal = Seal(
        int(catalog_entry["manifest_byte_len"]),
        str(catalog_entry["manifest_sha256"]).casefold(),
    )
    _check_sealed_bytes(manifest, manifest_seal, "qualified compiler profile manifest")
    expected_files[relative] = manifest_seal
    profile = _parse_json(
        manifest, "qualified compiler profile manifest", MAX_PROFILE_MANIFEST_BYTES
    )
    if (
        profile.get("schema") != "gore.as.compiler-profile"
        or profile.get("schema_version") != 1
    ):
        raise BundleError("qualified compiler profile schema/version is unsupported")
    if profile.get("profile_sha256") != catalog_entry["profile_sha256"]:
        raise BundleError(
            "catalog profile SHA-256 differs from the qualified manifest identity"
        )
    target = catalog_entry["target"]
    assert isinstance(target, dict)
    if profile.get("target") != target.get("target"):
        raise BundleError(
            "catalog target tuple differs from the qualified compiler profile"
        )
    oracle = profile.get("oracle")
    if not isinstance(oracle, dict) or oracle.get("pe_codeview") != target.get(
        "pe_codeview"
    ):
        raise BundleError(
            "catalog CodeView identity differs from the qualified compiler profile"
        )
    profile_root = manifest_path.parent
    blobs = _profile_blob_seals(profile)
    unique: set[str] = set()
    for blob_relative, seal, label in blobs:
        file_relative = (
            PurePosixPath(relative).parent / PurePosixPath(blob_relative)
        ).as_posix()
        if file_relative.casefold() in unique:
            continue
        unique.add(file_relative.casefold())
        bytes_ = _read_regular_no_follow(
            profile_root.joinpath(*PurePosixPath(blob_relative).parts),
            MAX_PROFILE_BLOB_BYTES,
            label,
        )
        _check_sealed_bytes(bytes_, seal, label)
        expected_files[file_relative] = seal
    expected_sidecar = _sidecar_identity(sidecar)
    _verify_qualification_identity(profile_root, profile, blobs, expected_sidecar)
    promotion_audits = _verify_profile_promotion(
        profile_root, profile, manifest, blobs, expected_sidecar
    )
    for audit_name, seal in promotion_audits.items():
        audit_relative = (PurePosixPath(relative).parent / audit_name).as_posix()
        expected_files[audit_relative] = seal
    profile_tree_seals: dict[str, Seal] = {
        "compiler-profile.json": manifest_seal,
        **{blob_relative: seal for blob_relative, seal, _ in blobs},
        **promotion_audits,
    }
    expected_tree = QualifiedProfileTreeAuthority(
        profile_sha256=str(catalog_entry["profile_sha256"]),
        manifest_sha256=manifest_seal.sha256,
        promotion_receipt_sha256=promotion_audits[
            QUALIFIED_PROMOTION_RECEIPT_FILE
        ].sha256,
        tree_sha256=_qualified_profile_tree_seal_sha256(
            list(profile_tree_seals.items())
        ),
        file_count=len(profile_tree_seals),
    )
    verified_tree = qualified_profile_verifier(
        profile_root, str(catalog_entry["profile_sha256"])
    )
    if verified_tree != expected_tree:
        raise BundleError(
            "Rust typed profile-tree authority differs from the initially ingested bytes"
        )
    return profile


def _verify_full_tree_verifications(
    bundle_root: Path,
    value: object,
    profiles: list[dict[str, object]],
    profile_documents: list[dict[str, object]],
    sidecar: dict[str, object],
    expected_files: dict[str, Seal],
) -> None:
    if not isinstance(value, list) or len(value) != len(profiles):
        raise BundleError(
            "full-tree verifications must contain exactly one receipt per product profile"
        )
    paths: set[str] = set()
    for index, (entry, catalog_entry, profile) in enumerate(
        zip(value, profiles, profile_documents, strict=True)
    ):
        label = f"full-tree verification {index}"
        if not isinstance(entry, dict):
            raise BundleError(f"{label} must be an object")
        _require_exact_fields(
            entry,
            ("profile_sha256", "relative_path", "byte_len", "sha256"),
            label,
        )
        expected_profile_sha256 = str(catalog_entry["profile_sha256"]).casefold()
        if (
            _require_hex(entry["profile_sha256"], 64, f"{label} profile SHA-256")
            != expected_profile_sha256
        ):
            raise BundleError(f"{label} names a different catalog profile")
        relative = _safe_relative(entry["relative_path"], f"{label} path")
        expected_relative = _full_tree_receipt_relative(catalog_entry)
        if relative != expected_relative:
            raise BundleError(
                f"{label} path must be exactly {expected_relative!r}"
            )
        if relative.casefold() in paths:
            raise BundleError("full-tree verification receipt paths are not unique")
        paths.add(relative.casefold())
        seal = _seal(
            {"byte_len": entry["byte_len"], "sha256": entry["sha256"]},
            label,
            MAX_FULL_TREE_RECEIPT_BYTES,
        )
        receipt_bytes = _read_regular_no_follow(
            bundle_root.joinpath(*PurePosixPath(relative).parts),
            MAX_FULL_TREE_RECEIPT_BYTES,
            label,
        )
        _check_sealed_bytes(receipt_bytes, seal, label)
        _parse_full_tree_receipt(
            receipt_bytes, catalog_entry, profile, sidecar
        )
        expected_files[relative] = seal


def _enumerate_regular_files(root: Path, label: str) -> set[str]:
    _check_no_follow_chain(root, label)
    try:
        root_status = root.lstat()
    except OSError as error:
        raise BundleError(f"cannot inspect {label}: {error}") from error
    if not stat.S_ISDIR(root_status.st_mode) or _is_reparse(root_status):
        raise BundleError(f"{label} is not a real directory")
    files: set[str] = set()
    pending = [root]
    while pending:
        directory = pending.pop()
        with os.scandir(directory) as entries:
            for entry in entries:
                entry_path = Path(entry.path)
                status = entry_path.lstat()
                relative = entry_path.relative_to(root).as_posix()
                if entry.is_symlink() or _is_reparse(status):
                    raise BundleError(
                        f"{label} contains forbidden reparse point {relative!r}"
                    )
                if stat.S_ISDIR(status.st_mode):
                    pending.append(entry_path)
                elif stat.S_ISREG(status.st_mode):
                    if status.st_nlink != 1:
                        raise BundleError(
                            f"{label} contains hard-linked file {relative!r}"
                        )
                    key = relative.casefold()
                    if any(existing.casefold() == key for existing in files):
                        raise BundleError(
                            f"{label} contains a case-aliasing file {relative!r}"
                        )
                    files.add(relative)
                else:
                    raise BundleError(
                        f"{label} contains non-regular entry {relative!r}"
                    )
    return files


def _promotion_file_seal(
    value: object,
    expected_relative: str,
    label: str,
    maximum: int = MAX_DESCRIPTOR_BYTES,
) -> Seal:
    if not isinstance(value, dict):
        raise BundleError(f"{label} must be an object")
    _require_exact_fields(value, ("relative_path", "byte_len", "sha256"), label)
    if value["relative_path"] != expected_relative:
        raise BundleError(f"{label} path differs from {expected_relative!r}")
    return _seal(
        {"byte_len": value["byte_len"], "sha256": value["sha256"]},
        label,
        maximum,
    )


def _verify_promotion_authority(
    bundle_root: Path,
    value: object,
    signed_sidecar: Seal,
    expected_files: dict[str, Seal],
    promotion_attestation_verifier: PromotionAttestationVerifier,
) -> PromotionAuthority:
    if not isinstance(value, dict):
        raise BundleError("compiler promotion authority must be an object")
    _require_exact_fields(
        value,
        (
            "repository",
            "commit",
            "workflow_sha",
            "workflow_run_id",
            "workflow_run_attempt",
            "signed_identity_file",
            "source_provenance_file",
            "github_attestation_file",
        ),
        "compiler promotion authority",
    )
    repository = _require_repository(
        value["repository"], "compiler promotion authority repository"
    )
    commit = _require_hex(value["commit"], 40, "compiler promotion authority commit")
    workflow_sha = _require_hex(
        value["workflow_sha"], 40, "compiler promotion authority workflow SHA"
    )
    run_id = _require_uint(
        value["workflow_run_id"], "compiler promotion authority workflow run ID"
    )
    run_attempt = _require_uint(
        value["workflow_run_attempt"],
        "compiler promotion authority workflow run attempt",
        maximum=(1 << 32) - 1,
    )
    identity_seal = _promotion_file_seal(
        value["signed_identity_file"],
        PROMOTION_IDENTITY_FILE,
        "compiler promotion signed-identity file",
    )
    provenance_seal = _promotion_file_seal(
        value["source_provenance_file"],
        PROMOTION_PROVENANCE_FILE,
        "compiler promotion source-provenance file",
    )
    attestation_seal = _promotion_file_seal(
        value["github_attestation_file"],
        PROMOTION_ATTESTATION_FILE,
        "compiler promotion GitHub attestation file",
        MAX_ATTESTATION_BYTES,
    )
    identity_path = bundle_root.joinpath(*PurePosixPath(PROMOTION_IDENTITY_FILE).parts)
    provenance_path = bundle_root.joinpath(
        *PurePosixPath(PROMOTION_PROVENANCE_FILE).parts
    )
    attestation_path = bundle_root.joinpath(
        *PurePosixPath(PROMOTION_ATTESTATION_FILE).parts
    )
    identity_bytes = _read_regular_no_follow(
        identity_path,
        MAX_DESCRIPTOR_BYTES,
        "compiler promotion signed identity",
    )
    provenance_bytes = _read_regular_no_follow(
        provenance_path,
        MAX_DESCRIPTOR_BYTES,
        "compiler promotion source provenance",
    )
    _check_sealed_bytes(
        identity_bytes, identity_seal, "compiler promotion signed identity"
    )
    _check_sealed_bytes(
        provenance_bytes, provenance_seal, "compiler promotion source provenance"
    )
    attestation_bytes = _read_regular_no_follow(
        attestation_path,
        MAX_ATTESTATION_BYTES,
        "compiler promotion GitHub attestation",
    )
    _check_sealed_bytes(
        attestation_bytes,
        attestation_seal,
        "compiler promotion GitHub attestation",
    )
    _parse_signed_sidecar_identity(identity_bytes, signed_sidecar)
    authority = _parse_promotion_provenance(
        provenance_bytes,
        identity_seal,
        expected_repository=repository,
        expected_commit=commit,
    )
    if (
        authority.workflow_sha != workflow_sha
        or authority.workflow_run_id != run_id
        or authority.workflow_run_attempt != run_attempt
    ):
        raise BundleError(
            "compiler promotion authority differs from its source provenance"
        )
    promotion_attestation_verifier(
        attestation_path,
        {
            SIDECAR_FILE: bundle_root / SIDECAR_FILE,
            PurePosixPath(PROMOTION_IDENTITY_FILE).name: identity_path,
            PurePosixPath(PROMOTION_PROVENANCE_FILE).name: provenance_path,
        },
        authority,
    )
    expected_files[PROMOTION_IDENTITY_FILE] = identity_seal
    expected_files[PROMOTION_PROVENANCE_FILE] = provenance_seal
    expected_files[PROMOTION_ATTESTATION_FILE] = attestation_seal
    return authority


def _verify_descriptor(
    bundle_root: Path,
    descriptor_bytes: bytes,
    *,
    sidecar_verifier: SidecarVerifier,
    qualified_profile_verifier: QualifiedProfileVerifier,
    promotion_attestation_verifier: PromotionAttestationVerifier,
    staged: bool,
    allow_legacy_internal_input_v1: bool = False,
) -> VerifiedBundle:
    descriptor = _parse_json(
        descriptor_bytes, "compiler internal input", MAX_DESCRIPTOR_BYTES
    )
    schema_version = descriptor.get("schema_version")
    legacy_v1 = (
        descriptor.get("schema") == INTERNAL_INPUT_SCHEMA and schema_version == 1
    )
    if legacy_v1 and not allow_legacy_internal_input_v1:
        raise BundleError(
            "compiler internal-input schema/version is unsupported; "
            "legacy v1 is accepted only from the pinned local build asset"
        )
    _require_exact_fields(
        descriptor,
        (
            "schema",
            "schema_version",
            "immutable",
            "qualified",
            "promotion",
            "catalog",
            *(("full_tree_verifications",) if not legacy_v1 else ()),
            "notices",
        ),
        "compiler internal input",
    )
    if descriptor["schema"] != INTERNAL_INPUT_SCHEMA or not (
        schema_version == INTERNAL_INPUT_SCHEMA_VERSION
        or (allow_legacy_internal_input_v1 and legacy_v1)
    ):
        raise BundleError("compiler internal-input schema/version is unsupported")
    _require_bool(descriptor["immutable"], True, "compiler internal input immutable")
    _require_bool(descriptor["qualified"], True, "compiler internal input qualified")
    catalog = _catalog(descriptor["catalog"])
    if not legacy_v1:
        _require_product_profile_targets(catalog)
    catalog_bytes = _canonical_pretty(catalog)
    if len(catalog_bytes) > MAX_CATALOG_BYTES:
        raise BundleError("generated compiler catalog exceeds its byte limit")
    expected_files: dict[str, Seal] = {}
    sidecar = catalog["sidecar"]
    assert isinstance(sidecar, dict)
    sidecar_path = bundle_root / SIDECAR_FILE
    sidecar_bytes = _read_regular_no_follow(
        sidecar_path, MAX_SIDECAR_BYTES, "signed sidecar"
    )
    sidecar_seal = Seal(int(sidecar["byte_len"]), str(sidecar["sha256"]).casefold())
    _check_sealed_bytes(sidecar_bytes, sidecar_seal, "signed sidecar")
    sidecar_verifier(sidecar_path, sidecar_bytes)
    expected_files[SIDECAR_FILE] = sidecar_seal
    _verify_promotion_authority(
        bundle_root,
        descriptor["promotion"],
        sidecar_seal,
        expected_files,
        promotion_attestation_verifier,
    )
    profiles = catalog["profiles"]
    assert isinstance(profiles, list)
    profile_documents: list[dict[str, object]] = []
    for profile in profiles:
        assert isinstance(profile, dict)
        profile_documents.append(
            _verify_profile(
                bundle_root,
                profile,
                sidecar,
                expected_files,
                qualified_profile_verifier,
            )
        )
    if not legacy_v1:
        _verify_full_tree_verifications(
            bundle_root,
            descriptor["full_tree_verifications"],
            profiles,
            profile_documents,
            sidecar,
            expected_files,
        )
    notices = descriptor["notices"]
    if not isinstance(notices, dict) or set(notices) != set(REQUIRED_NOTICES):
        raise BundleError(f"compiler notices must be exactly {list(REQUIRED_NOTICES)}")
    for name in REQUIRED_NOTICES:
        seal = _seal(notices[name], f"compiler notice {name}", MAX_DESCRIPTOR_BYTES)
        bytes_ = _read_regular_no_follow(
            bundle_root / name, MAX_DESCRIPTOR_BYTES, f"compiler notice {name}"
        )
        _check_sealed_bytes(bytes_, seal, f"compiler notice {name}")
        expected_files[name] = seal
    control = (
        {BUNDLE_DESCRIPTOR, CATALOG_FILE}
        if staged
        else {INTERNAL_INPUT_DESCRIPTOR_FILE}
    )
    actual_files = _enumerate_regular_files(bundle_root, "compiler bundle")
    expected_names = set(expected_files) | control
    if actual_files != expected_names:
        raise BundleError(
            f"compiler bundle file set differs: missing={sorted(expected_names - actual_files)}, "
            f"unknown={sorted(actual_files - expected_names)}"
        )
    if staged:
        catalog_file = _read_regular_no_follow(
            bundle_root / CATALOG_FILE, MAX_CATALOG_BYTES, "compiler catalog"
        )
        if catalog_file != catalog_bytes:
            raise BundleError(
                "staged compiler catalog differs from the internal-input catalog"
            )
        staged_descriptor = _read_regular_no_follow(
            bundle_root / BUNDLE_DESCRIPTOR,
            MAX_DESCRIPTOR_BYTES,
            "compiler bundle manifest",
        )
        if staged_descriptor != descriptor_bytes:
            raise BundleError(
                "staged compiler bundle manifest changed during verification"
            )
    return VerifiedBundle(
        descriptor_bytes,
        catalog_bytes,
        expected_files,
        SIDECAR_FILE,
        int(schema_version),
    )


def verify_internal_input(
    root: Path,
    *,
    sidecar_verifier: SidecarVerifier = verify_sidecar,
    qualified_profile_verifier: QualifiedProfileVerifier | None = None,
    promotion_attestation_verifier: PromotionAttestationVerifier | None = None,
    allow_legacy_internal_input_v1: bool = False,
) -> VerifiedBundle:
    if qualified_profile_verifier is None:
        raise BundleError("the Rust typed qualified-profile verifier is required")
    if promotion_attestation_verifier is None:
        raise BundleError("the GitHub promotion attestation verifier is required")
    root = _require_absolute_normalized(root, "compiler internal-input root")
    descriptor = _read_regular_no_follow(
        root / INTERNAL_INPUT_DESCRIPTOR_FILE,
        MAX_DESCRIPTOR_BYTES,
        "compiler internal-input descriptor",
    )
    return _verify_descriptor(
        root,
        descriptor,
        sidecar_verifier=sidecar_verifier,
        qualified_profile_verifier=qualified_profile_verifier,
        promotion_attestation_verifier=promotion_attestation_verifier,
        staged=False,
        allow_legacy_internal_input_v1=allow_legacy_internal_input_v1,
    )


def _require_safe_ascii_token(value: object, label: str, *, suffix: str = "") -> str:
    if (
        not isinstance(value, str)
        or not 1 <= len(value) <= 160
        or not value[0].isalnum()
        or not all(
            character.isascii() and (character.isalnum() or character in "-._")
            for character in value
        )
        or (suffix and not value.endswith(suffix))
    ):
        raise BundleError(f"{label} is not a safe ASCII token")
    return value


def _require_repository(value: object, label: str) -> str:
    if not isinstance(value, str) or value.count("/") != 1:
        raise BundleError(f"{label} must be one owner/repository pair")
    owner, repository = value.split("/", 1)
    _require_safe_ascii_token(owner, f"{label} owner")
    _require_safe_ascii_token(repository, f"{label} repository")
    return value


def _parse_internal_package_descriptor(bytes_: bytes) -> InternalPackageDescriptor:
    document = _parse_json(
        bytes_, "internal standalone compiler package descriptor", MAX_DESCRIPTOR_BYTES
    )
    _require_exact_fields(
        document,
        (
            "schema",
            "schema_version",
            "asset",
            "archive",
            "compression",
            "internal_input",
        ),
        "internal standalone compiler package descriptor",
    )
    if (
        document["schema"] != INTERNAL_PACKAGE_DESCRIPTOR_SCHEMA
        or document["schema_version"] != INTERNAL_PACKAGE_DESCRIPTOR_SCHEMA_VERSION
    ):
        raise BundleError(
            "internal standalone compiler package schema/version is unsupported"
        )
    if document["compression"] != "deflate-9":
        raise BundleError(
            "internal standalone compiler package compression must be deflate-9"
        )
    internal_input = document["internal_input"]
    if not isinstance(internal_input, dict):
        raise BundleError("internal compiler input summary must be an object")
    _require_exact_fields(
        internal_input,
        ("catalog_sha256", "file_count"),
        "internal compiler input summary",
    )
    return InternalPackageDescriptor(
        asset=_require_safe_ascii_token(
            document["asset"], "internal standalone compiler package asset", suffix=".zip"
        ),
        archive=_seal(
            document["archive"],
            "internal standalone compiler package archive",
            MAX_INTERNAL_PACKAGE_BYTES,
        ),
        compression="deflate-9",
        catalog_sha256=_require_hex(
            internal_input["catalog_sha256"],
            64,
            "internal compiler input catalog SHA-256",
        ),
        file_count=_require_uint(
            internal_input["file_count"],
            "internal compiler input file count",
            maximum=MAX_INTERNAL_INPUT_FILES,
        ),
    )


def read_internal_package_descriptor(path: Path) -> InternalPackageDescriptor:
    path = _require_absolute_normalized(
        path, "internal standalone compiler package descriptor"
    )
    bytes_ = _read_regular_no_follow(
        path, MAX_DESCRIPTOR_BYTES, "internal standalone compiler package descriptor"
    )
    return _parse_internal_package_descriptor(bytes_)


def _streaming_file_seal(path: Path, maximum: int, label: str) -> Seal:
    _check_no_follow_chain(path, label)
    flags = os.O_RDONLY | getattr(os, "O_BINARY", 0) | getattr(os, "O_NOFOLLOW", 0)
    try:
        descriptor = os.open(path, flags)
    except OSError as error:
        raise BundleError(
            f"cannot open {label} without following links: {path}: {error}"
        ) from error
    try:
        before = os.fstat(descriptor)
        if (
            not stat.S_ISREG(before.st_mode)
            or before.st_nlink != 1
            or _is_reparse(before)
        ):
            raise BundleError(
                f"{label} must be a non-reparse, single-link regular file: {path}"
            )
        if not 0 < before.st_size <= maximum:
            raise BundleError(f"{label} size is outside 1..{maximum} bytes: {path}")
        digest = hashlib.sha256()
        total = 0
        while True:
            chunk = os.read(descriptor, 1024 * 1024)
            if not chunk:
                break
            total += len(chunk)
            if total > maximum:
                raise BundleError(f"{label} exceeds {maximum} bytes: {path}")
            digest.update(chunk)
        after = os.fstat(descriptor)
        if (
            total != before.st_size
            or after.st_size != before.st_size
            or after.st_nlink != 1
            or (before.st_ino and after.st_ino != before.st_ino)
            or (before.st_dev and after.st_dev != before.st_dev)
        ):
            raise BundleError(f"{label} changed while held open: {path}")
        return Seal(total, digest.hexdigest())
    finally:
        os.close(descriptor)


def _require_real_output_parent(path: Path, label: str) -> None:
    _require_absolute_normalized(path, label)
    parent = path.parent
    _check_no_follow_chain(parent, f"{label} parent")
    try:
        status = parent.lstat()
    except OSError as error:
        raise BundleError(f"cannot inspect {label} parent {parent}: {error}") from error
    if not stat.S_ISDIR(status.st_mode) or _is_reparse(status):
        raise BundleError(f"{label} parent must be a real directory: {parent}")


def _canonical_archive_info(
    relative: str, *, compression: int = zipfile.ZIP_STORED
) -> zipfile.ZipInfo:
    info = zipfile.ZipInfo(relative, date_time=(1980, 1, 1, 0, 0, 0))
    info.compress_type = compression
    info.create_system = 3
    info.external_attr = (stat.S_IFREG | 0o644) << 16
    return info


def _write_zip_member_from_file(
    archive: zipfile.ZipFile,
    relative: str,
    source: Path,
    expected: Seal,
    *,
    compression: int = zipfile.ZIP_STORED,
    label_prefix: str = "compiler package archive",
) -> None:
    label = f"{label_prefix} entry {relative}"
    _check_no_follow_chain(source, label)
    flags = os.O_RDONLY | getattr(os, "O_BINARY", 0) | getattr(os, "O_NOFOLLOW", 0)
    try:
        descriptor = os.open(source, flags)
    except OSError as error:
        raise BundleError(
            f"cannot open {label} without following links: {error}"
        ) from error
    try:
        before = os.fstat(descriptor)
        if (
            not stat.S_ISREG(before.st_mode)
            or before.st_nlink != 1
            or _is_reparse(before)
            or before.st_size != expected.byte_len
        ):
            raise BundleError(f"{label} is not the expected single-link regular file")
        info = _canonical_archive_info(relative, compression=compression)
        info.file_size = expected.byte_len
        digest = hashlib.sha256()
        total = 0
        with archive.open(info, mode="w", force_zip64=False) as destination:
            while True:
                chunk = os.read(descriptor, 1024 * 1024)
                if not chunk:
                    break
                total += len(chunk)
                if total > expected.byte_len:
                    raise BundleError(f"{label} grew while archiving")
                digest.update(chunk)
                destination.write(chunk)
        after = os.fstat(descriptor)
        if (
            total != expected.byte_len
            or digest.hexdigest() != expected.sha256
            or after.st_size != before.st_size
            or after.st_nlink != 1
            or (before.st_ino and after.st_ino != before.st_ino)
            or (before.st_dev and after.st_dev != before.st_dev)
        ):
            raise BundleError(f"{label} changed while held open")
    finally:
        os.close(descriptor)


def _write_new_from_archive(
    path: Path,
    source: object,
    expected_length: int,
) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    _check_no_follow_chain(path.parent, f"archive output parent for {path.name}")
    flags = (
        os.O_WRONLY
        | os.O_CREAT
        | os.O_EXCL
        | getattr(os, "O_BINARY", 0)
        | getattr(os, "O_NOFOLLOW", 0)
    )
    try:
        descriptor = os.open(path, flags, 0o644)
    except OSError as error:
        raise BundleError(f"cannot create extracted file {path}: {error}") from error
    try:
        total = 0
        while True:
            chunk = source.read(1024 * 1024)  # type: ignore[attr-defined]
            if not chunk:
                break
            total += len(chunk)
            if total > expected_length:
                raise BundleError(f"archive entry {path.name!r} grew while extracting")
            view = memoryview(chunk)
            while view:
                written = os.write(descriptor, view)
                if written <= 0:
                    raise BundleError(f"short write while extracting {path}")
                view = view[written:]
        os.fsync(descriptor)
        status = os.fstat(descriptor)
        if (
            total != expected_length
            or status.st_size != expected_length
            or not stat.S_ISREG(status.st_mode)
            or status.st_nlink != 1
            or _is_reparse(status)
        ):
            raise BundleError(f"extracted file {path} has an unexpected final identity")
    finally:
        os.close(descriptor)


def _write_temporary_bytes(parent: Path, prefix: str, bytes_: bytes) -> Path:
    descriptor, name = tempfile.mkstemp(prefix=prefix, suffix=".tmp", dir=parent)
    try:
        view = memoryview(bytes_)
        while view:
            written = os.write(descriptor, view)
            if written <= 0:
                raise BundleError(f"short write while creating temporary file {name}")
            view = view[written:]
        os.fsync(descriptor)
    finally:
        os.close(descriptor)
    return Path(name)


def _same_file_identity(left: os.stat_result, right: os.stat_result) -> bool:
    return bool(
        left.st_ino
        and right.st_ino
        and left.st_dev == right.st_dev
        and left.st_ino == right.st_ino
    )


def _unlink_published_file_if_owned(path: Path, identity: os.stat_result) -> None:
    try:
        current = path.lstat()
        if (
            stat.S_ISREG(current.st_mode)
            and not _is_reparse(current)
            and _same_file_identity(current, identity)
        ):
            path.unlink()
    except OSError:
        pass


def _publish_directory_no_replace(source: Path, destination: Path) -> None:
    """Atomically make one verified directory visible without replacing a peer."""

    if os.name == "nt":
        move = ctypes.WinDLL("kernel32", use_last_error=True).MoveFileExW
        move.argtypes = [ctypes.c_wchar_p, ctypes.c_wchar_p, ctypes.c_uint32]
        move.restype = ctypes.c_int
        if not move(str(source), str(destination), 0):
            error = ctypes.get_last_error()
            raise BundleError(
                f"cannot exclusively publish extracted internal compiler input: WinError {error}"
            )
        return
    if sys.platform.startswith("linux"):
        libc = ctypes.CDLL(None, use_errno=True)
        renameat2 = getattr(libc, "renameat2", None)
        if renameat2 is None:
            raise BundleError("atomic no-replace directory publication is unavailable")
        renameat2.argtypes = [
            ctypes.c_int,
            ctypes.c_char_p,
            ctypes.c_int,
            ctypes.c_char_p,
            ctypes.c_uint,
        ]
        renameat2.restype = ctypes.c_int
        if renameat2(-100, os.fsencode(source), -100, os.fsencode(destination), 1) != 0:
            error = ctypes.get_errno()
            raise BundleError(
                f"cannot exclusively publish extracted internal compiler input: errno {error}"
            )
        return
    raise BundleError(
        "atomic no-replace directory publication is unsupported on this platform"
    )


def _remove_directory_if_owned(path: Path, identity: os.stat_result) -> None:
    try:
        current = path.lstat()
        if (
            stat.S_ISDIR(current.st_mode)
            and not _is_reparse(current)
            and _same_file_identity(current, identity)
        ):
            _remove_work_root(path)
    except OSError:
        pass


def _read_descriptor_exact(
    descriptor: int, offset: int, length: int, label: str
) -> bytes:
    try:
        os.lseek(descriptor, offset, os.SEEK_SET)
    except OSError as error:
        raise BundleError(f"cannot seek {label}: {error}") from error
    chunks: list[bytes] = []
    remaining = length
    while remaining:
        try:
            chunk = os.read(descriptor, remaining)
        except OSError as error:
            raise BundleError(f"cannot read {label}: {error}") from error
        if not chunk:
            raise BundleError(f"{label} is truncated")
        chunks.append(chunk)
        remaining -= len(chunk)
    return b"".join(chunks)


def _validate_canonical_archive(
    path: Path,
    *,
    expected_compression: int,
    maximum_archive_bytes: int,
    label: str,
) -> tuple[str, ...]:
    """Validate the exact raw ZIP shape accepted by one compiler transport.

    Internal inputs are bounded below all classic ZIP limits, so ZIP64 is neither needed nor
    accepted. Parsing both header sets closes ambiguities that a central-directory-only reader
    could otherwise hide. CRC is still checked by ``zipfile`` while reading every member; SHA-256
    and length remain the external authority for the complete archive.
    """

    path = _require_absolute_normalized(path, label)
    _check_no_follow_chain(path, label)
    flags = os.O_RDONLY | getattr(os, "O_BINARY", 0) | getattr(os, "O_NOFOLLOW", 0)
    try:
        descriptor = os.open(path, flags)
    except OSError as error:
        raise BundleError(
            f"cannot open {label}: {error}"
        ) from error
    try:
        status = os.fstat(descriptor)
        size = status.st_size
        if (
            not stat.S_ISREG(status.st_mode)
            or status.st_nlink != 1
            or _is_reparse(status)
            or not 22 < size <= maximum_archive_bytes
        ):
            raise BundleError(f"{label} is not a bounded regular file")

        eocd_offset = size - 22
        eocd = struct.unpack(
            "<4s4H2IH",
            _read_descriptor_exact(
                descriptor, eocd_offset, 22, f"{label} EOCD"
            ),
        )
        (
            signature,
            disk_number,
            central_disk,
            disk_entries,
            total_entries,
            central_size,
            central_offset,
            comment_length,
        ) = eocd
        if (
            signature != b"PK\x05\x06"
            or disk_number != 0
            or central_disk != 0
            or disk_entries != total_entries
            or not 0 < total_entries <= MAX_INTERNAL_INPUT_FILES
            or comment_length != 0
            or central_offset + central_size != eocd_offset
        ):
            raise BundleError(f"{label} has a noncanonical EOCD or trailing bytes")

        central_cursor = central_offset
        expected_local_offset = 0
        previous_name: bytes | None = None
        names: list[str] = []
        seen: set[str] = set()
        expected_external_attr = (stat.S_IFREG | 0o644) << 16
        for _ in range(total_entries):
            central = struct.unpack(
                "<4s6H3I5H2I",
                _read_descriptor_exact(
                    descriptor,
                    central_cursor,
                    46,
                    f"{label} central header",
                ),
            )
            (
                central_signature,
                version_made_by,
                version_needed,
                entry_flags,
                compression,
                modified_time,
                modified_date,
                crc32,
                compressed_size,
                uncompressed_size,
                filename_length,
                extra_length,
                entry_comment_length,
                entry_disk,
                internal_attr,
                external_attr,
                local_offset,
            ) = central
            if (
                central_signature != b"PK\x01\x02"
                or version_made_by != 0x0314
                or version_needed != 20
                or entry_flags != 0
                or compression != expected_compression
                or modified_time != 0
                or modified_date != 33
                or not 0 < compressed_size <= maximum_archive_bytes
                or (
                    expected_compression == zipfile.ZIP_STORED
                    and compressed_size != uncompressed_size
                )
                or not 0 < uncompressed_size <= MAX_PROFILE_BLOB_BYTES
                or not 0 < filename_length <= 512
                or extra_length != 0
                or entry_comment_length != 0
                or entry_disk != 0
                or internal_attr != 0
                or external_attr != expected_external_attr
                or local_offset != expected_local_offset
            ):
                raise BundleError(f"{label} has a noncanonical central header")
            name_offset = central_cursor + 46
            raw_name = _read_descriptor_exact(
                descriptor,
                name_offset,
                filename_length,
                f"{label} central filename",
            )
            try:
                relative = raw_name.decode("ascii")
            except UnicodeDecodeError as error:
                raise BundleError(f"{label} filename is not ASCII") from error
            relative = _safe_relative(
                relative, f"{label} central filename"
            )
            key = relative.casefold()
            if key in seen or (previous_name is not None and raw_name <= previous_name):
                raise BundleError(f"{label} names are duplicated or unsorted")
            seen.add(key)
            previous_name = raw_name

            local = struct.unpack(
                "<4s5H3I2H",
                _read_descriptor_exact(
                    descriptor,
                    local_offset,
                    30,
                    f"{label} local header",
                ),
            )
            (
                local_signature,
                local_version_needed,
                local_flags,
                local_compression,
                local_time,
                local_date,
                local_crc32,
                local_compressed_size,
                local_uncompressed_size,
                local_filename_length,
                local_extra_length,
            ) = local
            local_name = _read_descriptor_exact(
                descriptor,
                local_offset + 30,
                local_filename_length,
                f"{label} local filename",
            )
            if (
                local_signature != b"PK\x03\x04"
                or local_version_needed != version_needed
                or local_flags != entry_flags
                or local_compression != compression
                or local_time != modified_time
                or local_date != modified_date
                or local_crc32 != crc32
                or local_compressed_size != compressed_size
                or local_uncompressed_size != uncompressed_size
                or local_filename_length != filename_length
                or local_extra_length != 0
                or local_name != raw_name
            ):
                raise BundleError(f"{label} local and central headers differ")
            expected_local_offset = (
                local_offset + 30 + filename_length + compressed_size
            )
            if expected_local_offset > central_offset:
                raise BundleError(f"{label} members overlap its central directory")
            central_cursor = name_offset + filename_length
            names.append(relative)

        if central_cursor != eocd_offset or expected_local_offset != central_offset:
            raise BundleError(f"{label} contains gaps or unparsed records")
        return tuple(names)
    finally:
        os.close(descriptor)


def _validate_canonical_internal_package_archive(path: Path) -> tuple[str, ...]:
    return _validate_canonical_archive(
        path,
        expected_compression=zipfile.ZIP_DEFLATED,
        maximum_archive_bytes=MAX_INTERNAL_PACKAGE_BYTES,
        label="internal standalone compiler package archive",
    )


def pack_internal_package_archive(
    internal_input_root: Path,
    archive_path: Path,
    descriptor_output: Path,
    *,
    sidecar_verifier: SidecarVerifier = verify_sidecar,
    qualified_profile_verifier: QualifiedProfileVerifier | None = None,
    promotion_attestation_verifier: PromotionAttestationVerifier | None = None,
) -> InternalPackageDescriptor:
    """Create the deterministic compressed package committed inside GORE.

    The expensive promotion attestation is checked here, before the package can
    become a source-tree asset. Normal product builds subsequently trust the
    committed archive pin and still re-check Authenticode, the typed profile,
    every payload seal, and the complete catalog.
    """

    internal_input_root = _require_absolute_normalized(
        internal_input_root, "compiler internal-input root"
    )
    archive_path = _require_absolute_normalized(
        archive_path, "internal standalone compiler package archive"
    )
    descriptor_output = _require_absolute_normalized(
        descriptor_output, "internal standalone compiler package descriptor output"
    )
    if archive_path == descriptor_output:
        raise BundleError("internal package archive and descriptor must differ")
    if archive_path.name != INTERNAL_PACKAGE_ARCHIVE_FILE:
        raise BundleError(
            f"internal compiler package archive must be named {INTERNAL_PACKAGE_ARCHIVE_FILE}"
        )
    if descriptor_output.name != INTERNAL_PACKAGE_DESCRIPTOR_FILE:
        raise BundleError(
            "internal compiler package descriptor must be named "
            f"{INTERNAL_PACKAGE_DESCRIPTOR_FILE}"
        )
    _require_real_output_parent(
        archive_path, "internal standalone compiler package archive"
    )
    _require_real_output_parent(
        descriptor_output, "internal standalone compiler package descriptor output"
    )
    if archive_path.exists() or archive_path.is_symlink():
        raise BundleError("internal standalone compiler package archive must not exist")
    if descriptor_output.exists() or descriptor_output.is_symlink():
        raise BundleError("internal standalone compiler package descriptor must not exist")
    verified = verify_internal_input(
        internal_input_root,
        sidecar_verifier=sidecar_verifier,
        qualified_profile_verifier=qualified_profile_verifier,
        promotion_attestation_verifier=promotion_attestation_verifier,
    )
    planned = {
        INTERNAL_INPUT_DESCRIPTOR_FILE: Seal(
            len(verified.descriptor_bytes), _sha256(verified.descriptor_bytes)
        ),
        **verified.expected_files,
    }
    if not 0 < len(planned) <= MAX_INTERNAL_INPUT_FILES:
        raise BundleError("internal compiler package file count is outside its limit")

    temporary_handle, temporary_name = tempfile.mkstemp(
        prefix=f".{archive_path.name}.", suffix=".tmp", dir=archive_path.parent
    )
    os.close(temporary_handle)
    temporary_archive = Path(temporary_name)
    temporary_archive_identity: os.stat_result | None = temporary_archive.lstat()
    temporary_descriptor_path: Path | None = None
    temporary_descriptor_identity: os.stat_result | None = None
    published_archive_identity: os.stat_result | None = None
    published_descriptor_identity: os.stat_result | None = None
    complete = False
    try:
        with zipfile.ZipFile(
            temporary_archive,
            mode="w",
            compression=zipfile.ZIP_DEFLATED,
            compresslevel=9,
            allowZip64=False,
        ) as archive:
            for relative, seal in sorted(planned.items()):
                _safe_relative(relative, f"internal compiler package entry {relative!r}")
                _write_zip_member_from_file(
                    archive,
                    relative,
                    internal_input_root.joinpath(*PurePosixPath(relative).parts),
                    seal,
                    compression=zipfile.ZIP_DEFLATED,
                    label_prefix="internal standalone compiler package archive",
                )
        raw_names = _validate_canonical_internal_package_archive(temporary_archive)
        if raw_names != tuple(sorted(planned)):
            raise BundleError("internal compiler package file set differs after writing")
        archive_seal = _streaming_file_seal(
            temporary_archive,
            MAX_INTERNAL_PACKAGE_BYTES,
            "temporary internal standalone compiler package archive",
        )
        document = {
            "schema": INTERNAL_PACKAGE_DESCRIPTOR_SCHEMA,
            "schema_version": INTERNAL_PACKAGE_DESCRIPTOR_SCHEMA_VERSION,
            "asset": INTERNAL_PACKAGE_ARCHIVE_FILE,
            "archive": {
                "byte_len": archive_seal.byte_len,
                "sha256": archive_seal.sha256,
            },
            "compression": "deflate-9",
            "internal_input": {
                "catalog_sha256": _sha256(verified.catalog_bytes),
                "file_count": len(planned),
            },
        }
        descriptor_bytes = _canonical_pretty(document)
        parsed_descriptor = _parse_internal_package_descriptor(descriptor_bytes)
        temporary_descriptor_path = _write_temporary_bytes(
            descriptor_output.parent,
            f".{descriptor_output.name}.",
            descriptor_bytes,
        )
        if not _same_file_identity(
            temporary_archive_identity, temporary_archive.lstat()
        ):
            raise BundleError("temporary internal package archive path changed")
        temporary_descriptor_identity = temporary_descriptor_path.lstat()
        try:
            os.link(temporary_archive, archive_path)
        except FileExistsError as error:
            raise BundleError(
                "internal standalone compiler package archive must not exist"
            ) from error
        published_archive_identity = archive_path.lstat()
        if not _same_file_identity(
            temporary_archive_identity, published_archive_identity
        ):
            raise BundleError("published internal package archive identity differs")
        try:
            os.link(temporary_descriptor_path, descriptor_output)
        except FileExistsError as error:
            raise BundleError(
                "internal standalone compiler package descriptor must not exist"
            ) from error
        published_descriptor_identity = descriptor_output.lstat()
        if not _same_file_identity(
            temporary_descriptor_identity, published_descriptor_identity
        ):
            raise BundleError("published internal package descriptor identity differs")
        temporary_archive.unlink()
        temporary_descriptor_path.unlink()
        if (
            _streaming_file_seal(
                archive_path,
                MAX_INTERNAL_PACKAGE_BYTES,
                "internal standalone compiler package archive",
            )
            != archive_seal
        ):
            raise BundleError("internal compiler package archive changed while publishing")
        if _validate_canonical_internal_package_archive(archive_path) != raw_names:
            raise BundleError("published internal package archive structure changed")
        if _read_regular_no_follow(
            descriptor_output,
            MAX_DESCRIPTOR_BYTES,
            "internal standalone compiler package descriptor",
        ) != descriptor_bytes:
            raise BundleError("internal compiler package descriptor changed while publishing")
        complete = True
        return parsed_descriptor
    except (OSError, zipfile.BadZipFile) as error:
        raise BundleError(f"cannot create internal compiler package: {error}") from error
    finally:
        if not complete:
            if published_descriptor_identity is not None:
                _unlink_published_file_if_owned(
                    descriptor_output, published_descriptor_identity
                )
            if published_archive_identity is not None:
                _unlink_published_file_if_owned(archive_path, published_archive_identity)
        if temporary_descriptor_path is not None and temporary_descriptor_identity is not None:
            _unlink_published_file_if_owned(
                temporary_descriptor_path, temporary_descriptor_identity
            )
        if temporary_archive_identity is not None:
            _unlink_published_file_if_owned(temporary_archive, temporary_archive_identity)


def _validate_archive_entry(
    info: zipfile.ZipInfo,
    seen: set[str],
    *,
    expected_compression: int,
    maximum_compressed_bytes: int,
    label: str,
) -> str:
    relative = _safe_relative(
        info.filename, f"{label} entry"
    )
    key = relative.casefold()
    if key in seen:
        raise BundleError(
            f"{label} contains duplicate/case alias {relative!r}"
        )
    seen.add(key)
    mode = info.external_attr >> 16
    if (
        info.is_dir()
        or info.filename.endswith("/")
        or info.compress_type != expected_compression
        or info.flag_bits != 0
        or info.create_system != 3
        or mode != (stat.S_IFREG | 0o644)
        or info.date_time != (1980, 1, 1, 0, 0, 0)
        or info.extra
        or info.comment
        or not 0 < info.file_size <= MAX_PROFILE_BLOB_BYTES
        or not 0 < info.compress_size <= maximum_compressed_bytes
        or (
            expected_compression == zipfile.ZIP_STORED
            and info.compress_size != info.file_size
        )
    ):
        raise BundleError(
            f"{label} entry {relative!r} is unsafe or noncanonical"
        )
    return relative


def _extract_pinned_archive(
    archive_path: Path,
    expected_archive: Seal,
    expected_file_count: int,
    temporary_root: Path,
    *,
    expected_compression: int,
    maximum_archive_bytes: int,
    raw_validator: Callable[[Path], tuple[str, ...]],
    archive_label: str,
    extraction_label: str,
) -> tuple[set[str], set[Path]]:
    """Measure, parse, and extract one unchanged Windows archive file."""

    with _pin_windows_file_path(
        archive_path, archive_label
    ):
        observed_archive = _streaming_file_seal(
            archive_path,
            maximum_archive_bytes,
            archive_label,
        )
        if observed_archive != expected_archive:
            raise BundleError(
                f"{archive_label} differs from its pinned length/SHA-256"
            )
        raw_names = raw_validator(archive_path)
        if len(raw_names) != expected_file_count:
            raise BundleError(f"{archive_label} raw file count differs")
        with zipfile.ZipFile(archive_path, mode="r") as archive:
            if archive.comment:
                raise BundleError(f"{archive_label} comment is forbidden")
            infos = archive.infolist()
            if (
                len(infos) != expected_file_count
                or not 0 < len(infos) <= MAX_INTERNAL_INPUT_FILES
            ):
                raise BundleError(f"{archive_label} file count differs")
            seen: set[str] = set()
            entries: list[tuple[str, zipfile.ZipInfo]] = []
            total = 0
            for info in infos:
                relative = _validate_archive_entry(
                    info,
                    seen,
                    expected_compression=expected_compression,
                    maximum_compressed_bytes=maximum_archive_bytes,
                    label=archive_label,
                )
                total += info.file_size
                if total > MAX_ARCHIVE_UNCOMPRESSED_BYTES:
                    raise BundleError(f"{archive_label} expands beyond its limit")
                entries.append((relative, info))
            if tuple(relative for relative, _ in entries) != raw_names:
                raise BundleError(f"{archive_label} parser views disagree")
            entry_names = {relative for relative, _ in entries}
            if INTERNAL_INPUT_DESCRIPTOR_FILE not in entry_names:
                raise BundleError(
                    f"{archive_label} has no {INTERNAL_INPUT_DESCRIPTOR_FILE}"
                )
            extraction_directories = {temporary_root}
            for relative, _ in entries:
                relative_path = PurePosixPath(relative)
                for depth in range(1, len(relative_path.parts)):
                    extraction_directories.add(
                        temporary_root.joinpath(*relative_path.parts[:depth])
                    )
            for directory in sorted(
                extraction_directories - {temporary_root},
                key=lambda path: (len(path.parts), str(path).casefold()),
            ):
                directory.mkdir()
            with _pin_windows_directories(
                extraction_directories, extraction_label
            ):
                for relative, info in entries:
                    try:
                        with archive.open(info, mode="r") as source:
                            _write_new_from_archive(
                                temporary_root.joinpath(*PurePosixPath(relative).parts),
                                source,
                                info.file_size,
                            )
                    except (OSError, RuntimeError, zipfile.BadZipFile) as error:
                        raise BundleError(
                            f"cannot read {archive_label} entry {relative!r}: {error}"
                        ) from error
        return entry_names, extraction_directories


def trust_pinned_internal_package_attestation(
    _bundle_path: Path,
    _subjects: dict[str, Path],
    _authority: PromotionAuthority,
) -> None:
    """Accept promotion evidence only behind the committed internal archive pin.

    `pack_internal_package_archive` requires and runs the real GitHub/Sigstore
    verifier. A normal product build starts from the checked-in descriptor and
    exact archive SHA-256, so repeating that external-tool verification would
    add a GitHub CLI dependency without adding authority. All embedded seals,
    Authenticode, typed profile data, and qualification receipts are still
    verified on every materialization and staging pass.
    """


def _verify_internal_archive_pin(
    archive_path: Path, descriptor: InternalPackageDescriptor
) -> tuple[str, ...]:
    if archive_path.name != descriptor.asset:
        raise BundleError("internal compiler package archive name differs")
    with _pin_windows_file_path(
        archive_path, "internal standalone compiler package archive"
    ):
        observed = _streaming_file_seal(
            archive_path,
            MAX_INTERNAL_PACKAGE_BYTES,
            "internal standalone compiler package archive",
        )
        if observed != descriptor.archive:
            raise BundleError(
                "internal standalone compiler package differs from its pinned length/SHA-256"
            )
        names = _validate_canonical_internal_package_archive(archive_path)
        if len(names) != descriptor.file_count:
            raise BundleError("internal compiler package raw file count differs")
        return names


def _verify_internal_input(
    root: Path,
    descriptor: InternalPackageDescriptor,
    *,
    sidecar_verifier: SidecarVerifier,
    qualified_profile_verifier: QualifiedProfileVerifier,
    promotion_attestation_verifier: PromotionAttestationVerifier,
) -> VerifiedBundle:
    verified = verify_internal_input(
        root,
        sidecar_verifier=sidecar_verifier,
        qualified_profile_verifier=qualified_profile_verifier,
        promotion_attestation_verifier=promotion_attestation_verifier,
        allow_legacy_internal_input_v1=True,
    )
    if (
        _sha256(verified.catalog_bytes) != descriptor.catalog_sha256
        or len(verified.expected_files) + 1 != descriptor.file_count
    ):
        raise BundleError(
            "internal compiler input differs from its package descriptor"
        )
    return verified


def extract_internal_package_archive(
    archive_path: Path,
    package_descriptor_path: Path,
    output_root: Path,
    *,
    sidecar_verifier: SidecarVerifier = verify_sidecar,
    qualified_profile_verifier: QualifiedProfileVerifier | None = None,
    promotion_attestation_verifier: PromotionAttestationVerifier | None = None,
) -> Path:
    """Verify and atomically extract one committed compressed compiler package."""

    archive_path = _require_absolute_normalized(
        archive_path, "internal standalone compiler package archive"
    )
    output_root = _require_absolute_normalized(
        output_root, "internal compiler input root"
    )
    _require_real_output_parent(output_root, "internal compiler input root")
    if output_root.exists() or output_root.is_symlink():
        raise BundleError("internal compiler input root must not exist")
    if qualified_profile_verifier is None:
        raise BundleError("the Rust typed qualified-profile verifier is required")
    if promotion_attestation_verifier is None:
        raise BundleError("an internal package promotion verifier is required")
    descriptor = read_internal_package_descriptor(package_descriptor_path)
    if archive_path.name != descriptor.asset:
        raise BundleError("internal compiler package archive name differs")

    temporary_root = Path(
        tempfile.mkdtemp(
            prefix=f".{output_root.name}.", suffix=".tmp", dir=output_root.parent
        )
    )
    temporary_identity = temporary_root.lstat()
    published_identity: os.stat_result | None = None
    complete = False
    try:
        entry_names, extraction_directories = _extract_pinned_archive(
            archive_path,
            descriptor.archive,
            descriptor.file_count,
            temporary_root,
            expected_compression=zipfile.ZIP_DEFLATED,
            maximum_archive_bytes=MAX_INTERNAL_PACKAGE_BYTES,
            raw_validator=_validate_canonical_internal_package_archive,
            archive_label="internal standalone compiler package archive",
            extraction_label="internal compiler package extraction tree",
        )
        with _pin_windows_directories(
            extraction_directories, "internal compiler package verification tree"
        ):
            actual = _enumerate_regular_files(
                temporary_root, "internal compiler input root"
            )
            if actual != entry_names:
                raise BundleError("internal compiler package file set changed")
            if INTERNAL_INPUT_DESCRIPTOR_FILE not in actual:
                raise BundleError(
                    f"internal compiler package has no {INTERNAL_INPUT_DESCRIPTOR_FILE}"
                )
            internal_input_bytes = _read_regular_no_follow(
                temporary_root / INTERNAL_INPUT_DESCRIPTOR_FILE,
                MAX_DESCRIPTOR_BYTES,
                "internal compiler input descriptor",
            )
            internal_input = _parse_json(
                internal_input_bytes,
                "internal compiler input descriptor",
                MAX_DESCRIPTOR_BYTES,
            )
            if internal_input.get("schema") != INTERNAL_INPUT_SCHEMA:
                raise BundleError("internal compiler input schema differs")
            if (
                _sha256(_canonical_pretty(_catalog(internal_input.get("catalog"))))
                != descriptor.catalog_sha256
            ):
                raise BundleError("internal compiler catalog differs from its package pin")
            verified = _verify_internal_input(
                temporary_root,
                descriptor,
                sidecar_verifier=sidecar_verifier,
                qualified_profile_verifier=qualified_profile_verifier,
                promotion_attestation_verifier=promotion_attestation_verifier,
            )
        if not _same_file_identity(temporary_identity, temporary_root.lstat()):
            raise BundleError("temporary internal compiler input changed identity")
        _publish_directory_no_replace(temporary_root, output_root)
        published_status = output_root.lstat()
        if not _same_file_identity(temporary_identity, published_status):
            raise BundleError("published internal compiler input changed identity")
        published_identity = published_status
        final_verified = _verify_internal_input(
            output_root,
            descriptor,
            sidecar_verifier=sidecar_verifier,
            qualified_profile_verifier=qualified_profile_verifier,
            promotion_attestation_verifier=promotion_attestation_verifier,
        )
        if (
            final_verified.expected_files != verified.expected_files
            or final_verified.descriptor_bytes != verified.descriptor_bytes
            or final_verified.catalog_bytes != verified.catalog_bytes
        ):
            raise BundleError("published internal compiler input changed after publication")
        complete = True
        return output_root
    except (OSError, zipfile.BadZipFile) as error:
        raise BundleError(f"cannot extract internal compiler package: {error}") from error
    finally:
        if not complete and published_identity is not None:
            _remove_directory_if_owned(output_root, published_identity)
        if not complete:
            _remove_directory_if_owned(temporary_root, temporary_identity)


def materialize_internal_package(
    archive_path: Path,
    package_descriptor_path: Path,
    output_root: Path,
    *,
    sidecar_verifier: SidecarVerifier = verify_sidecar,
    qualified_profile_verifier: QualifiedProfileVerifier | None = None,
    promotion_attestation_verifier: PromotionAttestationVerifier = (
        trust_pinned_internal_package_attestation
    ),
) -> Path:
    """Return a verified content-addressed extraction, creating it once if needed."""

    archive_path = _require_absolute_normalized(
        archive_path, "internal standalone compiler package archive"
    )
    output_root = _require_absolute_normalized(
        output_root, "internal compiler input root"
    )
    if qualified_profile_verifier is None:
        raise BundleError("the Rust typed qualified-profile verifier is required")
    descriptor = read_internal_package_descriptor(package_descriptor_path)
    _verify_internal_archive_pin(archive_path, descriptor)
    if output_root.exists() or output_root.is_symlink():
        if output_root.is_symlink() or not output_root.is_dir():
            raise BundleError("cached internal compiler input is not a real directory")
        _verify_internal_input(
            output_root,
            descriptor,
            sidecar_verifier=sidecar_verifier,
            qualified_profile_verifier=qualified_profile_verifier,
            promotion_attestation_verifier=promotion_attestation_verifier,
        )
        return output_root
    return extract_internal_package_archive(
        archive_path,
        package_descriptor_path,
        output_root,
        sidecar_verifier=sidecar_verifier,
        qualified_profile_verifier=qualified_profile_verifier,
        promotion_attestation_verifier=promotion_attestation_verifier,
    )


def verify_staged_bundle(
    root: Path,
    *,
    sidecar_verifier: SidecarVerifier = verify_sidecar,
    qualified_profile_verifier: QualifiedProfileVerifier | None = None,
    promotion_attestation_verifier: PromotionAttestationVerifier | None = None,
    allow_legacy_internal_input_v1: bool = False,
) -> VerifiedBundle:
    if qualified_profile_verifier is None:
        raise BundleError("the Rust typed qualified-profile verifier is required")
    if promotion_attestation_verifier is None:
        raise BundleError("the GitHub promotion attestation verifier is required")
    root = _require_absolute_normalized(root, "staged compiler bundle root")
    descriptor = _read_regular_no_follow(
        root / BUNDLE_DESCRIPTOR, MAX_DESCRIPTOR_BYTES, "compiler bundle manifest"
    )
    return _verify_descriptor(
        root,
        descriptor,
        sidecar_verifier=sidecar_verifier,
        qualified_profile_verifier=qualified_profile_verifier,
        promotion_attestation_verifier=promotion_attestation_verifier,
        staged=True,
        allow_legacy_internal_input_v1=allow_legacy_internal_input_v1,
    )


def _remove_work_root(work_root: Path) -> None:
    if not work_root.exists() and not work_root.is_symlink():
        return
    _check_no_follow_chain(work_root, "compiler bundle work root")
    status = work_root.lstat()
    if not stat.S_ISDIR(status.st_mode) or _is_reparse(status):
        raise BundleError(
            f"refusing to remove unsafe compiler bundle work root: {work_root}"
        )
    shutil.rmtree(work_root)


def _copy_expected_file(
    source_root: Path, destination_root: Path, relative: str, seal: Seal
) -> None:
    source = source_root.joinpath(*PurePosixPath(relative).parts)
    bytes_ = _read_regular_no_follow(
        source, max(seal.byte_len, 1), f"internal input {relative}"
    )
    _check_sealed_bytes(bytes_, seal, f"internal input {relative}")
    _write_new(destination_root.joinpath(*PurePosixPath(relative).parts), bytes_)


def prepare_product_bundle(
    internal_input_root: Path | None,
    work_root: Path,
    *,
    sidecar_verifier: SidecarVerifier = verify_sidecar,
    qualified_profile_verifier: QualifiedProfileVerifier | None = None,
    promotion_attestation_verifier: PromotionAttestationVerifier | None = None,
    allow_legacy_internal_input_v1: bool = False,
) -> PreparedBundle:
    """Create one clean pre-host-build catalog/bundle workspace.

    `None` actively removes stale product data and writes a zero-byte embedded catalog.  A present
    internal input is verified before copying, then the copied tree is independently reverified.
    """

    work_root = _require_absolute_normalized(work_root, "compiler bundle work root")
    internal_input = (
        verify_internal_input(
            internal_input_root,
            sidecar_verifier=sidecar_verifier,
            qualified_profile_verifier=qualified_profile_verifier,
            promotion_attestation_verifier=promotion_attestation_verifier,
            allow_legacy_internal_input_v1=allow_legacy_internal_input_v1,
        )
        if internal_input_root is not None
        else None
    )
    _remove_work_root(work_root)
    work_root.mkdir(parents=True)
    catalog_path = work_root / EMBEDDED_CATALOG_FILE
    if internal_input is None:
        _write_new(catalog_path, b"")
        return PreparedBundle(False, work_root, catalog_path, None, None, _sha256(b""))
    assert internal_input_root is not None
    bundle_root = work_root / "compiler"
    bundle_root.mkdir()
    for relative, seal in sorted(internal_input.expected_files.items()):
        _copy_expected_file(internal_input_root, bundle_root, relative, seal)
    _write_new(bundle_root / BUNDLE_DESCRIPTOR, internal_input.descriptor_bytes)
    _write_new(bundle_root / CATALOG_FILE, internal_input.catalog_bytes)
    _write_new(catalog_path, internal_input.catalog_bytes)
    verified = verify_staged_bundle(
        bundle_root,
        sidecar_verifier=sidecar_verifier,
        qualified_profile_verifier=qualified_profile_verifier,
        promotion_attestation_verifier=promotion_attestation_verifier,
        allow_legacy_internal_input_v1=allow_legacy_internal_input_v1,
    )
    if verified.expected_files != internal_input.expected_files:
        raise BundleError(
            "copied compiler bundle differs from its immutable internal input"
        )
    return PreparedBundle(
        True,
        work_root,
        catalog_path,
        bundle_root,
        internal_input.sidecar_name,
        _sha256(internal_input.catalog_bytes),
        internal_input.internal_input_schema_version == 1,
    )


def stage_product_bundle(
    prepared: PreparedBundle,
    destination_parent: Path,
    *,
    sidecar_verifier: SidecarVerifier = verify_sidecar,
    qualified_profile_verifier: QualifiedProfileVerifier | None = None,
    promotion_attestation_verifier: PromotionAttestationVerifier | None = None,
) -> Path | None:
    """Stage the prepared bytes beside one CLI/Studio host, removing stale data on absence."""

    destination_parent = _require_absolute_normalized(
        destination_parent, "host staging root"
    )
    destination = destination_parent / "compiler"
    if destination.exists() or destination.is_symlink():
        _remove_work_root(destination)
    if not prepared.present:
        return None
    assert prepared.bundle_root is not None
    shutil.copytree(prepared.bundle_root, destination, copy_function=shutil.copy2)
    verified = verify_staged_bundle(
        destination,
        sidecar_verifier=sidecar_verifier,
        qualified_profile_verifier=qualified_profile_verifier,
        promotion_attestation_verifier=promotion_attestation_verifier,
        allow_legacy_internal_input_v1=prepared.legacy_internal_input_v1,
    )
    if _sha256(verified.catalog_bytes) != prepared.catalog_sha256:
        raise BundleError("host-staged compiler catalog changed")
    return destination


def record_internal_input(
    signed_sidecar: Path,
    qualified_profile_roots: list[Path],
    output_root: Path,
    *,
    full_tree_receipts: list[Path],
    promotion_identity: Path,
    promotion_provenance: Path,
    promotion_attestation: Path,
    expected_repository: str,
    expected_commit: str,
    sidecar_verifier: SidecarVerifier = verify_sidecar,
    qualified_profile_verifier: QualifiedProfileVerifier | None = None,
    promotion_attestation_verifier: PromotionAttestationVerifier | None = None,
    notice_sources: dict[str, Path] | None = None,
) -> VerifiedBundle:
    """Copy final signed/qualified bytes into one new immutable internal-input directory.

    This does not sign or qualify anything. It is intentionally ordered after both operations and
    refuses a profile whose parity reports identify any other sidecar bytes or protocol, or whose
    paired full-tree receipt does not prove the strict release-scale differential contract.
    """

    signed_sidecar = _require_absolute_normalized(signed_sidecar, "signed sidecar")
    promotion_identity = _require_absolute_normalized(
        promotion_identity, "compiler promotion signed identity"
    )
    promotion_provenance = _require_absolute_normalized(
        promotion_provenance, "compiler promotion source provenance"
    )
    promotion_attestation = _require_absolute_normalized(
        promotion_attestation, "compiler promotion GitHub attestation"
    )
    output_root = _require_absolute_normalized(output_root, "internal-input output root")
    expected_repository = _require_repository(
        expected_repository, "authorized compiler promotion repository"
    )
    expected_commit = _require_hex(
        expected_commit, 40, "authorized compiler promotion commit"
    )
    if not qualified_profile_roots:
        raise BundleError("at least one qualified compiler profile root is required")
    if len(full_tree_receipts) != len(qualified_profile_roots):
        raise BundleError(
            "recording requires exactly one full-tree receipt for each qualified profile root"
        )
    if qualified_profile_verifier is None:
        raise BundleError("the Rust typed qualified-profile verifier is required")
    if promotion_attestation_verifier is None:
        raise BundleError("the GitHub promotion attestation verifier is required")
    _require_real_output_parent(output_root, "internal-input output root")
    if output_root.exists() or output_root.is_symlink():
        raise BundleError("internal-input output root must not exist")
    sidecar_bytes = _read_regular_no_follow(
        signed_sidecar, MAX_SIDECAR_BYTES, "signed sidecar"
    )
    sidecar_verifier(signed_sidecar, sidecar_bytes)
    if len(sidecar_bytes) in STALE_SIDECAR_BYTE_LENGTHS:
        raise BundleError("retired 273408-byte standalone sidecar is forbidden")
    sidecar = {
        "relative_path": SIDECAR_FILE,
        "byte_len": len(sidecar_bytes),
        "sha256": _sha256(sidecar_bytes),
        "protocol": {
            "request_version": PRODUCTION_REQUEST_VERSION,
            "response_version": PROTOCOL_RESPONSE_VERSION,
        },
        "static_system_only": True,
    }
    sidecar_seal = Seal(len(sidecar_bytes), _sha256(sidecar_bytes))
    promotion_identity_bytes = _read_regular_no_follow(
        promotion_identity,
        MAX_DESCRIPTOR_BYTES,
        "compiler promotion signed identity",
    )
    promotion_provenance_bytes = _read_regular_no_follow(
        promotion_provenance,
        MAX_DESCRIPTOR_BYTES,
        "compiler promotion source provenance",
    )
    promotion_attestation_bytes = _read_regular_no_follow(
        promotion_attestation,
        MAX_ATTESTATION_BYTES,
        "compiler promotion GitHub attestation",
    )
    promotion_identity_seal = Seal(
        len(promotion_identity_bytes), _sha256(promotion_identity_bytes)
    )
    _parse_signed_sidecar_identity(promotion_identity_bytes, sidecar_seal)
    promotion_authority = _parse_promotion_provenance(
        promotion_provenance_bytes,
        promotion_identity_seal,
        expected_repository=expected_repository,
        expected_commit=expected_commit,
    )
    promotion_attestation_verifier(
        promotion_attestation,
        {
            SIDECAR_FILE: signed_sidecar,
            PurePosixPath(PROMOTION_IDENTITY_FILE).name: promotion_identity,
            PurePosixPath(PROMOTION_PROVENANCE_FILE).name: promotion_provenance,
        },
        promotion_authority,
    )
    identity = _sidecar_identity(sidecar)
    profile_plans: list[RecordedProfilePlan] = []
    output_names: set[str] = set()
    for source_root, full_tree_receipt in zip(
        qualified_profile_roots, full_tree_receipts, strict=True
    ):
        source_root = _require_absolute_normalized(
            source_root, "qualified profile root"
        )
        full_tree_receipt = _require_absolute_normalized(
            full_tree_receipt, "full-tree verification receipt"
        )
        manifest_path = source_root / "compiler-profile.json"
        manifest = _read_regular_no_follow(
            manifest_path,
            MAX_PROFILE_MANIFEST_BYTES,
            "qualified compiler profile manifest",
        )
        profile = _parse_json(
            manifest, "qualified compiler profile manifest", MAX_PROFILE_MANIFEST_BYTES
        )
        if (
            profile.get("schema") != "gore.as.compiler-profile"
            or profile.get("schema_version") != 1
        ):
            raise BundleError(
                "qualified compiler profile schema/version is unsupported"
            )
        profile_sha256 = _require_hex(
            profile.get("profile_sha256"), 64, "qualified compiler profile SHA-256"
        )
        target = profile.get("target")
        oracle = profile.get("oracle")
        if not isinstance(target, dict) or not isinstance(oracle, dict):
            raise BundleError("qualified compiler profile target/oracle is invalid")
        catalog_target = {"target": target, "pe_codeview": oracle.get("pe_codeview")}
        key = _target_key(catalog_target)
        blobs = _profile_blob_seals(profile)
        for relative, seal, label in blobs:
            payload = _read_regular_no_follow(
                source_root.joinpath(*PurePosixPath(relative).parts),
                MAX_PROFILE_BLOB_BYTES,
                label,
            )
            _check_sealed_bytes(payload, seal, label)
        _verify_qualification_identity(source_root, profile, blobs, identity)
        promotion_audits = _verify_profile_promotion(
            source_root, profile, manifest, blobs, identity
        )
        build_id = target.get("steam_build_id")
        if isinstance(build_id, bool) or not isinstance(build_id, int) or build_id <= 0:
            raise BundleError("qualified compiler profile Steam BuildID is invalid")
        guid = catalog_target["pe_codeview"]
        assert isinstance(guid, dict)
        guid_text = str(guid.get("guid", "")).casefold()
        output_name = f"build-{build_id}-{guid_text}"
        _safe_relative(output_name, "qualified profile output directory")
        if output_name.casefold() in output_names:
            raise BundleError("qualified profile targets collide in the internal input")
        output_names.add(output_name.casefold())
        manifest_relative = f"profiles/{output_name}/compiler-profile.json"
        catalog_entry = {
            "manifest_relative_path": manifest_relative,
            "manifest_byte_len": len(manifest),
            "manifest_sha256": _sha256(manifest),
            "profile_sha256": profile_sha256,
            "target": catalog_target,
        }
        full_tree_bytes = _read_regular_no_follow(
            full_tree_receipt,
            MAX_FULL_TREE_RECEIPT_BYTES,
            "full-tree verification receipt",
        )
        _parse_full_tree_receipt(
            full_tree_bytes, catalog_entry, profile, sidecar
        )
        full_tree_relative = _full_tree_receipt_relative(catalog_entry)
        full_tree_entry = {
            "profile_sha256": profile_sha256,
            "relative_path": full_tree_relative,
            **_seal_bytes(full_tree_bytes),
        }
        profile_plans.append(
            RecordedProfilePlan(
                key=key,
                source_root=source_root,
                manifest=manifest,
                blobs=blobs,
                promotion_audits=promotion_audits,
                catalog_entry=catalog_entry,
                full_tree_relative=full_tree_relative,
                full_tree_bytes=full_tree_bytes,
                full_tree_entry=full_tree_entry,
            )
        )
    profile_plans.sort(key=lambda plan: plan.key)
    if any(
        profile_plans[index - 1].key == profile_plans[index].key
        for index in range(1, len(profile_plans))
    ):
        raise BundleError("qualified internal input contains duplicate target tuples")
    catalog = {
        "schema": CATALOG_SCHEMA,
        "schema_version": CATALOG_SCHEMA_VERSION,
        "sidecar": sidecar,
        "profiles": [plan.catalog_entry for plan in profile_plans],
    }
    _require_product_profile_targets(_catalog(catalog))
    sources = notice_sources or {
        "UNREANGEL-LICENSE.md": ROOT
        / "crates/gore-as/native/standalone-compiler/vendor/unreangel/UNREANGEL-LICENSE.md",
        "SOURCE_INVENTORY.tsv": ROOT
        / "crates/gore-as/native/standalone-compiler/SOURCE_INVENTORY.tsv",
        "PROVENANCE.toml": ROOT
        / "crates/gore-as/native/standalone-compiler/PROVENANCE.toml",
    }
    if set(sources) != set(REQUIRED_NOTICES):
        raise BundleError("internal-input notice sources are incomplete")
    notice_bytes = {
        name: _read_regular_no_follow(
            _require_absolute_normalized(path, f"notice source {name}"),
            MAX_DESCRIPTOR_BYTES,
            f"notice source {name}",
        )
        for name, path in sources.items()
    }
    descriptor = {
        "schema": INTERNAL_INPUT_SCHEMA,
        "schema_version": INTERNAL_INPUT_SCHEMA_VERSION,
        "immutable": True,
        "qualified": True,
        "promotion": {
            "repository": promotion_authority.repository,
            "commit": promotion_authority.commit,
            "workflow_sha": promotion_authority.workflow_sha,
            "workflow_run_id": promotion_authority.workflow_run_id,
            "workflow_run_attempt": promotion_authority.workflow_run_attempt,
            "signed_identity_file": {
                "relative_path": PROMOTION_IDENTITY_FILE,
                **_seal_bytes(promotion_identity_bytes),
            },
            "source_provenance_file": {
                "relative_path": PROMOTION_PROVENANCE_FILE,
                **_seal_bytes(promotion_provenance_bytes),
            },
            "github_attestation_file": {
                "relative_path": PROMOTION_ATTESTATION_FILE,
                **_seal_bytes(promotion_attestation_bytes),
            },
        },
        "catalog": catalog,
        "full_tree_verifications": [
            plan.full_tree_entry for plan in profile_plans
        ],
        "notices": {name: _seal_bytes(bytes_) for name, bytes_ in notice_bytes.items()},
    }
    descriptor_bytes = _canonical_pretty(descriptor)
    temporary_root = Path(
        tempfile.mkdtemp(
            prefix=f".{output_root.name}.",
            suffix=".tmp",
            dir=output_root.parent,
        )
    )
    temporary_identity = temporary_root.lstat()
    published_identity: os.stat_result | None = None
    complete = False
    try:
        _write_new(temporary_root / SIDECAR_FILE, sidecar_bytes)
        _write_new(
            temporary_root.joinpath(*PurePosixPath(PROMOTION_IDENTITY_FILE).parts),
            promotion_identity_bytes,
        )
        _write_new(
            temporary_root.joinpath(*PurePosixPath(PROMOTION_PROVENANCE_FILE).parts),
            promotion_provenance_bytes,
        )
        _write_new(
            temporary_root.joinpath(*PurePosixPath(PROMOTION_ATTESTATION_FILE).parts),
            promotion_attestation_bytes,
        )
        for plan in profile_plans:
            manifest_relative = str(plan.catalog_entry["manifest_relative_path"])
            destination_root = temporary_root.joinpath(
                *PurePosixPath(manifest_relative).parent.parts
            )
            _write_new(destination_root / "compiler-profile.json", plan.manifest)
            copied: set[str] = set()
            for relative, seal, _ in plan.blobs:
                if relative.casefold() in copied:
                    continue
                copied.add(relative.casefold())
                _copy_expected_file(
                    plan.source_root, destination_root, relative, seal
                )
            for relative, seal in sorted(plan.promotion_audits.items()):
                _copy_expected_file(
                    plan.source_root, destination_root, relative, seal
                )
            qualified_profile_verifier(
                destination_root,
                str(plan.catalog_entry["profile_sha256"]),
            )
            _write_new(
                temporary_root.joinpath(
                    *PurePosixPath(plan.full_tree_relative).parts
                ),
                plan.full_tree_bytes,
            )
        for name, bytes_ in notice_bytes.items():
            _write_new(temporary_root / name, bytes_)
        _write_new(temporary_root / INTERNAL_INPUT_DESCRIPTOR_FILE, descriptor_bytes)
        verified = verify_internal_input(
            temporary_root,
            sidecar_verifier=sidecar_verifier,
            qualified_profile_verifier=qualified_profile_verifier,
            promotion_attestation_verifier=promotion_attestation_verifier,
        )
        if not _same_file_identity(temporary_identity, temporary_root.lstat()):
            raise BundleError(
                "temporary internal-input root changed identity before publication"
            )
        _publish_directory_no_replace(temporary_root, output_root)
        published_status = output_root.lstat()
        if not _same_file_identity(temporary_identity, published_status):
            raise BundleError(
                "published internal-input root differs from its verified temporary root"
            )
        published_identity = published_status
        final_verified = verify_internal_input(
            output_root,
            sidecar_verifier=sidecar_verifier,
            qualified_profile_verifier=qualified_profile_verifier,
            promotion_attestation_verifier=promotion_attestation_verifier,
        )
        if (
            final_verified.expected_files != verified.expected_files
            or final_verified.descriptor_bytes != verified.descriptor_bytes
            or final_verified.catalog_bytes != verified.catalog_bytes
        ):
            raise BundleError(
                "published internal-input changed after atomic publication"
            )
        complete = True
        return final_verified
    finally:
        if not complete and published_identity is not None:
            _remove_directory_if_owned(output_root, published_identity)
        if not complete:
            _remove_directory_if_owned(temporary_root, temporary_identity)


def _seal_bytes(bytes_: bytes) -> dict[str, object]:
    return {"byte_len": len(bytes_), "sha256": _sha256(bytes_)}


def build_native_lanes(build_root: Path, *, dry_run: bool = False) -> None:
    """Configure/build/test the sidecar and capture tools as separate source-CMake lanes."""

    build_root = _require_absolute_normalized(build_root, "native build root")
    lanes = (
        ("sidecar", ROOT / "crates/gore-as/native/standalone-compiler"),
        ("capture", ROOT / "crates/gore-as/native/compiler-profile-capture"),
    )
    for name, source in lanes:
        build = build_root / name
        commands = (
            [
                "cmake",
                "-S",
                str(source),
                "-B",
                str(build),
                "-A",
                "x64",
                "-DBUILD_TESTING=ON",
            ],
            ["cmake", "--build", str(build), "--config", "Release"],
            ["ctest", "--test-dir", str(build), "-C", "Release", "--output-on-failure"],
        )
        for command in commands:
            print(f"[{name}] {' '.join(command)}")
            if dry_run:
                continue
            completed = subprocess.run(command, cwd=ROOT)
            if completed.returncode != 0:
                raise BundleError(f"native {name} lane failed: {' '.join(command)}")


def sign_sidecar_once(sidecar: Path, identity_output: Path) -> None:
    """Explicit one-time promotion step; regular GORE builds never call this function."""

    sidecar = _require_absolute_normalized(sidecar, "unsigned sidecar")
    identity_output = _require_absolute_normalized(
        identity_output, "sidecar identity output"
    )
    with _pin_windows_mutable_file_path(sidecar, "one-time sidecar signing target"):
        bytes_ = _read_regular_no_follow(sidecar, MAX_SIDECAR_BYTES, "unsigned sidecar")
        _verify_static_imports(bytes_)
        if _authenticode_entry_count(bytes_) != 0:
            raise BundleError(
                "refusing to re-sign a sidecar that already has Authenticode data"
            )
        _verify_pinned_production_capabilities(sidecar, bytes_)
        sys.path.insert(0, str(ROOT))
        import build as gore_build  # pylint: disable=import-outside-toplevel

        gore_build.sign_paths([sidecar], dry=False)
        signed = _read_regular_no_follow(sidecar, MAX_SIDECAR_BYTES, "signed sidecar")
        verify_sidecar(sidecar, signed)
        _verify_pinned_production_capabilities(sidecar, signed)
    identity = {
        "schema": SIGNED_SIDECAR_IDENTITY_SCHEMA,
        "schema_version": SIGNED_SIDECAR_IDENTITY_SCHEMA_VERSION,
        "unsigned": _seal_bytes(bytes_),
        "signed": _seal_bytes(signed),
        "request_version": PRODUCTION_REQUEST_VERSION,
        "response_version": PROTOCOL_RESPONSE_VERSION,
    }
    _write_new(identity_output, _canonical_pretty(identity))


def _main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    native = commands.add_parser(
        "build-native", help="build/test sidecar and capture lanes"
    )
    native.add_argument("--build-root", type=Path, required=True)
    native.add_argument("--dry-run", action="store_true")
    prepare = commands.add_parser(
        "prepare", help="prepare an embedded catalog and staged bytes"
    )
    prepare.add_argument("--internal-input", type=Path)
    prepare.add_argument("--qualified-profile-verifier", type=Path)
    prepare.add_argument("--github-attestation-verifier", type=Path)
    prepare.add_argument("--work-root", type=Path, required=True)
    verify = commands.add_parser(
        "verify", help="independently verify a staged compiler bundle"
    )
    verify.add_argument("--bundle-root", type=Path, required=True)
    verify.add_argument("--qualified-profile-verifier", type=Path, required=True)
    verify.add_argument("--github-attestation-verifier", type=Path, required=True)
    record = commands.add_parser(
        "record-internal-input",
        help="record already signed and qualified bytes for GORE's internal package",
    )
    record.add_argument("--signed-sidecar", type=Path, required=True)
    record.add_argument("--promotion-identity", type=Path, required=True)
    record.add_argument("--promotion-provenance", type=Path, required=True)
    record.add_argument("--promotion-attestation", type=Path, required=True)
    record.add_argument("--expected-repository", required=True)
    record.add_argument("--expected-commit", required=True)
    record.add_argument(
        "--qualified-profile-root", type=Path, action="append", required=True
    )
    record.add_argument(
        "--full-tree-receipt", type=Path, action="append", required=True
    )
    record.add_argument("--qualified-profile-verifier", type=Path, required=True)
    record.add_argument("--github-attestation-verifier", type=Path, required=True)
    record.add_argument("--output", type=Path, required=True)
    pack_internal = commands.add_parser(
        "pack-internal-package",
        help="pack one verified internal input into GORE's compressed asset",
    )
    pack_internal.add_argument(
        "--internal-input", type=Path, required=True
    )
    pack_internal.add_argument(
        "--qualified-profile-verifier", type=Path, required=True
    )
    pack_internal.add_argument(
        "--github-attestation-verifier", type=Path, required=True
    )
    pack_internal.add_argument("--archive", type=Path, required=True)
    pack_internal.add_argument("--descriptor-output", type=Path, required=True)
    extract_internal = commands.add_parser(
        "extract-internal-package",
        help="verify and safely extract GORE's pinned internal compiler asset",
    )
    extract_internal.add_argument("--archive", type=Path, required=True)
    extract_internal.add_argument("--descriptor", type=Path, required=True)
    extract_internal.add_argument(
        "--qualified-profile-verifier", type=Path, required=True
    )
    extract_internal.add_argument("--output", type=Path, required=True)
    sign = commands.add_parser(
        "sign-sidecar-once", help="sign one previously unsigned sidecar"
    )
    sign.add_argument("--sidecar", type=Path, required=True)
    sign.add_argument("--identity-output", type=Path, required=True)
    args = parser.parse_args()
    try:
        if args.command == "build-native":
            build_native_lanes(args.build_root, dry_run=args.dry_run)
        elif args.command == "prepare":
            if (
                args.internal_input is not None
                and args.qualified_profile_verifier is None
            ):
                raise BundleError(
                    "--qualified-profile-verifier is required with --internal-input"
                )
            if (
                args.internal_input is not None
                and args.github_attestation_verifier is None
            ):
                raise BundleError(
                    "--github-attestation-verifier is required with --internal-input"
                )
            qualified_verifier = (
                None
                if args.qualified_profile_verifier is None
                else qualified_profile_verifier_from_path(
                    args.qualified_profile_verifier
                )
            )
            prepared = prepare_product_bundle(
                args.internal_input,
                args.work_root,
                qualified_profile_verifier=qualified_verifier,
                promotion_attestation_verifier=(
                    None
                    if args.github_attestation_verifier is None
                    else promotion_attestation_verifier_from_path(
                        args.github_attestation_verifier
                    )
                ),
            )
            print(
                json.dumps(
                    {
                        "present": prepared.present,
                        "catalog_path": str(prepared.catalog_path),
                        "bundle_root": str(prepared.bundle_root)
                        if prepared.bundle_root
                        else None,
                        "catalog_sha256": prepared.catalog_sha256,
                    },
                    indent=2,
                )
            )
        elif args.command == "verify":
            verified = verify_staged_bundle(
                args.bundle_root,
                qualified_profile_verifier=qualified_profile_verifier_from_path(
                    args.qualified_profile_verifier
                ),
                promotion_attestation_verifier=promotion_attestation_verifier_from_path(
                    args.github_attestation_verifier
                ),
            )
            print(
                json.dumps(
                    {
                        "catalog_sha256": _sha256(verified.catalog_bytes),
                        "sidecar": verified.sidecar_name,
                        "file_count": len(verified.expected_files) + 2,
                    },
                    indent=2,
                )
            )
        elif args.command == "record-internal-input":
            verified = record_internal_input(
                args.signed_sidecar,
                args.qualified_profile_root,
                args.output,
                full_tree_receipts=args.full_tree_receipt,
                promotion_identity=args.promotion_identity,
                promotion_provenance=args.promotion_provenance,
                promotion_attestation=args.promotion_attestation,
                expected_repository=args.expected_repository,
                expected_commit=args.expected_commit,
                qualified_profile_verifier=qualified_profile_verifier_from_path(
                    args.qualified_profile_verifier
                ),
                promotion_attestation_verifier=promotion_attestation_verifier_from_path(
                    args.github_attestation_verifier
                ),
            )
            print(
                json.dumps(
                    {
                        "internal_input": str(args.output),
                        "catalog_sha256": _sha256(verified.catalog_bytes),
                        "sidecar": verified.sidecar_name,
                        "file_count": len(verified.expected_files) + 1,
                    },
                    indent=2,
                )
            )
        elif args.command == "pack-internal-package":
            descriptor = pack_internal_package_archive(
                args.internal_input,
                args.archive,
                args.descriptor_output,
                qualified_profile_verifier=qualified_profile_verifier_from_path(
                    args.qualified_profile_verifier
                ),
                promotion_attestation_verifier=promotion_attestation_verifier_from_path(
                    args.github_attestation_verifier
                ),
            )
            print(
                json.dumps(
                    {
                        "asset": descriptor.asset,
                        "archive_byte_len": descriptor.archive.byte_len,
                        "archive_sha256": descriptor.archive.sha256,
                        "compression": descriptor.compression,
                        "catalog_sha256": descriptor.catalog_sha256,
                        "file_count": descriptor.file_count,
                        "descriptor": str(args.descriptor_output),
                    },
                    indent=2,
                )
            )
        elif args.command == "extract-internal-package":
            extracted = extract_internal_package_archive(
                args.archive,
                args.descriptor,
                args.output,
                qualified_profile_verifier=qualified_profile_verifier_from_path(
                    args.qualified_profile_verifier
                ),
                promotion_attestation_verifier=trust_pinned_internal_package_attestation,
            )
            descriptor = read_internal_package_descriptor(args.descriptor)
            print(
                json.dumps(
                    {
                        "internal_input": str(extracted),
                        "catalog_sha256": descriptor.catalog_sha256,
                        "file_count": descriptor.file_count,
                    },
                    indent=2,
                )
            )
        else:
            sign_sidecar_once(args.sidecar, args.identity_output)
    except (BundleError, OSError) as error:
        print(f"standalone compiler bundle failed: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(_main())
