# Getting started

This is the entry point for modding Gothic 1 Remake with GORE. It covers
installing the `gore` CLI, pointing it at your game, and choosing the right
tool for the job.

If you only want to edit a **save game**, you do not need any of this — use the
[save editor](../../apps/save-editor/README.md) instead. It never touches the
game install.

## Install the CLI

Two ways to get `gore.exe`:

**Download a release.** Grab a `gore-cli-v*` asset from the
[releases page](https://github.com/dh0er/gore/releases). The zip contains
`gore.exe` and this whole guide under `docs\` — so the
documentation is available offline, right next to the binary.

To read it offline, open `docs\guide.html`: one browsable file with every page,
a collapsible sidebar and a filter box. The `docs\*.md` files next to it are the
same content in Markdown, for `grep`. The [MCP server](mcp.md) serves that guide
too, but from a copy compiled into `gore.exe` rather than from these files —
editing them changes what you read, not what an assistant is told. You can
regenerate the HTML at any time with `gore guide html`.

**Build it yourself.** Requires a stable Rust toolchain:

```powershell
cargo build --release -p gore
# → target\release\gore.exe
```

See [Building](../development.md) for the full toolchain requirements and the
`build.py` orchestrator that also builds the GUI apps.

GORE is Windows-only. Every example in this documentation is PowerShell, assumes
`gore` is on your `PATH`, and uses the variable `$GAME` for your install root —
the folder that contains `G1R\`:

```powershell
$GAME = 'D:\SteamLibrary\steamapps\common\Gothic 1 Remake'
```

Use double quotes when you build a path from it (`"$GAME\G1R\..."`); single
quotes do not expand variables.

## Point GORE at the game

Set the game path once, and every command that needs it can omit `--game`:

```powershell
gore config set game-path $GAME     # an install root or the game .exe
gore config detect                  # …or auto-detect a Steam install and save it
gore config list                    # show the stored value + resolved root
gore config get game-path           # print just the value (non-zero exit if unset)
gore config unset game-path         # clear it
gore config path                    # where config.json lives
```

Resolution precedence for every command that needs the install:

1. an explicit `--game` on the command line (`--lcache` for `gore loc`, `--exe`
   for `gore as diagnostics-check`),
2. the configured `game-path`,
3. Steam auto-detect.

The value is stored in a shared `config.json` (`gore config path`) that the GUI
apps read too, so the install is configured in exactly one place.

## Check the setup

```powershell
gore doctor
```

One read-only pass over everything the rest of this guide assumes. One line
each: where the install is and which of the three sources above
answered, whether that folder holds Gothic 1 Remake, what is deployed, whether the `~mods`
override folder is there, what an interrupted run left behind, whether the
executable is running, whether the authenticated standalone AngelScript
compiler matches the installed cache/API, and whether the shared localized-text
catalog still describes the installed `.lcache`.

Every `problem` carries a `fix:` line. A `note` is informational; a `skipped`
check can point to the earlier missing prerequisite instead. Abridged example:

```
ok      game path     D:\SteamLibrary\steamapps\common\Gothic 1 Remake (source: config)
ok      deployment    nothing is deployed (no deploy record in the install)
ok      AS standalone authenticated standalone compiler is compatible with this cache/API; native diagnostics are available without a game launch
problem loc catalog   43851 ids in 19 language(s), but stale: extracted from 37081808 bytes and the installed cache is now 37093440
                      fix: … Run 'gore loc extract' so the shared catalog describes the file that is actually installed

check(s): the counts on the last line say how many of each verdict you have
```

| Verdict | Meaning |
|---|---|
| `ok` | checked, nothing to do |
| `note` | worth knowing, not a fault — the executable is running, the catalog was extracted elsewhere |
| `problem` | a reason a mod would silently do nothing, or a mess somebody has to clean up |
| `skipped` | not answerable, because something this check reads is absent; whatever reported that absence is on a line above |

**Exit code 0 either way.** A finding is not the command failing, and this is the
command you reach for once something has already gone wrong — no wrapper should
read its exit code as "this is broken too". To act on the findings from a
script, use `gore doctor --json`: each check carries a stable `id` and its own
`verdict`, and the top level carries `ok` / `note` / `problem` / `skipped`
counts.

Nothing here writes, creates or removes anything. The `deployment` check hashes
the files the deploy record claims, exactly as `gore mgr status` does. The
standalone-compiler check separately authenticates its package and verifies that
the installed compiler inputs match a qualified cache/API.

What each check reads — and therefore what it can and cannot prove — is in the
[CLI reference](cli-reference.md#doctor).

## Which tool for which job

| You want to… | Use |
|---|---|
| change values, text, audio, textures or scripts of the game | the `gore` CLI (this guide) |
| do the same without a terminal, for one mod | [Mod Studio](mod-studio.md) |
| install and order **many** mods at once | [Mod Manager](../../apps/mod-manager/README.md) or [`gore mgr`](mod-manager.md) |
| edit your saved progress | [Save Editor](../../apps/save-editor/README.md) |

The Flutter GUIs call the same Rust engine as the CLI through a `dart:ffi`
bridge. Use the CLI for expert and automated workflows; use a GUI when its
guided presentation is more useful. They share contracts and state rather than
maintaining separate implementations.

## How each domain becomes a mod

Every domain produces a mod a different way, and each one is usable on its own:

| Domain | Mechanism | Touches | Guide |
|--------|-----------|---------|-------|
| Item/stat values | rewritten class `default`, compiled into the script cache | the Shipping script cache | [items.md](items.md) |
| Text & dialogs | re-encrypted `.lcache` | the localization cache, in place | [text-and-dialogs.md](text-and-dialogs.md) |
| Audio | re-packed FMOD `.bank` | the sound bank, in place | [audio.md](audio.md) |
| Voice-over | copy-on-write localized ZIP edit | the selected language archive, in place | [voice.md](voice.md) |
| Textures | additive UE5 IoStore Zen triplet | the game's `~mods\` folder | [textures.md](textures.md) |
| Cooked DataAssets | additive Zen triplet from a fixed-leaf patch | the game's `~mods\` folder | [dataassets.md](dataassets.md) |
| Scripts | edited precompiled AngelScript cache | the script cache (experimental) | [scripts.md](scripts.md) |

You can ship each on its own, or combine them into one deployable **bundle** —
see [Bundling & deploying](bundles.md).

## Backups and safety

- In-place edits (localization, audio, script cache) write a `*.gore-bak`
  backup of the original file first.
- Texture and DataAsset mods are **additive**: they drop a container into
  `~mods\` and never modify an original game file.
- `gore mod undeploy` restores everything a bundle changed;
  `gore mgr reset` does the same for the whole managed loadout.
- Voice-over, texture pack, and DataAsset commands never modify their input and
  refuse to overwrite an existing output path.
- Close the game before any command that writes into the install. Script
  compilation and deployment take an install-wide lock
  (`.gore-install-mutation.lock`) so two GORE processes cannot fight, but the
  game itself does not participate in that lock.

## A first mod: Wiesel's letter

Diego is the first man who talks to you. This mod puts a new shadow, Wiesel,
in the Old Camp and gives Diego two new lines. The first sends you to Wiesel
for a tally of ore that never reached the storehouse. The second pays you once
you bring it back. Start a new game after deploying. A save that already passed
the opening conversation will not show a topic the game has already left behind.

Wiesel borrows Diego's appearance. A new id has no baked model of its own; the
generated visuals keep the template's `m_PreBakedName`. That is enough for a
first mod. Changing the face is [character authoring](npc-authoring.md).

### 1. Place Wiesel

List spawn points in the Old Camp and pick one that is already standing when
you leave Diego:

```powershell
gore npc sites --level OldCamp
gore npc new GORE_OC_WIESEL --from OC_STT_Diego --guild OldCamp_Shadow `
  --at <POINT_FROM_THE_LIST> --waypoint FP_OC_SMALLTALK_33 -o work/wiesel
gore npc text GORE_OC_WIESEL --name "Wiesel" -o work/wiesel
gore npc check work/wiesel
gore npc stage work/wiesel
```

`--at` must be a world point `npc sites` printed. An unknown name is refused.
`FP_OC_SMALLTALK_33` is only the daily spot, not the spawn. `stage` prints the
compile command. Run that, then the bundle commands it prints. The character
guide has the contract: [Characters](npc-authoring.md).

### 2. Diego offers the errand

```powershell
gore dialog new-topic oc_stt_diego --caption "Die Liste aus dem Lager." `
  --class UChoiceGoreWieselErrand --mod-name GoreWieselLetter -o work/diego
gore dialog new-topic oc_stt_diego --caption "Hier ist Wiesels Liste." `
  --class UChoiceGoreWieselReturn --mod-name GoreWieselLetter -o work/diego-return
```

`new-topic` checks Diego's conversation out and appends one class. Open the
generated `.as` and give the two topics these bodies. `Remembers` and
`Remember` are the hero's knowledge flags. `AddItemToInventory` is the same
call the game uses to put ore into a pocket.

```angelscript
class UChoiceGoreWieselErrand : UTopic_Hero__OC_STT_DIEGO
{
    default Caption = FText::FromString("Die Liste aus dem Lager.");
    default PriorityRank = 2;

    UFUNCTION(BlueprintOverride)
    bool IsVisible() const
    {
        AGothicCharacterState Hero = this.GetCharacter(n"Hero");
        return Hero != nullptr && !Hero.Remembers(n"gore_wiesel_letter_started");
    }

    UFUNCTION(BlueprintOverride)
    void Act()
    {
        AGothicCharacterState Hero = this.GetCharacter(n"Hero");
        if (Hero != nullptr)
            Hero.Remember(n"gore_wiesel_letter_started");
        this.EndConversation();
    }
}

class UChoiceGoreWieselReturn : UTopic_Hero__OC_STT_DIEGO
{
    default Caption = FText::FromString("Hier ist Wiesels Liste.");
    default PriorityRank = 2;

    UFUNCTION(BlueprintOverride)
    bool IsVisible() const
    {
        AGothicCharacterState Hero = this.GetCharacter(n"Hero");
        return Hero != nullptr
            && Hero.Remembers(n"gore_wiesel_letter_carried")
            && !Hero.Remembers(n"gore_wiesel_letter_paid");
    }

    UFUNCTION(BlueprintOverride)
    void Act()
    {
        AGothicCharacterState Hero = this.GetCharacter(n"Hero");
        if (Hero != nullptr && !Hero.Remembers(n"gore_wiesel_letter_paid"))
        {
            ::AddItemToInventory(Hero, UItMi_Orenugget, 5, EInventoryTypes(1));
            Hero.Remember(n"gore_wiesel_letter_paid");
        }
        this.EndConversation();
    }
}
```

`dialog new-topic` already writes `DebugId`, the base class and `BlueprintOverride`.
Keep those. Only replace `IsVisible` and `Act`. Then:

```powershell
gore dialog check work/diego
gore dialog stage work/diego --mod-name GoreWieselLetter
```

Run the compile line `stage` prints. Same-module topics do not need a loader
beside the game. The rules are in [Dialog authoring](dialog-authoring.md).

The two topics are two workspaces because each `new-topic` starts from the
pristine conversation. Copy the return class into the first workspace's `.as`
before `check`, so one module carries both classes. `check` refuses a class
that is not in that file.

### 3. Wiesel hands the tally over

Wiesel has no conversation yet. `new-conversation` adds the first one in the
module `npc new` already wrote:

```powershell
gore dialog new-conversation GORE_OC_WIESEL --caption "Die Liste für Diego." `
  --class UChoiceGoreWieselHandover --mod-name GoreWieselLetter -o work/wiesel-talk
```

Set his only topic so it shows after Diego's errand and pays the knowledge
flag the return line is waiting for:

```angelscript
UFUNCTION(BlueprintOverride)
bool IsVisible() const
{
    AGothicCharacterState Hero = this.GetCharacter(n"Hero");
    return Hero != nullptr
        && Hero.Remembers(n"gore_wiesel_letter_started")
        && !Hero.Remembers(n"gore_wiesel_letter_carried");
}

UFUNCTION(BlueprintOverride)
void Act()
{
    AGothicCharacterState Hero = this.GetCharacter(n"Hero");
    if (Hero != nullptr)
        Hero.Remember(n"gore_wiesel_letter_carried");
    this.EndConversation();
}
```

```powershell
gore dialog check work/wiesel-talk
gore dialog stage work/wiesel-talk --mod-name GoreWieselLetter
```

### 4. Build, deploy, play

`stage` writes a build spec whose script mini-cache is the mod. Finish with
the commands it prints, then:

```powershell
gore mod deploy --bundle <the bundle directory stage named>
```

Start a new game. Talk to Diego and take "Die Liste aus dem Lager." Find
Wiesel at the Old Camp point you spawned him on and take "Die Liste für
Diego." Bring that line back to Diego. He pays five ore nuggets, and neither
errand line appears again.

There is no journal page in this first mod. The three knowledge flags are the
quest. A journal entry is a `UQuest` class, the same shape as the batch
fixture under `scripts/fixtures/npc-batch-tests/quest/`, and it is more than
this walkthrough compiles.

If nobody new is standing in the camp, run `gore doctor` and `gore npc check`
before looking at the dialog. A wrong `--at` never reaches the game.

## Next steps

- [Characters](npc-authoring.md)
- [Dialog authoring](dialog-authoring.md)
- [Text & dialogs](text-and-dialogs.md)
- [Bundling & deploying](bundles.md)
- [CLI reference](cli-reference.md)
