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
    // Updates apply only through the user-confirmed "Restart to update" flow;
    // Velopack's default would silently install a staged update on startup,
    // and update_check surfaces such packages through the banner instead.
    VelopackApp::build().set_auto_apply_on_startup(false).run();
}

/// None when the app is not Velopack-installed (dev run, portable zip).
fn update_manager() -> Option<UpdateManager> {
    let source = HttpSource::new(UPDATE_FEED_URL);
    UpdateManager::new(source, None, None).ok()
}

pub fn update_check(_payload: &Value) -> Result<Value, CoreError> {
    let Some(manager) = update_manager() else {
        // A previously stored update must not survive a check that no longer
        // reports one, or download/apply could act on a superseded package.
        *PENDING_UPDATE.lock().unwrap() = None;
        return Ok(json!({ "status": "disabled" }));
    };
    // An update downloaded in an earlier run but not applied yet must surface
    // without network access: prefer the locally staged package over the
    // remote check. download_updates skips the download when the package file
    // already exists, so the regular check -> download -> ready flow works
    // offline for it.
    if let Some(asset) = manager.get_update_pending_restart() {
        let version = asset.Version.clone();
        *PENDING_UPDATE.lock().unwrap() = Some(UpdateInfo {
            TargetFullRelease: asset,
            BaseRelease: None,
            DeltasToTarget: Vec::new(),
            IsDowngrade: false,
        });
        return Ok(json!({ "status": "updateAvailable", "version": version }));
    }
    match manager.check_for_updates() {
        Ok(UpdateCheck::UpdateAvailable(info)) => {
            let version = info.TargetFullRelease.Version.clone();
            *PENDING_UPDATE.lock().unwrap() = Some(*info);
            Ok(json!({ "status": "updateAvailable", "version": version }))
        }
        Ok(_) => {
            *PENDING_UPDATE.lock().unwrap() = None;
            Ok(json!({ "status": "upToDate" }))
        }
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
    // On success Velopack exits this process before we get here; this response
    // only ever reaches the caller if the restart could not be initiated.
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

    // A check whose outcome is not "updateAvailable" must drop any previously
    // stored update so download/apply cannot act on a superseded package.
    // (Safe alongside the tests above: with the updater disabled they fail on
    // the manager guard before ever reading PENDING_UPDATE.)
    #[test]
    fn update_check_clears_stale_pending_update() {
        *super::PENDING_UPDATE.lock().unwrap() = Some(velopack::UpdateInfo::default());
        let response = crate::execute_json(r#"{"command":"update_check","payload":{}}"#);
        let value: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(value["data"]["status"], "disabled");
        assert!(super::PENDING_UPDATE.lock().unwrap().is_none());
    }
}
