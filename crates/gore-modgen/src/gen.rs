//! Generation engine: OverridesConfig -> Lua mod source string.
//!
//! The produced Lua applies CDO overrides using the proven pattern:
//!   StaticFindObject("/Script/Angelscript.Default__<Class>")
//! then sets `cdo[field] = value` and logs before/after.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

// ── Config types ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverridesConfig {
    pub meta: MetaConfig,
    #[serde(rename = "override")]
    pub overrides: Vec<SingleOverride>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaConfig {
    pub name: String,
    /// 0 = apply on first tick; >0 = apply via ExecuteWithDelay(ms).
    #[serde(default)]
    pub delay_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SingleOverride {
    pub class: String,
    pub field: String,
    /// UE module/package the class lives in, used as `/Script/<module>.Default__`.
    /// Defaults to `Angelscript` (where the moddable item classes live); set it
    /// for classes from another package (e.g. `G1R`) so the generated CDO lookup
    /// targets the right module instead of silently missing it.
    #[serde(default = "default_module")]
    pub module: String,
    #[serde(flatten)]
    pub value: OverrideValue,
}

fn default_module() -> String {
    "Angelscript".to_string()
}

/// A typed override value. Variants correspond to TOML keys
/// `value_int`, `value_float`, `value_bool`, `value_str`.
#[derive(Debug, Clone)]
pub enum OverrideValue {
    /// `value_int = 500` in TOML
    Int(i64),
    /// `value_float = 1.5` in TOML
    Float(f64),
    /// `value_bool = true` in TOML
    Bool(bool),
    /// `value_str = "some"` in TOML
    Str(String),
}

impl OverrideValue {
    /// Format as a Lua literal.
    pub fn lua_literal(&self) -> String {
        match self {
            OverrideValue::Int(n) => n.to_string(),
            OverrideValue::Float(f) => {
                // Emit a valid Lua number. `{f}` may render as `100` (needs a
                // decimal point to read as a float), `1.5` (already fine), or
                // `1e-8` (scientific — already a valid Lua float, and appending
                // `.0` would make `1e-8.0`, which is invalid).
                let s = format!("{f}");
                if s.contains('.') || s.contains('e') || s.contains('E') {
                    s
                } else {
                    format!("{s}.0")
                }
            }
            OverrideValue::Bool(b) => b.to_string(),
            OverrideValue::Str(s) => format!(r#""{}""#, lua_escape(s)),
        }
    }
}

/// Escape a string for safe interpolation inside a Lua double-quoted string
/// literal. Escapes backslash, double-quote, newline, carriage return, tab.
pub fn lua_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str(r"\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            other => out.push(other),
        }
    }
    out
}

// Manual serde for OverrideValue so it works with #[serde(flatten)] in
// SingleOverride. The flattened representation uses one of four keys:
// `value_int`, `value_float`, `value_bool`, `value_str`.

#[derive(Serialize, Deserialize)]
struct OverrideValueHelper {
    #[serde(skip_serializing_if = "Option::is_none")]
    value_int: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    value_float: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    value_bool: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    value_str: Option<String>,
}

impl Serialize for OverrideValue {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let helper = match self {
            OverrideValue::Int(v) => OverrideValueHelper {
                value_int: Some(*v),
                value_float: None,
                value_bool: None,
                value_str: None,
            },
            OverrideValue::Float(v) => OverrideValueHelper {
                value_int: None,
                value_float: Some(*v),
                value_bool: None,
                value_str: None,
            },
            OverrideValue::Bool(v) => OverrideValueHelper {
                value_int: None,
                value_float: None,
                value_bool: Some(*v),
                value_str: None,
            },
            OverrideValue::Str(v) => OverrideValueHelper {
                value_int: None,
                value_float: None,
                value_bool: None,
                value_str: Some(v.clone()),
            },
        };
        helper.serialize(s)
    }
}

