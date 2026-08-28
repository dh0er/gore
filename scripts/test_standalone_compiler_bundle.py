from __future__ import annotations

import copy
from contextlib import contextmanager
import hashlib
import json
import os
from pathlib import Path
import shutil
import stat
import struct
import sys
import tempfile
import unittest
from unittest import mock
import zipfile

sys.path.insert(0, str(Path(__file__).resolve().parent))
import standalone_compiler_bundle as bundle


def _sha256(bytes_: bytes) -> str:
    return hashlib.sha256(bytes_).hexdigest()


def _seal(bytes_: bytes) -> dict[str, object]:
    return {"byte_len": len(bytes_), "sha256": _sha256(bytes_)}


def _snapshot(root: Path) -> dict[str, bytes]:
    return {
        path.relative_to(root).as_posix(): path.read_bytes()
        for path in root.rglob("*")
        if path.is_file()
    }


class SyntheticInternalInput:
    def __init__(
        self, base: Path, *, sidecar_bytes: bytes = b"signed-synthetic-sidecar"
    ) -> None:
        self.root = base / "immutable-input"
        self.root.mkdir()
        self.sidecar_bytes = sidecar_bytes
        (self.root / bundle.SIDECAR_FILE).write_bytes(sidecar_bytes)
        self.profile_root = self.root / "profiles" / "build-24539464"
        self.profile_root.mkdir(parents=True)
        self.sidecar_identity = {
            "byte_len": len(sidecar_bytes),
            "sha256": _sha256(sidecar_bytes),
            "request_version": bundle.PRODUCTION_REQUEST_VERSION,
            "response_version": 1,
        }
        self.promotion_repository = "dh0er/gore"
        self.promotion_commit = "12" * 20
        self.promotion_workflow_sha = "56" * 20
        self.promotion_run_id = 123456789
        self.promotion_run_attempt = 1
        self.target = {
            "steam_app_id": 1_297_900,
            "steam_build_id": 24_539_464,
            "depot_id": 1_297_901,
            "depot_manifest_gid": 1_585_071_322_101_748_861,
            "platform": "windows",
            "architecture": "x86_64",
            "build_configuration": "shipping",
        }
        self.codeview = {"guid": "cf0b83bd-e023-061b-2100-0f0fccf871d2", "age": 1}
        self.profile_sha256 = "ab" * 32
        self.blobs = self._write_profile_payloads(self.profile_root)
        self.profile = self._profile()
        current_target = copy.deepcopy(
            bundle.REQUIRED_PRODUCT_COMPILER_TARGETS_V1[1]
        )
        self.current_target = current_target["target"]
        self.current_codeview = current_target["pe_codeview"]
        self.current_profile_sha256 = "cd" * 32
        self.current_profile_root = self.root / "profiles" / "build-24878692"
        self.current_profile_root.mkdir(parents=True)
        self.current_blobs = self._write_profile_payloads(
            self.current_profile_root, marker_prefix="current:"
        )
        self.current_profile = self._profile_for(
            self.current_target,
            self.current_codeview,
            self.current_profile_sha256,
            self.current_blobs,
        )
        self.profile_records: list[dict[str, object]] = [
            {
                "root": self.profile_root,
                "target": self.target,
                "codeview": self.codeview,
                "profile_sha256": self.profile_sha256,
                "blobs": self.blobs,
                "profile": self.profile,
            },
            {
                "root": self.current_profile_root,
                "target": self.current_target,
                "codeview": self.current_codeview,
                "profile_sha256": self.current_profile_sha256,
                "blobs": self.current_blobs,
                "profile": self.current_profile,
            },
        ]
        self.profile_roots = [
            record["root"] for record in self.profile_records
        ]
        self._write_descriptor()

    def _write_profile_payloads(
        self, profile_root: Path, *, marker_prefix: str = ""
    ) -> dict[tuple[str, str], dict[str, object]]:
        blobs: dict[tuple[str, str], dict[str, object]] = {}
        for index, (group, field) in enumerate(bundle.PROFILE_BLOB_FIELDS):
            relative = f"payload/{index:02d}-{field}.bin"
            if field in (
                "codegen_probe_corpus",
                "expected_probe_results",
                "diagnostic_parity",
                "semantic_parity",
            ):
                document = {
                    "schema": f"synthetic.{field}",
                    "schema_version": 1,
                    "canonical_sha256": f"{index + 1:064x}",
                }
                if field in ("diagnostic_parity", "semantic_parity"):
                    document["standalone_compiler"] = self.sidecar_identity
                payload = bundle._canonical_pretty(document)
            else:
                payload = f"synthetic:{marker_prefix}{group}.{field}\n".encode()
            path = profile_root / Path(relative)
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(payload)
            blobs[(group, field)] = {"path": relative, **_seal(payload)}
        return blobs

    def _profile(self) -> dict[str, object]:
        return self._profile_for(
            self.target, self.codeview, self.profile_sha256, self.blobs
        )

    @staticmethod
    def _profile_for(
        target: dict[str, object],
        codeview: dict[str, object],
        profile_sha256: str,
        blobs: dict[tuple[str, str], dict[str, object]],
    ) -> dict[str, object]:
        build_id = int(target["steam_build_id"])

        def oracle_file(marker: int) -> dict[str, object]:
            return {
                "byte_len": marker,
                "sha256": f"{build_id + marker:064x}",
                "steam_content_sha1": f"{build_id + marker:040x}",
            }

        return {
            "schema": "gore.as.compiler-profile",
            "schema_version": 1,
            "target": target,
            "oracle": {
                "executable": oracle_file(1),
                "binds_cache": oracle_file(2),
                "shipping_cache": oracle_file(3),
                "depot_manifest": oracle_file(4),
                "pe_codeview": codeview,
            },
            "binds": {},
            "engine": {
                "ordered_engine_properties": blobs[
                    ("engine", "ordered_engine_properties")
                ],
                "registration_trace": blobs[("engine", "registration_trace")],
                "post_bind_snapshot": blobs[("engine", "post_bind_snapshot")],
            },
            "unreal_semantics": {
                "reflected_type_graph": blobs[
                    ("unreal_semantics", "reflected_type_graph")
                ]
            },
            "frontend": {
                "preprocessor_config": blobs[("frontend", "preprocessor_config")],
                "class_generator_config": blobs[
                    ("frontend", "class_generator_config")
                ],
                "compiler_options": blobs[("frontend", "compiler_options")],
            },
            "bytecode": {
                "opcode_table": blobs[("bytecode", "opcode_table")],
                "operand_schema": blobs[("bytecode", "operand_schema")],
                "codegen_probe_corpus": blobs[
                    ("bytecode", "codegen_probe_corpus")
                ],
                "expected_probe_results": blobs[
                    ("bytecode", "expected_probe_results")
                ],
            },
            "cache_writer": {
                "serializer_schema": blobs[("cache_writer", "serializer_schema")],
                "reference_table_order": blobs[
                    ("cache_writer", "reference_table_order")
                ],
                "normalized_oracle_corpus": blobs[
                    ("cache_writer", "normalized_oracle_corpus")
                ],
            },
            "qualification": {
                "required_probe_suite_version": "synthetic-v1",
                "diagnostic_parity": blobs[("qualification", "diagnostic_parity")],
                "semantic_parity": blobs[("qualification", "semantic_parity")],
                "qualified": True,
            },
            "profile_sha256": profile_sha256,
        }

    def _write_descriptor(
        self, *, catalog_target: dict[str, object] | None = None
    ) -> None:
        self.profile_records[0].update(
            {
                "target": self.target,
                "codeview": self.codeview,
                "profile_sha256": self.profile_sha256,
                "blobs": self.blobs,
                "profile": self.profile,
            }
        )
        catalog_profiles: list[dict[str, object]] = []
        full_tree_verifications: list[dict[str, object]] = []
        self.full_tree_receipts: list[Path] = []
        for index, record in enumerate(self.profile_records):
            profile_root = record["root"]
            profile = record["profile"]
            target = record["target"]
            codeview = record["codeview"]
            profile_sha256 = record["profile_sha256"]
            blobs = record["blobs"]
            assert isinstance(profile_root, Path)
            assert isinstance(profile, dict)
            assert isinstance(target, dict)
            assert isinstance(codeview, dict)
            assert isinstance(profile_sha256, str)
            assert isinstance(blobs, dict)
            manifest = bundle._canonical_pretty(profile)
            manifest_path = profile_root / "compiler-profile.json"
            manifest_path.write_bytes(manifest)
            if index == 0:
                self.manifest = manifest_path
            self._write_promotion_audits(
                manifest,
                profile_root=profile_root,
                target=target,
                blobs=blobs,
                profile_sha256=profile_sha256,
            )
            build_id = int(target["steam_build_id"])
            catalog_entry = {
                "manifest_relative_path": (
                    f"profiles/build-{build_id}/compiler-profile.json"
                ),
                "manifest_byte_len": len(manifest),
                "manifest_sha256": _sha256(manifest),
                "profile_sha256": profile_sha256,
                "target": (
                    catalog_target
                    if index == 0 and catalog_target is not None
                    else {"target": target, "pe_codeview": codeview}
                ),
            }
            catalog_profiles.append(catalog_entry)
            receipt_bytes = self._full_tree_receipt(profile)
            receipt_relative = bundle._full_tree_receipt_relative(
                {
                    **catalog_entry,
                    "target": {"target": target, "pe_codeview": codeview},
                }
            )
            receipt_path = self.root.joinpath(
                *bundle.PurePosixPath(receipt_relative).parts
            )
            receipt_path.parent.mkdir(parents=True, exist_ok=True)
            receipt_path.write_bytes(receipt_bytes)
            self.full_tree_receipts.append(receipt_path)
            full_tree_verifications.append(
                {
                    "profile_sha256": profile_sha256,
                    "relative_path": receipt_relative,
                    **_seal(receipt_bytes),
                }
            )
        for name in bundle.REQUIRED_NOTICES:
            source = f"synthetic notice {name}\n".encode()
            (self.root / name).write_bytes(source)
        catalog = {
            "schema": bundle.CATALOG_SCHEMA,
            "schema_version": 1,
            "sidecar": {
                "relative_path": bundle.SIDECAR_FILE,
                **_seal(self.sidecar_bytes),
                "protocol": {
                    "request_version": bundle.PRODUCTION_REQUEST_VERSION,
                    "response_version": bundle.PROTOCOL_RESPONSE_VERSION,
                },
                "static_system_only": True,
            },
            "profiles": catalog_profiles,
        }
        notices = {
            name: _seal((self.root / name).read_bytes())
            for name in bundle.REQUIRED_NOTICES
        }
        promotion = self._write_internal_signing_provenance()
        self.descriptor = {
            "schema": bundle.INTERNAL_INPUT_SCHEMA,
            "schema_version": bundle.INTERNAL_INPUT_SCHEMA_VERSION,
            "immutable": True,
            "qualified": True,
            "promotion": promotion,
            "catalog": catalog,
            "full_tree_verifications": full_tree_verifications,
            "notices": notices,
        }
        (self.root / bundle.INTERNAL_INPUT_DESCRIPTOR_FILE).write_bytes(
            bundle._canonical_pretty(self.descriptor)
        )

    def _write_internal_signing_provenance(self) -> dict[str, object]:
        identity_path = self.root.joinpath(
            *bundle.PurePosixPath(bundle.PROMOTION_IDENTITY_FILE).parts
        )
        provenance_path = self.root.joinpath(
            *bundle.PurePosixPath(bundle.PROMOTION_PROVENANCE_FILE).parts
        )
        attestation_path = self.root.joinpath(
            *bundle.PurePosixPath(bundle.PROMOTION_ATTESTATION_FILE).parts
        )
        identity_path.parent.mkdir(parents=True, exist_ok=True)
        identity = {
            "schema": bundle.SIGNED_SIDECAR_IDENTITY_SCHEMA,
            "schema_version": bundle.SIGNED_SIDECAR_IDENTITY_SCHEMA_VERSION,
            "unsigned": _seal(b"u"),
            "signed": _seal(self.sidecar_bytes),
            "request_version": bundle.PRODUCTION_REQUEST_VERSION,
            "response_version": bundle.PROTOCOL_RESPONSE_VERSION,
        }
        identity_bytes = bundle._canonical_pretty(identity)
        identity_path.write_bytes(identity_bytes)
        provenance = {
            "schema": bundle.PROMOTION_PROVENANCE_SCHEMA,
            "schema_version": bundle.PROMOTION_PROVENANCE_SCHEMA_VERSION,
            "repository": self.promotion_repository,
            "commit": self.promotion_commit,
            "workflow_sha": self.promotion_workflow_sha,
            "workflow_run_id": self.promotion_run_id,
            "workflow_run_attempt": self.promotion_run_attempt,
            "signed_identity": _seal(identity_bytes),
        }
        provenance_bytes = bundle._canonical_pretty(provenance)
        provenance_path.write_bytes(provenance_bytes)
        attestation_bytes = b'{"synthetic":"github-attestation"}\n'
        attestation_path.write_bytes(attestation_bytes)
        return {
            "repository": self.promotion_repository,
            "commit": self.promotion_commit,
            "workflow_sha": self.promotion_workflow_sha,
            "workflow_run_id": self.promotion_run_id,
            "workflow_run_attempt": self.promotion_run_attempt,
            "signed_identity_file": {
                "relative_path": bundle.PROMOTION_IDENTITY_FILE,
                **_seal(identity_bytes),
            },
            "source_provenance_file": {
                "relative_path": bundle.PROMOTION_PROVENANCE_FILE,
                **_seal(provenance_bytes),
            },
            "github_attestation_file": {
                "relative_path": bundle.PROMOTION_ATTESTATION_FILE,
                **_seal(attestation_bytes),
            },
        }

    def _artifact_manifest(
        self,
        backend: str,
        marker: int,
        *,
        profile_root: Path,
        target: dict[str, object],
        blobs: dict[tuple[str, str], dict[str, object]],
    ) -> bytes:
        cache = {
            "blob_id": f"{backend}.0000.cache",
            "byte_len": marker,
            "sha256": f"{marker:064x}",
        }
        baseline_cache = {
            "blob_id": f"{backend}.0000.baseline.cache",
            "byte_len": marker + 100,
            "sha256": f"{marker + 100:064x}",
        }
        empty_semantics = {
            "semantic_sha256": f"{marker + 10:064x}",
            "observed_opcodes": [],
            "tail_table_counts": [0, 0, 0, 0, 0, 0, 0],
            "class_count": 0,
            "behaviour_function_count": 0,
            "property_count": 0,
            "global_count": 0,
            "initializer_function_count": 0,
            "string_global_reference_count": 0,
        }
        document = {
            "schema": "gore.as.offline-probe-artifacts",
            "schema_version": 1,
            "semantic_observer": "gore.as.whole-cache-semantic-observer/v1",
            "suite_id": "synthetic-v1",
            "corpus_sha256": self._payload_digest(
                "bytecode",
                "codegen_probe_corpus",
                profile_root=profile_root,
                blobs=blobs,
            ),
            "backend": backend,
            "source_profile_sha256": "11" * 32,
            "source_target": target,
            "standalone_compiler": (
                self.sidecar_identity if backend == "standalone" else None
            ),
            "entries": [
                {
                    "ordinal": 0,
                    "case_id": "positive.synthetic",
                    "outcome": "accepted",
                    "diagnostics": [],
                    "cache": cache,
                    "cache_semantics": empty_semantics,
                    "graph_transition": {
                        "baseline_cache": baseline_cache,
                        "baseline_cache_semantics": empty_semantics,
                        "baseline_sources": [],
                        "final_sources": [],
                        "changed_modules": [],
                        "deleted_modules": [],
                        "added_modules": [],
                        "baseline_cache_modules": [],
                        "final_cache_modules": [],
                    },
                }
            ],
            "canonical_sha256": "0" * 64,
        }
        document["canonical_sha256"] = bundle._domain_json_sha256(
            b"gore-as-offline-probe-artifacts-v1\0",
            document,
            include_length=True,
        )
        return bundle._canonical_pretty(document)

    @staticmethod
    def _payload_digest(
        group: str,
        field: str,
        *,
        profile_root: Path,
        blobs: dict[tuple[str, str], dict[str, object]],
    ) -> str:
        path = profile_root / str(blobs[(group, field)]["path"])
        return json.loads(path.read_text(encoding="utf-8"))["canonical_sha256"]

    def _write_promotion_audits(
        self,
        manifest: bytes,
        *,
        profile_root: Path,
        target: dict[str, object],
        blobs: dict[tuple[str, str], dict[str, object]],
        profile_sha256: str,
    ) -> None:
        embedded = self._artifact_manifest(
            "embedded_game",
            4,
            profile_root=profile_root,
            target=target,
            blobs=blobs,
        )
        standalone = self._artifact_manifest(
            "standalone",
            5,
            profile_root=profile_root,
            target=target,
            blobs=blobs,
        )
        embedded_path = (
            profile_root / bundle.EMBEDDED_QUALIFICATION_ARTIFACT_MANIFEST_FILE
        )
        standalone_path = (
            profile_root / bundle.STANDALONE_QUALIFICATION_ARTIFACT_MANIFEST_FILE
        )
        embedded_path.write_bytes(embedded)
        standalone_path.write_bytes(standalone)
        embedded_summary = bundle._offline_artifact_authority_summary(
            embedded, "embedded_game"
        )
        standalone_summary = bundle._offline_artifact_authority_summary(
            standalone, "standalone"
        )
        files = [{"path": "compiler-profile.json", **_seal(manifest)}]
        seen: set[str] = set()
        for blob in blobs.values():
            relative = str(blob["path"])
            if relative.casefold() in seen:
                continue
            seen.add(relative.casefold())
            files.append(
                {
                    "path": relative,
                    "byte_len": blob["byte_len"],
                    "sha256": blob["sha256"],
                }
            )
        files.extend(
            [
                {
                    "path": bundle.EMBEDDED_QUALIFICATION_ARTIFACT_MANIFEST_FILE,
                    **_seal(embedded),
                },
                {
                    "path": bundle.STANDALONE_QUALIFICATION_ARTIFACT_MANIFEST_FILE,
                    **_seal(standalone),
                },
            ]
        )
        files.sort(key=lambda value: str(value["path"]))
        receipt = {
            "schema": bundle.QUALIFIED_PROMOTION_RECEIPT_SCHEMA,
            "schema_version": bundle.QUALIFIED_PROMOTION_RECEIPT_SCHEMA_VERSION,
            "qualified": True,
            "source_profile_sha256": "11" * 32,
            "source_target": target,
            "source_materialization_receipt_sha256": "12" * 32,
            "capture_stream_sha256": "13" * 32,
            "static_support_manifest_sha256": "14" * 32,
            "standalone_compiler": self.sidecar_identity,
            "embedded_artifacts": embedded_summary,
            "standalone_artifacts": standalone_summary,
            "corpus_sha256": self._payload_digest(
                "bytecode",
                "codegen_probe_corpus",
                profile_root=profile_root,
                blobs=blobs,
            ),
            "expected_results_sha256": self._payload_digest(
                "bytecode",
                "expected_probe_results",
                profile_root=profile_root,
                blobs=blobs,
            ),
            "diagnostic_parity_sha256": self._payload_digest(
                "qualification",
                "diagnostic_parity",
                profile_root=profile_root,
                blobs=blobs,
            ),
            "semantic_parity_sha256": self._payload_digest(
                "qualification",
                "semantic_parity",
                profile_root=profile_root,
                blobs=blobs,
            ),
            "profile_sha256": profile_sha256,
            "files": files,
            "canonical_sha256": "0" * 64,
        }
        receipt["canonical_sha256"] = bundle._domain_json_sha256(
            b"gore-as-qualified-profile-promotion-v1\0",
            {key: value for key, value in receipt.items() if key != "canonical_sha256"},
            include_length=False,
        )
        (profile_root / bundle.QUALIFIED_PROMOTION_RECEIPT_FILE).write_bytes(
            bundle._canonical_pretty(receipt)
        )

    def _full_tree_receipt(self, profile: dict[str, object]) -> bytes:
        oracle = profile["oracle"]
        assert isinstance(oracle, dict)
        module_count = 3
        return bundle._canonical_pretty(
            {
                "schema": bundle.FULL_TREE_RECEIPT_SCHEMA,
                "version": bundle.FULL_TREE_RECEIPT_VERSION,
                "passed": True,
                "execution": {
                    "backend": "standalone",
                    "runner_invocations": 1,
                    "standalone_attempted": True,
                    "game_attempted": False,
                    "install_restore": "not_started",
                    "closing_audit": "passed",
                    "publication": "published",
                    "recovery_required": False,
                    "fallback_present": False,
                    "backend_diagnostic_count": 0,
                },
                "authority": {
                    "qualified_profile_sha256": profile["profile_sha256"],
                    "sidecar": self.sidecar_identity,
                    "shipping": {
                        "byte_len": oracle["shipping_cache"]["byte_len"],
                        "sha256": oracle["shipping_cache"]["sha256"],
                    },
                    "binds": {
                        "byte_len": oracle["binds_cache"]["byte_len"],
                        "sha256": oracle["binds_cache"]["sha256"],
                    },
                },
                "frozen_source": {
                    "module_count": module_count,
                    "byte_len": 30,
                    "aggregate_sha256": "31" * 32,
                    "operations": {"add": 0, "edit": module_count, "delete": 0},
                },
                "embedded_reference": {
                    "byte_len": 40,
                    "sha256": "32" * 32,
                },
                "standalone_candidate": {
                    "byte_len": 40,
                    "sha256": "33" * 32,
                    "module_count": module_count,
                },
                "bytediff": {
                    "equivalent_to_fail_on_semantic": True,
                    "context": 6,
                    "normalization": {
                        "n1_refs": True,
                        "n2_slots": False,
                        "n3_jumps": True,
                        "n4_consts": True,
                        "n5_scope": True,
                        "n6_reguard": True,
                    },
                    "aligned_functions": 10,
                    "identical": 8,
                    "benign": 2,
                    "semantic": 0,
                    "alignment_loss": 0,
                },
                "whole_cache_semantics_v1": {
                    "exact_struct_equality": True,
                    "semantic_sha256": "34" * 32,
                    "module_count": module_count,
                    "function_count": 10,
                    "opcode_counts": [0] * 213,
                    "class_count": 1,
                    "behaviour_function_count": 0,
                    "property_count": 2,
                    "global_count": 3,
                    "initializer_function_count": 0,
                    "string_global_reference_count": 4,
                    "tail_table_counts": [1, 1, 2, 2, 3, 4, 2],
                    "invoke_return_included": True,
                },
            }
        )


