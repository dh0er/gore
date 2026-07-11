# Cooked package carrier

`PackageCarrier` treats a split cooked Unreal package as two opaque byte
components: `Name.uasset` and its derived sibling `Name.uexp`.

The carrier provides only operations that are justified without knowing the
exact Unreal package version:

- bounded reads of regular, non-symlink files;
- a reopen plus length and SHA-256 verification after reading;
- explicit `(component, offset, length)` slices;
- same-length range replacement, preserving every byte outside that range;
- same-length compare-and-swap replacement that fails on the first drift byte;
- staging, syncing, reopening, hashing, and no-clobber publication to a new
  output pair.

## Proven G1R UE5.4 export envelope

`LegacyPackageEnvelope::parse_g1r_ue5_4(&carrier)` is the deliberately narrow
next layer. It uses retoc's version-aware legacy-header parser with an explicit
UE5.4 fallback, then requires all of the following before exposing an export:

- the package and property serialization are marked unversioned;
- the package carries Unreal's cooked-package flag;
- `total_header_size == .uasset.len()`;
- the `.uexp` ends in the four-byte package-file magic;
- every `SerialOffset - total_header_size` / `SerialSize` range is inside the
  pre-footer `.uexp` region;
- export ranges do not overlap;
- every export class index and its import/export outer chain are non-null,
  in-bounds, acyclic, and no deeper than 128 objects.

The result exposes exact `ExportBoundary` values and borrowed `ExportEnvelope`
bytes. Each boundary also exposes the exact qualified class path recovered from
the export's package-map class reference. `resolve_class_schema(&schemas)` binds
that path to a class in the USMAP; short-name guessing is not involved. It does
not resolve a class-specific property end. Once a schema-aware codec has
returned an exact consumed count, `split_decoded_prefix` retains the remaining
class-native suffix opaquely. `decode_primitive_properties` is a convenience
for primitive-only UObject exports and still rejects the first non-zero complex
property without guessing its size.

Package-index arithmetic is performed on the raw signed value in a widened
integer. The resolver does not use retoc's asserting index-conversion helpers,
so malformed `i32::MIN`, null, out-of-range, cyclic, and over-depth references
become typed errors rather than panics or unbounded traversal.

This boundary was checked against byte-identical double extractions from the
current G1R container for `PhysicMaterialsColor` and two `FootstepTag` assets.
For all three, the Zen and legacy maps agree, `.uasset` equals the cooked header
size, and the export ends immediately before the package footer.

### The apparent four-byte prefix is not a prefix

Those real UObject property streams begin with:

```text
00 00  00 00  <final/meaningful fragment> ...
```

The first four bytes are two legal empty, non-final 16-bit fragments in the
unversioned header. Each fragment advances the input by two bytes even when its
semantic skip/value counts are zero; the fragment-count limit provides the DoS
bound. `UnversionedHeader` therefore accepts and byte-preserves them. Skipping
four bytes would produce the same selected slots in these particular files but
would discard real header bytes and break byte-exact round trips.

The `+4` rule is also disproved by another export from the same current
container (`InputTriggerPressed_0`), whose valid unversioned header begins at
the exact export start without the two empty fragments. There is no generic
four-byte property-start adjustment or separate version gate. The applicable
gate is the cooked package's `UsesUnversionedProperties` flag under the explicit
G1R UE5.4 profile.

## Complex property spans

`PropertySpanWalker::g1r_ue5_4(&schemas)` consumes a known UObject property
stream at its exact export start and returns a `PropertyBlockSpans` tree. Every
`SliceSpan` borrows the original input and reports an absolute offset, length,
end, and opaque byte slice. `consumed()` is therefore a decoder-proven property
end that can be passed to `ExportEnvelope::split_decoded_prefix`; bytes after it
remain an opaque native suffix.

The current profile recognizes only wire forms backed by the current USMAP,
the three real exports, or fixed Unreal serializer layouts needed by them:

- fixed numeric primitives;
- one-byte validated bools, four-byte cooked package indices, and eight-byte
  `FName` references;
- `LinearColor` as four `float32` values and UE5 `Vector4` as four `float64`
  values, only after the installed USMAP matches the exact four-slot
  `/Script/CoreUObject` schemas;
