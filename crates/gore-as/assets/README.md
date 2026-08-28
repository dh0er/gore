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

## Qualified standalone compiler profiles

`standalone-compiler-qualified-profiles.zip` and its adjacent JSON descriptor are GORE source
assets, not a downloadable compiler product. The deterministic archive contains the qualified
`24539464` and `24878692` profile trees, their audit receipts, and notices. It deliberately contains
no compiler executable and no release signature.

CLI and Mod Studio builds verify and expand this profile pack, build and test the native sidecar
from the current source revision, and compose the product bundle. The sidecar advertises the same
semantic compatibility id as the profile pack. A release signs the freshly built copy exactly once
and pins its final length/SHA-256 in the embedded catalog; a local build leaves it unsigned. Save
Editor and Mod Manager never build or stage it.

The historical sidecar hash in the qualification reports remains exact audit evidence, but it is
not used as a whole-EXE release allowlist. Runtime selection is distribution-neutral: a bounded
AMD64 game executable, a supported Shipping cache format, and the fully parsed ordered Binds API
fingerprint select the compatible game API. Incompatible or ambiguous inputs visibly fall back to
the compiler embedded in the game. See
[`docs/standalone-compiler-internal-bundle.md`](../../../docs/standalone-compiler-internal-bundle.md).
