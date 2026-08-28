# Standalone compiler internal bundle contract

The standalone AngelScript compiler is an internal GORE component. It is shipped
only with GORE CLI and GORE Mod Studio. It has no separate GitHub release, tag,
download page, or update channel. Save Editor and Mod Manager do not contain it.

There are two lifecycles:

- A rare signing and qualification run creates new trusted compiler bytes and
  the complete required set of matching qualified profiles.
- Normal CLI and Studio builds verify and copy the checked-in compressed package.
  They never rebuild, re-sign, or re-qualify the compiler.

The embedded game compiler remains available as an explicit backend and as the
visible fallback for `standalone-then-game`.

## Native source gate

Build and test the sidecar and capture helper in separate source trees:

```powershell
python scripts/standalone_compiler_bundle.py build-native `
  --build-root C:\absolute\scratch\gore-as-native-gates
```

This runs CMake, the Release build, and CTest for both native trees. It performs
no signing, download, game launch, injection, or game-install mutation. The same
gate runs in normal Windows CI.

## Internal signing

A new sidecar is signed by manually dispatching
`.github/workflows/standalone-compiler-promotion.yml` at the exact reviewed
source commit. Despite the historical filename, the workflow is an internal
signing job and is not a release workflow.

It performs this closed sequence:

1. Run the exact normal CI workflow.
2. Rebuild and test the unsigned native candidate.
3. Sign that candidate once with Azure Trusted Signing.
4. Record the unsigned and signed length/SHA-256 plus protocol versions.
5. Create and locally verify a GitHub Sigstore attestation for the signed EXE,
   signing identity, and source provenance.
6. Upload those four files as one immutable workflow artifact retained for
   30 days.

The job has `contents: read`, not `contents: write`. It contains no release
or tag command. Azure credentials and `GORE_SIGN=1` exist only in the signing
step. Checkout and build steps receive neither Azure credentials nor a GitHub
write token.

The workflow artifact name contains the exact source commit, run ID, and attempt:

```text
gore-as-signed-candidate-<commit>-<run-id>-<attempt>
```

Download it with the GitHub CLI:

```powershell
gh run download <run-id> `
  --repo dh0er/gore `
  --name gore-as-signed-candidate-<commit>-<run-id>-<attempt> `
  --dir C:\absolute\signed-candidate
```

The signing command rejects a sidecar that is already signed, checks the
Windows-x64/static-system-DLL contract, verifies the exact candidate's
capabilities before and after signing, and requires one valid Authenticode
certificate-table entry. The signing toolchain and all of its private runtime
files are version-, length-, and SHA-256-pinned.

## Differential qualification

The final signed bytes must pass the complete canonical qualification corpus
separately for every product target against the embedded compiler from the
authorized private game copy. Use fresh output directories. Every standalone
and game-backed pair must contain the same 27 cases, including the Diego
dialog-authoring example, the same accepted cache artifacts, and no unexplained
diagnostic, frontend, bytecode, module-graph, invocation, or whole-cache
difference.

After both captures pass, create the qualified profile through the typed Rust
boundary:

```powershell
cargo build -p gore-as --release `
  --bin gore-as-standalone-qualification-capture `
  --bin gore-as-embedded-qualification-capture `
  --bin gore-as-promote-qualified-profile

target\release\gore-as-promote-qualified-profile.exe `
  C:\absolute\unqualified-profile `
  C:\absolute\embedded-qualification-output `
  C:\absolute\standalone-qualification-output `
  C:\absolute\new-qualified-profile
```

The promotion tool refuses an existing output, a changed corpus, the wrong
backend, missing or extra cache artifacts, different source/profile/sidecar
identities, any parity difference, and any typed profile reload failure.

The 27-case corpus is necessary but not the release-scale gate. For each
profile, copy the complete source tree to a stable location outside the game
installation, produce one embedded reference cache from those exact bytes, and
run the internal full-tree verifier with the same frozen source:

```powershell
cargo build -p gore-as --release --bin gore-as-full-tree-verifier

