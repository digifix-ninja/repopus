//! The engine the app talks to. Split by job:
//!
//! * `mod.rs`     – the `Engine` object, its shared plumbing, reading and settings
//! * `library.rs` – adding and removing accounts and repositories
//! * `checker.rs` – asking GitHub what's new
//! * `updater.rs` – downloading and installing new versions
//!
//! Ground rules:
//! * Check before downloading: only download when the commit differs from the copy on
//!   disk (or the copy is missing, or the storage settings changed).
//! * Downloads land in a temporary folder, are verified, and only then replace the old
//!   copy. An interrupted download never breaks a copy.
//! * Nothing that disappeared from GitHub is ever deleted locally; it's marked as gone.
//! * Repopus only removes what it wrote itself: the copy being replaced, and versions
//!   beyond "keep last N" if the user set a limit.

mod checker;
mod library;
mod updater;

use crate::github::GitHub;
use crate::model::*;
use crate::store::StateFile;
use chrono::Utc;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex as StdMutex};
use tokio::sync::Mutex;

// ==================================================================== events

/// What the engine tells the window.
#[derive(Debug, Clone)]
pub enum EngineEvent {
    /// One repository changed; here's its new row.
    Repo(RepoView),
    RepoRemoved(String),
    /// Progress of a busy repository.
    Progress { id: String, busy: Busy },
    Activity(Activity),
    /// Several things changed: reload everything.
    StoreChanged,
}

/// Receives engine events (the app forwards them to the window).
pub type Emitter = Arc<dyn Fn(EngineEvent) + Send + Sync>;

// ==================================================================== busy rows

/// Repositories being worked on right now, with their progress.
#[derive(Default)]
struct BusyBoard {
    rows: StdMutex<HashMap<String, Busy>>,
}

impl BusyBoard {
    /// A copy of all busy rows.
    fn snapshot(&self) -> HashMap<String, Busy> {
        self.rows.lock().unwrap().clone()
    }

    /// Whether `id` is busy.
    fn contains(&self, id: &str) -> bool {
        self.rows.lock().unwrap().contains_key(id)
    }

    /// Sets the progress shown for `id`.
    fn set(&self, id: &str, busy: Busy) {
        self.rows.lock().unwrap().insert(id.to_string(), busy);
    }

    /// `id` is no longer busy.
    fn clear(&self, id: &str) {
        self.rows.lock().unwrap().remove(id);
    }

    /// Reserves `id` for an update. Fails if it's already running; a row that's only
    /// waiting in a queue (`queued`) may be taken over by that queue.
    fn claim(&self, id: &str, queued: bool) -> bool {
        let mut rows = self.rows.lock().unwrap();
        let running = rows
            .get(id)
            .is_some_and(|busy| !(queued && busy.phase == BusyPhase::Waiting));
        if running {
            return false;
        }
        rows.insert(id.to_string(), Busy::working("Preparing…"));
        true
    }
}

// ==================================================================== engine

/// Cheap to clone: every clone shares the same state.
#[derive(Clone)]
pub struct Engine {
    inner: Arc<Inner>,
}

struct Inner {
    file: StateFile,
    store: Mutex<Store>,
    github: GitHub,
    emit: Emitter,
    busy: BusyBoard,
    checking: AtomicBool,
    check_again: AtomicBool,
    updating: AtomicUsize,
    run_in_background: AtomicBool,
    /// Login of the token owner. `None` = not looked up yet.
    token_owner: StdMutex<Option<Option<String>>>,
}

impl Engine {
    /// Loads the state from `state_file` and gets ready.
    pub fn new(state_file: PathBuf, emit: Emitter) -> Result<Self, String> {
        let file = StateFile::new(state_file);
        let store = file.load()?;
        let run_in_background = store.settings.run_in_background;
        Ok(Self {
            inner: Arc::new(Inner {
                file,
                store: Mutex::new(store),
                github: GitHub::new(),
                emit,
                busy: BusyBoard::default(),
                checking: AtomicBool::new(false),
                check_again: AtomicBool::new(false),
                updating: AtomicUsize::new(0),
                run_in_background: AtomicBool::new(run_in_background),
                token_owner: StdMutex::new(None),
            }),
        })
    }

