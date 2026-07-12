# Native defaults checkpoint (2026-07-12)

This is a deliberately incomplete, fail-closed checkpoint. It preserves the parallel-agent work
without enabling deployment or save mutation. The scalar default patcher is complete in commit
`e31ef08`; the work below still needs the listed gates before it is production-ready.

## Preserved evidence

- Shipping script cache semantic fingerprint (scalar-operand stable):
  `c1b38e083fdecc93d1c4a53953e2fb9016963c042c3db86ddfda6a408230468b`.
- Binds raw SHA-256: `46e6629ad5cacc112b9922d48a1aa948f40572d7285705b981c3eca3dc615fea`.
- Sealed Binds AS-type to Unreal-path map: 11,193 rows; digest begins `cffbce6f`.
- USMAP raw SHA-256: `73558c36895cd1b0f0fd1b3cb44305b240f8dbb93730ad03c88d7b8478b7ffca`.
- USMAP class graph: 6,594 rows; digest
  `0e64322222d3d32c5cd41254532d518be5feb722a24ed0142284fa4ec91d679d`.
- Resolved Binds/USMAP class profile: 6,572 rows; digest begins `1763379b`.
- Planned atomic tuple ID including the semantic cache fingerprint:
  `sha256:3f53ee63723e6eb0c1ed7212c76d17976592dff30921c7fb2be729f2aef61cd1`.

The current ancestry scaffold compiles and its configured profile/fingerprint tests passed. It
joins native ancestry only after the parsed script chain reaches an unresolved native terminal.

The standalone GameplayTag-to-float32 scanner in `default_tag_map.rs` is read-only. Its exact raw
shape is `SetV4; PSF; PshGPtr; PshVPtr(this); ADDSi; CALLSYS`. Synthetic tests pass. The Shipping
audit found 1,432 raw windows, 339 `m_DamageBase` windows in 323 functions, no semantic duplicates,
158 Edge-tag windows, and one unique Sword site. Sword's raw value is `10.0f`, operand file offset
`0x273e511`, and zeroed-window context SHA-256 is
`d02d0b0a7bd68cdae2d2e04b530fa959a94c2270cf178d406f64c474f1840312`.

## Required before enabling native mutation

1. Construct and validate ancestry from the raw cache bytes plus semantic fingerprint, not GUID
   alone; store and enforce the complete atomic profile tuple.
2. Bump the selector format and bind native-derived sites to an explicit ancestry profile ID so a
   stale selector cannot cross supported hotfix profiles.
3. Resolve parent and child with exact canonical case and require `SchemaKind::Class` on every edge;
   never admit the general case-insensitive/`Unknown` schema fallback as mutation evidence.
4. Add CLI USMAP loading/autodiscovery. Unknown or mismatched cache, Binds, USMAP, graph, or bridge
   must yield the existing strict scalar-only fallback.
5. Add real recovery, patch, rediscovery, tuple-mismatch, case, struct/unknown, ambiguity, cycle, and
   stale-selector tests. Then run complete tests and the release build.
6. Semantically resolve the tag-map scanner's field, native ancestry, `GameplayTag` global, exact
   `TMap<FGameplayTag,float32>` field type, and exact `TMap::Add(FGameplayTag&,float32&)` callee before
   exposing any selector or mutation API.

Unrelated generated Flutter localization changes and the pre-existing rustfmt-only `splice.rs`
change are intentionally excluded from this checkpoint.
