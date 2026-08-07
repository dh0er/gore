//! Localized text, FMOD audio banks, and voice-over archives.
//!
//! What these three share is that they edit the game's own content files, so each carries the
//! distinction between "produce a new file" and "rewrite the original". That distinction is the
//! whole reason [`Safety::write_or_in_place`] exists.
//!
//! Every `summary` and `help` string is copied verbatim from the corresponding clap doc comment.

use crate::spec::{
    ArgForm::{Long, Switch},
    ArgKind::{Bool, Int, Path, Str},
    ArgSpec, CommandSpec, GroupShape, GroupSpec, JsonSupport, Safety, T_FAST, T_LONG, T_NORMAL,
};

// ---------------------------------------------------------------------------------------------
// gore_loc
// ---------------------------------------------------------------------------------------------

const LOC_EXTRACT_ARGS: &[ArgSpec] = &[ArgSpec::new(
    "lcache",
    Long("lcache"),
    Path,
    "Path to the .lcache, the game dir, or a Steam library (else auto-detect)",
    false,
)
.with_default("auto-detect from the configured game path")];

const LOC_EXPORT_ARGS: &[ArgSpec] = &[
    ArgSpec::new(
        "lcache",
        Long("lcache"),
        Path,
        "Path to AlkimiaLocalization_*.lcache, the game dir, or a Steam library (else \
         auto-detect)",
        false,
    )
    .with_default("auto-detect from the configured game path"),
    ArgSpec::new("out", Long("out"), Path, "Output loc_catalog.json", true),
    ArgSpec::new(
        "keep_empty",
        Switch("keep-empty"),
        Bool,
        "Keep empty values / ids with no text",
        false,
    ),
];

const LOC_IMPORT_ARGS: &[ArgSpec] = &[
    ArgSpec::new(
        "lcache",
        Long("lcache"),
        Path,
        "Path to the .lcache to edit, the game dir, or a Steam library (else auto-detect)",
        false,
    )
    .with_default("auto-detect from the configured game path"),
    ArgSpec::new("edits", Long("edits"), Path, "Path to edits JSON ({id:{language:value}})", true),
    ArgSpec::new(
        "out",
        Long("out"),
        Path,
        "Output .lcache (defaults to overwriting the cache that was read)",
        false,
    ),
    ArgSpec::new(
        "add_missing",
        Switch("add-missing"),
        Bool,
        "Add ids absent from the input .lcache (default: reject them)",
        false,
    ),
];

const LOC_COMMANDS: &[CommandSpec] = &[
    // `gore loc extract` prompts on stdin for a y/N confirmation unless it is given `--yes`
    // (crates/gore/src/cmd/loc.rs). Under the stdio transport OUR stdin is the JSON-RPC channel,
    // so an unsuppressed prompt would deadlock the entire session — the child would wait for input
    // that can never arrive while we wait for it to exit. `--yes` is forced here and the child is
    // additionally spawned with a null stdin, so neither mechanism alone has to be trusted.
    CommandSpec::new(
        "extract",
        "Auto-detect (or --lcache) the game's .lcache and write the shared gore/loc_catalog.json \
         (used by the save editor and mod studio too)",
        LOC_EXTRACT_ARGS,
        // Gated even though it never touches the game installation. The CLI guards this write with
        // a y/N prompt, and forcing `--yes` above removes that confirmation without replacing it —
        // so the gate takes the prompt's place. The target is the shared catalog the save editor
        // and mod studio also read, and it is derived from no argument, so the `truncates` rule
        // cannot see it: an already-extracted catalog would otherwise be replaced ungated.
        Safety::mutate(),
        T_LONG,
    )
    .gated_because(
        "replaces the shared `gore/loc_catalog.json` that the save editor and mod studio read \
         too, and the CLI's own y/N confirmation is suppressed here, so this question takes its \
         place",
    )
    .forced(&["--yes"])
    .guide("text-and-dialogs"),
    CommandSpec::new(
        "status",
        "Show the shared loc catalog's status (ids, languages, source)",
        &[],
        Safety::read(),
        T_FAST,
    )
    .guide("text-and-dialogs"),
    CommandSpec::new(
        "export",
        "Decrypt the .lcache and write {id:{language:value}} JSON (all languages)",
        LOC_EXPORT_ARGS,
        Safety::write_truncating(&["out"]),
        T_NORMAL,
    )
    .guide("text-and-dialogs"),
    CommandSpec::new(
        "import",
        "Apply {id:{language:value}} edits and re-encrypt the .lcache",
        LOC_IMPORT_ARGS,
        Safety::write_or_in_place(&["out"]),
        T_NORMAL,
    )
    .guide("text-and-dialogs"),
];

