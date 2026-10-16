//! Downloading and installing new versions.
//!
//! Every update follows the same path: download into a temporary folder next to the
//! destination → verify → (unpack) → install in place → record the copy → clean up.

use super::{repositories, Engine, NOT_TRACKED};
use crate::archive::ZipFile;
use crate::files;
use crate::installer::Installer;
use crate::model::*;
use crate::placement::Placement;
use chrono::Utc;
use futures_util::{stream, StreamExt};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;

impl Engine {
    /// Downloads everything that needs it (including copies that went missing locally).
    pub async fn update_all(&self) -> Result<String, String> {
        self.update_matching(|_| true).await
    }

    /// Downloads everything that needs it within one category (owner).
    pub async fn update_owner(&self, owner: &str) -> Result<String, String> {
        let owner = owner.to_lowercase();
        self.update_matching(move |repo| repo.owner.to_lowercase() == owner).await
    }

    /// Downloads the latest version of one repository if it differs from the local copy.
    pub async fn update_repo(&self, id: &str) -> Result<(), String> {
        self.run_update(id, false).await
    }

    // ---------------------------------------------------------------- batches

    /// Updates every repository that needs it and passes `filter`.
    async fn update_matching(&self, filter: impl Fn(&Repo) -> bool) -> Result<String, String> {
        let ids: Vec<String> = {
            let st = self.inner.store.lock().await;
            let base = st.settings.base_folder.clone();
            st.repos
                .values()
                .filter(|r| filter(r) && r.needs_update(&base))
                .map(|r| r.id.clone())
                .collect()
        };
        let summary = if ids.is_empty() {
            "Everything is up to date".to_string()
        } else {
            self.update_ids(ids).await
        };
        self.activity(Some(summary.clone()), false);
        Ok(summary)
    }

    /// Updates the given repositories, a few at a time. Returns a summary line.
    pub(super) async fn update_ids(&self, ids: Vec<String>) -> String {
        let total = ids.len();
        let parallel = self.settings().await.concurrent_downloads.clamp(1, 8) as usize;
        for id in ids.iter().filter(|id| !self.inner.busy.contains(id)) {
            self.progress(id, Busy::waiting());
        }

        let engine = self.clone();
        let results: Vec<Result<(), String>> = stream::iter(ids)
            .map(move |id| {
                let engine = engine.clone();
                async move { engine.run_update(&id, true).await }
            })
            .buffer_unordered(parallel)
            .collect()
            .await;

        let failed = results.iter().filter(|r| r.is_err()).count();
        if failed == 0 {
            format!("Updated {}", repositories(total))
        } else {
            format!("Updated {} of {total} · {failed} failed", total - failed)
        }
    }

    // ---------------------------------------------------------------- one repository

    /// Runs one update with bookkeeping: busy mark, counters, error on the row.
    async fn run_update(&self, id: &str, queued: bool) -> Result<(), String> {
        if !self.inner.busy.claim(id, queued) {
            return Ok(()); // already being updated
        }
        self.inner.updating.fetch_add(1, Ordering::SeqCst);
        self.activity(None, false);
        self.emit_repo(id).await;

        let result = self.download_latest(id).await;

        self.inner.busy.clear(id);
        self.inner.updating.fetch_sub(1, Ordering::SeqCst);
        {
            let mut st = self.inner.store.lock().await;
            if let Some(repo) = st.repos.get_mut(id) {
                repo.last_error = result.as_ref().err().cloned();
            }
            self.save(&st);
        }
        self.emit_repo(id).await;
        self.activity(None, false);
        result
    }

