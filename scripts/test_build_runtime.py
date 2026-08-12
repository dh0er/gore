from __future__ import annotations

from contextlib import contextmanager
from pathlib import Path
import shutil
import struct
import subprocess
import sys
import tempfile
import unittest
from unittest import mock
import zipfile


ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))
import build as gore_build  # noqa: E402


_RUNTIME_NAMES = (
    "msvcp140.dll",
    "vcruntime140.dll",
    "vcruntime140_1.dll",
)


def _write_pe(path: Path, machine: int = 0x8664, marker: bytes = b"") -> None:
    payload = bytearray(0x100)
    payload[:2] = b"MZ"
    struct.pack_into("<I", payload, 0x3C, 0x80)
    payload[0x80:0x84] = b"PE\0\0"
    struct.pack_into("<H", payload, 0x84, machine)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(payload + marker)


def _snapshot(directory: Path) -> dict[str, bytes]:
    return {
        path.relative_to(directory).as_posix(): path.read_bytes()
        for path in directory.rglob("*")
        if path.is_file()
    }


class _RuntimeFixture:
    def __init__(
        self,
        *,
        toolset_version: str = "14.44.35207",
        redist_version: str = "14.44.35112",
        crt_family: str = "143",
    ) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.base = Path(self.temp.name)
        self.root = self.base / "repo"
        self.app = self.root / "apps" / "mod-manager"
        self.bundle = self.app / "build" / "windows" / "x64" / "runner" / "Release"
        self.bundle.mkdir(parents=True)
        _write_pe(self.bundle / "gore_manager.exe", marker=b"manager")

        self.vc = self.base / "Visual Studio" / "VC"
        self.linker = (
            self.vc
            / "Tools"
            / "MSVC"
            / toolset_version
            / "bin"
            / "Hostx64"
            / "x64"
            / "link.exe"
        )
        _write_pe(self.linker, marker=b"link")
        _write_pe(self.linker.with_name("dumpbin.exe"), marker=b"dumpbin")
        version_file = (
            self.vc / "Auxiliary" / "Build" / "Microsoft.VCRedistVersion.default.txt"
        )
        version_file.parent.mkdir(parents=True)
        version_file.write_text(f"{redist_version}\n", encoding="utf-8")
        self.crt = self.add_crt_family(crt_family, redist_version)

        cache = self.app / "build" / "windows" / "x64" / "CMakeCache.txt"
        cache.parent.mkdir(parents=True, exist_ok=True)
        cache.write_text(
            "CMAKE_GENERATOR_PLATFORM:INTERNAL=x64\n"
            f"CMAKE_LINKER:FILEPATH={self.linker.as_posix()}\n",
            encoding="utf-8",
        )
        self.extra_app_imports: tuple[str, ...] = ()

    def add_crt_family(self, family: str, version: str = "14.44.35112") -> Path:
        crt = (
            self.vc
            / "Redist"
            / "MSVC"
            / version
            / "x64"
            / f"Microsoft.VC{family}.CRT"
        )
        for index, name in enumerate(_RUNTIME_NAMES):
            _write_pe(crt / name, marker=f"{version}-source-{index}".encode())
        return crt

    def close(self) -> None:
        self.temp.cleanup()

    def dumpbin(self, args: list[str], **_: object) -> subprocess.CompletedProcess[str]:
        target = Path(args[-1])
        dependencies: tuple[str, ...]
        if target.name.casefold() == "gore_manager.exe":
            dependencies = _RUNTIME_NAMES + self.extra_app_imports
        elif target.name.casefold() == "msvcp140.dll":
            dependencies = ("VCRUNTIME140.dll", "VCRUNTIME140_1.dll")
        elif target.name.casefold() == "vcruntime140_1.dll":
            dependencies = ("VCRUNTIME140.dll",)
        else:
            dependencies = ()
        stdout = "\n".join(f"    {name}" for name in dependencies)
        return subprocess.CompletedProcess(args, 0, stdout=stdout, stderr="")

    @contextmanager
    def patched(self):
        with (
            mock.patch.object(gore_build, "ROOT", self.root),
            mock.patch.object(gore_build.subprocess, "run", side_effect=self.dumpbin),
        ):
            yield


