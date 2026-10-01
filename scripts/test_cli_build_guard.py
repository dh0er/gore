"""Compile the real CLI gate in each build mode without rebuilding the workspace."""

import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest


ROOT = Path(__file__).resolve().parent.parent
GATE = ROOT / "crates/gore/src/product_build.rs"


class CliBuildGuardTests(unittest.TestCase):
    def probe(self, *, catalog=False, development=False, debug=False, harness=False):
        rustc = shutil.which("rustc")
        if rustc is None:
            self.skipTest("Rust toolchain is required for compile-fail tests")
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            source = root / "probe.rs"
            # Model only the linked catalog bytes; compile the actual product gate unchanged.
            source.write_text(
                'extern crate self as gore_as;\n'
                'pub mod standalone_package {\n'
                'pub const EMBEDDED_PRODUCT_STANDALONE_COMPILER_CATALOG_JSON_V1: &[u8] = '
                + ('b"catalog"' if catalog else 'b""')
                + ';\n}\n'
                + f'#[path = "{GATE.as_posix()}"] mod product_build;\n'
                + 'fn main() { println!("{}", product_build::VERSION); }\n',
                encoding="utf-8",
            )
            output = root / ("probe.exe" if os.name == "nt" else "probe")
            command = [rustc, "--edition=2021", str(source), "-o", str(output),
                       "-C", f"debug-assertions={'yes' if debug else 'no'}"]
            if development:
                command += ["--cfg", 'feature="development-cli"']
            if harness:
                command += ["--test"]
            result = subprocess.run(command, capture_output=True, text=True,
                                    env={**os.environ, "CARGO_PKG_VERSION": "0.4.0"})
            version = ""
            if result.returncode == 0 and not harness:
                version = subprocess.check_output([str(output)], text=True).strip()
            return result, version

    def test_catalogless_normal_builds_fail_with_product_command(self):
        for debug in (False, True):
            with self.subTest(debug=debug):
                result, _ = self.probe(debug=debug)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn("GORE CLI build refused", result.stderr)
                self.assertIn("python build.py gore-cli dist", result.stderr)

    def test_development_feature_cannot_bypass_release_guard(self):
        result, _ = self.probe(development=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("GORE CLI build refused", result.stderr)

    def test_explicit_debug_build_is_marked_unbundled(self):
        result, version = self.probe(development=True, debug=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(version, "0.4.0-development-unbundled")

    def test_catalog_bound_builds_retain_product_version(self):
        for debug, development in ((False, False), (True, False), (False, True)):
            with self.subTest(debug=debug, development=development):
                result, version = self.probe(catalog=True, debug=debug, development=development)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(version, "0.4.0")

    def test_unit_test_harness_does_not_require_product_package(self):
        result, _ = self.probe(harness=True)
        self.assertEqual(result.returncode, 0, result.stderr)


if __name__ == "__main__":
    unittest.main()
