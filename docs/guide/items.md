# Item & stat values (overrides)

Change any default value on an item, NPC, or ability class — weapon damage,
item value, weight, and so on. GORE compiles a declarative `overrides.toml`
into a self-contained UE4SS Lua mod that patches the class default object (CDO)
at load.

## The override file

```toml
[meta]
name = "MyBalanceMod"
delay_ms = 0            # 0 = apply on first tick; >0 = apply after N ms

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
- `[meta].delay_ms` defers the patch. `0` applies it on the first tick, which is
  what you normally want. A positive value is an escape hatch for classes that
  are not yet resolvable that early.
- Each `[[override]]` names one `class`, one `field`, and exactly one typed
  value: `value_int`, `value_float`, or `value_bool`.

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
(`StaticFindObject("/Script/<module>.Default__<class>")`) and assigns the field
at load time.

It is fully self-contained: it does **not** require the
[gore-lua helpers](../../lua/README.md). Those are for hand-written mods, a
different path.

## Finding class and field names

Item, NPC, and knowledge classes are listed in the catalogs bundled with the
save editor (`apps/save-editor/assets/*_catalog.json`). That is the quickest
place to look up an exact class name.

To regenerate the catalogs and the field schema yourself, or to fold the
game's *real* default values into the model, see
[Catalogs & data models](catalogs-and-models.md).

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
