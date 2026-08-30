# Changelog

All notable changes to gore-cli are documented here. The release workflow
publishes the section matching the released version as the GitHub release
notes, so every release needs an entry.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added

- `dialog` — read the game's dialog trees offline: `list` the conversations,
  print one NPC's whole `tree` with its options, conditions, lines, effects and
  sub-menus, `show` a single topic, or `export` everything as JSON. Text comes
  from the shared localization catalog; `--lang` picks the language.
- `dialog text` — one conversation's lines as a `gore loc import` edits
  document, each under the localization column the game actually reads.
- `dialog new-topic` — scaffold a new native root or direct sub-topic with the
  conversation-private base and an unused class identity resolved from the
  cache. The class is integrated into the existing conversation module. A root
  workspace and its staged spec are script-only and contain no automatic
  UE4SS/`dialog_topics` adapter; `--subdialog-of` instead wires one empty slot
  in a shipped parent's single `Subdialog` call. Its default insertion keeps a
  trailing `TEXT_BACK`/Zurück option last, while `--subdialog-position <N>`
  selects any valid 1-based position and shifts existing children without
  dropping one. Generated sub-topics use the shipped rank `0`. Both compile as
  `--op edit --allow-new-symbols`, not as an isolated cross-module `--op add`.
- `dialog new-conversation` — scaffold settings, a private root and the first
  direct choice for an NPC with no root topics. Exactly one matching shipped
  topicless module is preserved and edited; when no conversation module exists,
  one complete module is added. Every further topic and new-to-new `Subdialog`
  level stays in that same source module. New topics remain direct subclasses
  of the private root; a new parent may own only new same-conversation children.
  Both operations and an all-new
  multi-level tree are qualified through strict standalone compilation and
  bundle inspection; native runtime association, discovery and navigation
  remain unproven.
- `dialog checkout` / `check` / `stage` — check out a shipped conversation, or
  validate and stage that edit or a generated complete-conversation workspace.
  Shipped source includes reconstructed defaults for `Caption`, `PriorityRank`,
  `Rules` and flags. Checks reject partial default coverage, removed shipped
  targets, unsupported generated-method loss and unsafe shipped-ABI drift before
  compilation; byte-exact default carry remains only for source without
  authored defaults.
- Checked edits can retain intentional new-symbol rows and append a new topic
  class inside the owning namespace of the same existing conversation module.
  Qualified class identity, namespace residence, native root/direct-sub-topic
  placement and the complete-default contract are checked together. Legacy
  workspaces may still carry the separate low-level `dialog_topics` adapter row;
  its participant, class and sentinel remain fail-closed, but the current
  high-level scaffold no longer emits one.
- Qualify the native same-module dialog path live on BuildID `24878692` against
  pristine Shipping cache SHA-256
  `7A18F954E32AF30FC24AE3A66EA35D3B5CB98560C8F5083C7846FC9CE1D77511`.
  A source-identical complete Diego recompile ran normally; a `Caption` edit was
  visible and selectable; a newly compiled root appeared and was selectable in
  two runs, including one with no UE4SS proxy; and a new direct sub-topic
  appeared, dispatched its new `Act`, ended the conversation and returned
  control. In the first root run the installed legacy adapter logged
  `sentinel-topic-missing` and skipped, proving that it did not insert the root.
- Exercise Stage A's exact 20-sibling submenu in game on the same build. The
  submenu rendered and multiple slots were selectable; the renamed shipped
  option retained and ran its long method body before returning to the menu.
- Exercise a new topic field and helper method in game. The authored
  `default ProbeMarker = 24878692` field and helper were used successfully.
- Exercise Stage B ordering in game. `PriorityRank = -100` appeared first,
  `+100` appeared last, and rank-zero entries preserved authored order.
- Prove placement on rebased current-head bundles. The default new sub-topic
  appeared immediately before Zurück and was selectable; explicit
  `--subdialog-position 1` appeared first while Zurück stayed last and was also
  selectable.
- Recheck the saturated Stage C rebuild. Its mini-cache (SHA-256 prefix
  `C675BB55…`) is byte-identical to the already live-tested artifact, which
  failed opening the reshaped 20-entry submenu with an array-capacity error.
  Treat manual full-menu reshaping as technically unsupported/unsafe. Dialog
  checks now reject every structural change to a pristine call that is already
  full, while leaving its defaults, methods and `PriorityRank` editable.
