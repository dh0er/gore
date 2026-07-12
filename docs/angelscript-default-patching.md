# Offline AngelScript scalar-default patching

`gore as default-sites` and `gore as patch-default` provide a narrow, fail-closed path for
inspecting and changing directly serialized scalar assignments in existing generated
`__InitDefaults` bytecode. They operate on cache files offline. Neither command launches the game,
injects a runtime loader, installs the result, or reads or writes a save.

This is not a source representation for arbitrary class defaults. `emit-all` still omits generated
`__InitDefaults`, and `compile-module --op edit` still refuses authored `default` statements. New
modules may continue to author defaults through `compile-module --op add`.

## Admitted sites

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
  `(module, class, field_owner, field, value_type)` selector.

The ancestry proof is built from the complete parsed module model before any field is admitted.
Bare class names must be globally unique across modules, every parsed inheritance chain must be
cycle-free, and the target class must reach the declaring `field_owner` by exact names. When a
chain reaches an unparsed native parent, that direct parent is a valid terminal owner, but ancestry
above it is unknown and is not guessed. This keeps inherited and shadowed same-name fields distinct.
In the audited Shipping cache this deliberately leaves 5,197 otherwise exact scalar windows
uneditable because their declaring owner lies above the first native parent. A separately sealed
native-ancestry profile is required before those sites can be enabled.

Script-declared field types come from the parsed module model. Native field types are mutation
evidence only when `Binds.Cache` matches both the sealed audited file identity
`46e6629ad5cacc112b9922d48a1aa948f40572d7285705b981c3eca3dc615fea` and the audited extracted
field-map identity `5ddf7fa6df36ac00d07bd068fcf19ad61a3f4b836133513966dc379b24241707`,
and the inspected `PrecompiledScript_Shipping.Cache` header has the paired audited per-build GUID
`450d65c04f0c014fbec568016378e69a`. All three identities must match. The CLI uses
`GORE_AS_BINDS` when set, otherwise `Binds.Cache` beside the input cache. An absent, unreadable,
unknown, parser-drifted, or differently paired native profile supplies no native mutation
evidence; its generic field information can still assist read-only decompilation.

Calls, computed expressions, branched initializers, structs, object handles, strings/text, arrays,
containers, gameplay-tag maps, and other complex defaults are not patchable. Repeated assignments
to one field and duplicate initializer identities are also rejected instead of being selected by
incidental byte order.

## Inspect a cache

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

## Create the semantic selector

`patch-default` accepts only the selector object from a reported site, saved as a small strict JSON
file. Selector v3 requires the declaring `field_owner` and canonical `value_type`; older selectors
are rejected. It does not
accept an instruction index, byte offset, opcode, context hash, or display value:

```json
{
  "format": "gore-as-default-site-v3",
  "kind": "scalar",
  "module": "Items.GenericItems.FoodGeneric",
  "class": "UItFo_Apple",
  "field_owner": "UItemDefinition",
  "field": "m_MaxStack",
  "value_type": "int"
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

## Patch to a new cache

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

## Fail-closed transaction and provenance

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
`field_owner` and `value_type` are required semantic selector input. The complete v3 selector plus raw
compare-and-swap guard direct the patch. This prevents offsets from an old build or a shadowed
same-name field from silently redirecting a write.

## Current boundary

This path is suitable for reviewed, fixed-width scalar changes already present as direct
assignments in vanilla `__InitDefaults`. It does not yet author new fields or assignments, resize
bytecode, edit complex initializer expressions, or change gameplay-tag/map defaults. It also does
not replace the separate transactional bundle/deployment and save-comparison workflow described in
[AngelScript dialog authoring](dialog-authoring.md).
