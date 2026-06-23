//! C ABI for gore-mod's `dart:ffi` bridge. Mirrors gore-save's
//! `goresave_execute`/`goresave_free`: a single JSON-in/JSON-out entry point.
//!
//! Request:  `{"command": "<name>", "payload": { ... }}`
//! Response: `{"ok": true, ...}` or `{"ok": false, "error": {"code","message"}}`
//!
//! Commands:
//! - `generate_mod` — payload is an [`OverridesConfig`] (keys `meta` +
//!   `override`); returns `{ok, files:{"enabled.txt":"","Scripts/main.lua":...}}`.
//! - `validate` — payload `{config: OverridesConfig, model: ReflectionModel}`;
//!   returns `{ok, valid, errors:[..]}`.

use std::ffi::{c_char, CStr, CString};
use std::ptr;

use serde_json::{json, Value};

use std::path::PathBuf;

use gore_modgen::gen::{gen_lua, OverridesConfig};
use gore_reflect::model::ReflectionModel;
use gore_modgen::validate::validate_config;
use gore_loc::{loc_store, paths};

/// # Safety
/// `request_json` must be null or a valid, NUL-terminated C string pointer that
/// stays valid for the duration of the call. The returned pointer is owned by
/// the caller and must be released with [`gore_core_free`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gore_core_execute(request_json: *const c_char) -> *mut c_char {
    if request_json.is_null() {
        return cstring_ptr(execute_json(r#"{"command":null}"#));
    }
    let input = unsafe { CStr::from_ptr(request_json) }
        .to_string_lossy()
        .to_string();
    cstring_ptr(execute_json(&input))
}

/// # Safety
/// `ptr` must be null or a pointer previously returned by [`gore_core_execute`]
/// that has not already been freed. Passing any other pointer is undefined
/// behavior.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gore_core_free(ptr: *mut c_char) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        drop(CString::from_raw(ptr));
    }
}

fn cstring_ptr(value: String) -> *mut c_char {
    match CString::new(value) {
        Ok(s) => s.into_raw(),
        Err(_) => ptr::null_mut(),
    }
}

/// Pure entry point (no FFI) — also the test seam.
pub fn execute_json(input: &str) -> String {
    let resp = dispatch(input);
    serde_json::to_string(&resp).unwrap_or_else(|_| {
        r#"{"ok":false,"error":{"code":"SERIALIZE","message":"response serialize failed"}}"#
            .to_string()
    })
}

fn err(code: &str, msg: impl Into<String>) -> Value {
    json!({"ok": false, "error": {"code": code, "message": msg.into()}})
}

fn dispatch(input: &str) -> Value {
    let req: Value = match serde_json::from_str(input) {
        Ok(v) => v,
        Err(e) => return err("BAD_REQUEST", format!("invalid request json: {e}")),
    };
    let command = req.get("command").and_then(Value::as_str).unwrap_or("");
    let payload = req.get("payload").cloned().unwrap_or(Value::Null);
    match command {
        "generate_mod" => generate_mod(payload),
        "validate" => validate(payload),
        "loc_status" => loc_status(),
        "loc_find" => loc_find(payload),
        "loc_extract" => loc_extract(payload),
        "audio_list" => audio_list(payload),
        "audio_extract" => audio_extract(payload),
        "mod_build" => mod_build(payload),
        "mod_deploy" => mod_deploy(payload),
        "mod_undeploy" => mod_undeploy(payload),
        other => err("UNKNOWN_COMMAND", format!("unknown command: {other}")),
    }
}

/// `{ok, present, meta?, catalog_path, dir}` — is the shared catalog extracted?
fn loc_status() -> Value {
    let present = loc_store::catalog_present();
    // Only report metadata while the catalog file is present, so stale sidecar
    // meta can't describe a catalog that no longer exists.
    let meta = if present { loc_store::status() } else { None };
    json!({
        "ok": true,
        "present": present,
        "meta": meta,
        "catalog_path": paths::loc_catalog_path().display().to_string(),
        "dir": paths::shared_data_dir().display().to_string(),
    })
}

/// `{ok, found, path?}` — auto-detect (or resolve `payload.lcache`) the .lcache.
fn loc_find(payload: Value) -> Value {
    let hint = payload.get("lcache").and_then(Value::as_str).map(PathBuf::from);
    let found = loc_store::resolve_lcache(hint.as_deref());
    json!({
        "ok": true,
        "found": found.is_some(),
        "path": found.map(|p| p.display().to_string()),
    })
}

