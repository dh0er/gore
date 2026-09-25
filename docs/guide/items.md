# Item & stat values

Change a class-default value on an item, NPC, or ability — weapon damage,
item value, weight, and so on. GORE reads the Shipping script cache, rewrites
the AngelScript `default` statement, and compiles that module into a script
mini-cache. Deploy uses the same script splice as other script mods. UE4SS is
not involved.

It edits the class **default**, so it does not change objects already
serialized into an existing save. Trader stock, instance inventories, and save
values are separate and are not written by this command.

## Inspect

```powershell
gore value inspect --class UItFo_Apple
gore value inspect --class UItFo_Apple --json
```

The report names the module, each recovered default, its type, and the current
value. A damage map entry includes its gameplay tag. Anything that is not a
recovered class-default assignment or a single `Field.Add(GameplayTag::Tag, …)`
entry is unsupported.

## Build

```json
{
  "meta": { "name": "MyBalanceMod", "version": "0.1.0", "author": "" },
  "values": [
    { "class": "UItFo_Apple", "field": "m_Value", "value": { "int": 500 } },
    { "class": "UItMw_1H_Sword_Old_01", "field": "m_DamageBase",
      "tag": "Item_Damage_Physical_Edge", "value": { "float": 15.0 } }
  ]
}
```

```powershell
gore mod build --spec apple.json --work-dir .gore-value-work -o mods
gore mod deploy --bundle mods\MyBalanceMod
```

`value` is one of `int`, `float`, `bool`, or `str`. The type has to match the
recovered default. Two mods that edit the same script module conflict. A build
is tied to the script cache it inspected; a later game update refuses the old
mini-cache.

`overrides.toml` and `gore gen` are retired. They are not translated into this
format.

## What m_Value does to prices

`m_Value` is neither the buy price nor the sell price. It is the number both are
derived from.

Measured once, in a single session at the trader Dexter, on difficulty
"Gothic". Four items were set to `m_Value = 1000`; three of them produced
readings:

| Item | Buy | Sell | Sell ÷ buy |
|---|---|---|---|
| Bread | 1167 | 585 | 0.5013 |
| Apple | 843 | 422 | 0.5006 |
| Health potion | 844 | 423 | 0.5012 |

Three things follow from those numbers, and nothing more:

- **Sell is half of buy.** Within a fraction of a percent, for all three items.
  So halving `m_Value` halves both sides — you cannot make buying cheap and
  keep selling lucrative with this field alone.
- **Buy is `m_Value` times a per-item factor, and that factor can exceed 1.**
  Bread came out at 1167 from a base of 1000.
- **The class-name prefix does not predict the factor.** All three are `ItFo_*`
  items, yet bread behaves differently from apple and potion.

Prices carry a jitter of roughly 0.3% between readings, so a single observation
does not pin an exact factor down. Whether the relationship is purely
multiplicative was not tested. This is one trader, one difficulty, one session:
nothing here establishes what a different trader charges.

## Compile it

`gore mod build` rewrites the recovered `default` statement, compiles that
module once, and writes a script mini-cache. Unknown classes, wrong types and
a cache that is not the one you inspected fail before anything is deployed.

```powershell
gore mod build --spec apple.json --work-dir .gore-value-work -o mods
gore mod deploy --bundle mods\MyBalanceMod
```

Confirm the new number on a new game. A save that already stored the item
keeps the old value. Details are in the build section at the top of this page.

## Finding class and field names

**Class names come from `gore find`**, which searches the catalogs compiled into
`gore.exe`. No game install, no dump, no setup:

```powershell
gore find ItFo_Potion_Health     # by class name
gore find healing potion         # by display name, after `gore loc extract`
gore find --domain item rune     # one namespace only
```

Each hit prints the class the game resolves. `gore value inspect --class`
then shows the defaults that class can take. Display names need `gore loc extract` first; every result says
which of the two states you are in, so an empty answer can be told apart from an
answer that could not look. The whole command is [Finding things](find.md).

`find` knows the classes. It does not know their **fields** — for those, read
the values out of the game's own compiled script cache.

**From a release install:**

```powershell
gore as default-sites "$GAME\G1R\Script\PrecompiledScript_Shipping.Cache" `
    --field m_Value
```

One line per class: the module it is declared in, the class, the field, its
type, and its current value in that cache. On a game build this toolkit has
audited, `--field m_Value` lists on the order of 900 item classes. Swap the
field for whatever you are after (`m_Weight`, `m_MaxStack`, …), or add
`--class` to look at a single class. Filters are exact names, not substrings.

Two things about that listing:

- It spells classes **with the UE `U` prefix** — `UItFo_Apple`. `gore value inspect`
  wants that spelling.
- Its `module=` column is the AngelScript module the value edit compiles.

The command only reads. It is the inspection half of [Offline AngelScript
default patching](angelscript-defaults.md), borrowed here as a lookup, and it
writes no cache.

It does depend on the toolkit having audited your exact build. One run here,
against an install newer than any build the toolkit has sealed, returned an
**empty** `m_Value` listing, with `Native field types and native ancestry are
unavailable for this build` and `this toolkit has not sealed this build yet` on
stderr. Read the stderr before concluding that a class has no such field.

To regenerate the catalogs `find` reads, or the field schema, or to fold the
game's *real* default values into the model, see
[Catalogs & data models](catalogs-and-models.md).

## Shipping it

The `values` section is compiled into the same bundle as text, audio, textures
and scripts. See [Bundling & deploying](bundles.md).

## Limits

- Edits change **class defaults**. They do not change objects already
  serialized into an existing save.
- Two mods that edit the same script module conflict. Field-level merge is
  not done.

## Related

- [Offline AngelScript default patching](angelscript-defaults.md) — changing a
  default inside the compiled script cache instead of at runtime.
- [Mod Studio](../../apps/mod-studio/README.md) — the same edits as a GUI, with
  a categorized item browser.
