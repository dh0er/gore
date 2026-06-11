# goresave

`goresave` is a Windows desktop savegame editor for Gothic Remake. It provides
a Flutter interface backed by a Rust savegame core, with backup-first writes for
supported save metadata and structured views into parsed save data.

## Features

- Browse Gothic Remake save slots from a selected save directory.
- Read GSAV container metadata and PersistentDataList slot details.
- Edit supported public save metadata with automatic backup creation.
- Synchronize edited slot names to the matching PersistentDataList entry.
- Inspect parsed save data in overview, player, inventory, progression, and JSON views.
- Configure optional local helper paths for advanced private payload support.
- List and restore slot backups created by the editor.

## Installation & Updates

Download `Goresave-win-Setup.exe` from the
[latest release](https://github.com/dh0er/goresave/releases/latest) and run it.
The app checks GitHub Releases on startup, downloads updates in the background,
and applies them when you click "Restart to update".

A portable zip is also attached to each release; the portable build does not
auto-update.

### Releasing (maintainers)

1. Set `version:` in `apps/goresave/pubspec.yaml` to the new `X.Y.Z`.
2. Commit, then `git tag vX.Y.Z && git push origin vX.Y.Z`.
3. The Release workflow builds the zip + Velopack packages and publishes the
   GitHub release. The tag must match the pubspec version or the build fails.

## Project Layout

- `apps/goresave`: Flutter Windows application.
- `crates/goresave_core`: Rust save parsing, writing, backup, and FFI layer.
- `crates/goresave_g1r_codec_host`: Optional out-of-process helper.
- `fixtures`: Small test fixtures that are safe to commit.
- `integration_test`: Manual integration test notes.
- `tools`: Build and packaging helpers.

## Requirements

- Windows 10 or newer.
- Rust toolchain.
- Flutter with Windows desktop support.
- Visual Studio 2022 with Desktop development with C++.

The local development setup used for this repository is Flutter 3.44.0, Dart
3.12.0, and Rust 1.96.0.

## Build And Test

Run the repository test suite:

```powershell
python test.py
```

Build the Rust native artifacts:

```powershell
cargo build
```

Run the Flutter app:

```powershell
cd apps\goresave
flutter run -d windows
```

Build a Windows release bundle:

```powershell
cargo build --release
cd apps\goresave
flutter build windows --release
cd ..\..
python tools\build_native.py --release --bundle-windows
```

Release bundles expect `goresave_core.dll` and
`goresave_g1r_codec_host.exe` next to `goresave.exe`.

## Safety

`goresave` writes backups before modifying save files. Even so, keep your own
copy of important saves before editing them. Do not commit personal save files;
the repository only keeps small fixtures intended for tests.

## License

This project is licensed under the MIT License. See [LICENSE](LICENSE).
