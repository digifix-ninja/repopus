//! The menu-bar icon and its menu.

use crate::state::AppState;
use tauri::image::Image;
use tauri::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager};

/// macOS: black-on-transparent template, tinted by the system for light and dark menu bars.
#[cfg(target_os = "macos")]
const TRAY_ICON: &[u8] = include_bytes!("../icons/tray.png");
/// Windows and Linux: white octopus, made for the usual dark taskbars and panels.
#[cfg(not(target_os = "macos"))]
const TRAY_ICON: &[u8] = include_bytes!("../icons/tray-white.png");

pub struct Tray;

impl Tray {
    /// Puts the icon in the menu bar (macOS) or system tray (Windows, Linux).
    pub fn install(app: &AppHandle) -> tauri::Result<()> {
        let menu = Menu::with_items(
            app,
            &[
                &MenuItem::with_id(app, "open", "Open Repopus", true, None::<&str>)?,
                &MenuItem::with_id(app, "check", "Check for Updates", true, None::<&str>)?,
                &MenuItem::with_id(app, "update", "Update All", true, None::<&str>)?,
                &PredefinedMenuItem::separator(app)?,
                &MenuItem::with_id(app, "quit", "Quit Repopus", true, None::<&str>)?,
            ],
        )?;
        TrayIconBuilder::with_id("repopus")
            .icon(Image::from_bytes(TRAY_ICON)?)
            .icon_as_template(true)
            .tooltip("Repopus")
            .menu(&menu)
            .show_menu_on_left_click(true)
            .on_menu_event(Self::on_menu)
            .build(app)?;
        Ok(())
    }

    /// Handles a click on one of the menu items.
    fn on_menu(app: &AppHandle, event: MenuEvent) {
        let engine = app.state::<AppState>().engine.clone();
        match event.id.as_ref() {
            "open" => crate::show_main_window(app),
            "check" => {
                tauri::async_runtime::spawn(async move {
                    let _ = engine.check_all().await;
                });
            }
            "update" => {
                tauri::async_runtime::spawn(async move {
                    let _ = engine.update_all().await;
                });
            }
            "quit" => app.exit(0),
            _ => {}
        }
    }
}
