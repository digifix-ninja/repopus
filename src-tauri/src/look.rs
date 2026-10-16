//! Light / dark window frame. The window's contents switch themselves (CSS); this makes
//! the native parts follow too: the title bar and, on macOS, the window buttons.

use repopus_core::Appearance;
use tauri::{AppHandle, Manager, Theme};

pub struct Look;

impl Look {
    /// Gives the main window the chosen appearance (`System` = follow the OS).
    pub fn apply(app: &AppHandle, appearance: Appearance) {
        let theme = match appearance {
            Appearance::System => None,
            Appearance::Light => Some(Theme::Light),
            Appearance::Dark => Some(Theme::Dark),
        };
        if let Some(window) = app.get_webview_window("main") {
            if let Err(e) = window.set_theme(theme) {
                log::warn!("couldn't change the window theme: {e}");
            }
        }
    }
}
