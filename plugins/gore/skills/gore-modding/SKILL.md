---
name: gore-modding
description: Use when modding Gothic 1 Remake with GORE - changing textures, localized text, dialog, audio, voice-over, item values or AngelScript; building a bundle; or importing, enabling, ordering, analyzing, preflighting, applying, checking status, recovering, removing, and resetting GORE or external mods with the Mod Manager. Covers the consent gate and what "deployed" does and does not prove.
---

# Modding Gothic 1 Remake with GORE

The `gore_*` MCP tools run the real `gore` CLI. This skill is the workflow around
them; the facts live in the guide, which ships with the binary and is always
current for the installed version. Read the guide rather than trusting anything
remembered.

## If the tools are not there

If no `gore_*` tool exists, the server did not start, and the overwhelmingly
likely reason is that `gore.exe` is not on `PATH` — this plugin starts it as
`gore mcp serve`, by name. Say so plainly rather than working around it: no
`gore` command can be reached, and `gore --version` in a terminal is the check.
Nothing below applies until that is fixed.

## Before you touch anything

Call `gore_guide` for the page that covers your domain: `textures`, `audio`,
`voice`, `text-and-dialogs`, `items`, `scripts`, `bundles`, `mod-manager`. Use
`action: "search"` first — it ranks single sections, so the follow-up read stays
small. `gore_help` gives exact current flags; the guide gives the order to do
things in and what breaks when a step is skipped.

## Manage a loadout as one declarative deployment

For installing or managing mods, use `gore_mgr`, not a sequence of direct
`gore_mod deploy` calls. A Manager loadout is one owned deployment; Apply
rebuilds its complete enabled state from the pristine base.

Read `gore_guide` page `mod-manager` and the exact `gore_help` entry first. The
guide is the authority for the current accepted GORE bundles, external folders,
archives, loose files, containers, UE4SS mods, and mixed packages. Do not infer
support for a format from its extension or from where the mod came from.

Use this lifecycle:

1. If the setup is unknown or already looks wrong, run `gore_doctor` once; do
   not repeat it between ordinary Manager steps. `gore_mgr import` the package
   and keep the returned entry id. Import writes
   protected Manager library/loadout state, not the game installation; it still
   needs consent because a verified re-import may replace the stored payload.
2. `gore_mgr enable` the intended entries, use `order` when needed, then run
   `analyze`. Position 0 goes first and loses recognized ordered conflicts. An
   intended winner is evidence from the analyzer, not proof of runtime priority.
   `enable`, `disable`, and `order` update the reversible target loadout
   immediately and intentionally do not open the protected-write consent gate;
   state the intended edit before the call and report the resulting order.
3. Immediately before an installation change, run `gore_mgr preflight` for the
   exact read-only readiness/recovery report. An active Studio deployment, game
   drift, or interrupted operation is a state to resolve, not something to
   overwrite.
4. Before `apply`, summarize the enabled loadout and observable effects, give
   the exact Apply and Reset commands, and obtain consent. Apply writes the game
   installation. Follow it with `status` and then a concrete in-game checklist.
5. `remove` deletes the library entry and its loadout slot, but does not change
   bytes already deployed in the game. Apply the remaining loadout afterwards
   to make the installation match, then check `status` again.
6. `reset` is the terminal cleanup path for a Manager-owned deployment and must
   refuse rather than remove a Studio deployment. Check `status` afterwards;
   never treat a successful command as proof that the game displayed or executed
   every component.

`list`, `analyze`, and `status` are not advertised as read-only because opening
the authoritative Manager store may reconcile its loadout; list and analyze may
also finish recovery of an interrupted library replacement. They remain ungated,
but their result must still be treated as authoritative refreshed state.

Never delete a GORE installation lock or recovery directory by hand. If an
operation is still active, wait. If `preflight` identifies an abandoned Manager
operation, pass its exact opaque action token as `expected_guard_id` to
`gore_mgr recover` and obtain consent for that call. Recovery atomically rechecks
the token; never guess or reuse one after the state changes. Compiler-owned,
ambiguous, or invalid recovery state gets help, not an improvised reset. Re-run
`preflight` and `status` after recovery before Apply or Reset.

## Pick a target that the engine actually reads

This is where mods silently fail, and the tools cannot warn you: they will
faithfully replace an asset nothing samples.

