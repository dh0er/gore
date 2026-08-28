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
assets, not a separate downloadable product. The currently checked-in, already signed archive is
the legacy internal-input V1 package for build `24539464`. `build.py` may verify and expand that
exact pinned archive for local `gore-cli` and `gore-mod-studio` builds, so ordinary development
does not require a premature replacement signature. Save Editor and Mod Manager never stage it.

Normal builds use the descriptor's exact length/SHA-256 as their source-tree authority and need no
network access or GitHub release. Package creation is a separate one-time operation which verifies
Authenticode, typed profile data, and GitHub/Sigstore provenance before publication into this
directory. Internal-input V2 additionally requires exactly the `24539464` and `24878692` product
profiles, both bound to the same final sidecar, with exactly one strict full-tree receipt each.
Distribution packaging, installers, tags, and pushes reject the legacy V1 bridge; only a later,
explicitly authorized one-time signing/promotion may replace it with that V2 publishing set. This
publishing policy is intentionally separate from runtime admission. Runtime selection is
distribution-neutral: whole Steam/GOG files and store metadata do
not gate use. A bounded AMD64 game executable, a supported Shipping cache format, and the fully
parsed ordered Binds API fingerprint select the compatible game API. Incompatible or ambiguous
inputs visibly fall back to the compiler embedded in the game.
