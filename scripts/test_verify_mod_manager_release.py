from __future__ import annotations

from pathlib import Path
import stat
import struct
import sys
import tempfile
from types import SimpleNamespace
from typing import Callable
import unittest
from unittest import mock
import warnings
import zipfile


ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))
import verify_mod_manager_release as verifier  # noqa: E402


VERSION = "0.1.0"


def _pe(machine: int = verifier.AMD64, marker: bytes = b"") -> bytes:
    payload = bytearray(0x100)
    payload[:2] = b"MZ"
    struct.pack_into("<I", payload, 0x3C, 0x80)
    payload[0x80:0x84] = b"PE\0\0"
    struct.pack_into("<H", payload, 0x84, machine)
    return bytes(payload) + marker


def _app_metadata() -> dict[str, str]:
    return {
        **verifier.APP_METADATA,
        "FileVersion": VERSION,
        "ProductVersion": VERSION,
    }


def _installer_metadata(name: str) -> dict[str, str]:
    return {
        **verifier.INSTALLER_METADATA,
        "FileVersion": VERSION,
        "OriginalFilename": name,
        "ProductVersion": VERSION,
    }


def _inno_installer_metadata(name: str) -> dict[str, str]:
    info = _installer_metadata(name)
    return {
        field: value.ljust(verifier.INNO_INSTALLER_METADATA_WIDTHS[field], " ")
        for field, value in info.items()
    }


