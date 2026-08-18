//! Rendering the command table into MCP tool definitions.
//!
//! # Why the arguments are not typed in the JSON Schema
//!
//! A namespace tool covers up to twenty subcommands with different arguments. JSON Schema can
//! express that with `oneOf` keyed on `subcommand`, but in practice many MCP clients neither
//! validate nor read conditional subschemas when constructing a call, so the model would get the
//! constraint only when a client happens to enforce it.
//!
//! What every client does show the model is the tool *description*. So the per-subcommand argument
//! reference is generated into the description as text, and `args` stays a free-form object that
//! [`crate::argv`] validates precisely. A wrong call then comes back as a tool error naming the
//! offending argument, which the specification explicitly designs for: input validation failures
//! belong in the result so the model can self-correct.
//!
//! The cost of that choice is description length. It is bounded by writing one line per argument
//! and never repeating the guide.

use serde_json::{json, Value};

use crate::spec::{ArgKind, Class, CommandSpec, GroupSpec, Safety, GROUPS};

/// Every tool definition, in registry order.
pub fn tool_definitions() -> Vec<Value> {
    GROUPS.iter().map(tool_json).collect()
}

pub fn tool_json(group: &GroupSpec) -> Value {
    json!({
        "name": group.tool,
        "title": group.title,
        "description": description(group),
        "inputSchema": input_schema(group),
        "annotations": annotations(group),
    })
}

fn input_schema(group: &GroupSpec) -> Value {
    json!({
        "type": "object",
        "properties": {
            "subcommand": {
                "type": "string",
                "enum": group.subcommands(),
                "description": "Which subcommand to run. See the tool description for each one's arguments.",
            },
            "args": {
                "type": "object",
                "description": "Arguments for the chosen subcommand, keyed by the names listed in \
                                the tool description. Omit it for subcommands that take none.",
                "additionalProperties": true,
            },
            // Only reachable because it is declared: the schema closes itself to anything else, so
            // an undeclared field would come back as an error rather than as consent.
            crate::consent::APPROVAL_FIELD: {
                "type": "string",
                "description": "The user's own words approving this exact call, quoted verbatim. \
                                Set it only after a call was refused for want of consent, you put \
                                the command line in front of the user, and they agreed. Never set \
                                it on your own initiative, and never paraphrase agreement the user \
                                did not give: this server cannot check the claim, so the result \
                                records that the command ran on your assertion rather than on a \
                                confirmation anyone saw.",
            },
        },
        "required": ["subcommand"],
        "additionalProperties": false,
    })
}

/// MCP tool annotations.
///
/// These are per-tool, but a namespace tool bundles subcommands of different severity, so the
/// annotation has to describe the worst of them. `gore_as` reads caches *and* launches the game;
/// advertising it as read-only because most of its leaves are would undermine the approval prompt a
/// client builds from these hints. The precise per-subcommand truth is in the description, and the
/// safety gate enforces it whatever a client does with the annotation.
fn annotations(group: &GroupSpec) -> Value {
    let worst = group.worst_case();
    let read_only = worst == Class::Read;

    let mut annotations = json!({
        "title": group.title,
        "readOnlyHint": read_only,
        // Every one of these commands acts on a local game installation and local files. Nothing
        // reaches an open-ended external world.
        "openWorldHint": false,
    });

    // `destructiveHint` and `idempotentHint` are defined as meaningful only when the tool is not
    // read-only, so they are omitted rather than set to a value a client should ignore.
    if !read_only {
        annotations["destructiveHint"] = json!(worst >= Class::Mutate);
        annotations["idempotentHint"] = json!(false);
    }

    annotations
}

fn description(group: &GroupSpec) -> String {
    let mut text = String::with_capacity(2048);
    text.push_str(group.summary);

    match group.shape {
        crate::spec::GroupShape::Nested => {
            text.push_str(&format!("\n\nRuns `gore {} <subcommand>`.", group.cli));
        }
        crate::spec::GroupShape::Flat => {
            text.push_str("\n\nRuns `gore <subcommand>` — these are separate top-level commands, \
                           grouped here because they belong to one workflow.");
        }
    }

    if let Some(page) = primary_guide_page(group) {
        text.push_str(&format!(
            " Read `gore://guide/{page}` (or call gore_guide) before using this for the first time."
        ));
    }

    text.push_str("\n\nSUBCOMMANDS  (* marks a required argument)\n");
    for command in group.commands {
        text.push_str(&describe_command(command));
    }

    text
}

