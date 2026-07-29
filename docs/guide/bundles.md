# Bundling & deploying

A **bundle** combines every domain — item overrides, localized text, audio,
voice archives, textures, scripts, and dialog topics — into one mod that
deploys and undeploys as a unit. This is the same engine
[Mod Studio](../../apps/mod-studio/README.md) drives.

## The build spec

Write a `spec.json`:

```json
{
  "meta": { "name": "MyMod", "version": "1.0.0", "author": "you" },
  "overrides": [ { "class": "ItFo_Apple", "field": "m_Value", "value_int": 500 } ],
  "loc_edits": { "some_text_id": { "german": "…" } },
  "audio":   [ { "bank": "SFX.bank", "sample": "Foo", "wav_path": "foo.wav" } ],
  "voice":   [ { "archive": "german_new.zip", "op": "replace", "archive_path": "NPC/Hero/DIA_Foo.ogg", "ogg_path": "DIA_Foo.ogg" } ],
  "texture": [ { "asset": "/Game/UI/.../T_Foo", "image_path": "foo.png" } ],
  "scripts": [ { "op": "add", "module_name": "MyModule", "mini_cache": "MyModule.cache" } ],
  "dialog_topics": [ { "id": "viper-test", "participant_name": "om_stt_viper_302", "topic_class": "/Script/Angelscript.ChoiceMyViper", "sentinel_class": "/Script/Angelscript.ChoiceStt302ViperExit" } ]
}
```

Every section is optional; `delay_ms` may be set alongside `overrides` to defer
the CDO patch. Each section maps to the domain guide of the same name:
[items](items.md), [text](text-and-dialogs.md), [audio](audio.md),
[voice](voice.md), [textures](textures.md), [scripts](scripts.md).

## Build, deploy, undeploy

```powershell
gore mod build    --spec spec.json -o build      # → build\MyMod\ (manifest + payloads)
gore mod deploy   --bundle build\MyMod --game "$GAME"
gore mod undeploy --game "$GAME"                 # restore everything
```

What deploy does per domain:

| Section | Deployment |
|---|---|
| `overrides` | a generated UE4SS Lua mod into `ue4ss\Mods\` |
| `loc_edits` | in-place `.lcache` rewrite, original backed up to `*.gore-bak` |
| `audio` | in-place bank rewrite, original backed up to `*.gore-bak` |
| `voice` | transactional ZIP rewrite under `G1R\Story\VoiceOver` |
| `texture` | cooks + packs a Zen triplet into `~mods\` (additive) |
| `scripts` | splices the mini-caches into the script cache, backed up to `*.gore-bak` |
| `dialog_topics` | guarded runtime topic registration |

`gore mod undeploy` restores every backup and removes every additive container.

## Dialog topics

A `dialog_topics` entry registers an authored AngelScript topic at the target
conversation's natural UI boundary. It needs explicit identities: the
participant, your authored `topic_class`, and a vanilla `sentinel_class`.

For a state-dependent choice, add `"allow_hidden": true`. A clean zero-match
after `IsVisible_Implementation` is then accepted as conditional, while
duplicates and mixed identity/class matches still fail closed. The default
remains strict: the registered topic must reach both UI proof stages.

Full template, runtime evidence, and safe test order:
[AngelScript dialog authoring](dialog-authoring.md).

## Voice packaging details

Voice entries are packaged into a versioned format-1 `voice/manifest.json` with
bundle-relative, validated Ogg payloads.

- `archive` must be one `.zip` filename under `G1R\Story\VoiceOver`.
- `archive_path` is a forward-slash `.ogg` member path.
- `replace` requires that member's exact, case-sensitive stored path.
- `add` requires that the path does **not** exist.

`add` is archive-safe, but whether the game resolves a brand-new voice path is
still runtime-dependent; replacements are the established deployment path.

Direct deploy and manager apply group edits into one verified rewrite per ZIP
and always rebuild from the pristine or prior-backup archive. A referenced
archive missing from the install is a hard preflight error: deployment refuses
to create a partial voice patch. All manifests, payload paths, files, and Oggs
are validated before an active loadout is transactionally replaced.

### Disk space

Each candidate ZIP is written and verified beside the archive it replaces before
anything is published, so the game volume needs temporary free space comparable
to the archives being rewritten. Running out of space or memory fails before a
live archive is changed.

## Running several mods at once

`gore mod deploy` deploys **one** bundle. For a library of mods with load order
and conflict detection, use [`gore mgr`](mod-manager.md) or the
[Mod Manager](../../apps/mod-manager/README.md) app, which consume the same
bundles.

## Other helpers

```powershell
gore scaffold MyMod -o "$GAME\...\Mods"   # empty hand-written gore-lua mod skeleton
gore deploy-shared --game "$GAME"         # install the gore-lua helpers (for custom Lua mods)
gore package mod_dir/ -o MyMod.zip        # zip a Lua mod for sharing
```

`deploy-shared` takes an optional `--src` for unusual layouts; by default it
locates the shared tree relative to the `gore` executable, independent of the
working directory.
