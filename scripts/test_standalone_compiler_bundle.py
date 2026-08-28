from __future__ import annotations

from contextlib import contextmanager
import hashlib
import json
from pathlib import Path
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


def _typed_profile(
    root: Path, _profile_sha256: str
) -> bundle.QualifiedProfileTreeAuthority:
    return bundle._qualified_profile_tree_summary(root)


def _synthetic_promotion_audits(
    root: Path,
    _profile: dict[str, object],
    _manifest: bytes,
    _blobs: list[tuple[str, bundle.Seal, str]],
    _expected_sidecar: dict[str, object],
) -> dict[str, bundle.Seal]:
    names = (
        bundle.EMBEDDED_QUALIFICATION_ARTIFACT_MANIFEST_FILE,
        bundle.STANDALONE_QUALIFICATION_ARTIFACT_MANIFEST_FILE,
        bundle.QUALIFIED_PROMOTION_RECEIPT_FILE,
    )
    return {
        name: bundle.Seal(
            len((root / name).read_bytes()), _sha256((root / name).read_bytes())
        )
        for name in names
    }


@contextmanager
def _synthetic_qualification():
    with mock.patch.object(
        bundle, "_verify_profile_promotion", side_effect=_synthetic_promotion_audits
    ):
        yield


class SyntheticProfiles:
    def __init__(self, root: Path) -> None:
        root.mkdir()
        self.root = root
        self.reference_identity = {
            "byte_len": 123_456,
            "sha256": "12" * 32,
            "request_version": bundle.PRODUCTION_REQUEST_VERSION,
            "response_version": bundle.PROTOCOL_RESPONSE_VERSION,
        }
        self.profile_roots = [
            self._write_profile(
                build_id=24_878_692,
                manifest_gid=382_135_126_159_906_494,
                guid="c2ca4ada-4878-d963-e567-717dc2c483a2",
                marker="new",
            ),
            self._write_profile(
                build_id=24_539_464,
                manifest_gid=1_585_071_322_101_748_861,
                guid="cf0b83bd-e023-061b-2100-0f0fccf871d2",
                marker="old",
            ),
        ]
        self.notices: dict[str, Path] = {}
        for name in bundle.REQUIRED_NOTICES:
            path = root / name
            path.write_bytes(f"synthetic {name}\n".encode())
            self.notices[name] = path

    def _write_profile(
        self, *, build_id: int, manifest_gid: int, guid: str, marker: str
    ) -> Path:
        root = self.root / f"source-{marker}"
        root.mkdir()
        payloads: dict[tuple[str, str], dict[str, object]] = {}
        for index, (group, field) in enumerate(bundle.PROFILE_BLOB_FIELDS):
            relative = f"payload/{index:02d}-{field}.json"
            path = root / relative
            path.parent.mkdir(exist_ok=True)
            if field in ("diagnostic_parity", "semantic_parity"):
                bytes_ = bundle._canonical_pretty(
                    {
                        "schema": f"synthetic.{field}",
                        "standalone_compiler": self.reference_identity,
                    }
                )
            else:
                bytes_ = f"{marker}:{group}.{field}\n".encode()
            path.write_bytes(bytes_)
            payloads[(group, field)] = {"path": relative, **_seal(bytes_)}
        target = {
            "steam_app_id": 1_297_900,
            "steam_build_id": build_id,
            "depot_id": 1_297_901,
            "depot_manifest_gid": manifest_gid,
            "platform": "windows",
            "architecture": "x86_64",
            "build_configuration": "shipping",
        }
        profile = {
            "schema": "gore.as.compiler-profile",
            "schema_version": 1,
            "target": target,
            "oracle": {"pe_codeview": {"guid": guid, "age": 1}},
            "engine": {
                field: payloads[("engine", field)]
                for field in (
                    "ordered_engine_properties",
                    "registration_trace",
                    "post_bind_snapshot",
                )
            },
            "unreal_semantics": {
                "reflected_type_graph": payloads[
                    ("unreal_semantics", "reflected_type_graph")
                ]
            },
            "frontend": {
                field: payloads[("frontend", field)]
                for field in (
                    "preprocessor_config",
                    "class_generator_config",
                    "compiler_options",
                )
            },
            "bytecode": {
                field: payloads[("bytecode", field)]
                for field in (
                    "opcode_table",
                    "operand_schema",
                    "codegen_probe_corpus",
                    "expected_probe_results",
                )
            },
            "cache_writer": {
                field: payloads[("cache_writer", field)]
                for field in (
                    "serializer_schema",
                    "reference_table_order",
                    "normalized_oracle_corpus",
                )
            },
            "qualification": {
                "qualified": True,
                "diagnostic_parity": payloads[
                    ("qualification", "diagnostic_parity")
                ],
                "semantic_parity": payloads[("qualification", "semantic_parity")],
            },
            "profile_sha256": _sha256(f"profile:{marker}".encode()),
        }
        (root / "compiler-profile.json").write_bytes(bundle._canonical_pretty(profile))
        for name in (
            bundle.EMBEDDED_QUALIFICATION_ARTIFACT_MANIFEST_FILE,
            bundle.STANDALONE_QUALIFICATION_ARTIFACT_MANIFEST_FILE,
            bundle.QUALIFIED_PROMOTION_RECEIPT_FILE,
        ):
            (root / name).write_bytes(f"synthetic audit {name}\n".encode())
        return root


class StandaloneCompilerBundleTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.base = Path(self.temp.name).resolve()
        self.fixture = SyntheticProfiles(self.base / "fixture")

    def tearDown(self) -> None:
        self.temp.cleanup()

    @staticmethod
    def _capabilities(
        *,
        compatibility_id: str = bundle.STANDALONE_COMPATIBILITY_ID,
        request_version: int = bundle.PRODUCTION_REQUEST_VERSION,
    ) -> mock.Mock:
        return mock.Mock(
            returncode=0,
            stdout=bundle._canonical_pretty(
                {
                    "backend": "gore-as-standalone-compiler",
                    "compatibility_id": compatibility_id,
                    "request_version": request_version,
                    "request_versions": [
                        bundle.LEGACY_SMOKE_REQUEST_VERSION,
                        request_version,
                    ],
                    "response_version": bundle.PROTOCOL_RESPONSE_VERSION,
                    "compile": {
                        "available": True,
                        "requires_qualified_profile": True,
                        "requires_unreal_runtime": False,
                        "requires_game_dll": False,
                    },
                }
            ),
            stderr=b"",
        )

    def _pack(
        self, name: str, roots: list[Path] | None = None
    ) -> tuple[Path, Path, Path]:
        output = self.base / name
        output.mkdir()
        archive = output / bundle.QUALIFIED_PROFILES_ARCHIVE_FILE
        descriptor = output / bundle.QUALIFIED_PROFILES_DESCRIPTOR_FILE
        with _synthetic_qualification():
            bundle.pack_qualified_profiles_archive(
                roots or self.fixture.profile_roots,
                archive,
                descriptor,
                qualified_profile_verifier=_typed_profile,
                notice_sources=self.fixture.notices,
            )
            extracted = self.base / f"{name}-extracted"
            bundle.materialize_qualified_profiles_package(
                archive,
                descriptor,
                extracted,
                qualified_profile_verifier=_typed_profile,
            )
        return archive, descriptor, extracted

    def _compose(
        self,
        extracted: Path,
        sidecar: Path,
        *,
        capabilities: mock.Mock | None = None,
    ) -> bundle.PreparedBundle:
        with (
            _synthetic_qualification(),
            mock.patch.object(bundle, "_verify_unsigned_sidecar"),
            mock.patch.object(
                bundle.subprocess,
                "run",
                return_value=capabilities or self._capabilities(),
            ),
        ):
            return bundle.prepare_product_bundle_from_profiles(
                extracted,
                sidecar,
                self.base / "product-work",
                qualified_profile_verifier=_typed_profile,
                require_authenticode=False,
            )

    def test_profile_pack_is_deterministic_and_sidecar_free(self) -> None:
        first, first_descriptor, _ = self._pack("first")
        second, second_descriptor, _ = self._pack(
            "second", list(reversed(self.fixture.profile_roots))
        )
        self.assertEqual(first.read_bytes(), second.read_bytes())
        self.assertEqual(first_descriptor.read_bytes(), second_descriptor.read_bytes())
        with zipfile.ZipFile(first) as archive:
            names = archive.namelist()
        self.assertTrue(any(name.startswith("profiles/") for name in names))
        self.assertFalse(any(name.casefold().endswith(".exe") for name in names))

    def test_fresh_sidecar_bytes_are_pinned_by_compatible_product_catalog(self) -> None:
        _, _, extracted = self._pack("asset")
        sidecar = self.base / "fresh.exe"
        sidecar.write_bytes(b"fresh release sidecar")
        prepared = self._compose(extracted, sidecar)
        catalog = json.loads(prepared.catalog_path.read_text(encoding="utf-8"))
        self.assertEqual(catalog["sidecar"]["sha256"], _sha256(sidecar.read_bytes()))
        self.assertNotEqual(
            catalog["sidecar"]["sha256"],
            catalog["qualification_reference"]["sha256"],
        )
        self.assertFalse(prepared.require_authenticode)

    def test_composer_rejects_compatibility_and_protocol_mismatch(self) -> None:
        _, _, extracted = self._pack("asset")
        sidecar = self.base / "fresh.exe"
        sidecar.write_bytes(b"fresh release sidecar")
        cases = (
            (self._capabilities(compatibility_id="different-abi"), "compatibility ID"),
            (
                self._capabilities(request_version=bundle.LEGACY_SMOKE_REQUEST_VERSION),
                "FullGraph 2/1",
            ),
        )
        for capabilities, message in cases:
            with self.subTest(message=message), self.assertRaisesRegex(
                bundle.BundleError, message
            ):
                self._compose(extracted, sidecar, capabilities=capabilities)

    def test_staged_bundle_rejects_current_sidecar_tampering(self) -> None:
        _, _, extracted = self._pack("asset")
        sidecar = self.base / "fresh.exe"
        sidecar.write_bytes(b"fresh release sidecar")
        prepared = self._compose(extracted, sidecar)
        assert prepared.bundle_root is not None
        (prepared.bundle_root / bundle.SIDECAR_FILE).write_bytes(b"tampered")
        with self.assertRaisesRegex(bundle.BundleError, "length|SHA-256"):
            bundle.verify_staged_bundle(
                prepared.bundle_root,
                sidecar_verifier=lambda _path, _bytes: None,
                qualified_profile_verifier=_typed_profile,
            )


if __name__ == "__main__":
    unittest.main()