    /// Makes sure the latest commit is known, then downloads it unless it's already there.
    async fn download_latest(&self, id: &str) -> Result<(), String> {
        let repo = self.repo_with_known_head(id).await?;
        let sha = repo.remote.sha.clone().unwrap_or_default();
        let base = self.settings().await.base_folder;
        if Self::already_installed(&repo, &base, &sha) {
            return Ok(());
        }

        let now = Utc::now();
        let placement = Placement::for_copy(&repo, &base, &sha, now);
        let scratch = Self::make_scratch_folder(&placement, &repo, now).await?;
        let outcome = self.download_and_install(&repo, &sha, &placement, &scratch).await;
        // Repopus's own temporary folder is always cleaned up.
        let _ = tokio::fs::remove_dir_all(&scratch).await;
        let (path, size) = outcome?;

        let copy = LocalCopy {
            sha,
            branch: repo.branch().to_string(),
            path,
            downloaded_at: now,
            size_bytes: size,
            format: repo.settings.format,
        };
        let replaced = self.record_copy(id, copy).await?;
        Self::remove_replaced_copies(replaced).await;
        Ok(())
    }

    /// The repository, with its latest commit looked up first if we don't know it yet.
    async fn repo_with_known_head(&self, id: &str) -> Result<Repo, String> {
        let mut repo = self.snapshot(id).await.ok_or(NOT_TRACKED)?;
        if repo.gone_since.is_some() {
            return Err("This repository is no longer available on GitHub. Your local copy is kept.".into());
        }
        let head_known = repo.remote.sha.is_some() && repo.remote.branch.as_deref() == Some(repo.branch());
        if !head_known {
            self.progress(id, Busy::working("Checking…"));
            let _ = self.check_remote(id).await;
            repo = self.snapshot(id).await.ok_or(NOT_TRACKED)?;
        }
        let head_known = repo.remote.sha.is_some() && repo.remote.branch.as_deref() == Some(repo.branch());
        if !head_known {
            return Err(repo
                .last_error
                .clone()
                .unwrap_or_else(|| "Couldn't find the latest version on GitHub.".into()));
        }
        Ok(repo)
    }

    /// Whether exactly this commit is already on disk where the settings want it.
    fn already_installed(repo: &Repo, base: &Path, sha: &str) -> bool {
        repo.local.as_ref().is_some_and(|local| {
            local.sha == sha
                && local.branch == repo.branch()
                && local.path.exists()
                && repo.layout_matches(base)
        })
    }

    /// Creates the temporary download folder next to the destination (same disk, so the
    /// final move is instant).
    async fn make_scratch_folder(placement: &Placement, repo: &Repo, now: chrono::DateTime<Utc>) -> Result<PathBuf, String> {
        tokio::fs::create_dir_all(&placement.dir)
            .await
            .map_err(|e| format!("Couldn't create {}: {e}", placement.dir.display()))?;
        let scratch = placement
            .dir
            .join(format!(".repopus-{}-{}", repo.name, now.timestamp_millis()));
        tokio::fs::create_dir_all(&scratch)
            .await
            .map_err(|e| format!("Couldn't create a temporary folder: {e}"))?;
        Ok(scratch)
    }

    /// Download → verify → (unpack) → install. Returns the final path and its size.
    async fn download_and_install(
        &self,
        repo: &Repo,
        sha: &str,
        placement: &Placement,
        scratch: &Path,
    ) -> Result<(PathBuf, u64), String> {
        let id = repo.id.clone();
        let zip_path = scratch.join("download.zip");

        // Without a size from GitHub, the last zip copy is a good guess.
        let estimate = repo
            .local
            .as_ref()
            .filter(|copy| copy.format == StorageFormat::Zip)
            .map(|copy| copy.size_bytes);
        self.progress(&id, Busy::downloading(0, estimate));
        let engine = self.clone();
        let progress_id = id.clone();
        self.inner
            .github
            .download_zip(&repo.owner, &repo.name, sha, &zip_path, move |bytes, total| {
                engine.progress(&progress_id, Busy::downloading(bytes, total.or(estimate)))
            })
            .await
            .map_err(|e| e.to_string())?;

        self.progress(&id, Busy::working("Verifying…"));
        let zip = zip_path.clone();
        blocking(move || ZipFile::new(zip).validate().map(|_| ())).await?;

        let staged = match repo.settings.format {
            StorageFormat::Zip => zip_path,
            StorageFormat::Folder => {
                self.progress(&id, Busy::working("Unpacking…"));
                let unpacked = scratch.join("unpacked");
                let (zip, dest) = (zip_path.clone(), unpacked.clone());
                blocking(move || ZipFile::new(zip).extract_stripped(&dest)).await?;
                unpacked
            }
        };

        self.progress(&id, Busy::working("Installing…"));
        let own_paths: Vec<PathBuf> = repo
            .local
            .iter()
            .chain(repo.versions.iter())
            .map(|copy| copy.path.clone())
            .collect();
        let (target, scratch) = (placement.target.clone(), scratch.to_path_buf());
        let installed = blocking(move || Installer::new(&staged, &target, &own_paths, &scratch).install()).await?;
        let measured = installed.clone();
        let size = blocking(move || Ok(files::size_of(&measured))).await?;
        Ok((installed, size))
    }

