//! Localized text, FMOD audio banks, and voice-over archives.
//!
//! What these three share is that they edit the game's own content files, so each carries the
//! distinction between "produce a new file" and "rewrite the original". That distinction is the
//! whole reason [`Safety::write_or_in_place`] exists.
//!
//! Every `summary` and `help` string is copied verbatim from the corresponding clap doc comment.

use crate::spec::{
    ArgForm::{Long, Switch},
    ArgKind::{Bool, Path, Str},
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
    ArgSpec::new("lcache", Long("lcache"), Path, "Path to AlkimiaLocalization_*.lcache", true),
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
    ArgSpec::new("lcache", Long("lcache"), Path, "Path to the .lcache to edit", true),
    ArgSpec::new("edits", Long("edits"), Path, "Path to edits JSON ({id:{language:value}})", true),
    ArgSpec::new(
        "out",
        Long("out"),
        Path,
        "Output .lcache. Omitting this overwrites the input .lcache in place, re-encrypted.",
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
        Safety::write(),
        T_NORMAL,
    )
    .guide("text-and-dialogs"),
    CommandSpec::new(
        "import",
        "Apply {id:{language:value}} edits and re-encrypt the .lcache",
        LOC_IMPORT_ARGS,
        Safety::write_or_in_place("out"),
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

const AUDIO_LIST_ARGS: &[ArgSpec] =
    &[ArgSpec::new("bank", Long("bank"), Path, "Path to a .bank file", true), AUDIO_KEY];

const AUDIO_EXTRACT_ARGS: &[ArgSpec] = &[
    ArgSpec::new("bank", Long("bank"), Path, "Path to a .bank file", true),
    ArgSpec::new("out", Long("out"), Path, "Output directory for .wav files", true),
    ArgSpec::new("sample", Long("sample"), Str, "A single sample name, or \"all\"", false)
        .with_default("all"),
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
        "list",
        "List a bank's samples (name, codec, sample rate, channels, duration)",
        AUDIO_LIST_ARGS,
        Safety::read(),
        T_NORMAL,
    )
    .guide("audio"),
    CommandSpec::new(
        "extract",
        "Extract samples to WAV (.wav) for listening/editing",
        AUDIO_EXTRACT_ARGS,
        Safety::write(),
        T_NORMAL,
    )
    .guide("audio"),
    CommandSpec::new(
        "replace",
        "Replace samples with new audio (WAV) via PCM injection",
        AUDIO_REPLACE_ARGS,
        Safety::write_or_in_place("out"),
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
    .guide("audio"),
    CommandSpec::new(
        "export-patch",
        "Build a shareable audio patch zip (manifest + replacement WAVs, no game audio)",
        AUDIO_EXPORT_PATCH_ARGS,
        Safety::write(),
        T_NORMAL,
    )
    .guide("audio"),
    CommandSpec::new(
        "apply-patch",
        "Apply a patch zip (from export-patch) to a bank",
        AUDIO_APPLY_PATCH_ARGS,
        Safety::write_or_in_place("out"),
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

const VOICE_LIST_ARGS: &[ArgSpec] = &[VOICE_ARCHIVE];

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
    ArgSpec::new("ogg", Long("ogg"), Path, "Ogg/Vorbis or Ogg/Opus file to add", true),
    VOICE_OUT_ZIP,
];

const VOICE_REPLACE_ARGS: &[ArgSpec] = &[
    VOICE_ARCHIVE,
    VOICE_BASENAME,
    VOICE_SELECTOR_PATH,
    ArgSpec::new("ogg", Long("ogg"), Path, "Ogg/Vorbis or Ogg/Opus replacement file", true),
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
        "Index and list every entry in a voice archive",
        VOICE_LIST_ARGS,
        Safety::read(),
        T_NORMAL,
    )
    .json(JsonSupport::Stdout)
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
    CommandSpec::new(
        "extract",
        "Extract one entry without overwriting an existing file",
        VOICE_EXTRACT_ARGS,
        Safety::write(),
        T_NORMAL,
    )
    .exactly_one(VOICE_SELECTOR)
    .guide("voice"),
    CommandSpec::new(
        "add",
        "Append a validated Ogg file to a new archive",
        VOICE_ADD_ARGS,
        Safety::write(),
        T_NORMAL,
    )
    .guide("voice"),
    CommandSpec::new(
        "replace",
        "Replace one entry with a validated Ogg file in a new archive",
        VOICE_REPLACE_ARGS,
        Safety::write(),
        T_NORMAL,
    )
    .exactly_one(VOICE_SELECTOR)
    .guide("voice"),
    CommandSpec::new(
        "apply-manifest",
        "Apply a versioned JSON edit manifest to a new archive in one pass",
        VOICE_APPLY_MANIFEST_ARGS,
        Safety::write(),
        T_LONG,
    )
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
        assert_eq!(AUDIO.commands.len(), 6);
        assert_eq!(VOICE.commands.len(), 6);
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
