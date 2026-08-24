from __future__ import annotations

from contextlib import contextmanager, nullcontext
import hashlib
import io
from pathlib import Path
import re
import sys
import tempfile
import unittest
from unittest import mock
import zipfile


ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))
import build as gore_build  # noqa: E402


SIGNING_CONFIG = {
    "TRUSTED_SIGNING_ENDPOINT": "https://example.codesigning.azure.net/",
    "TRUSTED_SIGNING_ACCOUNT": "account",
    "TRUSTED_SIGNING_PROFILE": "profile",
    "AZURE_TENANT_ID": "tenant",
    "AZURE_CLIENT_ID": "client",
    "AZURE_CLIENT_SECRET": "secret",
}


class TrustedSigningRuntimeTest(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.runtime = Path(self.temp.name) / "trusted-signing"
        self.files = {
            "Azure.CodeSigning.Dlib.dll": b"pinned dlib",
            "Azure.CodeSigning.dll": b"pinned dependency",
        }
        self.contract = {
            name: (len(payload), hashlib.sha256(payload).hexdigest())
            for name, payload in self.files.items()
        }

    @contextmanager
    def patched_contract(self):
        with (
            mock.patch.object(gore_build, "TS_DLIB_DIR", self.runtime),
            mock.patch.object(gore_build, "TS_DLIB_RUNTIME_FILES", self.contract),
        ):
            yield

    def write_runtime(self, files: dict[str, bytes] | None = None) -> None:
        self.runtime.mkdir()
        for name, payload in (files or self.files).items():
            (self.runtime / name).write_bytes(payload)

    def package(self, files: dict[str, bytes] | None = None) -> bytes:
        stream = io.BytesIO()
        with zipfile.ZipFile(stream, "w", compression=zipfile.ZIP_DEFLATED) as archive:
            for name, payload in (files or self.files).items():
                archive.writestr(f"bin/x64/{name}", payload)
        return stream.getvalue()

    def test_existing_gitignored_runtime_is_verified_without_network(self) -> None:
        self.write_runtime()
        with (
            self.patched_contract(),
            mock.patch("urllib.request.urlopen") as fetch,
        ):
            result = gore_build._ensure_dlib()

        self.assertEqual(result, self.runtime / "Azure.CodeSigning.Dlib.dll")
        fetch.assert_not_called()

    def test_existing_changed_file_is_rejected_without_repair_or_download(self) -> None:
        changed = dict(self.files)
        changed["Azure.CodeSigning.Dlib.dll"] = b"planted dll"
        self.write_runtime(changed)
        with (
            self.patched_contract(),
            mock.patch("urllib.request.urlopen") as fetch,
            self.assertRaisesRegex(SystemExit, "SHA-256 mismatch"),
        ):
            gore_build._ensure_dlib()

        self.assertEqual(
            (self.runtime / "Azure.CodeSigning.Dlib.dll").read_bytes(), b"planted dll"
        )
        fetch.assert_not_called()

    def test_extra_sibling_is_rejected_as_a_load_hijack_risk(self) -> None:
        self.write_runtime()
        (self.runtime / "version.dll").write_bytes(b"planted dependency")
        with (
            self.patched_contract(),
            self.assertRaisesRegex(SystemExit, "file set mismatch"),
        ):
            gore_build._ensure_dlib()

    def test_hardlinked_runtime_file_is_rejected(self) -> None:
        self.runtime.mkdir()
        outside = Path(self.temp.name) / "shared-dlib.dll"
        outside.write_bytes(self.files["Azure.CodeSigning.Dlib.dll"])
        gore_build.os.link(outside, self.runtime / "Azure.CodeSigning.Dlib.dll")
        (self.runtime / "Azure.CodeSigning.dll").write_bytes(
            self.files["Azure.CodeSigning.dll"]
        )
        with (
            self.patched_contract(),
            self.assertRaisesRegex(SystemExit, "multiple links"),
        ):
            gore_build._ensure_dlib()

    def test_bad_download_is_rejected_before_zip_parsing(self) -> None:
        downloaded = b"not the pinned package"
        response = mock.MagicMock()
        response.__enter__.return_value.read.return_value = downloaded
        with (
            self.patched_contract(),
            mock.patch.object(gore_build, "TS_DLIB_PACKAGE_BYTES", len(downloaded)),
            mock.patch.object(gore_build, "TS_DLIB_PACKAGE_SHA256", "00" * 32),
            mock.patch("urllib.request.urlopen", return_value=response),
            mock.patch.object(gore_build.zipfile, "ZipFile") as zip_reader,
            self.assertRaisesRegex(SystemExit, "package SHA-256 mismatch"),
        ):
            gore_build._ensure_dlib()

        zip_reader.assert_not_called()
        self.assertFalse(self.runtime.exists())

    def test_exact_download_is_checked_extracted_and_rechecked(self) -> None:
        downloaded = self.package()
        response = mock.MagicMock()
        response.__enter__.return_value.read.return_value = downloaded
        with (
            self.patched_contract(),
            mock.patch.object(gore_build, "TS_DLIB_PACKAGE_BYTES", len(downloaded)),
            mock.patch.object(
                gore_build,
                "TS_DLIB_PACKAGE_SHA256",
                hashlib.sha256(downloaded).hexdigest(),
            ),
            mock.patch("urllib.request.urlopen", return_value=response) as fetch,
        ):
            result = gore_build._ensure_dlib()

        self.assertEqual(result, self.runtime / "Azure.CodeSigning.Dlib.dll")
        self.assertEqual(
            {path.name: path.read_bytes() for path in self.runtime.iterdir()},
            self.files,
        )
        fetch.assert_called_once_with(gore_build.TS_DLIB_PACKAGE_URL, timeout=60)
        response.__enter__.return_value.read.assert_called_once_with(
            len(downloaded) + 1
        )

    def test_each_signtool_process_rechecks_the_runtime(self) -> None:
        dlib = self.runtime / "Azure.CodeSigning.Dlib.dll"
        signtool = Path("C:/Windows Kits/signtool.exe")
        metadata = Path("C:/temporary/metadata.json")
        target = Path("C:/output/product.exe")
        proxy = {"HTTPS_PROXY": "http://127.0.0.1:9"}
        with (
            mock.patch.object(
                gore_build,
                "_verify_trusted_signing_runtime",
                return_value=dlib,
            ) as verifier,
            mock.patch.object(
                gore_build,
                "_pin_trusted_signing_runtime",
                return_value=nullcontext(),
            ) as dlib_pin,
            mock.patch.object(
                gore_build,
                "_pin_signtool_runtime",
                return_value=nullcontext(),
            ) as signtool_pin,
            mock.patch.object(
                gore_build,
                "_verify_signtool_runtime",
                return_value=signtool,
            ) as signtool_verifier,
            mock.patch.object(
                gore_build.standalone_compiler_bundle,
                "_pin_windows_file_path",
                return_value=nullcontext(),
            ) as file_pin,
            mock.patch.object(
                gore_build.standalone_compiler_bundle,
                "_pin_windows_mutable_file_path",
                return_value=nullcontext(),
            ) as mutable_pin,
            mock.patch.object(gore_build, "_sign_proxy_overrides", return_value=proxy),
            mock.patch.object(gore_build, "run") as runner,
        ):
            gore_build._run_trusted_signing_once(signtool, dlib, metadata, [target])

        dlib_pin.assert_called_once_with(self.runtime)
        signtool_pin.assert_called_once_with(signtool.parent)
        signtool_verifier.assert_called_once_with(signtool.parent)
        verifier.assert_called_once_with(self.runtime)
        file_pin.assert_called_once_with(metadata, "Trusted Signing metadata")
        mutable_pin.assert_called_once_with(target, "Authenticode signing target")
        runner.assert_called_once_with(
            "code-sign 1 file(s)",
            gore_build._trusted_signing_args(signtool, dlib, metadata, [target]),
            extra_env=proxy,
        )

    @unittest.skipUnless(gore_build.os.name == "nt", "Windows path pins are required")
    def test_runtime_files_cannot_be_replaced_while_signtool_may_load_them(
        self,
    ) -> None:
        self.write_runtime()
        replacement = Path(self.temp.name) / "replacement.dll"
        replacement.write_bytes(b"replacement")
        dlib = self.runtime / "Azure.CodeSigning.Dlib.dll"
        with (
            self.patched_contract(),
            gore_build._pin_trusted_signing_runtime(self.runtime),
        ):
            with self.assertRaises(OSError):
                gore_build.os.replace(replacement, dlib)

        self.assertEqual(dlib.read_bytes(), self.files[dlib.name])
        self.assertTrue(replacement.exists())

    @unittest.skipUnless(gore_build.os.name == "nt", "Windows path pins are required")
    def test_signing_target_cannot_be_replaced_while_signtool_may_update_it(
        self,
    ) -> None:
        target = Path(self.temp.name) / "candidate.exe"
        replacement = Path(self.temp.name) / "replacement.exe"
        target.write_bytes(b"unsigned candidate")
        replacement.write_bytes(b"different candidate")

        with gore_build.standalone_compiler_bundle._pin_windows_mutable_file_path(
            target,
            "test signing target",
        ):
            with target.open("r+b") as mutable:
                mutable.seek(0, 2)
                mutable.write(b" signed")
                mutable.flush()
            with self.assertRaises(OSError):
                gore_build.os.replace(replacement, target)

        self.assertEqual(target.read_bytes(), b"unsigned candidate signed")
        self.assertTrue(replacement.exists())

    def test_internal_inno_wrapper_dispatches_one_checked_signature(self) -> None:
        signtool = Path("C:/Signing/signtool.exe")
        dlib = Path("C:/Signing/runtime/Azure.CodeSigning.Dlib.dll")
        metadata = Path("C:/Signing/metadata.json")
        target = Path("C:/output/setup.exe")
        argv = [
            "build.py",
            "__trusted_signing__",
            "trusted-sign-one",
            "--signtool",
            str(signtool),
            "--dlib",
            str(dlib),
            "--metadata",
            str(metadata),
            "--path",
            str(target),
        ]
        with (
            mock.patch.object(gore_build.sys, "argv", argv),
            mock.patch.object(
                gore_build, "_signing_config", return_value=SIGNING_CONFIG
            ),
            mock.patch.object(gore_build, "_run_trusted_signing_once") as signer,
        ):
            result = gore_build.main()

        self.assertEqual(result, 0)
        signer.assert_called_once_with(signtool, dlib, metadata, [target])


class PinnedSigntoolRuntimeTest(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.runtime = Path(self.temp.name) / "signtool-runtime"
        self.files = {
            "signtool.exe": b"pinned signtool",
            "wintrust.dll": b"pinned private dependency",
        }
        self.contract = {
            name: (len(payload), hashlib.sha256(payload).hexdigest())
            for name, payload in self.files.items()
        }

    def package(self) -> bytes:
        stream = io.BytesIO()
        prefix = f"bin/{gore_build.SIGNTOOL_SDK_VERSION}/x64/"
        with zipfile.ZipFile(stream, "w", compression=zipfile.ZIP_DEFLATED) as archive:
            for name, payload in self.files.items():
                archive.writestr(f"{prefix}{name}", payload)
        return stream.getvalue()

    def test_find_signtool_uses_only_the_pinned_package(self) -> None:
        expected = Path("C:/pinned-sdk/signtool.exe")
        with (
            mock.patch.dict(
                gore_build.os.environ,
                {"SIGNTOOL": "C:/attacker/signtool.exe"},
            ),
            mock.patch.object(
                gore_build,
                "_ensure_signtool",
                return_value=expected,
            ) as ensure,
            mock.patch.object(gore_build.shutil, "which") as path_lookup,
        ):
            self.assertEqual(gore_build._find_signtool(), expected)

        ensure.assert_called_once_with()
        path_lookup.assert_not_called()

    def test_exact_package_is_extracted_and_rechecked(self) -> None:
        with (
            mock.patch.object(gore_build, "SIGNTOOL_DIR", self.runtime),
            mock.patch.object(
                gore_build,
                "SIGNTOOL_RUNTIME_FILES",
                self.contract,
            ),
            mock.patch.object(
                gore_build,
                "_download_signtool_package",
                return_value=self.package(),
            ) as download,
        ):
            result = gore_build._ensure_signtool()

        self.assertEqual(result, self.runtime / "signtool.exe")
        self.assertEqual(
            {path.name: path.read_bytes() for path in self.runtime.iterdir()},
            self.files,
        )
        download.assert_called_once_with()

    def test_bad_download_is_rejected_before_zip_parsing(self) -> None:
        downloaded = b"not the pinned SDK package"
        response = mock.MagicMock()
        response.__enter__.return_value.read.return_value = downloaded
        with (
            mock.patch.object(gore_build, "SIGNTOOL_PACKAGE_BYTES", len(downloaded)),
            mock.patch.object(gore_build, "SIGNTOOL_PACKAGE_SHA256", "00" * 32),
            mock.patch("urllib.request.urlopen", return_value=response),
            mock.patch.object(gore_build.zipfile, "ZipFile") as zip_reader,
            self.assertRaisesRegex(SystemExit, "Signtool package SHA-256 mismatch"),
        ):
            gore_build._download_signtool_package()

        zip_reader.assert_not_called()

    @unittest.skipUnless(gore_build.os.name == "nt", "Windows path pins are required")
    def test_signtool_runtime_cannot_be_replaced_while_credentials_are_visible(
        self,
    ) -> None:
        self.runtime.mkdir()
        for name, payload in self.files.items():
            (self.runtime / name).write_bytes(payload)
        replacement = Path(self.temp.name) / "replacement.exe"
        replacement.write_bytes(b"attacker tool")
        signtool = self.runtime / "signtool.exe"

        with (
            mock.patch.object(
                gore_build,
                "SIGNTOOL_RUNTIME_FILES",
                self.contract,
            ),
            gore_build._pin_signtool_runtime(self.runtime),
        ):
            with self.assertRaises(OSError):
                gore_build.os.replace(replacement, signtool)

        self.assertEqual(signtool.read_bytes(), self.files["signtool.exe"])
        self.assertTrue(replacement.exists())


class InstallerSigningTest(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name) / "repo"
        self.release = self.root / "release"
        self.dist = self.root / "dist"
        self.release.mkdir(parents=True)
        self.dist.mkdir(parents=True)
        for project in ("mod-manager", "save-editor"):
            setup = self.root / "apps" / project / "installer" / "setup.iss"
            setup.parent.mkdir(parents=True)
            setup.write_text("[Setup]\n", encoding="utf-8")
        self.signtool = Path(r"C:\Signing Tools$\signtool.exe")
        self.iscc = Path(r"C:\Inno Setup\ISCC.exe")
        self.dlib = self.root / "Trusted Signing$" / "Azure.CodeSigning.Dlib.dll"
        self.metadata = self.root / "temp signing$" / "metadata.json"
        self.proxy = {"HTTPS_PROXY": "http://127.0.0.1:9"}

    @contextmanager
    def patched(self, signing_config: dict[str, str] | None):
        self.metadata.parent.mkdir(parents=True, exist_ok=True)
        self.metadata.write_text("{}", encoding="utf-8")
        with (
            mock.patch.object(gore_build, "ROOT", self.root),
            mock.patch.object(gore_build, "ISCC", self.iscc),
            mock.patch.object(gore_build, "dist_project"),
            mock.patch.object(gore_build, "read_version", return_value="0.1.0"),
            mock.patch.object(
                gore_build, "flutter_release_dir", return_value=self.release
            ),
            mock.patch.object(gore_build, "dist_dir", return_value=self.dist),
            mock.patch.object(gore_build, "_sign_and_stage_app_local_runtime"),
            mock.patch.object(
                gore_build, "_signing_config", return_value=signing_config
            ) as signing_config_reader,
            mock.patch.object(
                gore_build, "_find_signtool", return_value=self.signtool
            ) as finder,
            mock.patch.object(
                gore_build, "_ensure_dlib", return_value=self.dlib
            ) as dlib_loader,
            mock.patch.object(
                gore_build,
                "_verify_trusted_signing_runtime",
                return_value=self.dlib,
            ) as runtime_verifier,
            mock.patch.object(
                gore_build, "_write_metadata", return_value=self.metadata
            ) as metadata_writer,
            mock.patch.object(
                gore_build, "_sign_proxy_overrides", return_value=self.proxy
            ) as proxy_reader,
            mock.patch.object(gore_build, "sign_paths") as direct_signer,
            mock.patch.object(gore_build, "run") as runner,
        ):
            yield {
                "signing_config_reader": signing_config_reader,
                "finder": finder,
                "dlib_loader": dlib_loader,
                "runtime_verifier": runtime_verifier,
                "metadata_writer": metadata_writer,
                "proxy_reader": proxy_reader,
                "direct_signer": direct_signer,
                "runner": runner,
            }

    def test_signed_manager_routes_setup_and_uninstaller_through_inno(self) -> None:
        with self.patched(SIGNING_CONFIG) as calls:
            output = gore_build.installer_project("gore-mod-manager", dry=False)

        self.assertEqual(output, self.dist / "gore-mod-manager-0.1.0-setup.exe")
        calls["runner"].assert_called_once()
        command = calls["runner"].call_args.args[1]
        tool_name = gore_build.PROJECTS["gore-mod-manager"]["inno_sign_tool"]
        sign_options = [str(arg) for arg in command if str(arg).startswith("/S")]
        self.assertEqual(len(sign_options), 1)
        self.assertTrue(sign_options[0].startswith(f"/S{tool_name}="))
        self.assertIn("/DGORE_SIGNED_INSTALLER=1", command)
        cache_defines = [
            str(arg)
            for arg in command
            if str(arg).startswith("/DGORE_SIGNED_UNINSTALLER_DIR=")
        ]
        self.assertEqual(len(cache_defines), 1)
        cache = Path(cache_defines[0].split("=", 1)[1])
        self.assertEqual(cache.name.split("-")[:3], ["gore", "inno", "uninstaller"])
        self.assertFalse(cache.exists())
        self.assertEqual(
            command,
            [
                self.iscc,
                "/Qp",
                "/DAppVersion=0.1.0",
                f"/DSourceDir={self.release}",
                f"/DOutputDir={self.dist}",
                "/DOutputBaseName=gore-mod-manager-0.1.0-setup",
                sign_options[0],
                "/DGORE_SIGNED_INSTALLER=1",
                cache_defines[0],
                self.root / "apps" / "mod-manager" / "installer" / "setup.iss",
            ],
        )
        self.assertIn("__trusted_signing__ trusted-sign-one", sign_options[0])
        self.assertIn(
            "--signtool $qC:\\Signing Tools$$\\signtool.exe$q",
            sign_options[0],
        )
        self.assertIn(
            "--dlib $q" + str(self.dlib).replace("$", "$$") + "$q",
            sign_options[0],
        )
        self.assertIn(
            "--metadata $q" + str(self.metadata).replace("$", "$$") + "$q",
            sign_options[0],
        )
        self.assertTrue(sign_options[0].endswith(" $f"))
        self.assertEqual(sign_options[0].count("$f"), 1)
        self.assertEqual(calls["runner"].call_args.kwargs["extra_env"], self.proxy)
        calls["direct_signer"].assert_not_called()
        calls["metadata_writer"].assert_called_once_with(SIGNING_CONFIG)
        calls["runtime_verifier"].assert_called_once_with(self.dlib.parent)
        self.assertFalse(self.metadata.exists())

    def test_manager_recipe_and_build_share_one_unique_sign_tool_name(self) -> None:
        tool_name = gore_build.PROJECTS["gore-mod-manager"]["inno_sign_tool"]
        self.assertRegex(tool_name, re.compile(r"^gore_mod_manager_ats_[a-f0-9]{16}$"))
        setup = (ROOT / "apps" / "mod-manager" / "installer" / "setup.iss").read_text(
            encoding="utf-8"
        )
        self.assertEqual(setup.count(f"SignTool={tool_name}"), 1)
        self.assertIn("#ifdef GORE_SIGNED_INSTALLER", setup)
        self.assertIn("SignedUninstaller=yes", setup)
        self.assertIn("SignedUninstallerDir={#GORE_SIGNED_UNINSTALLER_DIR}", setup)
        self.assertNotIn("inno_sign_tool", gore_build.PROJECTS["gore-save-editor"])
        self.assertNotIn("inno_sign_tool", gore_build.PROJECTS["gore-mod-studio"])

    def test_inno_failure_still_removes_signing_metadata(self) -> None:
        with self.patched(SIGNING_CONFIG) as calls:
            calls["runner"].side_effect = SystemExit("ISCC failed")
            with self.assertRaisesRegex(SystemExit, "ISCC failed"):
                gore_build.installer_project("gore-mod-manager", dry=False)

        self.assertFalse(self.metadata.exists())
        command = calls["runner"].call_args.args[1]
        cache_define = next(
            str(arg)
            for arg in command
            if str(arg).startswith("/DGORE_SIGNED_UNINSTALLER_DIR=")
        )
        self.assertFalse(Path(cache_define.split("=", 1)[1]).exists())
        calls["direct_signer"].assert_not_called()

    def test_unsigned_manager_has_no_sign_tool_or_uninstaller_prompt_path(self) -> None:
        with self.patched(None) as calls:
            gore_build.installer_project("gore-mod-manager", dry=False)

        command = calls["runner"].call_args.args[1]
        self.assertEqual(
            command,
            [
                self.iscc,
                "/Qp",
                "/DAppVersion=0.1.0",
                f"/DSourceDir={self.release}",
                f"/DOutputDir={self.dist}",
                "/DOutputBaseName=gore-mod-manager-0.1.0-setup",
                self.root / "apps" / "mod-manager" / "installer" / "setup.iss",
            ],
        )
        self.assertNotIn("extra_env", calls["runner"].call_args.kwargs)
        calls["finder"].assert_not_called()
        calls["dlib_loader"].assert_not_called()
        calls["metadata_writer"].assert_not_called()
        calls["runtime_verifier"].assert_not_called()
        calls["proxy_reader"].assert_not_called()
        calls["direct_signer"].assert_not_called()
        self.assertTrue(self.metadata.exists())

    def test_other_installers_keep_outer_setup_signing(self) -> None:
        with self.patched(SIGNING_CONFIG) as calls:
            output = gore_build.installer_project("gore-save-editor", dry=False)

        self.assertEqual(output, self.dist / "gore-save-editor-0.1.0-setup.exe")
        command = calls["runner"].call_args.args[1]
        self.assertFalse(any(str(arg).startswith("/S") for arg in command))
        calls["signing_config_reader"].assert_not_called()
        calls["direct_signer"].assert_called_once_with([output], dry=False)
        self.assertTrue(self.metadata.exists())

    def test_inno_sign_arguments_reject_command_injection(self) -> None:
        with self.assertRaisesRegex(SystemExit, "invalid character"):
            gore_build._inno_quote_sign_arg("safe\nmalicious")
        self.assertEqual(
            gore_build._inno_quote_sign_arg("C:\\cash$dir"), "$qC:\\cash$$dir$q"
        )


if __name__ == "__main__":
    unittest.main()
