//! Save Editor CLI command surface. Checked against clap by mcp_spec_sync.
use crate::spec::{
    ArgForm::{Long, Positional, Switch},
    ArgKind::{Bool, Float, Int, Path, Str},
    ArgSpec, CommandSpec, GroupShape, GroupSpec, JsonSupport, Safety, T_LONG,
};
const OPTIONS: &[ArgSpec] = &[
    ArgSpec::new("save", Positional { order: 0 }, Path, "save", false),
    ArgSpec::new("root", Long("root"), Path, "root", false),
    ArgSpec::new(
        "profile",
        Long("profile"),
        Int {
            min: None,
            max: None,
        },
        "profile",
        false,
    ),
    ArgSpec::new("other", Switch("other"), Bool, "other", false),
    ArgSpec::new("actor", Long("actor"), Str, "actor", false),
    ArgSpec::new("id", Long("id"), Str, "id", false),
    ArgSpec::new("item", Long("item"), Str, "item", false),
    ArgSpec::new(
        "count",
        Long("count"),
        Int {
            min: None,
            max: None,
        },
        "count",
        false,
    ),
    ArgSpec::new("container", Long("container"), Str, "container", false),
    ArgSpec::new("slot", Long("slot"), Str, "slot", false),
    ArgSpec::new(
        "index",
        Long("index"),
        Int {
            min: Some(0),
            max: None,
        },
        "index",
        false,
    ),
    ArgSpec::new("map", Long("map"), Str, "map", false),
    ArgSpec::new("query", Long("query"), Str, "query", false),
    ArgSpec::new("state", Long("state"), Str, "state", false),
    ArgSpec::new("group", Long("group"), Str, "group", false),
    ArgSpec::new("category", Long("category"), Str, "category", false),
    ArgSpec::new("kind", Long("kind"), Str, "kind", false),
    ArgSpec::new("role", Long("role"), Str, "role", false),
    ArgSpec::new("source", Long("source"), Str, "source", false),
    ArgSpec::new("type_filter", Long("type"), Str, "type", false),
    ArgSpec::new("editable", Long("editable"), Bool, "editable", false),
    ArgSpec::new("all", Switch("all"), Bool, "all", false),
    ArgSpec::new(
        "offset",
        Long("offset"),
        Int {
            min: Some(0),
            max: None,
        },
        "offset",
        false,
    ),
    ArgSpec::new(
        "limit",
        Long("limit"),
        Int {
            min: Some(0),
            max: None,
        },
        "limit",
        false,
    ),
    ArgSpec::new(
        "include_unset",
        Switch("include-unset"),
        Bool,
        "include unset",
        false,
    ),
    ArgSpec::new(
        "semantic_type",
        Long("semantic-type"),
        Str,
        "semantic type",
        false,
    ),
    ArgSpec::new("set_class", Long("set-class"), Str, "set class", false),
    ArgSpec::new("attribute", Long("attribute"), Str, "attribute", false),
    ArgSpec::new("field", Long("field"), Str, "field", false),
    ArgSpec::new("base", Long("base"), Float, "base", false),
    ArgSpec::new("current", Long("current"), Float, "current", false),
    ArgSpec::new("skill", Long("skill"), Str, "skill", false),
    ArgSpec::new("tier", Long("tier"), Str, "tier", false),
    ArgSpec::new(
        "relationship",
        Long("relationship"),
        Str,
        "relationship",
        false,
    ),
    ArgSpec::new("location", Long("location"), Str, "location", false),
    ArgSpec::new(
        "apply_facing",
        Switch("apply-facing"),
        Bool,
        "apply facing",
        false,
    ),
    ArgSpec::new("stay", Switch("stay"), Bool, "stay", false),
    ArgSpec::new("x", Long("x"), Float, "x", false),
    ArgSpec::new("y", Long("y"), Float, "y", false),
    ArgSpec::new("z", Long("z"), Float, "z", false),
    ArgSpec::new("pitch", Long("pitch"), Float, "pitch", false),
    ArgSpec::new("yaw", Long("yaw"), Float, "yaw", false),
    ArgSpec::new("roll", Long("roll"), Float, "roll", false),
    ArgSpec::new("seconds", Long("seconds"), Float, "seconds", false),
    ArgSpec::new(
        "day",
        Long("day"),
        Int {
            min: Some(0),
            max: None,
        },
        "day",
        false,
    ),
    ArgSpec::new(
        "hour",
        Long("hour"),
        Int {
            min: Some(0),
            max: None,
        },
        "hour",
        false,
    ),
    ArgSpec::new(
        "minute",
        Long("minute"),
        Int {
            min: Some(0),
            max: None,
        },
        "minute",
        false,
    ),
    ArgSpec::new(
        "second",
        Long("second"),
        Int {
            min: Some(0),
            max: None,
        },
        "second",
        false,
    ),
    ArgSpec::new("preset", Long("preset"), Str, "preset", false),
    ArgSpec::new("combat", Long("combat"), Str, "combat", false),
    ArgSpec::new("resources", Long("resources"), Str, "resources", false),
    ArgSpec::new(
        "progression",
        Long("progression"),
        Str,
        "progression",
        false,
    ),
    ArgSpec::new(
        "flow_helper",
        Long("flow-helper"),
        Bool,
        "flow helper",
        false,
    ),
    ArgSpec::new("permadeath", Long("permadeath"), Bool, "permadeath", false),
    ArgSpec::new(
        "resources_level",
        Long("resources-level"),
        Str,
        "resources level",
        false,
    ),
    ArgSpec::new("document", Long("document"), Str, "document", false),
    ArgSpec::new("segment", Long("segment"), Str, "segment", false),
    ArgSpec::new("entry", Long("entry"), Str, "entry", false),
    ArgSpec::new("character", Long("character"), Str, "character", false),
    ArgSpec::new("guild", Long("guild"), Str, "guild", false),
    ArgSpec::new("lock", Long("lock"), Str, "lock", false),
    ArgSpec::new("name", Long("name"), Str, "name", false),
    ArgSpec::new(
        "clear_name",
        Switch("clear-name"),
        Bool,
        "clear name",
        false,
    ),
    ArgSpec::new("path_file", Long("path-file"), Path, "path file", false),
    ArgSpec::new("value_json", Long("value-json"), Str, "value json", false),
    ArgSpec::new(
        "request_file",
        Long("request-file"),
        Path,
        "request file",
        false,
    ),
    ArgSpec::new(
        "payload_file",
        Long("payload-file"),
        Path,
        "payload file",
        false,
    ),
    ArgSpec::new("expect", Long("expect"), Str, "expect", false),
    ArgSpec::new(
        "allow_unknown_create",
        Switch("allow-unknown-create"),
        Bool,
        "allow unknown create",
        false,
    ),
    ArgSpec::new("backup", Long("backup"), Path, "backup", false),
    ArgSpec::new(
        "include_companions",
        Switch("include-companions"),
        Bool,
        "include companions",
        false,
    ),
    ArgSpec::new("target", Long("target"), Path, "target", false),
    ArgSpec::new("out", Long("out"), Path, "out", false),
    ArgSpec::new("draft", Long("draft"), Path, "draft", false),
    ArgSpec::new(
        "operation",
        Long("operation"),
        Int {
            min: Some(0),
            max: None,
        },
        "operation",
        false,
    ),
    ArgSpec::new("dry_run", Switch("dry-run"), Bool, "dry run", false),
    ArgSpec::new("yes", Switch("yes"), Bool, "yes", false),
    ArgSpec::new("private", Switch("private"), Bool, "private", false),
    ArgSpec::new(
        "private_chunk_limit",
        Long("private-chunk-limit"),
        Int {
            min: Some(0),
            max: None,
        },
        "private chunk limit",
        false,
    ),
    ArgSpec::new("details", Switch("details"), Bool, "details", false),
    ArgSpec::new(
        "with_assets",
        Switch("with-assets"),
        Bool,
        "with assets",
        false,
    ),
    ArgSpec::new("game", Long("game"), Path, "game", false),
    ArgSpec::new("manifest", Long("manifest"), Path, "manifest", false),
    ArgSpec::new("lcache", Long("lcache"), Path, "lcache", false),
    ArgSpec::new("lang", Long("lang"), Str, "lang", false),
    ArgSpec::new("game_lang", Long("game-lang"), Str, "game_lang", false),
    ArgSpec::new("show_ids", Switch("show-ids"), Bool, "show ids", false),
    ArgSpec::new("copy", Switch("copy"), Bool, "copy", false),
    ArgSpec::new("open", Switch("open"), Bool, "open", false),
    ArgSpec::new("scope", Long("scope"), Str, "scope", false),
    ArgSpec::new("key", Long("key"), Str, "key", false),
    ArgSpec::new("value", Long("value"), Str, "value", false),
    ArgSpec::new("format", Long("format"), Str, "format", false),
    ArgSpec::new("product", Long("product"), Str, "product", false),
    ArgSpec::new("roundtrip", Switch("roundtrip"), Bool, "roundtrip", false),
    ArgSpec::new(
        "codec_roundtrip",
        Switch("codec-roundtrip"),
        Bool,
        "codec roundtrip",
        false,
    ),
];
// Read leaves share the CLI argument type; its inert --out flag is omitted
// from the MCP schema so it cannot advertise an output the leaf never writes.
const READ_OPTIONS: &[ArgSpec] = &[
    OPTIONS[0],
    OPTIONS[1],
    OPTIONS[2],
    OPTIONS[3],
    OPTIONS[4],
    OPTIONS[5],
    OPTIONS[6],
    OPTIONS[7],
    OPTIONS[8],
    OPTIONS[9],
    OPTIONS[10],
    OPTIONS[11],
    OPTIONS[12],
    OPTIONS[13],
    OPTIONS[14],
    OPTIONS[15],
    OPTIONS[16],
    OPTIONS[17],
    OPTIONS[18],
    OPTIONS[19],
    OPTIONS[20],
    OPTIONS[21],
    OPTIONS[22],
    OPTIONS[23],
    OPTIONS[24],
    OPTIONS[25],
    OPTIONS[26],
    OPTIONS[27],
    OPTIONS[28],
    OPTIONS[29],
    OPTIONS[30],
    OPTIONS[31],
    OPTIONS[32],
    OPTIONS[33],
    OPTIONS[34],
    OPTIONS[35],
    OPTIONS[36],
    OPTIONS[37],
    OPTIONS[38],
    OPTIONS[39],
    OPTIONS[40],
    OPTIONS[41],
    OPTIONS[42],
    OPTIONS[43],
    OPTIONS[44],
    OPTIONS[45],
    OPTIONS[46],
    OPTIONS[47],
    OPTIONS[48],
    OPTIONS[49],
    OPTIONS[50],
    OPTIONS[51],
    OPTIONS[52],
    OPTIONS[53],
    OPTIONS[54],
    OPTIONS[55],
    OPTIONS[56],
    OPTIONS[57],
    OPTIONS[58],
    OPTIONS[59],
    OPTIONS[60],
    OPTIONS[61],
    OPTIONS[62],
    OPTIONS[63],
    OPTIONS[64],
    OPTIONS[65],
    OPTIONS[66],
    OPTIONS[67],
    OPTIONS[68],
    OPTIONS[69],
    OPTIONS[70],
    OPTIONS[71],
    OPTIONS[73],
    OPTIONS[74],
    OPTIONS[75],
    OPTIONS[76],
    OPTIONS[77],
    OPTIONS[78],
    OPTIONS[79],
    OPTIONS[80],
    OPTIONS[81],
    OPTIONS[82],
    OPTIONS[83],
    OPTIONS[84],
    OPTIONS[85],
    OPTIONS[86],
    OPTIONS[87],
    OPTIONS[88],
    OPTIONS[89],
    OPTIONS[90],
    OPTIONS[91],
    OPTIONS[92],
    OPTIONS[93],
    OPTIONS[94],
    OPTIONS[95],
];
const COMMANDS: &[CommandSpec] = &[
    CommandSpec::new(
        "list",
        "Save Editor: list",
        READ_OPTIONS,
        Safety::read(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .hides_cli_flags(&["out"])
    .guide("save-games"),
    CommandSpec::new(
        "inspect",
        "Save Editor: inspect",
        READ_OPTIONS,
        Safety::read(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .hides_cli_flags(&["out"])
    .guide("save-games"),
    CommandSpec::new(
        "refresh",
        "Save Editor: refresh",
        READ_OPTIONS,
        Safety::read(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .hides_cli_flags(&["out"])
    .guide("save-games"),
    CommandSpec::new(
        "rename",
        "Save Editor: rename",
        OPTIONS,
        Safety::mutate(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .guide("save-games")
    .gated_because("changes save files, their backups, drafts, or shared Save Editor settings"),
    CommandSpec::new(
        "import",
        "Save Editor: import",
        OPTIONS,
        Safety::mutate(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .guide("save-games")
    .gated_because("changes save files, their backups, drafts, or shared Save Editor settings"),
    CommandSpec::new(
        "delete",
        "Save Editor: delete",
        OPTIONS,
        Safety::mutate(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .guide("save-games")
    .gated_because("changes save files, their backups, drafts, or shared Save Editor settings"),
    CommandSpec::new(
        "overview",
        "Save Editor: overview",
        READ_OPTIONS,
        Safety::read(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .hides_cli_flags(&["out"])
    .guide("save-games"),
    CommandSpec::new(
        "statistics",
        "Save Editor: statistics",
        READ_OPTIONS,
        Safety::read(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .hides_cli_flags(&["out"])
    .guide("save-games"),
    CommandSpec::new(
        "report",
        "Save Editor: report",
        OPTIONS,
        Safety::write_truncating(&["out"]),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .guide("save-games"),
    CommandSpec::new(
        "about",
        "Save Editor: about",
        READ_OPTIONS,
        Safety::read(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .hides_cli_flags(&["out"])
    .guide("save-games"),
    CommandSpec::new(
        "licenses",
        "Save Editor: licenses",
        READ_OPTIONS,
        Safety::read(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .hides_cli_flags(&["out"])
    .guide("save-games"),
    CommandSpec::new(
        "validate",
        "Save Editor: validate",
        READ_OPTIONS,
        Safety::read(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .hides_cli_flags(&["out"])
    .guide("save-games"),
    CommandSpec::new(
        "profiles list",
        "Save Editor: profiles list",
        READ_OPTIONS,
        Safety::read(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .hides_cli_flags(&["out"])
    .guide("save-games"),
    CommandSpec::new(
        "profiles show",
        "Save Editor: profiles show",
        READ_OPTIONS,
        Safety::read(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .hides_cli_flags(&["out"])
    .guide("save-games"),
    CommandSpec::new(
        "profile assign",
        "Save Editor: profile assign",
        OPTIONS,
        Safety::mutate(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .guide("save-games")
    .gated_because("changes save files, their backups, drafts, or shared Save Editor settings"),
    CommandSpec::new(
        "profile detach",
        "Save Editor: profile detach",
        OPTIONS,
        Safety::mutate(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .guide("save-games")
    .gated_because("changes save files, their backups, drafts, or shared Save Editor settings"),
    CommandSpec::new(
        "characters list",
        "Save Editor: characters list",
        READ_OPTIONS,
        Safety::read(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .hides_cli_flags(&["out"])
    .guide("save-games"),
    CommandSpec::new(
        "characters show",
        "Save Editor: characters show",
        READ_OPTIONS,
        Safety::read(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .hides_cli_flags(&["out"])
    .guide("save-games"),
    CommandSpec::new(
        "attributes list",
        "Save Editor: attributes list",
        READ_OPTIONS,
        Safety::read(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .hides_cli_flags(&["out"])
    .guide("save-games"),
    CommandSpec::new(
        "attributes show",
        "Save Editor: attributes show",
        READ_OPTIONS,
        Safety::read(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .hides_cli_flags(&["out"])
    .guide("save-games"),
    CommandSpec::new(
        "attributes set",
        "Save Editor: attributes set",
        OPTIONS,
        Safety::mutate(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .guide("save-games")
    .gated_because("changes save files, their backups, drafts, or shared Save Editor settings"),
    CommandSpec::new(
        "skills list",
        "Save Editor: skills list",
        READ_OPTIONS,
        Safety::read(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .hides_cli_flags(&["out"])
    .guide("save-games"),
    CommandSpec::new(
        "skills show",
        "Save Editor: skills show",
        READ_OPTIONS,
        Safety::read(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .hides_cli_flags(&["out"])
    .guide("save-games"),
    CommandSpec::new(
        "skills set",
        "Save Editor: skills set",
        OPTIONS,
        Safety::mutate(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .guide("save-games")
    .gated_because("changes save files, their backups, drafts, or shared Save Editor settings"),
    CommandSpec::new(
        "inventory list",
        "Save Editor: inventory list",
        READ_OPTIONS,
        Safety::read(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .hides_cli_flags(&["out"])
    .guide("save-games"),
    CommandSpec::new(
        "inventory show",
        "Save Editor: inventory show",
        READ_OPTIONS,
        Safety::read(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .hides_cli_flags(&["out"])
    .guide("save-games"),
    CommandSpec::new(
        "inventory set-count",
        "Save Editor: inventory set-count",
        OPTIONS,
        Safety::mutate(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .guide("save-games")
    .gated_because("changes save files, their backups, drafts, or shared Save Editor settings"),
    CommandSpec::new(
        "inventory add",
        "Save Editor: inventory add",
        OPTIONS,
        Safety::mutate(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .guide("save-games")
    .gated_because("changes save files, their backups, drafts, or shared Save Editor settings"),
    CommandSpec::new(
        "inventory remove",
        "Save Editor: inventory remove",
        OPTIONS,
        Safety::mutate(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .guide("save-games")
    .gated_because("changes save files, their backups, drafts, or shared Save Editor settings"),
    CommandSpec::new(
        "inventory reset",
        "Save Editor: inventory reset",
        OPTIONS,
        Safety::mutate(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .guide("save-games")
    .gated_because("changes save files, their backups, drafts, or shared Save Editor settings"),
    CommandSpec::new(
        "inventory check-slots",
        "Save Editor: inventory check-slots",
        READ_OPTIONS,
        Safety::read(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .hides_cli_flags(&["out"])
    .guide("save-games"),
    CommandSpec::new(
        "inventory repair-slots",
        "Save Editor: inventory repair-slots",
        OPTIONS,
        Safety::mutate(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .guide("save-games")
    .gated_because("changes save files, their backups, drafts, or shared Save Editor settings"),
    CommandSpec::new(
        "position show",
        "Save Editor: position show",
        READ_OPTIONS,
        Safety::read(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .hides_cli_flags(&["out"])
    .guide("save-games"),
    CommandSpec::new(
        "position set",
        "Save Editor: position set",
        OPTIONS,
        Safety::mutate(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .guide("save-games")
    .gated_because("changes save files, their backups, drafts, or shared Save Editor settings"),
    CommandSpec::new(
        "position reset-to-spawn",
        "Save Editor: position reset-to-spawn",
        OPTIONS,
        Safety::mutate(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .guide("save-games")
    .gated_because("changes save files, their backups, drafts, or shared Save Editor settings"),
    CommandSpec::new(
        "position pin-status",
        "Save Editor: position pin-status",
        READ_OPTIONS,
        Safety::read(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .hides_cli_flags(&["out"])
    .guide("save-games"),
    CommandSpec::new(
        "position resume-routine",
        "Save Editor: position resume-routine",
        OPTIONS,
        Safety::mutate(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .guide("save-games")
    .gated_because("changes save files, their backups, drafts, or shared Save Editor settings"),
    CommandSpec::new(
        "position undo",
        "Save Editor: position undo",
        OPTIONS,
        Safety::mutate(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .guide("save-games")
    .gated_because("changes save files, their backups, drafts, or shared Save Editor settings"),
    CommandSpec::new(
        "time show",
        "Save Editor: time show",
        READ_OPTIONS,
        Safety::read(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .hides_cli_flags(&["out"])
    .guide("save-games"),
    CommandSpec::new(
        "time set",
        "Save Editor: time set",
        OPTIONS,
        Safety::mutate(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .guide("save-games")
    .gated_because("changes save files, their backups, drafts, or shared Save Editor settings"),
    CommandSpec::new(
        "difficulty show",
        "Save Editor: difficulty show",
        READ_OPTIONS,
        Safety::read(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .hides_cli_flags(&["out"])
    .guide("save-games"),
    CommandSpec::new(
        "difficulty set",
        "Save Editor: difficulty set",
        OPTIONS,
        Safety::mutate(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .guide("save-games")
    .gated_because("changes save files, their backups, drafts, or shared Save Editor settings"),
    CommandSpec::new(
        "quests list",
        "Save Editor: quests list",
        READ_OPTIONS,
        Safety::read(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .hides_cli_flags(&["out"])
    .guide("save-games"),
    CommandSpec::new(
        "quests show",
        "Save Editor: quests show",
        READ_OPTIONS,
        Safety::read(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .hides_cli_flags(&["out"])
    .guide("save-games"),
    CommandSpec::new(
        "quests set-state",
        "Save Editor: quests set-state",
        OPTIONS,
        Safety::mutate(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .guide("save-games")
    .gated_because("changes save files, their backups, drafts, or shared Save Editor settings"),
    CommandSpec::new(
        "tutorials list",
        "Save Editor: tutorials list",
        READ_OPTIONS,
        Safety::read(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .hides_cli_flags(&["out"])
    .guide("save-games"),
    CommandSpec::new(
        "tutorials show",
        "Save Editor: tutorials show",
        READ_OPTIONS,
        Safety::read(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .hides_cli_flags(&["out"])
    .guide("save-games"),
    CommandSpec::new(
        "tutorials set-state",
        "Save Editor: tutorials set-state",
        OPTIONS,
        Safety::mutate(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .guide("save-games")
    .gated_because("changes save files, their backups, drafts, or shared Save Editor settings"),
    CommandSpec::new(
        "story list",
        "Save Editor: story list",
        READ_OPTIONS,
        Safety::read(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .hides_cli_flags(&["out"])
    .guide("save-games"),
    CommandSpec::new(
        "story show",
        "Save Editor: story show",
        READ_OPTIONS,
        Safety::read(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .hides_cli_flags(&["out"])
    .guide("save-games"),
    CommandSpec::new(
        "story set",
        "Save Editor: story set",
        OPTIONS,
        Safety::mutate(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .guide("save-games")
    .gated_because("changes save files, their backups, drafts, or shared Save Editor settings"),
    CommandSpec::new(
        "story unset",
        "Save Editor: story unset",
        OPTIONS,
        Safety::mutate(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .guide("save-games")
    .gated_because("changes save files, their backups, drafts, or shared Save Editor settings"),
    CommandSpec::new(
        "knowledge list",
        "Save Editor: knowledge list",
        READ_OPTIONS,
        Safety::read(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .hides_cli_flags(&["out"])
    .guide("save-games"),
    CommandSpec::new(
        "knowledge add",
        "Save Editor: knowledge add",
        OPTIONS,
        Safety::mutate(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .guide("save-games")
    .gated_because("changes save files, their backups, drafts, or shared Save Editor settings"),
    CommandSpec::new(
        "knowledge remove",
        "Save Editor: knowledge remove",
        OPTIONS,
        Safety::mutate(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .guide("save-games")
    .gated_because("changes save files, their backups, drafts, or shared Save Editor settings"),
    CommandSpec::new(
        "knowledge create-character",
        "Save Editor: knowledge create-character",
        OPTIONS,
        Safety::mutate(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .guide("save-games")
    .gated_because("changes save files, their backups, drafts, or shared Save Editor settings"),
    CommandSpec::new(
        "events list",
        "Save Editor: events list",
        READ_OPTIONS,
        Safety::read(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .hides_cli_flags(&["out"])
    .guide("save-games"),
    CommandSpec::new(
        "events show",
        "Save Editor: events show",
        READ_OPTIONS,
        Safety::read(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .hides_cli_flags(&["out"])
    .guide("save-games"),
    CommandSpec::new(
        "events remove",
        "Save Editor: events remove",
        OPTIONS,
        Safety::mutate(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .guide("save-games")
    .gated_because("changes save files, their backups, drafts, or shared Save Editor settings"),
    CommandSpec::new(
        "events duplicate",
        "Save Editor: events duplicate",
        OPTIONS,
        Safety::mutate(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .guide("save-games")
    .gated_because("changes save files, their backups, drafts, or shared Save Editor settings"),
    CommandSpec::new(
        "factions list",
        "Save Editor: factions list",
        READ_OPTIONS,
        Safety::read(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .hides_cli_flags(&["out"])
    .guide("save-games"),
    CommandSpec::new(
        "factions show",
        "Save Editor: factions show",
        READ_OPTIONS,
        Safety::read(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .hides_cli_flags(&["out"])
    .guide("save-games"),
    CommandSpec::new(
        "factions forgive",
        "Save Editor: factions forgive",
        OPTIONS,
        Safety::mutate(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .guide("save-games")
    .gated_because("changes save files, their backups, drafts, or shared Save Editor settings"),
    CommandSpec::new(
        "locks list",
        "Save Editor: locks list",
        READ_OPTIONS,
        Safety::read(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .hides_cli_flags(&["out"])
    .guide("save-games"),
    CommandSpec::new(
        "locks show",
        "Save Editor: locks show",
        READ_OPTIONS,
        Safety::read(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .hides_cli_flags(&["out"])
    .guide("save-games"),
    CommandSpec::new(
        "locks unlock",
        "Save Editor: locks unlock",
        OPTIONS,
        Safety::mutate(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .guide("save-games")
    .gated_because("changes save files, their backups, drafts, or shared Save Editor settings"),
    CommandSpec::new(
        "locks lock",
        "Save Editor: locks lock",
        OPTIONS,
        Safety::mutate(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .guide("save-games")
    .gated_because("changes save files, their backups, drafts, or shared Save Editor settings"),
    CommandSpec::new(
        "backups list",
        "Save Editor: backups list",
        READ_OPTIONS,
        Safety::read(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .hides_cli_flags(&["out"])
    .guide("save-games"),
    CommandSpec::new(
        "backups rename",
        "Save Editor: backups rename",
        OPTIONS,
        Safety::mutate(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .guide("save-games")
    .gated_because("changes save files, their backups, drafts, or shared Save Editor settings"),
    CommandSpec::new(
        "backups restore",
        "Save Editor: backups restore",
        OPTIONS,
        Safety::mutate(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .guide("save-games")
    .gated_because("changes save files, their backups, drafts, or shared Save Editor settings"),
    CommandSpec::new(
        "backups delete",
        "Save Editor: backups delete",
        OPTIONS,
        Safety::mutate(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .guide("save-games")
    .gated_because("changes save files, their backups, drafts, or shared Save Editor settings"),
    CommandSpec::new(
        "recovery list",
        "Save Editor: recovery list",
        READ_OPTIONS,
        Safety::read(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .hides_cli_flags(&["out"])
    .guide("save-games"),
    CommandSpec::new(
        "recovery show",
        "Save Editor: recovery show",
        READ_OPTIONS,
        Safety::read(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .hides_cli_flags(&["out"])
    .guide("save-games"),
    CommandSpec::new(
        "recovery restore",
        "Save Editor: recovery restore",
        OPTIONS,
        Safety::mutate(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .guide("save-games")
    .gated_because("changes save files, their backups, drafts, or shared Save Editor settings"),
    CommandSpec::new(
        "recovery dismiss",
        "Save Editor: recovery dismiss",
        OPTIONS,
        Safety::mutate(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .guide("save-games")
    .gated_because("changes save files, their backups, drafts, or shared Save Editor settings"),
    CommandSpec::new(
        "recovery repair",
        "Save Editor: recovery repair",
        OPTIONS,
        Safety::mutate(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .guide("save-games")
    .gated_because("changes save files, their backups, drafts, or shared Save Editor settings"),
    CommandSpec::new(
        "library list",
        "Save Editor: library list",
        READ_OPTIONS,
        Safety::read(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .hides_cli_flags(&["out"])
    .guide("save-games"),
    CommandSpec::new(
        "library add",
        "Save Editor: library add",
        OPTIONS,
        Safety::mutate(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .guide("save-games")
    .gated_because("changes save files, their backups, drafts, or shared Save Editor settings"),
    CommandSpec::new(
        "library remove",
        "Save Editor: library remove",
        OPTIONS,
        Safety::mutate(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .guide("save-games")
    .gated_because("changes save files, their backups, drafts, or shared Save Editor settings"),
    CommandSpec::new(
        "library hide",
        "Save Editor: library hide",
        OPTIONS,
        Safety::mutate(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .guide("save-games")
    .gated_because("changes save files, their backups, drafts, or shared Save Editor settings"),
    CommandSpec::new(
        "library unhide",
        "Save Editor: library unhide",
        OPTIONS,
        Safety::mutate(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .guide("save-games")
    .gated_because("changes save files, their backups, drafts, or shared Save Editor settings"),
    CommandSpec::new(
        "settings show",
        "Save Editor: settings show",
        READ_OPTIONS,
        Safety::read(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .hides_cli_flags(&["out"])
    .guide("save-games"),
    CommandSpec::new(
        "settings get",
        "Save Editor: settings get",
        READ_OPTIONS,
        Safety::read(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .hides_cli_flags(&["out"])
    .guide("save-games"),
    CommandSpec::new(
        "settings set",
        "Save Editor: settings set",
        OPTIONS,
        Safety::mutate(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .guide("save-games")
    .gated_because("changes save files, their backups, drafts, or shared Save Editor settings"),
    CommandSpec::new(
        "settings reset",
        "Save Editor: settings reset",
        OPTIONS,
        Safety::mutate(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .guide("save-games")
    .gated_because("changes save files, their backups, drafts, or shared Save Editor settings"),
    CommandSpec::new(
        "localization status",
        "Save Editor: localization status",
        READ_OPTIONS,
        Safety::read(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .hides_cli_flags(&["out"])
    .guide("save-games"),
    CommandSpec::new(
        "localization prepare",
        "Save Editor: localization prepare",
        OPTIONS,
        Safety::mutate(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .guide("save-games")
    .gated_because("changes save files, their backups, drafts, or shared Save Editor settings"),
    CommandSpec::new(
        "localization find",
        "Save Editor: localization find",
        READ_OPTIONS,
        Safety::read(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .hides_cli_flags(&["out"])
    .guide("save-games"),
    CommandSpec::new(
        "assets prepare",
        "Save Editor: assets prepare",
        OPTIONS,
        Safety::mutate(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .guide("save-games")
    .gated_because("changes save files, their backups, drafts, or shared Save Editor settings"),
    CommandSpec::new(
        "assets status",
        "Save Editor: assets status",
        READ_OPTIONS,
        Safety::read(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .hides_cli_flags(&["out"])
    .guide("save-games"),
    CommandSpec::new(
        "assets list",
        "Save Editor: assets list",
        READ_OPTIONS,
        Safety::read(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .hides_cli_flags(&["out"])
    .guide("save-games"),
    CommandSpec::new(
        "assets export",
        "Save Editor: assets export",
        OPTIONS,
        Safety::write_truncating(&["out"]),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .guide("save-games"),
    CommandSpec::new(
        "assets open",
        "Save Editor: assets open",
        OPTIONS,
        Safety::mutate(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .guide("save-games")
    .gated_because("changes save files, their backups, drafts, or shared Save Editor settings"),
    CommandSpec::new(
        "assets release",
        "Save Editor: assets release",
        OPTIONS,
        Safety::mutate(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .guide("save-games")
    .gated_because("changes save files, their backups, drafts, or shared Save Editor settings"),
    CommandSpec::new(
        "data search",
        "Save Editor: data search",
        READ_OPTIONS,
        Safety::read(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .hides_cli_flags(&["out"])
    .guide("save-games"),
    CommandSpec::new(
        "data show",
        "Save Editor: data show",
        READ_OPTIONS,
        Safety::read(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .hides_cli_flags(&["out"])
    .guide("save-games"),
    CommandSpec::new(
        "data set",
        "Save Editor: data set",
        OPTIONS,
        Safety::mutate(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .guide("save-games")
    .gated_because("changes save files, their backups, drafts, or shared Save Editor settings"),
    CommandSpec::new(
        "data set-add",
        "Save Editor: data set-add",
        OPTIONS,
        Safety::mutate(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .guide("save-games")
    .gated_because("changes save files, their backups, drafts, or shared Save Editor settings"),
    CommandSpec::new(
        "data set-remove",
        "Save Editor: data set-remove",
        OPTIONS,
        Safety::mutate(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .guide("save-games")
    .gated_because("changes save files, their backups, drafts, or shared Save Editor settings"),
    CommandSpec::new(
        "data array-remove",
        "Save Editor: data array-remove",
        OPTIONS,
        Safety::mutate(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .guide("save-games")
    .gated_because("changes save files, their backups, drafts, or shared Save Editor settings"),
    CommandSpec::new(
        "data array-duplicate",
        "Save Editor: data array-duplicate",
        OPTIONS,
        Safety::mutate(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .guide("save-games")
    .gated_because("changes save files, their backups, drafts, or shared Save Editor settings"),
    CommandSpec::new(
        "codec status",
        "Save Editor: codec status",
        READ_OPTIONS,
        Safety::read(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .hides_cli_flags(&["out"])
    .guide("save-games"),
    CommandSpec::new(
        "core exec",
        "Save Editor: core exec",
        OPTIONS,
        Safety::mutate(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .guide("save-games")
    .gated_because("changes save files, their backups, drafts, or shared Save Editor settings"),
    CommandSpec::new(
        "core capabilities",
        "Save Editor: core capabilities",
        READ_OPTIONS,
        Safety::read(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .hides_cli_flags(&["out"])
    .guide("save-games"),
    CommandSpec::new(
        "draft create",
        "Save Editor: draft create",
        OPTIONS,
        Safety::mutate(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .guide("save-games")
    .gated_because("changes save files, their backups, drafts, or shared Save Editor settings"),
    CommandSpec::new(
        "draft show",
        "Save Editor: draft show",
        READ_OPTIONS,
        Safety::read(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .hides_cli_flags(&["out"])
    .guide("save-games"),
    CommandSpec::new(
        "draft stage",
        "Save Editor: draft stage",
        OPTIONS,
        Safety::mutate(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .guide("save-games")
    .gated_because("changes save files, their backups, drafts, or shared Save Editor settings"),
    CommandSpec::new(
        "draft remove",
        "Save Editor: draft remove",
        OPTIONS,
        Safety::mutate(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .guide("save-games")
    .gated_because("changes save files, their backups, drafts, or shared Save Editor settings"),
    CommandSpec::new(
        "draft reset",
        "Save Editor: draft reset",
        OPTIONS,
        Safety::mutate(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .guide("save-games")
    .gated_because("changes save files, their backups, drafts, or shared Save Editor settings"),
    CommandSpec::new(
        "draft validate",
        "Save Editor: draft validate",
        READ_OPTIONS,
        Safety::read(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .hides_cli_flags(&["out"])
    .guide("save-games"),
    CommandSpec::new(
        "draft apply",
        "Save Editor: draft apply",
        OPTIONS,
        Safety::mutate(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .guide("save-games")
    .gated_because("changes save files, their backups, drafts, or shared Save Editor settings"),
    CommandSpec::new(
        "catalog list",
        "Save Editor: catalog list",
        READ_OPTIONS,
        Safety::read(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .hides_cli_flags(&["out"])
    .guide("save-games"),
    CommandSpec::new(
        "catalog show",
        "Save Editor: catalog show",
        READ_OPTIONS,
        Safety::read(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .hides_cli_flags(&["out"])
    .guide("save-games"),
    CommandSpec::new(
        "catalog search",
        "Save Editor: catalog search",
        READ_OPTIONS,
        Safety::read(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .hides_cli_flags(&["out"])
    .guide("save-games"),
    CommandSpec::new(
        "items list",
        "Save Editor: items list",
        READ_OPTIONS,
        Safety::read(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .hides_cli_flags(&["out"])
    .guide("save-games"),
    CommandSpec::new(
        "items show",
        "Save Editor: items show",
        READ_OPTIONS,
        Safety::read(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .hides_cli_flags(&["out"])
    .guide("save-games"),
    CommandSpec::new(
        "items search",
        "Save Editor: items search",
        READ_OPTIONS,
        Safety::read(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .hides_cli_flags(&["out"])
    .guide("save-games"),
    CommandSpec::new(
        "locations list",
        "Save Editor: locations list",
        READ_OPTIONS,
        Safety::read(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .hides_cli_flags(&["out"])
    .guide("save-games"),
    CommandSpec::new(
        "locations show",
        "Save Editor: locations show",
        READ_OPTIONS,
        Safety::read(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .hides_cli_flags(&["out"])
    .guide("save-games"),
    CommandSpec::new(
        "locations search",
        "Save Editor: locations search",
        READ_OPTIONS,
        Safety::read(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .hides_cli_flags(&["out"])
    .guide("save-games"),
    CommandSpec::new(
        "screenshot export",
        "Save Editor: screenshot export",
        OPTIONS,
        Safety::write_truncating(&["out"]),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .guide("save-games"),
    CommandSpec::new(
        "updates check",
        "Save Editor: updates check",
        READ_OPTIONS,
        Safety::read(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .hides_cli_flags(&["out"])
    .guide("save-games"),
    CommandSpec::new(
        "updates open-release",
        "Save Editor: updates open-release",
        OPTIONS,
        Safety::mutate(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .guide("save-games")
    .gated_because("changes save files, their backups, drafts, or shared Save Editor settings"),
    CommandSpec::new(
        "updates install",
        "Save Editor: updates install",
        OPTIONS,
        Safety::mutate(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .guide("save-games")
    .gated_because("changes save files, their backups, drafts, or shared Save Editor settings"),
    CommandSpec::new(
        "npc relationship show",
        "Save Editor: npc relationship show",
        READ_OPTIONS,
        Safety::read(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .hides_cli_flags(&["out"])
    .guide("save-games"),
    CommandSpec::new(
        "npc relationship set",
        "Save Editor: npc relationship set",
        OPTIONS,
        Safety::mutate(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .guide("save-games")
    .gated_because("changes save files, their backups, drafts, or shared Save Editor settings"),
    CommandSpec::new(
        "glossary segment unlock",
        "Save Editor: glossary segment unlock",
        OPTIONS,
        Safety::mutate(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .guide("save-games")
    .gated_because("changes save files, their backups, drafts, or shared Save Editor settings"),
    CommandSpec::new(
        "glossary segment lock",
        "Save Editor: glossary segment lock",
        OPTIONS,
        Safety::mutate(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .guide("save-games")
    .gated_because("changes save files, their backups, drafts, or shared Save Editor settings"),
    CommandSpec::new(
        "traders stock set",
        "Save Editor: traders stock set",
        OPTIONS,
        Safety::mutate(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .guide("save-games")
    .gated_because("changes save files, their backups, drafts, or shared Save Editor settings"),
    CommandSpec::new(
        "traders stock add",
        "Save Editor: traders stock add",
        OPTIONS,
        Safety::mutate(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .guide("save-games")
    .gated_because("changes save files, their backups, drafts, or shared Save Editor settings"),
    CommandSpec::new(
        "traders stock remove",
        "Save Editor: traders stock remove",
        OPTIONS,
        Safety::mutate(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .guide("save-games")
    .gated_because("changes save files, their backups, drafts, or shared Save Editor settings"),
    CommandSpec::new(
        "traders timing show",
        "Save Editor: traders timing show",
        READ_OPTIONS,
        Safety::read(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .hides_cli_flags(&["out"])
    .guide("save-games"),
    CommandSpec::new(
        "traders timing set",
        "Save Editor: traders timing set",
        OPTIONS,
        Safety::mutate(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .guide("save-games")
    .gated_because("changes save files, their backups, drafts, or shared Save Editor settings"),
    CommandSpec::new(
        "traders timing make-due",
        "Save Editor: traders timing make-due",
        OPTIONS,
        Safety::mutate(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .guide("save-games")
    .gated_because("changes save files, their backups, drafts, or shared Save Editor settings"),
    CommandSpec::new(
        "npc list",
        "Save Editor: npc list",
        READ_OPTIONS,
        Safety::read(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .hides_cli_flags(&["out"])
    .guide("save-games"),
    CommandSpec::new(
        "npc show",
        "Save Editor: npc show",
        READ_OPTIONS,
        Safety::read(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .hides_cli_flags(&["out"])
    .guide("save-games"),
    CommandSpec::new(
        "npc revive",
        "Save Editor: npc revive",
        OPTIONS,
        Safety::mutate(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .guide("save-games")
    .gated_because("changes save files, their backups, drafts, or shared Save Editor settings"),
    CommandSpec::new(
        "glossary list",
        "Save Editor: glossary list",
        READ_OPTIONS,
        Safety::read(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .hides_cli_flags(&["out"])
    .guide("save-games"),
    CommandSpec::new(
        "glossary show",
        "Save Editor: glossary show",
        READ_OPTIONS,
        Safety::read(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .hides_cli_flags(&["out"])
    .guide("save-games"),
    CommandSpec::new(
        "glossary set-state",
        "Save Editor: glossary set-state",
        OPTIONS,
        Safety::mutate(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .guide("save-games")
    .gated_because("changes save files, their backups, drafts, or shared Save Editor settings"),
    CommandSpec::new(
        "traders list",
        "Save Editor: traders list",
        READ_OPTIONS,
        Safety::read(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .hides_cli_flags(&["out"])
    .guide("save-games"),
    CommandSpec::new(
        "traders show",
        "Save Editor: traders show",
        READ_OPTIONS,
        Safety::read(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .hides_cli_flags(&["out"])
    .guide("save-games"),
];
pub const SAVE: GroupSpec = GroupSpec {
    tool: "gore_save",
    title: "Save Editor",
    cli: "save",
    shape: GroupShape::Nested,
    summary: "Inspect and edit saves, profiles, world state, inventory and raw data; manage backups and drafts.",
    commands: COMMANDS,
};
