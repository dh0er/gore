from __future__ import annotations

from contextlib import contextmanager
from pathlib import Path
import re
import sys
import tempfile
import unittest
from unittest import mock


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
            mock.patch.object(gore_build, "_find_signtool", return_value=self.signtool) as finder,
            mock.patch.object(gore_build, "_ensure_dlib", return_value=self.dlib) as dlib_loader,
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
        self.assertIn(
            "$qC:\\Signing Tools$$\\signtool.exe$q sign /v /fd SHA256",
            sign_options[0],
        )
        self.assertIn(
            "/dlib $q" + str(self.dlib).replace("$", "$$") + "$q",
            sign_options[0],
        )
        self.assertIn(
            "/dmdf $q" + str(self.metadata).replace("$", "$$") + "$q",
            sign_options[0],
        )
        self.assertTrue(sign_options[0].endswith(" $f"))
        self.assertEqual(sign_options[0].count("$f"), 1)
        self.assertEqual(calls["runner"].call_args.kwargs["extra_env"], self.proxy)
        calls["direct_signer"].assert_not_called()
        calls["metadata_writer"].assert_called_once_with(SIGNING_CONFIG)
        self.assertFalse(self.metadata.exists())

    def test_manager_recipe_and_build_share_one_unique_sign_tool_name(self) -> None:
        tool_name = gore_build.PROJECTS["gore-mod-manager"]["inno_sign_tool"]
        self.assertRegex(tool_name, re.compile(r"^gore_mod_manager_ats_[a-f0-9]{16}$"))
        setup = (
            ROOT / "apps" / "mod-manager" / "installer" / "setup.iss"
        ).read_text(encoding="utf-8")
        self.assertEqual(setup.count(f"SignTool={tool_name}"), 1)
        self.assertIn("#ifdef GORE_SIGNED_INSTALLER", setup)
        self.assertIn("SignedUninstaller=yes", setup)
        self.assertIn(
            "SignedUninstallerDir={#GORE_SIGNED_UNINSTALLER_DIR}", setup
        )
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
        self.assertEqual(gore_build._inno_quote_sign_arg("C:\\cash$dir"), "$qC:\\cash$$dir$q")


if __name__ == "__main__":
    unittest.main()
