"""Launching an existing app must refresh both its UI and native core."""

import contextlib
from pathlib import Path
import sys
import tarfile
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


    def _linux_dist_fixture(self, root, *, core=True):
        bundle = root / 'apps/save-editor/build/linux/x64/release/bundle'
        (bundle / 'lib').mkdir(parents=True)
        (bundle / 'data').mkdir()
        exe = bundle / 'goresave'
        exe.write_bytes(b'elf')
        exe.chmod(0o755)
        (bundle / 'lib' / 'libapp.so').write_bytes(b'app')
        if core:
            (bundle / 'libgore_save.so').write_bytes(b'core')
        (root / 'LICENSE').write_text('MIT')
        (root / 'LICENSE').chmod(0o777)
        return bundle

    def _linux_dist_patches(self, root):
        return (
            mock.patch.object(gore_build, 'ROOT', root),
            mock.patch.object(gore_build.sys, 'platform', 'linux'),
            mock.patch.object(gore_build.platform, 'machine', return_value='x86_64'),
            mock.patch.object(gore_build, 'build_project'),
            mock.patch.object(gore_build, 'read_version', return_value='1.6.0'),
            mock.patch.object(gore_build, 'sign_paths', side_effect=AssertionError('signed on Linux')),
        )

    def test_linux_dist_packs_save_editor_bundle_as_tarball(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self._linux_dist_fixture(root)
            with contextlib.ExitStack() as stack:
                for patch in self._linux_dist_patches(root):
                    stack.enter_context(patch)
                archive = gore_build.dist_project('gore-save-editor', dry=False)

            top = 'gore-save-editor-1.6.0-linux-x64'
            self.assertEqual(archive, root / 'dist/gore-save-editor' / f'{top}.tar.gz')
            with tarfile.open(archive) as tar:
                members = {member.name: member for member in tar.getmembers()}
            self.assertTrue(all(name == top or name.startswith(top + '/') for name in members))
            for name in ('goresave', 'libgore_save.so', 'lib/libapp.so', 'LICENSE'):
                self.assertIn(f'{top}/{name}', members)
            self.assertEqual(members[f'{top}/goresave'].mode, 0o755)
            self.assertEqual(members[f'{top}/LICENSE'].mode, 0o644)
            self.assertEqual(members[f'{top}/lib'].mode, 0o755)
            self.assertEqual(members[f'{top}/lib/libapp.so'].mode, 0o755)
            self.assertEqual({(m.uid, m.gid, m.uname, m.gname) for m in members.values()}, {(0, 0, '', '')})
            self.assertEqual(
                sorted(path.name for path in (root / 'dist/gore-save-editor').iterdir()),
                [f'{top}.tar.gz'],
            )

    def test_linux_dist_refuses_a_bundle_without_its_native_core(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self._linux_dist_fixture(root, core=False)
            with contextlib.ExitStack() as stack:
                for patch in self._linux_dist_patches(root):
                    stack.enter_context(patch)
                with self.assertRaisesRegex(SystemExit, 'libgore_save.so'):
                    gore_build.dist_project('gore-save-editor', dry=False)
            self.assertEqual(list((root / 'dist/gore-save-editor').iterdir()), [])

    def test_linux_dist_refuses_windows_only_products(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            with contextlib.ExitStack() as stack:
                for patch in self._linux_dist_patches(root):
                    stack.enter_context(patch)
                for project in ('gore-cli', 'gore-mod-studio', 'gore-mod-manager'):
                    with self.subTest(project=project):
                        with self.assertRaisesRegex(SystemExit, 'no Linux package'):
                            gore_build.dist_project(project, dry=False)


    def test_linux_prefers_native_tools_over_windows_ones_on_path(self):
        found = {'cargo.exe': '/mnt/c/cargo.exe', 'cargo': '/home/u/.cargo/bin/cargo'}
        with (
            mock.patch.dict(gore_build.os.environ, {}, clear=True),
            mock.patch.object(gore_build.shutil, 'which', side_effect=found.get),
        ):
            with mock.patch.object(gore_build.sys, 'platform', 'linux'):
                self.assertEqual(
                    gore_build._resolve_tool('CARGO', ['cargo.exe', 'cargo'], Path('x')),
                    Path('/home/u/.cargo/bin/cargo'),
                )
            with mock.patch.object(gore_build.sys, 'platform', 'win32'):
                self.assertEqual(
                    gore_build._resolve_tool('CARGO', ['cargo.exe', 'cargo'], Path('x')),
                    Path('/mnt/c/cargo.exe'),
                )


if __name__ == '__main__':
    unittest.main()
