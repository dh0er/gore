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

## Internal standalone compiler package

`standalone-compiler-internal-package.zip` and its adjacent JSON descriptor are GORE source
assets, not a separate downloadable product. The archive contains the already signed standalone
compiler plus its qualified game-API data. It is deterministic Deflate-9 so the roughly 152 MiB
qualified tree occupies about 7.5 MiB in source control; `build.py` verifies and expands it only
for `gore-cli` and `gore-mod-studio`. Save Editor and Mod Manager never stage it.

Normal builds use the descriptor's exact length/SHA-256 as their source-tree authority and need no
network access or GitHub release. Package creation is a separate one-time operation which verifies
Authenticode, typed profile data, and GitHub/Sigstore provenance before publication into this
directory. This replacement creates no compiler release or tag. Runtime selection is
distribution-neutral: whole Steam/GOG files and store metadata do
not gate use. A bounded AMD64 game executable, a supported Shipping cache format, and the fully
parsed ordered Binds API fingerprint select the compatible game API. Incompatible or ambiguous
inputs visibly fall back to the compiler embedded in the game.