    // ---------------------------------------------------------------- token

    /// Uses `token` for all GitHub requests from now on (`None` = no token).
    pub fn set_token(&self, token: Option<String>) {
        self.inner.github.set_token(token);
        *self.inner.token_owner.lock().unwrap() = None;
    }

    /// Whether a token is set.
    pub fn has_token(&self) -> bool {
        self.inner.github.has_token()
    }

    /// Login of the token owner (looked up once, then remembered).
    pub async fn token_login(&self) -> Result<Option<String>, String> {
        let cached = self.inner.token_owner.lock().unwrap().clone();
        if let Some(login) = cached {
            return Ok(login);
        }
        let login = self.inner.github.me().await.map_err(|e| e.to_string())?;
        *self.inner.token_owner.lock().unwrap() = Some(login.clone());
        Ok(login)
    }

    // ---------------------------------------------------------------- reading

    /// Everything the window needs to draw itself.
    pub async fn view(&self) -> AppView {
        let busy = self.inner.busy.snapshot();
        let st = self.inner.store.lock().await;
        let base = &st.settings.base_folder;
        AppView {
            settings: st.settings.clone(),
            sources: st.sources.clone(),
            repos: st.repos.values().map(|r| r.to_view(base, &busy)).collect(),
            last_check: st.last_check,
            checking: self.inner.checking.load(Ordering::SeqCst),
            updating: self.inner.updating.load(Ordering::SeqCst),
            collapsed_groups: st.collapsed_groups.clone(),
        }
    }

    /// The current app settings.
    pub async fn settings(&self) -> Settings {
        self.inner.store.lock().await.settings.clone()
    }

    /// Whether closing the window should keep the app running.
    pub fn run_in_background(&self) -> bool {
        self.inner.run_in_background.load(Ordering::SeqCst)
    }

    /// Whether the scheduled check is due now.
    pub async fn is_check_due(&self) -> bool {
        let st = self.inner.store.lock().await;
        if !st.settings.auto_check || (st.sources.is_empty() && st.repos.is_empty()) {
            return false;
        }
        let interval = chrono::Duration::minutes(st.settings.check_interval_minutes.max(5) as i64);
        st.last_check.is_none_or(|last| Utc::now() - last >= interval)
    }

    // ---------------------------------------------------------------- settings

    /// Saves new app settings.
    pub async fn set_settings(&self, mut s: Settings) -> Result<(), String> {
        if s.base_folder.as_os_str().is_empty() {
            return Err("Choose a folder for your repositories.".into());
        }
        s.check_interval_minutes = s.check_interval_minutes.clamp(5, 7 * 24 * 60);
        s.concurrent_downloads = s.concurrent_downloads.clamp(1, 8);
        s.defaults.destination = None;
        s.defaults.branch = None;
        self.inner.run_in_background.store(s.run_in_background, Ordering::SeqCst);
        {
            let mut st = self.inner.store.lock().await;
            st.settings = s;
            self.save(&st);
        }
        self.emit(EngineEvent::StoreChanged);
        Ok(())
    }

    /// Saves one repository's settings. Returns its new row, and whether the branch
    /// changed (then the caller should check it again).
    pub async fn set_repo_settings(
        &self,
        id: &str,
        mut s: RepoSettings,
    ) -> Result<(RepoView, bool), String> {
        s.branch = s.branch.map(|b| b.trim().to_string()).filter(|b| !b.is_empty());
        s.destination = s.destination.filter(|p| !p.as_os_str().is_empty());
        s.keep_versions = s.keep_versions.min(999);

        let busy = self.inner.busy.snapshot();
        let (view, branch_changed) = {
            let mut st = self.inner.store.lock().await;
            let base = st.settings.base_folder.clone();
            let repo = st.repos.get_mut(id).ok_or(NOT_TRACKED)?;
            let branch_changed = repo.settings.branch != s.branch;
            repo.settings = s;
            if branch_changed {
                // Forget what we knew about the old branch (keep the repository's ETag).
                repo.remote = Remote {
                    repo_etag: repo.remote.repo_etag.take(),
                    ..Remote::default()
                };
                repo.last_error = None;
            }
            let view = repo.to_view(&base, &busy);
            self.save(&st);
            (view, branch_changed)
        };
        self.emit(EngineEvent::Repo(view.clone()));
        Ok((view, branch_changed))
    }

