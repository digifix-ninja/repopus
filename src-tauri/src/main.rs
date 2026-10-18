//! Repopus for macOS, Windows and Linux: window, tray icon and background checks around
//! `repopus-core`.
//!
//! * `main.rs`      – starts the app and wires everything together
//! * `state.rs`     – what every command can reach (`AppState`)
//! * `events.rs`    – forwards engine events to the window
//! * `tray.rs`      – the menu-bar / system-tray icon and its menu
//! * `scheduler.rs` – runs the automatic check when it's due
//! * `token.rs`     – the GitHub token (Keychain / GitHub CLI)
//! * `about.rs`     – name, version, developer and licence for the About panel
//! * `desktop.rs`   – showing files and opening links, per operating system
//! * `look.rs`      – light / dark window frame
//! * `commands.rs`  – everything the window can ask for

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod about;
mod commands;
mod desktop;
mod events;
mod look;
mod scheduler;
mod state;
mod token;
mod tray;

use look::Look;
use repopus_core::Engine;
use scheduler::Scheduler;
use state::AppState;
use tauri::{AppHandle, Manager, RunEvent, WindowEvent};
use tray::Tray;

/// The app's identifier up to version 0.2.1, which named its data folder.
const OLD_IDENTIFIER: &str = "ninja.digifix.repopus";

/// Copies repopus.json from the old data folder the first time the new one is used,
/// so followed repositories and settings survive the identifier change.
fn bring_over_old_settings(state_file: &std::path::Path) {
    let Some(data_dir) = state_file.parent() else { return };
    let Some(old_file) = data_dir.parent().map(|p| p.join(OLD_IDENTIFIER).join("repopus.json")) else { return };
    if state_file.exists() || !old_file.exists() {
        return;
    }
    let copied = std::fs::create_dir_all(data_dir).and_then(|_| std::fs::copy(&old_file, state_file));
    if let Err(e) = copied {
        eprintln!("Couldn't bring over the old settings from {}: {e}", old_file.display());
    }
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .setup(|app| {
            let handle = app.handle().clone();
            // ~/Library/Application Support/dev.stean.repopus/repopus.json
            let state_file = app.path().app_data_dir()?.join("repopus.json");
            bring_over_old_settings(&state_file);
            let engine = Engine::new(state_file, events::forward_to(&handle))?;
            let appearance = tauri::async_runtime::block_on(engine.settings()).appearance;
            Look::apply(&handle, appearance);
            app.manage(AppState::new(engine.clone()));
            Tray::install(&handle)?;
            Scheduler::new(engine).start();
            Ok(())
        })
        .on_window_event(|window, event| {
            // With "keep running in the background", closing only hides the window.
            if let WindowEvent::CloseRequested { api, .. } = event {
                if let Some(state) = window.try_state::<AppState>() {
                    if state.engine.run_in_background() {
                        api.prevent_close();
                        let _ = window.hide();
                    }
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_state,
            commands::about,
            commands::add_link,
            commands::account_repos,
            commands::choose_repos,
            commands::check_now,
            commands::update_repo,
            commands::update_all,
            commands::update_owner,
            commands::set_repo_settings,
            commands::set_settings,
            commands::apply_defaults_to_all,
            commands::set_group_collapsed,
            commands::set_all_collapsed,
            commands::remove_repo,
            commands::remove_source,
            commands::restore_ignored,
            commands::pick_folder,
            commands::reveal_path,
            commands::open_url,
            commands::token_status,
            commands::set_token,
        ])
        .build(tauri::generate_context!())
        .expect("error while starting Repopus")
        .run(on_run_event);
}

/// Clicking the Dock icon brings a hidden window back.
fn on_run_event(app: &AppHandle, event: RunEvent) {
    #[cfg(target_os = "macos")]
    if let RunEvent::Reopen { .. } = event {
        show_main_window(app);
    }
    let _ = (app, &event);
}

/// Shows, un-minimises and focuses the main window.
pub fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}
