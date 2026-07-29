# Offline AngelScript default patching

GORE exposes two narrow, fail-closed cache workflows:

- `gore as default-sites` and `gore as patch-default` inspect and change directly serialized
  scalar assignments.
- `gore as tag-map-sites` and `gore as patch-tag-map` inspect and change already-present, sealed
  native `GameplayTag` to `float32` `TMap` entries.

Both workflows operate on generated `__InitDefaults` bytecode in cache files, entirely offline and
copy-on-write. None of these commands launches the game, injects a runtime loader, installs or
deploys the result, or reads or writes a save.

This is not a source representation for arbitrary class defaults. `emit-all` still omits generated
`__InitDefaults`, and `compile-module --op edit` still refuses authored `default` statements. New
modules may continue to author defaults through `compile-module --op add`.

## Admitted scalar sites

The inspector reports a site only when all of these facts are proven:

- The function is the class's unique, method-table-resident, generated `void __InitDefaults()`
  with one of the two observed generated trait shapes.
- The entire initializer is branch-free, contains no `ThrowException`, and has exactly one `RET`,
  as its final instruction. An early terminal instruction or any trailing dead bytecode makes the
  initializer ineligible, even if a matching assignment exists in that unreachable region.
- Three adjacent instructions have exactly this shape, use the same temporary slot, and use the
  same store width:

  ```text
  SetV{1,2,4,8} slot, immediate
  LoadThisR member_offset, owner_type_id
  WRTV{1,2,4,8} slot
  ```

- The owner type and member offset resolve to one declaring field owner, the target class is
  proven to be that owner or its descendant, and the field value type has one authoritative
  interpretation.
- The value is a supported scalar: `bool`, signed or unsigned integers, `float32`, the game's
  64-bit `float`/`float64`/`double`, or a 1-, 2-, or 4-byte enum whose identity is uniquely
  present in the parsed script modules. An `E...` spelling alone is not enum-kind evidence;
  native enums remain disabled until a sealed profile proves their kind.
- Exactly one editable assignment exists for the semantic
  `(module, class, field_owner, field, value_type, ancestry_profile)` selector.

The ancestry proof is built from the complete parsed module model before any field is admitted.
Bare class names must be globally unique across modules, every parsed inheritance chain must be
cycle-free, and the target class must reach the declaring `field_owner` by exact names. When a
chain reaches an unparsed native parent, that direct parent is a valid terminal owner, but ancestry
above it is unknown and is not guessed. This keeps inherited and shadowed same-name fields distinct.
Without additional evidence this deliberately leaves 5,197 otherwise exact Shipping scalar
windows uneditable because their declaring owner lies above the first native parent. The current
Shipping profile can recover that ancestry only when the cache semantic fingerprint, Binds bytes
and bridge, and USMAP bytes and exact Class graph all match their one atomic sealed tuple.

Script-declared field types come from the parsed module model. Native field types are mutation
evidence only when `Binds.Cache` matches both the sealed audited file identity
`46e6629ad5cacc112b9922d48a1aa948f40572d7285705b981c3eca3dc615fea` and the audited extracted
field-map identity `5ddf7fa6df36ac00d07bd068fcf19ad61a3f4b836133513966dc379b24241707`,
and the inspected `PrecompiledScript_Shipping.Cache` header has the paired audited per-build GUID
`450d65c04f0c014fbec568016378e69a`. All three identities must match. The CLI uses
`GORE_AS_BINDS` when set, otherwise `Binds.Cache` beside the input cache. An absent, unreadable,
unknown, parser-drifted, or differently paired native profile supplies no native mutation
evidence; its generic field information can still assist read-only decompilation.

For scalar default inspection and patching, set `GORE_AS_USMAP` to an exact mappings file when the
cache is outside its game layout. Otherwise the CLI scans regular `.usmap` files under
`<G1R>/Binaries/Win64/ue4ss`, derived from `<G1R>/Script/<cache>`. Neither a Steam location nor a
filename/version is trusted: bounded file contents are parsed once and must satisfy every sealed
identity and parser-output digest. Missing, unreadable, unknown, ambiguous, oversized, or
mismatched candidates produce a warning and the strict scalar-only fallback. Direct/script-proven
sites remain available; a selector with a non-null native `ancestry_profile` becomes not-found and
cannot publish output.

