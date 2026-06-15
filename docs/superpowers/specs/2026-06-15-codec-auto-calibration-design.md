# Codec Auto-Calibration & User-Facing Codec Errors — Design

Date: 2026-06-15
Branch: `feat/codec-auto-calibration`

## Problem

The G1R binary codec host trusts a game executable only when its whole-file
SHA-256 matches a hardcoded known profile. When the game ships a patch (e.g.
1.0.0 → 1.0.1), the hash no longer matches, the host falls back to pattern
resolution, and a pattern-resolved build reports `supported=false` /
`canCompress=false` / `canDecompress=false` by design. The app gates
`available` on `supported`, so the codec is disabled until a maintainer
reverse-engineers the new build and bakes a known profile into a release.

Two gaps:

1. **No automatic adaptation.** The host already has the machinery to safely
   adopt a new build at runtime (pattern resolver + runtime selftest +
   SHA-256-keyed derived profile cache), but nothing ever triggers the
   selftest, so the cache is never populated and new builds stay disabled.
2. **Techy error messages.** When a build genuinely can't be used, the UI
   surfaces strings like "G1R codec host is configured but not available" and
   raw decompress/compress flags — meaningless to an end user.

## Goals

- New/unknown game builds are adopted **fully automatically and silently**
  when the codec can be proven to work on the user's machine, with no user
  action and persistence across sessions.
- When a build truly cannot be used, the user sees a **plain-language**
  message + an actionable hint, with technical details tucked behind an
  expandable section.

## Non-Goals

- No maintainer-side CI automation for baking known profiles (possible future
  Tier 2; out of scope here).
- No report button / GitHub-issue integration in the error UI.
- No change to the pure-Rust Kraken backend.

## Decisions (locked)

- Calibration trigger: **fully automatic, silent** on the existing codec check.
- Error UX: **plain text + grund** + **was-tun hint** + **collapsible details**.
  No report button.
- User-facing strings: **English** (matches existing app UI language).
- Embedded calibration sample size: **4 KiB** synthetic plaintext.

## Architecture

Three layers, calibration logic owned by the host (it owns codec truth and the
embedded sample). The core triggers calibration; the app only renders.

```
Flutter app  ── check_codec ──▶  goresave_core
                                   probe binary host
                                   if resolutionMode == pattern_profile:
                                       calibrate (host)  ──▶  goresave_g1r_codec_host
                                       re-probe                  runtime selftest w/ embedded sample
                                                                 write derived profile cache (sha256)
                                   build status + plain message
app renders ◀── status (userTitle/userMessage/userHint + details) ──
```

### 1. Host (`goresave_g1r_codec_host`)

**Embedded calibration sample.** A const carrying a self-contained
known-answer decompress oracle:

- `plaintextBase64` / or generated deterministically in-code — 4 KiB of
  synthetic bytes we author (legal; not game data).
- `compressedBase64` — that plaintext Oodle-compressed (captured once using the
  working host against a current build; Oodle Kraken decode format is
  version-stable, so it decompresses correctly on future builds too).
- `expectedSize` (4096), `expectedDecompressedSha1`, `expectedDecompressedHeadHex`.

The compressed bytes are generated during implementation with the live host
(`compress` command on the synthetic buffer) and pasted into the const.

**New `calibrate` command.** Request: `exePath`, `derivedProfileCachePath`.
Behavior:

1. Probe the exe.
2. If already `known_profile` or `derived_profile_cache` (supported): no-op,
   return the probe result unchanged.
3. If `pattern_profile`: run the existing runtime selftest with
   - `runtimeSelftestSample` = the embedded decompress oracle (known-answer),
   - `runtimeSelftestCompressSample` = synthetic deterministic buffer,
   - `runRuntimeSelftests` / decompress / compress all true,
   - `derivedProfileCachePath` set.
   On pass, the existing `record_derived_profile_cache_after_self_test` writes
   the SHA-256-keyed cache entry. Return a final probe (now
   `derived_profile_cache`, supported).
4. If pattern resolution fails or selftest fails: return a structured
   `unsupported_exe` error (existing error code) with details
   (sha256, peTimestamp, reason).

This reuses the proven self_test path; `calibrate` is a thin orchestration
wrapper so the app never has to assemble selftest samples.