fn describe_command(command: &CommandSpec) -> String {
    let mut text = format!("\n{} — {}\n", command.sub, command.summary);
    text.push_str(&format!("    [{}]\n", safety_note(&command.safety)));

    if command.args.is_empty() {
        text.push_str("    (no arguments)\n");
        return text;
    }

    for arg in command.args {
        let marker = if arg.required { "*" } else { " " };
        let kind = match arg.kind {
            ArgKind::Enum(values) => format!("enum: {}", values.join("|")),
            ArgKind::Int { min: Some(min), max: Some(max) } => format!("integer {min}..={max}"),
            other => other.label().to_string(),
        };
        text.push_str(&format!("  {marker} {} <{kind}> — {}", arg.name, arg.help));
        if let Some(default) = arg.default_hint {
            text.push_str(&format!(" Default: {default}."));
        }
        text.push('\n');
    }

    text
}

fn safety_note(safety: &Safety) -> String {
    match safety.in_place_without {
        // The conditional case is worth spelling out: it is the difference between producing a new
        // file and overwriting one of the game's own, and it turns on a single argument.
        Some(escape) => format!(
            "{}, but overwrites its input in place when `{escape}` is omitted, which needs \
             --allow-write",
            Class::Write.label()
        ),
        None => safety.base.label().to_string(),
    }
}

