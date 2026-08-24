# AngelScript diagnostics hook

`gore-as-diagnostics-hook.dll` is embedded into `gore` and materialized only for the lifetime of a
compiler run. It is never installed into Gothic, UE4SS, or the game directory.

- SHA-256: `3D9852ED4A077C0B987A290FD1B349A92AF394FEEF297A33165464B2E4C2E39D`
- Source/build recipe: `crates/gore-as/native/diagnostics-hook/`
- Vendored MinHook/HDE source and required BSD notice:
  `crates/gore-as/native/diagnostics-hook/vendor/minhook/LICENSE.txt`
- Release notice: root `THIRD_PARTY_LICENSES.md`
- Deterministic Windows rebuild and asset refresh:
  `powershell -File crates/gore-as/native/diagnostics-hook/build.ps1 -UpdateEmbeddedAsset`

The Rust launcher and DLL independently require exactly one raw masked-AOB match for both the
per-message `LogAngelscriptError` callback and the structured
`FAngelscriptManager::ScriptCompileError` boundary. They verify matching sparse fingerprints for
all consumed `asSMessageInfo`, `FString`, and `FDiagnostic` fields. Both sides require the AMD64
machine type and PE32+ optional-header magic, and both checks stay in the same raw-backed byte range
of the image's exact `.text` section. A signature/structure mismatch, ambiguous match, incomplete
capture, or unsupported image disables acceptance of diagnostic output; safe infrastructure
failures fall back to the ordinary game compiler.
