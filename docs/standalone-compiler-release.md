# Standalone compiler product-bundle release contract

The product bundle has two deliberately separate lifecycles:

- a rare compiler promotion creates one signed, differentially qualified, immutable release input;
- ordinary GORE CLI and Mod Studio releases only verify and copy that exact input.

An ordinary GORE release never rebuilds, re-signs, re-qualifies, or edits the sidecar/profile. If
no promoted input is configured, the generated embedded catalog is zero bytes, the runtime reports
`BundleAbsent`, and packaging removes any stale `compiler/` directory.

## Offline source gates

Build and test the sidecar and the dormant capture tools in separate source-CMake trees:

```powershell
python scripts/standalone_compiler_bundle.py build-native `
  --build-root C:\absolute\scratch\gore-as-native-gates
```

This performs, independently for `native/standalone-compiler` and
`native/compiler-profile-capture`:

1. `cmake -S ... -B ... -A x64 -DBUILD_TESTING=ON`
2. `cmake --build ... --config Release`
3. `ctest --test-dir ... -C Release --output-on-failure`

The same combined source gate runs in normal Windows CI. It performs no signing, download, game
launch, injection, or install mutation.

## One-time promotion order

These steps run only when promoting a new compiler identity or qualified target profile.

1. Pass both source-CMake lanes above. Select the resulting Release sidecar as the sole
   distributable candidate.
2. Sign that previously unsigned candidate exactly once. The repository-owned
   production path is the manually dispatched
   `.github/workflows/standalone-compiler-promotion.yml` workflow on a branch
   whose head is the exact reviewed candidate commit. That workflow first
   requires the repository's reusable CI workflow, then reruns both native
   source lanes in the candidate job. GitHub Immutable Releases must already be
   enabled for the repository. Before Azure receives credentials, the workflow
   exclusively creates `gore-as-signing-claim-<exact commit>` with one
   `signing-claim.json` asset. It continues only after the normal Release API
   proves that the public prerelease and its exact commit tag are immutable and
   that the asset length/SHA-256 matches. An existing claim makes every rerun
   fail before Azure is called.

   Azure credentials and `GORE_SIGN` exist only in the one signing step; checkout,
   builds, tests and GitHub publication never receive them. After one successful
   signature the workflow creates a GitHub SLSA/Sigstore attestation over the
   final EXE, signed identity and source provenance. It verifies all three
   subjects against the exact repository, source commit, workflow path, workflow
   commit and a GitHub-hosted runner. The four files are then published together
   in the separate immutable prerelease `gore-as-promotion-<exact commit>`.
   Nothing is uploaded later, overwritten or deleted. Download those four files
   without modifying or signing the executable again.

   The equivalent explicit command below is reserved for an environment that
   already has the same normal `GORE_SIGN=1` Trusted Signing credentials:

   ```powershell
   python scripts/standalone_compiler_bundle.py sign-sidecar-once `
     --sidecar C:\absolute\promotion\gore-as-standalone-compiler.exe `
     --identity-output C:\absolute\promotion\signed-sidecar-identity.json
   ```

   The command used by the workflow refuses
   an input already containing Authenticode, checks x64/static system-DLL imports, and queries the
   exact candidate's offline `--capabilities` before signing. It then requires exactly one final
   PKCS#7 certificate-table entry, verifies the Windows signature, rechecks `--capabilities`, and
   records both the unsigned and final signed byte length/SHA-256 plus the FullGraph
   request/response protocol `2/1`. Both the Microsoft Trusted Signing client
   package and `Microsoft.Windows.SDK.BuildTools` package are fixed by version,
   package length and SHA-256. All 41 client-runtime files and the complete private
   x64 Signtool side-by-side closure are fixed individually by length/SHA-256,
   checked before extraction and held against replacement during every signing
   process. `PATH`, a host SDK and `SIGNTOOL` are never trusted. The target's
   single-link file identity is held from unsigned measurement through signature
   and final verification while permitting only the intended in-place write. The
   workflow also matches the copied publication candidate back to the signed seal
   before Authenticode verification and attestation. The separate provenance
   Schema 2 binds that identity to the repository,
   exact source commit, exact workflow commit, immutable claim tag, immutable
   promotion tag, numeric workflow run ID and attempt. Request v1
   remains a source-level legacy smoke only; it is not accepted in a product catalog. Regular release commands
   never call it.
3. Run the separately authorized differential qualification against these final signed bytes.
   Both parity reports must identify exactly the recorded length/SHA/protocol and the resulting
   `CompilerProfileV1` must be newly sealed with `qualification.qualified=true` through the typed
   promotion path. The promoted directory must also contain
   `qualification-promotion-receipt.json` and the sealed
   `embedded-qualification-artifacts.json` / `standalone-qualification-artifacts.json` archives.
   The receipt binds the original capture/materialization authority, both raw artifact manifests,
   every per-case cache seal, supplemental witnesses, the exact signed sidecar and all promoted
   profile files. There is currently no authorized production profile, so this is an honest
   remaining gate rather than an invented artifact.

   Use fresh output roots for both complete 26-case captures. The embedded command additionally
   takes the pinned game/capture/controller inputs documented by its `--help`/usage output; it must
   be run only in an authorized pristine-install window. After both capture commands succeed,
   reload and promote their exact disk artifacts with the typed Rust boundary:

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

   The promotion command refuses an existing output, a noncanonical corpus, either wrong backend,
   a missing/extra/drifting cache artifact, different source-profile/target/sidecar identities,
   any diagnostic, supplemental-witness, invoke or whole-cache semantic difference, and any typed
   profile reload failure. Never promote development runs made with the pre-signing binary.
4. Freeze the signed sidecar, qualified profile and required notices into a new release-input
   directory:

   ```powershell
   cargo build -p gore-as --release --bin gore-as-qualified-profile-verifier

   python scripts/standalone_compiler_bundle.py record-release-input `
     --signed-sidecar C:\absolute\promotion\gore-as-standalone-compiler.exe `
     --promotion-identity C:\absolute\promotion\signed-sidecar-identity.json `
     --promotion-provenance C:\absolute\promotion\source-provenance.json `
     --promotion-attestation C:\absolute\promotion\github-attestation.sigstore.json `
     --expected-repository dh0er/gore `
     --expected-commit <exact-40-character-commit> `
     --qualified-profile-root C:\absolute\qualification\build-24539464 `
     --qualified-profile-verifier C:\sbx\goresave\target\release\gore-as-qualified-profile-verifier.exe `
     --github-attestation-verifier 'C:\Program Files\GitHub CLI\gh.exe' `
     --output C:\absolute\immutable\standalone-compiler-release-input-v1
   ```

   `--output` must not exist. The recorder first requires all three promotion
   proofs, checks their schemas and cross-links, matches their signed seal to the
   exact sidecar, and cryptographically verifies the Sigstore bundle for all
   subjects against the authorized repository, source commit, workflow path and
   workflow commit. It refuses self-asserted or different provenance. It then
   copies the exact manifest, its 16 referenced payloads
   and the three promotion-audit files. Before accepting them it independently recomputes the
   promotion receipt, both artifact-manifest seals, per-case cache-seal and supplemental-evidence
   summaries, and verifies both parity reports against the final signed sidecar. It also pins the
   full target tuple and CodeView identity. After the create-new profile tree has been copied, the
   mandatory Rust verifier reopens it no-follow through
   `verify_qualified_profile_package_v1`, validates every typed payload/cross-link and must return
   the exact catalog `profile_sha256` plus a domain-separated tree seal over the manifest, all 16
   payloads, both artifact manifests and the promotion receipt. The verifier executable and every
   ancestor in its path are held against write/delete/rename while it runs; a Python-only
   self-consistent profile is never sufficient.
   The recorder builds the complete result in a private sibling directory,
   verifies it there, atomically publishes without replacement and verifies the
   final path again. It copies the three promotion proofs,
   `UNREANGEL-LICENSE.md`, `SOURCE_INVENTORY.tsv` and `PROVENANCE.toml`. A failed
   typed, signature or publication check leaves no partial final directory. More
   profiles can be supplied by repeating `--qualified-profile-root`.
5. Pack the verified directory as the one canonical, uncompressed classic-ZIP dialect and create
   its independent asset descriptor:

   ```powershell
   python scripts/standalone_compiler_bundle.py pack-release-input `
     --release-input C:\absolute\immutable\standalone-compiler-release-input-v1 `
     --qualified-profile-verifier C:\sbx\goresave\target\release\gore-as-qualified-profile-verifier.exe `
     --github-attestation-verifier 'C:\Program Files\GitHub CLI\gh.exe' `
     --archive C:\absolute\release\gore-as-standalone-compiler-build-24539464-v1.zip `
     --descriptor-output C:\absolute\release\standalone-compiler-release-asset.json `
     --repository dh0er/gore `
     --tag gore-as-standalone-compiler-build-24539464-v1
   ```

   The writer streams every input, fixes timestamps/order/permissions, forbids ZIP64,
   compression, extras, comments and data descriptors, and compares every local header with its
   central header. Prefixes, suffixes, gaps, overlaps, duplicate/case-alias names and unsafe paths
   fail closed. Archive and descriptor are prepared privately and published without replacement
   as one pair; losing either output race rolls back only the file identity created by this run.