def _accept_synthetic_sidecar(_path: Path, _bytes: bytes) -> None:
    return None


def _accept_synthetic_profile(
    root: Path, _profile_sha256: str
) -> bundle.QualifiedProfileTreeAuthority:
    return bundle._qualified_profile_tree_summary(root)


def _accept_synthetic_attestation(
    _bundle_path: Path,
    _subjects: dict[str, Path],
    _authority: bundle.PromotionAuthority,
) -> None:
    return None


class StandaloneCompilerBundleTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.base = Path(self.temp.name).resolve()

    def tearDown(self) -> None:
        self.temp.cleanup()

    def test_sidecar_authenticode_checks_the_exact_measured_path_bytes(self) -> None:
        sidecar = self.base / "signed-sidecar.exe"
        measured = b"synthetic-signed-sidecar"
        sidecar.write_bytes(measured)
        pinned = False

        @bundle.contextmanager
        def tracked_pin(_path: Path, _label: str, *, require_single_link: bool = True):
            nonlocal pinned
            self.assertTrue(require_single_link)
            pinned = True
            try:
                yield
            finally:
                pinned = False

        def verify_imports(bytes_: bytes) -> None:
            self.assertTrue(pinned)
            self.assertEqual(bytes_, measured)

        def count_authenticode(bytes_: bytes) -> int:
            self.assertTrue(pinned)
            self.assertEqual(bytes_, measured)
            return 1

        def verify_authenticode(path: Path) -> None:
            self.assertTrue(pinned)
            self.assertEqual(path, sidecar)

        with (
            mock.patch.object(bundle, "_pin_windows_file_path", tracked_pin),
            mock.patch.object(bundle, "_verify_static_imports", verify_imports),
            mock.patch.object(bundle, "_authenticode_entry_count", count_authenticode),
            mock.patch.object(
                bundle, "_verify_authenticode_windows", verify_authenticode
            ),
        ):
            bundle.verify_sidecar(sidecar, measured)
            with self.assertRaisesRegex(bundle.BundleError, "differs from the bytes"):
                bundle.verify_sidecar(sidecar, b"different-measurement")

    def test_one_time_signing_pin_spans_unsigned_measurement_and_signed_verification(
        self,
    ) -> None:
        import build as gore_build

        sidecar = self.base / "candidate.exe"
        identity = self.base / "identity.json"
        events: list[str] = []

        @bundle.contextmanager
        def signing_pin(path: Path, label: str):
            self.assertEqual(path, sidecar)
            self.assertIn("one-time", label)
            events.append("pin-enter")
            yield
            events.append("pin-exit")

        with (
            mock.patch.object(
                bundle,
                "_pin_windows_mutable_file_path",
                side_effect=signing_pin,
            ),
            mock.patch.object(
                bundle,
                "_read_regular_no_follow",
                side_effect=[b"unsigned", b"signed"],
            ),
            mock.patch.object(bundle, "_verify_static_imports"),
            mock.patch.object(bundle, "_authenticode_entry_count", return_value=0),
            mock.patch.object(
                bundle,
                "_verify_pinned_production_capabilities",
                side_effect=lambda *_: events.append("capabilities"),
            ),
            mock.patch.object(
                gore_build,
                "sign_paths",
                side_effect=lambda *_args, **_kwargs: events.append("sign"),
            ),
            mock.patch.object(
                bundle,
                "verify_sidecar",
                side_effect=lambda *_: events.append("verify-signed"),
            ),
            mock.patch.object(bundle, "_write_new") as writer,
        ):
            bundle.sign_sidecar_once(sidecar, identity)

        self.assertEqual(
            events,
            [
                "pin-enter",
                "capabilities",
                "sign",
                "verify-signed",
                "capabilities",
                "pin-exit",
            ],
        )
        writer.assert_called_once()

    def test_github_attestation_verifier_binds_all_subjects_and_origin(self) -> None:
        verifier = Path(sys.executable).resolve()
        verifier_bytes = verifier.read_bytes()
        verifier_seal = bundle.Seal(len(verifier_bytes), _sha256(verifier_bytes))
        attestation = self.base / "github-attestation.sigstore.json"
        attestation.write_bytes(b'{"synthetic":"sigstore-bundle"}\n')
        subjects = {
            bundle.SIDECAR_FILE: self.base / bundle.SIDECAR_FILE,
            bundle.PurePosixPath(bundle.PROMOTION_IDENTITY_FILE).name: self.base
            / bundle.PurePosixPath(bundle.PROMOTION_IDENTITY_FILE).name,
            bundle.PurePosixPath(bundle.PROMOTION_PROVENANCE_FILE).name: self.base
            / bundle.PurePosixPath(bundle.PROMOTION_PROVENANCE_FILE).name,
        }
        for name, path in subjects.items():
            path.write_bytes(f"synthetic:{name}".encode("ascii"))
        authority = bundle.PromotionAuthority(
            repository="dh0er/gore",
            commit="12" * 20,
            workflow_sha="56" * 20,
            workflow_run_id=123,
            workflow_run_attempt=1,
        )
        statement_subjects = [
            {
                "name": name,
                "digest": {"sha256": _sha256(path.read_bytes())},
            }
            for name, path in subjects.items()
        ]
        completed = mock.Mock(
            returncode=0,
            stdout=json.dumps(
                [{"verificationResult": {"statement": {"subject": statement_subjects}}}]
            ).encode("utf-8"),
            stderr=b"",
        )
        with mock.patch.object(bundle.subprocess, "run", return_value=completed) as run:
            bundle.verify_github_attestation_with_executable(
                verifier,
                verifier_seal,
                attestation,
                subjects,
                authority,
            )
        self.assertEqual(run.call_count, 3)
        for call in run.call_args_list:
            command = call.args[0]
            self.assertIn("--bundle", command)
            self.assertIn("--signer-workflow", command)
            self.assertIn("--signer-digest", command)
            self.assertIn(authority.workflow_sha, command)
            self.assertIn("--source-digest", command)
            self.assertIn(authority.commit, command)
            self.assertIn("--deny-self-hosted-runners", command)

        wrong = copy.deepcopy(statement_subjects)
        wrong[0]["digest"]["sha256"] = "ab" * 32
        completed.stdout = json.dumps(
            [{"verificationResult": {"statement": {"subject": wrong}}}]
        ).encode("utf-8")
        with (
            mock.patch.object(bundle.subprocess, "run", return_value=completed),
            self.assertRaisesRegex(bundle.BundleError, "subject set differs"),
        ):
            bundle.verify_github_attestation_with_executable(
                verifier,
                verifier_seal,
                attestation,
                subjects,
                authority,
            )

    def test_synthetic_internal_input_prepares_and_stages_byte_identical_cli_and_studio(
        self,
    ) -> None:
        fixture = SyntheticInternalInput(self.base)
        promotion_receipt = json.loads(
            (fixture.profile_root / bundle.QUALIFIED_PROMOTION_RECEIPT_FILE).read_text(
                encoding="utf-8"
            )
        )
        self.assertEqual(
            [
                row["artifact_role"]
                for row in promotion_receipt["embedded_artifacts"]["cache_seals"]
            ],
            ["accepted_final", "graph_baseline"],
        )
        prepared = bundle.prepare_product_bundle(
            fixture.root,
            self.base / "prepared",
            sidecar_verifier=_accept_synthetic_sidecar,
            qualified_profile_verifier=_accept_synthetic_profile,
            promotion_attestation_verifier=_accept_synthetic_attestation,
        )
        self.assertTrue(prepared.present)
        self.assertEqual(prepared.signing_exclusions, (bundle.SIDECAR_FILE,))
        self.assertGreater(prepared.catalog_path.stat().st_size, 0)
        cli = self.base / "cli"
        studio = self.base / "studio"
        cli.mkdir()
        studio.mkdir()
        cli_bundle = bundle.stage_product_bundle(
            prepared,
            cli,
            sidecar_verifier=_accept_synthetic_sidecar,
            qualified_profile_verifier=_accept_synthetic_profile,
            promotion_attestation_verifier=_accept_synthetic_attestation,
        )
        studio_bundle = bundle.stage_product_bundle(
            prepared,
            studio,
            sidecar_verifier=_accept_synthetic_sidecar,
            qualified_profile_verifier=_accept_synthetic_profile,
            promotion_attestation_verifier=_accept_synthetic_attestation,
        )
        assert cli_bundle is not None and studio_bundle is not None
        self.assertEqual(_snapshot(cli_bundle), _snapshot(studio_bundle))
        verified = bundle.verify_staged_bundle(
            cli_bundle,
            sidecar_verifier=_accept_synthetic_sidecar,
            qualified_profile_verifier=_accept_synthetic_profile,
            promotion_attestation_verifier=_accept_synthetic_attestation,
        )
        self.assertEqual(len(verified.expected_files), 49)

    def test_legacy_v1_is_explicit_local_build_bridge_only(self) -> None:
        fixture = SyntheticInternalInput(self.base)
        descriptor = copy.deepcopy(fixture.descriptor)
        descriptor["schema_version"] = 1
        descriptor.pop("full_tree_verifications")
        descriptor["catalog"]["profiles"] = descriptor["catalog"]["profiles"][:1]
        (fixture.root / bundle.INTERNAL_INPUT_DESCRIPTOR_FILE).write_bytes(
            bundle._canonical_pretty(descriptor)
        )
        shutil.rmtree(fixture.current_profile_root)
        shutil.rmtree(fixture.root / "verification")

        verification_args = {
            "sidecar_verifier": _accept_synthetic_sidecar,
            "qualified_profile_verifier": _accept_synthetic_profile,
            "promotion_attestation_verifier": _accept_synthetic_attestation,
        }
        with self.assertRaisesRegex(bundle.BundleError, "pinned local build asset"):
            bundle.verify_internal_input(fixture.root, **verification_args)

        prepared = bundle.prepare_product_bundle(
            fixture.root,
            self.base / "legacy-prepared",
            allow_legacy_internal_input_v1=True,
            **verification_args,
        )
        self.assertTrue(prepared.legacy_internal_input_v1)
        host = self.base / "legacy-host"
        host.mkdir()
        staged = bundle.stage_product_bundle(prepared, host, **verification_args)
        assert staged is not None
        with self.assertRaisesRegex(bundle.BundleError, "pinned local build asset"):
            bundle.verify_staged_bundle(staged, **verification_args)
        verified = bundle.verify_staged_bundle(
            staged,
            allow_legacy_internal_input_v1=True,
            **verification_args,
        )
        self.assertEqual(verified.internal_input_schema_version, 1)

    def test_product_gate_requires_exactly_both_supported_profile_targets(self) -> None:
        for mutation, expected in (
            ("missing", "missing=.*24878692"),
            ("extra", "extra=.*30000000"),
        ):
            with self.subTest(mutation=mutation):
                case = self.base / f"target-set-{mutation}"
                case.mkdir()
                fixture = SyntheticInternalInput(case)
                descriptor_path = fixture.root / bundle.INTERNAL_INPUT_DESCRIPTOR_FILE
                descriptor = json.loads(descriptor_path.read_text(encoding="utf-8"))
                if mutation == "missing":
                    descriptor["catalog"]["profiles"].pop()
                    descriptor["full_tree_verifications"].pop()
                else:
                    extra = copy.deepcopy(descriptor["catalog"]["profiles"][-1])
                    extra["manifest_relative_path"] = (
                        "profiles/build-30000000/compiler-profile.json"
                    )
                    extra["profile_sha256"] = "ef" * 32
                    extra["target"]["target"]["steam_build_id"] = 30_000_000
                    extra["target"]["target"]["depot_manifest_gid"] = 999
                    extra["target"]["pe_codeview"] = {
                        "guid": "eeeeeeee-eeee-eeee-eeee-eeeeeeeeeeee",
                        "age": 1,
                    }
                    descriptor["catalog"]["profiles"].append(extra)
                descriptor_path.write_bytes(bundle._canonical_pretty(descriptor))
                with self.assertRaisesRegex(bundle.BundleError, expected):
                    bundle.verify_internal_input(
                        fixture.root,
                        sidecar_verifier=_accept_synthetic_sidecar,
                        qualified_profile_verifier=_accept_synthetic_profile,
                        promotion_attestation_verifier=_accept_synthetic_attestation,
                    )

    def test_full_tree_receipts_are_one_per_profile_and_bind_every_authority(self) -> None:
        def mutate_receipt(
            fixture: SyntheticInternalInput,
            profile_index: int,
            mutation: callable,
        ) -> None:
            descriptor_path = fixture.root / bundle.INTERNAL_INPUT_DESCRIPTOR_FILE
            descriptor = json.loads(descriptor_path.read_text(encoding="utf-8"))
            entry = descriptor["full_tree_verifications"][profile_index]
            receipt_path = fixture.root.joinpath(
                *bundle.PurePosixPath(entry["relative_path"]).parts
            )
            receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
            mutation(receipt)
            receipt_bytes = bundle._canonical_pretty(receipt)
            receipt_path.write_bytes(receipt_bytes)
            entry.update(_seal(receipt_bytes))
            descriptor_path.write_bytes(bundle._canonical_pretty(descriptor))

        manipulations = (
            (
                "profile",
                lambda value: value["authority"].__setitem__(
                    "qualified_profile_sha256", "ef" * 32
                ),
                "different qualified profile",
            ),
            (
                "sidecar",
                lambda value: value["authority"]["sidecar"].__setitem__(
                    "sha256", "ef" * 32
                ),
                "different final sidecar",
            ),
            (
                "shipping",
                lambda value: value["authority"]["shipping"].__setitem__(
                    "sha256", "ef" * 32
                ),
                "Shipping authority differs",
            ),
            (
                "binds",
                lambda value: value["authority"]["binds"].__setitem__(
                    "sha256", "ef" * 32
                ),
                "Binds authority differs",
            ),
            (
                "source-coverage",
                lambda value: value["frozen_source"]["operations"].__setitem__(
                    "add", 1
                ),
                "edit-only base module universe",
            ),
            (
                "embedded-reference",
                lambda value: value["embedded_reference"].__setitem__(
                    "sha256", "0" * 64
                ),
                "must not be the zero digest",
            ),
            (
                "candidate-module-count",
                lambda value: value["standalone_candidate"].__setitem__(
                    "module_count", 4
                ),
                "candidate module count differs",
            ),
            (
                "semantic-diff",
                lambda value: value["bytediff"].update(
                    {"semantic": 1, "aligned_functions": 11}
                ),
                "zero semantic and alignment-loss",
            ),
            (
                "alignment-loss",
                lambda value: value["bytediff"].__setitem__("alignment_loss", 1),
                "zero semantic and alignment-loss",
            ),
            (
                "whole-cache",
                lambda value: value["whole_cache_semantics_v1"].__setitem__(
                    "exact_struct_equality", False
                ),
                "must be true",
            ),
        )
        for name, mutation, expected in manipulations:
            with self.subTest(mutation=name):
                case = self.base / f"full-tree-{name}"
                case.mkdir()
                fixture = SyntheticInternalInput(case)
                mutate_receipt(fixture, 1, mutation)
                with self.assertRaisesRegex(bundle.BundleError, expected):
                    bundle.verify_internal_input(
                        fixture.root,
                        sidecar_verifier=_accept_synthetic_sidecar,
                        qualified_profile_verifier=_accept_synthetic_profile,
                        promotion_attestation_verifier=_accept_synthetic_attestation,
                    )

        for mutation in ("missing", "extra", "swapped"):
            with self.subTest(receipt_set=mutation):
                case = self.base / f"full-tree-set-{mutation}"
                case.mkdir()
                fixture = SyntheticInternalInput(case)
                descriptor_path = fixture.root / bundle.INTERNAL_INPUT_DESCRIPTOR_FILE
                descriptor = json.loads(descriptor_path.read_text(encoding="utf-8"))
                entries = descriptor["full_tree_verifications"]
                if mutation == "missing":
                    entries.pop()
                elif mutation == "extra":
                    entries.append(copy.deepcopy(entries[-1]))
                else:
                    entries.reverse()
                descriptor_path.write_bytes(bundle._canonical_pretty(descriptor))
                with self.assertRaisesRegex(
                    bundle.BundleError,
                    "exactly one receipt|different catalog profile",
                ):
                    bundle.verify_internal_input(
                        fixture.root,
                        sidecar_verifier=_accept_synthetic_sidecar,
                        qualified_profile_verifier=_accept_synthetic_profile,
                        promotion_attestation_verifier=_accept_synthetic_attestation,
                    )

    def test_internal_package_is_compressed_deterministic_and_reusable(self) -> None:
        fixture = SyntheticInternalInput(self.base)
        outputs = []
        descriptors = []
        for name in ("first", "second"):
            directory = self.base / name
            directory.mkdir()
            archive = directory / bundle.INTERNAL_PACKAGE_ARCHIVE_FILE
            descriptor_path = directory / bundle.INTERNAL_PACKAGE_DESCRIPTOR_FILE
            descriptor = bundle.pack_internal_package_archive(
                fixture.root,
                archive,
                descriptor_path,
                sidecar_verifier=_accept_synthetic_sidecar,
                qualified_profile_verifier=_accept_synthetic_profile,
                promotion_attestation_verifier=_accept_synthetic_attestation,
            )
            outputs.append(archive)
            descriptors.append((descriptor_path, descriptor))

        self.assertEqual(outputs[0].read_bytes(), outputs[1].read_bytes())
        self.assertEqual(descriptors[0][1], descriptors[1][1])
        self.assertEqual(descriptors[0][1].compression, "deflate-9")
        with zipfile.ZipFile(outputs[0], "r") as archive:
            self.assertTrue(archive.infolist())
            self.assertTrue(
                all(
                    info.compress_type == zipfile.ZIP_DEFLATED
                    for info in archive.infolist()
                )
            )

        extracted = self.base / "internal-extracted"
        bundle.extract_internal_package_archive(
            outputs[0],
            descriptors[0][0],
            extracted,
            sidecar_verifier=_accept_synthetic_sidecar,
            qualified_profile_verifier=_accept_synthetic_profile,
            promotion_attestation_verifier=_accept_synthetic_attestation,
        )
        self.assertEqual(_snapshot(extracted), _snapshot(fixture.root))
        self.assertEqual(
            bundle.materialize_internal_package(
                outputs[0],
                descriptors[0][0],
                extracted,
                sidecar_verifier=_accept_synthetic_sidecar,
                qualified_profile_verifier=_accept_synthetic_profile,
                promotion_attestation_verifier=_accept_synthetic_attestation,
            ),
            extracted,
        )

    def test_internal_package_rejects_archive_or_descriptor_tamper(self) -> None:
        fixture = SyntheticInternalInput(self.base)
        package_dir = self.base / "internal"
        package_dir.mkdir()
        archive = package_dir / bundle.INTERNAL_PACKAGE_ARCHIVE_FILE
        descriptor_path = package_dir / bundle.INTERNAL_PACKAGE_DESCRIPTOR_FILE
        bundle.pack_internal_package_archive(
            fixture.root,
            archive,
            descriptor_path,
            sidecar_verifier=_accept_synthetic_sidecar,
            qualified_profile_verifier=_accept_synthetic_profile,
            promotion_attestation_verifier=_accept_synthetic_attestation,
        )
        original_archive = archive.read_bytes()
        archive.write_bytes(original_archive[:-1] + bytes([original_archive[-1] ^ 1]))
        with self.assertRaisesRegex(bundle.BundleError, "pinned length/SHA-256"):
            bundle.materialize_internal_package(
                archive,
                descriptor_path,
                self.base / "tampered-archive-output",
                sidecar_verifier=_accept_synthetic_sidecar,
                qualified_profile_verifier=_accept_synthetic_profile,
                promotion_attestation_verifier=_accept_synthetic_attestation,
            )
        archive.write_bytes(original_archive)

        descriptor = json.loads(descriptor_path.read_text(encoding="utf-8"))
        descriptor["compression"] = "stored"
        descriptor_path.write_bytes(bundle._canonical_pretty(descriptor))
        with self.assertRaisesRegex(bundle.BundleError, "compression"):
            bundle.materialize_internal_package(
                archive,
                descriptor_path,
                self.base / "tampered-descriptor-output",
                sidecar_verifier=_accept_synthetic_sidecar,
                qualified_profile_verifier=_accept_synthetic_profile,
                promotion_attestation_verifier=_accept_synthetic_attestation,
            )

    def test_internal_package_pin_and_typed_verification_precede_publication(
        self,
    ) -> None:
        fixture = SyntheticInternalInput(self.base)
        package_dir = self.base / "pinned-package"
        package_dir.mkdir()
        archive = package_dir / bundle.INTERNAL_PACKAGE_ARCHIVE_FILE
        descriptor = package_dir / bundle.INTERNAL_PACKAGE_DESCRIPTOR_FILE
        bundle.pack_internal_package_archive(
            fixture.root,
            archive,
            descriptor,
            sidecar_verifier=_accept_synthetic_sidecar,
            qualified_profile_verifier=_accept_synthetic_profile,
            promotion_attestation_verifier=_accept_synthetic_attestation,
        )

        events: list[str] = []
        real_pin = bundle._pin_windows_file_path
        real_seal = bundle._streaming_file_seal
        real_raw_validator = bundle._validate_canonical_internal_package_archive
        real_zip_file = zipfile.ZipFile

        @contextmanager
        def tracked_pin(path: Path, label: str, **kwargs: object):
            events.append("pin-enter")
            with real_pin(path, label, **kwargs):
                yield
            events.append("pin-exit")

        def tracked_seal(*args: object, **kwargs: object) -> bundle.Seal:
            events.append("seal")
            return real_seal(*args, **kwargs)

        def tracked_raw_validator(path: Path) -> tuple[str, ...]:
            events.append("raw-parse")
            return real_raw_validator(path)

        def tracked_zip_file(*args: object, **kwargs: object) -> zipfile.ZipFile:
            events.append("zip-parse")
            return real_zip_file(*args, **kwargs)

        output = self.base / "pinned-package-output"
        with (
            mock.patch.object(bundle, "_pin_windows_file_path", tracked_pin),
            mock.patch.object(bundle, "_streaming_file_seal", tracked_seal),
            mock.patch.object(
                bundle,
                "_validate_canonical_internal_package_archive",
                tracked_raw_validator,
            ),
            mock.patch.object(bundle.zipfile, "ZipFile", side_effect=tracked_zip_file),
        ):
            bundle.extract_internal_package_archive(
                archive,
                descriptor,
                output,
                sidecar_verifier=_accept_synthetic_sidecar,
                qualified_profile_verifier=_accept_synthetic_profile,
                promotion_attestation_verifier=_accept_synthetic_attestation,
            )
        self.assertEqual(_snapshot(output), _snapshot(fixture.root))
        self.assertLess(events.index("pin-enter"), events.index("seal"))
        self.assertLess(events.index("seal"), events.index("raw-parse"))
        self.assertLess(events.index("raw-parse"), events.index("zip-parse"))
        self.assertLess(events.index("zip-parse"), events.index("pin-exit"))

        def refuse_profile(
            _root: Path, _profile_sha256: str
        ) -> bundle.QualifiedProfileTreeAuthority:
            raise bundle.BundleError("typed verifier refused internal profile")

        refused_output = self.base / "typed-refusal-output"
        with self.assertRaisesRegex(bundle.BundleError, "typed verifier refused"):
            bundle.extract_internal_package_archive(
                archive,
                descriptor,
                refused_output,
                sidecar_verifier=_accept_synthetic_sidecar,
                qualified_profile_verifier=refuse_profile,
                promotion_attestation_verifier=_accept_synthetic_attestation,
            )
        self.assertFalse(refused_output.exists())
        self.assertEqual(
            list(self.base.glob(f".{refused_output.name}.*.tmp")), []
        )

    def test_internal_package_rejects_unsafe_or_ambiguous_zip_without_publication(
        self,
    ) -> None:
        def write_descriptor(archive: Path, count: int) -> Path:
            bytes_ = archive.read_bytes()
            descriptor = archive.parent / bundle.INTERNAL_PACKAGE_DESCRIPTOR_FILE
            descriptor.write_bytes(
                bundle._canonical_pretty(
                    {
                        "schema": bundle.INTERNAL_PACKAGE_DESCRIPTOR_SCHEMA,
                        "schema_version": bundle.INTERNAL_PACKAGE_DESCRIPTOR_SCHEMA_VERSION,
                        "asset": bundle.INTERNAL_PACKAGE_ARCHIVE_FILE,
                        "archive": _seal(bytes_),
                        "compression": "deflate-9",
                        "internal_input": {
                            "catalog_sha256": "ab" * 32,
                            "file_count": count,
                        },
                    }
                )
            )
            return descriptor

        def write_zip(case_dir: Path, infos: list[zipfile.ZipInfo]) -> Path:
            archive = case_dir / bundle.INTERNAL_PACKAGE_ARCHIVE_FILE
            with zipfile.ZipFile(archive, "x", allowZip64=False) as stream:
                for info in infos:
                    stream.writestr(info, b"{}")
            return archive

        def deflated_info(name: str) -> zipfile.ZipInfo:
            return bundle._canonical_archive_info(
                name, compression=zipfile.ZIP_DEFLATED
            )

        zip_shapes: dict[str, list[zipfile.ZipInfo]] = {
            "traversal": [deflated_info("../escape")],
            "backslash": [deflated_info(r"profiles\escape")],
            "absolute": [deflated_info("/absolute")],
            "case-alias": [
                deflated_info(bundle.INTERNAL_INPUT_DESCRIPTOR_FILE),
                deflated_info(bundle.INTERNAL_INPUT_DESCRIPTOR_FILE.upper()),
            ],
            "stored-member": [
                bundle._canonical_archive_info(bundle.INTERNAL_INPUT_DESCRIPTOR_FILE)
            ],
            "unsorted": [deflated_info("z.json"), deflated_info("a.json")],
        }
        symlink = deflated_info(bundle.INTERNAL_INPUT_DESCRIPTOR_FILE)
        symlink.external_attr = (stat.S_IFLNK | 0o777) << 16
        zip_shapes["symlink"] = [symlink]

        for name, infos in zip_shapes.items():
            with self.subTest(shape=name):
                case_dir = self.base / f"unsafe-{name}"
                case_dir.mkdir()
                archive = write_zip(case_dir, infos)
                descriptor = write_descriptor(archive, len(infos))
                output = case_dir / "output"
                with self.assertRaises(bundle.BundleError):
                    bundle.extract_internal_package_archive(
                        archive,
                        descriptor,
                        output,
                        sidecar_verifier=_accept_synthetic_sidecar,
                        qualified_profile_verifier=_accept_synthetic_profile,
                        promotion_attestation_verifier=_accept_synthetic_attestation,
                    )
                self.assertFalse(output.exists())
                self.assertEqual(list(case_dir.glob(".output.*.tmp")), [])

        canonical_dir = self.base / "raw-canonical"
        canonical_dir.mkdir()
        canonical = write_zip(
            canonical_dir,
            [deflated_info(bundle.INTERNAL_INPUT_DESCRIPTOR_FILE)],
        ).read_bytes()
        central_offset = canonical.index(b"PK\x01\x02")
        local_name_mismatch = bytearray(canonical)
        local_name_mismatch[30] = ord("x")
        descriptor_flags = bytearray(canonical)
        struct.pack_into("<H", descriptor_flags, 6, 0x0008)
        struct.pack_into("<H", descriptor_flags, central_offset + 8, 0x0008)
        local_method_mismatch = bytearray(canonical)
        struct.pack_into("<H", local_method_mismatch, 8, zipfile.ZIP_STORED)
        raw_mutations = {
            "trailing": canonical + b"foreign-trailing-bytes",
            "prefix": b"foreign-prefix" + canonical,
            "local-name": bytes(local_name_mismatch),
            "descriptor-flags": bytes(descriptor_flags),
            "local-method": bytes(local_method_mismatch),
        }
        for name, bytes_ in raw_mutations.items():
            with self.subTest(raw=name):
                case_dir = self.base / f"raw-{name}"
                case_dir.mkdir()
                archive = case_dir / bundle.INTERNAL_PACKAGE_ARCHIVE_FILE
                archive.write_bytes(bytes_)
                descriptor = write_descriptor(archive, 1)
                output = case_dir / "output"
                with self.assertRaises(bundle.BundleError):
                    bundle.extract_internal_package_archive(
                        archive,
                        descriptor,
                        output,
                        sidecar_verifier=_accept_synthetic_sidecar,
                        qualified_profile_verifier=_accept_synthetic_profile,
                        promotion_attestation_verifier=_accept_synthetic_attestation,
                    )
                self.assertFalse(output.exists())
                self.assertEqual(list(case_dir.glob(".output.*.tmp")), [])

    def test_absent_input_removes_stale_work_and_host_compiler_directories(
        self,
    ) -> None:
        work = self.base / "prepared"
        (work / "compiler").mkdir(parents=True)
        (work / "compiler" / bundle.SIDECAR_FILE).write_bytes(b"stale")
        prepared = bundle.prepare_product_bundle(None, work)
        self.assertFalse(prepared.present)
        self.assertEqual(prepared.catalog_path.read_bytes(), b"")
        self.assertFalse((work / "compiler").exists())
        host = self.base / "host"
        (host / "compiler").mkdir(parents=True)
        (host / "compiler" / bundle.SIDECAR_FILE).write_bytes(b"stale")
        self.assertIsNone(bundle.stage_product_bundle(prepared, host))
        self.assertFalse((host / "compiler").exists())

    def test_blob_notice_corruption_target_drift_and_extra_files_fail_closed(
        self,
    ) -> None:
        for mutation, expected in (
            ("blob", "pinned length/SHA-256"),
            ("notice", "pinned length/SHA-256"),
            ("promotion-identity", "pinned length/SHA-256"),
            ("promotion-provenance", "pinned length/SHA-256"),
            ("promotion-receipt", "promotion receipt canonical seal differs"),
            ("promotion-artifact", "promotion authority differs"),
            ("target", "product compiler target set differs"),
            ("target-width", "1..4294967295"),
            ("codeview", "CodeView GUID is invalid"),
            ("extra", "unknown="),
        ):
            with self.subTest(mutation=mutation):
                case = self.base / mutation
                case.mkdir()
                fixture = SyntheticInternalInput(case)
                if mutation == "blob":
                    path = (
                        fixture.profile_root
                        / "payload/00-ordered_engine_properties.bin"
                    )
                    path.write_bytes(b"corrupt")
                elif mutation == "notice":
                    (fixture.root / bundle.REQUIRED_NOTICES[0]).write_bytes(b"corrupt")
                elif mutation == "promotion-identity":
                    fixture.root.joinpath(
                        *bundle.PurePosixPath(bundle.PROMOTION_IDENTITY_FILE).parts
                    ).write_bytes(b"corrupt")
                elif mutation == "promotion-provenance":
                    fixture.root.joinpath(
                        *bundle.PurePosixPath(bundle.PROMOTION_PROVENANCE_FILE).parts
                    ).write_bytes(b"corrupt")
                elif mutation == "promotion-receipt":
                    path = (
                        fixture.profile_root / bundle.QUALIFIED_PROMOTION_RECEIPT_FILE
                    )
                    receipt = json.loads(path.read_text(encoding="utf-8"))
                    receipt["capture_stream_sha256"] = "ff" * 32
                    path.write_bytes(bundle._canonical_pretty(receipt))
                elif mutation == "promotion-artifact":
                    path = (
                        fixture.profile_root
                        / bundle.EMBEDDED_QUALIFICATION_ARTIFACT_MANIFEST_FILE
                    )
                    path.write_bytes(path.read_bytes() + b" ")
                elif mutation in ("target", "target-width", "codeview"):
                    target = {
                        "target": copy.deepcopy(fixture.target),
                        "pe_codeview": fixture.codeview,
                    }
                    if mutation == "target":
                        target["target"]["steam_build_id"] += 1
                    elif mutation == "target-width":
                        target["target"]["steam_app_id"] = 1 << 32
                    else:
                        target["pe_codeview"] = {"guid": "z" * 36, "age": 1}
                    fixture._write_descriptor(catalog_target=target)
                else:
                    (fixture.root / "old-sidecar.exe").write_bytes(b"stale")
                with self.assertRaisesRegex(bundle.BundleError, expected):
                    bundle.verify_internal_input(
                        fixture.root,
                        sidecar_verifier=_accept_synthetic_sidecar,
                        qualified_profile_verifier=_accept_synthetic_profile,
                        promotion_attestation_verifier=_accept_synthetic_attestation,
                    )

        legacy_identity = b"legacy signed identity"
        legacy_provenance = bundle._canonical_pretty(
            {
                "schema": "gore.as.promotion-candidate-provenance",
                "schema_version": 2,
                "repository": "dh0er/gore",
                "commit": "12" * 20,
                "workflow_sha": "56" * 20,
                "workflow_run_id": 1,
                "workflow_run_attempt": 1,
                "claim_tag": "gore-as-candidate-build-24539464-v1",
                "promotion_tag": "gore-as-qualified-build-24539464-v1",
                "signed_identity": _seal(legacy_identity),
            }
        )
        with self.assertRaisesRegex(bundle.BundleError, "schema/version is unsupported"):
            bundle._parse_promotion_provenance(
                legacy_provenance,
                bundle.Seal(len(legacy_identity), _sha256(legacy_identity)),
            )

    def test_staged_catalog_byte_drift_fails_closed(self) -> None:
        fixture = SyntheticInternalInput(self.base)
        prepared = bundle.prepare_product_bundle(
            fixture.root,
            self.base / "work",
            sidecar_verifier=_accept_synthetic_sidecar,
            qualified_profile_verifier=_accept_synthetic_profile,
            promotion_attestation_verifier=_accept_synthetic_attestation,
        )
        assert prepared.bundle_root is not None
        catalog = prepared.bundle_root / bundle.CATALOG_FILE
        catalog.write_bytes(catalog.read_bytes() + b" ")
        with self.assertRaisesRegex(bundle.BundleError, "catalog differs"):
            bundle.verify_staged_bundle(
                prepared.bundle_root,
                sidecar_verifier=_accept_synthetic_sidecar,
                qualified_profile_verifier=_accept_synthetic_profile,
                promotion_attestation_verifier=_accept_synthetic_attestation,
            )

    def test_retired_273408_byte_sidecar_is_rejected_even_when_self_pinned(
        self,
    ) -> None:
        fixture = SyntheticInternalInput(
            self.base,
            sidecar_bytes=b"x" * next(iter(bundle.STALE_SIDECAR_BYTE_LENGTHS)),
        )
        with self.assertRaisesRegex(bundle.BundleError, "retired stale sidecar"):
            bundle.verify_internal_input(
                fixture.root,
                sidecar_verifier=_accept_synthetic_sidecar,
                qualified_profile_verifier=_accept_synthetic_profile,
                promotion_attestation_verifier=_accept_synthetic_attestation,
            )

    def test_qualification_must_bind_the_exact_signed_sidecar_protocol(self) -> None:
        fixture = SyntheticInternalInput(self.base)
        parity = fixture.profile_root / "payload/14-diagnostic_parity.bin"
        payload = json.loads(parity.read_text(encoding="utf-8"))
        payload["standalone_compiler"]["sha256"] = "cd" * 32
        changed = bundle._canonical_pretty(payload)
        parity.write_bytes(changed)
        fixture.profile["qualification"]["diagnostic_parity"] = {
            "path": "payload/14-diagnostic_parity.bin",
            **_seal(changed),
        }
        fixture._write_descriptor()
        with self.assertRaisesRegex(bundle.BundleError, "different signed sidecar"):
            bundle.verify_internal_input(
                fixture.root,
                sidecar_verifier=_accept_synthetic_sidecar,
                qualified_profile_verifier=_accept_synthetic_profile,
                promotion_attestation_verifier=_accept_synthetic_attestation,
            )

    def test_product_internal_input_rejects_legacy_v1_protocol(self) -> None:
        fixture = SyntheticInternalInput(self.base)
        descriptor_path = fixture.root / bundle.INTERNAL_INPUT_DESCRIPTOR_FILE
        descriptor = json.loads(descriptor_path.read_text(encoding="utf-8"))
        descriptor["catalog"]["sidecar"]["protocol"]["request_version"] = (
            bundle.LEGACY_SMOKE_REQUEST_VERSION
        )
        descriptor_path.write_bytes(bundle._canonical_pretty(descriptor))
        with self.assertRaisesRegex(bundle.BundleError, "legacy-smoke-only"):
            bundle.verify_internal_input(
                fixture.root,
                sidecar_verifier=_accept_synthetic_sidecar,
                qualified_profile_verifier=_accept_synthetic_profile,
                promotion_attestation_verifier=_accept_synthetic_attestation,
            )

    def test_internal_input_rejects_hard_links(self) -> None:
        fixture = SyntheticInternalInput(self.base)
        hardlink = fixture.root / "hardlink-notice"
        try:
            os.link(fixture.root / bundle.REQUIRED_NOTICES[0], hardlink)
        except OSError as error:
            self.skipTest(f"hard links unavailable: {error}")
        with self.assertRaisesRegex(bundle.BundleError, "single-link|hard-linked"):
            bundle.verify_internal_input(
                fixture.root,
                sidecar_verifier=_accept_synthetic_sidecar,
                qualified_profile_verifier=_accept_synthetic_profile,
                promotion_attestation_verifier=_accept_synthetic_attestation,
            )

    def test_record_step_copies_already_signed_qualified_bytes_into_new_input(
        self,
    ) -> None:
        fixture = SyntheticInternalInput(self.base)
        output = self.base / "recorded"
        verified_profiles: list[tuple[Path, str]] = []

        def verify_copied_profile(
            root: Path, profile_sha256: str
        ) -> bundle.QualifiedProfileTreeAuthority:
            self.assertTrue((root / "compiler-profile.json").is_file())
            self.assertTrue((root / bundle.QUALIFIED_PROMOTION_RECEIPT_FILE).is_file())
            verified_profiles.append((root, profile_sha256))
            return bundle._qualified_profile_tree_summary(root)

        verified = bundle.record_internal_input(
            fixture.root / bundle.SIDECAR_FILE,
            fixture.profile_roots,
            output,
            full_tree_receipts=fixture.full_tree_receipts,
            promotion_identity=fixture.root.joinpath(
                *bundle.PurePosixPath(bundle.PROMOTION_IDENTITY_FILE).parts
            ),
            promotion_provenance=fixture.root.joinpath(
                *bundle.PurePosixPath(bundle.PROMOTION_PROVENANCE_FILE).parts
            ),
            promotion_attestation=fixture.root.joinpath(
                *bundle.PurePosixPath(bundle.PROMOTION_ATTESTATION_FILE).parts
            ),
            expected_repository=fixture.promotion_repository,
            expected_commit=fixture.promotion_commit,
            sidecar_verifier=_accept_synthetic_sidecar,
            qualified_profile_verifier=verify_copied_profile,
            promotion_attestation_verifier=_accept_synthetic_attestation,
            notice_sources={
                name: fixture.root / name for name in bundle.REQUIRED_NOTICES
            },
        )
        self.assertEqual(verified.sidecar_name, bundle.SIDECAR_FILE)
        self.assertEqual(len(verified_profiles), 6)
        self.assertTrue(
            any(root.is_relative_to(output) for root, _ in verified_profiles)
        )
        self.assertTrue(
            any(not root.is_relative_to(output) for root, _ in verified_profiles)
        )
        self.assertEqual(
            {profile_sha256 for _, profile_sha256 in verified_profiles},
            {fixture.profile_sha256, fixture.current_profile_sha256},
        )
        self.assertEqual(
            (output / bundle.SIDECAR_FILE).read_bytes(), fixture.sidecar_bytes
        )
        with self.assertRaisesRegex(bundle.BundleError, "must not exist"):
            bundle.record_internal_input(
                fixture.root / bundle.SIDECAR_FILE,
                fixture.profile_roots,
                output,
                full_tree_receipts=fixture.full_tree_receipts,
                promotion_identity=fixture.root.joinpath(
                    *bundle.PurePosixPath(bundle.PROMOTION_IDENTITY_FILE).parts
                ),
                promotion_provenance=fixture.root.joinpath(
                    *bundle.PurePosixPath(bundle.PROMOTION_PROVENANCE_FILE).parts
                ),
                promotion_attestation=fixture.root.joinpath(
                    *bundle.PurePosixPath(bundle.PROMOTION_ATTESTATION_FILE).parts
                ),
                expected_repository=fixture.promotion_repository,
                expected_commit=fixture.promotion_commit,
                sidecar_verifier=_accept_synthetic_sidecar,
                qualified_profile_verifier=_accept_synthetic_profile,
                promotion_attestation_verifier=_accept_synthetic_attestation,
                notice_sources={
                    name: fixture.root / name for name in bundle.REQUIRED_NOTICES
                },
            )

    def test_record_step_never_publishes_a_partial_or_failed_input(self) -> None:
        fixture = SyntheticInternalInput(self.base)
        output = self.base / "failed-record"

        def refuse_profile(
            _root: Path, _profile_sha256: str
        ) -> bundle.QualifiedProfileTreeAuthority:
            raise bundle.BundleError("typed verifier refused recorded profile")

        with self.assertRaisesRegex(bundle.BundleError, "typed verifier refused"):
            bundle.record_internal_input(
                fixture.root / bundle.SIDECAR_FILE,
                fixture.profile_roots,
                output,
                full_tree_receipts=fixture.full_tree_receipts,
                promotion_identity=fixture.root.joinpath(
                    *bundle.PurePosixPath(bundle.PROMOTION_IDENTITY_FILE).parts
                ),
                promotion_provenance=fixture.root.joinpath(
                    *bundle.PurePosixPath(bundle.PROMOTION_PROVENANCE_FILE).parts
                ),
                promotion_attestation=fixture.root.joinpath(
                    *bundle.PurePosixPath(bundle.PROMOTION_ATTESTATION_FILE).parts
                ),
                expected_repository=fixture.promotion_repository,
                expected_commit=fixture.promotion_commit,
                sidecar_verifier=_accept_synthetic_sidecar,
                qualified_profile_verifier=refuse_profile,
                promotion_attestation_verifier=_accept_synthetic_attestation,
                notice_sources={
                    name: fixture.root / name for name in bundle.REQUIRED_NOTICES
                },
            )
        self.assertFalse(output.exists())
        self.assertEqual(list(self.base.glob(f".{output.name}.*.tmp")), [])

    def test_record_step_requires_one_full_tree_receipt_per_profile(self) -> None:
        fixture = SyntheticInternalInput(self.base)
        output = self.base / "missing-full-tree-receipt"
        with self.assertRaisesRegex(bundle.BundleError, "exactly one full-tree receipt"):
            bundle.record_internal_input(
                fixture.root / bundle.SIDECAR_FILE,
                fixture.profile_roots,
                output,
                full_tree_receipts=fixture.full_tree_receipts[:1],
                promotion_identity=fixture.root.joinpath(
                    *bundle.PurePosixPath(bundle.PROMOTION_IDENTITY_FILE).parts
                ),
                promotion_provenance=fixture.root.joinpath(
                    *bundle.PurePosixPath(bundle.PROMOTION_PROVENANCE_FILE).parts
                ),
                promotion_attestation=fixture.root.joinpath(
                    *bundle.PurePosixPath(bundle.PROMOTION_ATTESTATION_FILE).parts
                ),
                expected_repository=fixture.promotion_repository,
                expected_commit=fixture.promotion_commit,
                sidecar_verifier=_accept_synthetic_sidecar,
                qualified_profile_verifier=_accept_synthetic_profile,
                promotion_attestation_verifier=_accept_synthetic_attestation,
                notice_sources={
                    name: fixture.root / name for name in bundle.REQUIRED_NOTICES
                },
            )
        self.assertFalse(output.exists())

    def test_record_step_requires_the_rust_typed_profile_verifier(self) -> None:
        fixture = SyntheticInternalInput(self.base)
        output = self.base / "recorded-without-typed-verifier"
        with self.assertRaisesRegex(bundle.BundleError, "Rust typed"):
            bundle.record_internal_input(
                fixture.root / bundle.SIDECAR_FILE,
                fixture.profile_roots,
                output,
                full_tree_receipts=fixture.full_tree_receipts,
                promotion_identity=fixture.root.joinpath(
                    *bundle.PurePosixPath(bundle.PROMOTION_IDENTITY_FILE).parts
                ),
                promotion_provenance=fixture.root.joinpath(
                    *bundle.PurePosixPath(bundle.PROMOTION_PROVENANCE_FILE).parts
                ),
                promotion_attestation=fixture.root.joinpath(
                    *bundle.PurePosixPath(bundle.PROMOTION_ATTESTATION_FILE).parts
                ),
                expected_repository=fixture.promotion_repository,
                expected_commit=fixture.promotion_commit,
                sidecar_verifier=_accept_synthetic_sidecar,
                notice_sources={
                    name: fixture.root / name for name in bundle.REQUIRED_NOTICES
                },
            )
        self.assertFalse(output.exists())
        with self.assertRaisesRegex(bundle.BundleError, "Rust typed"):
            bundle.verify_internal_input(
                fixture.root, sidecar_verifier=_accept_synthetic_sidecar
            )
        with self.assertRaisesRegex(bundle.BundleError, "Rust typed"):
            bundle.prepare_product_bundle(
                fixture.root,
                self.base / "unverified-prepare",
                sidecar_verifier=_accept_synthetic_sidecar,
            )

    def test_record_step_rejects_a_different_authorized_commit(self) -> None:
        fixture = SyntheticInternalInput(self.base)
        output = self.base / "wrong-promotion-authority"
        with self.assertRaisesRegex(bundle.BundleError, "authorized commit"):
            bundle.record_internal_input(
                fixture.root / bundle.SIDECAR_FILE,
                fixture.profile_roots,
                output,
                full_tree_receipts=fixture.full_tree_receipts,
                promotion_identity=fixture.root.joinpath(
                    *bundle.PurePosixPath(bundle.PROMOTION_IDENTITY_FILE).parts
                ),
                promotion_provenance=fixture.root.joinpath(
                    *bundle.PurePosixPath(bundle.PROMOTION_PROVENANCE_FILE).parts
                ),
                promotion_attestation=fixture.root.joinpath(
                    *bundle.PurePosixPath(bundle.PROMOTION_ATTESTATION_FILE).parts
                ),
                expected_repository=fixture.promotion_repository,
                expected_commit="34" * 20,
                sidecar_verifier=_accept_synthetic_sidecar,
                qualified_profile_verifier=_accept_synthetic_profile,
                promotion_attestation_verifier=_accept_synthetic_attestation,
                notice_sources={
                    name: fixture.root / name for name in bundle.REQUIRED_NOTICES
                },
            )
        self.assertFalse(output.exists())

    def test_rust_profile_verifier_response_is_exact_and_profile_bound(self) -> None:
        self.assertEqual(
            bundle._qualified_profile_tree_sha256(
                [("z.bin", bytes((0, 1, 2))), ("a.json", b"alpha")]
            ),
            "9792a24a47c6628dd81200843908c370b162b65a5caf7b1d45d175c5fca365a6",
        )
        fixture = SyntheticInternalInput(self.base)
        verifier = Path(sys.executable).resolve()
        verifier_bytes = verifier.read_bytes()
        verifier_seal = bundle.Seal(len(verifier_bytes), _sha256(verifier_bytes))
        tree = bundle._qualified_profile_tree_summary(fixture.profile_root)
        response = {
            "schema": "gore.as.qualified-profile-verification",
            "schema_version": 1,
            "qualified": True,
            "profile_sha256": fixture.profile_sha256,
            "manifest_sha256": tree.manifest_sha256,
            "promotion_receipt_sha256": tree.promotion_receipt_sha256,
            "tree_sha256": tree.tree_sha256,
            "file_count": tree.file_count,
        }
        completed = mock.Mock(
            returncode=0,
            stdout=json.dumps(response).encode("utf-8"),
            stderr=b"",
        )
        with mock.patch.object(bundle.subprocess, "run", return_value=completed) as run:
            bundle.verify_qualified_profile_with_executable(
                verifier, verifier_seal, fixture.profile_root, fixture.profile_sha256
            )
        self.assertEqual(
            run.call_args.args[0], [str(verifier), str(fixture.profile_root)]
        )

        response["profile_sha256"] = "cd" * 32
        completed.stdout = json.dumps(response).encode("utf-8")
        with (
            mock.patch.object(bundle.subprocess, "run", return_value=completed),
            self.assertRaisesRegex(
                bundle.BundleError, "different profile-tree authority"
            ),
        ):
            bundle.verify_qualified_profile_with_executable(
                verifier, verifier_seal, fixture.profile_root, fixture.profile_sha256
            )

        response["profile_sha256"] = fixture.profile_sha256
        completed.stdout = json.dumps(response).encode("utf-8")

        def swap_after_typed_reload(*_args, **_kwargs):
            (fixture.profile_root / "post-reload-swap.bin").write_bytes(b"forged")
            return completed

        with (
            mock.patch.object(
                bundle.subprocess, "run", side_effect=swap_after_typed_reload
            ),
            self.assertRaisesRegex(bundle.BundleError, "profile tree file set differs"),
        ):
            bundle.verify_qualified_profile_with_executable(
                verifier, verifier_seal, fixture.profile_root, fixture.profile_sha256
            )

    @unittest.skipUnless(os.name == "nt", "Authenticode verifier is Windows-only")
    def test_authenticode_verifier_passes_literal_path_out_of_band(self) -> None:
        sidecar = self.base / "a path with [literal] characters.exe"
        completed = mock.Mock(returncode=0, stdout="", stderr="")
        with (
            mock.patch.object(bundle.shutil, "which", side_effect=[r"C:\pwsh.exe"]),
            mock.patch.object(bundle.subprocess, "run", return_value=completed) as run,
        ):
            bundle._verify_authenticode_windows(sidecar)
        (command,) = run.call_args.args
        self.assertNotIn(str(sidecar), command)
        self.assertEqual(
            run.call_args.kwargs["env"]["GORE_AUTHENTICODE_VERIFY_LITERAL_PATH"],
            str(sidecar),
        )
        self.assertIn("$env:GORE_AUTHENTICODE_VERIFY_LITERAL_PATH", command[4])

    def test_promotion_capabilities_require_full_graph_v2(self) -> None:
        sidecar = self.base / bundle.SIDECAR_FILE
        capabilities = {
            "backend": "gore-as-standalone-compiler",
            "request_version": bundle.PRODUCTION_REQUEST_VERSION,
            "request_versions": [
                bundle.LEGACY_SMOKE_REQUEST_VERSION,
                bundle.PRODUCTION_REQUEST_VERSION,
            ],
            "response_version": bundle.PROTOCOL_RESPONSE_VERSION,
            "compile": {
                "available": True,
                "requires_qualified_profile": True,
                "requires_unreal_runtime": False,
                "requires_game_dll": False,
            },
        }
        completed = mock.Mock(
            returncode=0,
            stdout=bundle._canonical_pretty(capabilities),
            stderr=b"",
        )
        with mock.patch.object(bundle.subprocess, "run", return_value=completed):
            bundle._verify_production_capabilities(sidecar)
        capabilities["request_version"] = bundle.LEGACY_SMOKE_REQUEST_VERSION
        completed.stdout = bundle._canonical_pretty(capabilities)
        with (
            mock.patch.object(bundle.subprocess, "run", return_value=completed),
            self.assertRaisesRegex(bundle.BundleError, "FullGraph 2/1"),
        ):
            bundle._verify_production_capabilities(sidecar)


if __name__ == "__main__":
    unittest.main()