    /// Saves the new copy on the repository. Returns Repopus's own copies that it
    /// replaces: in "replace" mode the previous copy if it sat elsewhere (e.g. a zip that
    /// became a folder), in "keep versions" mode the versions beyond the user's limit.
    async fn record_copy(&self, id: &str, copy: LocalCopy) -> Result<Vec<PathBuf>, String> {
        let mut st = self.inner.store.lock().await;
        let repo = st.repos.get_mut(id).ok_or(NOT_TRACKED)?;
        repo.versions.retain(|v| v.path != copy.path && v.path.exists());
        let mut replaced = Vec::new();
        match repo.settings.mode {
            UpdateMode::KeepVersions => {
                repo.versions.push(copy.clone());
                replaced = Self::take_versions_over_limit(repo, &copy);
            }
            UpdateMode::Replace => {
                if let Some(old) = Self::previous_copy_elsewhere(repo, &copy) {
                    replaced.push(old);
                }
            }
        }
        repo.local = Some(copy);
        self.save(&st);
        Ok(replaced)
    }

    /// In "replace" mode: the previous copy, if it's at another path and isn't one of
    /// the kept versions (those stay).
    fn previous_copy_elsewhere(repo: &Repo, new: &LocalCopy) -> Option<PathBuf> {
        let old = repo.local.as_ref()?;
        let is_kept_version = repo.versions.iter().any(|v| v.path == old.path);
        (old.path != new.path && !is_kept_version && old.path.exists()).then(|| old.path.clone())
    }

    /// Removes the oldest versions (in the current versions folder) beyond "keep last N".
    fn take_versions_over_limit(repo: &mut Repo, newest: &LocalCopy) -> Vec<PathBuf> {
        let keep = repo.settings.keep_versions as usize;
        if keep == 0 {
            return Vec::new();
        }
        let folder = newest.path.parent().map(Path::to_path_buf);
        let in_folder: Vec<usize> = repo
            .versions
            .iter()
            .enumerate()
            .filter(|(_, v)| v.path.parent().map(Path::to_path_buf) == folder)
            .map(|(i, _)| i)
            .collect();
        if in_folder.len() <= keep {
            return Vec::new();
        }
        let oldest: HashSet<usize> = in_folder[..in_folder.len() - keep].iter().copied().collect();
        let mut removed = Vec::new();
        let mut index = 0;
        repo.versions.retain(|v| {
            let drop_it = oldest.contains(&index);
            index += 1;
            if drop_it {
                removed.push(v.path.clone());
            }
            !drop_it
        });
        removed
    }

    /// Deletes copies Repopus itself made and that a newer copy replaced.
    async fn remove_replaced_copies(paths: Vec<PathBuf>) {
        if paths.is_empty() {
            return;
        }
        let _ = tokio::task::spawn_blocking(move || {
            for path in paths {
                if let Err(e) = files::remove_any(&path) {
                    log::warn!("couldn't remove replaced copy {}: {e}", path.display());
                }
            }
        })
        .await;
    }
}

/// Runs disk-heavy work off the async threads.
async fn blocking<T, F>(work: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, String> + Send + 'static,
{
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|e| format!("Internal error: {e}"))?
}
