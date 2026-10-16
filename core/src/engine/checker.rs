//! Asking GitHub what's new. Nothing is downloaded here, except by the auto-update
//! step at the end of `check_all`.

use super::{repositories, Engine, EngineEvent};
use crate::github::Fetch;
use crate::model::Repo;
use chrono::Utc;
use futures_util::{stream, StreamExt};
use std::sync::atomic::Ordering;

/// How many repositories are checked at the same time.
const PARALLEL_CHECKS: usize = 6;

/// Outcome of one round of checking.
struct CheckReport {
    /// "Checked 12 repositories · 2 to update".
    message: String,
    /// The first problem, if any.
    problem: Option<String>,
    /// Every repository failed: nothing useful was learned.
    failed_completely: bool,
}

impl Engine {
    /// Checks everything, then downloads updates for repositories with auto-update on.
    /// If a check is already running, another round is queued right after it.
    pub async fn check_all(&self) -> Result<String, String> {
        if self.inner.checking.swap(true, Ordering::SeqCst) {
            self.inner.check_again.store(true, Ordering::SeqCst);
            return Ok("A check is already running.".into());
        }
        self.activity(None, false);
        let mut report = self.check_once().await;
        while self.inner.check_again.swap(false, Ordering::SeqCst) {
            report = self.check_once().await;
        }
        self.inner.checking.store(false, Ordering::SeqCst);

        match &report.problem {
            Some(p) => self.activity(Some(format!("{} · {p}", report.message)), true),
            None => self.activity(Some(report.message.clone()), false),
        }
        self.run_auto_updates().await;

        match report.problem {
            Some(p) if report.failed_completely => Err(p),
            _ => Ok(report.message),
        }
    }

    /// Checks one repository now (e.g. after its branch was changed).
    pub async fn check_repo(&self, id: &str) -> Result<(), String> {
        let result = self.check_remote(id).await;
        let st = self.inner.store.lock().await;
        self.save(&st);
        result
    }

    /// One round: list followed accounts, then learn the latest commit of every repository.
    async fn check_once(&self) -> CheckReport {
        let mut problems = Vec::new();

        for source_id in self.followed_account_ids().await {
            if let Err(e) = self.refresh_account(&source_id).await {
                problems.push(format!("{source_id}: {e}"));
            }
        }

        let ids = self.checkable_ids().await;
        let total = ids.len();
        let engine = self.clone();
        let results: Vec<Result<(), String>> = stream::iter(ids)
            .map(move |id| {
                let engine = engine.clone();
                async move { engine.check_remote(&id).await }
            })
            .buffer_unordered(PARALLEL_CHECKS)
            .collect()
            .await;
        problems.extend(results.into_iter().filter_map(Result::err));

        let message = self.finish_round().await;
        let failed_completely = total > 0 && problems.len() >= total;
        let extra = if problems.len() > 1 {
            format!(" · {} problems", problems.len())
        } else {
            String::new()
        };
        CheckReport {
            message: if failed_completely {
                "Couldn't check your repositories".into()
            } else {
                message + &extra
            },
            problem: problems.into_iter().next(),
            failed_completely,
        }
    }

    /// Saves the time of the check and builds the summary line.
    async fn finish_round(&self) -> String {
        let (count, updates, missing) = {
            let mut st = self.inner.store.lock().await;
            st.last_check = Some(Utc::now());
            self.save(&st);
            let base = st.settings.base_folder.clone();
            let updates = st
                .repos
                .values()
                .filter(|r| r.needs_update(&base) && (r.local.is_none() || r.local_exists()))
                .count();
            let missing = st
                .repos
                .values()
                .filter(|r| r.gone_since.is_some() || (r.local.is_some() && !r.local_exists()))
                .count();
            (st.repos.len(), updates, missing)
        };
        self.emit(EngineEvent::StoreChanged);

        let mut message = format!("Checked {}", repositories(count));
        if updates > 0 {
            message += &format!(" · {updates} to update");
        }
        if missing > 0 {
            message += &format!(" · {missing} missing");
        }
        if updates == 0 && missing == 0 && count > 0 {
            message += " · all up to date";
        }
        message
    }

    /// Downloads what changed for repositories with auto-update on.
    async fn run_auto_updates(&self) {
        let ids: Vec<String> = {
            let st = self.inner.store.lock().await;
            let base = st.settings.base_folder.clone();
            st.repos
                .values()
                .filter(|r| r.wants_auto_update(&base))
                .map(|r| r.id.clone())
                .collect()
        };
        if !ids.is_empty() {
            let summary = self.update_ids(ids).await;
            self.activity(Some(summary), false);
        }
    }

    /// Ids of all followed accounts.
    async fn followed_account_ids(&self) -> Vec<String> {
        let st = self.inner.store.lock().await;
        st.sources.iter().map(|s| s.id.clone()).collect()
    }

