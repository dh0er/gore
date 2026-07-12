//! Declarative, fail-closed runtime registration for authored conversation topics.
//!
//! The generated UE4SS Lua is deliberately self-contained. It does not request conversations or
//! select topics; it only adds an already-compiled AngelScript topic at the natural
//! `ClientShowConversationUI` boundary after proving an exact participant and an exact vanilla
//! sentinel in that conversation's own topic set. Two later read-only hooks prove that each
//! visible object reaches the choice array and the rendered widget array; state-dependent topics
//! may opt into a separately logged clean-hidden state.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

const MAX_DIALOG_TOPICS: usize = 64;
const RUNTIME_TEMPLATE: &str = include_str!("dialog_runtime.lua");
const MOD_NAME_MARKER: &str = "__GORE_DIALOG_MOD_NAME__";
const REGISTRATIONS_MARKER: &str = "__GORE_DIALOG_REGISTRATIONS__";

/// One authored topic to register into a matching live conversation.
///
/// `topic_class` and `sentinel_class` are exact reflected UClass paths such as
/// `/Script/Angelscript.ChoiceMyTopic`. `participant_name` is compared case-insensitively with the
/// result of `ConversationGroup.GetParticipantName` and must use its stable identifier form.
/// Strict registration proves delivery to the rendered root-topic list. Conditional registration
/// can instead report that the engine cleanly hid a topic in the current state. Neither mode makes
/// the authored topic's selection behavior or automatic knowledge/`ActedTopics` persistence
/// save-neutral.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DialogTopicSpec {
    /// Stable, human-readable identifier used only in diagnostic log records.
    pub id: String,
    /// Exact stable participant identifier, for example `om_stt_example_123`.
    pub participant_name: String,
    /// Exact authored AngelScript topic UClass path.
    pub topic_class: String,
    /// Exact vanilla topic UClass path proving that the live topic set belongs to the target.
    pub sentinel_class: String,
    /// Allow this topic to be absent from the current visible-topic array after registration.
    ///
    /// Use this for state-dependent AngelScript topics whose `IsVisible_Implementation` is
    /// intentionally false on some conversation openings. A topic that is present is still held
    /// to the same exact object/class identity proof; this only turns a clean zero-match into a
    /// diagnostic `HIDDEN` result instead of a failure.
    #[serde(default, skip_serializing_if = "is_false")]
    pub allow_hidden: bool,
}

fn is_false(value: &bool) -> bool {
    !*value
}

/// Validate and render the self-contained UE4SS runtime for `topics`.
pub(crate) fn render_dialog_runtime(
    mod_name: &str,
    topics: &[DialogTopicSpec],
) -> Result<String, String> {
    if topics.is_empty() {
        return Err("dialog topic list must not be empty".into());
    }
    if topics.len() > MAX_DIALOG_TOPICS {
        return Err(format!(
            "dialog topic list has {} entries; maximum is {MAX_DIALOG_TOPICS}",
            topics.len()
        ));
    }

    let mut ids = BTreeSet::new();
    let mut registrations = BTreeSet::new();
    let mut topic_classes = BTreeSet::new();
    let mut sentinel_classes = BTreeSet::new();
    let mut rows = String::new();
    for (index, topic) in topics.iter().enumerate() {
        validate_id(index, &topic.id)?;
        validate_participant(index, &topic.participant_name)?;
        validate_class_path(index, "topic_class", &topic.topic_class)?;
        validate_class_path(index, "sentinel_class", &topic.sentinel_class)?;
        if topic
            .topic_class
            .eq_ignore_ascii_case(&topic.sentinel_class)
        {
            return Err(format!(
                "dialog topic {index} uses the same topic_class and sentinel_class"
            ));
        }

        let folded_id = topic.id.to_lowercase();
        if !ids.insert(folded_id) {
            return Err(format!("duplicate dialog topic id {:?}", topic.id));
        }
        let key = (
            topic.participant_name.to_ascii_lowercase(),
            topic.topic_class.to_ascii_lowercase(),
            topic.sentinel_class.to_ascii_lowercase(),
        );
        if !registrations.insert(key) {
            return Err(format!(
                "duplicate dialog topic registration at index {index}"
            ));
        }
        if !topic_classes.insert(topic.topic_class.to_ascii_lowercase()) {
            return Err(format!(
                "dialog topic class {:?} is registered more than once",
                topic.topic_class
            ));
        }
        sentinel_classes.insert(topic.sentinel_class.to_ascii_lowercase());

        rows.push_str("    { id = ");
        rows.push_str(&lua_string(&topic.id));
        rows.push_str(", participant_name = ");
        rows.push_str(&lua_string(&topic.participant_name.to_ascii_lowercase()));
        rows.push_str(", topic_class_path = ");
        rows.push_str(&lua_string(&topic.topic_class));
        rows.push_str(", sentinel_class_path = ");
        rows.push_str(&lua_string(&topic.sentinel_class));
        rows.push_str(", allow_hidden = ");
        rows.push_str(if topic.allow_hidden { "true" } else { "false" });
        rows.push_str(" },\n");
    }

    // A sentinel is a pre-existing locality proof, never another authored topic. Preflighting the
    // full batch prevents same-attempt chaining; this validation also prevents an authored topic
    // retained in a live topic set from becoming another registration's proof on a later opening.
    if let Some(class) = topic_classes.intersection(&sentinel_classes).next() {
        return Err(format!(
            "authored dialog topic class {class:?} is also used as a sentinel class"
        ));
    }

    // Assemble around the two fixed markers rather than chaining `replace`: an otherwise-valid
    // mod name that happens to contain the second marker must stay inert data, not be interpreted
    // as another template placeholder.
    let (before_mod_name, after_mod_name) = RUNTIME_TEMPLATE
        .split_once(MOD_NAME_MARKER)
        .expect("dialog runtime template must contain its mod-name marker");
    let (between_markers, after_registrations) = after_mod_name
        .split_once(REGISTRATIONS_MARKER)
        .expect("dialog runtime template must contain its registrations marker");
    let mut runtime = String::with_capacity(
        RUNTIME_TEMPLATE.len() + rows.len() + mod_name.len() + topics.len() * 32,
    );
    runtime.push_str(before_mod_name);
    runtime.push_str(&lua_string(mod_name));
    runtime.push_str(between_markers);
    runtime.push_str(&rows);
    runtime.push_str(after_registrations);
    Ok(runtime)
}

fn validate_id(index: usize, value: &str) -> Result<(), String> {
    if value.is_empty() {
        return Err(format!("dialog topic {index} has an empty id"));
    }
    if value.len() > 128 {
        return Err(format!("dialog topic {index} id exceeds 128 UTF-8 bytes"));
    }
    if value.chars().any(char::is_control) {
        return Err(format!(
            "dialog topic {index} id contains a control character"
        ));
    }
    Ok(())
}

