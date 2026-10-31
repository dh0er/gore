from __future__ import annotations

from contextlib import nullcontext
import json
from pathlib import Path
import shutil
import sys
import tempfile
from types import SimpleNamespace
import unittest
from unittest import mock
import zipfile


ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))
import build as gore_build  # noqa: E402


class BuildStandaloneCompilerBundleTests(unittest.TestCase):
    def setUp(self) -> None:
        gore_build._PREPARED_STANDALONE_BUNDLES.clear()
        gore_build._QUALIFIED_PROFILE_VERIFIER = None
        if gore_build.os.name != "nt":
            # Exercise orchestration and catalog seals on non-Windows hosts;
            # handle pinning itself is covered by the Windows-only tests.
            pin = mock.patch.object(
                gore_build.standalone_compiler_bundle, "_pin_windows_file_path",
                side_effect=lambda *_args, **_kwargs: nullcontext(),
            )
            pin.start()
            self.addCleanup(pin.stop)

    def test_cli_studio_and_manager_are_bundle_hosts(self) -> None:
        enabled = {
            project
            for project, config in gore_build.PROJECTS.items()
            if config.get("standalone_compiler_bundle")
        }
        self.assertEqual(enabled, {"gore-cli", "gore-mod-studio", "gore-mod-manager"})

    def test_cli_studio_and_manager_share_one_prepared_bundle_authority(self) -> None:
        cli = gore_build._prepare_standalone_compiler_bundle(
            "gore-cli", dry=True
        )
        studio = gore_build._prepare_standalone_compiler_bundle(
            "gore-mod-studio", dry=True
        )
        manager = gore_build._prepare_standalone_compiler_bundle(
            "gore-mod-manager", dry=True
        )

        self.assertIs(cli, studio)
        self.assertIs(cli, manager)
        self.assertIsNotNone(cli)
        self.assertEqual(cli.catalog_path, studio.catalog_path)
        self.assertEqual(cli.bundle_root, studio.bundle_root)

    def test_catalog_is_prepared_before_cli_host_build(self) -> None:
        events: list[str] = []

        def build_env(project: str, *, dry: bool) -> dict[str, str]:
            self.assertEqual(project, "gore-cli")
            self.assertFalse(dry)
            events.append("catalog")
            return {
                "GORE_STANDALONE_COMPILER_CATALOG_PATH": "C:\\sealed\\catalog.json",
                "GORE_STANDALONE_COMPILER_CATALOG_SHA256": "ab" * 32,
            }

        def run(_label: str, _command: list[object], **kwargs: object) -> None:
            events.append("host")
            self.assertEqual(
                kwargs["extra_env"],
                {
                    "GORE_STANDALONE_COMPILER_CATALOG_PATH": "C:\\sealed\\catalog.json",
                    "GORE_STANDALONE_COMPILER_CATALOG_SHA256": "ab" * 32,
                },
            )

        with (
            mock.patch.object(
                gore_build, "_standalone_compiler_build_env", side_effect=build_env
            ),
            mock.patch.object(
                gore_build, "_verify_host_embedded_standalone_compiler_catalog"
            ) as linked,
            mock.patch.object(
                gore_build, "_stage_standalone_compiler_bundle"
            ) as staged,
            mock.patch.object(gore_build, "run", side_effect=run),
        ):
            gore_build.build_project("gore-cli", release=True, dry=False)
        self.assertEqual(events, ["catalog", "host"])
        linked.assert_called_once_with(
            "gore-cli", gore_build.target_dir(True) / "gore.exe", dry=False
        )
        staged.assert_called_once_with(
            "gore-cli", gore_build.target_dir(True), dry=False
        )

    def test_manager_core_embeds_prepared_catalog_before_link_verification(self) -> None:
        events: list[str] = []
        env = {
            "GORE_STANDALONE_COMPILER_CATALOG_PATH": "C:/sealed/catalog.json",
            "GORE_STANDALONE_COMPILER_CATALOG_SHA256": "ab" * 32,
        }

        def build_env(project: str, *, dry: bool) -> dict[str, str]:
            self.assertEqual(project, "gore-mod-manager")
            self.assertFalse(dry)
            events.append("catalog")
            return env

        def run(_label: str, command: list[object], **kwargs: object) -> None:
            self.assertEqual(command, [gore_build.CARGO, "build", "-p", "gore-ffi", "--release"])
            self.assertEqual(kwargs["extra_env"], env)
            events.append("host")

        with (
            mock.patch.object(gore_build, "_standalone_compiler_build_env", side_effect=build_env),
            mock.patch.object(gore_build, "run", side_effect=run),
            mock.patch.object(gore_build, "_verify_host_embedded_standalone_compiler_catalog") as linked,
        ):
            host = gore_build.build_core_dll("gore-mod-manager", release=True, dry=False)
        self.assertEqual(events, ["catalog", "host"])
        linked.assert_called_once_with("gore-mod-manager", host, dry=False)
        self.assertEqual(host.name, "gore_ffi.dll")

    def test_flutter_build_stages_cached_compiler_after_core_in_both_modes(self) -> None:
        bundle = gore_build.standalone_compiler_bundle
        prepared = bundle.PreparedBundle(
            present=True,
            work_root=ROOT / "target/synthetic-prepared",
            catalog_path=ROOT / "target/synthetic-prepared/embedded-catalog.json",
            bundle_root=ROOT / "target/synthetic-prepared/compiler",
            sidecar_name=bundle.SIDECAR_FILE,
            catalog_sha256="ab" * 32,
            require_authenticode=False,
        )
        for project in ("gore-mod-manager", "gore-mod-studio"):
            for release in (False, True):
                with self.subTest(project=project, release=release):
                    gore_build._PREPARED_STANDALONE_BUNDLES[(False, False)] = prepared
                    events: list[str] = []
                    typed = mock.Mock()

                    def run(_label: str, command: list[object], **kwargs: object) -> None:
                        if command[0] == gore_build.CARGO:
                            self.assertEqual(kwargs["extra_env"], {
                                "GORE_STANDALONE_COMPILER_CATALOG_PATH": str(prepared.catalog_path),
                                "GORE_STANDALONE_COMPILER_CATALOG_SHA256": prepared.catalog_sha256,
                            })
                            events.append("cargo")
                        else:
                            events.append("flutter")

                    with (
                        mock.patch.dict(gore_build.os.environ, {}, clear=True),
                        mock.patch.object(gore_build, "run", side_effect=run),
                        mock.patch.object(gore_build, "resolve_git_sha", return_value="test"),
                        mock.patch.object(gore_build, "discard_line_ending_only_churn"),
                        mock.patch.object(gore_build, "_verify_host_embedded_standalone_compiler_catalog"),
                        mock.patch.object(gore_build, "stage_core_dll", side_effect=lambda *_args, **_kwargs: events.append("core")) as core,
                        mock.patch.object(gore_build, "_qualified_profile_verifier", return_value=typed),
                        mock.patch.object(bundle, "stage_product_bundle", side_effect=lambda *_args, **_kwargs: events.append("compiler")) as stage,
                        mock.patch.object(bundle, "build_native_sidecar") as native,
                    ):
                        gore_build.build_project(project, release=release, dry=False)

                    self.assertEqual(events, ["cargo", "flutter", "core", "compiler"])
                    core.assert_called_once_with(project, release=release)
                    stage.assert_called_once_with(
                        prepared, gore_build.flutter_build_dir(project, release),
                        qualified_profile_verifier=typed,
                    )
                    native.assert_not_called()

    def test_manager_run_stages_compiler_before_launch_in_both_modes(self) -> None:
        for release in (False, True):
            with self.subTest(release=release):
                events: list[str] = []
                exe = mock.Mock()
                exe.exists.return_value = True
                with (
                    mock.patch.object(gore_build, "runnable_exe", return_value=exe),
                    mock.patch.object(gore_build, "build_core_dll"),
                    mock.patch.object(gore_build, "run"),
                    mock.patch.object(gore_build, "resolve_git_sha", return_value="test"),
                    mock.patch.object(gore_build, "discard_line_ending_only_churn"),
                    mock.patch.object(gore_build, "stage_core_dll", side_effect=lambda *_args, **_kwargs: events.append("core")),
                    mock.patch.object(gore_build, "_stage_standalone_compiler_bundle", side_effect=lambda *_args, **_kwargs: events.append("compiler")) as stage,
                    mock.patch.object(gore_build.subprocess, "Popen", side_effect=lambda *_args, **_kwargs: events.append("launch")) as launch,
                ):
                    gore_build.run_project("gore-mod-manager", release=release)

                self.assertEqual(events, ["core", "compiler", "launch"])
                stage.assert_called_once_with(
                    "gore-mod-manager", gore_build.flutter_build_dir("gore-mod-manager", release),
                    dry=False,
                )
                self.assertEqual(launch.call_args.kwargs["cwd"], exe.parent)

    def test_manager_run_does_not_launch_when_compiler_staging_fails(self) -> None:
        with (
            mock.patch.object(gore_build, "build_core_dll"),
            mock.patch.object(gore_build, "run"),
            mock.patch.object(gore_build, "resolve_git_sha", return_value="test"),
            mock.patch.object(gore_build, "discard_line_ending_only_churn"),
            mock.patch.object(gore_build, "stage_core_dll"),
            mock.patch.object(gore_build, "_stage_standalone_compiler_bundle", side_effect=SystemExit("compiler verification failed")),
            mock.patch.object(gore_build.subprocess, "Popen") as launch,
        ):
            with self.assertRaisesRegex(SystemExit, "compiler verification failed"):
                gore_build.run_project("gore-mod-manager", release=True)
        launch.assert_not_called()

    def test_manager_signing_preserves_compiler_and_microsoft_runtime_bytes(self) -> None:
        bundle_dir = Path("C:/synthetic/stage")
        runtime_names = gore_build.PROJECTS["gore-mod-manager"]["app_local_msvc_runtime"]
        plan = SimpleNamespace(names=runtime_names)
        sidecar = gore_build.standalone_compiler_bundle.SIDECAR_FILE
        with (
            mock.patch.object(gore_build, "_prepare_app_local_runtime", return_value=plan),
            mock.patch.object(gore_build, "sign_dir") as signer,
            mock.patch.object(gore_build, "_stage_runtime_atomically") as stage_runtime,
        ):
            result = gore_build._sign_and_stage_app_local_runtime(
                "gore-mod-manager", bundle_dir, dry=False, exclude_names=(sidecar,)
            )
        self.assertIs(result, plan)
        signer.assert_called_once_with(
            bundle_dir, dry=False, exclude_names=(*runtime_names, sidecar)
        )
        stage_runtime.assert_called_once_with(bundle_dir, plan)

    def test_manager_portable_and_installer_verify_the_same_signed_bundle(self) -> None:
        bundle = gore_build.standalone_compiler_bundle
        catalog = b"synthetic sealed catalog"
        digest = gore_build.hashlib.sha256(catalog).hexdigest()
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            release = root / "apps/mod-manager/build/windows/x64/runner/Release"
            release.mkdir(parents=True)
            host = release / "gore_ffi.dll"
            host.write_bytes(b"GORE_AS_EMBEDDED_COMPILER_CATALOG_SHA256=" + digest.encode())
            (release / "gore_manager.exe").write_bytes(b"synthetic runner")
            for name in ("WinSparkle.dll", "auto_updater_windows_plugin.dll"):
                (release / name).write_bytes(b"updater")
            source = root / "prepared/compiler"
            source.mkdir(parents=True)
            for name, payload in {
                bundle.SIDECAR_FILE: b"signed once",
                bundle.CATALOG_FILE: catalog,
                "profiles/test/compiler-profile.json": b"sealed profile",
            }.items():
                path = source / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(payload)
            prepared = bundle.PreparedBundle(
                present=True, work_root=source.parent,
                catalog_path=source.parent / bundle.EMBEDDED_CATALOG_FILE,
                bundle_root=source, sidecar_name=bundle.SIDECAR_FILE,
                catalog_sha256=digest, require_authenticode=True,
            )
            typed = mock.Mock()
            events: list[str] = []

            def stage(_prepared: object, destination: Path, **kwargs: object) -> None:
                self.assertIs(_prepared, prepared)
                self.assertIs(kwargs["qualified_profile_verifier"], typed)
                shutil.copytree(source, destination / "compiler")

            def verify(destination: Path, **kwargs: object) -> object:
                self.assertIs(kwargs["sidecar_verifier"], bundle.verify_sidecar)
                self.assertIs(kwargs["qualified_profile_verifier"], typed)
                self.assertEqual((destination / bundle.SIDECAR_FILE).read_bytes(), b"signed once")
                events.append("compiler")
                return SimpleNamespace(catalog_bytes=catalog)

            def inno(_label: str, _command: object, **_kwargs: object) -> None:
                events.append("installer")

            with (
                mock.patch.dict(gore_build.os.environ, {}, clear=True),
                mock.patch.object(gore_build, "ROOT", root),
                mock.patch.object(gore_build, "build_project"),
                mock.patch.object(gore_build, "read_version", return_value="0.1.0"),
                mock.patch.object(gore_build, "_prepare_standalone_compiler_bundle", return_value=prepared),
                mock.patch.object(gore_build, "_qualified_profile_verifier", return_value=typed),
                mock.patch.object(bundle, "stage_product_bundle", side_effect=stage),
                mock.patch.object(bundle, "verify_staged_bundle", side_effect=verify) as verified,
                mock.patch.object(gore_build, "_sign_and_stage_app_local_runtime") as signer,
                mock.patch.object(gore_build, "_signing_config", return_value=None),
                mock.patch.object(gore_build, "sign_paths"),
                mock.patch.object(gore_build, "run", side_effect=inno),
            ):
                # Returning no runtime plan keeps this test on compiler packaging;
                # runtime archive publication has its own corruption tests.
                signer.return_value = None
                installer = gore_build.installer_project("gore-mod-manager", dry=False)
            self.assertEqual(events, ["compiler", "compiler", "installer"])
            self.assertEqual(verified.call_count, 2)
            for call in signer.call_args_list:
                self.assertEqual(call.kwargs["exclude_names"], (bundle.SIDECAR_FILE,))
            self.assertEqual(installer.name, "gore-mod-manager-0.1.0-setup.exe")
            archive = root / "dist/gore-mod-manager/gore-mod-manager-0.1.0-windows-x64.zip"
            with zipfile.ZipFile(archive) as package:
                self.assertEqual(package.read(f"compiler/{bundle.SIDECAR_FILE}"), b"signed once")
                self.assertEqual(package.read("compiler/profiles/test/compiler-profile.json"), b"sealed profile")
                self.assertNotIn("WinSparkle.dll", package.namelist())
            self.assertEqual((release / "compiler" / bundle.SIDECAR_FILE).read_bytes(), b"signed once")

    def test_later_sign_dir_excludes_the_composed_sidecar(self) -> None:
        bundle_dir = Path("C:/synthetic/stage")
        with mock.patch.object(gore_build, "sign_dir") as signer:
            gore_build._sign_and_stage_app_local_runtime(
                "gore-mod-studio",
                bundle_dir,
                dry=False,
                exclude_names=(gore_build.standalone_compiler_bundle.SIDECAR_FILE,),
            )
        signer.assert_called_once_with(
            bundle_dir,
            dry=False,
            exclude_names=(gore_build.standalone_compiler_bundle.SIDECAR_FILE,),
        )

    def test_default_build_materializes_profiles_and_builds_sidecar(self) -> None:
        profile_pack = ROOT / "target" / "synthetic-qualified-profiles"
        sidecar = ROOT / "target/synthetic-native/gore-as-standalone-compiler.exe"
        verifier = mock.Mock()
        descriptor = gore_build.standalone_compiler_bundle.QualifiedProfilesDescriptor(
            asset=gore_build.standalone_compiler_bundle.QUALIFIED_PROFILES_ARCHIVE_FILE,
            archive=gore_build.standalone_compiler_bundle.Seal(123, "cd" * 32),
            compression="deflate-9",
            manifest_sha256="ef" * 32,
            file_count=45,
        )
        prepared = gore_build.standalone_compiler_bundle.PreparedBundle(
            present=True,
            work_root=ROOT / "target" / "standalone-compiler-product-bundle",
            catalog_path=ROOT
            / "target"
            / "standalone-compiler-product-bundle"
            / gore_build.standalone_compiler_bundle.EMBEDDED_CATALOG_FILE,
            bundle_root=ROOT / "target" / "standalone-compiler-product-bundle/compiler",
            sidecar_name=gore_build.standalone_compiler_bundle.SIDECAR_FILE,
            catalog_sha256="ab" * 32,
            require_authenticode=False,
        )
        with (
            mock.patch.dict(gore_build.os.environ, {}, clear=True),
            mock.patch.object(
                gore_build, "_qualified_profile_verifier", return_value=verifier
            ) as typed,
            mock.patch.object(
                gore_build.standalone_compiler_bundle,
                "read_qualified_profiles_descriptor",
                return_value=descriptor,
            ) as read_descriptor,
            mock.patch.object(
                gore_build.standalone_compiler_bundle,
                "materialize_qualified_profiles_package",
                return_value=profile_pack,
            ) as materialize,
            mock.patch.object(
                gore_build.standalone_compiler_bundle,
                "build_native_sidecar",
                return_value=sidecar,
            ) as build_sidecar,
            mock.patch.object(
                gore_build.standalone_compiler_bundle,
                "prepare_product_bundle_from_profiles",
                return_value=prepared,
            ) as prepare,
        ):
            result = gore_build._prepare_standalone_compiler_bundle(
                "gore-cli", dry=False
            )
        self.assertIs(result, prepared)
        typed.assert_called_once_with(dry=False)
        read_descriptor.assert_called_once_with(
            gore_build._QUALIFIED_STANDALONE_COMPILER_PROFILES_DESCRIPTOR
        )
        materialize.assert_called_once_with(
            gore_build._QUALIFIED_STANDALONE_COMPILER_PROFILES_ARCHIVE,
            gore_build._QUALIFIED_STANDALONE_COMPILER_PROFILES_DESCRIPTOR,
            ROOT / "target" / "standalone-compiler-qualified-profiles" / ("cd" * 32),
            qualified_profile_verifier=verifier,
        )
        build_sidecar.assert_called_once()
        prepare.assert_called_once_with(
            profile_pack,
            sidecar,
            ROOT / "target" / "standalone-compiler-product-bundle",
            qualified_profile_verifier=verifier,
            require_authenticode=False,
        )

    def test_non_hosts_never_touch_the_qualified_profiles(self) -> None:
        with mock.patch.object(
            gore_build.standalone_compiler_bundle,
            "read_qualified_profiles_descriptor",
        ) as read_descriptor:
            self.assertIsNone(
                gore_build._prepare_standalone_compiler_bundle(
                    "gore-save-editor", dry=False
                )
            )
        read_descriptor.assert_not_called()

    def test_checked_in_profile_pack_covers_both_supported_generations(self) -> None:
        archive_path = (
            ROOT
            / "crates/gore-as/assets"
            / gore_build.standalone_compiler_bundle.QUALIFIED_PROFILES_ARCHIVE_FILE
        )
        with zipfile.ZipFile(archive_path) as archive:
            names = archive.namelist()
            manifest = json.loads(archive.read("qualified-profiles.json"))

        self.assertFalse(any(name.casefold().endswith(".exe") for name in names))
        self.assertEqual(
            {
                profile["target"]["target"]["steam_build_id"]
                for profile in manifest["profiles"]
            },
            {24539464, 24878692},
        )

    def test_linked_host_must_report_exact_prepared_catalog_digest(self) -> None:
        digest = "ab" * 32
        prepared = gore_build.standalone_compiler_bundle.PreparedBundle(
            present=True,
            work_root=ROOT / "target/compiler",
            catalog_path=ROOT / "target/compiler/catalog.json",
            bundle_root=ROOT / "target/compiler/compiler",
            sidecar_name="gore-as-standalone-compiler.exe",
            catalog_sha256=digest,
        )
        with tempfile.TemporaryDirectory() as temporary:
            host = Path(temporary).resolve() / "synthetic-host.exe"
            host.write_bytes(
                b"binary\0GORE_AS_EMBEDDED_COMPILER_CATALOG_SHA256="
                + digest.encode("ascii")
                + b"\0"
            )
            with mock.patch.object(
                gore_build, "_prepare_standalone_compiler_bundle", return_value=prepared
            ):
                gore_build._verify_host_embedded_standalone_compiler_catalog(
                    "gore-cli", host, dry=False
                )

            host.write_bytes(b"host without authority marker")
            with (
                mock.patch.object(
                    gore_build,
                    "_prepare_standalone_compiler_bundle",
                    return_value=prepared,
                ),
                self.assertRaisesRegex(SystemExit, "does not report exactly"),
            ):
                gore_build._verify_host_embedded_standalone_compiler_catalog(
                    "gore-cli", host, dry=False
                )

    @unittest.skipUnless(gore_build.os.name == "nt", "Windows handle pins are required")
    def test_cargo_hardlink_is_promoted_to_single_link_verifier_authority(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            target_root = Path(temporary).resolve()
            release = target_root / "release"
            deps = release / "deps"
            deps.mkdir(parents=True)
            original = deps / "gore_as_qualified_profile_verifier.exe"
            original.write_bytes(b"synthetic verifier image")
            candidate = release / "gore-as-qualified-profile-verifier.exe"
            gore_build.os.link(original, candidate)
            self.assertEqual(candidate.stat().st_nlink, 2)

            authority, seal = gore_build._promote_qualified_profile_verifier_authority(
                candidate, target_root
            )

            self.assertEqual(authority.read_bytes(), b"synthetic verifier image")
            self.assertEqual(authority.stat().st_nlink, 1)
            self.assertEqual(seal.byte_len, len(b"synthetic verifier image"))
            self.assertEqual(
                seal.sha256,
                gore_build.hashlib.sha256(b"synthetic verifier image").hexdigest(),
            )

    def test_signed_build_signs_fresh_sidecar_once_before_catalog(self) -> None:
        profile_pack = ROOT / "target/synthetic-qualified-profiles"
        sidecar = ROOT / "target/synthetic-native/gore-as-standalone-compiler.exe"
        verifier = mock.Mock()
        descriptor = gore_build.standalone_compiler_bundle.QualifiedProfilesDescriptor(
            asset=gore_build.standalone_compiler_bundle.QUALIFIED_PROFILES_ARCHIVE_FILE,
            archive=gore_build.standalone_compiler_bundle.Seal(123, "cd" * 32),
            compression="deflate-9",
            manifest_sha256="ef" * 32,
            file_count=45,
        )
        prepared = gore_build.standalone_compiler_bundle.PreparedBundle(
            present=True,
            work_root=ROOT / "target" / "standalone-compiler-product-bundle",
            catalog_path=ROOT
            / "target"
            / "standalone-compiler-product-bundle/catalog.json",
            bundle_root=ROOT / "target" / "standalone-compiler-product-bundle/compiler",
            sidecar_name=gore_build.standalone_compiler_bundle.SIDECAR_FILE,
            catalog_sha256="ab" * 32,
            require_authenticode=True,
        )
        with (
            mock.patch.dict(gore_build.os.environ, {"GORE_SIGN": "1"}, clear=True),
            mock.patch.object(gore_build, "_signing_config", return_value=mock.Mock()),
            mock.patch.object(
                gore_build, "_qualified_profile_verifier", return_value=verifier
            ),
            mock.patch.object(
                gore_build.standalone_compiler_bundle,
                "read_qualified_profiles_descriptor",
                return_value=descriptor,
            ),
            mock.patch.object(
                gore_build.standalone_compiler_bundle,
                "materialize_qualified_profiles_package",
                return_value=profile_pack,
            ),
            mock.patch.object(
                gore_build.standalone_compiler_bundle,
                "build_native_sidecar",
                return_value=sidecar,
            ),
            mock.patch.object(
                gore_build.standalone_compiler_bundle, "sign_sidecar_once"
            ) as sign_once,
            mock.patch.object(
                gore_build.standalone_compiler_bundle,
                "prepare_product_bundle_from_profiles",
                return_value=prepared,
            ) as prepare,
        ):
            result = gore_build._prepare_standalone_compiler_bundle(
                "gore-mod-manager", dry=False
            )
            for project in ("gore-cli", "gore-mod-studio"):
                self.assertIs(
                    gore_build._prepare_standalone_compiler_bundle(project, dry=False),
                    result,
                )
        self.assertIs(result, prepared)
        sign_once.assert_called_once()
        self.assertEqual(sign_once.call_args.args[0], sidecar)
        self.assertEqual(sign_once.call_args.args[1].name, "signed-sidecar-identity.json")
        prepare.assert_called_once_with(
            profile_pack,
            sidecar,
            ROOT / "target" / "standalone-compiler-product-bundle",
            qualified_profile_verifier=verifier,
            require_authenticode=True,
        )


if __name__ == "__main__":
    unittest.main()