The game's content lives in two worlds. Most of it is cooked into the IoStore
containers, which is what `gore_texture` and `gore_asset` reach. A short,
enumerable set sits loose on disk under `G1R\Content` — the FMOD banks, the Bink
movies and their subtitles, the splash bitmap, and the mouse-cursor PNGs — and
none of those is reachable through the texture commands. Read
`gore_guide{action:"read",page:"textures"}` before choosing, and check which world your target
is in. A bundle's `files` section replaces a loose file; `texture` replaces a
cooked one. Using the wrong one deploys cleanly and changes nothing.

Three specific traps the guide documents, each found by shipping a mod that
changed nothing. The mouse cursor does not come from the cooked cursor texture.
The pre-rendered intro movie plays its own embedded audio, so replacing the
intro's voice lines is correct and inaudible — for a fast audible proof use a
real in-engine conversation line instead. And a sound sample is not a sound the
game triggers: the game plays events, several samples often share one trigger,
and near-identical names belong to different surfaces. Before you tell the user
where to listen, confirm the sample you replaced is one that surface plays and
whether it is one of a numbered set — the audio guide says how.

Then read your own work back. Every write path in this toolkit can be re-read:
a replaced sample lists as replaced, a bundle records a hash per file. Do that
before reporting success, and say which items you could not check.

## Never put a name in a spec that you have not seen in a listing

Every id in a bundle spec — a sample name, an archive path, a texture asset, a
localization id — has to come from a listing you actually ran, not from the
pattern the neighbouring names suggested. The naming looks regular enough to
extrapolate from and is not: one session's spec named a Diego line that appeared
in no listing, and it happened to exist. The failure mode when it does not is
`mod build` accepting the spec and `mod deploy` refusing it afterwards, which
costs you the whole build.

## The consent gate

A call asks first when it would change the game installation, or destroy
something outside it that this server can see is there — an output file that
already exists, an output directory that already holds files, a bundle folder
about to be cleared and rebuilt. Writing into a fresh or empty scratch directory
asks nobody. If a question does arrive, it is about something real; read what it
names rather than approving it reflexively.

Many clients answer that question themselves in milliseconds without showing
anybody anything, so the call comes back refused even though nobody declined.

When that happens: do not resend the call unchanged, and do not tell the user they
said no — the server cannot see who answered. Show them the command line the
refusal prints, ask in the conversation, and only if they agree, send the same
call again with `user_approved` set to their own words. Never fill that field in
without having asked; it is recorded in the result as your claim.

## Two things that will cost you a build

Asset paths in a bundle spec (`wav_path`, `ogg_path`, `image_path`,
`mini_cache`, `source_path`) resolve against the **spec file's directory**, not
the working directory. Put the assets beside the spec, or spell them absolutely.

Listings are bounded on purpose. `gore_voice list` and `gore_audio list` print a
page and say how much they left out. Narrow with `--filter`; raise `--max` only as
far as you need. Asking for everything at once rebuilds the oversized result the
bound exists to prevent — and a cut-off JSON array does not parse.

## Hand the build over before you ask to install it

The moment `mod build` succeeds, and **before** the deploy question is put to the
user in any form, tell them four things:

1. **What the mod changes**, domain by domain — for each one, what they will see
   or hear, and where. Not the spec; what it does.
2. **Where the bundle is**, as a full path.
3. **The installation route**, spelled out. For a Manager loadout this is
   `mgr import`, `enable`, and `apply`; use direct `mod deploy` only when the user
   explicitly chose the single-bundle route and no Manager deployment owns the
   installation.
4. **The matching cleanup route**: Manager `remove` plus Apply, or Manager Reset;
   direct `mod undeploy` only for a direct deployment.

Then ask.

This is the order that keeps the decision with them. Deploying writes into a game
installation they may have hours in, and "may I deploy?" is not a question anyone
can answer without already knowing what is in the bundle and how to get rid of
it. Sessions keep getting this backwards — asking first and explaining afterwards
— which turns the summary into a report on something already done.

It also stops the build from being trapped behind you. A person who has the path
and the two commands can install it next week, from a shell, without this
conversation.

## After deploying, be honest about what is proven

`gore mod deploy` and `gore mgr apply` verify that the bytes they wrote are the
bytes they meant to write, by hash. Nothing observes the screen. A successful
deploy or Apply is not evidence that the change is visible.

So follow it with a checklist the user can actually run: what to look at, in what
order, and what each item would look like if it worked. Mark the items you could
not verify offline as uncertain, and say why.

If something did not show up, the first question is not "did the tool fail" but
"does the engine read that asset". Check that before changing the pipeline.
