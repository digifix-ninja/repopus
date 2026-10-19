//! Looks on GitHub for a newer version of Repopus and tells the user about it.
//!
//! Checks shortly after launch and then twice a day (compared with the time of the last
//! check, so a computer that was asleep catches up when it wakes). A new version shows a
//! system notification once and a banner in the window; the user downloads it from the
//! release page.

use crate::about::RELEASES_REPO;
use repopus_core::app_release::{self, AppRelease};
use serde::Serialize;
use std::sync::Mutex;
use std::time::{Duration, SystemTime};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;

/// Wait after launch before the first look, so the window and the repository check go first.
const STARTUP_DELAY: Duration = Duration::from_secs(20);
const EVERY: Duration = Duration::from_secs(12 * 60 * 60);
const TICK: Duration = Duration::from_secs(10 * 60);

/// What the window shows about the app's own updates.
#[derive(Debug, Clone, Default, Serialize)]
pub struct UpdateStatus {
    /// The running version.
    current: String,
    /// A newer published version, if there is one.
    available: Option<AppRelease>,
    /// Whether GitHub has been asked at least once.
    checked: bool,
    /// Why the last check failed.
    error: Option<String>,
}

/// Kept in the app state: the last answer and what the user was already told about.
#[derive(Default)]
pub struct AppUpdates {
    status: Mutex<UpdateStatus>,
    last_check: Mutex<Option<SystemTime>>,
    notified: Mutex<Option<String>>,
}

impl AppUpdates {
    /// Starts the background loop (runs until the app quits).
    pub fn start(app: &AppHandle) {
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(STARTUP_DELAY).await;
            loop {
                if app.state::<AppUpdates>().is_due() {
                    let _ = Self::check(&app).await;
                }
                tokio::time::sleep(TICK).await;
            }
        });
    }

    /// The last known status (without asking GitHub).
    pub fn status(app: &AppHandle) -> UpdateStatus {
        let mut status = app.state::<AppUpdates>().status.lock().unwrap().clone();
        status.current = app.package_info().version.to_string();
        status
    }

    /// Asks GitHub now, tells the window, and notifies once per new version.
    pub async fn check(app: &AppHandle) -> UpdateStatus {
        let current = app.package_info().version.to_string();
        let answer = app_release::latest(RELEASES_REPO).await;
        let updates = app.state::<AppUpdates>();
        *updates.last_check.lock().unwrap() = Some(SystemTime::now());
        let status = match answer {
            Ok(latest) => UpdateStatus {
                available: latest.filter(|r| app_release::is_newer(&r.version, &current)),
                checked: true,
                error: None,
                current,
            },
            Err(error) => {
                // Keep showing a version found earlier; only record the error.
                let mut status = updates.status.lock().unwrap().clone();
                status.current = current;
                status.error = Some(error);
                status
            }
        };
        *updates.status.lock().unwrap() = status.clone();
        let _ = app.emit("app-update", &status);
        if let Some(release) = &status.available {
            updates.notify_once(app, release);
        }
        status
    }

    /// Whether the next automatic check is due.
    fn is_due(&self) -> bool {
        match *self.last_check.lock().unwrap() {
            None => true,
            Some(at) => at.elapsed().map_or(true, |since| since >= EVERY),
        }
    }

    /// A system notification, the first time this version is found.
    fn notify_once(&self, app: &AppHandle, release: &AppRelease) {
        let mut notified = self.notified.lock().unwrap();
        if notified.as_deref() == Some(release.version.as_str()) {
            return;
        }
        *notified = Some(release.version.clone());
        let shown = app
            .notification()
            .builder()
            .title(format!("Repopus {} is available", release.version))
            .body("Open Repopus to download the new version.")
            .show();
        if let Err(e) = shown {
            eprintln!("Couldn't show the update notification: {e}");
        }
    }
}
