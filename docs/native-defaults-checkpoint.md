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
- Runtime-derived and sealed atomic tuple ID including the semantic cache fingerprint:
  `sha256:3f53ee63723e6eb0c1ed7212c76d17976592dff30921c7fb2be729f2aef61cd1`.

The current ancestry scaffold compiles and its configured profile/fingerprint tests passed. It
joins native ancestry only after the parsed script chain reaches an unresolved native terminal.

The GameplayTag-to-float32 scanner in `default_tag_map.rs` remains read-only. It now losslessly
parses the exact tail identities and proves the generated initializer, `GameplayTag` global,
`TMap<FGameplayTag,float32>::Add` signature, and exact declared native USMAP field shape. The
Shipping audit proves all 1,432 raw windows and eight distinct native map fields; Sword remains
unique. The sealed field-profile digest is
`5fa2e35616cb6b04a3060202e55ff575d8e8aeab5a25602aeddc10b3ad542708`, and its opaque proof ID is
`sha256:c1b9f5e3e85e0637dc56c2228d1b38c7fdf9fc8d7aa96342cd56460c936d9b71`.

## Required before enabling native mutation

Ancestry tuple binding, selector v4, exact Class edges, content-sealed CLI USMAP discovery, and the
real Sword scalar recovery/patch/rediscovery test are complete in `40611d8` and `6658f80`.

Before enabling tag-map mutation, the current public textual site-upgrade API must be replaced by
an opaque report constructed from one exact `cache + profile` pair. Its cache fingerprint must
normalize both proven direct-scalar and proven tag-map operands, so a tag patch can be reconstructed
and rediscovered in a later process without allowing a site from another cache to borrow the sealed
field proof. Selector uniqueness, CAS, copy-on-write, post-patch reparse, CLI, full tests, and the
release build then remain to be added for the tag-map path.

Unrelated generated Flutter localization changes and the pre-existing rustfmt-only `splice.rs`
change are intentionally excluded from this checkpoint.