fn validate_participant(index: usize, value: &str) -> Result<(), String> {
    if value.is_empty() || value.len() > 128 {
        return Err(format!(
            "dialog topic {index} participant_name must contain 1..=128 ASCII characters"
        ));
    }
    if !value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
    {
        return Err(format!(
            "dialog topic {index} participant_name must contain only ASCII letters, digits, and '_'"
        ));
    }
    Ok(())
}

fn validate_class_path(index: usize, field: &str, value: &str) -> Result<(), String> {
    const PREFIX: &str = "/Script/Angelscript.";
    let Some(leaf) = value.strip_prefix(PREFIX) else {
        return Err(format!(
            "dialog topic {index} {field} must start with {PREFIX:?}"
        ));
    };
    if value.len() > 256 || leaf.is_empty() {
        return Err(format!(
            "dialog topic {index} {field} must contain a non-empty class name and fit in 256 bytes"
        ));
    }
    let mut bytes = leaf.bytes();
    let first = bytes.next().expect("non-empty checked above");
    if !(first.is_ascii_alphabetic() || first == b'_')
        || !bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
    {
        return Err(format!(
            "dialog topic {index} {field} has an invalid AngelScript class name"
        ));
    }
    Ok(())
}

/// Encode a UTF-8 string as one Lua 5.4 double-quoted literal.
fn lua_string(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            other => out.push(other),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use gore_modgen::gen::{OverrideValue, SingleOverride};
    use mlua::{Lua, Table};

    const TOPIC_PATH: &str = "/Script/Angelscript.ChoiceGoreFixture";
    const SENTINEL_PATH: &str = "/Script/Angelscript.ChoiceVanillaSentinel";

    fn topic(id: &str) -> DialogTopicSpec {
        DialogTopicSpec {
            id: id.into(),
            participant_name: "om_test_target_001".into(),
            topic_class: TOPIC_PATH.into(),
            sentinel_class: SENTINEL_PATH.into(),
            allow_hidden: false,
        }
    }

    fn loader(topic: DialogTopicSpec) -> String {
        render_dialog_runtime("Dialog Test", &[topic]).unwrap()
    }

    fn mock_lua(runtime: &str, scenario: &str) -> Lua {
        let lua = Lua::new();
        lua.load(MOCK_UE4SS).exec().unwrap();
        lua.load(runtime).exec().unwrap();
        lua.load(scenario).exec().unwrap();
        lua
    }

    fn logs(lua: &Lua) -> String {
        let table: Table = lua.globals().get("gore_test_logs").unwrap();
        table
            .sequence_values::<String>()
            .map(Result::unwrap)
            .collect::<Vec<_>>()
            .join("")
    }

    #[test]
    fn validation_rejects_duplicate_ids_and_unsafe_reflection_inputs() {
        let first = topic("Same");
        let mut second = topic("same");
        second.topic_class = "/Script/Angelscript.ChoiceOther".into();
        assert!(render_dialog_runtime("M", &[first, second])
            .unwrap_err()
            .contains("duplicate dialog topic id"));

        let mut invalid = topic("invalid participant");
        invalid.participant_name = "target\"; os.execute('x')".into();
        assert!(render_dialog_runtime("M", &[invalid])
            .unwrap_err()
            .contains("participant_name"));

        let mut invalid = topic("invalid class");
        invalid.topic_class = "/Script/Angelscript.Bad\"); print('x')".into();
        assert!(render_dialog_runtime("M", &[invalid])
            .unwrap_err()
            .contains("invalid AngelScript class name"));

        let first = topic("first");
        let mut second = topic("second");
        second.sentinel_class = "/Script/Angelscript.ChoiceOtherSentinel".into();
        assert!(render_dialog_runtime("M", &[first, second])
            .unwrap_err()
            .contains("registered more than once"));

        let first = topic("first");
        let second = DialogTopicSpec {
            id: "second".into(),
            participant_name: "om_test_target_001".into(),
            topic_class: "/Script/Angelscript.ChoiceSecond".into(),
            sentinel_class: TOPIC_PATH.into(),
            allow_hidden: false,
        };
        assert!(render_dialog_runtime("M", &[first, second])
            .unwrap_err()
            .contains("also used as a sentinel"));
    }

    #[test]
    fn allow_hidden_is_opt_in_and_default_specs_stay_compact() {
        let legacy_json = serde_json::json!({
            "id": "legacy",
            "participant_name": "om_test_target_001",
            "topic_class": TOPIC_PATH,
            "sentinel_class": SENTINEL_PATH,
        });
        let mut parsed: DialogTopicSpec = serde_json::from_value(legacy_json).unwrap();
        assert!(!parsed.allow_hidden);
        let strict = serde_json::to_value(&parsed).unwrap();
        assert!(strict.get("allow_hidden").is_none());

        parsed.allow_hidden = true;
        assert_eq!(serde_json::to_value(&parsed).unwrap()["allow_hidden"], true);
    }

    #[test]
    fn generation_escapes_labels_and_is_valid_lua() {
        let runtime = loader(topic("quote\" and slash\\ and ü"));
        assert!(runtime.contains("quote\\\" and slash\\\\ and ü"));
        let lua = Lua::new();
        lua.load(MOCK_UE4SS).exec().unwrap();
        lua.load(&runtime).exec().unwrap();

        let marker_name = "__GORE_DIALOG_REGISTRATIONS__";
        let marker_runtime = render_dialog_runtime(marker_name, &[topic("marker")]).unwrap();
        assert!(marker_runtime.contains(&format!("local MOD_NAME = \"{marker_name}\"")));
        lua.load(&marker_runtime).exec().unwrap();
    }

    #[test]
    fn build_bundle_coalesces_overrides_and_dialog_runtime_into_one_lua_root() {
        let spec = crate::BuildSpec {
            meta: crate::ModMeta {
                name: "CombinedRuntime".into(),
                version: "1".into(),
                author: "test".into(),
            },
            delay_ms: 0,
            overrides: vec![SingleOverride {
                class: "ItFo_Apple".into(),
                field: "m_Value".into(),
                module: "Angelscript".into(),
                value: OverrideValue::Int(7),
            }],
            loc_edits: Default::default(),
            audio: vec![],
            texture: vec![],
            scripts: vec![],
            dialog_topics: vec![topic("fixture")],
            voice: vec![],
        };
        let bundle = crate::build_bundle(&spec).unwrap();
        let roots = bundle
            .manifest
            .components
            .iter()
            .filter(|component| matches!(component, crate::Component::Ue4ssLua { .. }))
            .count();
        assert_eq!(roots, 1);
        let lua =
            std::str::from_utf8(&bundle.files["ue4ss/CombinedRuntime/Scripts/main.lua"]).unwrap();
        assert!(lua.contains("local OVERRIDES"));
        assert!(lua.contains("[GoreDialogRuntime]"));
        Lua::new().load(lua).into_function().unwrap();
        assert!(bundle
            .files
            .contains_key("ue4ss/CombinedRuntime/enabled.txt"));
        assert!(matches!(
            bundle.manifest.components.first(),
            Some(crate::Component::Ue4ssLua {
                targets,
                opaque: true,
                ..
            }) if targets == &["ItFo_Apple.m_Value"]
        ));
        let manifest_json = std::str::from_utf8(&bundle.files["gore-mod.json"]).unwrap();
        assert!(manifest_json.contains("\"opaque\": true"));
        assert!(manifest_json.contains("ItFo_Apple.m_Value"));
    }

    #[test]
    fn dialog_only_bundle_still_emits_one_self_contained_lua_component() {
        let spec = crate::BuildSpec {
            meta: crate::ModMeta {
                name: "DialogOnly".into(),
                version: String::new(),
                author: String::new(),
            },
            delay_ms: 0,
            overrides: vec![],
            loc_edits: Default::default(),
            audio: vec![],
            texture: vec![],
            scripts: vec![],
            dialog_topics: vec![topic("fixture")],
            voice: vec![],
        };
        let bundle = crate::build_bundle(&spec).unwrap();
        assert_eq!(bundle.manifest.components.len(), 1);
        assert!(matches!(
            bundle.manifest.components.as_slice(),
            [crate::Component::Ue4ssLua {
                targets,
                opaque: true,
                ..
            }] if targets.is_empty()
        ));
        assert!(
            std::str::from_utf8(&bundle.files["ue4ss/DialogOnly/Scripts/main.lua"])
                .unwrap()
                .contains("status=%s")
        );
    }

    #[test]
    fn prepare_rejects_multiple_hand_authored_lua_roots() {
        let temp = tempfile::tempdir().unwrap();
        for name in ["A", "B"] {
            let root = temp.path().join(format!("ue4ss/{name}"));
            std::fs::create_dir_all(root.join("Scripts")).unwrap();
            std::fs::write(root.join("enabled.txt"), b"").unwrap();
            std::fs::write(root.join("Scripts/main.lua"), b"-- test\n").unwrap();
        }
        let manifest = crate::ModManifest {
            format: 1,
            mod_meta: crate::ModMeta {
                name: "Multiple".into(),
                version: String::new(),
                author: String::new(),
            },
            components: ["A", "B"]
                .into_iter()
                .map(|name| crate::Component::Ue4ssLua {
                    name: name.into(),
                    path: format!("ue4ss/{name}"),
                    targets: vec![],
                    opaque: false,
                })
                .collect(),
        };
        let game = temp.path().join("game");
        let error = match crate::prepare(
            temp.path(),
            &manifest,
            &crate::resolve_game_paths(&game),
            None,
        ) {
            Ok(_) => panic!("multiple UE4SS roots unexpectedly accepted"),
            Err(error) => error,
        };
        assert!(
            error
                .to_string()
                .contains("multiple UE4SS components in one bundle"),
            "{error}"
        );
    }

    #[test]
    fn generated_runtime_contains_only_the_proven_mutation_and_hook_order() {
        let runtime = loader(topic("fixture"));
        assert_eq!(runtime.matches(":AddTopic(").count(), 1);
        for forbidden in [
            "FindAllOf",
            "ExecuteWithDelay",
            "RegisterConsoleCommand",
            "RegisterKeyBind",
            "ServerRequestConversationWith",
            "RequestConversation",
            "Remember",
            "Knowledge",
            "Quest",
            "SaveGame",
            ":ForEach(",
            ":Empty(",
            "RemoveTopic",
            "array[index] =",
        ] {
            assert!(
                !runtime.contains(forbidden),
                "forbidden operation {forbidden}"
            );
        }
        let render = runtime
            .find("register_native_pre_hook(RENDER_TOPICS_PATH")
            .unwrap();
        let choice = runtime
            .find("register_native_pre_hook(SHOW_CHOICE_PATH")
            .unwrap();
        let mutation = runtime
            .find("register_native_pre_hook(SHOW_CONVERSATION_PATH")
            .unwrap();
        assert!(render < choice && choice < mutation);
    }

    #[test]
    fn runtime_adds_once_then_reuses_and_proves_both_render_attempts() {
        let lua = mock_lua(
            &loader(topic("fixture")),
            r#"
                gore_test_setup("om_test_target_001", true)
                gore_test_open()
                gore_test_open()
            "#,
        );
        assert_eq!(lua.globals().get::<i64>("gore_test_add_calls").unwrap(), 1);
        let output = logs(&lua);
        assert_eq!(output.matches("status=ARMED").count(), 2, "{output}");
        assert_eq!(output.matches("status=CHOICE_PASS").count(), 2, "{output}");
        assert_eq!(output.matches("status=RENDER_PASS").count(), 2, "{output}");
        assert!(output.contains("mutation=added"), "{output}");
        assert!(output.contains("mutation=reused"), "{output}");

        let hooks: Table = lua.globals().get("gore_test_hook_order").unwrap();
        let order = hooks
            .sequence_values::<String>()
            .map(Result::unwrap)
            .collect::<Vec<_>>();
        assert_eq!(
            order,
            [
                "/Script/G1R.ConversationWidget:OnShowTopicSelection",
                "/Script/G1R.GameplayAbilityConversationV2WithUI:ClientShowChoiceUI",
                "/Script/G1R.GameplayAbilityConversationV2WithUI:ClientShowConversationUI",
            ]
        );
    }

    #[test]
    fn lua_nil_topic_results_are_also_treated_as_missing() {
        for lookup_body in ["return nil", "return gore_test_wrap(nil)"] {
            let scenario = format!(
                r#"
                    gore_test_setup("om_test_target_001", true)
                    local original_find = gore_test_topic_set.FindTopicInstanceOfClass
                    local probe_lookup_count = 0
                    gore_test_topic_set.FindTopicInstanceOfClass = function(self, wanted)
                        if wanted == gore_test_probe_class then
                            probe_lookup_count = probe_lookup_count + 1
                            if probe_lookup_count == 1 then {lookup_body} end
                        end
                        return original_find(self, wanted)
                    end
                    gore_test_open()
                "#
            );
            let lua = mock_lua(&loader(topic("fixture")), &scenario);
            assert_eq!(lua.globals().get::<i64>("gore_test_add_calls").unwrap(), 1);
            let output = logs(&lua);
            assert!(output.contains("status=ARMED mutation=added"), "{output}");
            assert!(output.contains("status=RENDER_PASS"), "{output}");
        }
    }

    #[test]
    fn context_local_missing_sentinel_topic_skips_after_batch_class_preflight() {
        let first = topic("first");
        let second = DialogTopicSpec {
            id: "second".into(),
            participant_name: "om_test_target_001".into(),
            topic_class: "/Script/Angelscript.ChoiceSecondFixture".into(),
            sentinel_class: "/Script/Angelscript.ChoiceSecondSentinel".into(),
            allow_hidden: false,
        };
        let runtime = render_dialog_runtime("Dialog Test", &[first, second]).unwrap();
        let lua = mock_lua(
            &runtime,
            r#"
                gore_test_setup("om_test_target_001", true)
                gore_test_class("/Script/Angelscript.ChoiceSecondFixture")
                local second_sentinel =
                    gore_test_class("/Script/Angelscript.ChoiceSecondSentinel")
                local original_find = gore_test_topic_set.FindTopicInstanceOfClass
                gore_test_second_preflight_before_mutation = false
                gore_test_topic_set.FindTopicInstanceOfClass = function(self, wanted)
                    if wanted == second_sentinel then
                        gore_test_second_preflight_before_mutation = gore_test_add_calls == 0
                    end
                    return original_find(self, wanted)
                end
                gore_test_open()
            "#,
        );
        assert_eq!(lua.globals().get::<i64>("gore_test_add_calls").unwrap(), 1);
        assert!(lua
            .globals()
            .get::<bool>("gore_test_second_preflight_before_mutation")
            .unwrap());
        let output = logs(&lua);
        assert!(
            output.contains("registration=\"first\" attempt=1 status=ARMED"),
            "{output}"
        );
        assert!(
            output.contains(
                "registration=\"second\" attempt=1 status=SKIP reason=sentinel-topic-missing"
            ),
            "{output}"
        );
    }

    #[test]
    fn unavailable_declared_class_aborts_entire_batch_before_mutation() {
        const SECOND_TOPIC: &str = "/Script/Angelscript.ChoiceSecondFixture";
        const SECOND_SENTINEL: &str = "/Script/Angelscript.ChoiceSecondSentinel";

        let runtime = render_dialog_runtime(
            "Dialog Test",
            &[
                topic("first"),
                DialogTopicSpec {
                    id: "second".into(),
                    participant_name: "om_test_target_001".into(),
                    topic_class: SECOND_TOPIC.into(),
                    sentinel_class: SECOND_SENTINEL.into(),
                    allow_hidden: false,
                },
            ],
        )
        .unwrap();
        let scenarios = [
            (
                format!(
                    r#"
                        gore_test_setup("om_test_target_001", true)
                        gore_test_class({SECOND_SENTINEL:?})
                        gore_test_open()
                    "#
                ),
                "authored",
                "missing",
            ),
            (
                format!(
                    r#"
                        gore_test_setup("om_test_target_001", true)
                        gore_test_class({SECOND_TOPIC:?})
                        gore_test_open()
                    "#
                ),
                "sentinel",
                "missing",
            ),
            (
                format!(
                    r#"
                        gore_test_setup("om_test_target_001", true)
                        local malformed = gore_test_class({SECOND_TOPIC:?})
                        gore_test_class({SECOND_SENTINEL:?})
                        malformed.GetAddress = function() return 12.5 end
                        gore_test_open()
                    "#
                ),
                "authored",
                "malformed",
            ),
            (
                format!(
                    r#"
                        gore_test_setup("om_test_target_001", true)
                        gore_test_class({SECOND_TOPIC:?})
                        local malformed = gore_test_class({SECOND_SENTINEL:?})
                        malformed.GetAddress = function() return 12.5 end
                        gore_test_open()
                    "#
                ),
                "sentinel",
                "malformed",
            ),
        ];

        for (scenario, role, reason) in scenarios {
            let lua = mock_lua(&runtime, &scenario);
            assert_eq!(lua.globals().get::<i64>("gore_test_add_calls").unwrap(), 0);
            let output = logs(&lua);
            assert!(!output.contains("status=ARMED"), "{output}");
            assert!(
                output.contains(&format!(
                    "registration=\"second\" attempt=1 status=BATCH_FAIL stage=class-preflight role={role} reason={reason}"
                )),
                "{output}"
            );
            assert!(
                output.contains(
                    "status=BATCH_FAIL attempt=1 stage=class-preflight reason=active-class-unavailable"
                ),
                "{output}"
            );
        }
    }

    #[test]
    fn unavailable_class_for_unrelated_participant_does_not_poison_active_conversation() {
        let runtime = render_dialog_runtime(
            "Dialog Test",
            &[
                topic("active"),
                DialogTopicSpec {
                    id: "unrelated".into(),
                    participant_name: "om_other_target_999".into(),
                    topic_class: "/Script/Angelscript.ChoiceMissingOther".into(),
                    sentinel_class: "/Script/Angelscript.ChoiceMissingOtherSentinel".into(),
                    allow_hidden: false,
                },
            ],
        )
        .unwrap();
        let lua = mock_lua(
            &runtime,
            r#"
                gore_test_setup("om_test_target_001", true)
                gore_test_open()
            "#,
        );
        assert_eq!(lua.globals().get::<i64>("gore_test_add_calls").unwrap(), 1);
        let output = logs(&lua);
        assert!(
            output.contains("registration=\"active\" attempt=1 status=ARMED"),
            "{output}"
        );
        assert!(output.contains("status=CHOICE_PASS"), "{output}");
        assert!(output.contains("status=RENDER_PASS"), "{output}");
        assert!(
            output.contains(
                "registration=\"unrelated\" attempt=1 status=SKIP stage=participant-preflight reason=target-participants:0"
            ),
            "{output}"
        );
        assert!(!output.contains("status=BATCH_FAIL"), "{output}");
    }

    #[test]
    fn later_authored_lookup_failure_aborts_active_batch_before_mutation() {
        let second_path = "/Script/Angelscript.ChoiceSecondFixture";
        let runtime = render_dialog_runtime(
            "Dialog Test",
            &[
                topic("first"),
                DialogTopicSpec {
                    id: "second".into(),
                    participant_name: "om_test_target_001".into(),
                    topic_class: second_path.into(),
                    sentinel_class: SENTINEL_PATH.into(),
                    allow_hidden: false,
                },
            ],
        )
        .unwrap();
        let lua = mock_lua(
            &runtime,
            &format!(
                r#"
                    gore_test_setup("om_test_target_001", true)
                    local second_class = gore_test_class({second_path:?})
                    local original_find = gore_test_topic_set.FindTopicInstanceOfClass
                    gore_test_topic_set.FindTopicInstanceOfClass = function(self, wanted)
                        if wanted == second_class then error("lookup failed") end
                        return original_find(self, wanted)
                    end
                    gore_test_open()
                "#
            ),
        );
        assert_eq!(lua.globals().get::<i64>("gore_test_add_calls").unwrap(), 0);
        let output = logs(&lua);
        assert!(
            output.contains(
                "registration=\"second\" attempt=1 status=BATCH_FAIL stage=topic-preflight reason=topic-lookup-error"
            ),
            "{output}"
        );
        assert!(!output.contains("status=ARMED"), "{output}");
    }

    #[test]
    fn first_add_failure_prevents_later_mutation_attempts() {
        let second_path = "/Script/Angelscript.ChoiceSecondFixture";
        let runtime = render_dialog_runtime(
            "Dialog Test",
            &[
                topic("first"),
                DialogTopicSpec {
                    id: "second".into(),
                    participant_name: "om_test_target_001".into(),
                    topic_class: second_path.into(),
                    sentinel_class: SENTINEL_PATH.into(),
                    allow_hidden: false,
                },
            ],
        )
        .unwrap();
        let lua = mock_lua(
            &runtime,
            &format!(
                r#"
                    gore_test_setup("om_test_target_001", true)
                    gore_test_class({second_path:?})
                    gore_test_topic_set.AddTopic = function(_self, _wanted, _replacement)
                        gore_test_add_calls = gore_test_add_calls + 1
                        error("injected add failure")
                    end
                    gore_test_open()
                "#
            ),
        );
        assert_eq!(lua.globals().get::<i64>("gore_test_add_calls").unwrap(), 1);
        let output = logs(&lua);
        assert!(
            output.contains("registration=\"first\" attempt=1 status=FAIL"),
            "{output}"
        );
        assert!(
            !output.contains("registration=\"second\" attempt=1 status=ARMED"),
            "{output}"
        );
    }

    #[test]
    fn conversation_identity_change_during_class_preflight_aborts_before_mutation() {
        let lua = mock_lua(
            &loader(topic("fixture")),
            r#"
                gore_test_setup("om_test_target_001", true)
                local original_find = StaticFindObject
                local find_calls = 0
                StaticFindObject = function(path)
                    find_calls = find_calls + 1
                    local found = original_find(path)
                    if find_calls == 2 then gore_test_replace_group() end
                    return found
                end
                gore_test_open()
            "#,
        );
        assert_eq!(lua.globals().get::<i64>("gore_test_add_calls").unwrap(), 0);
        let output = logs(&lua);
        assert!(
            output.contains(
                "registration=\"fixture\" attempt=1 status=BATCH_FAIL stage=context-preflight reason=identity-changed"
            ),
            "{output}"
        );
        assert!(!output.contains("status=ARMED"), "{output}");
    }

    #[test]
    fn wrong_participant_or_unusable_sentinel_never_mutates() {
        for setup in [
            r#"gore_test_setup("someone_else", true); gore_test_open()"#,
            r#"gore_test_setup("om_test_target_001", false); gore_test_open()"#,
            r#"
                gore_test_setup("om_test_target_001", true)
                local original_find = gore_test_topic_set.FindTopicInstanceOfClass
                gore_test_topic_set.FindTopicInstanceOfClass = function(self, wanted)
                    if wanted == gore_test_sentinel_class then error("lookup failed") end
                    return original_find(self, wanted)
                end
                gore_test_open()
            "#,
            r#"
                gore_test_setup("om_test_target_001", true)
                local original_find = gore_test_topic_set.FindTopicInstanceOfClass
                gore_test_topic_set.FindTopicInstanceOfClass = function(self, wanted)
                    if wanted == gore_test_sentinel_class then
                        return gore_test_object(997, gore_test_probe_class)
                    end
                    return original_find(self, wanted)
                end
                gore_test_open()
            "#,
            r#"
                gore_test_setup("om_test_target_001", true)
                gore_test_sentinel_class.GetAddress = function() return 12.5 end
                gore_test_open()
            "#,
        ] {
            let lua = mock_lua(&loader(topic("fixture")), setup);
            assert_eq!(lua.globals().get::<i64>("gore_test_add_calls").unwrap(), 0);
            let output = logs(&lua);
            assert!(!output.contains("status=ARMED"), "{output}");
            assert!(!output.contains("status=RENDER_PASS"), "{output}");
        }
    }

    #[test]
    fn unreadable_or_duplicate_participants_fail_the_entire_gate() {
        for setup in [
            r#"
                gore_test_setup("om_test_target_001", true)
                gore_test_group.GetParticipantName = function() error("unreadable") end
                gore_test_open()
            "#,
            r#"
                gore_test_setup("om_test_target_001", true)
                gore_test_duplicate_target_participant()
                gore_test_open()
            "#,
            r#"
                gore_test_setup("om_test_target_001", true)
                gore_test_invalidate_participant()
                gore_test_open()
            "#,
            r#"
                gore_test_setup("om_test_target_001", true)
                gore_test_group.Participants.GetArrayNum = function() return 2.5 end
                gore_test_open()
            "#,
        ] {
            let lua = mock_lua(&loader(topic("fixture")), setup);
            assert_eq!(lua.globals().get::<i64>("gore_test_add_calls").unwrap(), 0);
            let output = logs(&lua);
            assert!(!output.contains("status=ARMED"), "{output}");
            assert!(!output.contains("status=RENDER_PASS"), "{output}");
        }
    }

    #[test]
    fn authored_or_sentinel_class_absence_is_fail_closed() {
        for missing_path in [TOPIC_PATH, SENTINEL_PATH] {
            let lua = mock_lua(
                &loader(topic("fixture")),
                &format!(
                    r#"
                        gore_test_setup("om_test_target_001", true)
                        gore_test_classes[{missing_path:?}] = nil
                        gore_test_open()
                    "#
                ),
            );
            assert_eq!(lua.globals().get::<i64>("gore_test_add_calls").unwrap(), 0);
            let output = logs(&lua);
            assert!(!output.contains("status=ARMED"), "{output}");
        }

        let lua = mock_lua(
            &loader(topic("fixture")),
            r#"
                gore_test_setup("om_test_target_001", true)
                gore_test_classes["/Script/Angelscript.ChoiceGoreFixture"].GetAddress =
                    function() return 12.5 end
                gore_test_open()
            "#,
        );
        assert_eq!(lua.globals().get::<i64>("gore_test_add_calls").unwrap(), 0);
        assert!(!logs(&lua).contains("status=ARMED"));
    }

    #[test]
    fn authored_topic_lookup_error_or_invalid_result_never_mutates() {
        for (lookup_body, expected_reason) in [
            (r#"error("lookup failed")"#, "topic-lookup-error"),
            (
                r#"return {
                    type = function() return "RemoteUnrealParam" end,
                    get = function() error("wrapper unreadable") end
                }"#,
                "topic-lookup-unreadable-wrapper",
            ),
            (
                r#"
                local invalid = gore_test_object(999, gore_test_probe_class)
                invalid.IsValid = function() return false end
                return invalid
            "#,
                "topic-lookup-invalid-result",
            ),
            (
                r#"return gore_test_object(998, gore_test_sentinel_class)"#,
                "topic-lookup-class-mismatch",
            ),
            (
                r#"return {
                    type = function() return "UnknownWrapper" end,
                    get = function() return gore_test_null_object() end
                }"#,
                "topic-lookup-unexpected-result-type",
            ),
            (
                r#"
                    local direct = gore_test_object(997, gore_test_sentinel_class)
                    direct.get = function() return gore_test_null_object() end
                    return direct
                "#,
                "topic-lookup-class-mismatch",
            ),
        ] {
            let scenario = format!(
                r#"
                    gore_test_setup("om_test_target_001", true)
                    local original_find = gore_test_topic_set.FindTopicInstanceOfClass
                    gore_test_topic_set.FindTopicInstanceOfClass = function(self, wanted)
                        if wanted == gore_test_probe_class then {lookup_body} end
                        return original_find(self, wanted)
                    end
                    gore_test_open()
                "#
            );
            let lua = mock_lua(&loader(topic("fixture")), &scenario);
            assert_eq!(lua.globals().get::<i64>("gore_test_add_calls").unwrap(), 0);
            let output = logs(&lua);
            assert!(
                output.contains(&format!(
                    "status=BATCH_FAIL stage=topic-preflight reason={expected_reason}"
                )),
                "{output}"
            );
            assert!(!output.contains("status=ARMED"), "{output}");
        }
    }

    #[test]
    fn duplicate_visible_topic_and_changed_context_fail_before_render_pass() {
        let duplicate = mock_lua(
            &loader(topic("fixture")),
            r#"
                gore_test_setup("om_test_target_001", true)
                gore_test_open_show_only()
                gore_test_duplicate_probe()
                gore_test_finish_open()
            "#,
        );
        let output = logs(&duplicate);
        assert!(output.contains("status=FAIL stage=choice"), "{output}");
        assert!(!output.contains("status=RENDER_PASS"), "{output}");

        let changed = mock_lua(
            &loader(topic("fixture")),
            r#"
                gore_test_setup("om_test_target_001", true)
                gore_test_open_show_only()
                gore_test_replace_group()
                gore_test_finish_open()
            "#,
        );
        let output = logs(&changed);
        assert!(output.contains("context-identity-changed"), "{output}");
        assert!(!output.contains("status=RENDER_PASS"), "{output}");

        let split = mock_lua(
            &loader(topic("fixture")),
            r#"
                gore_test_setup("om_test_target_001", true)
                gore_test_open_show_only()
                gore_test_split_probe_identity_and_class()
                gore_test_finish_open()
            "#,
        );
        let output = logs(&split);
        assert!(
            output.contains("topic-set-membership:topic-identity-changed"),
            "{output}"
        );
        assert!(!output.contains("status=RENDER_PASS"), "{output}");
    }

    #[test]
    fn two_distinct_topics_can_complete_one_verified_batch() {
        let first = topic("first");
        let second = DialogTopicSpec {
            id: "second".into(),
            participant_name: "om_test_target_001".into(),
            topic_class: "/Script/Angelscript.ChoiceSecondFixture".into(),
            sentinel_class: SENTINEL_PATH.into(),
            allow_hidden: false,
        };
        let runtime = render_dialog_runtime("Dialog Test", &[first, second]).unwrap();
        let lua = mock_lua(
            &runtime,
            r#"
                gore_test_setup("om_test_target_001", true)
                gore_test_class("/Script/Angelscript.ChoiceSecondFixture")
                gore_test_open()
            "#,
        );
        assert_eq!(lua.globals().get::<i64>("gore_test_add_calls").unwrap(), 2);
        let output = logs(&lua);
        assert_eq!(output.matches("status=ARMED").count(), 2, "{output}");
        assert_eq!(output.matches("status=CHOICE_PASS").count(), 2, "{output}");
        assert_eq!(output.matches("status=RENDER_PASS").count(), 2, "{output}");
        assert!(!output.contains("status=BATCH_FAIL"), "{output}");
    }

    #[test]
    fn conditional_hidden_topic_does_not_block_visible_sibling_proof() {
        let first = topic("visible");
        let second_path = "/Script/Angelscript.ChoiceConditionalFixture";
        let second = DialogTopicSpec {
            id: "conditional".into(),
            participant_name: "om_test_target_001".into(),
            topic_class: second_path.into(),
            sentinel_class: SENTINEL_PATH.into(),
            allow_hidden: true,
        };
        let runtime = render_dialog_runtime("Dialog Test", &[first, second]).unwrap();
        let lua = mock_lua(
            &runtime,
            &format!(
                r#"
                    gore_test_setup("om_test_target_001", true)
                    gore_test_hidden_class = gore_test_class({second_path:?})
                    gore_test_open()
                "#
            ),
        );
        assert_eq!(lua.globals().get::<i64>("gore_test_add_calls").unwrap(), 2);
        let output = logs(&lua);
        assert_eq!(output.matches("status=ARMED").count(), 2, "{output}");
        assert_eq!(output.matches("status=CHOICE_PASS").count(), 1, "{output}");
        assert_eq!(output.matches("status=HIDDEN").count(), 1, "{output}");
        assert_eq!(output.matches("status=RENDER_PASS").count(), 1, "{output}");
        assert!(!output.contains("status=FAIL"), "{output}");
    }

    #[test]
    fn a_lone_conditional_topic_can_be_cleanly_hidden() {
        let mut conditional = topic("conditional");
        conditional.allow_hidden = true;
        let lua = mock_lua(
            &loader(conditional),
            r#"
                gore_test_setup("om_test_target_001", true)
                gore_test_hidden_class = gore_test_probe_class
                gore_test_open()
            "#,
        );
        let output = logs(&lua);
        assert!(output.contains("status=HIDDEN stage=choice"), "{output}");
        assert!(output.contains("status=CHOICE_EMPTY"), "{output}");
        assert!(!output.contains("status=FAIL"), "{output}");
        assert!(!output.contains("status=RENDER_PASS"), "{output}");
    }

    #[test]
    fn a_required_topic_still_fails_when_hidden() {
        let lua = mock_lua(
            &loader(topic("required")),
            r#"
                gore_test_setup("om_test_target_001", true)
                gore_test_hidden_class = gore_test_probe_class
                gore_test_open()
            "#,
        );
        let output = logs(&lua);
        assert!(output.contains("reason=required-topic-hidden"), "{output}");
        assert!(!output.contains("status=RENDER_PASS"), "{output}");
    }

    #[test]
    fn conditional_topic_must_be_the_exact_member_returned_by_add_topic() {
        let mut conditional = topic("conditional");
        conditional.allow_hidden = true;
        for add_body in [
            r#"
                gore_test_add_calls = gore_test_add_calls + 1
                return gore_test_object(991, wanted)
            "#,
            r#"
                gore_test_add_calls = gore_test_add_calls + 1
                local inserted = gore_test_object(992, wanted)
                table.insert(self._topics, inserted)
                local wrong_class = gore_test_class("/Script/Angelscript.ChoiceWrongReturn")
                return gore_test_object(993, wrong_class)
            "#,
        ] {
            let scenario = format!(
                r#"
                    gore_test_setup("om_test_target_001", true)
                    gore_test_topic_set.AddTopic = function(self, wanted, _replacement)
                        {add_body}
                    end
                    gore_test_open()
                "#
            );
            let lua = mock_lua(&loader(conditional.clone()), &scenario);
            let output = logs(&lua);
            assert!(
                output.contains("status=FAIL reason=post-mutation-membership:topic-"),
                "{output}"
            );
            assert!(!output.contains("status=ARMED"), "{output}");
            assert!(!output.contains("status=HIDDEN"), "{output}");
        }
    }

    #[test]
    fn conditional_topic_removed_before_choice_fails_membership_gate() {
        let mut conditional = topic("conditional");
        conditional.allow_hidden = true;
        let lua = mock_lua(
            &loader(conditional),
            r#"
                gore_test_setup("om_test_target_001", true)
                gore_test_open_show_only()
                table.remove(gore_test_topic_set._topics, #gore_test_topic_set._topics)
                gore_test_finish_open()
            "#,
        );
        let output = logs(&lua);
        assert!(output.contains("status=ARMED"), "{output}");
        assert!(
            output.contains("stage=choice reason=topic-set-membership:topic-missing"),
            "{output}"
        );
        assert!(!output.contains("status=HIDDEN"), "{output}");
        assert!(!output.contains("status=RENDER_PASS"), "{output}");
    }

    #[test]
    fn conditional_duplicate_or_malformed_choice_array_stays_fail_closed() {
        let mut conditional = topic("conditional");
        conditional.allow_hidden = true;
        let duplicate = mock_lua(
            &loader(conditional.clone()),
            r#"
                gore_test_setup("om_test_target_001", true)
                gore_test_open_show_only()
                gore_test_duplicate_probe()
                gore_test_finish_open()
            "#,
        );
        let output = logs(&duplicate);
        assert!(output.contains("reason=visible-topic-invalid"), "{output}");
        assert!(output.contains("class_count=2"), "{output}");
        assert!(!output.contains("status=HIDDEN"), "{output}");

        let malformed = mock_lua(
            &loader(conditional),
            r#"
                gore_test_setup("om_test_target_001", true)
                gore_test_open_show_only()
                local choice = "/Script/G1R.GameplayAbilityConversationV2WithUI:ClientShowChoiceUI"
                gore_test_hooks[choice](gore_test_wrap(gore_test_ability), nil)
            "#,
        );
        let output = logs(&malformed);
        assert!(output.contains("reason=visible-topic-invalid"), "{output}");
        assert!(output.contains("nil-array"), "{output}");
        assert!(!output.contains("status=HIDDEN"), "{output}");
    }

    #[test]
    fn choice_visible_topic_must_remain_exact_at_render() {
        let mut conditional = topic("conditional");
        conditional.allow_hidden = true;
        for render_change in [
            r#"
                gore_test_hidden_class = gore_test_probe_class
                gore_test_refresh_render_topics()
            "#,
            r#"
                gore_test_duplicate_probe()
                gore_test_refresh_render_topics()
            "#,
        ] {
            let scenario = format!(
                r#"
                    gore_test_setup("om_test_target_001", true)
                    gore_test_open_show_only()
                    gore_test_finish_choice_only()
                    {render_change}
                    gore_test_finish_render_only()
                "#
            );
            let lua = mock_lua(&loader(conditional.clone()), &scenario);
            let output = logs(&lua);
            assert!(output.contains("status=CHOICE_PASS"), "{output}");
            assert!(output.contains("status=FAIL stage=render"), "{output}");
            assert!(!output.contains("status=RENDER_PASS"), "{output}");
        }
    }

    #[test]
    fn changed_widget_fails_the_render_identity_gate() {
        let lua = mock_lua(
            &loader(topic("fixture")),
            r#"
                gore_test_setup("om_test_target_001", true)
                gore_test_open_show_only()
                gore_test_finish_choice_only()
                gore_test_widget = gore_test_object(999, nil)
                gore_test_finish_render_only()
            "#,
        );
        let output = logs(&lua);
        assert!(
            output.contains("stage=render reason=widget-changed"),
            "{output}"
        );
        assert!(!output.contains("status=RENDER_PASS"), "{output}");
    }

    #[test]
    fn oversized_choice_array_fails_without_render_proof() {
        let lua = mock_lua(
            &loader(topic("fixture")),
            r#"
                gore_test_setup("om_test_target_001", true)
                gore_test_open_show_only()
                gore_test_oversize_topics()
                gore_test_finish_open()
            "#,
        );
        let output = logs(&lua);
        assert!(output.contains("count-over-limit:65"), "{output}");
        assert!(!output.contains("status=RENDER_PASS"), "{output}");
    }

    #[test]
    fn missing_observation_hook_prevents_mutating_hook_registration() {
        let lua = Lua::new();
        lua.load(MOCK_UE4SS).exec().unwrap();
        lua.load(
            r#"gore_test_fail_hook = "/Script/G1R.GameplayAbilityConversationV2WithUI:ClientShowChoiceUI""#,
        )
        .exec()
        .unwrap();
        lua.load(loader(topic("fixture"))).exec().unwrap();
        let hooks: Table = lua.globals().get("gore_test_hook_order").unwrap();
        let order = hooks
            .sequence_values::<String>()
            .map(Result::unwrap)
            .collect::<Vec<_>>();
        assert_eq!(
            order,
            [
                "/Script/G1R.ConversationWidget:OnShowTopicSelection",
                "/Script/G1R.GameplayAbilityConversationV2WithUI:ClientShowChoiceUI",
            ]
        );
        assert!(!lua
            .globals()
            .get::<Table>("gore_test_hooks")
            .unwrap()
            .contains_key(
                "/Script/G1R.GameplayAbilityConversationV2WithUI:ClientShowConversationUI"
            )
            .unwrap());
    }

    const MOCK_UE4SS: &str = r#"
        gore_test_logs = {}
        gore_test_hooks = {}
        gore_test_hook_order = {}
        gore_test_classes = {}
        gore_test_add_calls = 0
        gore_test_next_address = 100
        gore_test_fail_hook = nil
        gore_test_hidden_class = nil
        EFindName = { FNAME_Find = 1 }

        function print(value)
            table.insert(gore_test_logs, tostring(value))
        end

        function RegisterHook(path, callback)
            table.insert(gore_test_hook_order, path)
            if path == gore_test_fail_hook then error("injected hook failure") end
            gore_test_hooks[path] = callback
            return #gore_test_hook_order, nil
        end

        function gore_test_name(value)
            return { ToString = function() return value end }
        end

        function FName(value, _mode)
            return gore_test_name(value)
        end

        function gore_test_object(address, class_object)
            return {
                _address = address,
                _class = class_object,
                IsValid = function() return true end,
                GetAddress = function(self) return self._address end,
                GetClass = function(self) return self._class end,
                type = function() return "UObject" end,
            }
        end

        function gore_test_class(path)
            gore_test_next_address = gore_test_next_address + 1
            local class_object = gore_test_object(gore_test_next_address, nil)
            class_object.IsAnyClass = function() return true end
            class_object._path = path
            gore_test_classes[path] = class_object
            return class_object
        end

        function StaticFindObject(path)
            return gore_test_classes[path]
        end

        function gore_test_array(values)
            local array = {}
            for index, value in ipairs(values) do array[index] = value end
            array.GetArrayNum = function() return #values end
            return array
        end

        function gore_test_wrap(value)
            return {
                type = function() return "RemoteUnrealParam" end,
                get = function() return value end,
            }
        end

        function gore_test_null_object()
            local null_object = gore_test_object(0, nil)
            null_object.IsValid = function() return false end
            return null_object
        end

        function gore_test_setup(participant_name, include_sentinel)
            gore_test_classes = {}
            gore_test_add_calls = 0
            gore_test_probe_class = gore_test_class("/Script/Angelscript.ChoiceGoreFixture")
            gore_test_sentinel_class = gore_test_class("/Script/Angelscript.ChoiceVanillaSentinel")

            local hero = gore_test_object(201, nil)
            hero._participant_name = "hero"
            local target = gore_test_object(202, nil)
            target._participant_name = participant_name

            gore_test_topic_set = gore_test_object(301, nil)
            gore_test_topic_set._topics = {}
            if include_sentinel then
                table.insert(
                    gore_test_topic_set._topics,
                    gore_test_object(302, gore_test_sentinel_class)
                )
            end
            gore_test_topic_set.FindTopicInstanceOfClass = function(self, wanted)
                for _, topic_object in ipairs(self._topics) do
                    if topic_object:GetClass() == wanted then
                        return gore_test_wrap(topic_object)
                    end
                end
                -- Match the live UE4SS ABI: nullable UObject returns arrive as
                -- a RemoteUnrealParam containing UObject(nullptr), not Lua nil.
                return gore_test_wrap(gore_test_null_object())
            end
            gore_test_topic_set.AddTopic = function(self, wanted, _replacement)
                gore_test_add_calls = gore_test_add_calls + 1
                local added = gore_test_object(400 + gore_test_add_calls, wanted)
                table.insert(self._topics, added)
                return added
            end

            gore_test_group = gore_test_object(501, nil)
            gore_test_group.bEndRequested = false
            gore_test_group.Participants = gore_test_array({ hero, target })
            gore_test_group.TopicSet = gore_test_topic_set
            gore_test_group.GetParticipantName = function(_self, participant)
                return gore_test_name(participant._participant_name)
            end

            gore_test_widget = gore_test_object(601, nil)
            gore_test_ability = gore_test_object(701, nil)
            gore_test_ability.ConversationGroup = gore_test_group
            gore_test_ability.PlayerConversationWidget = gore_test_widget
        end

        local SHOW = "/Script/G1R.GameplayAbilityConversationV2WithUI:ClientShowConversationUI"
        local CHOICE = "/Script/G1R.GameplayAbilityConversationV2WithUI:ClientShowChoiceUI"
        local RENDER = "/Script/G1R.ConversationWidget:OnShowTopicSelection"

        function gore_test_open_show_only()
            gore_test_hooks[SHOW](gore_test_wrap(gore_test_ability))
        end

        function gore_test_finish_open()
            gore_test_finish_choice_only()
            gore_test_finish_render_only()
        end

        function gore_test_current_visible_topics()
            local visible_topics = {}
            for _, topic in ipairs(gore_test_topic_set._topics) do
                if topic:GetClass() ~= gore_test_hidden_class then
                    table.insert(visible_topics, topic)
                end
            end
            return gore_test_array(visible_topics)
        end

        function gore_test_finish_choice_only()
            gore_test_last_visible_topics = gore_test_current_visible_topics()
            gore_test_hooks[CHOICE](
                gore_test_wrap(gore_test_ability),
                gore_test_wrap(gore_test_last_visible_topics)
            )
        end

        function gore_test_refresh_render_topics()
            gore_test_last_visible_topics = gore_test_current_visible_topics()
        end

        function gore_test_finish_render_only()
            gore_test_hooks[RENDER](
                gore_test_wrap(gore_test_widget),
                gore_test_wrap(gore_test_last_visible_topics)
            )
        end

        function gore_test_open()
            gore_test_open_show_only()
            gore_test_finish_open()
        end

        function gore_test_duplicate_probe()
            table.insert(
                gore_test_topic_set._topics,
                gore_test_object(499, gore_test_probe_class)
            )
        end

        function gore_test_split_probe_identity_and_class()
            local original = gore_test_topic_set._topics[#gore_test_topic_set._topics]
            original._class = gore_test_sentinel_class
            table.insert(
                gore_test_topic_set._topics,
                gore_test_object(498, gore_test_probe_class)
            )
        end

        function gore_test_duplicate_target_participant()
            gore_test_group.Participants[1]._participant_name = "om_test_target_001"
        end

        function gore_test_invalidate_participant()
            gore_test_group.Participants[1].IsValid = function() return false end
        end

        function gore_test_replace_group()
            local replacement = gore_test_object(888, nil)
            replacement.bEndRequested = false
            replacement.Participants = gore_test_group.Participants
            replacement.TopicSet = gore_test_topic_set
            replacement.GetParticipantName = gore_test_group.GetParticipantName
            gore_test_ability.ConversationGroup = replacement
        end

        function gore_test_oversize_topics()
            while #gore_test_topic_set._topics < 65 do
                table.insert(
                    gore_test_topic_set._topics,
                    gore_test_object(1000 + #gore_test_topic_set._topics, gore_test_sentinel_class)
                )
            end
        end
    "#;
}
