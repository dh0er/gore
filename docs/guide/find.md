# Finding things

`gore find <words>` is the answer to "what is this thing called". It searches
everything the toolkit knows offline — the bundled catalogs of what exists, and
the effect register of what things do in game — and says, for every hit, which
of the two it came from.

```powershell
gore find ItFo_Potion_Health        # by class name
gore find healing potion            # by display name, if you have extracted the text
gore find T_LogoRemake              # by asset path
gore find --domain item rune        # one namespace only
```

Several words are one query and **all of them have to match**, so a second word
narrows the result instead of widening it. No quoting is needed on any shell.

## The two layers

| | Catalog | Effect register |
|---|---|---|
| Answers | does this exist, and what is it called | what does it *do* on screen |
| Covers | 831 item, 1,095 NPC and 3,913 knowledge classes | roughly thirty ids, several of them refuted |
| Comes from | a UE4SS object dump, regenerated per game build | a person who looked at a screen |
| A wrong entry is caught by | the next tool run | nothing but another person looking |

Both ship inside `gore.exe`, so `find` needs no game install, no dump and no
generation step.

The register **annotates** the catalog; it does not gate it. An id the register
has never heard of is an id nobody has looked at yet — not an id that does
nothing. That is the overwhelming majority of them, and it is a perfectly good
hit.

Namespaces with no bundled catalog at all — textures, localization keys, FMOD
samples, voice lines — can still appear, through the register. A hit like that
says so outright, because "not in a catalog" there does not mean "unknown to the
game".

## Display names need one extra step

The catalogs carry class ids, categories and asset paths. They do **not** carry
display names: `ItFo_Potion_Health_01` is "Essence of Healing" only inside the
game's encrypted localization cache, which GORE cannot ship.

So `gore find` matches names through the shared text catalog, once you have
extracted it:

```powershell
gore loc extract        # once; also what gore-save and Mod Studio read
gore loc status         # what is currently extracted
```

Every result says which of the two states you are in, whether it found something
or nothing:

```
display names: searched — 3853 of 5839 catalog entries have one
               (shared loc catalog: 43851 ids, 18 languages)
```

```
display names: NOT searched — the bundled catalogs carry class ids and
               categories, not names. Run `gore loc extract` once to search
               names too; until then a word that appears in no id cannot match
```

Read that line before concluding a thing does not exist. Without the text
catalog, `gore find healing potion` genuinely returns nothing, and the item is
right there.

All shipped languages are searched at once, and the name is shown in the
language your query matched — search `Heilung` and you are answered in German —
so there is no language flag to get wrong. What is searched is the **name**;
item descriptions and lines of dialogue are not a name index, and full text
search belongs to [Text & dialogs](text-and-dialogs.md).

## Reading a hit

```
ItFo_Potion_Health_01
  from      bundled catalog · item · food
  name      Essence of Healing (english)
  class     /Script/Angelscript.ItFo_Potion_Health_01
  register  bundled · item · confirmed by 2 observations across 1 build
            effect: `m_Value` is the item's base worth, from which the
            trader's buy and sell prices derive.
            note: Vanilla `m_Value` is 25. At `m_Value` 1000, at Dexter on
            difficulty Gothic, it bought for 844 and sold for 423 …
  matched   display name (english)
```

| Line | What it is |
|---|---|
| `from` | which layer carries this id, and its domain and category |
| `name` | the display name, with the language it is written in |
| `class` | the name the game resolves — what goes into `overrides.toml` or a script |
| `module` · `loc key` | knowledge entries only: where the topic is declared and where its text lives |
| `register` | one line per register entry, each labelled with its own provenance |
| `matched` | why this hit is here, when the id does not show it |

`matched` is printed only when it says something the id does not. A hit found by
its German name is unreadable without it: nothing in `ItFo_Potion_Health_01`
looks like "Essenz heilender Kraft".

Hits are ordered by how strong the evidence is — an exact id, then an id
substring, then a register entry, then a category or class, then a display name
last. Searching every shipped language is what makes a name search work at all,
and it is also what lets `gore find logo` turn up five Portuguese subtitles;
nothing is dropped, only ordered, so the likely answer survives `--max`.

## What the register says about an id

```
  register  bundled · texture · disputed — 1 confirm, 1 refute across 2 builds
```

| Word | Meaning |
|---|---|
| `bundled` | the provenance of the file this came from. Sources are never blended, and every line says which one it is |
| `confirmed` / `refuted` | someone changed this and watched the game, and said what happened |
| `disputed` | some observations confirm, others refute. **Surfaced, never resolved** — usually a patch, a language or a display scale, and picking a side would throw the finding away |
| `unconfirmed` | nobody has checked this in game |
| `across N builds` | how much independent agreement stands behind it. Ten confirmations on one build are not three builds' worth |

One further line appears when it applies:

```
            1 observation claiming confirmed with no witness — recorded, not counted
```

An observation may claim `confirmed` or `refuted` only if it carries the
observer's own words. Without them the loader degrades it to `unconfirmed` and
keeps the claim visible, because "somebody said they saw this and showed
nothing" is a different state from "nobody has tried it". An assistant cannot
see a screen, so this is the line that keeps a confident guess from reading as a
fact.

Refuted entries are the ones nobody else can produce cheaply: a change that was
built and deployed exactly as written and did nothing. They are why the register
is worth carrying.

## Flags

| Flag | Effect |
|---|---|
| `--domain <NAME>` | one id namespace only: `item`, `npc`, `knowledge`, `texture`, `loc`, `audio`, `voice`, `asset`. An unknown one is refused with the list |
| `--max <N>` | stop after N hits (default 50). The result says how many matched and how to see the rest |
| `--json` | one JSON document instead of the blocks, carrying the same name-index notice |

`find` never fails on an empty result: a search that matched nothing is a real
answer, and exiting non-zero would bury the line explaining what was not
searched.

## Related

- [Item & stat values](items.md) — what to do with a class name once you have it
- [Text & dialogs](text-and-dialogs.md) — extracting, searching and editing the
  text this command reads its names from
- [Catalogs & data models](catalogs-and-models.md) — regenerating the catalogs
  after a game patch, and `gore location` for waypoint names