target\release\gore-as-full-tree-verifier.exe `
  C:\absolute\signed-candidate\gore-as-standalone-compiler.exe `
  C:\absolute\qualified-profile `
  C:\absolute\game-root `
  C:\absolute\game-root\G1R\Binaries\Win64\G1R-Win64-Shipping.exe `
  C:\absolute\game-root\G1R\Script\PrecompiledScript_Shipping.Cache `
  C:\absolute\game-root\G1R\Script\Binds.Cache `
  C:\absolute\frozen-full-source `
  C:\absolute\copied-embedded-reference.Cache `
  C:\absolute\standalone-work `
  C:\absolute\new-standalone-output.Cache `
  C:\absolute\full-tree-build-<BuildID>.json
```

The helper can invoke only the strict standalone backend. Its canonical receipt
binds the qualified profile SHA-256, the final sidecar length/SHA-256 and
protocol, Shipping and Binds seals, the frozen source aggregate, the copied
embedded reference and standalone candidate, and the complete WholeCache
semantic digest/counts. Publication requires exact WholeCache structural
equality, `semantic = 0`, and `alignment_loss = 0`. `benign` may be nonzero: it
remains visible and is accepted only through the pinned default normalizers.
Both output paths are create-new; the receipt is canonical UTF-8 JSON written
directly by the helper, not shell-redirection output.

## Record and compress the internal package

First copy the signed, attested, qualified bytes into one create-new verified
intermediate directory:

```powershell
cargo build -p gore-as --release --bin gore-as-qualified-profile-verifier

python scripts/standalone_compiler_bundle.py record-internal-input `
  --signed-sidecar C:\absolute\signed-candidate\gore-as-standalone-compiler.exe `
  --promotion-identity C:\absolute\signed-candidate\signed-sidecar-identity.json `
  --promotion-provenance C:\absolute\signed-candidate\source-provenance.json `
  --promotion-attestation C:\absolute\signed-candidate\github-attestation.sigstore.json `
  --expected-repository dh0er/gore `
  --expected-commit <exact-40-character-source-commit> `
  --qualified-profile-root C:\absolute\qualified-build-24539464 `
  --full-tree-receipt C:\absolute\full-tree-build-24539464.json `
  --qualified-profile-root C:\absolute\qualified-build-24878692 `
  --full-tree-receipt C:\absolute\full-tree-build-24878692.json `
  --qualified-profile-verifier C:\absolute\gore-as-qualified-profile-verifier.exe `
  --github-attestation-verifier 'C:\Program Files\GitHub CLI\gh.exe' `
  --output C:\absolute\internal-package-source
```

The profile-root and full-tree-receipt options pair by occurrence. The command
verifies Authenticode, all Sigstore subjects, source/workflow/run provenance,
the qualification receipt, both artifact manifests, every profile payload,
every full-tree receipt, and the complete expected file set. It accepts exactly
the required product targets shown below, with neither omissions nor additions,
and requires every qualification and full-tree receipt to identify the one
final sidecar in the catalog. It has no release or tag operation.

| required product target | depot manifest | CodeView GUID / age |
|---|---:|---|
| BuildID `24539464` | `1585071322101748861` | `cf0b83bd-e023-061b-2100-0f0fccf871d2` / `1` |
| BuildID `24878692` | `382135126159906494` | `c2ca4ada-4878-d963-e567-717dc2c483a2` / `1` |

This closed set is a publishing-completeness policy, not a runtime binary-hash
allowlist. Changing it requires an intentional package-contract review. The
runtime remains structurally/API qualified as described below.

Then replace the two checked-in internal assets with a newly generated pair:

```powershell
python scripts/standalone_compiler_bundle.py pack-internal-package `
  --internal-input C:\absolute\internal-package-source `
  --qualified-profile-verifier C:\absolute\gore-as-qualified-profile-verifier.exe `
  --github-attestation-verifier 'C:\Program Files\GitHub CLI\gh.exe' `
  --archive C:\absolute\new-assets\standalone-compiler-internal-package.zip `
  --descriptor-output C:\absolute\new-assets\standalone-compiler-internal-package.json
```

