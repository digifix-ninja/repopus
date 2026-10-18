//! Everything the window can ask for.
//! Long work (checking, downloading) runs in the background and reports back through
//! events, so the window never waits on the network.

use crate::about::About;
use crate::desktop::Desktop;
use crate::look::Look;
use crate::state::AppState;
use crate::token::{Token, TokenSource, TokenStatus};
use repopus_core::{AccountChoice, AddOutcome, AppView, Engine, RepoSettings, RepoView, Settings};
use std::future::Future;
use std::path::Path;
use tauri::{AppHandle, State};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_dialog::DialogExt;

type Res<T> = Result<T, String>;

/// Runs engine work in the background (results arrive as events).
fn in_background<F, Fut>(engine: &Engine, work: F)
where
    F: FnOnce(Engine) -> Fut + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    let engine = engine.clone();
    tauri::async_runtime::spawn(async move { work(engine).await });
}

// ==================================================================== reading

/// Everything the window needs to draw itself.
#[tauri::command]
pub async fn get_state(state: State<'_, AppState>) -> Res<AppView> {
    Ok(state.engine.view().await)
}

/// Name, version, developer and licence for the About panel.
#[tauri::command]
pub fn about(app: AppHandle) -> About {
    About::of(&app)
}

// ==================================================================== adding / removing

/// Adds a repository from a pasted link, then checks. An account link returns its
/// repositories instead, for the window to let the user choose.
#[tauri::command]
pub async fn add_link(state: State<'_, AppState>, link: String) -> Res<AddOutcome> {
    let outcome = state.engine.add_link(&link).await?;
    if matches!(outcome, AddOutcome::Added { .. }) {
        in_background(&state.engine, |e| async move {
            let _ = e.check_all().await;
        });
    }
    Ok(outcome)
}

/// An account's repositories, marked with the ones tracked now.
#[tauri::command]
pub async fn account_repos(state: State<'_, AppState>, owner: String) -> Res<AccountChoice> {
    state.engine.account_choice(&owner).await
}

/// Follows an account with only the chosen repositories (or changes the choice), then checks.
#[tauri::command]
pub async fn choose_repos(state: State<'_, AppState>, owner: String, chosen: Vec<String>, add_new: bool) -> Res<String> {
    let message = state.engine.choose_repos(&owner, &chosen, add_new).await?;
    in_background(&state.engine, |e| async move {
        let _ = e.check_all().await;
    });
    Ok(message)
}

/// Stops tracking a repository (files are kept).
#[tauri::command]
pub async fn remove_repo(state: State<'_, AppState>, id: String) -> Res<()> {
    state.engine.remove_repo(&id).await
}

/// Unfollows an account (files are kept).
#[tauri::command]
pub async fn remove_source(state: State<'_, AppState>, id: String) -> Res<()> {
    state.engine.remove_source(&id).await
}

/// Tracks a previously removed repository of a followed account again.
#[tauri::command]
pub async fn restore_ignored(state: State<'_, AppState>, source_id: String, name: String) -> Res<()> {
    state.engine.restore_ignored(&source_id, &name).await?;
    in_background(&state.engine, |e| async move {
        let _ = e.check_all().await;
    });
    Ok(())
}

// ==================================================================== checking / updating

/// Checks everything now.
#[tauri::command]
pub async fn check_now(state: State<'_, AppState>) -> Res<()> {
    in_background(&state.engine, |e| async move {
        let _ = e.check_all().await;
    });
    Ok(())
}

/// Updates one repository.
#[tauri::command]
pub async fn update_repo(state: State<'_, AppState>, id: String) -> Res<()> {
    in_background(&state.engine, move |e| async move {
        let _ = e.update_repo(&id).await;
    });
    Ok(())
}

/// Updates everything that needs it.
#[tauri::command]
pub async fn update_all(state: State<'_, AppState>) -> Res<()> {
    in_background(&state.engine, |e| async move {
        let _ = e.update_all().await;
    });
    Ok(())
}

/// Updates everything that needs it in one category (owner).
#[tauri::command]
pub async fn update_owner(state: State<'_, AppState>, owner: String) -> Res<()> {
    in_background(&state.engine, move |e| async move {
        let _ = e.update_owner(&owner).await;
    });
    Ok(())
}

// ==================================================================== settings