class _ReleaseFixture:
    def __init__(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.release = (
            self.root
            / "apps"
            / "mod-manager"
            / "build"
            / "windows"
            / "x64"
            / "runner"
            / "Release"
        )
        self.release.mkdir(parents=True)
        for name in verifier.INSTALLER_SOURCE_ROOT_FILES:
            (self.release / name).write_bytes(_pe(marker=name.encode()))
        for name in verifier.REQUIRED_DATA_FILES:
            path = self.release / Path(name)
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(name.encode())

        (self.root / "LICENSE").write_text("MIT\n", encoding="utf-8")
        source_notices = (ROOT / "about.hbs").read_text(encoding="utf-8")
        notices = verifier._winsparkle_notice_section(source_notices)
        assert notices is not None
        notices += f"\n\n{verifier.WINSPARKLE_NOTICE_END}\n"
        (self.root / "about.hbs").write_text(notices, encoding="utf-8")
        (self.root / "THIRD_PARTY_LICENSES.md").write_text(notices, encoding="utf-8")
        (self.root / "apps" / "mod-manager" / "pubspec.lock").write_text(
            verifier.AUTO_UPDATER_WINDOWS_LOCK_STANZA + "\n", encoding="utf-8"
        )
        self.setup = self.root / "apps" / "mod-manager" / "installer" / "setup.iss"
        self.setup.parent.mkdir(parents=True)
        self.setup.write_text(
            """[Setup]
AppVersion={#AppVersion}
OutputBaseFilename={#OutputBaseName}
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
LicenseFile=..\\..\\..\\LICENSE
VersionInfoCompany=dh0er
VersionInfoCopyright=Copyright (C) 2026 dh0er. All rights reserved.
VersionInfoDescription=GORE Mod Manager Setup
VersionInfoOriginalFileName={#OutputBaseName}.exe
VersionInfoProductName=GORE Mod Manager
VersionInfoProductTextVersion={#AppVersion}
VersionInfoProductVersion={#AppVersion}.0
VersionInfoTextVersion={#AppVersion}
VersionInfoVersion={#AppVersion}.0

[Files]
Source: "{#SourceDir}\\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "..\\..\\..\\LICENSE"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\\..\\..\\THIRD_PARTY_LICENSES.md"; DestDir: "{app}"; Flags: ignoreversion
""",
            encoding="utf-8",
        )

        self.dist = self.root / "dist" / "gore-mod-manager"
        self.dist.mkdir(parents=True)
        self.portable = self.dist / f"gore-mod-manager-{VERSION}-windows-x64.zip"
        self.installer = self.dist / f"gore-mod-manager-{VERSION}-setup.exe"
        self.installer.write_bytes(_pe(marker=b"installer"))
        self._write_portable(self._portable_entries())

    def close(self) -> None:
        self.temp.cleanup()

    def metadata(self, path: Path) -> dict[str, str]:
        if path.name == self.installer.name:
            return _inno_installer_metadata(path.name)
        return _app_metadata()

    def _portable_entries(self) -> list[tuple[str, bytes | None]]:
        entries: list[tuple[str, bytes | None]] = [("data/", None)]
        entries.append(("data/flutter_assets/", None))
        for name in sorted(verifier.PORTABLE_ROOT_FILES):
            if name in verifier.BASE_PE_FILES:
                payload = _pe(marker=name.encode())
            elif name in ("LICENSE", "THIRD_PARTY_LICENSES.md"):
                payload = (self.root / name).read_bytes()
            else:
                payload = name.encode()
            entries.append((name, payload))
        for name in sorted(verifier.REQUIRED_DATA_FILES):
            entries.append((name, name.encode()))
        return entries

    def _write_portable(self, entries: list[tuple[str, bytes | None]]) -> None:
        with warnings.catch_warnings():
            warnings.simplefilter("ignore", UserWarning)
            with zipfile.ZipFile(self.portable, "w") as package:
                for name, payload in entries:
                    package.writestr(name, b"" if payload is None else payload)

    def mutate_portable(
        self,
        mutate: Callable[[list[tuple[str, bytes | None]]], None],
    ) -> None:
        entries = self._portable_entries()
        mutate(entries)
        self._write_portable(entries)


class ModManagerReleaseContractTest(unittest.TestCase):
    def fixture(self) -> _ReleaseFixture:
        fixture = _ReleaseFixture()
        self.addCleanup(fixture.close)
        return fixture

    def assert_problem(self, problems: list[str], text: str) -> None:
        self.assertTrue(
            any(text in problem for problem in problems),
            f"no problem contained {text!r}: {problems}",
        )

    def test_valid_release_contract_passes(self) -> None:
        fixture = self.fixture()
        self.assertEqual(
            verifier.verify_release(
                fixture.root, VERSION, version_info_reader=fixture.metadata
            ),
            [],
        )

    def test_portable_requires_payload_and_forbids_updater(self) -> None:
        mutations = (
            (
                "missing core",
                lambda entries: entries.__setitem__(
                    slice(None), [item for item in entries if item[0] != "gore_ffi.dll"]
                ),
                "missing gore_ffi.dll",
            ),
            (
                "updater",
                lambda entries: entries.append(("WinSparkle.dll", _pe())),
                "updater payload is forbidden",
            ),
            (
                "x86",
                lambda entries: entries.__setitem__(
                    slice(None),
                    [
                        (name, _pe(0x014C) if name == "gore_manager.exe" else payload)
                        for name, payload in entries
                    ],
                ),
                "expected x64",
            ),
            (
                "wrong license",
                lambda entries: entries.__setitem__(
                    slice(None),
                    [
                        (name, b"wrong") if name == "LICENSE" else (name, payload)
                        for name, payload in entries
                    ],
                ),
                "LICENSE does not match repository source",
            ),
        )
        for label, mutate, expected in mutations:
            with self.subTest(label=label):
                fixture = _ReleaseFixture()
                try:
                    fixture.mutate_portable(mutate)
                    problems = verifier.verify_release(
                        fixture.root, VERSION, version_info_reader=fixture.metadata
                    )
                    self.assert_problem(problems, expected)
                finally:
                    fixture.close()

    def test_notice_source_and_generated_file_require_winsparkle_markers(self) -> None:
        for relative in verifier.THIRD_PARTY_NOTICE_FILES:
            for label, marker in verifier.THIRD_PARTY_NOTICE_MARKERS.items():
                with self.subTest(file=relative, marker=label):
                    fixture = self.fixture()
                    path = fixture.root / relative
                    text = path.read_text(encoding="utf-8")
                    path.write_text(text.replace(marker, "", 1), encoding="utf-8")
                    problems = verifier.verify_release(
                        fixture.root, VERSION, version_info_reader=fixture.metadata
                    )
                    self.assert_problem(problems, f"{relative} is missing {label}")

    def test_exact_notice_sections_reject_non_marker_license_clause_drift(self) -> None:
        clauses = (
            (
                "WinSparkle license",
                "of the Software, and to permit persons to whom the Software is furnished to do",
            ),
            (
                "WinSparkle license",
                "OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE",
            ),
            (
                "Expat license",
                "without limitation the rights to use, copy, modify, merge, publish,",
            ),
        )
        for _, clause in clauses:
            self.assertFalse(
                any(clause in marker for marker in verifier.THIRD_PARTY_NOTICE_MARKERS.values())
            )
        for relative in verifier.THIRD_PARTY_NOTICE_FILES:
            for heading, clause in clauses:
                with self.subTest(file=relative, clause=clause):
                    fixture = self.fixture()
                    path = fixture.root / relative
                    text = path.read_text(encoding="utf-8")
                    self.assertIn(clause, text)
                    path.write_text(text.replace(clause, "", 1), encoding="utf-8")
                    problems = verifier.verify_release(
                        fixture.root, VERSION, version_info_reader=fixture.metadata
                    )
                    self.assert_problem(
                        problems,
                        f"{relative} exact WinSparkle 0.8.1 notice section changed",
                    )
                    self.assert_problem(
                        problems, f"{relative} exact upstream {heading} block changed"
                    )

    def test_winsparkle_notices_are_bound_to_pinned_dependency_source(self) -> None:
        mutations = (
            (
                "56dc406f6e0f6ccf01d70d2fbc88f7ca1c3ebf9a",
                "0000000000000000000000000000000000000000",
            ),
            ('version: "1.0.1"', 'version: "1.0.2"'),
            ("packages/auto_updater_windows", "packages/changed_updater_windows"),
        )
        for old, new in mutations:
            with self.subTest(changed=old):
                fixture = self.fixture()
                lock = fixture.root / "apps" / "mod-manager" / "pubspec.lock"
                text = lock.read_text(encoding="utf-8")
                self.assertIn(old, text)
                lock.write_text(text.replace(old, new, 1), encoding="utf-8")
                problems = verifier.verify_release(
                    fixture.root, VERSION, version_info_reader=fixture.metadata
                )
                self.assert_problem(
                    problems, "auto_updater_windows dependency pin changed"
                )

    def test_windows_path_collisions_and_traversal_fail(self) -> None:
        cases = (
            (
                [("gore_manager.exe", False), ("GORE_MANAGER.EXE", False)],
                "duplicate Windows path",
            ),
            ([("../gore_manager.exe", False)], "unsafe path"),
            ([("data", False), ("data/app.so", False)], "descends through file"),
            ([("NUL.txt", False)], "reserved Windows device"),
        )
        for entries, expected in cases:
            with self.subTest(expected=expected):
                _, problems = verifier._validate_names(entries, "fixture")
                self.assert_problem(problems, expected)

        fixture = self.fixture()
        fixture.mutate_portable(
            lambda entries: entries.append(("GORE_MANAGER.EXE", _pe()))
        )
        problems = verifier.verify_release(
            fixture.root, VERSION, version_info_reader=fixture.metadata
        )
        self.assert_problem(problems, "duplicate Windows path")

    def test_zip_directory_symlink_is_rejected(self) -> None:
        fixture = self.fixture()
        link = zipfile.ZipInfo("linked/")
        link.create_system = 3
        link.external_attr = (stat.S_IFLNK | 0o777) << 16
        with zipfile.ZipFile(fixture.portable, "a") as package:
            package.writestr(link, b"outside")

        problems = verifier.verify_release(
            fixture.root, VERSION, version_info_reader=fixture.metadata
        )
        self.assert_problem(problems, "symbolic link is forbidden: linked/")

    def test_installer_source_rejects_windows_junction_without_traversal(self) -> None:
        fixture = self.fixture()
        linked = fixture.release / "linked"
        linked.mkdir()
        (linked / "outside.dll").write_bytes(_pe())
        original = verifier._is_reparse_entry

        def emulate_junction(entry: object, status: object) -> bool:
            if getattr(entry, "name", None) == "linked":
                status = SimpleNamespace(
                    st_file_attributes=verifier._FILE_ATTRIBUTE_REPARSE_POINT
                )
            return original(entry, status)

        with mock.patch.object(
            verifier, "_is_reparse_entry", side_effect=emulate_junction
        ):
            problems = verifier.verify_release(
                fixture.root, VERSION, version_info_reader=fixture.metadata
            )

        self.assert_problem(problems, "reparse point is forbidden: linked")
        self.assertFalse(any("outside.dll" in problem for problem in problems))

    def test_installer_source_and_recipe_are_fail_closed(self) -> None:
        fixture = self.fixture()
        (fixture.release / "WinSparkle.dll").unlink()
        problems = verifier.verify_release(
            fixture.root, VERSION, version_info_reader=fixture.metadata
        )
        self.assert_problem(problems, "missing WinSparkle.dll")

        fixture = self.fixture()
        (fixture.release / "file_selector_windows_plugin.dll").write_bytes(_pe(0x014C))
        problems = verifier.verify_release(
            fixture.root, VERSION, version_info_reader=fixture.metadata
        )
        self.assert_problem(problems, "expected x64")

        fixture = self.fixture()
        text = fixture.setup.read_text(encoding="utf-8")
        fixture.setup.write_text(
            text.replace(
                'Source: "..\\..\\..\\THIRD_PARTY_LICENSES.md"; DestDir: "{app}"; Flags: ignoreversion\n',
                "",
            ),
            encoding="utf-8",
        )
        problems = verifier.verify_release(
            fixture.root, VERSION, version_info_reader=fixture.metadata
        )
        self.assert_problem(problems, "[Files] contract changed")

    def test_version_product_metadata_and_exact_artifact_names_are_required(self) -> None:
        fixture = self.fixture()

        def wrong_metadata(path: Path) -> dict[str, str]:
            info = fixture.metadata(path)
            if path.name == "gore_manager.exe":
                info["CompanyName"] = "com.example"
            else:
                info["ProductVersion"] = "9.9.9"
            return info

        problems = verifier.verify_release(
            fixture.root, VERSION, version_info_reader=wrong_metadata
        )
        self.assert_problem(problems, "CompanyName='com.example'")
        self.assert_problem(problems, "ProductVersion='9.9.9'")

        (fixture.dist / "gore-mod-manager-0.1.zip").write_bytes(b"stale")
        problems = verifier.verify_release(
            fixture.root, VERSION, version_info_reader=fixture.metadata
        )
        self.assert_problem(problems, "unexpected package artifact")

    def test_inno_installer_metadata_accepts_only_canonical_padding(self) -> None:
        fixture = self.fixture()

        def padded_metadata(path: Path) -> dict[str, str]:
            if path.name == fixture.installer.name:
                return _inno_installer_metadata(path.name)
            return _app_metadata()

        self.assertEqual(
            verifier.verify_release(
                fixture.root, VERSION, version_info_reader=padded_metadata
            ),
            [],
        )

        canonical = _inno_installer_metadata(fixture.installer.name)
        self.assertEqual(
            {field: len(value) for field, value in canonical.items()},
            verifier.INNO_INSTALLER_METADATA_WIDTHS,
        )
        self.assertEqual(
            {
                field: len(value) - len(value.rstrip(" "))
                for field, value in canonical.items()
            },
            {
                "CompanyName": 55,
                "FileDescription": 38,
                "FileVersion": 15,
                "LegalCopyright": 56,
                "OriginalFilename": 18,
                "ProductName": 44,
                "ProductVersion": 45,
            },
        )

        for field, value in canonical.items():
            for suffix, invalid_value in (
                ("missing space", value[:-1]),
                ("extra space", value + " "),
            ):
                with self.subTest(field=field, suffix=suffix):
                    def invalid_padding(path: Path) -> dict[str, str]:
                        if path.name == fixture.installer.name:
                            info = canonical.copy()
                            info[field] = invalid_value
                            return info
                        return _app_metadata()

                    problems = verifier.verify_release(
                        fixture.root, VERSION, version_info_reader=invalid_padding
                    )
                    self.assert_problem(problems, field)

        for label, field, value in (
            ("non-space suffix", "CompanyName", "dh0er X"),
            ("internal mismatch", "ProductName", "GORE  Mod Manager   "),
            (
                "unconverted copyright",
                "LegalCopyright",
                "Copyright (C) 2026 dh0er. All rights reserved.   ",
            ),
        ):
            with self.subTest(label=label):
                def invalid_metadata(path: Path) -> dict[str, str]:
                    if path.name == fixture.installer.name:
                        info = _inno_installer_metadata(path.name)
                        info[field] = value
                        return info
                    return _app_metadata()

                problems = verifier.verify_release(
                    fixture.root, VERSION, version_info_reader=invalid_metadata
                )
                self.assert_problem(problems, field)

        def padded_app_metadata(path: Path) -> dict[str, str]:
            if path.name == fixture.installer.name:
                return _inno_installer_metadata(path.name)
            info = _app_metadata()
            info["CompanyName"] += " "
            return info

        problems = verifier.verify_release(
            fixture.root, VERSION, version_info_reader=padded_app_metadata
        )
        self.assert_problem(problems, "portable zip app: CompanyName='dh0er '")
        self.assert_problem(problems, "installer source app: CompanyName='dh0er '")

    def test_repository_metadata_and_installer_recipe_are_not_templates(self) -> None:
        runner = (
            ROOT / "apps" / "mod-manager" / "windows" / "runner" / "Runner.rc"
        ).read_text(encoding="utf-8")
        self.assertNotIn("com.example", runner)
        self.assertIn('VALUE "CompanyName", "dh0er"', runner)
        self.assertIn(
            'VALUE "LegalCopyright", "Copyright (C) 2026 dh0er. All rights reserved."',
            runner,
        )

        setup = ROOT / "apps" / "mod-manager" / "installer" / "setup.iss"
        self.assertEqual(
            verifier._installer_recipe_contract(
                setup, VERSION, f"gore-mod-manager-{VERSION}-setup.exe"
            ),
            [],
        )


if __name__ == "__main__":
    unittest.main()