/// Extract to the shared catalog. `payload.lcache` is an optional path hint.
fn loc_extract(payload: Value) -> Value {
    let hint = payload.get("lcache").and_then(Value::as_str).map(PathBuf::from);
    match loc_store::extract(hint.as_deref()) {
        Ok(meta) => json!({ "ok": true, "meta": meta }),
        Err(gore_loc::loc_store::LocStoreError::NotFound) => err(
            "LCACHE_NOT_FOUND",
            "could not find AlkimiaLocalization .lcache (auto-detect failed); pick it manually",
        ),
        Err(e) => err("EXTRACT_FAILED", e.to_string()),
    }
}

fn generate_mod(payload: Value) -> Value {
    let cfg: OverridesConfig = match serde_json::from_value(payload) {
        Ok(c) => c,
        Err(e) => return err("BAD_CONFIG", format!("invalid overrides config: {e}")),
    };
    let lua = gen_lua(&cfg);
    json!({
        "ok": true,
        "files": {
            "enabled.txt": "",
            "Scripts/main.lua": lua,
        }
    })
}

/// `{bank}` → `{ok, codec, samples:[{index,name,freq,channels,seconds}]}`
fn audio_list(payload: Value) -> Value {
    let Some(bank) = payload.get("bank").and_then(Value::as_str) else {
        return err("BAD_REQUEST", "missing 'bank'");
    };
    let bytes = match std::fs::read(bank) {
        Ok(b) => b,
        Err(e) => return err("IO", format!("reading bank: {e}")),
    };
    let key = payload
        .get("key")
        .and_then(Value::as_str)
        .map(|s| s.as_bytes().to_vec())
        .unwrap_or_else(|| gore_fmod::GOTHIC_STUDIO_KEY.to_vec());
    let fsb = match gore_fmod::bank_fsb0(&bytes, &key) {
        Ok(f) => f,
        Err(e) => return err("DECODE", e),
    };
    let samples: Vec<Value> = fsb
        .samples
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let secs = if s.freq > 0 { s.num_samples as f64 / s.freq as f64 } else { 0.0 };
            json!({"index": i, "name": s.name, "freq": s.freq, "channels": s.channels, "seconds": secs})
        })
        .collect();
    json!({"ok": true, "codec": format!("{:?}", fsb.codec), "samples": samples})
}

/// `{bank, sample}` → `{ok, ogg_path}` — extract one Vorbis sample to a temp .ogg for preview.
fn audio_extract(payload: Value) -> Value {
    let (Some(bank), Some(sample)) = (
        payload.get("bank").and_then(Value::as_str),
        payload.get("sample").and_then(Value::as_str),
    ) else {
        return err("BAD_REQUEST", "missing 'bank' or 'sample'");
    };
    let bytes = match std::fs::read(bank) {
        Ok(b) => b,
        Err(e) => return err("IO", format!("reading bank: {e}")),
    };
    let key = payload
        .get("key")
        .and_then(Value::as_str)
        .map(|s| s.as_bytes().to_vec())
        .unwrap_or_else(|| gore_fmod::GOTHIC_STUDIO_KEY.to_vec());
    let (block, fsb) = match gore_fmod::decrypt_fsb0(&bytes, &key) {
        Ok(v) => v,
        Err(e) => return err("DECODE", e),
    };
    let Some(index) = fsb.samples.iter().position(|s| s.name == sample) else {
        return err("NOT_FOUND", format!("sample not found: {sample}"));
    };
    let ogg = match gore_fmod::extract_ogg(&block, &fsb, index) {
        Ok(o) => o,
        Err(e) => return err("EXTRACT", e),
    };
    let dir = std::env::temp_dir().join("gore-fmod-preview");
    if let Err(e) = std::fs::create_dir_all(&dir) {
        return err("IO", format!("temp dir: {e}"));
    }
    let safe: String = sample.chars().map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '_' }).collect();
    let path = dir.join(format!("{safe}.ogg"));
    if let Err(e) = std::fs::write(&path, &ogg) {
        return err("IO", format!("writing ogg: {e}"));
    }
    json!({"ok": true, "ogg_path": path.display().to_string()})
}

