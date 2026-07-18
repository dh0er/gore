# Quest authoring

Quest authoring is a greenfield Revision-3 workflow. The only persisted
generator contract is version 4.

## Current model

A Quest consists of one `quest_draft` entity and one generated
`script_module` entity. Both entities use:

- generator ID `gore-authoring.draft-quest-skeleton`
- generator version `4`
- an exact owner/reference pair
- revisions that advance together for Quest edits

The Quest input contains the runtime identity, resolved parent and giver,
localized text literals, objective titles, collision-catalog evidence, and a
required `transition_plan`.

The transition plan owns stable positive objective slots. Slot `1` is always
present, active slots are strictly ascending, the visible objective order is a
complete permutation of those slots, and `next_slot_ordinal` never reuses a
slot. New Quests receive the canonical default plan for their objective count.

## Authoring surfaces

### Create Quest

The Quest wizard captures the human-facing name, technical identity, parent,
giver, description, and one to eight objectives. Native authoring inserts the
Quest/module pair directly at generator version 4 with its required transition
plan. The generated AngelScript and its hashes are part of the same prepared
candidate.

### Journey and outline

Journey is the primary Quest workspace. Outline editing changes the display
name, title, objective order, and objective titles while preserving stable
slots. The request is bound to the exact project head, Quest/module revisions,
and transition-plan seal.

### Context

Context editing changes the description, parent Quest, and giver through
sealed catalog selections. Runtime strings are not accepted as free-form
authority. The candidate must preserve every unrelated Quest field.

### Behavior

Behavior editing exposes the transition plan: availability, start, success,
failure, predicates, effects, and parent completion. Plans are validated before
native preparation. An exact no-op is rejected.

### Transcript

Transcript entries reference exact DialogLine entities and may target the Quest
root or an active stable objective slot. Reordering or replacing transcript
entries is an exact-head transaction; creating a line and inserting it into the
transcript is atomic.

### Source and checks

Source inspection reads the exact generated module without publishing. Managed
compiler checks bind the selected Quest and module to the current project head.
Generated source, source SHA-256, and the domain-separated input fingerprint
must agree before any candidate is accepted.

## Publication contract

All edits use the managed project transaction lane:

1. Read an exact-current private seed where the editor needs one.
2. Build a bounded request carrying the expected head and entity revisions.
3. Prepare one unpublished native candidate.
4. Verify that only the permitted Quest/module delta occurred.
5. Publish with compare-and-swap and fully reopen the resulting checkpoint.

Head conflicts never overwrite a winner. Integrity uncertainty marks the
session as requiring reopen. Correctable semantic conflicts remain retryable.

## Runtime boundary

Mod Studio can author, persist, inspect, and prepare generator-version-4 Quest
content. Runtime qualification and game installation remain explicit status
claims; the editor never presents an offline draft as proven playable. The
managed compiler and runtime validation work must succeed before publication to
the game can be claimed.

## Invariants for new work

- Add functionality to the single Revision-3/version-4 path.
- Keep the transition plan required in every persisted Quest.
- Preserve stable objective slots across outline, behavior, and transcript
  edits.
- Keep generated module source and seals derived from the exact Quest input.
- Do not add alternate readers or duplicate editor APIs.
