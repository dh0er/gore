from __future__ import annotations

import tempfile
import unittest
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import build_native


class BuildNativeTests(unittest.TestCase):
    def test_native_artifacts_are_copied_next_to_windows_release_exe(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            app = root / "apps" / "goresave"
            target = root / "target" / "release"
            bundle = app / "build" / "windows" / "x64" / "runner" / "Release"
            target.mkdir(parents=True)
            bundle.mkdir(parents=True)

            (target / "goresave_core.dll").write_bytes(b"core")
            (target / "goresave_g1r_codec_host.exe").write_bytes(b"host")
            (bundle / "goresave.exe").write_bytes(b"app")

            copied = build_native.copy_native_artifacts_to_windows_bundle(
                root=root,
                app=app,
                profile="release",
            )

            self.assertEqual(
                copied,
                [
                    bundle / "goresave_core.dll",
                    bundle / "goresave_g1r_codec_host.exe",
                ],
            )
            self.assertEqual((bundle / "goresave_core.dll").read_bytes(), b"core")
            self.assertEqual(
                (bundle / "goresave_g1r_codec_host.exe").read_bytes(),
                b"host",
            )

    def test_missing_native_artifact_fails_before_packaging(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            app = root / "apps" / "goresave"
            bundle = app / "build" / "windows" / "x64" / "runner" / "Release"
            (root / "target" / "release").mkdir(parents=True)
            bundle.mkdir(parents=True)
            (root / "target" / "release" / "goresave_core.dll").write_bytes(b"core")
            (bundle / "goresave.exe").write_bytes(b"app")

            with self.assertRaises(FileNotFoundError) as err:
                build_native.copy_native_artifacts_to_windows_bundle(
                    root=root,
                    app=app,
                    profile="release",
                )

            self.assertIn("goresave_g1r_codec_host.exe", str(err.exception))


if __name__ == "__main__":
    unittest.main()