6. Publish that ZIP once under its unique release tag without clobbering, then commit the
   independently reviewed descriptor so later product releases have a non-circular SHA-256 and
   length pin. A consumer must download the descriptor-named asset and use:

   ```powershell
   python scripts/standalone_compiler_bundle.py extract-release-input `
     --archive C:\absolute\download\gore-as-standalone-compiler-build-24539464-v1.zip `
     --asset-descriptor C:\sbx\goresave\crates\gore-as\assets\standalone-compiler-release-asset.json `
     --expected-repository dh0er/gore `
     --qualified-profile-verifier C:\sbx\goresave\target\release\gore-as-qualified-profile-verifier.exe `
     --github-attestation-verifier 'C:\Program Files\GitHub CLI\gh.exe' `
     --output C:\absolute\download\verified-release-input
   ```

   Extraction pins one unchanged archive path across its SHA-256, raw ZIP parser
   and standard ZIP parser, streams into a private sibling directory, performs
   the full Authenticode/Rust/Sigstore/profile/file-set verification there, atomically publishes it without
   replacement, and repeats the complete verification through the final path. The resulting
   directory is the input to later GORE releases; do not sign or qualify again merely because the
   CLI/Studio version changed.

The promotion command explicitly rejects the retired 273,408-byte sidecar even if a local
descriptor attempts to self-pin it.