Copy the reviewed pair to:

```text
crates/gore-as/assets/standalone-compiler-internal-package.zip
crates/gore-as/assets/standalone-compiler-internal-package.json
```

The archive is deterministic Deflate with a canonical sorted file set. The
descriptor independently pins its filename, byte length, SHA-256, compression
mode, catalog SHA-256, and file count. Extraction is bounded, no-follow,
no-clobber, and followed by the complete typed verification again.

## Normal CLI and Studio builds

Without an override, `build.py` verifies and extracts the checked-in internal
package into a content-addressed work directory. Normal builds need neither
network access nor GitHub CLI.

The fixed build order is:

1. Build the small typed-profile verifier.
2. Verify and extract the internal archive.
3. Generate and embed the exact compiler catalog.
4. Build the CLI executable or Studio native host.
5. Stage the same verified `compiler/` tree beside the host.
6. Sign newly built GORE-owned host files, excluding the already signed sidecar.
7. Verify the staged compiler tree again.
8. Create the portable package and, for Studio, the installer.

CLI and Studio therefore receive byte-identical compiler files. Save Editor and
Mod Manager return before any compiler-package access and remove stale compiler
staging from their outputs.

A development-only `GORE_STANDALONE_COMPILER_INTERNAL_INPUT` override may point
at an exploded verified intermediate tree. Unlike the normal checked-in package,
this override also requires GitHub CLI for external attestation verification.

## Runtime compatibility and fallback

Runtime selection is deliberately not bound to one exact Steam executable.
Full EXE SHA-256, CodeView identity, Steam build/depot data, cache GUID, and
whole cache hashes are qualification provenance only.

A target is accepted for standalone compilation when all of these hold:

- the executable is a bounded valid Windows-x64 PE;
- the Shipping cache has the supported fully parsed format and build identifier;
- the Binds cache parses completely and its ordered semantic API fingerprint and
  counts match a qualified profile.

This allows differently packed Steam files, GOG builds, harmless one-byte EXE
changes, changed cache GUIDs, and ANSI/UTF-16 encodings of the same Binds API.
A real format or API difference, missing package, invalid signature, corrupted
profile, or ambiguous match makes standalone unavailable.

In `standalone-then-game` mode, that unavailability is reported and GORE then
uses the embedded game compiler. There is no silent fallback: the result records
which backend was attempted, why it changed, and which backend produced the
cache.

## Verified bundle layout

The expanded internal source contains:

```text
internal-input.json
gore-as-standalone-compiler.exe
promotion/signed-sidecar-identity.json
promotion/source-provenance.json
promotion/github-attestation.sigstore.json
profiles/build-<BuildID>-<CodeView GUID>/compiler-profile.json
profiles/build-<BuildID>-<CodeView GUID>/<profile payloads>
profiles/build-<BuildID>-<CodeView GUID>/qualification-promotion-receipt.json
profiles/build-<BuildID>-<CodeView GUID>/embedded-qualification-artifacts.json
profiles/build-<BuildID>-<CodeView GUID>/standalone-qualification-artifacts.json
verification/full-tree/build-<BuildID>-<CodeView GUID>.json
UNREANGEL-LICENSE.md
SOURCE_INVENTORY.tsv
PROVENANCE.toml
```

The verifier rejects links, reparses, hard links, case aliases, unknown or
missing files, unbounded inputs, seal drift, unqualified profiles, target/API
drift, parity bound to another sidecar, invalid Authenticode, invalid workflow
attestation, changed notices, a missing/additional product profile or full-tree
receipt, receipt/profile/Shipping/Binds/source/reference drift, non-standalone
execution, semantic differences, or alignment loss.
