from __future__ import annotations

import base64
import os
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from appcast import main, sign_dsa, verify_dsa


class AppcastSignatureTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls._keys = tempfile.TemporaryDirectory(prefix="gore-appcast-keys-")
        cls.keys = Path(cls._keys.name)
        cls.private_key, cls.public_key = cls._generate_keypair("release")
        _, cls.other_public_key = cls._generate_keypair("other")

    @classmethod
    def tearDownClass(cls) -> None:
        cls._keys.cleanup()

    @classmethod
    def _generate_keypair(cls, name: str) -> tuple[Path, Path]:
        parameters = cls.keys / f"{name}-parameters.pem"
        private_key = cls.keys / f"{name}-private.pem"
        public_key = cls.keys / f"{name}-public.pem"
        subprocess.run(
            [
                "openssl",
                "genpkey",
                "-genparam",
                "-algorithm",
                "DSA",
                "-pkeyopt",
                "dsa_paramgen_bits:1024",
                "-out",
                str(parameters),
            ],
            capture_output=True,
            check=True,
        )
        subprocess.run(
            [
                "openssl",
                "genpkey",
                "-paramfile",
                str(parameters),
                "-out",
                str(private_key),
            ],
            capture_output=True,
            check=True,
        )
        subprocess.run(
            [
                "openssl",
                "pkey",
                "-in",
                str(private_key),
                "-pubout",
                "-out",
                str(public_key),
            ],
            capture_output=True,
            check=True,
        )
        return private_key, public_key

    def setUp(self) -> None:
        self._files = tempfile.TemporaryDirectory(prefix="gore-appcast-test-")
        self.files = Path(self._files.name)
        self.installer = self.files / "gore-mod-manager-0.1.0-setup.exe"
        self.installer.write_bytes(b"test installer payload\0")
        private_key_b64 = base64.b64encode(self.private_key.read_bytes()).decode()
        self.environment = mock.patch.dict(
            os.environ,
            {"WINSPARKLE_DSA_PRIV_KEY_B64": private_key_b64},
        )
        self.environment.start()

    def tearDown(self) -> None:
        self.environment.stop()
        self._files.cleanup()

    def test_generated_signature_matches_embedded_public_key(self) -> None:
        signature = sign_dsa(self.installer)

        verify_dsa(self.installer, signature, self.public_key)

    def test_wrong_public_key_is_refused(self) -> None:
        signature = sign_dsa(self.installer)

        with self.assertRaisesRegex(SystemExit, "does not match"):
            verify_dsa(self.installer, signature, self.other_public_key)

    def test_changed_installer_is_refused(self) -> None:
        signature = sign_dsa(self.installer)
        self.installer.write_bytes(b"different payload")

        with self.assertRaisesRegex(SystemExit, "does not match"):
            verify_dsa(self.installer, signature, self.public_key)

    def test_invalid_signature_inputs_are_refused(self) -> None:
        cases = (("not base64!", "invalid base64"), ("", "empty DSA signature"))
        for signature, message in cases:
            with self.subTest(signature=signature):
                with self.assertRaisesRegex(SystemExit, message):
                    verify_dsa(self.installer, signature, self.public_key)

    def test_missing_public_key_is_refused(self) -> None:
        signature = sign_dsa(self.installer)

        with self.assertRaisesRegex(SystemExit, "public key not found"):
            verify_dsa(self.installer, signature, self.files / "missing.pem")

    def test_key_mismatch_publishes_no_appcast(self) -> None:
        output = self.files / "appcast-windows.xml"
        arguments = [
            "appcast.py",
            "--title",
            "gore-mod-manager",
            "--version",
            "0.1.0",
            "--installer",
            str(self.installer),
            "--public-key",
            str(self.other_public_key),
            "--release-tag",
            "gore-mod-manager-v0.1.0",
            "--output",
            str(output),
        ]

        with mock.patch("sys.argv", arguments):
            with self.assertRaisesRegex(SystemExit, "does not match"):
                main()
        self.assertFalse(output.exists())


if __name__ == "__main__":
    unittest.main()