Calls, computed expressions, branched initializers, structs, object handles, strings/text, arrays,
containers, arbitrary gameplay-tag maps, and other complex defaults are not patchable by the
scalar commands. The separate tag-map workflow below admits only its sealed native
`GameplayTag`-to-`float32` shape. Repeated assignments to one field and duplicate initializer
identities are also rejected instead of being selected by incidental byte order.

## Inspect scalar sites

Filters are exact semantic names, not substrings:

```powershell
$CACHE = 'D:\SteamLibrary\steamapps\common\Gothic 1 Remake\G1R\Script\PrecompiledScript_Shipping.Cache'

gore as default-sites $CACHE `
  --module Items.GenericItems.FoodGeneric `
  --class UItFo_Apple `
  --field m_MaxStack
```

For the audited 1.0.3 hotfix this read-only query reports `m_MaxStack` as an `int` with display
value `99` and `expected_hex=63000000`. JSON includes the input length and SHA-256, scan statistics,
the strict selector, the raw operand, and audit provenance:

```powershell
gore as default-sites $CACHE --class UItFo_Apple --json > apple-sites.json
```

The JSON `site_count` reflects the exact CLI filters. The diagnostic `stats` describe the complete
cache inspection performed before filtering. A zero result means the requested field was not
uniquely proven editable; it is not permission to guess an offset.

## Create a scalar semantic selector

`patch-default` accepts only the selector object from a reported site, saved as a small strict JSON
file. Selector v4 requires the declaring `field_owner`, canonical `value_type`, and explicit
nullable `ancestry_profile`; older selectors
are rejected. It does not
accept an instruction index, byte offset, opcode, context hash, or display value:

```json
{
  "format": "gore-as-default-site-v4",
  "kind": "scalar",
  "module": "Items.GenericItems.FoodGeneric",
  "class": "UItFo_Apple",
  "field_owner": "UItemDefinition",
  "field": "m_MaxStack",
  "value_type": "int",
  "ancestry_profile": null
}
```

Save that object as `apple-max-stack.selector.json`. Unknown JSON fields, unsupported format/kind,
empty names, outer whitespace in names, non-files, and selector files over 64 KiB are rejected.
The target class, declaring owner, field, canonical value type, and exact ancestry are re-resolved
against the input cache on every patch attempt. Copy `field_owner` and `value_type` from the
reported selector; do not infer either from the target class or field name. Binding the type means
that identical raw bytes cannot silently cross an `int32` to `float32` hotfix.
For script enums, the reported canonical value also binds the serialized module, namespace, and
enum name reached through the field's exact `type_info` reference; do not shorten it to a bare name.
Direct and wholly script-proven ancestry uses JSON `null`. A native-derived site instead binds the
exact SHA-256 identity of its atomic cache/Binds/USMAP evidence tuple. Missing, stale, or altered
profile IDs cannot match that site even when field names and raw CAS bytes happen to be identical.

## Patch a scalar to a new cache

The following example changes the raw little-endian `int` operand from 99 to 50 in a new file:

```powershell
gore as patch-default $CACHE `
  --selector .\apple-max-stack.selector.json `
  --expected-hex 63000000 `
  --replacement-hex 32000000 `
  --out .\PrecompiledScript_Apple50.Cache `
  --json
```

This example only produces an offline cache. It does not deploy it to the game.

`expected_hex` is a compare-and-swap guard and must be copied verbatim from a fresh
`default-sites` result for the exact input cache. Both hex arguments are lowercase, have no `0x`
prefix, and contain the complete serialized operand:

- `SetV1`, `SetV2`, and `SetV4`: exactly 8 hex characters (4 bytes).
- `SetV8`: exactly 16 hex characters (8 bytes).
- Narrow 1- and 2-byte values still occupy a full 4-byte operand; unused high bytes must be zero.
- Boolean operands must be canonical `00000000` or `01000000`.
- Floating-point replacements are raw IEEE-754 little-endian bytes. The printed decimal
  `display_value` is informational and is not accepted as patch input.

