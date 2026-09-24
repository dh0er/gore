# Native item and stat values — greenfield plan

Status: design only. This is a clean break, not a migration of `overrides.toml`,
`gore gen`, bundle `overrides`, or their UE4SS/Lua behavior.

## Product contract

GORE should inspect and author **game-defined item and stat defaults** using
only files shipped with Gothic 1 Remake, then build, deploy and remove the mod
without UE4SS. A build is tied to the exact game generation it inspected.
Unknown targets, unsupported value shapes and changed game builds fail with a
specific reason before deployment. Values already serialized into a save are a
separate Save Editor concern.

The public interface is new and typed. For example, `gore value inspect
--class UItFo_Apple` would show the module, available defaults, their types and
current values. `gore mod build` would accept a new `values` section such as:

```json
{
  "values": [
    { "class": "UItFo_Apple", "field": "m_Value", "value": { "int": 500 } },
    { "class": "UItMw_1H_Sword_Old_01", "field": "m_DamageBase",
      "tag": "Item_Damage_Physical_Edge", "value": { "float": 15.0 } }
  ]
}
```

The exact syntax is settled with the inspected game schema, not inherited from
the Lua generator. The advertised type set should reflect what the native
builder can actually compile and verify. Item definitions, NPC/ability stats,
trader stock, instance inventories and save values must be classified separately:
they do not all live in a class-default field or share one editing mechanism.

## Technical route

1. **Map the game's value sources.** Starting from the pristine Shipping script
   cache and matching `Binds.Cache`, identify item definitions, representative
   NPC/ability defaults, damage-tag maps and any values stored outside script
   defaults. Record exact class, declaring owner, module, type and baseline
   expression. The installed live cache currently contains a deployed test mod;
   use GORE's pristine-source selection / owned backup for this work.
2. **Implement one native default builder.** Resolve each requested target
   semantically, edit its class-scope AngelScript `default` statement in the
   emitted source, group edits by module and compile each module once using the
   existing standalone compiler. Preserve every unrelated default and function.
   Reinspect the output to prove the requested value changed and the rest of
   the module remains semantically equivalent. Do not implement a second
   general backend around raw byte offsets or UE4SS CDO writes.
3. **Package and deploy through the script-cache path.** Bundle the compiled
   mini-cache and exact game-generation/target metadata. Reuse the existing
   Mod Manager script splice, backup, recovery and undeploy transaction. Two
   enabled mods editing one module initially produce an explicit conflict;
   field-level merge can be designed later if real use needs it. An authored
   script mod touching that module conflicts by the same rule.
4. **Make discovery self-contained.** Build CLI and Mod Studio field lists from
   the shipped cache, `Binds.Cache` and any GORE-owned exact-build evidence.
   The native workflow must not read a UE4SS object dump, SDK dump or a `.usmap`
   produced by UE4SS. If a target needs information those sources cannot prove,
   add a game-native evidence source or mark that target unsupported; do not
   silently require UE4SS. Studio consumes the new native value contract rather
   than its current non-deployable ItemPatch draft schema.
5. **Remove the old first-party path and update claims.** Delete the Lua item
   generator and old bundle `overrides` contract instead of translating them.
   Replace README, quick-start, item guide, doctor expectations and CLI help
   with the native workflow. The historical `dialog_topics` Lua insertion
   adapter is a separate first-party UE4SS path and must also be retired or
   replaced before claiming GORE as a whole has no UE4SS requirement. Optional
   import of third-party UE4SS mods can remain, clearly labeled as such.

## Completion check

After the implementation slice is complete, run focused offline compile,
semantic-diff, bundle/manager, conflict, reapply, undeploy and game-update
checks. Then test a clean game install with **no `ue4ss` directory**: item value,
weapon damage, one NPC or ability stat, and each additional value type the new
CLI advertises. Confirm the in-game result with a new-game case and document the
expected existing-save behavior. Only then mark Items & Values complete in the
README.

The present pristine BuildID 25168047 corpus has a zero-semantic-difference
whole-tree AngelScript round trip, and offline inspectors locate the Apple
`m_Value` and sword `m_DamageBase` examples. These are a foundation, not proof
that every item/stat field or future game build is writable.
