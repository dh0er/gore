# Mod Studio

GORE Mod Studio is the no-code GUI over the same engine as the CLI: wizards and
workbenches over a managed project instead of commands over files. It is a work
in progress — authoring, review, and offline builds are real today, while
deployment and every in-game claim stay deliberately out of its hands (see
[Current limits](#current-limits)).

Three rules hold everywhere. Everything you author lands in your managed
project, saves are never written, and normal use of a configured Gothic 1
Remake installation is read-only evidence. The separately invoked,
evidence-only compiler check authenticates the installed cache/API and uses the
selected product policy. Strict standalone returns native file, line, column,
severity, and message diagnostics without starting the game or entering an
installation-mutation window. A deliberately selected game-capable policy is
the bounded exception: it prepares the complete validated base source tree
with the derived managed module overlaid, stages that tree under an
installation-mutation guard, restores every touched path, and blocks later
compiler or deployment mutation if exact recovery is uncertain. Neither route
grants build, deployment, or runtime authority. Normal
surfaces never ask for or show entity IDs, LocIDs, hashes, or paths — advanced
disclosures exist where inspection needs them. And every dialog is bound to
the exact project state it opened on: if the project changes underneath it or
the session must be reopened, it locks or asks for a refresh instead of
applying stale input.

## Workspaces

Every open managed Revision-3 project has exactly five primary workspaces:

- **Home** for project status and common actions;
- **Content** for finding and opening project or base-game content;
- **Story** for the bounded NPC and Quest workbenches;
- **Text & Voice** for project localization, dialog-line, and Voice work; and
- **Test & Release** for the available checks and offline release actions.

These destinations organize existing capabilities; they do not add runtime,
deployment, or game-mutation authority.

## NPCs

The Guided NPC Draft wizard creates a new character derived from a vanilla
template: you supply a display name, pick a qualified vanilla archetype in the
searchable picker, and every technical identity is derived for you and stays
hidden. The wizard is reachable from Home, the project Create command, Story
creation, and the Base-game and search starters, in English and German.

The recommended path is **Character + first greeting** — the NPC wizard plus a
greeting form, saved as two separate steps, not one pretended transaction.
Step 1 creates the character. Step 2, opened only after Studio has re-verified
the new character and that its greeting list is still empty, creates one
localized dialog line (newly authored text or one eligible existing project
localization), optionally prepares an empty Voice slot for one language, and
inserts the line as the first greeting. Completion opens the NPC's **Dialog &
Voice** tab with the created line selected. Cancelling before step 1 saves
nothing; cancelling step 2 keeps the already-created character and leaves you
on its empty **Dialog & Voice** surface — there is no hidden rollback.

Selecting one of your NPC Drafts under **Content → This mod** opens the Story
Workbench — list and detail side by side on a wide window, a scrollable detail
sheet on a narrow one — with exactly four tabs: **Profile**, **Dialog & Voice**,
**References**, and **Problems & Checks**:

- **Profile** shows the display name and **Edit name & archetype**, which
  changes exactly two things: the name and the archetype the character derives
  from. Existing greetings survive both kinds of edit, and closing with
  unsaved changes asks for an explicit discard.
- **Dialog & Voice** is a plain-language greeting editor: authored order,
  friendly line and speaker labels, language and Voice coverage, text
  previews. Attach an existing project dialog line, reorder or detach bindings
  (detaching never deletes the shared line), create a line — with localization
  and an optional empty Voice slot — at the selected position, or jump to that
  line and language in the **Text & Voice** workspace.
- **References** lists outgoing entity and asset links plus same-project
  incoming links; its problem count means unresolved references only.
- **Problems & Checks** owns the inspections below.

The workbench reports **Draft only**, **Build blocked**, and **Runtime not
verified** as three separate states, never one merged "ready" claim.

**Remove Draft...** scans every reference first; a remaining local backlink
blocks removal and can take you to its source. The confirmation names the NPC
and its generated script module. The V1 dialog has no action-local rollback;
once published, however, the removal is a normal managed-project revision and
can be revisited through bounded project **History** or global **Undo** while
that authenticated revision remains retained. The game installation and saves
stay untouched.

**Open profile & compiler checks** (from **Profile** or **Problems & Checks**)
shows saved-source, persisted-parent, and exact-project checks plus the
remaining readiness blockers; an advanced disclosure reveals the generated
AngelScript, IDs, parent classes, runtime name, and seals. It works without a
configured game installation because it verifies persisted project evidence
only. With an installation configured, the same dialog offers a separate
evidence-only compiler check of the generated source against your exact
installed game.

## Spawn and placement

Putting a character into the world has a supported path since 2026-09-05, and
it is `gore npc` on the command line, not Studio. Two authored characters were
built, deployed and observed in game: they stand at their world points, animate,
can be focused and spoken to, and the save records them under their own
identity. [NPC authoring](npc-authoring.md) is that workflow; Studio's NPC Draft
is still the offline-only thing described below.

Moving a character that is already standing in the world remains a different
question, and the answer is still the
[save editor](../../apps/save-editor/README.md).

Calling a *vanilla* spawn definition a second time would still only produce
another body sharing the vanilla identity — which is why an authored character
brings its own definition chain
([the contract](../reference/studio-authoring.md#remaining-runtime-gates)).
`gore location resolve` confirms that a waypoint name exists, which is not the
same as being able to send anyone to it. The one thing that does move a
character is the [save editor](../../apps/save-editor/README.md), which edits a
position already recorded in an existing save; there is no hook that places
anybody in a new game.

## Quests

The **Create Quest** wizard captures the human-facing name, technical
identity, parent, giver, description, and one to eight objectives, and creates
the quest together with its generated script module in one step. A selected
Quest has exactly four Story tabs: **Journey**, **Dialog & Voice**,
**References**, and **Problems & Checks**. **Journey** hosts its three
editing areas:

- **Outline** edits the display name, title, objective order, and objective
  titles; objectives keep their stable slots across reorders.
- **Context** edits the description, parent quest, and giver through catalog
  selections — free-form runtime strings are not accepted.
- **Behavior** edits the transition plan: availability, start, success,
  failure, predicates, effects, and parent completion. Plans are validated
  before saving, and a save that changes nothing is rejected.
- **Dialog & Voice** hosts **Transcript** entries that reference exact dialog
  lines and target the quest
  root or an active objective. Reordering or replacing entries is one
  transaction; creating a line and inserting it is atomic.
- **References** shows the selected Quest's project relationships.
- **Problems & Checks** exposes **Source and checks**, which shows the generated
  module without publishing anything and runs the managed compiler checks for
  the selected quest.

## Voice

With a managed project open, **Text & Voice** is a direct production
workspace: it opens on the **Work list**, with a **Project texts** switch
beside it. Text and take work is project-only; importing audio and resolving
installed targets need a Gothic 1 Remake installation configured in Settings,
as does **Build Voice bundle** under **Test & Release**.

The **Work list** derives the next evidence-backed production decision and
shows two kinds of rows. **Language not added** means a project authoring
language is absent from one of your project texts — it never claims an
existing translation is blank, wrong, or low quality, because there is no
evidence for that judgment. **Voice production** means one existing Voice slot
for a dialog line and language; a line without a slot never invents a row, so
record that intent with **Plan recording** if you want it queued. For an
existing slot, one rule picks exactly one next step:

1. no candidate takes → **Add a recording**;
2. candidates but none Approved → **Review and approve a recording**;
3. an Approved take exists but none is validly selected → **Choose an approved
   recording**;
4. a take is selected but its target is unresolved or ambiguous → **Resolve
   the Voice target**;
5. selected and resolved → **Production decisions complete**.

Draft or Recorded alternatives stay visible as optional review backlog without
regressing a finished slot, and **Production decisions complete** deliberately
does not mean ready, buildable, deployed, or audible in game — its action
opens **Test & Release** so the real checks can be reviewed. **Add language**
rows open the matching project text with the locale prefilled; recording rows
open the right line and language in the take dialogs. The Work list itself
never chooses a take, approves audio, or resolves a target.

**Project texts** searches your project-owned localization entries and opens
the complete multilingual text map inline, with shared dialog-line backlinks
and speaker labels, so you see the scope of a change before saving. A fresh
project starts with the guided dialog-line flow: one new localization entry
and one dialog line — or the new line bound to an exact existing, unused
project localization, preserved byte-for-byte — plus an optional empty Voice
slot for one language. It is a small prerequisite flow, not a dialog editor.

**Add Voice take** imports one real local Ogg for an existing line and
language. The search-first wizard hides technical identities, retains
alternate takes, tracks Draft/Recorded/Reviewed/Approved status, and lets only
an Approved take become selected; the selection changes only when you
explicitly confirm a replacement. Before importing, you can open the
still-local file in your operating-system player.

**Import recording folder** reviews and atomically imports up to 256 direct
`<LocID>.ogg` children of one folder for one language; non-Ogg files are
ignored and subfolders are never traversed. The plan is strictly
all-or-nothing — every included file must be ready or already present before
import can start, and there is no partial success. Already-present files are
no-ops, and every new take arrives as Recorded, never selected automatically,
so approval and selection stay your decisions.

**Manage Voice takes** searches existing lines and owns the review loop:

- Move a take through Draft, Recorded, Reviewed, and Approved — a status is
  your workflow label, not proof the audio is right in game. A newly Approved
  take becomes selectable immediately; a selected take can only be set to
  Approved, so change or clear the selection first.
- Select one Approved candidate. Candidates are listed in authored order with
  the current choice marked and non-Approved ones disabled; nothing is picked
  implicitly, and clearing a selection warns that bundle builds stay blocked
  until another Approved take is selected.
- **Preview** plays a managed take in-app; **Check media** reports a take's
  duration and codec assurance on demand. Neither is an audio-quality or
  in-game playback test.
- Remove one take from the line and language. If it was selected, removal
  clears the selection atomically; a take shared by another slot remains
  there. Status and selection are separate saved changes, no operation
  rewrites or physically deletes media, and none of this needs a game path.

Your visible line and language selection carries into the take, manage, and
resolve dialogs, so you never repeat the same global search.

**Resolve Voice target** inspects the exact installed locale archive for one
existing Voice slot and records zero, one, or several matching members as
unresolved, resolved, or ambiguous — an ambiguous match is never chosen
implicitly. A full slot stays eligible for resolution even when its candidate
capacity stops another take from being added.

**Build Voice bundle** evaluates every current Voice slot and either shows all
structured blockers without creating output, or writes one sealed voice-only
bundle into a brand-new folder you select. This is an offline build; the
dialog has no deployment action.

## Project history and undo

The command bar's **Undo** action restores the immediate previous retained
project revision through the same authenticated mechanism as **History**.
History is bounded: recording starts at an explicit revision, older entries
can expire, and Studio never invents missing checkpoints from storage.
Restoring an earlier entry publishes its content as a new revision, so later
versions are not erased. An unsaved text edit, a busy action, project drift, or
a session that requires reopen blocks the operation rather than applying a
stale checkpoint. History and Undo change only the managed project, never the
game installation or saves.

## Project backup and restore

**Create project backup** in the Project menu writes one exact restorable
snapshot of your project to a new `.goremod` file; only **Snapshot V2** is
emitted. The dialog asks for a new filename plus an existing destination
folder, and is explicit that the result is a restorable Mod Studio project
backup — not a playable mod, build, or deployment. Game and save files are
untouched. Backup needs no game installation and ignores build blockers; it is
unavailable only while the session requires recovery or a visible text draft
is unsaved. If the chosen name already exists, Studio keeps the dialog open
and asks for another.

**Restore project backup** is available from the Project menu in every project
state and directly on the empty landing surface; restore is currently
Windows-only. It verifies the archive first (a read-only check you can cancel
safely), asks for an existing parent folder plus a new project-folder name,
materializes exactly once, and opens only a fully confirmed result — anything
without the exact Snapshot V2 format is rejected as an invalid project backup.
Unsaved editor drafts are confirmed before you pick a source, and Studio
blocks closing the dialog or double-submitting while a restore runs. Restore
preserves project identity, revision, and authenticated history: it is
restore/relocate, not Clone or Save As.

In the rare terminal where Studio cannot prove whether an output was
completely written, it names the attempted destination, opens nothing, and
asks you to inspect it yourself rather than retrying automatically.

## Current limits

Mod Studio names what it has not done. **Build blocked** and **Runtime not
verified** are normal states, and no workflow above claims in-game proof.

NPC and quest Drafts:

- An NPC Draft is a logical clone of a vanilla archetype. Visuals, faction,
  stats, inventory, routine, dialog topics, quest links, world placement, and
  spawning are not authored yet, and a passing compiler check still leaves the
  production, residence, and spawn blockers standing.
- Greeting lines are authoring metadata only — no dialog topic, player choice,
  condition, effect, or playable conversation is created.
- Draft removal has no action-local rollback. Bounded project-wide History and
  Undo can restore a retained exact revision as a new revision, and Snapshot V2
  can restore a complete backed-up project; general project deletion remains
  unavailable.

The Voice workflow still does not provide:

- managed deployment, undeployment, load-order integration, or an isolated
  playable test profile for the sealed bundle;
- audible in-game qualification for the selected line, persistence, save/load,
  or clean runtime removal;
- explicit choice among ambiguous installed archive matches;
- recording, trimming, normalization, transcoding, loudness comparison, actor
  notes, or lineage — take previews and **Check media** are integrated, but
  media QA reports only duration and codec-specific validation assurance, and
  none of it is in-game proof;
- recursive, partial, or multi-locale folder import, coverage dashboards,
  CSV/XLIFF, or broader batch and team review queues — the Work list covers
  only absent authoring languages and next decisions for existing Voice slots,
  and **Plan recording** creates one intent at a time;
- qualified Opus output — bundles are qualified for Vorbis only, and selecting
  an Opus take raises an explicit build blocker;
- new archive members, or adopting vanilla dialog and localization identities;
- topic registration, AngelScript generation, conditions and effects, or a
  playable dialog path for a newly authored line;
- localization delete/clone, general line relinking, speaker and NPC
  relationships, bulk language production, provenance/rebase workflows, or a
  complete conversation graph editor — empty line/language Voice setups can be
  created and removed safely, but that narrow pair is not a general
  relationship editor.

None of this completes the Voice production milestone, vanilla adoption, or
any runtime dialog workflow.

Backup and restore: restore is Windows-only, Clone/Save As does not exist yet,
and recovery tooling for an uncertain restore is future work.

## Related

- [Bundling & deploying](bundles.md)
- [Voice-over](voice.md)
- [Dialog authoring](dialog-authoring.md)
- The contracts behind these surfaces:
  [NPC and quest authoring internals](../reference/studio-authoring.md),
  [voice authoring internals](../reference/studio-voice.md), and
  [project snapshot internals](../reference/studio-project-archive.md)
