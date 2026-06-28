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
        "find_game" => find_game(),
        "audio_list" => audio_list(payload),
        "audio_extract" => audio_extract(payload),
        "mod_build" => mod_build(payload),
        "mod_deploy" => mod_deploy(payload),
        "mod_undeploy" => mod_undeploy(payload),
        "texture_index" => texture_index(payload),
        "texture_extract" => texture_extract(payload),
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

/// `{ok, found, game_root?, exe?}` — auto-detect the game install via Steam.
fn find_game() -> Value {
    let root = gore_loc::discover::find_game_root();
    let exe = gore_loc::discover::find_game_exe();
    json!({
        "ok": true,
        "found": root.is_some(),
        "game_root": root.map(|p| p.display().to_string()),
        "exe": exe.map(|p| p.display().to_string()),
    })
}

/// Read a bank's PRISTINE bytes for listing/preview. The live bank is the source of truth when it
/// isn't injected yet (a single FSB5): that covers an un-deployed bank AND the case where a
/// `restore` or a Steam update refreshed the live bank, so the Audio tab never lists/previews
/// obsolete samples from a stale `*.gore-bak`. Only when the live bank is already injected
/// (>1 FSB5) do we fall back to the backup, which holds the true original. Mirrors the CLI's
/// `read_pristine_bank`.
/// The install's recovered FMOD key (from the `gore_fmod_key.json` gore-dump writes to
/// `Binaries/Win64`) when present and valid, else the compiled-in constant — so the Audio tab can
/// browse/preview banks on installs whose key changed after a game patch.
fn resolve_fmod_key_for_bank(bank: &str) -> Vec<u8> {
    // bank == <...>/G1R/Content/FMOD/Desktop/<file>.bank; G1R is 4 levels up, then Binaries/Win64.
    if let Some(g1r) = std::path::Path::new(bank).ancestors().nth(4) {
        let key_file = g1r.join("Binaries").join("Win64").join("gore_fmod_key.json");
        if let Ok(bytes) = std::fs::read(&key_file) {
            if let Ok(v) = serde_json::from_slice::<Value>(&bytes) {
                if v.get("found").and_then(Value::as_bool).unwrap_or(false) {
                    if let Some(k) = v.get("encryption_key").and_then(Value::as_str) {
                        if !k.is_empty() {
                            return k.as_bytes().to_vec();
                        }
                    }
                }
            }
        }
    }
    gore_fmod::GOTHIC_STUDIO_KEY.to_vec()
}

fn read_bank_pristine(bank: &str) -> std::io::Result<Vec<u8>> {
    let live = std::fs::read(bank)?;
    if !gore_fmod::is_pristine_bank(&live) {
        // The live bank is injected (or unparseable) — its true pristine is the backup, if any.
        let bak = format!("{bank}.gore-bak");
        if std::path::Path::new(&bak).exists() {
            return std::fs::read(&bak);
        }
    }
    Ok(live)
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
    let bytes = match read_bank_pristine(bank) {
        Ok(b) => b,
        Err(e) => return err("IO", format!("reading bank: {e}")),
    };
    let key = match payload.get("key").and_then(Value::as_str) {
        Some(k) if k.is_empty() => return err("BAD_KEY", "encryption key must not be empty"),
        Some(k) => k.as_bytes().to_vec(),
        None => resolve_fmod_key_for_bank(bank),
    };
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
    let bytes = match read_bank_pristine(bank) {
        Ok(b) => b,
        Err(e) => return err("IO", format!("reading bank: {e}")),
    };
    let key = match payload.get("key").and_then(Value::as_str) {
        Some(k) if k.is_empty() => return err("BAD_KEY", "encryption key must not be empty"),
        Some(k) => k.as_bytes().to_vec(),
        None => resolve_fmod_key_for_bank(bank),
    };
    let (block, fsb) = match gore_fmod::decrypt_fsb0(&bytes, &key) {
        Ok(v) => v,
        Err(e) => return err("DECODE", e),
    };
    let Some(index) = fsb.samples.iter().position(|s| s.name == sample) else {
        return err("NOT_FOUND", format!("sample not found: {sample}"));
    };
    let wav = match gore_fmod::extract_wav(&block, &fsb, index) {
        Ok(o) => o,
        Err(e) => return err("EXTRACT", e),
    };
    let dir = std::env::temp_dir().join("gore-fmod-preview");
    if let Err(e) = std::fs::create_dir_all(&dir) {
        return err("IO", format!("temp dir: {e}"));
    }
    let safe: String = sample.chars().map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '_' }).collect();
    let path = dir.join(format!("{safe}.wav"));
    if let Err(e) = std::fs::write(&path, &wav) {
        return err("IO", format!("writing wav: {e}"));
    }
    json!({"ok": true, "ogg_path": path.display().to_string(), "wav_path": path.display().to_string()})
}