pub const LOC: GroupSpec = GroupSpec {
    tool: "gore_loc",
    title: "gore loc",
    cli: "loc",
    summary: "Read and edit the game's localized text, which lives in an encrypted \
              AlkimiaLocalization .lcache. Export decrypts it to JSON, import re-encrypts edits \
              back in.",
    shape: GroupShape::Nested,
    commands: LOC_COMMANDS,
};

// ---------------------------------------------------------------------------------------------
// gore_audio
// ---------------------------------------------------------------------------------------------

const AUDIO_KEY: ArgSpec = ArgSpec::new(
    "key",
    Long("key"),
    Str,
    "Override the bank encryption key (defaults to the Gothic 1 Remake key)",
    false,
)
.with_default("the built-in Gothic 1 Remake studio key");

/// `banks` is the one `audio` command that takes no `--bank`, because it is where a `--bank` comes
/// from: it resolves the configured install and describes its FMOD directory. Exposing `game` here
/// matters for the same reason it does on `texture` — an agent handed a path by the caller must be
/// able to describe that install rather than the configured one.
const AUDIO_BANKS_ARGS: &[ArgSpec] = &[
    ArgSpec::new(
        "game",
        Long("game"),
        Path,
        "Game root (the folder containing G1R/)",
        false,
    )
    .with_default("the configured game path, then Steam auto-detect"),
    AUDIO_KEY,
];

/// `list` is bounded by `--max` in the CLI itself, which is what keeps `SFX.bank`'s 7,218 samples
/// from being clipped mid-line into a result whose back half is simply absent. The bound is only
/// useful if an agent can move it, so both narrowing flags are exposed here.
const AUDIO_LIST_ARGS: &[ArgSpec] = &[
    ArgSpec::new("bank", Long("bank"), Path, "Path to a .bank file", true),
    ArgSpec::new(
        "filter",
        Long("filter"),
        Str,
        "Keep only sample names containing this substring (case-insensitive)",
        false,
    ),
    ArgSpec::new(
        "max",
        Long("max"),
        Int { min: Some(0), max: None },
        "Max samples to print. The result states how many matched when it stops here; 0 lists \
         nothing and reports only the counts",
        false,
    )
    .with_default("100"),
    AUDIO_KEY,
];

const AUDIO_EXTRACT_ARGS: &[ArgSpec] = &[
    ArgSpec::new("bank", Long("bank"), Path, "Path to a .bank file", true),
    ArgSpec::new("out", Long("out"), Path, "Output directory for .wav files", true),
    ArgSpec::new("sample", Long("sample"), Str, "A single sample name, or \"all\"", false)
        .with_default("all"),
    ArgSpec::new(
        "filter",
        Long("filter"),
        Str,
        "Extract every sample whose name contains this substring (case-insensitive)",
        false,
    ),
    AUDIO_KEY,
];

const AUDIO_REPLACE_ARGS: &[ArgSpec] = &[
    ArgSpec::new(
        "map",
        Long("map"),
        Path,
        "Path to map JSON: { \"SampleName\": \"path/to/new.wav\", … } (WAV paths relative to it)",
        true,
    ),
    ArgSpec::new("bank", Long("bank"), Path, "Path to the .bank to modify", true),
    ArgSpec::new(
        "out",
        Long("out"),
        Path,
        "Output .bank. Omitting this overwrites --bank in place, backing up to *.gore-bak.",
        false,
    ),
    AUDIO_KEY,
];

const AUDIO_RESTORE_ARGS: &[ArgSpec] =
    &[ArgSpec::new("bank", Long("bank"), Path, "Path to the .bank to restore", true)];

const AUDIO_EXPORT_PATCH_ARGS: &[ArgSpec] = &[
    ArgSpec::new(
        "map",
        Long("map"),
        Path,
        "Path to map JSON: { \"SampleName\": \"path/to/new.wav\", … }",
        true,
    ),
    ArgSpec::new("out", Long("out"), Path, "Output patch .zip", true),
];