- recursive unversioned `BoneFeetData` and `BoneTrackedData` structs resolved
  specifically from `/Script/G1R`;
- `Map` removed-key and live-entry sections with recursively walked key/value
  forms.

The walker itself is inspection-only and exposes no structural encoder. Unknown
non-zero arrays, sets, strings, structs, reference forms, and other USMAP types
return a typed `UnsupportedType`/`UnsupportedStruct` error before a payload size
is assumed. Zero-masked properties remain safe because they consume no payload.
Truncation, invalid bool/count encodings, schema failures, and arithmetic errors
are typed as well. `SpanLimits` bounds nesting, each map section's element
count, and the total returned tree nodes.

An ignored local integration test walks the hotfix copies of
`DA_PhysicsMaterialColor`, `DA_HumanFootsteps`, and `DA_WolfFootsteps` through
this public API. It reaches byte 290, 82, and 82 respectively, leaving the same
four-byte zero suffix immediately before each package footer.

## Snapshot-bound fixed-leaf patches

`FixedLeafPatch::plan` promotes one non-referential `FixedValueSpan` into an
owned compare-and-swap plan. It accepts no absolute offset from the caller. The
export, property block, and leaf must all borrow from the same carrier, and the
planner independently resolves the export's exact class and walks it again with
the supplied USMAP.

The plan owns all evidence needed after those borrows are dropped:

- SHA-256 seals for the complete `.uasset` and `.uexp` pair;
- exact export index, name, class path, component, boundary, and root schema
  layout;
- a semantic path of property slots, named structs, map shape, entry index, and
  fixed map-key kind/length/hash;
- the fixed wire kind, expected bytes, replacement bytes, and decoder-proven
  property/header boundaries.

The planner rejects width changes, no-ops, invalid Bool bytes, cooked package
indices, `FName` references, live/removed map keys, and values whose enclosing
map key is a schema-recursive struct or map. The latter stays fail-closed until
the full semantic schema of a complex key can be sealed. Map values with
fixed-width keys (including validated fixed native structs) are supported; key
identity is rechecked before and after the edit.

`apply` first requires the complete pair seal, then reparses the UE5.4 envelope,
re-resolves the exact schema, rewalks the property tree, and follows the owned
semantic path. Only then does `replace_range_if_equal` copy the same-length
bytes. A second parse/walk must find the replacement at the same semantic leaf.
An unexpected postcondition triggers a compare-and-swap rollback and verifies
the original pair seal. Publication remains the existing no-clobber
`write_new` path; neither API overwrites a loaded source package.

The ignored real-fixture test changes one finite `float64` lane inside
`DA_WolfFootsteps`' proven `Vector4`, verifies that every other `.uexp` byte plus
the complete `.uasset`, native suffix, and footer remain exact, then publishes,
reopens, reparses, and rewalks the new pair successfully.

An existing destination is never overwritten, and a loaded source pair cannot
be selected as its own output. Both output files are fully staged and verified
before publication. Each file is published atomically; because ordinary
filesystems cannot rename two files as one transaction, a second-file publish
failure triggers removal of the first file.

## Still deliberately unknown

The envelope does **not** claim that every export starts with script properties.
The span walker and fixed-leaf patch can identify the proven Map/Struct forms
above, but do not infer or resize any other complex layout. Variable-width
values, collection structure changes, reference retargeting, zero-mask/header
changes, and complex map-key edits remain unsupported. The three native
PrimaryDataAsset fixtures have
a four-byte zero suffix after their fully walked property data, but its
class-native meaning has not been proven and no generic suffix size is encoded
in the API. Blueprint-generated DataAssets also require their generated `_C`
schemas, which the installed USMAP does not contain. A local generated class is
bound only when that exact package-qualified `_C` schema exists. Otherwise
`LocalGeneratedClassSchemaMissing` is returned; the resolver never substitutes
`PrimaryDataAsset` or another parent schema.

Accordingly, callers must select a known UObject export, bind its boundary to
the exact USMAP class, and pass only a decoder-proven consumed count. Unknown
bytes remain untouched; there is no `export_start + 4` heuristic.
