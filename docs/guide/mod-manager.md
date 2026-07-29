# Running many mods (`gore mgr`)

`gore mod` deploys one bundle. `gore mgr` owns the multi-mod story: a library,
a load order, conflict analysis, and one composed deployment of the whole
enabled set. It is the CLI behind the
[Mod Manager](../../apps/mod-manager/README.md) app, and the two share the same
library and loadout files.

## Library and loadout

The library is the set of mods you have imported. The loadout is what is
enabled and in which order. Both default to a shared per-user location; every
subcommand accepts `--library <DIR>` and `--loadout <FILE>` to work on a
different set.

```powershell
gore mgr import C:\Downloads\SomeMod.zip   # folder, .zip, or a single game file
gore mgr list                             # library joined to loadout state
gore mgr remove <ID>                      # drop from library and loadout
```

`import` accepts built GORE bundles (a folder or zip with a root
`gore-mod.json`), foreign mod zips and folders, loose `_P.pak` files, IoStore
triplets (`.utoc`/`.ucas`/`.pak`), UE4SS Lua mod folders, and raw game-file
replacements. `list` prints the entry ids the other commands take.

## Order and enablement

```powershell
gore mgr enable  <ID>
gore mgr disable <ID>
gore mgr order   <ID> <POS>    # 0 mounts first, and loses conflicts
```

Position `0` mounts first; later entries win conflicts. `<POS>` is clamped to
the last slot.

## Conflicts

```powershell
gore mgr analyze
```

Reports conflicts among the **enabled** mods across localization, audio,
texture/asset, item overrides (CDO), scripts, and raw-file replacements, and
which mod wins each one.

Voice collisions on `(archive, archive_path)` are case-insensitive, soft, and
order-dependent: the later mod wins while retaining the winning spelling and
operation.

Script mods that do not declare their CDO targets are treated as opaque — the
manager cannot prove what they touch, so it cannot rule out a conflict with
them.

## Apply

```powershell
gore mgr apply  --game "$GAME"    # compose the enabled loadout into one deployment
gore mgr status --game "$GAME"    # is the install in sync with the target loadout?
gore mgr reset  --game "$GAME"    # undeploy everything the manager has active
```

`apply` is **declarative**: it recomputes the full modded state from a pristine
base and deploys the whole enabled set, backups first. It is not an incremental
patch on top of whatever happened to be installed, which is what makes
disabling a mod in the middle of the order safe.

`reset` restores the pristine install.

## Flag summary

| Flag | Commands | Meaning |
|---|---|---|
| `--library <DIR>` | all except `reset` | Library dir. Default: the shared per-user library. |
| `--loadout <FILE>` | all except `reset` | Loadout file. Default: the shared per-user loadout. |
| `--game <PATH>` | `apply`, `status`, `reset` | Game root containing `G1R\`. Falls back to the configured path. |

## Related

- [Bundling & deploying](bundles.md) — producing the bundles this manages.
- [Mod Manager app](../../apps/mod-manager/README.md) — the same operations with
  drag-to-reorder and a conflict view.