/// The guide page most of a group's commands point at, if they agree on one.
fn primary_guide_page(group: &GroupSpec) -> Option<&'static str> {
    let mut pages = group.commands.iter().filter_map(|command| command.guide);
    let first = pages.next()?;
    pages.all(|page| page == first).then_some(first)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec;

    fn tool(name: &str) -> Value {
        tool_json(spec::group(name).expect("group exists"))
    }

    #[test]
    fn every_group_produces_a_definition_with_the_required_fields() {
        let tools = tool_definitions();
        assert_eq!(tools.len(), spec::GROUPS.len());
        for definition in &tools {
            assert!(definition["name"].as_str().is_some());
            assert!(definition["description"].as_str().is_some_and(|d| !d.is_empty()));
            assert_eq!(definition["inputSchema"]["type"], "object");
        }
    }

    #[test]
    fn the_subcommand_enum_lists_exactly_the_groups_leaves() {
        let definition = tool("gore_config");
        let listed: Vec<String> = definition["inputSchema"]["properties"]["subcommand"]["enum"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_str().unwrap().to_string())
            .collect();

        assert_eq!(listed, vec!["set", "get", "unset", "list", "path", "detect"]);
    }

    #[test]
    fn subcommand_is_the_only_required_property_and_no_others_are_accepted() {
        let schema = tool("gore_config")["inputSchema"].clone();
        assert_eq!(schema["required"], json!(["subcommand"]));
        assert_eq!(schema["additionalProperties"], json!(false));
        // `args` itself must stay open: the argv builder, not the schema, validates its contents.
        assert_eq!(schema["properties"]["args"]["additionalProperties"], json!(true));
    }

    #[test]
    fn the_schema_offers_an_approval_field_and_binds_how_it_may_be_used() {
        // The field is the only route past the gate in a client whose dialog reaches nobody, so it
        // has to be reachable — `additionalProperties: false` would otherwise reject it — and its
        // description has to make plain that inventing one is not a use of it.
        let schema = tool("gore_loc")["inputSchema"].clone();
        let approval = &schema["properties"]["user_approved"];

        assert_eq!(approval["type"], "string", "{schema}");
        let description = approval["description"].as_str().expect("a description");
        assert!(description.contains("verbatim"), "{description}");
        assert!(description.contains("Never"), "{description}");
        assert_eq!(schema["required"], json!(["subcommand"]), "approval is never required");
    }

    #[test]
    fn a_read_only_group_omits_the_hints_that_only_apply_to_writers() {
        // gore_config mixes reads and writes, so build a read-only group to check the branch.
        let mut group = *spec::group("gore_config").unwrap();
        group.commands = &group.commands[1..2]; // `get`
        let annotations = tool_json(&group)["annotations"].clone();

        assert_eq!(annotations["readOnlyHint"], json!(true));
        assert!(annotations.get("destructiveHint").is_none());
        assert!(annotations.get("idempotentHint").is_none());
    }

    #[test]
    fn a_group_containing_a_mutating_command_is_annotated_as_destructive() {
        // gore_project bundles three harmless generators with `deploy-shared`, which writes into
        // the game installation. The annotation must reflect the worst of them.
        let annotations = tool("gore_project")["annotations"].clone();
        assert_eq!(annotations["readOnlyHint"], json!(false));
        assert_eq!(annotations["destructiveHint"], json!(true));
    }

    #[test]
    fn the_description_documents_every_subcommand_and_marks_required_arguments() {
        let definition = tool("gore_catalog");
        let description = definition["description"].as_str().unwrap();

        for command in spec::group("gore_catalog").unwrap().commands {
            assert!(
                description.contains(command.sub),
                "{} is missing from the description",
                command.sub
            );
        }
        assert!(description.contains("* kind <enum: item|npc|knowledge>"));
        assert!(description.contains("  script_cache <path>"), "optional args are unmarked");
    }

    #[test]
    fn a_command_without_arguments_says_so_rather_than_showing_nothing() {
        let description = tool("gore_config")["description"].as_str().unwrap().to_string();
        assert!(description.contains("(no arguments)"));
    }

    #[test]
    fn a_flat_group_explains_that_its_subcommands_are_top_level_commands() {
        let flat = tool("gore_catalog")["description"].as_str().unwrap().to_string();
        assert!(flat.contains("Runs `gore <subcommand>`"));

        let nested = tool("gore_config")["description"].as_str().unwrap().to_string();
        assert!(nested.contains("Runs `gore config <subcommand>`"));
    }

    #[test]
    fn a_group_whose_commands_share_a_guide_page_points_at_it() {
        let catalog = tool("gore_catalog")["description"].as_str().unwrap().to_string();
        assert!(catalog.contains("gore://guide/catalogs-and-models"));

        // A command that names no page does not veto the pointer — `gore config path` has none,
        // and the rest agree on `getting-started`.
        let config = tool("gore_config")["description"].as_str().unwrap().to_string();
        assert!(config.contains("gore://guide/getting-started"), "{config}");
    }

    #[test]
    fn a_group_whose_commands_disagree_about_the_guide_claims_no_page() {
        // Pointing at one page when half the subcommands are documented elsewhere would send the
        // model to the wrong place more often than not, so nothing is claimed.
        const MIXED: &[CommandSpec] = &[
            CommandSpec::new("a", "", &[], Safety::read(), 1).guide("items"),
            CommandSpec::new("b", "", &[], Safety::read(), 1).guide("audio"),
        ];
        let group = GroupSpec {
            tool: "gore_mixed",
            title: "mixed",
            cli: "",
            summary: "",
            shape: crate::spec::GroupShape::Flat,
            commands: MIXED,
        };
        assert_eq!(primary_guide_page(&group), None);
        assert!(!description(&group).contains("gore://guide/"));
    }

    #[test]
    fn a_conditionally_in_place_command_says_which_argument_makes_it_safe() {
        let safety = Safety::write_or_in_place(&["out"]);
        let note = safety_note(&safety);
        assert!(note.contains("`out`"));
        assert!(note.contains("--allow-write"));
    }

    #[test]
    fn manager_import_is_a_protected_state_write_not_an_install_mutation() {
        let command = spec::group("gore_mgr").unwrap().command("import").unwrap();
        let note = safety_note(&command.safety);

        assert_eq!(note, "WRITES MANAGER STATE — needs --allow-write");
        assert!(!note.contains("INSTALL"), "{note}");

        let description = tool("gore_mgr")["description"].as_str().unwrap().to_string();
        let import = description
            .split_once("\nimport —")
            .and_then(|(_, tail)| tail.split_once("\nlist —").map(|(section, _)| section))
            .expect("rendered import section");
        assert!(import.contains(&format!("[{note}]")), "{import}");
    }

    #[test]
    fn authoritative_manager_reads_disclose_ungated_reconciliation() {
        let manager = spec::group("gore_mgr").unwrap();
        for sub in ["list", "analyze", "status"] {
            let command = manager.command(sub).unwrap();
            assert_eq!(safety_note(&command.safety), "may reconcile Manager state", "{sub}");
            assert!(!command.safety.requirements(&serde_json::Map::new()).write, "{sub}");
        }
    }

    #[test]
    fn reversible_manager_edits_are_accurately_labeled_but_ungated() {
        let manager = spec::group("gore_mgr").unwrap();
        for sub in ["enable", "disable", "order"] {
            let command = manager.command(sub).unwrap();
            assert_eq!(safety_note(&command.safety), "updates Manager loadout", "{sub}");
            assert!(!command.safety.requirements(&serde_json::Map::new()).write, "{sub}");
        }
    }
}