const AUDIO_APPLY_PATCH_ARGS: &[ArgSpec] = &[
    ArgSpec::new("patch", Long("patch"), Path, "Path to the patch .zip", true),
    ArgSpec::new("bank", Long("bank"), Path, "Path to the .bank to modify", true),
    ArgSpec::new(
        "out",
        Long("out"),
        Path,
        "Output .bank. Omitting this overwrites --bank in place, backing up to *.gore-bak.",
        false,
    ),
    AUDIO_KEY,
];

const AUDIO_COMMANDS: &[CommandSpec] = &[
    CommandSpec::new(
        "banks",
        "List the .bank files the configured install carries, with each one's sample count",
        AUDIO_BANKS_ARGS,
        Safety::read(),
        T_NORMAL,
    )
    .json(JsonSupport::Stdout)
    .guide("audio"),
    CommandSpec::new(
        "list",
        "List a bank's samples (name, codec, sample rate, channels, duration)",
        AUDIO_LIST_ARGS,
        Safety::read(),
        T_NORMAL,
    )
    .json(JsonSupport::Stdout)
    .guide("audio"),
    CommandSpec::new(
        "extract",
        "Extract samples to WAV (.wav) for listening/editing",
        AUDIO_EXTRACT_ARGS,
        // Writes `<out>/<index>_<sample>.wav` per sample, under names taken from the bank's own
        // sample list, which this layer cannot read. It used to ask whenever `out` was non-empty,
        // and that is the workflow: auditioning candidates one at a time filled the directory, so
        // the second extract raised a confirmation because the first had succeeded. The CLI now
        // refuses the individual file it would replace and names it, which protects the same thing
        // without standing in front of the ordinary case. The two halves ship together — dropping
        // this facet against an older CLI would leave neither layer checking.
        Safety::write(),
        T_NORMAL,
    )
    .guide("audio"),
    CommandSpec::new(
        "replace",
        "Replace samples with new audio (WAV) via PCM injection",
        AUDIO_REPLACE_ARGS,
        Safety::write_or_in_place(&["out"]),
        T_NORMAL,
    )
    .guide("audio"),
    // Restoring is always in place by definition — that is what it means.
    CommandSpec::new(
        "restore",
        "Restore a bank from its *.gore-bak backup",
        AUDIO_RESTORE_ARGS,
        Safety::mutate(),
        T_NORMAL,
    )
    .gated_because(
        "overwrites the bank in place with its `*.gore-bak` backup, discarding whatever is in the \
         bank now",
    )
    .guide("audio"),
    CommandSpec::new(
        "export-patch",
        "Build a shareable audio patch zip (manifest + replacement WAVs, no game audio)",
        AUDIO_EXPORT_PATCH_ARGS,
        Safety::write_truncating(&["out"]),
        T_NORMAL,
    )
    .guide("audio"),
    CommandSpec::new(
        "apply-patch",
        "Apply a patch zip (from export-patch) to a bank",
        AUDIO_APPLY_PATCH_ARGS,
        Safety::write_or_in_place(&["out"]),
        T_NORMAL,
    )
    .guide("audio"),
];

pub const AUDIO: GroupSpec = GroupSpec {
    tool: "gore_audio",
    title: "gore audio",
    cli: "audio",
    summary: "Read and replace sounds in the game's encrypted FMOD banks (.bank): list samples, \
              extract them to WAV, inject replacements, and package the result as a shareable \
              patch.",
    shape: GroupShape::Nested,
    commands: AUDIO_COMMANDS,
};

// ---------------------------------------------------------------------------------------------
// gore_voice
// ---------------------------------------------------------------------------------------------

const VOICE_ARCHIVE: ArgSpec =
    ArgSpec::new("archive", Long("archive"), Path, "Input voice ZIP (never modified)", true);

/// The `--basename` / `--path` selector, shared by `extract` and `replace`.
///
/// clap declares these as mutually exclusive and jointly required; the same rule is registered on
/// the command via `exactly_one` so a wrong call is rejected before a process is spawned.
const VOICE_BASENAME: ArgSpec = ArgSpec::new(
    "basename",
    Long("basename"),
    Str,
    "Case-insensitive basename; accepted only when it identifies one entry. Give either this or \
     `path`, never both.",
    false,
);

const VOICE_SELECTOR_PATH: ArgSpec = ArgSpec::new(
    "path",
    Long("path"),
    Str,
    "Case-sensitive complete archive path (use this to disambiguate basenames). Give either this \
     or `basename`, never both.",
    false,
);

const VOICE_SELECTOR: &[&[&str]] = &[&["basename", "path"]];