Never reuse an old `expected_hex` merely because a selector name survived a hotfix. Re-run the
inspector against the new cache and review its cache SHA-256, type, encoding, expected bytes, and
provenance. A produced cache is tied to the exact input build and its reference tables.

## Scalar fail-closed transaction and provenance

Before mutation, the patcher reparses the header, complete module region, all seven tail tables,
and EOF; rejects duplicate tail-table keys; rebuilds module and reference semantics; requires one
semantic selector match; and compares the complete current operand with `expected_hex` both in the
decoded bytecode and at the proven cache location.

It then clones the input in memory and changes only that fixed-width operand. Postconditions require:

- unchanged cache length;
- no byte difference outside the operand range;
- another full structural parse;
- exactly one rediscovered semantic site at the same operand location; and
- replacement bytes equal to the rediscovered current operand.

Only after those checks does the CLI write and sync a temporary file in the output directory and
publish it with a no-clobber rename. The output parent must already exist, and any existing output
path, including a symlink, is refused. The input file is never overwritten.

The reported absolute `operand_offset`, instruction positions, opcode, numeric owner ID/member
offset, and `context_sha256` are audit provenance only. `context_sha256` hashes the exact
three-instruction window with only the value operand zeroed. Unlike those numeric details,
`field_owner`, `value_type`, and `ancestry_profile` are required semantic selector input. The complete v4 selector plus raw
compare-and-swap guard direct the patch. This prevents offsets from an old build or a shadowed
same-name field from silently redirecting a write.

## Native GameplayTag-to-float32 map entries

`gore as tag-map-sites` and `gore as patch-tag-map` form a separate, stricter workflow for one
observed generated shape: an already-existing entry in a sealed native field whose value is a
`TMap<GameplayTag, float32>`. The entry must already occur in generated, branch-free
`__InitDefaults` bytecode and must pass the exact map-call, field-schema, target-ancestry, and
reference proofs. This workflow changes only the existing four-byte value operand. It cannot add a
key, create or resize a map, author bytecode, or create an NPC, dialog, quest, or any other gameplay
object.

Like the scalar workflow, tag-map inspection and patching are purely offline. The patch command
produces a new cache file; it does not provide a loader, touch a save, deploy the cache, or make the
game consume it.

### Mandatory sealed evidence

Every tag-map command requires the exact supported cache/Binds/USMAP tuple. There is no scalar or
best-effort fallback:

- `Binds.Cache` is read from `GORE_AS_BINDS` when set, otherwise from beside the input cache.
- `GORE_AS_USMAP`, when set, is the sole mappings candidate. Otherwise the CLI derives
  `<G1R>/Binaries/Win64/ue4ss` from an input at `<G1R>/Script/<cache>` and considers the bounded
  regular `.usmap` files there.
- File names and locations are only discovery hints. The bounded bytes, parsed identities, cache
  GUID, combined cache fingerprint, native ancestry profile, map proof, and schema graph must form
  the one sealed exact-build tuple.

Inputs must be regular bounded files: the tag-map cache limit is 512 MiB, each Binds/USMAP limit is
128 MiB, and the strict selector-file limit is 64 KiB.

Missing, unreadable, oversized, ambiguous, parser-drifted, or mismatched evidence is a hard error
for both listing and patching. It never widens the admitted set and never publishes an output. For
an input copied outside the game layout, point both variables at the original exact-build evidence:

```powershell
$env:GORE_AS_BINDS = 'D:\SteamLibrary\steamapps\common\Gothic 1 Remake\G1R\Script\Binds.Cache'
$env:GORE_AS_USMAP = `
  'D:\SteamLibrary\steamapps\common\Gothic 1 Remake\G1R\Binaries\Win64\ue4ss\G1R-5.4.3-168781-272ce2f8.usmap'
```

### List and select an existing entry

The four optional filters are exact, case-sensitive semantic names, not substrings:

```powershell
$CACHE = 'D:\SteamLibrary\steamapps\common\Gothic 1 Remake\G1R\Script\PrecompiledScript_Shipping.Cache'