/// `{out_dir, spec:BuildSpec}` → build the unified bundle into `out_dir`.
fn mod_build(payload: Value) -> Value {
    let Some(out_dir) = payload.get("out_dir").and_then(Value::as_str) else {
        return err("BAD_REQUEST", "missing 'out_dir'");
    };
    let spec_val = payload.get("spec").cloned().unwrap_or(Value::Null);
    let spec: gore_mod::BuildSpec = match serde_json::from_value(spec_val) {
        Ok(s) => s,
        Err(e) => return err("BAD_SPEC", format!("invalid build spec: {e}")),
    };
    let bundle = match gore_mod::build_bundle(&spec) {
        Ok(b) => b,
        Err(e) => return err("BUILD_FAILED", e.to_string()),
    };
    let dir = std::path::Path::new(out_dir).join(&spec.meta.name);
    if let Err(e) = gore_mod::write_bundle(&dir, &bundle) {
        return err("IO", e.to_string());
    }
    json!({
        "ok": true,
        "bundle_dir": dir.display().to_string(),
        "components": bundle.manifest.components.len(),
        "files": bundle.files.len(),
    })
}

/// `{bundle_dir, game_root}` → deploy.
fn mod_deploy(payload: Value) -> Value {
    let (Some(bundle_dir), Some(game_root)) = (
        payload.get("bundle_dir").and_then(Value::as_str),
        payload.get("game_root").and_then(Value::as_str),
    ) else {
        return err("BAD_REQUEST", "missing 'bundle_dir' or 'game_root'");
    };
    match gore_mod::deploy(std::path::Path::new(bundle_dir), std::path::Path::new(game_root)) {
        Ok(rec) => json!({"ok": true, "record": serde_json::to_value(rec).unwrap_or(Value::Null)}),
        Err(e) => err("DEPLOY_FAILED", e.to_string()),
    }
}

/// `{game_root}` → undeploy the active mod.
fn mod_undeploy(payload: Value) -> Value {
    let Some(game_root) = payload.get("game_root").and_then(Value::as_str) else {
        return err("BAD_REQUEST", "missing 'game_root'");
    };
    match gore_mod::undeploy(std::path::Path::new(game_root)) {
        Ok(rec) => json!({"ok": true, "record": serde_json::to_value(rec).unwrap_or(Value::Null)}),
        Err(e) => err("UNDEPLOY_FAILED", e.to_string()),
    }
}

fn validate(payload: Value) -> Value {
    let cfg: OverridesConfig = match payload
        .get("config")
        .cloned()
        .and_then(|c| serde_json::from_value(c).ok())
    {
        Some(c) => c,
        None => return err("BAD_CONFIG", "missing/invalid 'config'"),
    };
    let model: ReflectionModel = match payload
        .get("model")
        .cloned()
        .and_then(|m| serde_json::from_value(m).ok())
    {
        Some(m) => m,
        None => return err("BAD_MODEL", "missing/invalid 'model'"),
    };
    let errors: Vec<String> = validate_config(&cfg, &model)
        .iter()
        .map(ToString::to_string)
        .collect();
    json!({ "ok": true, "valid": errors.is_empty(), "errors": errors })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_mod_returns_files_with_cdo_pattern() {
        let req = r#"{"command":"generate_mod","payload":{
            "meta":{"name":"M","delay_ms":0},
            "override":[{"class":"ItFo_Apple","field":"m_Value","value_int":500}]
        }}"#;
        let v: Value = serde_json::from_str(&execute_json(req)).unwrap();
        assert_eq!(v["ok"], true);
        let lua = v["files"]["Scripts/main.lua"].as_str().unwrap();
        assert!(lua.contains("ItFo_Apple"));
        assert!(lua.contains("Default__"));
        assert_eq!(v["files"]["enabled.txt"], "");
    }

    #[test]
    fn loc_status_reports_shared_catalog_path() {
        let v: Value = serde_json::from_str(&execute_json(r#"{"command":"loc_status"}"#)).unwrap();
        assert_eq!(v["ok"], true);
        assert!(v["catalog_path"].as_str().unwrap().contains("gore-tools"));
        assert!(v.get("present").is_some());
    }

    #[test]
    fn unknown_command_errors() {
        let v: Value = serde_json::from_str(&execute_json(r#"{"command":"nope"}"#)).unwrap();
        assert_eq!(v["ok"], false);
        assert_eq!(v["error"]["code"], "UNKNOWN_COMMAND");
    }

    #[test]
    fn bad_config_errors() {
        let v: Value = serde_json::from_str(&execute_json(
            r#"{"command":"generate_mod","payload":{"meta":{"name":"M"}}}"#,
        ))
        .unwrap();
        assert_eq!(v["ok"], false);
    }
}
