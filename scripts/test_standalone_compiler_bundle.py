from __future__ import annotations

import copy
import hashlib
import json
import os
from pathlib import Path
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


class SyntheticReleaseInput:
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
        self.promotion_claim_tag = (
            f"{bundle.PROMOTION_CLAIM_TAG_PREFIX}{self.promotion_commit}"
        )
        self.promotion_tag = f"{bundle.PROMOTION_TAG_PREFIX}{self.promotion_commit}"
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
        self.blobs: dict[tuple[str, str], dict[str, object]] = {}
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
                payload = f"synthetic:{group}.{field}\n".encode()
            path = self.profile_root / Path(relative)
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(payload)
            self.blobs[(group, field)] = {"path": relative, **_seal(payload)}
        self.profile = self._profile()
        self._write_descriptor()

    def _profile(self) -> dict[str, object]:
        return {
            "schema": "gore.as.compiler-profile",
            "schema_version": 1,
            "target": self.target,
            "oracle": {"pe_codeview": self.codeview},
            "binds": {},
            "engine": {
                "ordered_engine_properties": self.blobs[
                    ("engine", "ordered_engine_properties")
                ],
                "registration_trace": self.blobs[("engine", "registration_trace")],
                "post_bind_snapshot": self.blobs[("engine", "post_bind_snapshot")],
            },
            "unreal_semantics": {
                "reflected_type_graph": self.blobs[
                    ("unreal_semantics", "reflected_type_graph")
                ]
            },
            "frontend": {
                "preprocessor_config": self.blobs[("frontend", "preprocessor_config")],
                "class_generator_config": self.blobs[
                    ("frontend", "class_generator_config")
                ],
                "compiler_options": self.blobs[("frontend", "compiler_options")],
            },
            "bytecode": {
                "opcode_table": self.blobs[("bytecode", "opcode_table")],
                "operand_schema": self.blobs[("bytecode", "operand_schema")],
                "codegen_probe_corpus": self.blobs[
                    ("bytecode", "codegen_probe_corpus")
                ],
                "expected_probe_results": self.blobs[
                    ("bytecode", "expected_probe_results")
                ],
            },
            "cache_writer": {
                "serializer_schema": self.blobs[("cache_writer", "serializer_schema")],
                "reference_table_order": self.blobs[
                    ("cache_writer", "reference_table_order")
                ],
                "normalized_oracle_corpus": self.blobs[
                    ("cache_writer", "normalized_oracle_corpus")
                ],
            },
            "qualification": {
                "required_probe_suite_version": "synthetic-v1",
                "diagnostic_parity": self.blobs[("qualification", "diagnostic_parity")],
                "semantic_parity": self.blobs[("qualification", "semantic_parity")],
                "qualified": True,
            },
            "profile_sha256": self.profile_sha256,
        }

    def _write_descriptor(
        self, *, catalog_target: dict[str, object] | None = None
    ) -> None:
        manifest = bundle._canonical_pretty(self.profile)
        self.manifest = self.profile_root / "compiler-profile.json"
        self.manifest.write_bytes(manifest)
        self._write_promotion_audits(manifest)
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
            "profiles": [
                {
                    "manifest_relative_path": "profiles/build-24539464/compiler-profile.json",
                    "manifest_byte_len": len(manifest),
                    "manifest_sha256": _sha256(manifest),
                    "profile_sha256": self.profile_sha256,
                    "target": catalog_target
                    or {"target": self.target, "pe_codeview": self.codeview},
                }
            ],
        }
        notices = {
            name: _seal((self.root / name).read_bytes())
            for name in bundle.REQUIRED_NOTICES
        }
        promotion = self._write_release_promotion()
        self.descriptor = {
            "schema": bundle.RELEASE_INPUT_SCHEMA,
            "schema_version": 1,
            "immutable": True,
            "qualified": True,
            "promotion": promotion,
            "catalog": catalog,
            "notices": notices,
        }
        (self.root / "release-input.json").write_bytes(
            bundle._canonical_pretty(self.descriptor)
        )

    def _write_release_promotion(self) -> dict[str, object]:
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
            "claim_tag": self.promotion_claim_tag,
            "promotion_tag": self.promotion_tag,
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
            "claim_tag": self.promotion_claim_tag,
            "promotion_tag": self.promotion_tag,
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

    def _artifact_manifest(self, backend: str, marker: int) -> bytes:
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
            "corpus_sha256": self._payload_digest("bytecode", "codegen_probe_corpus"),
            "backend": backend,
            "source_profile_sha256": "11" * 32,
            "source_target": self.target,
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

    def _payload_digest(self, group: str, field: str) -> str:
        path = self.profile_root / str(self.blobs[(group, field)]["path"])
        return json.loads(path.read_text(encoding="utf-8"))["canonical_sha256"]

    def _write_promotion_audits(self, manifest: bytes) -> None:
        embedded = self._artifact_manifest("embedded_game", 4)
        standalone = self._artifact_manifest("standalone", 5)
        embedded_path = (
            self.profile_root / bundle.EMBEDDED_QUALIFICATION_ARTIFACT_MANIFEST_FILE
        )
        standalone_path = (
            self.profile_root / bundle.STANDALONE_QUALIFICATION_ARTIFACT_MANIFEST_FILE
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
        for blob in self.blobs.values():
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
            "source_target": self.target,
            "source_materialization_receipt_sha256": "12" * 32,
            "capture_stream_sha256": "13" * 32,
            "static_support_manifest_sha256": "14" * 32,
            "standalone_compiler": self.sidecar_identity,
            "embedded_artifacts": embedded_summary,
            "standalone_artifacts": standalone_summary,
            "corpus_sha256": self._payload_digest("bytecode", "codegen_probe_corpus"),
            "expected_results_sha256": self._payload_digest(
                "bytecode", "expected_probe_results"
            ),
            "diagnostic_parity_sha256": self._payload_digest(
                "qualification", "diagnostic_parity"
            ),
            "semantic_parity_sha256": self._payload_digest(
                "qualification", "semantic_parity"
            ),
            "profile_sha256": self.profile_sha256,
            "files": files,
            "canonical_sha256": "0" * 64,
        }
        receipt["canonical_sha256"] = bundle._domain_json_sha256(
            b"gore-as-qualified-profile-promotion-v1\0",
            {key: value for key, value in receipt.items() if key != "canonical_sha256"},
            include_length=False,
        )
        (self.profile_root / bundle.QUALIFIED_PROMOTION_RECEIPT_FILE).write_bytes(
            bundle._canonical_pretty(receipt)
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
            claim_tag=f"{bundle.PROMOTION_CLAIM_TAG_PREFIX}{'12' * 20}",
            promotion_tag=f"{bundle.PROMOTION_TAG_PREFIX}{'12' * 20}",
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

    def test_archive_measurement_raw_parse_and_extraction_share_one_path_pin(
        self,
    ) -> None:
        fixture = SyntheticReleaseInput(self.base)
        archive = self.base / "pinned-archive.zip"
        descriptor = self.base / "pinned-archive.json"
        output = self.base / "pinned-archive-output"
        bundle.pack_release_input_archive(
            fixture.root,
            archive,
            descriptor,
            repository="dh0er/gore",
            tag="gore-as-pinned-archive-v1",
            sidecar_verifier=_accept_synthetic_sidecar,
            qualified_profile_verifier=_accept_synthetic_profile,
            promotion_attestation_verifier=_accept_synthetic_attestation,
        )
        pinned = False
        real_seal = bundle._streaming_file_seal
        real_raw = bundle._validate_canonical_release_archive
        real_zip = bundle.zipfile.ZipFile

        @bundle.contextmanager
        def tracked_pin(_path: Path, _label: str, *, require_single_link: bool = True):
            nonlocal pinned
            self.assertTrue(require_single_link)
            pinned = True
            try:
                yield
            finally:
                pinned = False

        def tracked_seal(path: Path, maximum: int, label: str) -> bundle.Seal:
            self.assertTrue(pinned)
            return real_seal(path, maximum, label)

        def tracked_raw(path: Path) -> tuple[str, ...]:
            self.assertTrue(pinned)
            return real_raw(path)

        def tracked_zip(*args: object, **kwargs: object) -> zipfile.ZipFile:
            self.assertTrue(pinned)
            return real_zip(*args, **kwargs)

        with (
            mock.patch.object(bundle, "_pin_windows_file_path", tracked_pin),
            mock.patch.object(bundle, "_streaming_file_seal", tracked_seal),
            mock.patch.object(
                bundle, "_validate_canonical_release_archive", tracked_raw
            ),
            mock.patch.object(bundle.zipfile, "ZipFile", tracked_zip),
        ):
            bundle.extract_release_input_archive(
                archive,
                descriptor,
                output,
                expected_repository="dh0er/gore",
                sidecar_verifier=_accept_synthetic_sidecar,
                qualified_profile_verifier=_accept_synthetic_profile,
                promotion_attestation_verifier=_accept_synthetic_attestation,
            )
        self.assertFalse(pinned)
        self.assertEqual(_snapshot(output), _snapshot(fixture.root))

    def test_synthetic_release_input_prepares_and_stages_byte_identical_cli_and_studio(
        self,
    ) -> None:
        fixture = SyntheticReleaseInput(self.base)
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
        self.assertEqual(len(verified.expected_files), 27)

    def test_release_input_archive_is_deterministic_pinned_and_round_trips(
        self,
    ) -> None:
        fixture = SyntheticReleaseInput(self.base)
        first_archive = self.base / "compiler-input-a.zip"
        first_descriptor = self.base / "compiler-input-a.json"
        second_archive = self.base / "compiler-input-b.zip"
        second_descriptor = self.base / "compiler-input-b.json"
        first = bundle.pack_release_input_archive(
            fixture.root,
            first_archive,
            first_descriptor,
            repository="dh0er/gore",
            tag="gore-as-standalone-compiler-build-24539464-v1",
            sidecar_verifier=_accept_synthetic_sidecar,
            qualified_profile_verifier=_accept_synthetic_profile,
            promotion_attestation_verifier=_accept_synthetic_attestation,
        )
        second = bundle.pack_release_input_archive(
            fixture.root,
            second_archive,
            second_descriptor,
            repository="dh0er/gore",
            tag="gore-as-standalone-compiler-build-24539464-v1",
            sidecar_verifier=_accept_synthetic_sidecar,
            qualified_profile_verifier=_accept_synthetic_profile,
            promotion_attestation_verifier=_accept_synthetic_attestation,
        )
        self.assertEqual(first.archive, second.archive)
        self.assertEqual(first_archive.read_bytes(), second_archive.read_bytes())
        self.assertEqual(first.file_count, len(_snapshot(fixture.root)))
        extracted = self.base / "extracted-input"
        with mock.patch.object(
            bundle.zipfile.ZipFile,
            "read",
            side_effect=AssertionError("extraction must stream members"),
        ):
            bundle.extract_release_input_archive(
                first_archive,
                first_descriptor,
                extracted,
                expected_repository="dh0er/gore",
                sidecar_verifier=_accept_synthetic_sidecar,
                qualified_profile_verifier=_accept_synthetic_profile,
                promotion_attestation_verifier=_accept_synthetic_attestation,
            )
        self.assertEqual(_snapshot(extracted), _snapshot(fixture.root))
        bundle.verify_release_input(
            extracted,
            sidecar_verifier=_accept_synthetic_sidecar,
            qualified_profile_verifier=_accept_synthetic_profile,
            promotion_attestation_verifier=_accept_synthetic_attestation,
        )
        with self.assertRaisesRegex(bundle.BundleError, "must not exist"):
            bundle.extract_release_input_archive(
                first_archive,
                first_descriptor,
                extracted,
                expected_repository="dh0er/gore",
                sidecar_verifier=_accept_synthetic_sidecar,
                qualified_profile_verifier=_accept_synthetic_profile,
                promotion_attestation_verifier=_accept_synthetic_attestation,
            )

    def test_release_archive_is_not_published_before_raw_validation(self) -> None:
        fixture = SyntheticReleaseInput(self.base)
        archive = self.base / "compiler-input.zip"
        descriptor = self.base / "compiler-input.json"
        with mock.patch.object(
            bundle,
            "_validate_canonical_release_archive",
            side_effect=bundle.BundleError("forced raw validation failure"),
        ):
            with self.assertRaisesRegex(
                bundle.BundleError, "forced raw validation failure"
            ):
                bundle.pack_release_input_archive(
                    fixture.root,
                    archive,
                    descriptor,
                    repository="dh0er/gore",
                    tag="gore-as-standalone-compiler-build-24539464-v1",
                    sidecar_verifier=_accept_synthetic_sidecar,
                    qualified_profile_verifier=_accept_synthetic_profile,
                    promotion_attestation_verifier=_accept_synthetic_attestation,
                )
        self.assertFalse(archive.exists())
        self.assertFalse(descriptor.exists())
        self.assertEqual(list(self.base.glob(f".{archive.name}.*.tmp")), [])

    def test_release_archive_and_descriptor_publish_as_one_no_clobber_pair(
        self,
    ) -> None:
        fixture = SyntheticReleaseInput(self.base)
        archive = self.base / "paired-release.zip"
        descriptor = self.base / "paired-release.json"
        real_link = bundle.os.link

        def lose_descriptor_race(source: object, destination: object) -> None:
            if Path(destination) == descriptor:
                descriptor.write_bytes(b"foreign descriptor")
                raise FileExistsError("simulated descriptor race")
            real_link(source, destination)

        with mock.patch.object(bundle.os, "link", side_effect=lose_descriptor_race):
            with self.assertRaisesRegex(
                bundle.BundleError, "descriptor output must not exist"
            ):
                bundle.pack_release_input_archive(
                    fixture.root,
                    archive,
                    descriptor,
                    repository="dh0er/gore",
                    tag="gore-as-pair-race-v1",
                    sidecar_verifier=_accept_synthetic_sidecar,
                    qualified_profile_verifier=_accept_synthetic_profile,
                    promotion_attestation_verifier=_accept_synthetic_attestation,
                )
        self.assertFalse(archive.exists())
        self.assertEqual(descriptor.read_bytes(), b"foreign descriptor")
        self.assertEqual(list(self.base.glob(f".{archive.name}.*.tmp")), [])
        self.assertEqual(list(self.base.glob(f".{descriptor.name}.*.tmp")), [])

    def test_extraction_runs_typed_verification_before_publication(self) -> None:
        fixture = SyntheticReleaseInput(self.base)
        archive = self.base / "typed-verification.zip"
        descriptor = self.base / "typed-verification.json"
        output = self.base / "typed-verification-output"
        bundle.pack_release_input_archive(
            fixture.root,
            archive,
            descriptor,
            repository="dh0er/gore",
            tag="gore-as-typed-verification-v1",
            sidecar_verifier=_accept_synthetic_sidecar,
            qualified_profile_verifier=_accept_synthetic_profile,
            promotion_attestation_verifier=_accept_synthetic_attestation,
        )

        def refuse_profile(
            _root: Path, _profile_sha256: str
        ) -> bundle.QualifiedProfileTreeAuthority:
            raise bundle.BundleError("typed verifier refused synthetic profile")

        with self.assertRaisesRegex(bundle.BundleError, "typed verifier refused"):
            bundle.extract_release_input_archive(
                archive,
                descriptor,
                output,
                expected_repository="dh0er/gore",
                sidecar_verifier=_accept_synthetic_sidecar,
                qualified_profile_verifier=refuse_profile,
                promotion_attestation_verifier=_accept_synthetic_attestation,
            )
        self.assertFalse(output.exists())
        self.assertEqual(list(self.base.glob(f".{output.name}.*.tmp")), [])

    def test_release_archive_extraction_rejects_unpinned_and_unsafe_zip_shapes(
        self,
    ) -> None:
        def descriptor_for(
            archive: Path, *, count: int, sha256: str | None = None
        ) -> Path:
            descriptor = self.base / f"{archive.stem}.json"
            bytes_ = archive.read_bytes()
            descriptor.write_bytes(
                bundle._canonical_pretty(
                    {
                        "schema": bundle.RELEASE_ASSET_DESCRIPTOR_SCHEMA,
                        "schema_version": bundle.RELEASE_ASSET_DESCRIPTOR_SCHEMA_VERSION,
                        "repository": "dh0er/gore",
                        "tag": "gore-as-standalone-compiler-build-24539464-v1",
                        "asset": archive.name,
                        "archive": {
                            "byte_len": len(bytes_),
                            "sha256": sha256 or _sha256(bytes_),
                        },
                        "release_input": {
                            "catalog_sha256": "ab" * 32,
                            "file_count": count,
                        },
                    }
                )
            )
            return descriptor

        cases: list[tuple[str, list[zipfile.ZipInfo]]] = []
        traversal = bundle._release_archive_info("../escape")
        cases.append(("traversal", [traversal]))
        backslash = bundle._release_archive_info(r"profiles\escape")
        cases.append(("backslash", [backslash]))
        absolute = bundle._release_archive_info("/absolute")
        cases.append(("absolute", [absolute]))
        symlink = bundle._release_archive_info("release-input.json")
        symlink.external_attr = (stat.S_IFLNK | 0o777) << 16
        cases.append(("symlink", [symlink]))
        first_alias = bundle._release_archive_info("release-input.json")
        second_alias = bundle._release_archive_info("RELEASE-INPUT.JSON")
        cases.append(("case-alias", [first_alias, second_alias]))
        compressed = bundle._release_archive_info("release-input.json")
        compressed.compress_type = zipfile.ZIP_DEFLATED
        cases.append(("compressed", [compressed]))

        for name, infos in cases:
            with self.subTest(name=name):
                archive = self.base / f"{name}.zip"
                with zipfile.ZipFile(archive, "x", allowZip64=True) as stream:
                    for info in infos:
                        stream.writestr(info, b"{}")
                descriptor = descriptor_for(archive, count=len(infos))
                output = self.base / f"{name}-output"
                with self.assertRaises(bundle.BundleError):
                    bundle.extract_release_input_archive(
                        archive,
                        descriptor,
                        output,
                        expected_repository="dh0er/gore",
                        sidecar_verifier=_accept_synthetic_sidecar,
                        qualified_profile_verifier=_accept_synthetic_profile,
                        promotion_attestation_verifier=_accept_synthetic_attestation,
                    )
                self.assertFalse(output.exists())

        archive = self.base / "unpinned.zip"
        with zipfile.ZipFile(archive, "x") as stream:
            stream.writestr(bundle._release_archive_info("release-input.json"), b"{}")
        descriptor = descriptor_for(archive, count=1, sha256="01" * 32)
        with self.assertRaisesRegex(bundle.BundleError, "pinned length/SHA-256"):
            bundle.extract_release_input_archive(
                archive,
                descriptor,
                self.base / "unpinned-output",
                expected_repository="dh0er/gore",
                sidecar_verifier=_accept_synthetic_sidecar,
                qualified_profile_verifier=_accept_synthetic_profile,
                promotion_attestation_verifier=_accept_synthetic_attestation,
            )

    def test_release_archive_rejects_raw_header_ambiguity_and_trailing_bytes(
        self,
    ) -> None:
        def descriptor_for(archive: Path, count: int) -> Path:
            bytes_ = archive.read_bytes()
            descriptor = archive.with_suffix(".json")
            descriptor.write_bytes(
                bundle._canonical_pretty(
                    {
                        "schema": bundle.RELEASE_ASSET_DESCRIPTOR_SCHEMA,
                        "schema_version": bundle.RELEASE_ASSET_DESCRIPTOR_SCHEMA_VERSION,
                        "repository": "dh0er/gore",
                        "tag": "gore-as-standalone-compiler-build-24539464-v1",
                        "asset": archive.name,
                        "archive": {
                            "byte_len": len(bytes_),
                            "sha256": _sha256(bytes_),
                        },
                        "release_input": {
                            "catalog_sha256": "ab" * 32,
                            "file_count": count,
                        },
                    }
                )
            )
            return descriptor

        canonical = self.base / "raw-canonical.zip"
        with zipfile.ZipFile(canonical, "x", allowZip64=False) as stream:
            stream.writestr(bundle._release_archive_info("release-input.json"), b"{}")
        canonical_bytes = canonical.read_bytes()
        central_offset = canonical_bytes.index(b"PK\x01\x02")

        local_name_mismatch = bytearray(canonical_bytes)
        local_name_mismatch[30] = ord("x")
        descriptor_flags = bytearray(canonical_bytes)
        struct.pack_into("<H", descriptor_flags, 6, 0x0008)
        struct.pack_into("<H", descriptor_flags, central_offset + 8, 0x0008)
        local_method_mismatch = bytearray(canonical_bytes)
        struct.pack_into("<H", local_method_mismatch, 8, zipfile.ZIP_DEFLATED)

        mutations = {
            "trailing": canonical_bytes + b"foreign-trailing-bytes",
            "prefix": b"foreign-prefix" + canonical_bytes,
            "local-name": bytes(local_name_mismatch),
            "data-descriptor-flags": bytes(descriptor_flags),
            "local-method": bytes(local_method_mismatch),
        }
        for name, bytes_ in mutations.items():
            with self.subTest(name=name):
                archive = self.base / f"raw-{name}.zip"
                archive.write_bytes(bytes_)
                descriptor = descriptor_for(archive, 1)
                output = self.base / f"raw-{name}-output"
                with self.assertRaises(bundle.BundleError):
                    bundle.extract_release_input_archive(
                        archive,
                        descriptor,
                        output,
                        expected_repository="dh0er/gore",
                        sidecar_verifier=_accept_synthetic_sidecar,
                        qualified_profile_verifier=_accept_synthetic_profile,
                        promotion_attestation_verifier=_accept_synthetic_attestation,
                    )
                self.assertFalse(output.exists())

        unsorted = self.base / "raw-unsorted.zip"
        with zipfile.ZipFile(unsorted, "x", allowZip64=False) as stream:
            stream.writestr(bundle._release_archive_info("z.json"), b"{}")
            stream.writestr(bundle._release_archive_info("a.json"), b"{}")
        descriptor = descriptor_for(unsorted, 2)
        with self.assertRaisesRegex(bundle.BundleError, "duplicated or unsorted"):
            bundle.extract_release_input_archive(
                unsorted,
                descriptor,
                self.base / "raw-unsorted-output",
                expected_repository="dh0er/gore",
                sidecar_verifier=_accept_synthetic_sidecar,
                qualified_profile_verifier=_accept_synthetic_profile,
                promotion_attestation_verifier=_accept_synthetic_attestation,
            )

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
            ("target", "target tuple differs"),
            ("target-width", "1..4294967295"),
            ("codeview", "CodeView GUID is invalid"),
            ("extra", "unknown="),
        ):
            with self.subTest(mutation=mutation):
                case = self.base / mutation
                case.mkdir()
                fixture = SyntheticReleaseInput(case)
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
                    bundle.verify_release_input(
                        fixture.root,
                        sidecar_verifier=_accept_synthetic_sidecar,
                        qualified_profile_verifier=_accept_synthetic_profile,
                        promotion_attestation_verifier=_accept_synthetic_attestation,
                    )

    def test_staged_catalog_byte_drift_fails_closed(self) -> None:
        fixture = SyntheticReleaseInput(self.base)
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
        fixture = SyntheticReleaseInput(
            self.base,
            sidecar_bytes=b"x" * next(iter(bundle.STALE_SIDECAR_BYTE_LENGTHS)),
        )
        with self.assertRaisesRegex(bundle.BundleError, "retired stale sidecar"):
            bundle.verify_release_input(
                fixture.root,
                sidecar_verifier=_accept_synthetic_sidecar,
                qualified_profile_verifier=_accept_synthetic_profile,
                promotion_attestation_verifier=_accept_synthetic_attestation,
            )

    def test_qualification_must_bind_the_exact_signed_sidecar_protocol(self) -> None:
        fixture = SyntheticReleaseInput(self.base)
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
            bundle.verify_release_input(
                fixture.root,
                sidecar_verifier=_accept_synthetic_sidecar,
                qualified_profile_verifier=_accept_synthetic_profile,
                promotion_attestation_verifier=_accept_synthetic_attestation,
            )

    def test_product_release_input_rejects_legacy_v1_protocol(self) -> None:
        fixture = SyntheticReleaseInput(self.base)
        descriptor_path = fixture.root / "release-input.json"
        descriptor = json.loads(descriptor_path.read_text(encoding="utf-8"))
        descriptor["catalog"]["sidecar"]["protocol"]["request_version"] = (
            bundle.LEGACY_SMOKE_REQUEST_VERSION
        )
        descriptor_path.write_bytes(bundle._canonical_pretty(descriptor))
        with self.assertRaisesRegex(bundle.BundleError, "legacy-smoke-only"):
            bundle.verify_release_input(
                fixture.root,
                sidecar_verifier=_accept_synthetic_sidecar,
                qualified_profile_verifier=_accept_synthetic_profile,
                promotion_attestation_verifier=_accept_synthetic_attestation,
            )

    def test_release_input_rejects_hard_links(self) -> None:
        fixture = SyntheticReleaseInput(self.base)
        hardlink = fixture.root / "hardlink-notice"
        try:
            os.link(fixture.root / bundle.REQUIRED_NOTICES[0], hardlink)
        except OSError as error:
            self.skipTest(f"hard links unavailable: {error}")
        with self.assertRaisesRegex(bundle.BundleError, "single-link|hard-linked"):
            bundle.verify_release_input(
                fixture.root,
                sidecar_verifier=_accept_synthetic_sidecar,
                qualified_profile_verifier=_accept_synthetic_profile,
                promotion_attestation_verifier=_accept_synthetic_attestation,
            )

    def test_record_step_copies_already_signed_qualified_bytes_into_new_input(
        self,
    ) -> None:
        fixture = SyntheticReleaseInput(self.base)
        output = self.base / "recorded"
        verified_profiles: list[tuple[Path, str]] = []

        def verify_copied_profile(
            root: Path, profile_sha256: str
        ) -> bundle.QualifiedProfileTreeAuthority:
            self.assertTrue((root / "compiler-profile.json").is_file())
            self.assertTrue((root / bundle.QUALIFIED_PROMOTION_RECEIPT_FILE).is_file())
            verified_profiles.append((root, profile_sha256))
            return bundle._qualified_profile_tree_summary(root)

        verified = bundle.record_immutable_release_input(
            fixture.root / bundle.SIDECAR_FILE,
            [fixture.profile_root],
            output,
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
        self.assertEqual(len(verified_profiles), 3)
        self.assertTrue(
            any(root.is_relative_to(output) for root, _ in verified_profiles)
        )
        self.assertTrue(
            any(not root.is_relative_to(output) for root, _ in verified_profiles)
        )
        self.assertTrue(
            all(
                profile_sha256 == fixture.profile_sha256
                for _, profile_sha256 in verified_profiles
            )
        )
        self.assertEqual(
            (output / bundle.SIDECAR_FILE).read_bytes(), fixture.sidecar_bytes
        )
        with self.assertRaisesRegex(bundle.BundleError, "must not exist"):
            bundle.record_immutable_release_input(
                fixture.root / bundle.SIDECAR_FILE,
                [fixture.profile_root],
                output,
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
        fixture = SyntheticReleaseInput(self.base)
        output = self.base / "failed-record"

        def refuse_profile(
            _root: Path, _profile_sha256: str
        ) -> bundle.QualifiedProfileTreeAuthority:
            raise bundle.BundleError("typed verifier refused recorded profile")

        with self.assertRaisesRegex(bundle.BundleError, "typed verifier refused"):
            bundle.record_immutable_release_input(
                fixture.root / bundle.SIDECAR_FILE,
                [fixture.profile_root],
                output,
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

    def test_record_step_requires_the_rust_typed_profile_verifier(self) -> None:
        fixture = SyntheticReleaseInput(self.base)
        output = self.base / "recorded-without-typed-verifier"
        with self.assertRaisesRegex(bundle.BundleError, "Rust typed"):
            bundle.record_immutable_release_input(
                fixture.root / bundle.SIDECAR_FILE,
                [fixture.profile_root],
                output,
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
            bundle.verify_release_input(
                fixture.root, sidecar_verifier=_accept_synthetic_sidecar
            )
        with self.assertRaisesRegex(bundle.BundleError, "Rust typed"):
            bundle.prepare_product_bundle(
                fixture.root,
                self.base / "unverified-prepare",
                sidecar_verifier=_accept_synthetic_sidecar,
            )

    def test_record_step_rejects_a_different_authorized_commit(self) -> None:
        fixture = SyntheticReleaseInput(self.base)
        output = self.base / "wrong-promotion-authority"
        with self.assertRaisesRegex(bundle.BundleError, "authorized commit"):
            bundle.record_immutable_release_input(
                fixture.root / bundle.SIDECAR_FILE,
                [fixture.profile_root],
                output,
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
        fixture = SyntheticReleaseInput(self.base)
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