    /// Gives every tracked repository the default "update automatically", "keep as" and
    /// "when a new version arrives" settings. Folders and branches stay as they are.
    /// Returns how many repositories changed.
    pub async fn apply_defaults_to_all(&self) -> usize {
        let changed = {
            let mut st = self.inner.store.lock().await;
            let defaults = st.settings.defaults.clone();
            let mut changed = 0;
            for repo in st.repos.values_mut() {
                let before = repo.settings.clone();
                repo.settings.auto_update = defaults.auto_update;
                repo.settings.format = defaults.format;
                repo.settings.mode = defaults.mode;
                if repo.settings != before {
                    changed += 1;
                }
            }
            self.save(&st);
            changed
        };
        self.emit(EngineEvent::StoreChanged);
        changed
    }

    /// Remembers whether a category (owner) is folded shut in the window.
    pub async fn set_group_collapsed(&self, owner: &str, collapsed: bool) {
        let key = owner.to_lowercase();
        let mut st = self.inner.store.lock().await;
        st.collapsed_groups.retain(|g| *g != key);
        if collapsed {
            st.collapsed_groups.push(key);
        }
        self.save(&st);
    }

    /// Folds all categories shut (`true`) or opens them all (`false`).
    pub async fn set_all_collapsed(&self, collapsed: bool) {
        let mut st = self.inner.store.lock().await;
        st.collapsed_groups = if collapsed {
            let mut owners: Vec<String> = st.repos.values().map(|r| r.owner.to_lowercase()).collect();
            owners.extend(st.sources.iter().map(|s| s.id.clone()));
            owners.sort();
            owners.dedup();
            owners
        } else {
            Vec::new()
        };
        self.save(&st);
    }

    // ---------------------------------------------------------------- shared plumbing

    /// Sends an event to the window.
    fn emit(&self, event: EngineEvent) {
        (self.inner.emit)(event)
    }

    /// Shows progress on a busy row.
    fn progress(&self, id: &str, busy: Busy) {
        self.inner.busy.set(id, busy.clone());
        self.emit(EngineEvent::Progress {
            id: id.to_string(),
            busy,
        });
    }

    /// Tells the window what the app as a whole is doing.
    fn activity(&self, message: Option<String>, error: bool) {
        self.emit(EngineEvent::Activity(Activity {
            checking: self.inner.checking.load(Ordering::SeqCst),
            updating: self.inner.updating.load(Ordering::SeqCst),
            message,
            error,
        }));
    }

    /// Writes the state file; problems are shown in the window, not fatal.
    fn save(&self, st: &Store) {
        if let Err(e) = self.inner.file.save(st) {
            log::error!("{e}");
            self.activity(Some(e), true);
        }
    }

    /// A copy of one repository's data.
    async fn snapshot(&self, id: &str) -> Option<Repo> {
        self.inner.store.lock().await.repos.get(id).cloned()
    }

    /// Sends one repository's fresh row to the window.
    async fn emit_repo(&self, id: &str) {
        let busy = self.inner.busy.snapshot();
        let view = {
            let st = self.inner.store.lock().await;
            st.repos.get(id).map(|r| r.to_view(&st.settings.base_folder, &busy))
        };
        if let Some(v) = view {
            self.emit(EngineEvent::Repo(v));
        }
    }
}

/// Shown when a repository was removed while something was still working on it.
const NOT_TRACKED: &str = "This repository is no longer in Repopus.";

/// "1 repository" / "5 repositories".
fn repositories(n: usize) -> String {
    if n == 1 {
        "1 repository".into()
    } else {
        format!("{n} repositories")
    }
}
