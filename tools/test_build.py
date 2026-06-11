from __future__ import annotations

import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

import build  # noqa: E402


class BuildDistTest(unittest.TestCase):
    def test_copy_license_to_bundle(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            release = Path(tmp)
            destination = build.copy_license_to_bundle(release)
            self.assertTrue(destination.exists())
            self.assertEqual(
                destination.read_text(encoding="utf-8"),
                build.LICENSE_FILE.read_text(encoding="utf-8"),
            )


if __name__ == "__main__":
    unittest.main()
