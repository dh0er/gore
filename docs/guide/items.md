# Item & stat values (overrides)

Change any default value on an item, NPC, or ability class — weapon damage,
item value, weight, and so on. GORE compiles a declarative `overrides.toml`
into a self-contained UE4SS Lua mod that patches the class default object (CDO)
at load.

It patches the class **default**, so it does not change objects already
serialized into an existing save. Settle that first — it decides whether this
mechanism can reach your case at all. The remaining limits are
[below](#limits).

## The override file

```toml
[meta]
name = "MyBalanceMod"
delay_ms = 0            # 0 = start on the first tick; >0 = start after N ms

[[override]]
class = "ItFo_Apple"    # AngelScript class name
field = "m_Value"
value_int = 500

[[override]]
class = "ItMw_1H_Sword_01"
field = "m_Weight"
value_float = 1.5
```

- `[meta].name` becomes the mod folder name under `ue4ss\Mods\`.
- `[meta].delay_ms` defers the *first* attempt. `0` starts on the first tick,
  which is what you normally want — the retry loop described below already
  covers classes that are not resolvable that early.
- Each `[[override]]` names one `class`, one `field`, and exactly one typed
  value: `value_int`, `value_float`, or `value_bool`.

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

```powershell
gore gen overrides.toml -o "$GAME\G1R\Binaries\Win64\ue4ss\Mods"
```

`-o` is the UE4SS `Mods` directory the mod folder is written into. Writing
straight into the game install is the fast path; write it elsewhere if you want
to inspect or package the result first.

Optionally validate the class and field names against a reflection model before
generating:

```powershell
gore gen overrides.toml -o "$GAME\...\Mods" --model model.json
```

With `--model`, unknown classes, unknown fields, and type mismatches are
rejected at generation time instead of silently doing nothing in game. Without
it, validation is skipped. Building `model.json` is covered in
[Catalogs & data models](catalogs-and-models.md).

## What the generated mod does

The emitted Lua mod looks up each class's CDO
(`StaticFindObject("/Script/<module>.Default__<class>")`) and assigns the field.

That lookup does not succeed at launch — the target classes do not exist yet
when the mod first runs. So the mod polls: every 1000 ms, up to 120 attempts,
applying each override the first time its CDO appears and leaving it alone
afterwards. If a CDO never turns up, it logs that it gave up and stops.

Measured once, on Steam build 24340829: the mod started at 10:58:03.4 and the
override landed at 10:58:07.5 — about four seconds and several retries later.
Quitting as soon as the main menu is up can therefore show you nothing, and
that is not the same as the mod being broken.

It is fully self-contained: it does **not** require the
[gore-lua helpers](../../lua/README.md). Those are for hand-written mods, a
different path.

## Checking that it applied

`$GAME\G1R\Binaries\Win64\ue4ss\UE4SS.log` gets one line per applied override:

```
[<ModName>] <Class>.<Field> <old> -> <new>
```

UE4SS timestamps each line and prefixes it with `[Lua]`, so search the log for
your mod name. The new value is read back off the CDO after the assignment, so
the line is evidence that the write took — not merely that the mod ran. No line
for a class means its CDO was never found; look for the mod's "gave up" line
towards the end of the run.

Nothing in this toolkit watches the game, so this log is the only place the
applied-or-not question gets answered without inferring it from numbers on
screen.

## Finding class and field names

**Class names come from `gore find`**, which searches the catalogs compiled into
`gore.exe`. No game install, no dump, no setup:

```powershell
gore find ItFo_Potion_Health     # by class name
gore find healing potion         # by display name, after `gore loc extract`
gore find --domain item rune     # one namespace only
```

Each hit prints the class the game resolves — `/Script/Angelscript.ItFo_Apple` —
which is what an `[[override]]` names, and anything the effect register records
about that id. Display names need `gore loc extract` first; every result says
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

- It spells classes **with the UE `U` prefix** — `UItFo_Apple`, where this page
  writes `ItFo_Apple`. Both spellings work in `overrides.toml`:
  `runtime_class_name` in `crates/gore-modgen/src/gen.rs` strips a leading `U`
  that is followed by an uppercase letter before the CDO lookup, and leaves a
  bare name untouched.
- Its `module=` column is the AngelScript module, which is *not* the `module`
  an override uses. Leave `module` out of `overrides.toml`; its default is the
  right one.

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
[Catalogs & data models](catalogs-and-models.md). Both start from a UE4SS
object dump, so neither is a way in while you are still setting UE4SS up.

## Shipping it

Two options:

```powershell
gore package mod_dir/ -o MyMod.zip     # zip the Lua mod folder for distribution
```

or fold the overrides into a unified bundle together with text, audio,
textures, and scripts — see [Bundling & deploying](bundles.md). In a bundle,
overrides are declared inline:

```json
{ "overrides": [ { "class": "ItFo_Apple", "field": "m_Value", "value_int": 500 } ] }
```

## Limits

- Overrides change **class defaults**. They do not retroactively change objects
  already serialized into an existing save.
- One value per `class` + `field` pair. When several mods override the same
  pair, [Mod Manager](mod-manager.md) reports the conflict and the later mod in
  load order wins.
- The mechanism requires UE4SS to be installed and enabled in the game.

## Related

- [Offline AngelScript default patching](angelscript-defaults.md) — changing a
  default inside the compiled script cache instead of at runtime.
- [Mod Studio](../../apps/mod-studio/README.md) — the same edits as a GUI, with
  a categorized item browser.