- Exercise authored gameplay state and persistence from new Diego options on
  the same build. One option changed ore inventory from 0 to 1 and the item
  remained after quicksave/restart. Another authored
  `Rules.HideIfKnowsId` for `gore_diego_quest_knowledge_24878692`, remembered
  that exact knowledge id, started the Stonehenge quest, disappeared from the
  menu, and remained absent after restart; save inspection found the id on the
  hero and the quest remained `Running` in the journal.
- Resolve a newly authored localization and voice identity end to end on the
  same build. The new subtitle appeared exactly, the new Ogg member referenced
  by the compiled `Say` call played, and a system-loopback capture correlated
  `0.763` with the authored source recording. This is bounded evidence for that
  Diego fixture, not a general qualification of every voice path or game API.
- Keep compile, packaging, deployment and runtime evidence separate. Independent
  add/edit mini-caches still cannot depend on one another; FullGraph V2 resolves
  coordinated cross-module references offline. Mod Manager deployed the raw
  complete cache byte-for-byte, but its 10,782 semantic deviations (81 in
  Diego) produced a main menu whose entries could not be activated. A selective
  hybrid whole-cache product preserved all untouched modules, booted and loaded
  a save; that proves complete-cache replacement, not the raw regeneration or
  the cross-module dialog edge. Normal cross-module dialog minis remain
  technically unsupported.
- An ambient fixture entered `State.AmbientConversation` with
  `GA_Human_Conversation_Ambient` active without player selection, proving the
  automatic activation boundary. It crashed only after forcing an artificial
  20-choice `Subdialog`; that menu shape remains unqualified and is not evidence
  that broad ambient flags are broken.
- `gore_dialog` MCP tool for the same ten subcommands.

### Fixed

- Reject AngelScript preprocessor directives before dialog default coverage is
  accepted, both in `dialog check` and direct generated-default edit
  preparation. Disabled `#if`/`#else` branches can no longer make partial
  defaults look complete; directive spellings inside comments and strings are
  ignored as ordinary text.

- Preserve the shipped `FunctionTraits` and complete Unreal-function tail for
  every identity-matched existing declaration in an edited module, while
  retaining regenerated bytecode and its matching frame metadata. Genuinely new
  functions keep compiler-authored metadata. This prevents a complete dialog-
  module recompile from turning `Act`/`IsVisible` overrides into ordinary
  callables and leaving the game in conversation input lock without choice UI.
- Generate new dialog topics with real `BlueprintOverride` source methods
  (`IsVisible() const` and `Act()`) so the compiler records the corresponding
  `*_Implementation` functions as Unreal overrides even though no shipped
  function metadata exists to overlay. Scaffolds also author a deterministic,
  nonzero `DebugId`; `dialog check` rejects old `_Implementation` source
  spelling, duplicate hook overloads, missing `const`, and missing, zero or
  non-`int64` IDs. The generator avoids existing module IDs without forbidding
  intentional ID reuse in manually authored continuation chains. Check also
  binds `bIsSubTopic = true` to actual `Subdialog` wiring and rejects that flag
  on either a native or legacy-registered root.
- Rehydrate cached modules for standalone mixed-source compilation with their
  `AutomaticImports` relationships intact after source reset, reconstruct const
  qualification from both cached const encodings, and publish cached script
  enums to the compiler-wide type registry.
- Publish cached `__StaticType` globals through engine-wide automatic imports,
  preserve the `SCRIPT_OBJECT`, `TEMPLATE` and `APPOBJECT` kind of rehydrated
  script-class type IDs, and expose cached mixin globals only during source
  binding before restoring their original traits.
- Compile the real Payfine and Brannok emitter shapes where `Say` receives a
  prepared `LocText` temporary. The Brannok product oracle now also covers
  `Subdialog`, cached cross-module class values and mixins.
- Raise the bounded new-symbol portable-identity and namespace-comparison
  ceilings to 512 MiB while retaining the four-times composed-input and
  identity-footprint limits.

## [0.2.3] - 2026-09-01

- `gore texture story-images` lists the loose story images — the glossary
  artwork, tutorial pictures and writings — that live outside the asset
  container.
- Before you change a script, `as emit` and `as compile-module` say what else
  that module carries. Editing one function recompiles the whole file, so
  anything else in it the decompiler cannot reproduce exactly is rewritten too.
  Most modules stay silent because there is nothing to report — 6,982 of 7,317 —
  and the fifteen that contain a loop coming back broken say so by name.
- A class's default values are counted separately in that warning: they are only
  rebuilt if your edited file spells them out, and are otherwise copied over
  unchanged.
- Three script differences turned out to be a different program rather than
  different wording: one file called a function the game never calls, a value
  written through a returned reference never arrived, and a "largest so far"
  comparison picked the last positive value instead of the largest.
