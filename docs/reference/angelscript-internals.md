# AngelScript default-patching internals

This page records the admission criteria, seal semantics, and receipt fields
behind `gore as default-sites`, `gore as patch-default`, `gore as
tag-map-sites`, and `gore as patch-tag-map`. The user-facing workflow is
described in [Offline default patching](../guide/angelscript-defaults.md).

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

## Mandatory sealed evidence

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

## Tag-map patch transaction and receipt

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

## Current boundary

These paths are suitable for reviewed, fixed-width changes already present in generated vanilla
`__InitDefaults`: direct supported scalar assignments, plus the separately sealed native
`GameplayTag`-to-`float32` map-entry shape above. They do not author new fields, assignments, map
keys, or maps; resize bytecode or containers; edit arbitrary complex initializer expressions; or
create NPCs, dialogs, or quests. For anything beyond that fixed-width envelope, decompile the
module instead: `gore as emit` writes the initializer back out as class-scope `default`
statements, which can be edited freely and recompiled — see
[Scripts](../guide/scripts.md). They also do not replace the separate transactional
bundle/deployment and save-comparison workflow described in
[AngelScript dialog authoring](../guide/dialog-authoring.md).