/// `{ok, build_id, count, entries:{path:package_id_str}}` — load the cached index, building it
/// if absent or if `payload.rebuild` is true. `payload.game` = install dir.
fn texture_index(payload: Value) -> Value {
    let game = match payload.get("game").and_then(Value::as_str) {
        Some(g) => std::path::PathBuf::from(g),
        None => return err("BAD_REQUEST", "missing game"),
    };
    let rebuild = payload.get("rebuild").and_then(Value::as_bool).unwrap_or(false);
    let cache = gore_tex::paths::texture_index_path();
    let usmap = match gore_tex::paths::usmap(&game) {
        Ok(p) => p, Err(e) => return err("USMAP", e.to_string()) };
    let utoc = match gore_tex::paths::main_container(&game) {
        Ok(p) => p, Err(e) => return err("CONTAINER", e.to_string()) };
    let build_id = gore_tex::index::build_id_for(&utoc, &usmap);
    // Only reuse the cache when it's current for THIS game build; build_id keys on the .usmap
    // name AND the container's identity, so a game patch (even one keeping the .usmap name) that
    // rewrites the container invalidates a cache mapping paths to outdated package ids.
    let cached = if rebuild { None } else { gore_tex::index::TextureIndex::load_current(&cache, &build_id) };
    let mut cache_saved = true; // a loaded cache is, by definition, already persisted
    let idx = match cached {
        Some(i) => i,
        None => {
            let i = match gore_tex::index::build_index(&utoc, &build_id) {
                Ok(i) => i, Err(e) => return err("INDEX_BUILD", e.to_string()) };
            // Don't silently ignore a failed persist: the index is usable in-memory this call,
            // but a failed write means every later load rebuilds. Surface it (warning + flag)
            // instead of reporting unqualified success.
            if let Err(e) = i.save(&cache) {
                eprintln!("warning: failed to persist texture index cache: {e}");
                cache_saved = false;
            }
            i
        }
    };
    let entries: serde_json::Map<String, Value> = idx.entries.iter()
        .map(|(k, v)| (k.clone(), Value::String(v.to_string()))).collect();
    json!({ "ok": true, "build_id": idx.build_id, "count": idx.entries.len(), "cache_saved": cache_saved, "entries": entries })
}

/// `{ok, png_path, width, height, format}` — extract a texture to a temp PNG. `payload.game`,
/// and either `payload.package_id` (string) or `payload.asset` (path).
fn texture_extract(payload: Value) -> Value {
    let game = match payload.get("game").and_then(Value::as_str) {
        Some(g) => std::path::PathBuf::from(g), None => return err("BAD_REQUEST", "missing game") };
    let utoc = match gore_tex::paths::main_container(&game) {
        Ok(p) => p, Err(e) => return err("CONTAINER", e.to_string()) };
    let usmap = match gore_tex::paths::usmap(&game) {
        Ok(p) => p, Err(e) => return err("USMAP", e.to_string()) };
    let asset = payload.get("asset").and_then(Value::as_str).unwrap_or("");
    let leaf = asset.rsplit('/').next().unwrap_or("texture").to_string();
    let (info, px) = if let Some(pid) = payload.get("package_id").and_then(Value::as_str).and_then(|s| s.parse::<u64>().ok()) {
        match gore_tex::index::extract_by_package_id(&utoc, &usmap, pid, &leaf) {
            Ok(x) => x, Err(e) => return err("EXTRACT", e.to_string()) }
    } else if !asset.is_empty() {
        let tmp = match gore_tex::paths::unique_temp_dir("gore-tex-ffi-extract") {
            Ok(t) => t, Err(_) => return err("IO", "tmp") };
        let ua = match gore_tex::container::unpack_asset(&utoc, &usmap, asset, &tmp) {
            Ok(p) => p, Err(e) => { let _ = std::fs::remove_dir_all(&tmp); return err("UNPACK", e.to_string()) } };
        // Surface read failures instead of defaulting to empty bytes (which would yield a
        // misleading PARSE/DECODE error). `.ubulk` is legitimately optional (inline-mip textures).
        macro_rules! read_or_err { ($p:expr) => { match std::fs::read($p) {
            Ok(b) => b, Err(e) => { let _ = std::fs::remove_dir_all(&tmp); return err("READ", e.to_string()) } } }; }
        let ua_bytes = read_or_err!(&ua);
        let uexp_bytes = read_or_err!(ua.with_extension("uexp"));
        let usmap_bytes = read_or_err!(&usmap);
        let ubulk_bytes = std::fs::read(ua.with_extension("ubulk")).unwrap_or_default();
        let info = match gore_tex::decode::parse(&ua_bytes, &uexp_bytes, &ubulk_bytes, &usmap_bytes) {
            Ok(i) => i, Err(e) => { let _ = std::fs::remove_dir_all(&tmp); return err("PARSE", e.to_string()) } };
        let px = match gore_tex::decode::to_rgba8(&info) {
            Ok(p) => p, Err(e) => { let _ = std::fs::remove_dir_all(&tmp); return err("DECODE", e.to_string()) } };
        let _ = std::fs::remove_dir_all(&tmp);
        (info, px)
    } else { return err("BAD_REQUEST", "need package_id or asset"); };
    let mut buf = Vec::with_capacity(px.len() * 4);
    for p in px { buf.extend_from_slice(&[(p >> 16) as u8, (p >> 8) as u8, p as u8, (p >> 24) as u8]); }
    // Unique per-request output path: a deterministic name would let two
    // extractions of the same texture (e.g. a stale request finishing after a
    // game/index change) race on one file. Each call owns its own PNG and the UI
    // deletes exactly the file it was handed.
    let out = gore_tex::paths::unique_temp_file(&format!("gore-tex-preview-{leaf}"), "png");
    if image::save_buffer(&out, &buf, info.width, info.height, image::ColorType::Rgba8).is_err() {
        return err("PNG", "save failed");
    }
    // `replaceable` is the AUTHORITATIVE capability flag the UI gates the Replace
    // button on (always a plain bool, present on both the package_id and asset
    // extract paths since both bind `info`). `is_virtual`/`vt_layers` are exposed
    // for diagnostics.
    json!({ "ok": true, "png_path": out.display().to_string(), "width": info.width, "height": info.height,
        "format": info.format, "replaceable": gore_tex::decode::replace_supported(&info),
        "is_virtual": info.is_virtual, "vt_layers": info.vt_layers })
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