class AppLocalMsvcRuntimeTest(unittest.TestCase):
    def test_runtime_contract_is_manager_only(self) -> None:
        self.assertEqual(
            tuple(gore_build.PROJECTS["gore-mod-manager"]["app_local_msvc_runtime"]),
            _RUNTIME_NAMES,
        )
        self.assertNotIn("app_local_msvc_runtime", gore_build.PROJECTS["gore-save-editor"])
        self.assertNotIn("app_local_msvc_runtime", gore_build.PROJECTS["gore-mod-studio"])

    def test_signs_owned_pes_then_stages_exact_toolchain_runtime(self) -> None:
        fixture = _RuntimeFixture()
        self.addCleanup(fixture.close)
        for index, name in enumerate(_RUNTIME_NAMES):
            (fixture.bundle / name).write_bytes(f"old-{index}".encode())

        signed: list[Path] = []

        def record_sign(paths: list[Path], dry: bool) -> None:
            self.assertFalse(dry)
            signed.extend(paths)
            for name in _RUNTIME_NAMES:
                self.assertTrue((fixture.bundle / name).read_bytes().startswith(b"old-"))

        with fixture.patched(), mock.patch.object(
            gore_build, "sign_paths", side_effect=record_sign
        ):
            plan = gore_build._sign_and_stage_app_local_runtime(
                "gore-mod-manager", fixture.bundle, dry=False
            )

        self.assertIsNotNone(plan)
        assert plan is not None
        self.assertEqual(plan.names, _RUNTIME_NAMES)
        self.assertIn(fixture.bundle / "gore_manager.exe", signed)
        self.assertTrue(
            all((fixture.bundle / name) not in signed for name in _RUNTIME_NAMES)
        )
        for name in _RUNTIME_NAMES:
            self.assertEqual((fixture.bundle / name).read_bytes(), (fixture.crt / name).read_bytes())

    def test_stages_vs2026_vc145_runtime(self) -> None:
        fixture = _RuntimeFixture(
            toolset_version="14.51.36231",
            redist_version="14.51.36231",
            crt_family="145",
        )
        self.addCleanup(fixture.close)

        with fixture.patched(), mock.patch.object(gore_build, "sign_paths"):
            plan = gore_build._sign_and_stage_app_local_runtime(
                "gore-mod-manager", fixture.bundle, dry=False
            )

        self.assertIsNotNone(plan)
        for name in _RUNTIME_NAMES:
            self.assertEqual((fixture.bundle / name).read_bytes(), (fixture.crt / name).read_bytes())

    def test_rejects_missing_or_ambiguous_crt_family_before_signing(self) -> None:
        def missing(fixture: _RuntimeFixture) -> None:
            shutil.rmtree(fixture.crt)

        def ambiguous(fixture: _RuntimeFixture) -> None:
            fixture.add_crt_family("145")

        for label, mutate in (("missing", missing), ("ambiguous", ambiguous)):
            with self.subTest(label=label):
                fixture = _RuntimeFixture()
                try:
                    before = _snapshot(fixture.bundle)
                    mutate(fixture)
                    with fixture.patched(), mock.patch.object(
                        gore_build, "sign_paths"
                    ) as signer:
                        with self.assertRaisesRegex(
                            SystemExit, "exactly one CRT family"
                        ):
                            gore_build._sign_and_stage_app_local_runtime(
                                "gore-mod-manager", fixture.bundle, dry=False
                            )
                    signer.assert_not_called()
                    self.assertEqual(_snapshot(fixture.bundle), before)
                finally:
                    fixture.close()

    def test_invalid_sources_and_closure_leave_bundle_unchanged(self) -> None:
        def missing_source(fixture: _RuntimeFixture) -> None:
            (fixture.crt / "vcruntime140_1.dll").unlink()

        def wrong_arch(fixture: _RuntimeFixture) -> None:
            _write_pe(fixture.crt / "vcruntime140.dll", machine=0x014C, marker=b"x86")

        def expanded_closure(fixture: _RuntimeFixture) -> None:
            fixture.extra_app_imports = ("MSVCP140_2.dll",)

        for label, mutate in (
            ("missing", missing_source),
            ("x86", wrong_arch),
            ("closure", expanded_closure),
        ):
            with self.subTest(label=label):
                fixture = _RuntimeFixture()
                try:
                    for index, name in enumerate(_RUNTIME_NAMES):
                        (fixture.bundle / name).write_bytes(f"old-{index}".encode())
                    before = _snapshot(fixture.bundle)
                    mutate(fixture)
                    with fixture.patched(), mock.patch.object(
                        gore_build, "sign_paths"
                    ) as signer:
                        with self.assertRaises(SystemExit):
                            gore_build._sign_and_stage_app_local_runtime(
                                "gore-mod-manager", fixture.bundle, dry=False
                            )
                    signer.assert_not_called()
                    self.assertEqual(_snapshot(fixture.bundle), before)
                finally:
                    fixture.close()

    def test_zip_contract_rejects_old_package_without_runtime(self) -> None:
        fixture = _RuntimeFixture()
        self.addCleanup(fixture.close)
        with fixture.patched():
            plan = gore_build._prepare_app_local_runtime(
                "gore-mod-manager", fixture.bundle
            )
        self.assertIsNotNone(plan)
        assert plan is not None

        old_zip = fixture.base / "old-package.zip"
        with zipfile.ZipFile(old_zip, "w") as package:
            package.write(fixture.bundle / "gore_manager.exe", "gore_manager.exe")
        with self.assertRaises(SystemExit):
            gore_build._verify_runtime_zip(old_zip, plan)

        gore_build._stage_runtime_atomically(fixture.bundle, plan)
        complete = Path(
            shutil.make_archive(
                str(fixture.base / "complete-package"), "zip", root_dir=fixture.bundle
            )
        )
        gore_build._verify_runtime_zip(complete, plan)

    def test_failed_stage_leaves_no_packaged_transaction_residue(self) -> None:
        fixture = _RuntimeFixture()
        self.addCleanup(fixture.close)
        with fixture.patched():
            plan = gore_build._prepare_app_local_runtime(
                "gore-mod-manager", fixture.bundle
            )
        self.assertIsNotNone(plan)
        assert plan is not None

        for index, name in enumerate(_RUNTIME_NAMES):
            _write_pe(fixture.bundle / name, marker=f"old-{index}".encode())
        before = _snapshot(fixture.bundle)
        real_replace = gore_build.os.replace
        real_unlink = Path.unlink
        failed_replace = False

        def fail_one_install(source: object, target: object) -> None:
            nonlocal failed_replace
            source_path = Path(source)
            target_path = Path(target)
            is_prepared_runtime = source_path.parent.name == "prepared" or source_path.name.endswith(
                ".gore-runtime.tmp"
            )
            if (
                not failed_replace
                and is_prepared_runtime
                and target_path.name.casefold() == "vcruntime140.dll"
            ):
                failed_replace = True
                raise OSError("injected runtime replace failure")
            real_replace(source, target)

        def fail_legacy_temp_cleanup(path: Path, *args: object, **kwargs: object) -> None:
            if path.name.endswith(".gore-runtime.tmp"):
                raise OSError("injected in-bundle temp cleanup failure")
            real_unlink(path, *args, **kwargs)

        with (
            mock.patch.object(gore_build.os, "replace", side_effect=fail_one_install),
            mock.patch.object(Path, "unlink", new=fail_legacy_temp_cleanup),
        ):
            with self.assertRaisesRegex(OSError, "injected runtime replace failure"):
                gore_build._stage_runtime_atomically(fixture.bundle, plan)

        self.assertTrue(failed_replace)
        self.assertEqual(_snapshot(fixture.bundle), before)
        self.assertFalse(
            any(".gore-runtime." in path.name for path in fixture.bundle.rglob("*"))
        )
        self.assertFalse(list(fixture.bundle.parent.glob(".gore-runtime-*")))

        gore_build._stage_runtime_atomically(fixture.bundle, plan)
        archive = Path(
            shutil.make_archive(
                str(fixture.base / "rerun-package"), "zip", root_dir=fixture.bundle
            )
        )
        gore_build._verify_runtime_zip(archive, plan)
        with zipfile.ZipFile(archive) as package:
            self.assertFalse(
                any(".gore-runtime." in name for name in package.namelist())
            )

    def test_failed_zip_verification_preserves_previous_artifact(self) -> None:
        fixture = _RuntimeFixture()
        self.addCleanup(fixture.close)
        version = "0.1.0"
        dist = fixture.root / "dist" / "gore-mod-manager"
        dist.mkdir(parents=True)
        final_archive = dist / f"gore-mod-manager-{version}-windows-x64.zip"
        final_archive.write_bytes(b"previous verified artifact")
        real_make_archive = gore_build.shutil.make_archive

        def make_archive_without_one_runtime(*args: object, **kwargs: object) -> str:
            archive = Path(real_make_archive(*args, **kwargs))
            broken = archive.with_suffix(".missing-runtime")
            with (
                zipfile.ZipFile(archive) as source,
                zipfile.ZipFile(broken, "w") as target,
            ):
                for info in source.infolist():
                    if Path(info.filename).name.casefold() == "vcruntime140_1.dll":
                        continue
                    target.writestr(info, source.read(info.filename))
            broken.replace(archive)
            return str(archive)

        with (
            fixture.patched(),
            mock.patch.object(gore_build, "build_project"),
            mock.patch.object(gore_build, "stage_companions"),
            mock.patch.object(gore_build, "read_version", return_value=version),
            mock.patch.object(gore_build, "sign_paths"),
            mock.patch.object(
                gore_build.shutil,
                "make_archive",
                side_effect=make_archive_without_one_runtime,
            ),
        ):
            with self.assertRaisesRegex(
                SystemExit, "portable zip is missing app-local MSVC runtime"
            ):
                gore_build.dist_project("gore-mod-manager", dry=False)

        self.assertEqual(final_archive.read_bytes(), b"previous verified artifact")
        self.assertFalse((dist / "_stage").exists())
        self.assertFalse(list(dist.glob(".gore-package-*")))
        self.assertFalse(list(dist.glob(".gore-runtime-*")))

    def test_verified_zip_publishes_exact_dotted_version_filename(self) -> None:
        fixture = _RuntimeFixture()
        self.addCleanup(fixture.close)
        version = "0.1.0"
        dist = fixture.root / "dist" / "gore-mod-manager"
        expected = dist / f"gore-mod-manager-{version}-windows-x64.zip"

        with (
            fixture.patched(),
            mock.patch.object(gore_build, "build_project"),
            mock.patch.object(gore_build, "stage_companions"),
            mock.patch.object(gore_build, "read_version", return_value=version),
            mock.patch.object(gore_build, "sign_paths"),
        ):
            archive = gore_build.dist_project("gore-mod-manager", dry=False)

        self.assertEqual(archive, expected)
        self.assertEqual(list(dist.glob("*.zip")), [expected])
        self.assertFalse((dist / "_stage").exists())
        self.assertFalse(list(dist.glob(".gore-package-*")))
        self.assertFalse(list(dist.glob(".gore-runtime-*")))


if __name__ == "__main__":
    unittest.main()
