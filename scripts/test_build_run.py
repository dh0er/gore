"""Launching an existing app must refresh both its UI and native core."""

from pathlib import Path
import sys
import tempfile
import unittest
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
import build as gore_build


class RunBuildTest(unittest.TestCase):
    def test_build_run_builds_once_before_launching(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            bundle = root / 'apps/save-editor/build/windows/x64/runner/Debug'
            bundle.mkdir(parents=True)
            exe = bundle / 'goresave.exe'
            exe.write_bytes(b'stale runner')
            events = []

            def build(project, release, dry):
                self.assertEqual((project, release, dry), ('gore-save-editor', False, False))
                events.append('build')
                exe.write_bytes(b'current runner')

            def launch(args, **kwargs):
                self.assertEqual(Path(args[0]), exe)
                self.assertEqual(exe.read_bytes(), b'current runner')
                events.append('launch')

            with (
                mock.patch.object(gore_build, 'ROOT', root),
                mock.patch.object(gore_build.sys, 'platform', 'win32'),
                mock.patch.object(gore_build, 'build_project', side_effect=build),
                mock.patch.object(gore_build.subprocess, 'Popen', side_effect=launch),
                mock.patch.object(sys, 'argv', ['build.py', 'gore-save-editor', 'build', '--debug', '--run']),
            ):
                self.assertEqual(gore_build.main(), 0)
            self.assertEqual(events, ['build', 'launch'])

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
                    mock.patch.object(gore_build.sys, 'platform', 'win32'),
                    mock.patch.object(gore_build, 'run', side_effect=compile_step),
                    mock.patch.object(gore_build, 'discard_line_ending_only_churn'),
                    mock.patch.object(gore_build, 'resolve_git_sha', return_value='test-sha'),
                    mock.patch.object(gore_build.subprocess, 'Popen', side_effect=launch) as process,
                ):
                    gore_build.run_project('gore-save-editor', release=False)
                    process.assert_called_once()

    def test_linux_save_editor_uses_linux_bundle_and_shared_library_names(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            with (
                mock.patch.object(gore_build, 'ROOT', root),
                mock.patch.object(gore_build.sys, 'platform', 'linux'),
                mock.patch.object(gore_build.platform, 'machine', return_value='x86_64'),
            ):
                self.assertEqual(
                    gore_build.flutter_build_dir('gore-save-editor', release=False),
                    root / 'apps/save-editor/build/linux/x64/debug/bundle',
                )
                self.assertEqual(
                    gore_build.runnable_exe('gore-save-editor', release=False),
                    root / 'apps/save-editor/build/linux/x64/debug/bundle/goresave',
                )
                self.assertEqual(
                    gore_build.core_library_filename('gore-save-editor'),
                    'libgore_save.so',
                )

    def test_linux_save_editor_bundle_follows_an_arm64_host(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            with (
                mock.patch.object(gore_build, 'ROOT', root),
                mock.patch.object(gore_build.sys, 'platform', 'linux'),
                mock.patch.object(gore_build.platform, 'machine', return_value='aarch64'),
            ):
                self.assertEqual(
                    gore_build.flutter_build_dir('gore-save-editor'),
                    root / 'apps/save-editor/build/linux/arm64/release/bundle',
                )

    def test_linux_build_builds_runner_and_stages_shared_library(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            bundle = root / 'apps/save-editor/build/linux/x64/debug/bundle'
            native_library = root / 'target/debug/libgore_save.so'
            commands = []

            def build_step(label, args, **kwargs):
                commands.append(args)
                if args[0] == gore_build.CARGO:
                    native_library.parent.mkdir(parents=True, exist_ok=True)
                    native_library.write_bytes(b'linux core')
                elif args[0] == gore_build.FLUTTER:
                    bundle.mkdir(parents=True, exist_ok=True)

            with (
                mock.patch.object(gore_build, 'ROOT', root),
                mock.patch.object(gore_build.sys, 'platform', 'linux'),
                mock.patch.object(gore_build.platform, 'machine', return_value='x86_64'),
                mock.patch.object(gore_build, 'run', side_effect=build_step),
                mock.patch.object(gore_build, 'discard_line_ending_only_churn'),
                mock.patch.object(gore_build, 'resolve_git_sha', return_value='test-sha'),
            ):
                gore_build.build_project('gore-save-editor', release=False, dry=False)

            self.assertEqual(commands[0][1:4], ['build', '-p', 'gore-save'])
            self.assertEqual(commands[1][1:4], ['build', 'linux', '--debug'])
            self.assertEqual((bundle / 'libgore_save.so').read_bytes(), b'linux core')


if __name__ == '__main__':
    unittest.main()
