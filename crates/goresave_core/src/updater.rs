//! Velopack-backed self-update commands.
//!
//! The update feed is the latest GitHub release: `releases/latest/download/`
//! redirects to the newest release's assets, where CI uploads
//! `releases.win.json` and the Velopack packages. No API calls, no token.

use std::sync::Mutex;

use serde_json::{Value, json};
use velopack::sources::HttpSource;
use velopack::{UpdateCheck, UpdateInfo, UpdateManager, VelopackApp};

use crate::CoreError;

const UPDATE_FEED_URL: &str = "https://github.com/dh0er/goresave/releases/latest/download/";

/// Update found by `update_check`, consumed by download/apply.
static PENDING_UPDATE: Mutex<Option<UpdateInfo>> = Mutex::new(None);

/// Runs Velopack startup hooks (install/update/uninstall callbacks). May exit
/// the process, so the app must call this before any other work.
pub fn velopack_startup() {
    VelopackApp::build().run();
}

/// None when the app is not Velopack-installed (dev run, portable zip).
fn update_manager() -> Option<UpdateManager> {
    let source = HttpSource::new(UPDATE_FEED_URL);
    UpdateManager::new(source, None, None).ok()
}

pub fn update_check(_payload: &Value) -> Result<Value, CoreError> {
    let Some(manager) = update_manager() else {
        return Ok(json!({ "status": "disabled" }));
    };
    match manager.check_for_updates() {
        Ok(UpdateCheck::UpdateAvailable(info)) => {
            let version = info.TargetFullRelease.Version.clone();
            *PENDING_UPDATE.lock().unwrap() = Some(*info);
            Ok(json!({ "status": "updateAvailable", "version": version }))
        }
        Ok(_) => Ok(json!({ "status": "upToDate" })),
        Err(err) => Err(CoreError::Update(err.to_string())),
    }
}

pub fn update_download(_payload: &Value) -> Result<Value, CoreError> {
    let manager =
        update_manager().ok_or_else(|| CoreError::Update("updater is disabled".to_string()))?;
    let pending = PENDING_UPDATE.lock().unwrap().clone();
    let info = pending
        .ok_or_else(|| CoreError::Update("no update pending; run update_check".to_string()))?;
    manager
        .download_updates(&info, None)
        .map_err(|err| CoreError::Update(err.to_string()))?;
    Ok(json!({
        "downloaded": true,
        "version": info.TargetFullRelease.Version,
    }))
}

pub fn update_apply_restart(_payload: &Value) -> Result<Value, CoreError> {
    let manager =
        update_manager().ok_or_else(|| CoreError::Update("updater is disabled".to_string()))?;
    let pending = PENDING_UPDATE.lock().unwrap().clone();
    let info = pending
        .ok_or_else(|| CoreError::Update("no update pending; run update_check".to_string()))?;
    manager
        .apply_updates_and_restart(&info)
        .map_err(|err| CoreError::Update(err.to_string()))?;
    Ok(json!({ "applied": true }))
}

#[cfg(test)]
mod tests {
    // cargo test binaries are never Velopack-installed, so the updater must
    // report itself disabled instead of erroring.
    #[test]
    fn update_check_reports_disabled_outside_velopack_install() {
        let response = crate::execute_json(r#"{"command":"update_check","payload":{}}"#);
        let value: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(value["ok"], true, "response: {response}");
        assert_eq!(value["data"]["status"], "disabled");
    }

    #[test]
    fn update_download_without_pending_update_fails() {
        let response = crate::execute_json(r#"{"command":"update_download","payload":{}}"#);
        let value: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(value["ok"], false);
        assert_eq!(value["error"]["code"], "UPDATE_ERROR");
    }

    #[test]
    fn update_apply_restart_without_pending_update_fails() {
        let response =
            crate::execute_json(r#"{"command":"update_apply_restart","payload":{}}"#);
        let value: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(value["ok"], false);
        assert_eq!(value["error"]["code"], "UPDATE_ERROR");
    }
}