impl<'de> Deserialize<'de> for OverrideValue {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let h = OverrideValueHelper::deserialize(d)?;
        // Count how many value_* keys are present; exactly one is required.
        let count = [
            h.value_int.is_some(),
            h.value_float.is_some(),
            h.value_bool.is_some(),
            h.value_str.is_some(),
        ]
        .iter()
        .filter(|&&b| b)
        .count();
        if count == 0 {
            return Err(serde::de::Error::custom(
                "override must have exactly one of: value_int, value_float, value_bool, value_str",
            ));
        }
        if count > 1 {
            return Err(serde::de::Error::custom(
                "override must have exactly one of: value_int, value_float, value_bool, value_str (multiple found)",
            ));
        }
        if let Some(v) = h.value_int {
            Ok(OverrideValue::Int(v))
        } else if let Some(v) = h.value_float {
            // Non-finite floats (TOML allows nan/inf/-inf) would serialize to
            // invalid Lua like `NaN.0`/`inf.0`; reject them at parse time.
            if !v.is_finite() {
                return Err(serde::de::Error::custom(
                    "value_float must be a finite number (nan/inf are not allowed)",
                ));
            }
            Ok(OverrideValue::Float(v))
        } else if let Some(v) = h.value_bool {
            Ok(OverrideValue::Bool(v))
        } else if let Some(v) = h.value_str {
            Ok(OverrideValue::Str(v))
        } else {
            unreachable!("count == 1 guarantees one Some above")
        }
    }
}

// ── Code generation ───────────────────────────────────────────────────────────

/// The runtime AngelScript class name used in `Default__<name>` lookups. The
/// reflection model spells classes with the UE C++ `U` prefix
/// (`UItMi_Orenugget`), but the live CDO is the bare AngelScript name
/// (`Default__ItMi_Orenugget`, proven in-game). Strip a leading `U` that
/// precedes an uppercase letter (the UE convention); leave bare names untouched
/// so an override may use either spelling and the emitted Lua is always correct.
pub fn runtime_class_name(class: &str) -> &str {
    let b = class.as_bytes();
    if b.first() == Some(&b'U') && b.get(1).is_some_and(u8::is_ascii_uppercase) {
        &class[1..]
    } else {
        class
    }
}

/// Generate a `main.lua` string from the given config.
pub fn gen_lua(cfg: &OverridesConfig) -> String {
    // The mod name is embedded in Lua log string literals below, so it must be
    // Lua-escaped (validate_mod_name only guarantees path-safety, not that the
    // name is free of Lua delimiters like `"`).
    let mod_name = lua_escape(&cfg.meta.name);
    let mod_name = mod_name.as_str();

    // Build OVERRIDES table
    let mut overrides_rows = Vec::new();
    for o in &cfg.overrides {
        overrides_rows.push(format!(
            r#"  {{module="{}", class="{}", field="{}", value={}}}"#,
            lua_escape(&o.module),
            lua_escape(runtime_class_name(&o.class)),
            lua_escape(&o.field),
            o.value.lua_literal()
        ));
    }
    let overrides_table = overrides_rows.join(",\n");

    // In the string.format template the mod name must also have its `%` doubled
    // (Lua treats `%` as a format directive); the plain concat below must not.
    let mod_fmt = mod_name.replace('%', "%%");

    // Build apply()/retry body. AngelScript item CDOs load lazily and are
    // absent when the mod first runs, so a single apply() at startup misses
    // them ("CDO not found"). Poll until each override's CDO appears (capped),
    // applying each exactly once via a per-row `_done` flag.
    let apply_body = format!(
        r#"local INTERVAL_MS = 1000
local MAX_ATTEMPTS = 120

local function apply()
  local pending = 0
  for _, o in ipairs(OVERRIDES) do
    if not o._done then
      local cdo = StaticFindObject("/Script/" .. o.module .. ".Default__" .. o.class)
      if cdo and cdo:IsValid() then
        local before = cdo[o.field]
        cdo[o.field] = o.value
        o._done = true
        print(string.format("[{mod_fmt}] %s.%s %s -> %s\n",
          o.class, o.field, tostring(before), tostring(cdo[o.field])))
      else
        pending = pending + 1
      end
    end
  end
  return pending
end

local attempt = 0
local function tryApply()
  attempt = attempt + 1
  local pending = apply()
  if pending > 0 and attempt < MAX_ATTEMPTS then
    ExecuteWithDelay(INTERVAL_MS, tryApply)
  elseif pending > 0 then
    print("[{mod}] gave up after " .. attempt .. " attempts; " .. pending .. " CDO(s) never appeared\n")
  end
end"#,
        mod_fmt = mod_fmt,
        mod = mod_name
    );

    // Startup invocation. delay_ms now gates the FIRST attempt; the retry loop
    // keeps polling regardless, so a 0 delay is safe.
    let startup = if cfg.meta.delay_ms == 0 {
        "tryApply()".to_string()
    } else {
        format!("ExecuteWithDelay({}, tryApply)", cfg.meta.delay_ms)
    };

    format!(
        "-- Generated by gore-cli — do not edit by hand\n\
         local OVERRIDES = {{\n\
         {overrides_table}\n\
         }}\n\
         \n\
         {apply_body}\n\
         \n\
         {startup}\n",
        overrides_table = overrides_table,
        apply_body = apply_body,
        startup = startup
    )
}