## Ordinary CLI/Studio release order

Point the build at the immutable input before starting either host build:

```powershell
$env:GORE_STANDALONE_COMPILER_RELEASE_INPUT = `
  'C:\absolute\immutable\standalone-compiler-release-input-v1'

python build.py gore-cli dist
python build.py gore-mod-studio installer
```

`build.py` then performs this fixed order:

1. builds the small Rust typed-profile verifier from the same source tree, then independently
   copies and measures the installed GitHub CLI as the Sigstore verifier, then
   no-follow verifies `release-input.json`, the one signed sidecar, every qualified
   manifest and referenced payload, qualification-sidecar identity, exact target/CodeView tuple,
   file set, licenses and notices; every profile must also reproduce the Rust-validated tree seal;
2. generates `catalog.json` and a clean staging tree under `target/` before invoking Cargo;
3. passes the generated catalog through `GORE_STANDALONE_COMPILER_CATALOG_PATH`; `gore-as/build.rs`
   copies those exact bytes into `OUT_DIR`, and the product catalog constant embeds them;
4. builds the CLI executable or Studio native host DLL (and Studio's companion CLI) with that
   catalog already embedded;
5. stages the same verified `compiler/` tree beside the CLI and Studio hosts;
6. signs the newly built GORE-owned host PE files, explicitly excluding
   `gore-as-standalone-compiler.exe` so its qualification-bound bytes cannot receive a second
   signature;
7. independently verifies the staged compiler tree again after host signing;
8. creates the portable archive and, for Studio, the Inno installer from that same Release tree.

The CLI and Studio bundle bytes are therefore byte-identical for a given release input. The
portable and installer paths do not have separate compiler-copy or signing recipes.

## Bundle layout and independent verifier

The immutable input mirrors the eventual `compiler/` tree:

```text
release-input.json
gore-as-standalone-compiler.exe
promotion/signed-sidecar-identity.json
promotion/source-provenance.json
promotion/github-attestation.sigstore.json
profiles/build-<BuildID>-<CodeView GUID>/compiler-profile.json
profiles/build-<BuildID>-<CodeView GUID>/<16 manifest-referenced payloads>
profiles/build-<BuildID>-<CodeView GUID>/qualification-promotion-receipt.json
profiles/build-<BuildID>-<CodeView GUID>/embedded-qualification-artifacts.json
profiles/build-<BuildID>-<CodeView GUID>/standalone-qualification-artifacts.json
UNREANGEL-LICENSE.md
SOURCE_INVENTORY.tsv
PROVENANCE.toml
```

The staged tree additionally contains byte-identical `compiler-bundle-manifest.json` descriptor
bytes and the generated `catalog.json`. Verify an exploded staged tree independently of the Rust
resolver with:

```powershell
python scripts/standalone_compiler_bundle.py verify `
  --bundle-root C:\absolute\host-stage\compiler `
  --qualified-profile-verifier C:\sbx\goresave\target\release\gore-as-qualified-profile-verifier.exe `
  --github-attestation-verifier 'C:\Program Files\GitHub CLI\gh.exe'
```

The verifier rejects reparse points, symlinks, hard links, case aliases, unknown/missing files,
unbounded inputs, catalog/manifest/payload drift, unqualified profiles, target drift, parity bound
to another sidecar/protocol, missing or forged signing identity, self-asserted or
cryptographically invalid commit/workflow/run provenance,
promotion receipts or artifact authorities, non-x64
sidecars, non-system DLL imports, absent/multiple/malformed Authenticode entries, invalid Windows
signatures, and missing or changed notices.

## Honest absent state and remaining release gate

Without `GORE_STANDALONE_COMPILER_RELEASE_INPUT`, `build.py` actively creates an empty embedded
catalog and removes stale `compiler/` trees before packaging. It never scans build directories for
a convenient sidecar and cannot silently pick up an older binary.

The remaining production gate is concrete: authorize one real final sidecar signature, execute
differential qualification for BuildID `24539464` against that signed identity, review the
qualified payloads, and record the first immutable release input. Until that exists, shipped
products correctly remain `BundleAbsent`; the tooling and synthetic fixtures do not manufacture a
99%-qualified substitute.