    /// Repositories worth asking about. Gone ones from followed accounts are skipped:
    /// the account listing already tells us if they come back.
    async fn checkable_ids(&self) -> Vec<String> {
        let st = self.inner.store.lock().await;
        st.repos
            .values()
            .filter(|r| !(r.source.is_some() && r.gone_since.is_some()))
            .map(|r| r.id.clone())
            .collect()
    }

    /// Learns the latest commit id of one repository. Problems are also stored on it.
    pub(super) async fn check_remote(&self, id: &str) -> Result<(), String> {
        let Some(repo) = self.snapshot(id).await else {
            return Ok(());
        };
        // Repositories added on their own get their details here (free when unchanged).
        if repo.source.is_none() {
            let still_there = self.refresh_single_repo(&repo).await?;
            if !still_there {
                return Ok(());
            }
        }
        let Some(repo) = self.snapshot(id).await else {
            return Ok(());
        };
        if repo.gone_since.is_some() {
            return Ok(());
        }
        let result = if Self::nothing_pushed_since_last_check(&repo) {
            self.touch_checked(id).await;
            Ok(())
        } else {
            self.fetch_branch_head(&repo).await
        };
        self.emit_repo(id).await;
        result
    }

    /// Refreshes the details of a repository that was added on its own.
    /// Returns `false` when it's gone from GitHub.
    async fn refresh_single_repo(&self, repo: &Repo) -> Result<bool, String> {
        // A gone repository is asked without ETag, so its return is noticed.
        let etag = if repo.gone_since.is_some() { None } else { repo.remote.repo_etag.clone() };
        let answer = self.inner.github.repo(&repo.owner, &repo.name, etag.as_deref()).await;
        let mut st = self.inner.store.lock().await;
        let Some(stored) = st.repos.get_mut(&repo.id) else {
            return Ok(false);
        };
        match answer {
            Ok(Fetch::Modified { data, etag }) => {
                stored.apply_api(&data);
                stored.remote.repo_etag = etag;
                Ok(true)
            }
            Ok(Fetch::NotModified) => Ok(true),
            Ok(Fetch::NotFound) => {
                let now = Utc::now();
                stored.gone_since.get_or_insert(now);
                stored.remote.checked_at = Some(now);
                stored.remote.repo_etag = None;
                drop(st);
                self.emit_repo(&repo.id).await;
                Ok(false)
            }
            Err(e) => {
                drop(st);
                self.record_error(&repo.id, e.to_string()).await.map(|_| false)
            }
        }
    }

    /// Same branch, and GitHub's `pushed_at` hasn't moved: no need to ask again.
    fn nothing_pushed_since_last_check(repo: &Repo) -> bool {
        repo.remote.sha.is_some()
            && repo.remote.branch.as_deref() == Some(repo.branch())
            && repo.pushed_at.is_some()
            && repo.remote.pushed_at_seen == repo.pushed_at
    }

    /// Notes that the repository was checked and is fine.
    async fn touch_checked(&self, id: &str) {
        let mut st = self.inner.store.lock().await;
        if let Some(repo) = st.repos.get_mut(id) {
            repo.remote.checked_at = Some(Utc::now());
            repo.last_error = None;
        }
    }

    /// Asks GitHub for the commit at the head of the followed branch.
    async fn fetch_branch_head(&self, repo: &Repo) -> Result<(), String> {
        let branch = repo.branch().to_string();
        let same_branch = repo.remote.branch.as_deref() == Some(branch.as_str());
        let etag = if same_branch { repo.remote.sha_etag.clone() } else { None };
        let answer = self
            .inner
            .github
            .branch_sha(&repo.owner, &repo.name, &branch, etag.as_deref())
            .await;

        let now = Utc::now();
        let mut st = self.inner.store.lock().await;
        let Some(stored) = st.repos.get_mut(&repo.id) else {
            return Ok(());
        };
        match answer {
            Ok(Fetch::Modified { data, etag }) => {
                stored.remote.sha = Some(data);
                stored.remote.sha_etag = etag;
                stored.remote.branch = Some(branch);
            }
            Ok(Fetch::NotModified) => {}
            Ok(Fetch::NotFound) => {
                let message = format!("The branch “{branch}” doesn't exist on GitHub.");
                stored.last_error = Some(message.clone());
                stored.remote.checked_at = Some(now);
                return Err(format!("{}: {message}", repo.name));
            }
            Err(e) => {
                drop(st);
                return self.record_error(&repo.id, e.to_string()).await;
            }
        }
        stored.remote.pushed_at_seen = repo.pushed_at.clone();
        stored.remote.checked_at = Some(now);
        stored.last_error = None;
        Ok(())
    }

    /// Stores a problem on the repository and returns it as an error.
    async fn record_error(&self, id: &str, message: String) -> Result<(), String> {
        let name = {
            let mut st = self.inner.store.lock().await;
            match st.repos.get_mut(id) {
                Some(repo) => {
                    repo.last_error = Some(message.clone());
                    repo.name.clone()
                }
                None => id.to_string(),
            }
        };
        self.emit_repo(id).await;
        Err(format!("{name}: {message}"))
    }
}
