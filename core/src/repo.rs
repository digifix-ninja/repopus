//! Behaviour of one tracked repository: its branch, its status and its row in the list.

use crate::github::ApiRepo;
use crate::model::*;
use crate::placement::Placement;
use chrono::Utc;
use std::collections::HashMap;
use std::path::Path;

impl Repo {
    /// Starts tracking a repository GitHub told us about.
    pub fn from_api(api: &ApiRepo, source: Option<String>, defaults: &RepoSettings) -> Repo {
        let settings = RepoSettings {
            destination: None,
            branch: None,
            ..defaults.clone()
        };
        Repo {
            id: api.full_name.to_lowercase(),
            owner: api.owner.login.clone(),
            name: api.name.clone(),
            description: api.description.clone(),
            html_url: api.html_url.clone(),
            private: api.private,
            fork: api.fork,
            archived: api.archived,
            default_branch: api.default_branch.clone().unwrap_or_else(|| "main".into()),
            pushed_at: api.pushed_at.clone(),
            remote_size_kb: api.size,
            source,
            added_at: Utc::now(),
            settings,
            remote: Remote::default(),
            local: None,
            versions: Vec::new(),
            gone_since: None,
            last_error: None,
        }
    }

    /// Refreshes what GitHub says about it. The name used for paths stays the same,
    /// so copies don't move around if the repository is renamed.
    pub fn apply_api(&mut self, api: &ApiRepo) {
        self.description = api.description.clone();
        self.html_url = api.html_url.clone();
        self.private = api.private;
        self.fork = api.fork;
        self.archived = api.archived;
        if let Some(b) = &api.default_branch {
            self.default_branch = b.clone();
        }
        self.pushed_at = api.pushed_at.clone();
        self.remote_size_kb = api.size;
        self.gone_since = None;
    }

    /// The branch Repopus follows: the user's choice, otherwise the default branch.
    pub fn branch(&self) -> &str {
        self.settings
            .branch
            .as_deref()
            .filter(|b| !b.trim().is_empty())
            .unwrap_or(&self.default_branch)
    }

    /// Whether the newest local copy is still on disk.
    pub fn local_exists(&self) -> bool {
        self.local.as_ref().is_some_and(|l| l.path.exists())
    }

    /// Whether the newest local copy sits where the current settings would put it.
    pub fn layout_matches(&self, base: &Path) -> bool {
        let Some(local) = &self.local else {
            return false;
        };
        if local.format != self.settings.format {
            return false;
        }
        let expected = Placement::for_copy(self, base, &local.sha, local.downloaded_at);
        match self.settings.mode {
            UpdateMode::Replace => local.path == expected.target,
            UpdateMode::KeepVersions => local.path.parent() == Some(expected.dir.as_path()),
        }
    }

    /// Whether downloading would change anything.
    pub fn needs_update(&self, base: &Path) -> bool {
        if self.gone_since.is_some() {
            return false;
        }
        let Some(local) = &self.local else {
            return true;
        };
        if !local.path.exists() {
            return true;
        }
        let newer = match &self.remote.sha {
            Some(remote) => *remote != local.sha || local.branch != self.branch(),
            None => false,
        };
        newer || !self.layout_matches(base)
    }

    /// Whether a scheduled check may download it on its own. A copy that vanished
    /// locally is not restored automatically, so the red mark gets noticed.
    pub fn wants_auto_update(&self, base: &Path) -> bool {
        let vanished = self.local.is_some() && !self.local_exists();
        self.settings.auto_update
            && self.remote.sha.is_some()
            && !vanished
            && self.needs_update(base)
    }

    /// The mark and text shown in the list.
    pub fn status(&self, base: &Path, busy: Option<&Busy>) -> (Status, String) {
        if let Some(busy) = busy {
            return (Status::Busy, busy.text.clone());
        }
        if self.gone_since.is_some() {
            let text = if self.local_exists() {
                "No longer available on GitHub. Your local copy is kept."
            } else {
                "No longer available on GitHub."
            };
            return (Status::Missing, text.into());
        }
        if let Some(error) = &self.last_error {
            return (Status::Error, error.clone());
        }
        let Some(local) = &self.local else {
            return (Status::UpdateAvailable, "Not downloaded yet".into());
        };
        if !local.path.exists() {
            return (
                Status::Missing,
                "The local copy is missing. It was moved or deleted.".into(),
            );
        }
        match &self.remote.sha {
            None => (Status::Unknown, "Not checked yet".into()),
            Some(remote) if *remote != local.sha || local.branch != self.branch() => {
                (Status::UpdateAvailable, "A newer version is available".into())
            }
            Some(_) if !self.layout_matches(base) => (
                Status::UpdateAvailable,
                "Storage settings changed. Update to apply them.".into(),
            ),
            Some(_) => (Status::UpToDate, "Up to date".into()),
        }
    }

    /// The row the window receives.
    pub fn to_view(&self, base: &Path, busy: &HashMap<String, Busy>) -> RepoView {
        let busy = busy.get(&self.id);
        let (status, status_text) = self.status(base, busy);
        let hint_sha = self
            .remote
            .sha
            .clone()
            .or_else(|| self.local.as_ref().map(|l| l.sha.clone()))
            .unwrap_or_else(|| "0000000".into());
        let next = Placement::for_copy(self, base, &hint_sha, Utc::now());

        RepoView {
            id: self.id.clone(),
            owner: self.owner.clone(),
            name: self.name.clone(),
            description: self.description.clone(),
            html_url: self.html_url.clone(),
            private: self.private,
            fork: self.fork,
            archived: self.archived,
            branch: self.branch().to_string(),
            default_branch: self.default_branch.clone(),
            source: self.source.clone(),
            status,
            status_text,
            busy: busy.cloned(),
            can_update: busy.is_none() && self.needs_update(base),
            remote_sha: self.remote.sha.clone(),
            remote_checked_at: self.remote.checked_at,
            pushed_at: self.pushed_at.clone(),
            remote_size_kb: self.remote_size_kb,
            local_exists: self.local_exists(),
            local: self.local.clone(),
            versions: self.versions.clone(),
            settings: self.settings.clone(),
            destination: Placement::root(self, base).to_string_lossy().into_owned(),
            target_hint: next.target.to_string_lossy().into_owned(),
            gone_since: self.gone_since,
            last_error: self.last_error.clone(),
        }
    }
}
