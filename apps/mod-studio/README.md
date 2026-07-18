# GORE Mod Studio

> **Development status:** Mod Studio has not been released. There are no active
> legacy user projects; Managed R3 is the sole product project model and
> Snapshot V2 is the sole backup/restore format.

A no-code Windows GUI under active development for *authoring* one Gothic 1
Remake mod, over the same Rust mod engine as the
[`gore` CLI](../../README.md#modding-with-the-gore-cli) (via a `dart:ffi`
bridge, `gore_ffi.dll`). The current Managed-R3 surface covers NPC, Quest,
Dialog/Voice, localization, and DataAsset workflows plus installed Item and
Texture browsing. General Audio/Script authoring, vanilla localization and
runtime-topic patches, Item/Texture mutation, and the complete Managed-R3
build/deploy path are still being implemented and must not be treated as
released capabilities. See the
[repo README](../../README.md#gore-mod-studio) for the detailed development
status.

## Bundled CLI

Development builds of the installer (and portable zip) include the standalone
**`gore.exe`** CLI beside the app, plus its `shared/` Lua SDK. It gives you the
power tools the GUI does not surface — `gore as disasm`/`decompile` (deep AngelScript RE),
`catalog`/`dump`/`stubs` (data-model regen), and `mgr` (multi-mod management).
Open a terminal in the install dir and run `gore --help`.

The release infrastructure is configured to update the bundled copy with
Studio; the CLI can also be released on its own (`gore-cli-v*`) for
terminal/CI-only use.

## Build / run

Driven by the top-level orchestrator (see the [repo README](../../README.md#build)):

```sh
python build.py gore-mod run          # build (if needed) + launch
python build.py gore-mod dist         # portable zip (incl. bundled gore.exe)
python build.py gore-mod installer    # Windows installer
python build.py gore-mod test         # cargo (gore-ffi) + flutter analyze + test
```
