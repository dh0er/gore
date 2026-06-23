use gore_modgen::gen::{gen_lua, MetaConfig, OverrideValue, OverridesConfig, SingleOverride};

fn apple_config() -> OverridesConfig {
    OverridesConfig {
        meta: MetaConfig {
            name: "TestBalanceMod".to_string(),
            delay_ms: 0,
        },
        overrides: vec![
            SingleOverride {
                module: "Angelscript".to_string(),
                class: "ItFo_Apple".to_string(),
                field: "m_Value".to_string(),
                value: OverrideValue::Int(500),
            },
            SingleOverride {
                module: "Angelscript".to_string(),
                class: "ItMw_1H_Sword_01".to_string(),
                field: "m_Weight".to_string(),
                value: OverrideValue::Float(1.5),
            },
        ],
    }
}

#[test]
fn gen_lua_contains_static_find_object_pattern() {
    let lua = gen_lua(&apple_config());
    // The CDO path is built at runtime from the per-override module, so check
    // the StaticFindObject call + the runtime path template.
    assert!(lua.contains("StaticFindObject"), "must use StaticFindObject");
    assert!(
        lua.contains(r#"".Default__" .. o.class"#),
        "must build the CDO path from the module + class at runtime"
    );
}

#[test]
fn gen_lua_contains_overrides_table() {
    let lua = gen_lua(&apple_config());
    assert!(lua.contains(r#"class="ItFo_Apple""#) || lua.contains(r#"class = "ItFo_Apple""#));
    assert!(lua.contains("m_Value"));
    assert!(lua.contains("500"));
}

#[test]
fn gen_lua_mod_name_in_log() {
    let lua = gen_lua(&apple_config());
    assert!(lua.contains("TestBalanceMod"), "mod name must appear in log strings");
}

#[test]
fn gen_lua_no_delay_uses_direct_call() {
    let lua = gen_lua(&apple_config());
    // delay_ms=0: the startup invocation is a direct tryApply(), not delayed.
    // (The retry loop itself still uses ExecuteWithDelay to re-poll for CDOs
    // that load lazily, so ExecuteWithDelay appears regardless.)
    assert!(lua.contains("\ntryApply()\n"), "delay_ms=0 must call tryApply() directly");
    assert!(!lua.contains("ExecuteWithDelay(0"), "startup call must not be delayed");
    assert!(lua.contains("apply()"), "apply() must be invoked");
}

#[test]
fn gen_lua_with_delay() {
    let mut cfg = apple_config();
    cfg.meta.delay_ms = 500;
    let lua = gen_lua(&cfg);
    assert!(lua.contains("ExecuteWithDelay"), "delay_ms>0 must emit ExecuteWithDelay");
    assert!(lua.contains("500"), "delay value must appear");
}

#[test]
fn gen_lua_float_value_format() {
    let lua = gen_lua(&apple_config());
    // m_Weight = 1.5 — must be a Lua number literal, not a quoted string
    assert!(lua.contains("1.5"));
}

#[test]
fn gen_lua_from_toml_fixture() {
    let toml_str = include_str!("fixtures/overrides_valid.toml");
    let cfg: OverridesConfig = toml::from_str(toml_str).expect("fixture must parse");
    let lua = gen_lua(&cfg);
    assert!(lua.contains("ItFo_Apple"));
    assert!(lua.contains("ItMw_1H_Sword_01"));
    assert!(lua.contains("ItFo_Cheese"));
    assert!(lua.contains("777"));
}