const VOICE_OUT_ZIP: ArgSpec =
    ArgSpec::new("out", Long("out"), Path, "New output ZIP; must not already exist", true);

/// `list` is bounded by `--max` in the CLI itself, which is what keeps a 33,000-entry archive from
/// being clipped mid-array into a JSON document that no longer parses. The bound is only useful if
/// an agent can move it, so all three narrowing flags are exposed here.
const VOICE_LIST_ARGS: &[ArgSpec] = &[
    VOICE_ARCHIVE,
    ArgSpec::new(
        "filter",
        Long("filter"),
        Str,
        "Keep only entry paths containing this substring (case-insensitive)",
        false,
    ),
    ArgSpec::new(
        "max",
        Long("max"),
        Int { min: Some(0), max: None },
        "Max entries to print. The result states how many matched when it stops here; 0 lists \
         nothing and reports only the counts",
        false,
    )
    .with_default("100"),
    ArgSpec::new(
        "directories",
        Switch("directories"),
        Bool,
        "Also list the archive's directory entries, which carry no audio",
        false,
    ),
];

const VOICE_MATCH_LINE_ARGS: &[ArgSpec] = &[
    VOICE_ARCHIVE,
    ArgSpec::new(
        "loc_id",
        Long("loc-id"),
        Str,
        "Trimmed ASCII localization ID (without the `.ogg` suffix)",
        true,
    ),
];

const VOICE_EXTRACT_ARGS: &[ArgSpec] = &[
    VOICE_ARCHIVE,
    VOICE_BASENAME,
    VOICE_SELECTOR_PATH,
    ArgSpec::new(
        "out",
        Long("out"),
        Path,
        "Extraction root; the archive path is preserved below it",
        true,
    ),
];

const VOICE_ADD_ARGS: &[ArgSpec] = &[
    VOICE_ARCHIVE,
    ArgSpec::new(
        "path",
        Long("path"),
        Str,
        "Full path for the new entry inside the archive",
        true,
    ),
    ArgSpec::new(
        "ogg",
        Long("ogg"),
        Path,
        "Ogg file to add — Vorbis or Opus. A WAV needs converting first: ffmpeg -i line.wav -c:a          libvorbis -ar 48000 -ac 1 -q:a 5 line.ogg",
        true,
    ),
    VOICE_OUT_ZIP,
];

const VOICE_REPLACE_ARGS: &[ArgSpec] = &[
    VOICE_ARCHIVE,
    VOICE_BASENAME,
    VOICE_SELECTOR_PATH,
    ArgSpec::new(
        "ogg",
        Long("ogg"),
        Path,
        "Ogg replacement file — Vorbis or Opus. A WAV needs converting first: ffmpeg -i line.wav          -c:a libvorbis -ar 48000 -ac 1 -q:a 5 line.ogg",
        true,
    ),
    VOICE_OUT_ZIP,
];

const VOICE_APPLY_MANIFEST_ARGS: &[ArgSpec] = &[
    VOICE_ARCHIVE,
    ArgSpec::new(
        "manifest",
        Long("manifest"),
        Path,
        "Versioned JSON manifest; Ogg paths are relative to this file",
        true,
    ),
    VOICE_OUT_ZIP,
];

const VOICE_COMMANDS: &[CommandSpec] = &[
    CommandSpec::new(
        "list",
        "Index a voice archive and list a bounded page of its entries",
        VOICE_LIST_ARGS,
        Safety::read(),
        T_NORMAL,
    )
    .json(JsonSupport::Stdout)
    .aliases(&["index"])
    .guide("voice"),
    CommandSpec::new(
        "match-line",
        "Resolve an exact `${loc_id}.ogg` basename without extracting it",
        VOICE_MATCH_LINE_ARGS,
        Safety::read(),
        T_NORMAL,
    )
    .json(JsonSupport::Stdout)
    .guide("voice"),
    // Aimed at the same scratch directory, `voice extract` runs and `audio extract` asks. The
    // difference is not the destination but what each can promise about it: this one extracts one
    // named entry to one named path and refuses that path if it exists (`gore-vo` writes
    // copy-on-write), while `audio extract` writes a file per sample under names it only learns
    // from the bank. Only the second is unpreflightable, and the reason each command now states is
    // what makes the pair legible from outside.
    CommandSpec::new(
        "extract",
        "Extract one entry without overwriting an existing file",
        VOICE_EXTRACT_ARGS,
        Safety::write().installs_via(&["out"]),
        T_NORMAL,
    )
    .exactly_one(VOICE_SELECTOR)
    .guide("voice"),
    CommandSpec::new(
        "add",
        "Append a validated Ogg file to a new archive",
        VOICE_ADD_ARGS,
        Safety::write().installs_via(&["out"]),
        T_NORMAL,
    )
    .guide("voice"),
    CommandSpec::new(
        "replace",
        "Replace one entry with a validated Ogg file in a new archive",
        VOICE_REPLACE_ARGS,
        Safety::write().installs_via(&["out"]),
        T_NORMAL,
    )
    .exactly_one(VOICE_SELECTOR)
    .guide("voice"),
    CommandSpec::new(
        "apply-manifest",
        "Apply a versioned JSON edit manifest to a new archive in one pass",
        VOICE_APPLY_MANIFEST_ARGS,
        Safety::write().installs_via(&["out"]),
        T_LONG,
    )
    .aliases(&["apply"])
    .guide("voice"),
];

