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
    #[serde(flatten)]
    pub value: OverrideValue,
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
                // Ensure there is always a decimal point so Lua treats it as a float
                let s = format!("{f}");
                if s.contains('.') {
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

/// Generate a `main.lua` string from the given config.
pub fn gen_lua(cfg: &OverridesConfig) -> String {
    let mod_name = &cfg.meta.name;

    // Build OVERRIDES table
    let mut overrides_rows = Vec::new();
    for o in &cfg.overrides {
        overrides_rows.push(format!(
            r#"  {{class="{}", field="{}", value={}}}"#,
            lua_escape(&o.class),
            lua_escape(&o.field),
            o.value.lua_literal()
        ));
    }
    let overrides_table = overrides_rows.join(",\n");

    // Build apply() body
    let apply_body = format!(
        r#"local function apply()
  for _, o in ipairs(OVERRIDES) do
    local cdo = StaticFindObject("/Script/Angelscript.Default__" .. o.class)
    if cdo and cdo:IsValid() then
      local before = cdo[o.field]
      cdo[o.field] = o.value
      print(string.format("[{mod}] %s.%s %s -> %s\n",
        o.class, o.field, tostring(before), tostring(cdo[o.field])))
    else
      print("[{mod}] CDO not found: " .. o.class .. "\n")
    end
  end
end"#,
        mod = mod_name
    );

    // Startup invocation
    let startup = if cfg.meta.delay_ms == 0 {
        "apply()".to_string()
    } else {
        format!("ExecuteWithDelay({}, function() apply() end)", cfg.meta.delay_ms)
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
            meta: MetaConfig { name: "TestMod".to_string(), delay_ms: 0 },
            overrides: vec![SingleOverride {
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
            meta: MetaConfig { name: "TestMod".to_string(), delay_ms: 0 },
            overrides: vec![SingleOverride {
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
