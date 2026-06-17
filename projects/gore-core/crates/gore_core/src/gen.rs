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
            OverrideValue::Str(s) => format!(r#""{s}""#),
        }
    }
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
        if let Some(v) = h.value_int {
            Ok(OverrideValue::Int(v))
        } else if let Some(v) = h.value_float {
            Ok(OverrideValue::Float(v))
        } else if let Some(v) = h.value_bool {
            Ok(OverrideValue::Bool(v))
        } else if let Some(v) = h.value_str {
            Ok(OverrideValue::Str(v))
        } else {
            Err(serde::de::Error::custom(
                "override must have one of: value_int, value_float, value_bool, value_str",
            ))
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
            o.class,
            o.field,
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