- Decompiled scripts recompile closer to the original: enum returns, increments,
  copied values, loop bodies and construction order now come back the way the
  game wrote them.
- A standalone AngelScript compile that dies without answering now says how the
  compiler process ended, instead of blaming its output.
- Correct stale command flags, install paths and counts throughout the guide and
  the reference docs.

## [0.2.2] - 2026-08-29

- Fix standalone AngelScript compilation for patch 1.0.5.

## [0.2.1] - 2026-08-28

- Add support for Gothic 1 Remake patch 1.0.5.

## [0.2.0] - 2026-08-27

- Bundle the qualified standalone AngelScript compiler with gore-cli. Script
  builds and `as compile-module` can now compile without starting the game.
- Compile the complete script project in one standalone run, including added,
  edited, and deleted modules, cross-module references, and emitted sources
  whose global function names collide across modules.
- Stage only authored script changes and reuse already authenticated compiler
  inputs instead of exporting and copying the full 7,000-plus-module base tree
  for every compile. In the one-file comparison used during qualification, the
  standalone path completed faster than the game's embedded compiler.
- Use `standalone-then-game` by default: GORE shows why the standalone attempt
  was rejected before using the game's embedded compiler as a fallback.
  Strict `standalone` and explicit `game` modes remain available.
- Match compatible game installations by Shipping cache format and the
  complete AngelScript Binds API instead of requiring an exact whole-EXE hash,
  so compatible Steam, GOG, and differently packed binaries are supported.
- Add dedicated MCP tools for strict standalone script compilation. They cannot
  select or fall back to the game compiler and therefore never ask for game
  launch or installation-write approval.
- Make `gore doctor` authenticate the bundled compiler and check the installed
  Shipping/Binds inputs through the same compatibility resolver used by real
  standalone compilation.
- Preserve structured standalone diagnostics with source file, line, column,
  severity, code, and message, and keep temporary compiler work isolated and
  cleaned up after success or failure.
- Reject ambiguous global calls and function handles before compilation while
  preserving calls whose overload is proven safe, including qualified calls
  in return expressions and arguments containing generic types.
- Add `gore mod inspect` and its read-only MCP alias for bounded offline
  validation of bundle directories and ZIPs, including declared payloads,
  UE4SS `Scripts/main.lua`, component formats, and deterministic hashes.
- Add `gore voice validate` for read-only Ogg/Vorbis and Opus validation with
  exact duration metadata, and reject invalid end-of-stream timing before a
  voice bundle is built.
- Derive newly added AngelScript module identities from their relative `.as`
  paths, matching both the standalone compiler and the game's source discovery.

- `as emit` / `as emit-all` now write class `default` statements, so item, NPC
  and config classes decompile with their values instead of as empty shells.
- A module whose defaults cannot all be recovered says so in its header and
  keeps them byte-exact on recompile.
- A module whose source declares defaults can be edited and spliced back;
  `as emit --no-defaults` still produces the previous shape.
- Scalar member stores into fields declared on a native base are recovered
  instead of dropped.
- Editing a module and splicing it back now keeps its `n"..."` names: they were
  remapped to unrelated entries before, so an item could come back wearing
  another item's model.
- Decompiled source keeps namespaces, `const` methods and parameter defaults, so
  quest, document and conversation modules can be edited and spliced at all.
- Every module in the game writes its class defaults now, down to the main map's
  worldpoint and item-spawn tables.
- A fluent chain keeps its links: a temporary's destructor between two of them
  no longer ends the statement, so AI rule tables decompile as the one call
  chain they were written as.
- A `Cast<>` comes back as a cast instead of the compiler's null-guarded
  if/else, and a bool field written from an int gets the bool form.
- A method's `const` return type is written again, so an edited module keeps
  that part of its identity; locals that receive such a value are declared
  const to match.
- Both halves of an accessor pair (`T f()` and `const T f() const`) are written,
  where the const half used to be dropped — every function in the cache is
  regenerated now.
- Decompiled bodies read the way they were written: an argument expression sits
  inside its call, a constant sits where it is used, `!(!(x))` is `x`, and a
  local the original never initialized is not initialized.
- A constructor that only gives members their values decompiles as member
  initializers, and a member store that used to be dropped from constructors is
  recovered — an `FName` global reads its real value instead of its own name.
- A `default` statement whose call lost an argument is refused instead of
  written, so a module keeps its byte-exact defaults rather than quietly
  changing meaning.