#[cfg(test)]
mod gen_tests {
    use super::*;

    #[test]
    fn runtime_class_name_strips_ue_u_prefix() {
        assert_eq!(runtime_class_name("UItMi_Orenugget"), "ItMi_Orenugget");
        assert_eq!(runtime_class_name("ItMi_Orenugget"), "ItMi_Orenugget");
        // Lowercase after U is not the UE class convention — leave untouched.
        assert_eq!(runtime_class_name("Underground"), "Underground");
        assert_eq!(runtime_class_name("U"), "U");
    }

    #[test]
    fn float_lua_literal_is_valid_for_all_forms() {
        // Integer-valued float gets a decimal point.
        assert_eq!(OverrideValue::Float(100.0).lua_literal(), "100.0");
        assert_eq!(OverrideValue::Float(1.5).lua_literal(), "1.5");
        // A tiny value: Rust's Display renders full decimal ("0.00000001"), so
        // it already has a '.' and is a valid Lua number — never `<x>.0` twice.
        let tiny = OverrideValue::Float(1e-8).lua_literal();
        assert!(tiny.contains('.') && !tiny.ends_with(".0.0"), "got: {tiny}");
        // The guard also keeps any exponent form valid (defensive: append `.0`
        // only when there is neither a '.' nor an exponent).
        assert!(!lua_literal_appends_dot_to("1e-8"));
        assert!(lua_literal_appends_dot_to("100"));
    }

    // Mirror of the gen.rs guard, for the exponent-form assertion above.
    fn lua_literal_appends_dot_to(s: &str) -> bool {
        !(s.contains('.') || s.contains('e') || s.contains('E'))
    }

