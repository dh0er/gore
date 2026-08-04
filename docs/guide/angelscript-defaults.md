# Offline AngelScript default patching

GORE can change selected class-default values that ship inside the game's
compiled AngelScript cache, entirely offline and copy-on-write. `gore as
default-sites` and `gore as patch-default` inspect and change directly
serialized scalar assignments (bool, integer, float, and supported enum
defaults). `gore as tag-map-sites` and `gore as patch-tag-map` inspect and
change already-present native `GameplayTag`-to-`float32` map entries. None of
these commands launches the game, deploys a cache, or reads or writes a save;
every patch writes a new cache file and never overwrites the input. The
admission criteria, seal semantics, and receipt fields behind these commands
are documented in [AngelScript patching internals](../reference/angelscript-internals.md).

## What can be patched

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

## Native GameplayTag-to-float32 map entries

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

## Related

- [Scripts (AngelScript)](scripts.md)
- [AngelScript patching internals](../reference/angelscript-internals.md)