pub const VOICE: GroupSpec = GroupSpec {
    tool: "gore_voice",
    title: "gore voice",
    cli: "voice",
    summary: "Inspect and edit voice-over ZIP archives. Strictly copy-on-write: the input archive \
              is never modified and the output must not already exist, so nothing here can \
              overwrite recorded audio.",
    shape: GroupShape::Nested,
    commands: VOICE_COMMANDS,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_group_sizes_match_the_cli() {
        assert_eq!(LOC.commands.len(), 4);
        assert_eq!(AUDIO.commands.len(), 7);
        assert_eq!(VOICE.commands.len(), 6);
    }

    #[test]
    fn both_bounded_listings_expose_the_same_narrowing_flags() {
        // `voice list` was bounded first and `audio list` was not, so an agent that hit the 256 KiB
        // result cap on `SFX.bank` was told to "narrow the query with a filter" by a command that
        // had none. They are one convention now, and this is what keeps them one: neither may gain
        // or lose a narrowing flag alone.
        for group in [AUDIO, VOICE] {
            let list = group.command("list").expect("both groups list");
            for name in ["filter", "max"] {
                assert!(
                    list.arg(name).is_some(),
                    "{} list must declare `{name}`",
                    group.cli
                );
            }
            assert_eq!(
                list.json,
                JsonSupport::Stdout,
                "{} list must offer the machine-readable mode",
                group.cli
            );
            // The switch is implied by `JsonSupport::Stdout` and appended by the argv builder;
            // declaring it as well would let a caller pass it twice.
            assert!(
                list.arg("json").is_none(),
                "{} list must not also declare `json` as an argument",
                group.cli
            );
        }
    }

    #[test]
    fn loc_extract_always_suppresses_its_stdin_prompt() {
        let extract = LOC.command("extract").expect("extract exists");
        assert_eq!(extract.forced_argv, &["--yes"]);
        // The caller must not be able to turn it back on.
        assert!(extract.arg("yes").is_none());
        // Suppressing the confirmation is only defensible because the gate replaces it: the
        // shared catalog is not something an agent may overwrite on its own initiative.
        assert!(
            extract.safety.requirements(&serde_json::Map::new()).write,
            "a forced --yes must be paid for with --allow-write"
        );
    }

    #[test]
    fn every_command_that_can_overwrite_its_input_declares_the_argument_that_prevents_it() {
        let in_place: Vec<&str> = [LOC, AUDIO, VOICE]
            .iter()
            .flat_map(|group| group.commands.iter())
            .filter(|command| command.safety.in_place_without.is_some())
            .map(|command| command.sub)
            .collect();
        assert_eq!(in_place, vec!["import", "replace", "apply-patch"]);
    }

    #[test]
    fn the_voice_selector_is_registered_on_exactly_the_commands_that_take_one() {
        let with_selector: Vec<&str> = VOICE
            .commands
            .iter()
            .filter(|command| !command.exactly_one_of.is_empty())
            .map(|command| command.sub)
            .collect();
        assert_eq!(with_selector, vec!["extract", "replace"]);
    }

    #[test]
    fn voice_add_takes_path_as_a_required_argument_rather_than_a_selector() {
        // `add` reuses the name `path` for something else entirely — where the new entry goes —
        // so it is required there and must not be part of an exclusive set.
        let add = VOICE.command("add").expect("add exists");
        assert!(add.arg("path").expect("path").required);
        assert!(add.exactly_one_of.is_empty());
    }
}
