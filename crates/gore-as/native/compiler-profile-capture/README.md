# G1R compiler-profile capture tooling

This directory contains the offline-buildable Windows-x64 capture lane for Steam BuildID
`24539464`. Its live controller is deliberately limited to the exact pinned executable and
production bridge: it can launch that executable windowed or attach to its exact running image,
install the closed hook table, seal the capture, restore every patched byte, unload the bridge,
and terminate the controller-owned launch. It is not a generic injector or detour tool.

## Produced targets

- `gore_as_compiler_profile_capture_bridge.dll` is the production in-process bridge. Its passive
  `DllMain` only disables thread notifications. It performs no target inspection, allocation,
  file I/O, hook installation, or synchronization under the loader lock.
- The same DLL contains an exact BuildID-specific instrumentation contract. It validates the
  loaded PE/CodeView identity, the exact nine instruction spans, all fourteen registration
  prologs/vtable entries and the three frontend callback CALL instructions. All nine transfer/frame ABIs,
  the exact fourteen central registration-entry contracts, target registration context/result
  projection, Build/JIT/compiler-flag extraction, frontend boundary frames, public AS 2.33 registry
  projection, target-witnessed bounded FinalState extraction, all-fourteen Delta JSON, HostStub
  support, trace/private-ID correlation, canonical FinalState JSON, target TypeUsage operation
  projection, preprocessor module/source/hash layout and transactional patch mechanics are
  statically implemented. The fourteen-frame observer also performs complete typed entry/
  result projection, public-flag filtering, recursive container/template validation, pointer-neutral
  HostStub derivation, Slot-16 storage equality and post-bind 1:1 enumeration. Frontend settings,
  pointer-neutral UClass/FName projection, all three pointer-neutral before/after callback
  observers, canonical config/graph digests and InitialCompile boundary mapping are also
  implemented. The bounded immutable-snapshot materializer now closes the pinned target layouts
  for UE arrays/sparse maps and sets, FString/FName, shared ownership, UObject/UClass,
  FFile/TChunkedArray and module/class descriptors. The same path validates both exact 24-byte
  graph-delegate objects and forces ProcessChunks/PostProcessCode to `bound=false` with empty
  captures; BuildID 24539464 has no binding xrefs, and any runtime-object drift is terminal.
  The bounded sparse CurrentProcess snapshot builder, direct 26-site state-preserving MASM/patch
  coordinator and single semantic observer/serializer/bridge phase machine are connected and
  synthetic-tested across seal, abort and exact uninstall.
  See [`INSTRUMENTATION.md`](INSTRUMENTATION.md).
- `gore_as_compiler_profile_capture.lib` remains the C++ writer used by the bridge.
- `gore_as_capture_live_controller.exe` is the BuildID-pinned launch/attach controller. Its
  `--capture-windowed` mode is the normal exclusive capture path; create-new output and the exact
  target/bridge identities are mandatory.
- `gore_as_capture_materializer.exe` validates a sealed capture wire and writes a deterministic,
  create-new audit summary.
- `gore_as_capture_offline_host.exe` is a synthetic self-host only. It has one mode and loads a
  test bridge into its **own** process; it cannot select or open another process.
- `gore_as_capture_bridge_fixture.dll` is built only with `BUILD_TESTING=ON`. It is a separately
  named test byte image and is never an install/export artifact. Its only relaxation is the exact
  executable byte/CodeView check needed for the synthetic host. Source handle identity,
  pointer-neutral records, phase ordering, output safety, limits, and sealing remain active.

The bridge exports an ABI-v1 contract plus exactly nine hook observations. The table is fixed to
the target build and RVAs in `format.hpp`: engine property, bind callback call/return, build/JIT,
and the four frontend points. Its FNV fingerprint is an ABI drift detector; product authenticity
still comes from a catalogued final DLL byte image and the exact target checks in
`CaptureSession::open_pinned`.

## Attach/detach contract

An authorized in-process instrumentation layer must:

1. query ABI, target, hook-table version, hook count, and fingerprint;
2. pass the current primary module base (it must equal `GetModuleHandleW(nullptr)`), exact build
   ID, the queried hook-table identity, bounded path lengths, and a nonzero capture ID to
   `gore_as_capture_bridge_attach_v1`;
3. emit typed records through the bridge functions in `CAPTURE_POINTS.md` order;
4. let the coordinator seal/detach on the attaching thread, uninstall instrumentation on its
   owner thread, then call `gore_as_capture_bridge_prepare_unload_v1` immediately before unload.

Append calls are serialized and may originate on instrumentation callback threads. Seal or abort
is owner-thread-bound. `abort_and_detach` deliberately retains an unsealed diagnostic stream;
neither native nor Rust materialization accepts it as evidence. Abrupt DLL unload is outside the
contract and can only close the writer, leaving an unsealed file.
Successful `prepare_unload` is terminal for that loaded DLL image: subsequent bridge attaches and
instrumentation preflights are refused, closing the post-return unload race.