/// Saves one repository's settings (re-checks if the branch changed).
#[tauri::command]
pub async fn set_repo_settings(state: State<'_, AppState>, id: String, settings: RepoSettings) -> Res<RepoView> {
    let (view, branch_changed) = state.engine.set_repo_settings(&id, settings).await?;
    if branch_changed {
        in_background(&state.engine, move |e| async move {
            let _ = e.check_repo(&id).await;
        });
    }
    Ok(view)
}

/// Saves the app settings (and switches "open at login" on or off).
#[tauri::command]
pub async fn set_settings(app: AppHandle, state: State<'_, AppState>, settings: Settings) -> Res<()> {
    let before = state.engine.settings().await;
    if before.launch_at_login != settings.launch_at_login {
        let autolaunch = app.autolaunch();
        let result = if settings.launch_at_login {
            autolaunch.enable()
        } else {
            autolaunch.disable()
        };
        result.map_err(|e| format!("Couldn't change “open at login”: {e}"))?;
    }
    let appearance = settings.appearance;
    state.engine.set_settings(settings).await?;
    Look::apply(&app, appearance);
    Ok(())
}

/// Gives every repository the default storage and auto-update settings.
#[tauri::command]
pub async fn apply_defaults_to_all(state: State<'_, AppState>) -> Res<usize> {
    Ok(state.engine.apply_defaults_to_all().await)
}

/// Remembers whether a category is folded shut.
#[tauri::command]
pub async fn set_group_collapsed(state: State<'_, AppState>, owner: String, collapsed: bool) -> Res<()> {
    state.engine.set_group_collapsed(&owner, collapsed).await;
    Ok(())
}

/// Folds all categories shut, or opens them all.
#[tauri::command]
pub async fn set_all_collapsed(state: State<'_, AppState>, collapsed: bool) -> Res<()> {
    state.engine.set_all_collapsed(collapsed).await;
    Ok(())
}

// ==================================================================== GitHub token

/// Where the token comes from and whose it is.
#[tauri::command]
pub async fn token_status(state: State<'_, AppState>) -> Res<TokenStatus> {
    Ok(describe_token(&state).await)
}

/// Saves a token in the system keychain; `None` or empty removes it again.
#[tauri::command]
pub async fn set_token(state: State<'_, AppState>, token: Option<String>) -> Res<TokenStatus> {
    let clean = token.map(|t| t.trim().to_string()).filter(|t| !t.is_empty());
    Token::save(clean.as_deref())?;
    let (token, source) = match clean {
        Some(t) => (Some(t), TokenSource::Keychain),
        None => Token::load(),
    };
    state.engine.set_token(token);
    state.set_token_source(source);
    Ok(describe_token(&state).await)
}

/// Builds the token description for the settings panel.
async fn describe_token(state: &AppState) -> TokenStatus {
    let source = state.token_source();
    if source == TokenSource::None {
        return TokenStatus { source, login: None, error: None };
    }
    match state.engine.token_login().await {
        Ok(login) => TokenStatus { source, login, error: None },
        Err(error) => TokenStatus { source, login: None, error: Some(error) },
    }
}

// ==================================================================== system helpers

/// Lets the user choose a folder; `None` if they cancel.
#[tauri::command]
pub async fn pick_folder(app: AppHandle, start: Option<String>) -> Res<Option<String>> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    let mut dialog = app.dialog().file().set_title("Choose a folder");
    if let Some(dir) = start.filter(|s| Path::new(s).is_dir()) {
        dialog = dialog.set_directory(dir);
    }
    dialog.pick_folder(move |picked| {
        let _ = tx.send(picked);
    });
    let picked = rx.await.map_err(|e| e.to_string())?;
    Ok(picked
        .and_then(|p| p.into_path().ok())
        .map(|p| p.to_string_lossy().into_owned()))
}

/// Shows a file or folder in the file manager (or the closest folder that still exists).
#[tauri::command]
pub fn reveal_path(path: String) -> Res<()> {
    Desktop::reveal(Path::new(&path))
}

/// Opens a page in the browser (only GitHub and the developer's website).
#[tauri::command]
pub fn open_url(url: String) -> Res<()> {
    if !About::may_open(&url) {
        return Err("This link can't be opened from Repopus.".into());
    }
    Desktop::open_url(&url)
}
