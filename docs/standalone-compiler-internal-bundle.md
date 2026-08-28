# Standalone compiler build and qualification contract

The standalone AngelScript compiler is an internal component of GORE CLI and
GORE Mod Studio. It has no separate release, tag, download, or update channel.
GORE Save Editor and GORE Mod Manager do not contain it.

Two independent contracts are kept deliberately separate:

- **Compiler compatibility** is identified by the explicit semantic ABI from
  `--capabilities` (currently `gore-as-standalone-semantic-v1`) and the request
  and response protocol versions.
- **Artifact integrity** is the exact length and SHA-256 of the sidecar built
  for one product release. Release signing changes these bytes, so this seal is
  not a compiler-compatibility identifier.

The checked-in source asset therefore contains qualified profiles and evidence,
not a release compiler executable. A CLI or Studio build compiles the sidecar
from the same source revision, runs its native tests, verifies its semantic ABI,
and then composes the product bundle. `GORE_SIGN=1` signs that fresh sidecar once
before its final length and SHA-256 are written to the embedded product catalog.
Later directory-signing passes explicitly exclude it.

## What a product tag does

A `gore-cli-v*` tag first passes the normal repository CI gate. Its existing
`python build.py gore-cli dist` step then performs the complete compiler path:

1. verify and extract the checked-in qualified-profile pack;
2. configure and build the native sidecar in Release mode;
3. run the standalone-compiler CTest suite;
4. verify AMD64/static imports and the advertised semantic ABI and protocol;
5. sign the fresh sidecar exactly once when `GORE_SIGN=1`;
6. measure the final sidecar and generate the product catalog;
7. build and sign `gore.exe`, stage the compiler tree, and verify it again;
8. create and publish the normal CLI ZIP.

There is no separate compiler-promotion release and no generated compiler
binary is committed to the repository. A local build follows the same path but
leaves the sidecar unsigned unless signing was explicitly enabled. Native tests
never launch the game and never touch the game installation.

## Qualified-profile source asset

The repository carries a deterministic, sidecar-free pair:

```text
crates/gore-as/assets/standalone-compiler-qualified-profiles.zip
crates/gore-as/assets/standalone-compiler-qualified-profiles.json
```

The descriptor pins the archive length, SHA-256, compression, and complete file
count. The archive contains:

```text
qualified-profiles.json
profiles/build-<BuildID>-<CodeView GUID>/compiler-profile.json
profiles/build-<BuildID>-<CodeView GUID>/<sealed profile payloads>
profiles/build-<BuildID>-<CodeView GUID>/qualification-promotion-receipt.json
verification/full-tree/build-<BuildID>-<CodeView GUID>.json
UNREANGEL-LICENSE.md
SOURCE_INVENTORY.tsv
PROVENANCE.toml
```

The profile pack records one common qualification reference: the exact
historical sidecar length/SHA-256 and wire protocol that actually produced the
parity reports, plus the semantic ABI those results qualify. That historical
seal remains audit evidence. It is intentionally allowed to differ from the
exact release-sidecar seal in the product catalog.

Extraction and staging are bounded, no-follow, no-clobber operations. The Rust
typed-profile verifier reloads every profile and its sealed payloads. Unknown,
missing, duplicated, case-aliased, or modified files are rejected.

To create a reviewed replacement pack after a real qualification, build the
typed verifier and use the profile-pack command:

```powershell
cargo build -p gore-as --release --bin gore-as-qualified-profile-verifier

python scripts/standalone_compiler_bundle.py pack-qualified-profiles `
  --qualified-profile-root C:\absolute\qualified-build-24539464 `
  --full-tree-receipt C:\absolute\full-tree-build-24539464.json `
  --qualified-profile-root C:\absolute\qualified-build-24878692 `
  --full-tree-receipt C:\absolute\full-tree-build-24878692.json `
  --qualified-profile-verifier C:\absolute\gore-as-qualified-profile-verifier.exe `
  --archive C:\absolute\new-assets\standalone-compiler-qualified-profiles.zip `
  --descriptor-output C:\absolute\new-assets\standalone-compiler-qualified-profiles.json
```

Profile and receipt arguments pair by occurrence. The pack may contain more
than two compatible generations later; runtime selection remains structural.

## When requalification is required

Changing the CLI version, documentation, packaging, signing certificate, or
native toolchain output does not by itself change compiler semantics. These
changes may produce different EXE bytes while retaining the same compiler ABI.
They need native build/tests and final artifact verification, not a game launch.

Any observable parser, frontend, registry, code-generation, StringFactory,
StaticJIT, cache-writer, or native-diagnostic behavior change requires a new
semantic ABI and fresh qualification. For each affected profile:

1. run the frozen 27-case corpus against the embedded game compiler and the
   standalone candidate over identical stable inputs;
2. require identical accepted cache artifacts and native diagnostics;
3. promote and typed-reload the profile;
4. run one full-tree differential over one frozen source snapshot;
5. require exact WholeCache structural equality, zero semantic differences,
   and zero alignment loss.

A game update that changes only the ordered AngelScript API normally needs a
new profile at the existing compiler ABI. A speculative compiler-core edit is
not justified. The full-tree verifier remains an internal qualification tool;
its receipt is retained as audit evidence but is not rerun for every product
tag.

The game-backed half uses the normal installation guard, verified diagnostics
hook, restoration, and recovery paths. It refuses an ambient
`GORE_AS_DIAGNOSTICS_HOOK` override. The strict standalone half cannot launch
the game or mutate the installation.

## Product catalog and runtime checks

The catalog embedded in the signed GORE host binds:

- the exact final sidecar path, length, SHA-256, semantic ABI, and protocol;
- the historical qualification reference and its semantic ABI/protocol; and
- the exact manifest and profile hashes for every shipped game API profile.

Runtime first authenticates the actual sidecar bytes against their release
seal. It then requires the sidecar and qualification reference to advertise the
same semantic ABI and protocol, and requires the chosen profile's parity reports
to match the historical reference. A release rebuild or Authenticode signature
may therefore change the executable hash without pretending that new game
qualification occurred.

Game compatibility is not an allowlist of complete Steam or GOG binaries. A
target is accepted for standalone compilation only when all of these hold:

- the executable is a bounded valid Windows-x64 PE;
- the Shipping cache has the supported fully parsed format and build id; and
- the Binds cache parses completely and its ordered semantic API fingerprint
  and counts match exactly one qualified profile.

Full executable hashes, CodeView data, Steam build/depot values, and cache GUIDs
remain qualification provenance. Repacked executables, changed cache GUIDs, and
equivalent ANSI/UTF-16 Binds encodings can still qualify structurally. A real
format/API difference, corrupted package, wrong ABI, sidecar tamper, or
ambiguous profile match makes standalone unavailable.

`standalone-then-game` reports that reason before using the embedded game
compiler. Strict standalone never starts the game and returns the sidecar's
native file/line/column diagnostics. The optional diagnostics hook is a separate
capability used only by the game backend; its two AOB/structure checks are not
replaced by compiler versioning.
