"""Launching an existing app must refresh both its UI and native core."""

from pathlib import Path
import sys
import tempfile
import unittest
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
import build as gore_build


class RunBuildTest(unittest.TestCase):
    def test_existing_exe_launches_with_current_ui_and_core(self):
        for existing_core in (False, True):
            with self.subTest(existing_core=existing_core), tempfile.TemporaryDirectory() as tmp:
                root = Path(tmp)
                bundle = root / 'apps/save-editor/build/windows/x64/runner/Debug'
                bundle.mkdir(parents=True)
                (bundle / 'goresave.exe').write_bytes(b'existing runner')
                kernel = bundle / 'data/flutter_assets/kernel_blob.bin'
                kernel.parent.mkdir(parents=True)
                kernel.write_bytes(b'stale UI')
                if existing_core:
                    (bundle / 'gore_save.dll').write_bytes(b'stale core')

                def compile_step(label, args, **kwargs):
                    if args[0] == gore_build.CARGO:
                        dll = root / 'target/debug/gore_save.dll'
                        dll.parent.mkdir(parents=True)
                        dll.write_bytes(b'current core')
                    elif args[0] == gore_build.FLUTTER:
                        kernel.write_bytes(b'current UI')
                    else:
                        self.fail(f'Unexpected build step: {label}')

                def launch(args, **kwargs):
                    self.assertEqual(Path(args[0]), bundle / 'goresave.exe')
                    self.assertEqual(kwargs['cwd'], bundle)
                    self.assertEqual(kernel.read_bytes(), b'current UI')
                    self.assertEqual((bundle / 'gore_save.dll').read_bytes(), b'current core')

                with (
                    mock.patch.object(gore_build, 'ROOT', root),
                    mock.patch.object(gore_build, 'run', side_effect=compile_step),
                    mock.patch.object(gore_build, 'discard_line_ending_only_churn'),
                    mock.patch.object(gore_build, 'resolve_git_sha', return_value='test-sha'),
                    mock.patch.object(gore_build.subprocess, 'Popen', side_effect=launch) as process,
                ):
                    gore_build.run_project('gore-save-editor', release=False)
                    process.assert_called_once()


if __name__ == '__main__':
    unittest.main()