`open_pinned` independently verifies the on-disk/loaded primary-image file identity, Steam BuildID
`24539464`, executable length/SHA-256, PE image size, and CodeView RSDS GUID/age. Output is
`CREATE_NEW`, final-component no-follow, non-shareable, handle-resolved outside the executable
tree, and held by the exact creating handle. Unsafe-output cleanup failure is
`output_recovery_required`. Runtime pointers are never serialized; primary-image pointers become
RVA-backed tokens.

## Exact offline build and use

From the repository root in PowerShell:

```powershell
cmake -S crates/gore-as/native/compiler-profile-capture `
  -B target/compiler-profile-capture `
  -G "Visual Studio 17 2022" -A x64 -DBUILD_TESTING=ON
cmake --build target/compiler-profile-capture --config Release --parallel
ctest --test-dir target/compiler-profile-capture -C Release --output-on-failure
```

All native targets compile as C++20 MSVC x64 with `/W4 /WX /permissive- /utf-8` and the static
MSVC runtime. The bundled synthetic end-to-end invocation used by CTest is equivalent to:

```powershell
target/compiler-profile-capture/Release/gore_as_capture_offline_host.exe `
  --synthetic-e2e `
  (Resolve-Path target/compiler-profile-capture/Release/gore_as_capture_bridge_fixture.dll) `
  (Resolve-Path target/compiler-profile-capture/Release/gore_as_compiler_profile_capture_bridge.dll)
```

It audits the production contract/refusal first, exercises all-nine synthetic patch/restore,
rollback, prolog-drift, wrong-thread, unload and record-order gates, then performs two deterministic
fake captures, seal/detach, wire materialization, create-new collisions, wrong-thread detach,
abort/unsealed refusal, and hook-table drift refusal. Temporary artifacts are outside the
executable directory and are removed after the test.

For a separately authorized real capture, the offline materializer syntax is:

```powershell
target/compiler-profile-capture/Release/gore_as_capture_materializer.exe `
  C:\capture-evidence\build-24539464.capture `
  C:\capture-evidence\build-24539464.wire-summary.json
```

The corresponding exclusive windowed live capture is:

```powershell
target/compiler-profile-capture/Release/gore_as_capture_live_controller.exe `
  --capture-windowed `
  'C:\Program Files (x86)\Steam\steamapps\common\Gothic 1 Remake\G1R\Binaries\Win64\G1R-Win64-Shipping.exe' `
  (Resolve-Path target/compiler-profile-capture/Release/gore_as_compiler_profile_capture_bridge.dll) `
  C:\capture-evidence\build-24539464.capture `
  180
```

The output must not already exist. A failed or aborted run is retained only as unsealed
diagnostic evidence and is rejected by every materializer.

Both arguments are final-component no-follow. The summary is `CREATE_NEW`; an existing file is
never replaced. It records the target tuple, capture ID, sealed-stream digest and record-kind
counts plus the four target compiler-build flags under schema
`gore.as.capture-wire-materialization/v1` with scope
`wire_only_not_a_qualified_compiler_profile`.

## Artifact flow (important)

The native wire summary is an audit/operations artifact only. It is **not** consumed by the
compiler profile package and must never be treated as qualification evidence by itself.

The authoritative data flow is:

```text
BuildID-pinned bridge -> sealed *.capture -> Rust decode_capture_v1(bytes)
                                      \-> optional *.wire-summary.json (native audit only)

decode_capture_v1 -> typed engine properties + registration trace + post-bind snapshot
                  + typed frontend configs + build/JIT/frontend boundary evidence
                  -> gore-as-profile-materializer + pinned static support
                  -> sealed, unqualified CompilerProfileV1 directory
```

`decode_capture_v1` consumes the original sealed `.capture` bytes directly. It rejects missing
phases, unknown fields, target drift, pointer-bearing/out-of-order data, broken seals, incomplete
host-stub semantics, or a registry that fails the existing replay validators. The offline
[`gore-as-profile-materializer`](../../src/compiler_profile/capture/MATERIALIZER.md) consumes the
decoder's typed projection plus all pinned static support payloads, not the native summary. Its
output is forcibly `qualified=false`; no deployable compiler package exists until authorized
runtime values have been captured and differential qualification has passed.

## Exact remaining runtime boundary

Offline tooling is backed by deterministic fake-target coverage plus prior authorized real target
captures. After any mandatory capture-schema expansion, the following runtime/qualification cycle
must be repeated before promotion:

1. after separate authorization, load the final catalogued production bridge into the already
   running, pinned G1R process;
2. install all version-pinned observations atomically and extract the actual typed values;
3. run `decode_capture_v1` over the sealed capture offline;
4. materialize the decoder projections into a complete profile package;
5. run the game-oracle/differential qualification before cataloguing that package.

The production DLL exposes the exact BuildID-only contract with
`production_installable=1`; the separately named fixture bridge exposes `0`. Public install
validates the image/session and delegates every write to the atomic 26-site coordinator. Semantic
records remain buffered until successful frontend completion, then append in canonical order and
seal; any rejection aborts, and exact uninstall remains owner-thread-only. No game was started,
no DLL was loaded into G1R, and no production installation or save was touched by the offline
build/test commands above.