- Decompiled bodies write the range-for the compiler desugared, fold the
  temporaries it invented, drop stores nothing reads, and call `Super::` where
  an override calls the method it overrides.
- A branch that returns comes back as a return: a bool function's guarded return
  kept its condition, and every branch that leaves through the shared exit
  returns its own value instead of the one another branch left behind.
- A call's receiver and its arguments sit inside the call, including a call that
  stands in an `if`, a `while` or a `return` — they used to be evaluated earlier
  than the source evaluated them.
- A slot that holds what a call returned is declared with that type, so a bool
  result reads as a bool instead of `(x != 0)`.
- Decompiled bodies stop naming the temporaries the compiler made: a call whose
  result nothing reads is the statement it was, a by-value struct return returns
  its value instead of assigning a hidden slot first, and a `!` is applied where
  the value already sits.
- A comparison the source read as a value comes back: those functions returned
  the declaration's default instead of what they compared.
- A call whose result the source threw away is written as the statement it was,
  wherever the language allows the result to be dropped.
- `as extract-remap` and `compile-module` resolve a repeated string literal
  instead of refusing the module.
- `GORE_AS_REMAP_DIAG=1` prints the two identities behind an unresolved or
  ambiguous reference.
- A function returns its expression instead of naming it first, a condition
  tests its expression, and a handle copied from another handle is gone.
- A class says a member's initial value once instead of repeating it in every
  constructor that also takes a parameter.
- A chain of `&&` comes back as the chain it was rather than one `if`/`else`
  per link.
- A value passed to a constructor sits inside the call, where it used to be
  left behind in a local.
- A condition or a call argument holds the expression it tests, instead of a
  local the source never named — including a whole `&&` chain.
- A comparison handed to a call sits inside the call.
- A bool the cache proved from a comparison, a `!` or a call's return is
  recognized as one everywhere, and so is a bool field of the class.
- A guard that returns is written as one: an early `return` no longer goes
  missing, so the code behind it stays guarded.
- A cast reads the call it casts, a value the source built at a call site is
  written there, and an enum field is passed as itself.

## [0.1.0] - 2026-08-18

First release. Command-line toolkit for modding Gothic 1 Remake.

- `mod` — build and transactionally deploy one bundle containing item
  overrides, localization, audio, voice, textures/assets, loose or packed
  files, AngelScript, and dialog topics.
- `mgr` — keep a verified library and ordered loadout; import supported GORE and
  foreign mods; enable, disable, reorder, remove, analyze, Apply, inspect status,
  recover an abandoned Manager operation, and Reset through the shared Manager
  engine. Import accepts zip/folder packages, loose `_P.pak`, IoStore pairs with
  an optional same-stem `.pak`, UE4SS Lua folders, and known raw game files;
  unsupported or malformed inputs fail without partial publication.
- `mgr preflight` exposes the bounded read-only setup/recovery report;
  `mgr recover` requires the exact current abandoned-operation token and either
  interactive confirmation or `--yes`; `status --json` returns the full native
  report. Reset is Manager-only and refuses a Studio deployment, Remove points
  to the required next Apply, and zero-row Analyze output keeps coverage gaps
  visible as "no recognized conflicts" rather than claiming conflict freedom.
- `loc`, `audio`, `voice`, `texture`, `asset`, `as` — edit localized text, FMOD
  banks, voice-over archives, IoStore textures, cooked DataAssets and the
  AngelScript cache.
- `location-catalog` and `location` — build the named-location catalog, and look
  a waypoint or spot name up in it offline.
- `mcp serve` — all 87 commands over the Model Context Protocol, with protected
  installation, Manager-library, deletion, and in-place-rewrite operations
  confirmed first.
- `guide` — the manual, built into the binary and rendered to one HTML file.
- Commands whose evidence is audited against a specific game build say which
  build you have and which ones were audited, instead of quietly doing less.
- `as qualify` derives everything a new game build needs to be audited, and
  refuses rather than guessing when the evidence does not hold together.
- Prepared AngelScript mini-caches now resolve their absolute private
  StaticNames operands during loadout composition while raw minis retain their
  local-index convention; missing prepared rows fail closed.
- A 2026-08-18 real-install Manager campaign ran Nexus mods #244, #512, #269,
  and Attack Input V4, verified both numeric #244/#512 order directions, loaded
  a new game and an existing save, exercised enable/disable/reorder/Reset, and
  restored the captured loadout byte-for-byte. Its live Viper AngelScript proof
  was a GORE-authored fixture using the PR #91 Core DLL, not a third-party or
  three-way script qualification. Clean-Windows package acceptance remains open.
