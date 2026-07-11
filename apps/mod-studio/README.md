# GORE Mod Studio

A no-code Windows GUI for *authoring* one Gothic 1 Remake mod, over the same
Rust mod engine as the [`gore` CLI](../../README.md#modding-with-the-gore-cli)
(via a `dart:ffi` bridge, `gore_ffi.dll`). Edit item/stat values, localization
IDs and dialog lines, audio, textures, and AngelScript modules, then build a
bundle and deploy/undeploy it to your install. The Dialog view also stages
selectable conversation-topic registration with explicit participant, authored
AngelScript class, and vanilla sentinel identities; it emits the same
`BuildSpec.dialog_topics` representation as the CLI without guessing them.
See the [repo README](../../README.md#gore-mod-studio) for the full capability
list.

## Bundled CLI

The installer (and portable zip) ship the standalone **`gore.exe`** CLI beside
the app, plus its `shared/` Lua SDK. It gives you the power tools the GUI does
not surface — `gore as disasm`/`decompile` (deep AngelScript RE),
`catalog`/`dump`/`stubs` (data-model regen), and `mgr` (multi-mod management).
Open a terminal in the install dir and run `gore --help`.

The bundled copy updates with Studio; the CLI is also released on its own
(`gore-cli-v*`) for terminal/CI-only use.

## Build / run

Driven by the top-level orchestrator (see the [repo README](../../README.md#build)):

```sh
python build.py gore-mod run          # build (if needed) + launch
python build.py gore-mod dist         # portable zip (incl. bundled gore.exe)
python build.py gore-mod installer    # Windows installer
python build.py gore-mod test         # cargo (gore-ffi) + flutter analyze + test
```