gore as tag-map-sites $CACHE `
  --module Items.GenericItems.WeaponsOneHandedGeneric `
  --class UItMw_1H_Sword_Old_01 `
  --field m_DamageBase `
  --tag Item_Damage_Physical_Edge `
  --json > .\sword-edge-sites.json
```

With `--json`, stdout is exactly one JSON document; evidence diagnostics remain on stderr. As with
scalar listing, `site_count` reflects the filters while `stats` describes the complete inspection.
A zero count is not permission to infer a nearby key or offset.

Only `.sites[N].selector` from a fresh successful result is valid selector input. Save that object,
not the complete site, provenance object, or report. This PowerShell example writes UTF-8 without a
byte-order mark:

```powershell
$REPORT = Get-Content .\sword-edge-sites.json -Raw | ConvertFrom-Json
$SELECTOR_JSON = $REPORT.sites[0].selector | ConvertTo-Json -Depth 8
$SELECTOR_PATH = Join-Path $PWD 'sword-edge.selector.json'
[System.IO.File]::WriteAllText(
  $SELECTOR_PATH,
  $SELECTOR_JSON,
  [System.Text.UTF8Encoding]::new($false)
)
```

The selector is strict and semantic. Missing or unknown fields, altered format/kind, empty or
whitespace-padded names, a string tag, or a different ancestry/map proof ID are rejected. Function
name, operand offset/range, context SHA-256, current bytes, display value, and encoding are
output-only audit data and must not be copied into the selector. The current bytes instead form the
separate compare-and-swap argument.

### Patch with raw float32 compare-and-swap bytes

Both hex arguments are exactly eight lowercase hexadecimal characters with no `0x` prefix. They
are the complete raw IEEE-754 `float32` little-endian operand, not a decimal string. Always copy
`expected_hex` from a fresh listing of the exact input. For example, `10.0` is `00002041` and
`11.0` is `00003041`:

```powershell
$OUT = '.\PrecompiledScript_SwordEdge11.Cache'

gore as patch-tag-map $CACHE `
  --selector .\sword-edge.selector.json `
  --expected-hex 00002041 `
  --replacement-hex 00003041 `
  --out $OUT `
  --json > .\sword-edge-patch-receipt.json
```

The patcher requires exactly one selector match and exact current-byte equality, clones the input,
changes only the proven four-byte range, then structurally and semantically reinspects the result.
It publishes through a synced temporary file and a durable no-clobber operation. The output parent
must already exist, `$OUT` must be a new path, the input is never overwritten, and a racing creator
cannot be replaced.

After publication, the CLI reopens the persisted file, verifies its length, SHA-256, and exact
bytes, and rediscovers the same selector and replacement at the original range. Only then does it
emit the JSON receipt. The receipt records input and persisted-output path/length/SHA-256, exact
Binds and USMAP path/length/SHA-256, cache GUID, the mutation-stable combined scalar/tag
fingerprint and operand counts, ancestry and map-proof identities, the strict selector and CAS
bytes, plus output-side function, context SHA-256, operand range, and field-schema proof. If a rare
failure occurs after publication, the CLI reports that the path may be an unverified recovery
artifact and deliberately does not delete it behind the user's back.

Re-run the listing against the produced cache before treating it as a verified input. Because an
output such as `$OUT` is normally outside `<G1R>/Script`, use explicit exact-build evidence:

```powershell
gore as tag-map-sites $OUT `
  --class UItMw_1H_Sword_Old_01 `
  --field m_DamageBase `
  --tag Item_Damage_Physical_Edge `
  --json > .\sword-edge-11-sites.json
```

The rediscovered site must retain the same selector and audit provenance and report
`expected_hex=00003041`. A stale `00002041` compare-and-swap against that output fails without
creating another cache.

## Current boundary

These paths are suitable for reviewed, fixed-width changes already present in generated vanilla
`__InitDefaults`: direct supported scalar assignments, plus the separately sealed native
`GameplayTag`-to-`float32` map-entry shape above. They do not author new fields, assignments, map
keys, or maps; resize bytecode or containers; edit arbitrary complex initializer expressions; or
create NPCs, dialogs, or quests. They also do not replace the separate transactional
bundle/deployment and save-comparison workflow described in
[AngelScript dialog authoring](dialog-authoring.md).
