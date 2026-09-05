# Characters (NPCs)

`gore npc` reads the game's characters — which ones exist, what one of them is
made of, and which world points spawn it — and writes the AngelScript that adds
a new character to the world or stops a shipped one being placed. Everything on
this page happens offline. The authoring commands write source, a manifest and a
build spec; compiling, packaging and deploying are separate steps you run
afterwards, and this group launches nothing.

The read half is proven. The authoring half is checked offline and **has never
been run in game** — read
[What is proven, and what is not](#what-is-proven-and-what-is-not) before you
build anything on it.

## The class chain

A character in Gothic 1 Remake is not a data record. It is a chain of
AngelScript classes, and each link names the next one through a class-scope
`default`:

```
USpawnAIAgentDefinition_<ID>          the spawn handle
  default AIAgentConfigClass    ->  UAIAgentConfig_Human_<ID>
                                      default m_CharacterDefinition -> UCharacterDefinition_Human_<ID>
                                        default m_UniqueName = n"<ID>"
                                        its SUPER CLASS is the faction (16 guild bases)
```

The faction is not a field. A character definition **derives** from one of the
16 guild base classes, so "which guild is this NPC in" is answered by its super
class, not by reading a value out of it.

The shipped cache declares 658 character definitions, 621 agent configs and
1053 spawn definitions, plus the surrounding parts a character refers to:
1923 daily routines, 1091 conversation settings and 846 visual definitions.
The NPC catalog compiled into `gore.exe` has 1095 rows.

## Placement lives in the level scripts

Placement is not part of that chain. A character appears in the world because a
**level script** — one world section's script module — calls `SpawnAIAgent`
from a world point:

```angelscript
class UWP_EZ_START_DIEGO_SPAWN : UWorldPointScript
{
    UFUNCTION()
    void OnWorldStart()
    {
        this.SpawnAIAgent(TSubclassOf<USpawnAIAgentDefinition>(USpawnAIAgentDefinition_OC_STT_Diego::StaticClass()), nullptr);
    }
}
```

There are 1764 such calls across 3939 `UWorldPointScript` classes. 14 of the
1764 pass the definition as a bare class reference, without the
`TSubclassOf<>(…::StaticClass())` wrapper — all of them creatures in the
Ancient Fortress and the Free Mine. `gore npc sites` reads both forms, so
neither shape is invisible.

Seventeen world sections carry characters. Pick the one you mean here, then
pass its module to `gore npc sites --level`:

| World section | Level script module | Characters |
|---|---|---|
| Old Camp | `LevelScripts.Map_x2_y1_OldCamp_AI_script` | 401 |
| Exchange Zone | `LevelScripts.Map_x2_y2_ExchangeZone_AI_script` | 242 |
| Ancient Fortress | `LevelScripts.Map_x1_y1_AncientFortress_AI_script` | 229 |
| New Camp | `LevelScripts.Map_x3_y2_NewCamp_AI_script` | 159 |
| Swamp Camp | `LevelScripts.Map_x0_y1_SwampCamp_AI_script` | 140 |
| Free Mine | `LevelScripts.Map_x3_y1_FreeMine_AI_script` | 139 |
| Monastery Ruins | `LevelScripts.Map_x1_y2_MonasteryRuins_AI_script` | 131 |
| Old Mine | `LevelScripts.Old_Mine_AI_script` | 114 |
| Sleeper Temple | `LevelScripts.SleeperTemple_AI_script` | 105 |
| Tundra | `LevelScripts.Tundra_AI` | 34 |
| Orc Graveyard | `LevelScripts.OrcGraveyard_AI` | 20 |
| Sleeper Dream | `LevelScripts.Map_SleeperDream_AI_script` | 20 |
| Shipwreck | `LevelScripts.ShipWreck_AI` | 13 |
| In Extremo (Old Camp) | `LevelScripts.Map_OldCamp_IE_script` | 7 |
| Stonehenge | `LevelScripts.StoneHenge_AI` | 5 |
| Sunken Tower | `LevelScripts.SunkenTower_OldCamp_AI_script` | 3 |
| Xardas' Tower | `LevelScripts.XardasTower_AI` | 2 |

`--level` matches on the module name, so a distinctive fragment such as
`XardasTower` or `OldCamp` is enough.

## `npc list` — the ids the game ships

```
$ gore npc list diego
human    OC_STT_Diego  CharacterDefinition_Human_OC_STT_Diego
human    OC_STT_Diego_Sleeper  CharacterDefinition_Human_OC_STT_Diego_Sleeper
2 of 2 shown
```

The filter is one positional word matched against the id and the class name.
`--category <human|creature|other>` narrows further, `--max <N>` caps the
printed rows (50 by default) while the final line still reports how many
matched, and `--json` returns the same rows as one document.

`list` answers from the catalog compiled into `gore.exe`. **It needs no game
installation**, which makes it the right first step when you only want the
exact id to hand to `show`.

## `npc show` — the whole chain and its spawn sites

```
$ gore npc show OC_STT_Diego
OC_STT_Diego
  spawn definition       USpawnAIAgentDefinition_OC_STT_Diego
  ai agent config        UAIAgentConfig_Human_OC_STT_Diego
  character definition   UCharacterDefinition_Human_OC_STT_Diego
  guild base             UCharacterDefinition_Human_OldCamp_Shadow
  unique name            OC_STT_Diego
spawns at 2 site(s):
  UWP_INTRO_FALL3  in LevelScripts.Map_x2_y2_ExchangeZone_AI_script
    translation: measured, no known difference
  UWP_EZ_START_DIEGO_SPAWN  in LevelScripts.Map_x2_y2_ExchangeZone_AI_script
    translation: measured, no known difference
```

`show` takes the **exact** id, not a filter; it resolves
`USpawnAIAgentDefinition_<ID>` and walks the chain from there. A link it cannot
resolve prints as `—` rather than being dropped, so a gap in the chain never
looks like a character without a guild.

Unlike `list`, `show` reads the installed script cache. `--cache <PATH>` reads
an exact cache instead, `--game <ROOT>` picks the install; without either, the
configured game path is used and then Steam auto-detect. `--json` returns the
chain, the sites and the translation judgement as one document.

`show` deliberately emits only the modules it needs — the 29 modules in the
`LevelScripts.` namespace plus the few that declare the classes in the chain.
That is not an optimisation detail you can ignore: one shipped module,
`Map.MainMap.WorldPointManagerConfig_MainMap`, needs over five minutes on its
own to recover its class defaults, against about two seconds for an ordinary
module. A command that emitted the whole tree would not answer.

## `npc sites` — where the level scripts spawn

```
$ gore npc sites --level XardasTower
UOW_XT_DEMON_LESSER_SPAWN_WP  USpawnAIAgentDefinition_XT_XardasDemon  LevelScripts.XardasTower_AI
UXT_Skeleton_SPAWN_WP  USpawnAIAgentDefinition_Skeleton_XardasServant  LevelScripts.XardasTower_AI
2 of 2 shown
```

Each row is the world point, the spawn definition it names, and the level
script it lives in. `--level <TEXT>` keeps the sites whose module contains that
text, `--npc <ID>` keeps only the sites that spawn one character, `--max <N>`
caps the printed rows, and `--json` returns them as one document. Like `show`,
it reads the installed cache and takes `--cache` / `--game`.

## Reading the `translation:` verdict

Every site `show` prints carries one line about its level script. It reports
what is known about **recompiling that module** — because changing where a
character stands means recompiling the level script that places it. There are
exactly three states, and they must not be conflated:

| Line | What it means |
|---|---|
| `translation: measured, no known difference` | Somebody measured this module on this game version, and its recompile produced no known difference. |
| `translation: N divergent function(s), M behaviour risk(s)` | Measured, and the recompile is known to differ. `N` functions came back different; `M` of those differences are behaviour risks. |
| `translation: NOT MEASURED for this game version` | **Nobody measured this module on this game build.** This is an absence of evidence, not a clean bill of health. Do not read it as reassurance. |

The judgement is keyed to the exact cache and Binds seals, so a game update
turns measured lines into `NOT MEASURED` rather than silently carrying an old
verdict forward. `--json` returns the same three states as
`{"measured": false}` or `{"measured": true, "divergent_functions": …,
"behaviour_risks": …}`, so a caller does not have to parse the sentence.

The same risk model backs `gore as emit` and `gore as compile-module`; the
module-level detail is in [Scripts (AngelScript)](scripts.md).

## Authoring a character

Four commands make one path, and each one prints the next:

| Command | What it does |
|---|---|
| `gore npc new <ID> --from <NPC> --at <POINT> -o <DIR>` | Write a workspace: the new character's module, and the level script one spawn line longer. |
| `gore npc delete <NPC> -o <DIR>` | The other opening move: take a shipped character's spawn line out again. |
| `gore npc check <DIR>` | Read that workspace back and refuse anything outside the contract. |
| `gore npc stage <DIR>` | Write the build spec, and print the commands that compile and package it. |

`gore npc text` stands beside them and writes the display name. None of the four
compiles, packages, deploys or launches anything — `stage` prints the commands
that do, and you run them.

### `npc new` — derive a character and place it

```
$ gore npc new GORE_TEST_NPC --from OC_STT_Diego --at UOW_XT_DEMON_LESSER_SPAWN_WP --waypoint FP_OC_SMALLTALK_33 -o work/npc
authored GORE_TEST_NPC in work/npc
  GORE_TEST_NPC.as  the character, 6 classes
  XardasTower_AI.as  one added spawn line at UOW_XT_DEMON_LESSER_SPAWN_WP
  translation: measured, no known difference
next: gore npc check work/npc
```

The workspace it writes:

```
work/npc/
  GORE_TEST_NPC.as                 the character: all its classes in one module
  XardasTower_AI.as                the level script, one line longer
  pristine/XardasTower_AI.as       the untouched copy check compares against
  gore-npc-edit.json               the manifest
```

The six classes are the chain from the top of this page plus the parts hanging
off it: character definition, visuals, AI agent config, spawn definition, the
conversation settings the voice comes from, and — with `--waypoint` — a daily
routine. Vanilla spreads those across four modules plus two shared ones
(`Spawning/SpawningDefinition_Human.as`, `InteractiveObjects/NpcVisualLibrary.as`).
Putting all six in one module of the character's own is what keeps the mod off
those shared files, and it costs nothing: AngelScript registers classes
globally, so which module a class lives in is free.

`-o` must name a directory that does not exist. `--at` takes a world point from
`gore npc sites`; an unknown one is refused, with the nearest names it was not.

**`--from` takes a character, not a guild.** The 16 guild bases carry the
faction and nothing else — no model, no stats, no voice — so a character derived
straight from one would stand in the world with no appearance at all. `--from`
is what gives the new character its looks, stats and voice. `--guild <BASE>`
then swaps *only* the faction, by changing which class the character definition
derives from: `--from OC_STT_Diego --guild OldCamp_Guard` is Diego's body in the
guards' faction.

**The appearance is borrowed, not built.** 817 shipped characters carry a
prebaked model and not one of them assembles its looks from parts at runtime. A
new id has no prebaked model of its own, so the generated visuals class keeps
the template's — `default m_PreBakedName = "OC_STT_Diego"`. `--modular-visuals`
takes the other path instead, and the source it generates says in a comment of
its own that nothing ships that way and it is unproven.

`--trader` adds an empty trader configuration. `--waypoint` gives the character
a daily routine that sends it to one spot; without it, the character has no
routine at all. `check` looks that waypoint up in the bundled location catalog,
because the game ignores an unknown one without a word.

### `npc delete` — stop a shipped character being placed

```
$ gore npc delete XT_XardasDemon -o work/demon
XT_XardasDemon will no longer be placed, from work/demon
  removed from UOW_XT_DEMON_LESSER_SPAWN_WP
  translation: measured, no known difference
  NOTE: this only stops future placement. A save that already spawned XT_XardasDemon still carries that body
next: gore npc check work/demon
```

Read that note literally. Removing the spawn line changes what the level script
does at world start; it does not reach into a save. A character a save has
already seen is a body in that save and stays one. Only a new game starts
without it.

A character placed from more than one level script is refused rather than half
removed — one bundle entry carries one edited level script, so that would take
one mod per script — and the message names the scripts.

### Why an authored character does not derive from its template

A character's own class is almost always a leaf: nothing in the shipped game
derives from `UCharacterDefinition_Human_OC_STT_Diego`. The compiler declares
the generated `__InitDefaults()` of such a class **final**, and a subclass that
brings its own `default` statements needs its own `__InitDefaults`. So the
obvious shape does not compile:

```
GORE_AS_COMPILER_ERROR: Method 'void UCharacterDefinition_Human_OC_STT_Diego::__InitDefaults()'
declared as final and cannot be overridden
```

`new` and `clone` therefore climb from the template to the nearest ancestor
that has siblings, and write everything skipped along the way into the file.
For Diego that ancestor is `UCharacterDefinition_Human_OldCamp_Shadow`, which
29 characters share, and the file carries his 44 values.

That is why the generated header says `derived from OC_STT_Diego` while the
class line names the guild base: the character is Diego's, the parent is what
the compiler allows.

One class needs no climb — `UCharacterVisualsDefinition_Human_OC_STT_Diego` is
not a leaf, because Diego's Sleeper variant derives from it. That single
difference is what separated the one class that compiled from the three that
did not, the first time this was built for real.

### `npc clone` — the same character, with its numbers on the table

`new` derives: the generated class states its identity and inherits everything
else without saying so. That is right for a fresh character, and useless when
the point is to change something, because nothing is there to change.

```
$ gore npc clone OC_STT_Diego --id DIEGO_TWIN --at UOW_XT_DEMON_LESSER_SPAWN_WP -o work/twin
authored DIEGO_TWIN in work/twin
  DIEGO_TWIN.as  the character, 5 classes
  XardasTower_AI.as  one added spawn line at UOW_XT_DEMON_LESSER_SPAWN_WP
```

`clone` writes the template's resolved defaults into the file — 51 lines for
Diego: his level, his health, every resistance, his whole starting inventory,
his skills, his personality, his combat AI. Change what you want and leave the
rest.

```angelscript
class UCharacterDefinition_Human_DIEGO_TWIN : UCharacterDefinition_Human_OC_STT_Diego
{
    default m_UniqueName = n"DIEGO_TWIN";
    default m_CharacterVisualsDefinition = UCharacterVisualsDefinition_Human_DIEGO_TWIN::StaticClass();
    default m_CharacterType = GameplayTag::AIAgent_Human_Shadow;
    default m_InitialGuildEffect = UGE_Guild_Human_OldCamp_ShadowLeader::StaticClass();
    default m_Personality = UGothicCharacterPersonality_Brave_Archer_Patient::StaticClass();
    default SetAttributeValue("AttributeSet_LevelProgression.Level", 100.0f, TSubclassOf<UDifficultySettings>(nullptr));
    …
}
```

The two identity lines stay the generator's. Copying `m_UniqueName` across
would give the clone the template's name, and the save keys a character by that
name.

### `npc checkout` — change a shipped character's values

```
$ gore npc checkout OC_STT_Diego -o work/diego
checked OC_STT_Diego out into work/diego
  CharacterDefinition_OC_STT_Diego.as  1 classes from AI.AIAgent.Human.Config.OC_STT_Diego.CharacterDefinition_OC_STT_Diego
  translation: measured, no known difference
  edit the values; class names and their parents have to stay as they are, because they are the character's identity in the cache
next: gore npc check work/diego
```

What comes out is the module that declares the character's
`CharacterDefinition` — its level, health, resistances, strength and dexterity,
its starting inventory, its guild parent, its skills, its personality and its
combat AI. For Diego that is 44 `default` statements. Edit the values and run
`check`.

Two parts of a character are deliberately **not** checked out: its appearance
lives in `InteractiveObjects/NpcVisualLibrary.as` and its spawn definition in
`Spawning/SpawningDefinition_Human.as`, each shared by hundreds of characters.
Replacing one of those to change a single character would put a module every
other character depends on into your mod.

The guard here is the mirror image of the one for authoring. Values may change
freely — that is the whole point. Class names and their parent classes may not:

```
$ gore npc check work/diego
  [blocking] class UCharacterDefinition_Human_OC_STT_Diego is gone. A shipped class may change its values, but removing or renaming it produces a different symbol that no longer matches the base cache
  [blocking] class UCharacterDefinition_Human_RENAMED is new. Checking a shipped character out is for changing its values; a new class needs `gore npc new`, which carries the contract for one
```

A checkout touches one shipped module, so `stage` sends it down the fast
`compile-module` route — no source tree, no fifteen-minute wait.

### `npc check` — the diff guard

```
$ gore npc check work/npc
translation: measured, no known difference
no problems found in work/npc
offline-checked only: that this character appears, keeps its routine and survives a save is not proven in game
next: gore npc stage work/npc
```

The guard exists because of the company a spawn line keeps. The Old Camp level
script places 401 characters, and splicing recompiles the whole module — so a
line that moved by accident would surface in game as somebody else's character
misbehaving, a long way from anything you edited. `check` diffs the edited
level script against the `pristine/` copy line by line and blocks on every
change that is not a spawn line of the character being authored, naming the
line number and quoting what was on it.

It also blocks on:

- a workspace authored against a different script cache than the installed one
  — a game patch, or a different `--cache`. Checking against the wrong cache is
  not checking;
- an id the game already ships, whose authored module would collide with the
  shipped one;
- a level script that did not change at all, which has nothing to build.

An unresolvable waypoint is a warning rather than a block: the character is
still valid, it simply never goes there.

### `npc stage` — the build spec and the commands

```
$ gore npc stage work/npc --tree work/tree --mod-name GoreTestNpc
reusing the source tree in work/tree (7317 modules)
wrote work/npc/spec.json
now run:
  gore as compile "work/tree" -o "work/npc/full.Cache" --mini "work/npc/GoreTestNpc.mini.Cache" --work-dir "work/npc.work" --backend standalone
  gore mod build --spec "work/npc/spec.json" -o "work/npc/build"
then: gore mod deploy --bundle work/npc/build/GoreTestNpc
```

**There are two speed classes, and `stage` picks — you do not.** A new
character touches two modules, one new and one shipped, and the shipped one
refers to the new one. Separate mini-caches cannot depend on each other, so
those two have to be compiled together, which is the complete-tree route above.
A suppression touches exactly one shipped module and goes through
`gore as compile-module`, which is many times faster and needs no tree at all:

```
$ gore npc stage work/demon --mod-name NoDemon
wrote work/demon/spec.json
now run:
  gore as compile-module --backend standalone --op edit --module "LevelScripts.XardasTower_AI" --rel-path "LevelScripts/XardasTower_AI.as" --source "work/demon/XardasTower_AI.as" --work-dir "work/demon.work" -o "work/demon/NoDemon.mini.Cache"
  gore mod build --spec "work/demon/spec.json" -o "work/demon/build"
then: gore mod deploy --bundle work/demon/build/NoDemon
```

That is why `--tree` is required for a new character and pointless for a
suppression. Emitting all 7317 modules takes around 19 minutes, nearly all of it
one module (`Map.MainMap.WorldPointManagerConfig_MainMap`). The tree is
therefore written once per game version, stamped with the cache it came from,
and reused — which is what the `reusing` line reports. A tree stamped with a
different cache is refused rather than quietly mixed with a newer one.

`stage` runs neither command itself. A quarter of an hour is not something a
tool should start without being asked.

### `npc text` — the name above the lines

A character's localization id is its id in lowercase, so the display name needs
no lookup and no installation:

```
$ gore npc text GORE_TEST_NPC --name "Hannes" -o work/name.json
wrote work/name.json
  gore_test_npc -> "Hannes" in both German columns
next: gore loc import --edits work/name.json
```

```json
{
  "gore_test_npc": {
    "german": "Hannes",
    "german_new": "Hannes"
  }
}
```

Both German columns, deliberately. Where `german_new` exists it wins over
`german`, so a document that sets only `german` is a silent no-op — a mistake
that has cost this project time before. `--english <NAME>` fills the three
English columns the same way. The file goes into the game through
`gore loc import --edits`, like any other text edit; see
[Text & dialogs](text-and-dialogs.md).

### Two NPC mods for the same world section do not run together

Every authored character carries its own compiled copy of one level script, and
that script is the whole world section. Two mods that both place a character in
the Old Camp each carry a complete version of the same module, so installing
both would mean taking one copy and discarding the other, silently losing
whichever character lost. The Manager refuses that rather than installing the
mix: a mini that would keep only some of the modules it carries is rejected with
*a multi-module mini composes as one unit*. `gore mgr analyze` names the shared
module as a hard conflict before it gets that far. Two NPC mods in *different*
world sections touch different modules and coexist normally — see
[Running many mods](mod-manager.md).

## What is proven, and what is not

Read this before you build on it.

| | |
|---|---|
| **The three read commands** | Everything `list`, `show` and `sites` report is proven and produced entirely offline. `list` needs no installation at all; `show` and `sites` read a script cache and launch nothing. |
| **What `check` verifies** | Proven, offline, about the workspace in front of it: that the edited level script differs from its pristine copy in nothing but spawn lines of the character being authored, that the workspace was authored against the installed script cache, that the id is not one the game already ships, and that the routine's waypoint is a spot the bundled catalog knows. That is a statement about source text, not about the game. |
| **Recompiling a level script does not disturb its neighbours** | On the measured game build, BuildID `24878692`, the complete script tree emitted and recompiled unchanged produces a **byte-identical** cache: SHA-256 `7A18F954E32AF30FC24AE3A66EA35D3B5CB98560C8F5083C7846FC9CE1D77511`, 124,459,412 bytes, 7317 modules. On that build, recompiling a level script therefore cannot change code the author did not touch. |
| **Per-module translation** | Whether one particular level script survives its own recompile is the separate, per-module judgement the `translation:` line reports. Byte-identity of the whole tree does not answer it for a build nobody measured. |

### Not proven in game

The authoring path has never been through the game. None of the following has
been observed even once, and nothing on this page should be read as a hint that
it works:

- That an authored character appears in the world at all.
- That it looks like the character it was derived from.
- That it keeps its daily routine, or goes anywhere.
- That it survives a save and a reload.
- That its save key — the identity a save file records it under — works.

That is also what `check` and `stage` say in the lines they end with:
*offline-checked only* and *offline-prepared only*. Those are not modesty. Until
somebody builds one of these, deploys it and looks, the honest description of an
authored character is source that compiles.

Changing a shipped character's values is `npc checkout`, described above, and
carries the same offline-only status. What it does not reach: visuals and the
spawn definition, which live in modules hundreds of characters share, and what
a character says. Use the surfaces that do:
[Dialog authoring](dialog-authoring.md) for what a character says,
[Offline default patching](angelscript-defaults.md) for a single class default,
and [Scripts (AngelScript)](scripts.md) for the general emit/recompile/splice
route with its own risk reporting.

## Related

- [Reading and editing dialog trees](dialog-trees.md) — what a character says
- [Dialog authoring](dialog-authoring.md) — authoring that conversation
- [Scripts (AngelScript)](scripts.md) — emit, recompile and splice a module,
  and the risk report behind the `translation:` line
- [Bundling & deploying](bundles.md) — what `stage` writes a spec for, and what
  `gore mod build` and `gore mod deploy` then do with it
- [Running many mods](mod-manager.md) — load order and the conflict report that
  names two mods over one level script
- [Text & dialogs](text-and-dialogs.md) — where an `npc text` document goes
- [Finding things](find.md) — `gore find --domain npc` searches the same
  catalog `npc list` reads, alongside every other id namespace
- [Catalogs & data models](catalogs-and-models.md) — regenerating that catalog
