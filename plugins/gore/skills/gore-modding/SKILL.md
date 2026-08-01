---
name: gore-modding
description: Use when modding Gothic 1 Remake with the GORE tools - changing textures, localized text, dialog, audio, voice-over, item values or AngelScript, or building and deploying a mod bundle. Covers what the tools can and cannot reach, the consent gate, and what "deployed" does and does not prove.
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

## Pick a target that the engine actually reads

This is where mods silently fail, and the tools cannot warn you: they will
faithfully replace an asset nothing samples.

The game's content lives in two worlds. Most of it is cooked into the IoStore
containers, which is what `gore_texture` and `gore_asset` reach. A short,
enumerable set sits loose on disk under `G1R\Content` — the FMOD banks, the Bink
movies and their subtitles, the splash bitmap, and the mouse-cursor PNGs — and
none of those is reachable through the texture commands. Read
`gore_guide{page:"textures"}` before choosing, and check which world your target
is in. A bundle's `files` section replaces a loose file; `texture` replaces a
cooked one. Using the wrong one deploys cleanly and changes nothing.

Two specific traps the guide documents, both found by shipping a mod that changed
neither: the mouse cursor does not come from the cooked cursor texture, and the
pre-rendered intro movie plays its own embedded audio, so replacing the intro's
voice lines is correct and inaudible. If you want a fast audible proof, use a real
in-engine conversation line instead.

## The consent gate

Any call that changes the installation asks first, over MCP elicitation. Many
clients answer that question themselves in milliseconds without showing anybody
anything, so the call comes back refused even though nobody declined.

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

## After deploying, be honest about what is proven

`gore mod deploy` verifies that the bytes it wrote are the bytes it meant to
write, by hash. Nothing observes the screen. A successful deploy is not evidence
that the change is visible.

So end with a checklist the user can actually run: what to look at, in what order,
and what each item would look like if it worked. Mark the items you could not
verify offline as uncertain, and say why. Then tell them the one command that
undoes everything — `gore mod undeploy` — and offer to run it.

If something did not show up, the first question is not "did the tool fail" but
"does the engine read that asset". Check that before changing the pipeline.