### 2. Core (`goresave_core`)

**`check_codec` auto-calibration.** After probing the binary host, if the probe
returned `resolutionMode == "pattern_profile"` (resolvable but untrusted) and a
`derivedProfileCachePath` is configured, invoke the host `calibrate` command,
then re-probe. The final probe drives the status. All within the single
`check_codec` core call — the Dart side is unchanged.

**Structured user-facing fields.** `check_codec` result gains:

- `userTitle` — short headline.
- `userMessage` — one plain sentence, no jargon.
- `userHint` — concrete next step (may be empty when none applies).
- `userSeverity` — `ok` | `info` | `error` (drives icon/color).

Existing `details`/`status`/flags stay for the collapsible section. The techy
`binary_host_message` strings are replaced by this mapping. Categories:

| Category | severity | title | message | hint |
|---|---|---|---|---|
| ready (known / cache / calibrated) | ok | "Game codec ready" | "The editor can read and write this game version." | — |
| unsupported build (pattern fail / calibration fail) | error | "This game version can't be opened yet" | "Looks like a new game update the editor doesn't recognize yet." | "Check for an editor update — a new version usually follows shortly." |
| exe not found | error | "Gothic 1 Remake not found" | "The game executable wasn't found at the saved path." | "Set the game path in settings." |
| host not configured | info | (existing config copy) | | |

### 3. App (Flutter)

- Codec status panel renders `userTitle` (with severity icon) + `userMessage` +
  `userHint` as the primary content.
- Technical block (profile, resolutionMode, decompress/compress flags, version,
  sha) moves behind an expandable `Details` disclosure.
- No change to `checkCodec()` flow; the core call now also calibrates, so the
  existing fire-and-forget check yields a ready status after a short spinner.

## Data Flow

1. App boots / save dir chosen → `checkCodec()` → core `check_codec`.
2. Core probes host. Known/cache build → ready immediately.
3. Unknown build → core calls host `calibrate` → host runs selftest with
   embedded sample → on pass writes derived cache → core re-probes → ready.
4. On fail → core returns plain-language `unsupported build` status.
5. Cache persists keyed by exe SHA-256; next launch the build is known via
   `derived_profile_cache`, so calibration runs once per new patch.

## Error Handling

- Host `calibrate` failures map to existing `ErrorCode` values; no new codes.
- Core never propagates raw host error text to `userMessage`; it categorizes
  and maps. Raw text is preserved only inside `details`.
- Calibration is best-effort: a calibration failure degrades to the
  "unsupported build" status, it does not break `check_codec` (the pure-Rust
  backend probe still returns).

## Testing

**Host (Rust unit/integration):**
- `calibrate` is a no-op on a known-profile exe (1.0.0/1.0.1) and returns
  supported.
- `calibrate` on a pattern-only exe writes the derived cache and returns a
  promoted `derived_profile_cache` probe. (Uses real 1.0.1 build when present;
  gated/skipped when the local binary fixture is absent.)
- Embedded sample decompresses to the expected SHA-1 (known-answer) against a
  current build.
- A deliberately broken/unsupported input yields a structured `unsupported_exe`
  error.

**Core (Rust):**
- `check_codec` invokes calibrate when the binary probe is `pattern_profile`
  and surfaces the promoted status (with an injected/mocked invoker).
- `check_codec` maps each category to the correct
  `userTitle`/`userMessage`/`userHint`/`userSeverity`.
- exe-missing input → `Gothic 1 Remake not found` category.

**App (Flutter widget):**
- Status panel renders plain message + hint and hides technical fields behind
  `Details`.
- Unsupported-build state shows the error copy and the hint.

## Risks

- **Sample version-stability:** relies on Oodle Kraken decode being
  backward-compatible across builds. True for the Kraken container format; the
  known-answer test itself catches any violation (calibration fails → safe
  "unsupported" fallback rather than silent corruption).
- **Calibration latency:** maps the exe + spawns a worker (~1–2 s) on first
  encounter of a new build. Acceptable; runs once per patch, behind the
  existing check spinner.
- **Trust model preserved:** a build is enabled only after a real Oodle
  round-trip passes on the user's machine — same safety bar as the manual
  verify path, now automatic.
