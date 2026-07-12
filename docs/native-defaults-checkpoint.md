# Native defaults checkpoint (2026-07-12)

This is a deliberately incomplete, fail-closed checkpoint. It preserves the parallel-agent work
without enabling deployment or save mutation. The scalar default patcher is complete in commit
`e31ef08`; the work below still needs the listed gates before it is production-ready.

## Preserved evidence

- Shipping script cache fingerprint format:
  `gore-as-default-cache-fingerprint-v2-scalar-tag`.
- Versioned combined fingerprint after one checked normalization pass over 26,339 direct-scalar
  operands and 1,432 exact reference-proven tag-map operands:
  `01fe4e37cc3a5dee15c2beb49a3f406110774b5e300f2de4ad811d0df9addd6b`.
- Binds raw SHA-256: `46e6629ad5cacc112b9922d48a1aa948f40572d7285705b981c3eca3dc615fea`.
- Sealed Binds AS-type to Unreal-path map: 11,193 rows; digest begins `cffbce6f`.
- USMAP raw SHA-256: `73558c36895cd1b0f0fd1b3cb44305b240f8dbb93730ad03c88d7b8478b7ffca`.
- USMAP class graph: 6,594 rows; digest
  `0e64322222d3d32c5cd41254532d518be5feb722a24ed0142284fa4ec91d679d`.
- Resolved Binds/USMAP class profile: 6,572 rows; digest begins `1763379b`.
- Runtime-derived and sealed atomic tuple ID including fingerprint format, combined digest, both
  operand counts, GUID, Binds identities, and USMAP identities:
  `sha256:98da5430f213b0107bd7361fa3c78316bf5320fbd15a53a9258d50d8d3ac9ed5`.

The current ancestry scaffold compiles and its configured profile/fingerprint tests passed. It
joins native ancestry only after the parsed script chain reaches an unresolved native terminal.

The cache-only GameplayTag-to-float32 scanner in `default_tag_map.rs` remains read-only. It
losslessly parses the exact tail identities and proves the generated initializer, `GameplayTag`
global, and `TMap<FGameplayTag,float32>::Add` signature. The only public promotion boundary is now
`inspect_native_tag_maps(cache, profile)`, which rebuilds the cache proof, binds full raw SHA/GUID
and combined-fingerprint provenance, and admits only exact declared native USMAP field shapes into
an opaque read-only report. The Shipping audit proves all 1,432 raw windows and eight distinct
native map fields; Sword remains unique. The sealed field-profile digest is
`5fa2e35616cb6b04a3060202e55ff575d8e8aeab5a25602aeddc10b3ad542708`, and its opaque proof ID is
`sha256:f20ce5ce571f3d121046ac1942e0705cfb30c3761a3e390cd5d77ea2c16159cc`.

Configured Shipping tests prove that changing a tag operand changes the legacy scalar-only digest
but preserves the combined fingerprint, permits ancestry reconstruction in a later pass, and
rediscovers all 1,432 opaque native sites with the changed expected bytes. Native scalar-default
inspection also retains the same sealed ancestry after that tag edit.

## Required before enabling native mutation

Ancestry tuple binding, selector v4, exact Class edges, content-sealed CLI USMAP discovery, and the
real Sword scalar recovery/patch/rediscovery test are complete in `40611d8` and `6658f80`.

The opaque cache/profile report, combined mutation-stable fingerprint, and post-tag reconstruction
and rediscovery gates are complete. Tag-map mutation still requires a semantic selector with
uniqueness rules, compare-and-swap and copy-on-write mutation, post-patch rebuild/reparse, CLI
wiring, a real end-to-end patch fixture, full regression tests, and the release build. A future
patch path must rebuild proof from its input cache and must never accept a retained inspection
report as write authority.

Unrelated generated Flutter localization changes and the pre-existing rustfmt-only `splice.rs`
change are intentionally excluded from this checkpoint.