    #[test]
    fn gen_lua_doubles_percent_in_mod_name_format_template() {
        let cfg = OverridesConfig {
            meta: MetaConfig {
                name: "100%Balance".to_string(),
                delay_ms: 0,
            },
            overrides: vec![SingleOverride {
                module: "Angelscript".to_string(),
                class: "ItFo_Apple".to_string(),
                field: "m_Value".to_string(),
                value: OverrideValue::Int(1),
            }],
        };
        let lua = gen_lua(&cfg);
        // In the string.format template the % must be doubled (%%); the plain
        // concat ("gave up") keeps the single %.
        assert!(
            lua.contains("[100%%Balance] %s.%s"),
            "format template must double %: {lua}"
        );
        assert!(
            lua.contains(r#"[100%Balance] gave up"#),
            "concat keeps single %: {lua}"
        );
    }

    #[test]
    fn gen_lua_escapes_mod_name_in_log_strings() {
        let cfg = OverridesConfig {
            meta: MetaConfig {
                name: r#"Bad"Mod"#.to_string(),
                delay_ms: 0,
            },
            overrides: vec![SingleOverride {
                module: "Angelscript".to_string(),
                class: "ItFo_Apple".to_string(),
                field: "m_Value".to_string(),
                value: OverrideValue::Int(1),
            }],
        };
        let lua = gen_lua(&cfg);
        // The raw unescaped delimiter must not appear; the escaped form must.
        assert!(
            lua.contains(r#"[Bad\"Mod]"#),
            "mod name must be Lua-escaped: {lua}"
        );
        assert!(
            !lua.contains(r#"[Bad"Mod]"#),
            "raw quote must not leak: {lua}"
        );
    }

    #[test]
    fn gen_lua_emits_bare_class_name_for_u_prefixed_override() {
        let cfg = OverridesConfig {
            meta: MetaConfig {
                name: "M".to_string(),
                delay_ms: 0,
            },
            overrides: vec![SingleOverride {
                module: "Angelscript".to_string(),
                class: "UItMi_Orenugget".to_string(),
                field: "m_Value".to_string(),
                value: OverrideValue::Int(5),
            }],
        };
        let lua = gen_lua(&cfg);
        assert!(
            lua.contains(r#"class="ItMi_Orenugget""#),
            "must emit bare name: {lua}"
        );
        assert!(
            !lua.contains("UItMi_Orenugget"),
            "must not emit U-prefixed name: {lua}"
        );
    }

    // ── Bug 1: lua_escape ────────────────────────────────────────────────────

    #[test]
    fn lua_escape_plain_string_unchanged() {
        assert_eq!(lua_escape("hello"), "hello");
    }

    #[test]
    fn lua_escape_special_chars() {
        assert_eq!(lua_escape(r#"say "hi""#), r#"say \"hi\""#);
        assert_eq!(lua_escape("back\\slash"), r"back\\slash");
        assert_eq!(lua_escape("new\nline"), r"new\nline");
        assert_eq!(lua_escape("carriage\rreturn"), r"carriage\rreturn");
        assert_eq!(lua_escape("tab\there"), r"tab\there");
    }

    #[test]
    fn lua_literal_str_escapes_quotes_and_backslash() {
        let v = OverrideValue::Str(r#"Bob "X""#.to_string());
        let lit = v.lua_literal();
        // Must be a valid Lua string literal — no raw unescaped double-quote inside
        assert_eq!(lit, r#""Bob \"X\"""#);
    }

    #[test]
    fn gen_lua_escapes_value_str_with_special_chars() {
        let cfg = OverridesConfig {
            meta: MetaConfig {
                name: "TestMod".to_string(),
                delay_ms: 0,
            },
            overrides: vec![SingleOverride {
                module: "Angelscript".to_string(),
                class: "SomeClass".to_string(),
                field: "m_Name".to_string(),
                value: OverrideValue::Str("say \"hello\"\nworld".to_string()),
            }],
        };
        let lua = gen_lua(&cfg);
        // The value must be escaped — raw quote or newline would break Lua
        assert!(lua.contains(r#"say \"hello\"\nworld"#));
        assert!(!lua.contains("say \"hello\"\nworld"));
    }

    #[test]
    fn gen_lua_escapes_class_and_field_names() {
        let cfg = OverridesConfig {
            meta: MetaConfig {
                name: "TestMod".to_string(),
                delay_ms: 0,
            },
            overrides: vec![SingleOverride {
                module: "Angelscript".to_string(),
                class: r#"Evil"Class"#.to_string(),
                field: r#"bad"field"#.to_string(),
                value: OverrideValue::Int(1),
            }],
        };
        let lua = gen_lua(&cfg);
        assert!(lua.contains(r#"Evil\"Class"#));
        assert!(lua.contains(r#"bad\"field"#));
    }

    // ── Bug 2: reject multiple / zero value_* keys ───────────────────────────

    #[test]
    fn deserialize_override_value_zero_keys_fails() {
        let toml_str = r#"
[meta]
name = "TestMod"
[[override]]
class = "Foo"
field = "bar"
"#;
        let result: Result<OverridesConfig, _> = toml::from_str(toml_str);
        assert!(result.is_err(), "expected error for zero value_ keys");
    }

    #[test]
    fn deserialize_override_value_two_keys_fails() {
        let toml_str = r#"
[meta]
name = "TestMod"
[[override]]
class = "Foo"
field = "bar"
value_int = 1
value_float = 1.0
"#;
        let result: Result<OverridesConfig, _> = toml::from_str(toml_str);
        assert!(result.is_err(), "expected error for two value_ keys");
    }

    #[test]
    fn deserialize_non_finite_float_fails() {
        for bad in ["inf", "-inf", "nan"] {
            let toml_str = format!(
                "[meta]\nname = \"TestMod\"\n[[override]]\nclass = \"Foo\"\nfield = \"bar\"\nvalue_float = {bad}\n"
            );
            let result: Result<OverridesConfig, _> = toml::from_str(&toml_str);
            assert!(result.is_err(), "expected error for value_float = {bad}");
        }
    }

    #[test]
    fn deserialize_override_value_exactly_one_key_succeeds() {
        let toml_str = r#"
[meta]
name = "TestMod"
[[override]]
class = "Foo"
field = "bar"
value_int = 42
"#;
        let cfg: OverridesConfig = toml::from_str(toml_str).expect("should succeed");
        assert!(matches!(cfg.overrides[0].value, OverrideValue::Int(42)));
    }
}
